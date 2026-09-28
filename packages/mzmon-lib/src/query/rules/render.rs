// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Rendering the registry's alerts into Prometheus rule files.
//!
//! [`render_rules`] turns every alert into a [`RenderedRule`], validating as it
//! goes, and [`RuleSet`] serializes the result: one `groups:` document per source
//! registry file, plus an index the chart selects from. Every problem is an
//! error, never a warning, and all of them are collected before returning, so a
//! contributor sees the whole list at once.
//!
//! The checks are the ones a rule needs and a dashboard query does not: a rule
//! is evaluated unattended, so a mistake that a panel would show as an empty
//! chart is, in a rule, silence that reads as health.

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;

use crate::query::docgen::extract_metric_docs;
use crate::query::extract::ExtractedMetric;
use crate::query::importance::Importance;
use crate::query::model::{Alert, QueryEngine};
use crate::query::registry::QueryRegistry;
use crate::query::render::{SQL_PREFIX_SENTINEL, TemplateContext, tier_context};
use crate::query::rules::capability::{Capability, MetricSource, capability_for_metric};
use crate::query::rules::context::{Placeholder, alerting_context};
use crate::query::stability::Stability;

/// Where an alert's `runbook_url` points until runbooks exist: its own entry on
/// the generated Common Alerts page, whose headings are anchored by alert name.
pub const COMMON_ALERTS_URL: &str = "https://materializeinc.github.io/materialize-monitoring/reference/stable-metrics/common-alerts/";

/// The severities the chart's routing presets know how to route.
pub const SEVERITIES: &[&str] = &["critical", "warning", "notice"];

/// One alert, rendered and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedRule {
    pub alert: String,
    /// The registry file stem this rule is written under.
    pub source: String,
    pub group: String,
    /// The rule expression, placeholders intact.
    pub expr: String,
    pub for_: String,
    pub keep_firing_for: Option<String>,
    pub labels: IndexMap<String, String>,
    pub annotations: IndexMap<String, String>,
    /// Inferred from the metrics the rule reads, plus what it declares.
    pub requires: BTreeSet<Capability>,
    pub enabled_by_default: bool,
    /// The least important metric-tier any metric the rule reads is in. A
    /// destination filtering below it does not receive everything the rule
    /// needs.
    pub min_importance: Option<Importance>,
}

/// A problem with one alert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    pub alert: String,
    pub message: String,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.alert, self.message)
    }
}

/// Every rendered rule, in source-file then registration order.
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    pub rules: Vec<RenderedRule>,
}

static ALERT_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap());
static GROUP_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+(_[a-z0-9]+)*$").unwrap());
/// A Prometheus duration: units in descending order, each at most once.
static DURATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([0-9]+y)?([0-9]+w)?([0-9]+d)?([0-9]+h)?([0-9]+m)?([0-9]+s)?([0-9]+ms)?$")
        .unwrap()
});
/// A Grafana variable: `$name`, `${name}`, `$__builtin`. `$1` in a
/// `label_replace` replacement is not one.
static GRAFANA_VARIABLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$(\{|__|[A-Za-z_])").unwrap());
static PLACEHOLDER_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"__mzmon_[a-z_]+?__").unwrap());

/// Render and validate every alert in `registry`.
pub fn render_rules(registry: &QueryRegistry) -> Result<RuleSet, Vec<RuleError>> {
    let ctx = alerting_context(registry, QueryEngine::PromQl, false);
    let inference_ctx = alerting_context(registry, QueryEngine::PromQl, true);
    let importance = metric_importance(registry);

    let mut rules = Vec::new();
    let mut errors = Vec::new();
    for alert in registry.alerts() {
        let mut problems = Vec::new();
        let rule = render_one(
            registry,
            alert,
            &ctx,
            &inference_ctx,
            &importance,
            &mut problems,
        );
        if problems.is_empty() {
            if let Some(rule) = rule {
                rules.push(rule);
            }
        } else {
            errors.extend(problems.into_iter().map(|message| RuleError {
                alert: alert.alert.clone(),
                message,
            }));
        }
    }

    if errors.is_empty() {
        // Group by source file while keeping each file's own order.
        let mut by_source: IndexMap<String, Vec<RenderedRule>> = IndexMap::new();
        for rule in rules {
            by_source.entry(rule.source.clone()).or_default().push(rule);
        }
        by_source.sort_keys();
        Ok(RuleSet {
            rules: by_source.into_values().flatten().collect(),
        })
    } else {
        Err(errors)
    }
}

fn render_one(
    registry: &QueryRegistry,
    alert: &Alert,
    ctx: &TemplateContext,
    inference_ctx: &TemplateContext,
    importance: &HashMap<String, Importance>,
    problems: &mut Vec<String>,
) -> Option<RenderedRule> {
    if !ALERT_NAME.is_match(&alert.alert) {
        problems.push("the alert name must be kebab-case (`[a-z0-9]` words joined by `-`)".into());
    }
    if !GROUP_NAME.is_match(&alert.group) {
        problems.push(format!(
            "group `{}` must be snake_case (`[a-z0-9]` words joined by `_`)",
            alert.group
        ));
    }
    match alert.labels.get("severity").map(String::as_str) {
        Some(severity) if SEVERITIES.contains(&severity) => {}
        Some(other) => problems.push(format!(
            "severity `{other}` is not one of {}; the routing presets have no class for it",
            SEVERITIES.join(", ")
        )),
        None => problems.push("a `severity` label is required".into()),
    }
    if alert.labels.get("component").is_none_or(String::is_empty) {
        problems.push("a `component` label is required".into());
    }
    if alert.labels.contains_key("deploymentMode") {
        problems.push(
            "`deploymentMode` is not a rule label; declare what the rule needs with `requires`"
                .into(),
        );
    }
    check_duration("for", &alert.for_, problems);
    if let Some(keep) = &alert.keep_firing_for {
        check_duration("keepFiringFor", keep, problems);
    }
    if alert.enabled_by_default
        && !matches!(
            alert.stability,
            Stability::BestEffort | Stability::Canonical
        )
    {
        problems.push(format!(
            "`enabledByDefault` needs a `best-effort` or `canonical` query, not `{}`",
            alert.stability
        ));
    }
    let Some(source) = alert.source.clone() else {
        problems.push("the alert was not loaded from a registry file".into());
        return None;
    };

    let Some(query) = registry.get(&alert.query_id) else {
        problems.push(format!("`queryId` `{}` names no query", alert.query_id));
        return None;
    };
    if query.is_log_query() {
        problems.push("LogQL alerts are not rendered yet; only PromQL rules are supported".into());
        return None;
    }
    if query.promql.len() != 1 {
        problems.push(format!(
            "a rule needs exactly one PromQL expression, and `{}` has {}",
            query.id,
            query.promql.len()
        ));
        return None;
    }

    let expr = match query.render(ctx) {
        Ok(mut rendered) => rendered.remove(0).trim().to_string(),
        Err(err) => {
            problems.push(format!("rendering failed: {err}"));
            return None;
        }
    };
    check_expr(&expr, problems);

    let requires = match query.render(inference_ctx) {
        Ok(mut rendered) => infer_requires(&rendered.remove(0), &alert.requires, problems),
        Err(err) => {
            problems.push(format!("rendering for capability inference failed: {err}"));
            return None;
        }
    };
    let min_importance = ExtractedMetric::extract_from_promql(&expr)
        .ok()
        .and_then(|metrics| {
            metrics
                .iter()
                .filter_map(|m| importance.get(&normalize_sql_prefix(&m.name)).copied())
                .min_by_key(|i| i.rank())
        });

    Some(RenderedRule {
        alert: alert.alert.clone(),
        source,
        group: alert.group.clone(),
        expr,
        for_: alert.for_.clone(),
        keep_firing_for: alert.keep_firing_for.clone(),
        labels: alert.labels.clone(),
        annotations: annotations(alert),
        requires,
        enabled_by_default: alert.enabled_by_default,
        min_importance,
    })
}

fn check_duration(field: &str, value: &str, problems: &mut Vec<String>) {
    if value.is_empty() || !DURATION.is_match(value) {
        problems.push(format!(
            "`{field}: {value}` is not a Prometheus duration (e.g. `5m`, `1h30m`)"
        ));
    }
}

/// Checks on the expression as the ruler will see it, placeholders aside.
fn check_expr(expr: &str, problems: &mut Vec<String>) {
    if let Err(message) = promql_parser::parser::parse(expr) {
        problems.push(format!(
            "the rendered expression is not valid PromQL: {message}"
        ));
    }
    if expr.contains("%%{") {
        problems.push("an unrendered `%%{…}` placeholder remains".into());
    }
    if GRAFANA_VARIABLE.is_match(expr) {
        problems.push(
            "the expression references a Grafana variable, which no ruler can resolve".into(),
        );
    }
    if expr.contains(SQL_PREFIX_SENTINEL) {
        problems.push(format!(
            "`{SQL_PREFIX_SENTINEL}` is metric-tiers' sentinel, not an alerting placeholder"
        ));
    }
    for token in PLACEHOLDER_TOKEN.find_iter(expr) {
        if Placeholder::from_token(token.as_str()).is_none() {
            problems.push(format!(
                "`{}` is not a placeholder the chart replaces",
                token.as_str()
            ));
        }
    }
}

/// The capabilities `expr` needs, from the metrics it names, plus `declared`.
fn infer_requires(
    expr: &str,
    declared: &[Capability],
    problems: &mut Vec<String>,
) -> BTreeSet<Capability> {
    let mut requires: BTreeSet<Capability> = declared.iter().copied().collect();
    let metrics = match ExtractedMetric::extract_from_promql(expr) {
        Ok(metrics) => metrics,
        // Already reported by `check_expr`.
        Err(_) => return requires,
    };
    if metrics.is_empty() {
        problems.push(
            "the expression names no metric, so what it needs cannot be inferred or checked".into(),
        );
        return requires;
    }
    let mut named_capability = false;
    let mut unknown = false;
    for metric in &metrics {
        match capability_for_metric(&metric.name) {
            Some(MetricSource::Capability(capability)) => {
                named_capability = true;
                requires.insert(capability);
            }
            Some(MetricSource::Neutral) => {}
            None => {
                unknown = true;
                problems.push(format!(
                    "nothing is known to produce `{}`; add its source to the capability table in \
                 packages/mzmon-lib/src/query/rules/capability.rs",
                    metric.name
                ));
            }
        }
    }
    if !named_capability && !unknown && declared.is_empty() {
        problems.push(
            "the expression reads only metrics every target exposes (such as `up`), so declare \
             what it needs with `requires`"
                .into(),
        );
    }
    requires
}

/// Build the annotations: `summary`, a `description` from the structured prose,
/// and a `runbook_url`. Anything the author wrote wins.
fn annotations(alert: &Alert) -> IndexMap<String, String> {
    let mut out = IndexMap::new();
    out.insert(
        "summary".to_string(),
        alert.description.summary.trim().to_string(),
    );
    let prose: Vec<&str> = [
        alert.description.unhealthy.as_deref(),
        alert.description.degraded.as_deref(),
        alert.description.notes.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .collect();
    if !prose.is_empty() {
        out.insert("description".to_string(), prose.join("\n\n"));
    }
    out.insert(
        "runbook_url".to_string(),
        format!("{COMMON_ALERTS_URL}#{}", alert.alert),
    );
    for (key, value) in &alert.annotations {
        out.insert(key.clone(), value.clone());
    }
    out
}

/// Each metric's rolled-up importance, keyed with the alerting SQL placeholder
/// so it can be looked up by the names a rendered rule contains.
fn metric_importance(registry: &QueryRegistry) -> HashMap<String, Importance> {
    let ctx = tier_context(registry, QueryEngine::PromQl);
    extract_metric_docs(registry, &ctx)
        .metrics
        .into_iter()
        .filter_map(|doc| {
            let importance = doc.importance.parse::<Importance>().ok()?;
            Some((normalize_sql_prefix(&doc.name), importance))
        })
        .collect()
}

fn normalize_sql_prefix(name: &str) -> String {
    let rule_token = Placeholder::SqlPrefix.token();
    match name.strip_prefix(SQL_PREFIX_SENTINEL) {
        Some(rest) => format!("{rule_token}{rest}"),
        None => name.to_string(),
    }
}

// -- serialization -----------------------------------------------------------

/// Header written at the top of every generated rule file.
pub const RULE_FILE_HEADER: &str = "\
# Generated by `mz-monitoring-build gen-rules` from packages/queries/. DO NOT EDIT.
#
# Tokens of the form __mzmon_*__ are replaced by the chart at install time
# (templates/alerts/prometheusrules.yaml); which rules install is decided there
# from _index.yaml.
";

#[derive(Serialize)]
struct RuleFileDoc<'a> {
    groups: Vec<GroupDoc<'a>>,
}

#[derive(Serialize)]
struct GroupDoc<'a> {
    name: &'a str,
    rules: Vec<RuleDoc<'a>>,
}

#[derive(Serialize)]
struct RuleDoc<'a> {
    alert: &'a str,
    expr: &'a str,
    #[serde(rename = "for")]
    for_: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_firing_for: Option<&'a str>,
    labels: &'a IndexMap<String, String>,
    annotations: &'a IndexMap<String, String>,
}

#[derive(Serialize)]
struct IndexDoc<'a> {
    placeholders: Vec<&'static str>,
    capabilities: IndexMap<&'static str, CapabilityDoc>,
    rules: IndexMap<&'a str, IndexRuleDoc<'a>>,
}

#[derive(Serialize)]
struct CapabilityDoc {
    derived: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexRuleDoc<'a> {
    file: String,
    group: &'a str,
    severity: &'a str,
    requires: Vec<&'static str>,
    enabled_by_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_importance: Option<String>,
}

impl RuleSet {
    /// The source file stems, in output order.
    pub fn sources(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for rule in &self.rules {
            if !seen.contains(&rule.source.as_str()) {
                seen.push(&rule.source);
            }
        }
        seen
    }

    /// The `groups:` document for one source file.
    pub fn rule_file_yaml(&self, source: &str) -> serde_yaml_ng::Result<String> {
        let mut groups: IndexMap<&str, Vec<RuleDoc>> = IndexMap::new();
        for rule in self.rules.iter().filter(|r| r.source == source) {
            groups.entry(&rule.group).or_default().push(RuleDoc {
                alert: &rule.alert,
                expr: &rule.expr,
                for_: &rule.for_,
                keep_firing_for: rule.keep_firing_for.as_deref(),
                labels: &rule.labels,
                annotations: &rule.annotations,
            });
        }
        let doc = RuleFileDoc {
            groups: groups
                .into_iter()
                .map(|(name, rules)| GroupDoc { name, rules })
                .collect(),
        };
        Ok(format!(
            "{RULE_FILE_HEADER}{}",
            serde_yaml_ng::to_string(&doc)?
        ))
    }

    /// The index the chart selects from.
    pub fn index_yaml(&self) -> serde_yaml_ng::Result<String> {
        let doc = IndexDoc {
            placeholders: Placeholder::ALL.iter().map(|p| p.token()).collect(),
            capabilities: Capability::ALL
                .iter()
                .map(|c| {
                    (
                        c.as_str(),
                        CapabilityDoc {
                            derived: c.is_derived(),
                        },
                    )
                })
                .collect(),
            rules: self
                .rules
                .iter()
                .map(|rule| {
                    (
                        rule.alert.as_str(),
                        IndexRuleDoc {
                            file: format!("{}.yaml", rule.source),
                            group: &rule.group,
                            severity: rule.labels.get("severity").map_or("", String::as_str),
                            requires: rule.requires.iter().map(|c| c.as_str()).collect(),
                            enabled_by_default: rule.enabled_by_default,
                            min_importance: rule.min_importance.map(|i| i.to_string()),
                        },
                    )
                })
                .collect(),
        };
        let header = "\
# Generated by `mz-monitoring-build gen-rules`. DO NOT EDIT.
#
# What the chart needs to decide which rules install: each rule's capabilities
# (inferred from the metrics it reads, plus any it declares) and whether it is in
# the default set, and the vocabularies the chart validates values against.
";
        Ok(format!("{header}{}", serde_yaml_ng::to_string(&doc)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::def::RegistryDoc;

    fn registry(alerts_yaml: &str) -> QueryRegistry {
        let yaml =
            format!("description: test\nmetricImportanceHint: essential\nalerts:\n{alerts_yaml}");
        let doc = RegistryDoc::from_yaml_str(&yaml).expect("test registry parses");
        let mut registry = QueryRegistry::new();
        registry.load_from(doc, Some("test-alerts")).unwrap();
        registry
    }

    fn alert(name: &str, labels: &str, promql: &str, extra: &str) -> String {
        format!(
            r#"  - alert: {name}
    group: test_group
    stability: best-effort
    for: 5m
    labels: {labels}
    description:
      summary: Something is wrong.
      unhealthy: Do the thing.
{extra}    query:
      id: test.{id}
      stability: best-effort
      description:
        summary: q
      promQL: '{promql}'
"#,
            id = name.replace('-', "_")
        )
    }

    const LABELS: &str = "{severity: warning, component: test}";

    fn errors(yaml: &str) -> Vec<String> {
        render_rules(&registry(yaml))
            .expect_err("expected errors")
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    fn has(errors: &[String], needle: &str) -> bool {
        errors.iter().any(|e| e.contains(needle))
    }

    #[test]
    fn renders_a_rule_with_placeholders_and_inferred_requires() {
        let set = render_rules(&registry(&alert(
            "pod-pending",
            LABELS,
            r#"max by (namespace) (kube_pod_status_phase{%%{mzEnvironmentNamespaceFilter}, phase="Pending"}) > 0"#,
            "    enabledByDefault: true\n",
        )))
        .unwrap();
        let rule = &set.rules[0];
        assert_eq!(
            rule.expr,
            r#"max by (namespace) (kube_pod_status_phase{namespace=~"__mzmon_environment_namespaces__", phase="Pending"}) > 0"#
        );
        assert_eq!(
            rule.requires.iter().collect::<Vec<_>>(),
            vec![&Capability::KubeStateMetrics]
        );
        assert!(rule.enabled_by_default);
        assert_eq!(rule.annotations["summary"], "Something is wrong.");
        assert_eq!(rule.annotations["description"], "Do the thing.");
        assert!(rule.annotations["runbook_url"].ends_with("#pod-pending"));

        let file = set.rule_file_yaml("test-alerts").unwrap();
        assert!(file.starts_with("# Generated by"));
        assert!(file.contains("- name: test_group"));
        let index = set.index_yaml().unwrap();
        assert!(index.contains("file: test-alerts.yaml"));
        assert!(index.contains("enabledByDefault: true"));
    }

    #[test]
    fn the_environment_join_does_not_leak_into_requires() {
        // `mzEnvironmentName` reads `up` for itself; the rule does not need it.
        let yaml = r#"  - alert: envd-memory
    group: test_group
    stability: best-effort
    for: 5m
    labels: {severity: warning, component: test}
    description:
      summary: s
    query:
      id: test.envd_memory
      stability: best-effort
      description:
        summary: q
      promQL:
        template: 'container_memory_working_set_bytes{%%{cAdvisorFilter}} > 0'
        functions:
          - name: mzEnvironmentName
            args: ["namespace"]
"#;
        let set = render_rules(&registry(yaml)).unwrap();
        let rule = &set.rules[0];
        assert!(
            rule.expr
                .contains("group_left(materialize_cloud_organization_name)")
        );
        assert_eq!(
            rule.requires.iter().collect::<Vec<_>>(),
            vec![&Capability::Cadvisor]
        );
    }

    #[test]
    fn metadata_problems_are_all_reported() {
        let errs = errors(&alert(
            "Bad Name",
            "{severity: urgent, deploymentMode: cloud-only}",
            "mz_up_thing > 0",
            "",
        ));
        assert!(has(&errs, "kebab-case"), "{errs:?}");
        assert!(has(&errs, "severity `urgent`"), "{errs:?}");
        assert!(has(&errs, "`component` label"), "{errs:?}");
        assert!(has(&errs, "`deploymentMode`"), "{errs:?}");
    }

    #[test]
    fn bad_durations_are_errors() {
        let yaml = alert("x", LABELS, "mz_thing > 0", "").replace("for: 5m", "for: banana");
        assert!(has(&errors(&yaml), "not a Prometheus duration"));
    }

    #[test]
    fn range_parameters_fail_to_render() {
        let errs = errors(&alert("x", LABELS, "rate(mz_thing%%{interval}) > 0", ""));
        assert!(has(&errs, "rendering failed"), "{errs:?}");
    }

    #[test]
    fn invalid_promql_is_an_error() {
        assert!(has(
            &errors(&alert("x", LABELS, "this is ((( not promql", "")),
            "not valid PromQL"
        ));
    }

    #[test]
    fn grafana_variables_and_stray_tokens_are_errors() {
        let errs = errors(&alert(
            "x",
            LABELS,
            r#"mz_thing{a="$environmentNameList"} > 0"#,
            "",
        ));
        assert!(has(&errs, "Grafana variable"), "{errs:?}");
        let errs = errors(&alert(
            "x",
            LABELS,
            r#"mz_thing{a="__mzmon_bogus__"} > 0"#,
            "",
        ));
        assert!(has(&errs, "not a placeholder"), "{errs:?}");
        let errs = errors(&alert("x", LABELS, "__mz_sql_prefix__thing > 0", ""));
        assert!(has(&errs, "metric-tiers' sentinel"), "{errs:?}");
    }

    #[test]
    fn label_replace_group_references_are_not_grafana_variables() {
        let set = render_rules(&registry(&alert(
            "x",
            LABELS,
            r#"label_replace(mz_thing, "a", "$1", "b", "(.*)") > 0"#,
            "",
        )));
        assert!(set.is_ok(), "{set:?}");
    }

    #[test]
    fn unknown_and_neutral_metrics() {
        let errs = errors(&alert("x", LABELS, "vendor_widget_total > 0", ""));
        assert!(
            has(&errs, "nothing is known to produce `vendor_widget_total`"),
            "{errs:?}"
        );

        let errs = errors(&alert("x", LABELS, r#"up{job="x"} == 0"#, ""));
        assert!(has(&errs, "declare what it needs"), "{errs:?}");

        let set = render_rules(&registry(&alert(
            "x",
            LABELS,
            r#"up{job="x"} == 0"#,
            "    requires: [materialize]\n",
        )))
        .unwrap();
        assert!(set.rules[0].requires.contains(&Capability::Materialize));

        let errs = errors(&alert("x", LABELS, "vector(1)", ""));
        assert!(has(&errs, "names no metric"), "{errs:?}");
    }

    #[test]
    fn dangling_query_ids_are_errors() {
        let yaml = r#"  - alert: x
    group: test_group
    stability: best-effort
    for: 5m
    labels: {severity: warning, component: test}
    description:
      summary: s
    queryId: does.not.exist
"#;
        assert!(has(&errors(yaml), "names no query"));
    }

    /// Every placeholder, and the environment join, is a token promtool accepts
    /// in the position it occupies, which is what lets the pre-rendered files be
    /// checked before the chart fills them in.
    #[test]
    fn promtool_accepts_every_placeholder_as_rendered() {
        let joined = r#"  - alert: envd-memory
    group: test_group
    stability: best-effort
    for: 5m
    labels: {severity: warning, component: test}
    description:
      summary: s
    query:
      id: test.envd_memory
      stability: best-effort
      description:
        summary: q
      promQL:
        template: 'container_memory_working_set_bytes{%%{cAdvisorFilter}} > 0'
        functions:
          - name: mzEnvironmentName
            args: ["namespace"]
"#;
        let yaml = format!(
            "{}{}{joined}",
            alert(
                "cluster-offline",
                LABELS,
                r#"%%{mzSqlPrefix}compute_cluster_status{%%{mzEnvironmentNamespaceFilter}, %%{excludeEnvironmentFilter}} == 0"#,
                ""
            ),
            alert(
                "infra-pod-pending",
                LABELS,
                r#"kube_pod_status_phase{%%{excludeMzDeploymentNamespaceFilter}, phase="Pending"} > 0"#,
                ""
            ),
        );
        let set = render_rules(&registry(&yaml)).unwrap();
        let file = set.rule_file_yaml("test-alerts").unwrap();
        for placeholder in Placeholder::ALL {
            assert!(file.contains(placeholder.token()), "{placeholder:?} unused");
        }
        crate::scrape::test_support::assert_promtool_rules_ok("test-alerts", &file);
    }

    #[test]
    fn output_is_deterministic() {
        let yaml = format!(
            "{}{}",
            alert("b-alert", LABELS, "mz_b > 0", ""),
            alert("a-alert", LABELS, "mz_a > 0", "")
        );
        let registry = registry(&yaml);
        let one = render_rules(&registry).unwrap();
        let two = render_rules(&registry).unwrap();
        assert_eq!(
            one.rule_file_yaml("test-alerts").unwrap(),
            two.rule_file_yaml("test-alerts").unwrap()
        );
        // Registration order, not alphabetical.
        assert_eq!(one.rules[0].alert, "b-alert");
    }
}
