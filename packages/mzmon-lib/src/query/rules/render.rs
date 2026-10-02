// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Rendering the registry's alerts into rule files for the two rulers.
//!
//! [`render_rules`] turns every alert into a [`RenderedRule`], validating as it
//! goes, and [`RuleSet`] serializes the result: one `groups:` document per source
//! registry file and [`RuleEngine`], plus one index of every rule the chart
//! selects from. Every problem is an error, never a warning, and all of them are
//! collected before returning, so a contributor sees the whole list at once.
//!
//! An alert whose query is PromQL is a Prometheus rule for the Thanos ruler; one
//! whose query is LogQL is a rule for the Loki ruler. Both files share the
//! Prometheus rule format, and the two engines share every check that is about
//! the rule rather than its language.
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
pub const COMMON_ALERTS_URL: &str =
    "https://materializeinc.github.io/materialize-monitoring/reference/common-alerts/";

/// The severities the chart's routing presets know how to route.
pub const SEVERITIES: &[&str] = &["critical", "warning", "notice"];

/// Who an alert is for, as its `audience` label: `platform` is whoever runs the
/// deployment, `workload` whoever runs what is on it. A route matches on it to
/// send the two to different people.
pub const AUDIENCES: &[&str] = &["platform", "workload"];

/// The ruler that evaluates a rule, which follows from its query's language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleEngine {
    /// PromQL, evaluated by the Thanos ruler from `PrometheusRule` resources.
    PromQl,
    /// LogQL, evaluated by the Loki ruler from rule files the chart delivers
    /// into its rule store.
    LogQl,
}

impl RuleEngine {
    pub const ALL: [RuleEngine; 2] = [RuleEngine::PromQl, RuleEngine::LogQl];

    /// The query language, as the index records it.
    pub fn as_str(self) -> &'static str {
        match self {
            RuleEngine::PromQl => "promql",
            RuleEngine::LogQl => "logql",
        }
    }

    /// The directory under `pre-rendered/rules/` this engine's files go in.
    pub fn dir(self) -> &'static str {
        match self {
            RuleEngine::PromQl => "prometheus",
            RuleEngine::LogQl => "loki",
        }
    }

    /// The chart template that installs this engine's files.
    fn template(self) -> &'static str {
        match self {
            RuleEngine::PromQl => "templates/alerts/prometheusrules.yaml",
            RuleEngine::LogQl => "templates/alerts/lokirules.yaml",
        }
    }
}

impl fmt::Display for RuleEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One alert, rendered and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedRule {
    pub alert: String,
    /// The registry file stem this rule is written under.
    pub source: String,
    pub engine: RuleEngine,
    pub group: String,
    /// The rule expression, placeholders intact.
    pub expr: String,
    pub for_: String,
    pub keep_firing_for: Option<String>,
    pub labels: IndexMap<String, String>,
    pub annotations: IndexMap<String, String>,
    /// Inferred from the metrics the rule reads, plus what it declares. A LogQL
    /// rule names no metrics, so its set is what it declares.
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
/// A LogQL range: `[5m]`, `[1h30m]`. LogQL has no subqueries, so no `:step`.
static LOGQL_RANGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\s*([0-9]+(ms|s|m|h|d|w|y))+\s*\]").unwrap());

/// The contexts a rule renders through.
struct Contexts<'a> {
    promql: TemplateContext<'a>,
    /// PromQL with every enrichment function as the identity, for inference.
    inference: TemplateContext<'a>,
    logql: TemplateContext<'a>,
}

/// Render and validate every alert in `registry`.
pub fn render_rules(registry: &QueryRegistry) -> Result<RuleSet, Vec<RuleError>> {
    let contexts = Contexts {
        promql: alerting_context(registry, QueryEngine::PromQl, false),
        inference: alerting_context(registry, QueryEngine::PromQl, true),
        logql: alerting_context(registry, QueryEngine::LogQl, false),
    };
    let importance = metric_importance(registry);

    let mut rules = Vec::new();
    let mut errors = Vec::new();
    for alert in registry.alerts() {
        let mut problems = Vec::new();
        let rule = render_one(registry, alert, &contexts, &importance, &mut problems);
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

    // The chart names each `PrometheusRule` after its registry file, so a file
    // holding both engines' alerts would render two objects with one name.
    let mut engines_by_source: IndexMap<&str, BTreeSet<RuleEngine>> = IndexMap::new();
    for rule in &rules {
        engines_by_source
            .entry(&rule.source)
            .or_default()
            .insert(rule.engine);
    }
    for rule in &rules {
        if rule.engine == RuleEngine::LogQl && engines_by_source[rule.source.as_str()].len() > 1 {
            errors.push(RuleError {
                alert: rule.alert.clone(),
                message: format!(
                    "`{}` holds PromQL and LogQL alerts, and each registry file installs as one \
                     `PrometheusRule` per ruler under the file's name; move the LogQL alerts to \
                     their own file, such as `materialize-log-alerts.yaml`",
                    rule.source
                ),
            });
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
    contexts: &Contexts,
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
    match alert.labels.get("audience").map(String::as_str) {
        Some(audience) if AUDIENCES.contains(&audience) => {}
        Some(other) => problems.push(format!(
            "audience `{other}` is not one of {}",
            AUDIENCES.join(", ")
        )),
        None => problems.push(
            "an `audience` label is required; set it for the whole file with `alertLabels`".into(),
        ),
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
    let engine = if query.is_log_query() {
        RuleEngine::LogQl
    } else {
        RuleEngine::PromQl
    };
    let expressions = match engine {
        RuleEngine::PromQl => &query.promql,
        RuleEngine::LogQl => &query.logql,
    };
    if engine == RuleEngine::LogQl && !query.promql.is_empty() {
        problems.push(format!(
            "`{}` has both a PromQL and a LogQL expression; a rule is evaluated by one ruler, so \
             its query is one or the other",
            query.id
        ));
        return None;
    }
    if expressions.len() != 1 {
        problems.push(format!(
            "a rule needs exactly one {} expression, and `{}` has {}",
            match engine {
                RuleEngine::PromQl => "PromQL",
                RuleEngine::LogQl => "LogQL",
            },
            query.id,
            expressions.len()
        ));
        return None;
    }

    let ctx = match engine {
        RuleEngine::PromQl => &contexts.promql,
        RuleEngine::LogQl => &contexts.logql,
    };
    let expr = match query.render(ctx) {
        Ok(mut rendered) => rendered.remove(0).trim().to_string(),
        Err(err) => {
            problems.push(format!("rendering failed: {err}"));
            return None;
        }
    };

    let (requires, min_importance) = match engine {
        RuleEngine::PromQl => {
            check_promql_expr(&expr, problems);
            let requires = match query.render(&contexts.inference) {
                Ok(mut rendered) => infer_requires(&rendered.remove(0), &alert.requires, problems),
                Err(err) => {
                    problems.push(format!("rendering for capability inference failed: {err}"));
                    return None;
                }
            };
            let min_importance =
                ExtractedMetric::extract_from_promql(&expr)
                    .ok()
                    .and_then(|metrics| {
                        metrics
                            .iter()
                            .filter_map(|m| importance.get(&normalize_sql_prefix(&m.name)).copied())
                            .min_by_key(|i| i.rank())
                    });
            (requires, min_importance)
        }
        // A log rule reads no metrics, so there is nothing to infer from and no
        // metric tier a destination could filter it out of. Whether the logs it
        // reads are collected, and whether a Loki ruler evaluates it, is the
        // chart's to decide.
        RuleEngine::LogQl => {
            check_logql_expr(&expr, problems);
            (alert.requires.iter().copied().collect(), None)
        }
    };

    Some(RenderedRule {
        alert: alert.alert.clone(),
        source,
        engine,
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

/// Checks on a PromQL expression as the Thanos ruler will see it, placeholders
/// aside.
fn check_promql_expr(expr: &str, problems: &mut Vec<String>) {
    if let Err(message) = promql_parser::parser::parse(expr) {
        problems.push(format!(
            "the rendered expression is not valid PromQL: {message}"
        ));
    }
    check_tokens(expr, problems);
}

/// Checks on a LogQL expression as the Loki ruler will see it, placeholders
/// aside.
///
/// There is no LogQL parser here, so this checks the shape a rule needs and
/// `make rules-check` parses every rendered rule with Loki's own (`logcli`).
/// The shape matters beyond syntax: a log query, rather than a metric query,
/// parses and is refused by the ruler when it loads the group.
fn check_logql_expr(expr: &str, problems: &mut Vec<String>) {
    let code = match strip_strings(expr) {
        Ok(code) => code,
        Err(message) => {
            problems.push(format!(
                "the rendered expression is not valid LogQL: {message}"
            ));
            return;
        }
    };
    if let Err(message) = check_balanced(&code) {
        problems.push(format!(
            "the rendered expression is not valid LogQL: {message}"
        ));
    }
    // On the expression with its strings emptied: `|= "[5m]"` is a line
    // filter that happens to look like a range, not a range.
    if !code.contains('{') {
        problems.push("the expression has no stream selector (`{…}`)".into());
    }
    if !LOGQL_RANGE.is_match(&code) {
        problems.push(
            "the expression has no range (such as `[5m]`), so it is a log query rather than a \
             metric query, and a ruler evaluates only the second. The range is how long a matching \
             line keeps the alert firing; write it out."
                .into(),
        );
    }
    check_tokens(expr, problems);
}

/// `expr` with the contents of every string literal removed, keeping the
/// quotes, so what is left is the expression's structure. LogQL strings are
/// `"…"` with backslash escapes, or raw `` `…` ``.
fn strip_strings(expr: &str) -> Result<String, String> {
    let mut out = String::with_capacity(expr.len());
    let mut chars = expr.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push_str("\"\"");
                loop {
                    match chars.next() {
                        Some('\\') => {
                            chars.next();
                        }
                        Some('"') => break,
                        Some(_) => {}
                        None => return Err("a `\"` string is not closed".into()),
                    }
                }
            }
            '`' => {
                out.push_str("``");
                if !chars.any(|c| c == '`') {
                    return Err("a `` ` `` string is not closed".into());
                }
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

/// That every bracket in `code`, an expression with its strings emptied by
/// [`strip_strings`], closes.
fn check_balanced(code: &str) -> Result<(), String> {
    let mut open: Vec<char> = Vec::new();
    for c in code.chars() {
        match c {
            '(' | '{' | '[' => open.push(c),
            ')' | '}' | ']' => {
                let expected = match c {
                    ')' => '(',
                    '}' => '{',
                    _ => '[',
                };
                if open.pop() != Some(expected) {
                    return Err(format!("unbalanced `{c}`"));
                }
            }
            _ => {}
        }
    }
    match open.last() {
        Some(c) => Err(format!("`{c}` is not closed")),
        None => Ok(()),
    }
}

/// Checks on what remains of the templating in a rendered expression, which no
/// ruler resolves whatever its language.
fn check_tokens(expr: &str, problems: &mut Vec<String>) {
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
fn rule_file_header(engine: RuleEngine) -> String {
    format!(
        "\
# Generated by `mz-monitoring-build gen-rules` from packages/queries/. DO NOT EDIT.
#
# Tokens of the form __mzmon_*__ are replaced by the chart at install time
# ({}); which rules install is decided there
# from ../_index.yaml.
",
        engine.template()
    )
}

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
    engine: &'static str,
    group: &'a str,
    severity: &'a str,
    audience: &'a str,
    requires: Vec<&'static str>,
    enabled_by_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_importance: Option<String>,
}

impl RuleSet {
    /// The source file stems with rules for `engine`, in output order.
    pub fn sources(&self, engine: RuleEngine) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for rule in self.rules.iter().filter(|r| r.engine == engine) {
            if !seen.contains(&rule.source.as_str()) {
                seen.push(&rule.source);
            }
        }
        seen
    }

    /// The `groups:` document for one source file's `engine` rules.
    pub fn rule_file_yaml(
        &self,
        engine: RuleEngine,
        source: &str,
    ) -> serde_yaml_ng::Result<String> {
        let mut groups: IndexMap<&str, Vec<RuleDoc>> = IndexMap::new();
        for rule in self
            .rules
            .iter()
            .filter(|r| r.engine == engine && r.source == source)
        {
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
            "{}{}",
            rule_file_header(engine),
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
                            file: format!("{}/{}.yaml", rule.engine.dir(), rule.source),
                            engine: rule.engine.as_str(),
                            group: &rule.group,
                            severity: rule.labels.get("severity").map_or("", String::as_str),
                            audience: rule.labels.get("audience").map_or("", String::as_str),
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
# What the chart needs to decide which rules install: each rule's engine (which
# ruler evaluates it, and so which directory its file is in), its capabilities
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
        let yaml = format!(
            "description: test\nmetricImportanceHint: essential\nalertLabels: {{audience: platform}}\nalerts:\n{alerts_yaml}"
        );
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

        let file = set
            .rule_file_yaml(RuleEngine::PromQl, "test-alerts")
            .unwrap();
        assert!(file.starts_with("# Generated by"));
        assert!(file.contains("- name: test_group"));
        let index = set.index_yaml().unwrap();
        assert!(index.contains("file: prometheus/test-alerts.yaml"));
        assert!(index.contains("engine: promql"));
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
    fn audience_comes_from_the_file_unless_the_alert_sets_it() {
        let yaml = format!(
            "{}{}",
            alert("from-file", LABELS, "mz_a > 0", ""),
            alert(
                "own-audience",
                "{severity: warning, component: test, audience: workload}",
                "mz_b > 0",
                ""
            )
        );
        let set = render_rules(&registry(&yaml)).unwrap();
        assert_eq!(set.rules[0].labels["audience"], "platform");
        assert_eq!(set.rules[1].labels["audience"], "workload");
        assert!(set.index_yaml().unwrap().contains("audience: workload"));
    }

    #[test]
    fn audience_is_required_and_checked() {
        let doc = |labels: &str| {
            let yaml = format!(
                "description: test\nmetricImportanceHint: essential\nalerts:\n{}",
                alert("x", labels, "mz_a > 0", "")
            );
            let mut registry = QueryRegistry::new();
            registry
                .load_from(RegistryDoc::from_yaml_str(&yaml).unwrap(), Some("t"))
                .unwrap();
            render_rules(&registry)
                .expect_err("expected errors")
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
        };
        assert!(has(&doc(LABELS), "`audience` label is required"));
        assert!(has(
            &doc("{severity: warning, component: test, audience: everyone}"),
            "audience `everyone`"
        ));
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
            "{}{}{}{joined}",
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
            alert(
                "infra-oomkill",
                LABELS,
                r#"kube_pod_container_status_restarts_total{container=~"%%{infraCoreWorkloadList}|%%{infraImportantWorkloadList}|%%{infraDaemonsetWorkloadList}", container!~"%%{infraNonessentialWorkloadList}"} > 0"#,
                ""
            ),
        );
        let set = render_rules(&registry(&yaml)).unwrap();
        let file = set
            .rule_file_yaml(RuleEngine::PromQl, "test-alerts")
            .unwrap();
        for placeholder in Placeholder::ALL {
            assert!(file.contains(placeholder.token()), "{placeholder:?} unused");
        }
        crate::scrape::test_support::assert_promtool_rules_ok("test-alerts", &file);
    }

    /// The real registry renders without a single error, and every file it
    /// produces is one promtool accepts.
    #[test]
    fn the_registry_renders_and_promtool_accepts_it() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../queries");
        let registry = QueryRegistry::from_directory(&dir).expect("registry loads");
        let set = render_rules(&registry).unwrap_or_else(|errors| {
            let listed: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            panic!("the registry has rule errors:\n{}", listed.join("\n"))
        });
        assert!(set.rules.len() > 50, "expected the whole alert set");
        assert!(set.rules.iter().any(|r| r.enabled_by_default));
        for source in set.sources(RuleEngine::PromQl) {
            let file = set.rule_file_yaml(RuleEngine::PromQl, source).unwrap();
            crate::scrape::test_support::assert_promtool_rules_ok(source, &file);
        }
        let log_rules: Vec<&RenderedRule> = set
            .rules
            .iter()
            .filter(|r| r.engine == RuleEngine::LogQl)
            .collect();
        assert!(!log_rules.is_empty(), "expected the log-derived alerts");
        for rule in log_rules {
            assert_logcli_parses(&rule.alert, &rule.expr);
        }
    }

    // --- LogQL ----------------------------------------------------------------

    fn log_alert(name: &str, logql: &str, extra: &str) -> String {
        format!(
            r#"  - alert: {name}
    group: test_logs
    stability: best-effort
    for: 0s
    labels: {LABELS}
    description:
      summary: Something was logged.
{extra}    query:
      id: test.{id}
      stability: best-effort
      description:
        summary: q
      logQL: '{logql}'
"#,
            id = name.replace('-', "_")
        )
    }

    /// Parse `expr` with Loki's own LogQL parser, through `logcli`'s offline
    /// `--stdin` mode, when it is installed; `make rules-check` does the same
    /// in CI. A metric query parses and is then refused as unsupported over
    /// stdin, and that refusal is what distinguishes it from a log query, which
    /// runs and which a ruler would refuse.
    fn assert_logcli_parses(name: &str, expr: &str) {
        use std::process::{Command, Stdio};
        let Ok(output) = Command::new("logcli")
            .args(["query", "--stdin", "--quiet", expr])
            .stdin(Stdio::null())
            .output()
        else {
            eprintln!("skipping LogQL parse check for {name}: `logcli` not found on PATH");
            return;
        };
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("parse error"),
            "Loki's parser rejected {name}: {stderr}\n{expr}"
        );
        assert!(
            stderr.contains("Query: not supported"),
            "Loki parsed {name} as a log query, not a metric query: {stderr}\n{expr}"
        );
    }

    #[test]
    fn renders_a_log_rule_for_the_loki_ruler() {
        let set = render_rules(&registry(&log_alert(
            "thing-panicked",
            r#"sum by (namespace, pod) (count_over_time({%%{mzEnvironmentNamespaceFilter}, level="CRITICAL"} | panic_location != "" [5m])) > 0"#,
            "    enabledByDefault: true\n    requires: [materialize]\n",
        )))
        .unwrap();
        let rule = &set.rules[0];
        assert_eq!(rule.engine, RuleEngine::LogQl);
        assert_eq!(
            rule.expr,
            r#"sum by (namespace, pod) (count_over_time({namespace=~"__mzmon_environment_namespaces__", level="CRITICAL"} | panic_location != "" [5m])) > 0"#
        );
        // Nothing to infer from: what it declares is what it requires.
        assert_eq!(
            rule.requires.iter().collect::<Vec<_>>(),
            vec![&Capability::Materialize]
        );
        assert_eq!(rule.min_importance, None);

        assert!(set.sources(RuleEngine::PromQl).is_empty());
        assert_eq!(set.sources(RuleEngine::LogQl), vec!["test-alerts"]);
        let file = set
            .rule_file_yaml(RuleEngine::LogQl, "test-alerts")
            .unwrap();
        assert!(file.contains("templates/alerts/lokirules.yaml"));
        assert!(file.contains("- name: test_logs"));
        let index = set.index_yaml().unwrap();
        assert!(index.contains("file: loki/test-alerts.yaml"), "{index}");
        assert!(index.contains("engine: logql"), "{index}");
        assert_logcli_parses(&rule.alert, &rule.expr);
    }

    #[test]
    fn a_log_rule_needs_no_requires() {
        // Unlike a PromQL rule reading only `up`, there is no metric to say
        // what it needs; the chart gates it on the Loki ruler instead.
        let set = render_rules(&registry(&log_alert(
            "x",
            r#"sum(count_over_time({%%{mzEnvironmentNamespaceFilter}} |= "boom" [5m])) > 0"#,
            "",
        )))
        .unwrap();
        assert!(set.rules[0].requires.is_empty());
    }

    #[test]
    fn a_log_query_is_not_a_rule() {
        let errs = errors(&log_alert(
            "x",
            r#"{%%{mzEnvironmentNamespaceFilter}} |= "boom""#,
            "",
        ));
        assert!(
            has(&errs, "log query rather than a metric query"),
            "{errs:?}"
        );
    }

    #[test]
    fn grafana_ranges_and_viewer_pickers_are_refused_in_log_rules() {
        // The clicked-in rules' shape: `$__range` is Grafana's, and a Loki
        // ruler cannot parse it.
        let errs = errors(&log_alert(
            "x",
            r#"sum(count_over_time({%%{mzEnvironmentNamespaceFilter}} |~ "(?i)panic" [$__range])) > 0"#,
            "",
        ));
        assert!(has(&errs, "Grafana variable"), "{errs:?}");
        assert!(has(&errs, "no range"), "{errs:?}");

        let errs = errors(&log_alert(
            "x",
            r#"sum(count_over_time({%%{mzLogNamespaceFilter}} |= "boom" [5m])) > 0"#,
            "",
        ));
        assert!(has(&errs, "rendering failed"), "{errs:?}");
    }

    #[test]
    fn unbalanced_log_rules_are_errors() {
        let errs = errors(&log_alert(
            "x",
            r#"sum(count_over_time({%%{mzEnvironmentNamespaceFilter}} |= "a)" [5m]) > 0"#,
            "",
        ));
        assert!(has(&errs, "not valid LogQL"), "{errs:?}");
        // A bracket inside a string is not one.
        let balanced = |e: &str| strip_strings(e).and_then(|code| check_balanced(&code));
        assert!(balanced(r#"sum(count_over_time({a="b"} |~ "[(" [5m]))"#).is_ok());
        assert!(balanced(r#"sum(count_over_time({a="b"} |~ `\)` [5m]))"#).is_ok());
        assert!(balanced(r#"sum(count_over_time({a="b\"} [5m]))"#).is_err());
    }

    #[test]
    fn a_range_inside_a_string_is_not_a_range() {
        // A log query whose line filter looks like a range parses, and the
        // ruler refuses it.
        let errs = errors(&log_alert(
            "x",
            r#"{%%{mzEnvironmentNamespaceFilter}} |= "[5m]""#,
            "",
        ));
        assert!(
            has(&errs, "log query rather than a metric query"),
            "{errs:?}"
        );
    }

    #[test]
    fn a_file_is_promql_or_logql_not_both() {
        let yaml = format!(
            "{}{}",
            alert("metric-alert", LABELS, "mz_a > 0", ""),
            log_alert(
                "log-alert",
                r#"sum(count_over_time({%%{mzEnvironmentNamespaceFilter}} |= "boom" [5m])) > 0"#,
                ""
            ),
        );
        let errs = render_rules(&registry(&yaml)).expect_err("mixed file");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].alert, "log-alert");
        assert!(errs[0].message.contains("holds PromQL and LogQL alerts"));
    }

    #[test]
    fn a_query_is_promql_or_logql_not_both() {
        let yaml = r#"  - alert: x
    group: test_group
    stability: best-effort
    for: 5m
    labels: {severity: warning, component: test}
    description:
      summary: s
    query:
      id: test.both
      stability: best-effort
      description:
        summary: q
      promQL: 'mz_thing > 0'
      logQL: 'sum(count_over_time({namespace="a"} [5m])) > 0'
"#;
        assert!(has(&errors(yaml), "both a PromQL and a LogQL expression"));
    }

    #[test]
    fn each_engine_writes_its_own_file_and_one_index_lists_both() {
        let mut registry = registry(&alert("metric-alert", LABELS, "mz_a > 0", ""));
        let logs = format!(
            "description: test\nmetricImportanceHint: essential\nalertLabels: {{audience: platform}}\nalerts:\n{}",
            log_alert(
                "log-alert",
                r#"sum(count_over_time({%%{mzEnvironmentNamespaceFilter}} |= "boom" [5m])) > 0"#,
                ""
            )
        );
        registry
            .load_from(
                RegistryDoc::from_yaml_str(&logs).unwrap(),
                Some("test-log-alerts"),
            )
            .unwrap();
        let set = render_rules(&registry).unwrap();
        assert_eq!(set.sources(RuleEngine::PromQl), vec!["test-alerts"]);
        assert_eq!(set.sources(RuleEngine::LogQl), vec!["test-log-alerts"]);
        let prom = set
            .rule_file_yaml(RuleEngine::PromQl, "test-alerts")
            .unwrap();
        let loki = set
            .rule_file_yaml(RuleEngine::LogQl, "test-log-alerts")
            .unwrap();
        assert!(prom.contains("metric-alert") && !prom.contains("log-alert"));
        assert!(loki.contains("log-alert") && !loki.contains("metric-alert"));
        let index = set.index_yaml().unwrap();
        assert!(index.contains("file: loki/test-log-alerts.yaml"), "{index}");
        assert!(index.contains("metric-alert:") && index.contains("log-alert:"));
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
            one.rule_file_yaml(RuleEngine::PromQl, "test-alerts")
                .unwrap(),
            two.rule_file_yaml(RuleEngine::PromQl, "test-alerts")
                .unwrap()
        );
        // Registration order, not alphabetical.
        assert_eq!(one.rules[0].alert, "b-alert");
    }
}
