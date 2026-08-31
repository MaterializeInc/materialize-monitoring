// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Grafana-flavored render context.
//!
//! [`crate::query::render`] already ships two contexts, both extraction-flavored:
//! `doc_context` and `tier_context` fill parameters with recognizable sentinels
//! (`[42m]`, `your-env-name`) because their job is to be *parsed*, not displayed.
//! This one is for rendering into a dashboard, so parameters resolve to Grafana
//! built-ins (`$__rate_interval`, `$__range`) and dashboard variables
//! (`$environmentNameList`, `$mzClusterList`), and the enrichment functions do the
//! real catalog joins rather than the identity.
//!
//! **Every `$variable` referenced here must be defined by the dashboard.** An
//! undefined Grafana variable interpolates to nothing and the selector silently
//! matches no series — a panel that looks fine and is empty. [`REQUIRED_VARIABLES`]
//! is the list to assert against a dashboard's variable set; that is why the
//! system namespace below is a plain value rather than a `$…` reference, since no
//! dashboard defines a variable for it.
//!
//! The operator namespace is the one that goes both ways. It defaults to a plain
//! value for the same reason, but a dashboard that defines
//! [`variable::operator_namespace`](crate::grafana::variable::operator_namespace)
//! sets [`DashboardScope::operator_namespace`] to `$operatorNamespace` and gets a
//! control instead of a constant. That is a property of the scope rather than of
//! the parameter, so the two kinds of dashboard coexist without either knowing
//! about the other.

use std::collections::HashMap;

use crate::query::enrich;
use crate::query::render::{TemplateFn, promql_or_zero};
use crate::query::{QueryEngine, QueryRegistry, TemplateContext};

/// Grafana dashboard variables the rendered queries reference.
pub mod variables {
    /// Metrics (Prometheus/Thanos) datasource.
    pub const METRICS_DATASOURCE: &str = "metricsDatasource";
    /// Selected environments, by name.
    ///
    /// Holds `materialize_cloud_organization_name` values and is matched against
    /// that label. Self-managed Materialize has no cloud organization id at all,
    /// so the name is the only stable identifier — the reverse of
    /// [`MZ_CLUSTER_LIST`], where the id is the value precisely because cluster
    /// names are neither stable nor unique.
    ///
    /// The Python still emits this as `environmentIdList`; the two diverge until
    /// the Rust implementation takes over rendering, and a dashboard rendered by
    /// each is not permalink-compatible with the other.
    pub const ENVIRONMENT_NAME_LIST: &str = "environmentNameList";
    /// Materialize namespaces the selected environments live in.
    pub const MZ_NAMESPACE_LIST: &str = "mzNamespaceList";
    /// Selected clusters within the environment.
    pub const MZ_CLUSTER_LIST: &str = "mzClusterList";
    /// Selected replicas within the selected clusters.
    pub const MZ_REPLICA_LIST: &str = "mzReplicaList";
    /// Namespace the Materialize operator (orchestratord) runs in.
    ///
    /// Optional, unlike the four above: only a dashboard that scopes itself to the
    /// operator defines it, and [`super::DashboardScope::operator_namespace`] is a
    /// plain value everywhere else. Not a `*List` because it is single-select —
    /// see [`variable::operator_namespace`](crate::grafana::variable::operator_namespace).
    pub const OPERATOR_NAMESPACE: &str = "operatorNamespace";
    /// Logs (Loki) datasource.
    pub const LOGS_DATASOURCE: &str = "logsDatasource";
    /// Namespaces a logs dashboard reads from.
    ///
    /// Loki-discovered, unlike [`MZ_NAMESPACE_LIST`], which comes from the metrics
    /// side. A logs dashboard is answering "why is this broken", and deriving its
    /// own scope from the metrics pipeline would make it depend on the thing it is
    /// often being used to investigate.
    pub const LOG_NAMESPACE_LIST: &str = "logNamespaceList";
    /// Applications a logs dashboard reads from.
    pub const LOG_APP_LIST: &str = "logAppList";
    /// Severity levels a logs dashboard includes.
    pub const LOG_LEVEL_LIST: &str = "logLevelList";
    /// Sub-components a logs dashboard reads from.
    pub const LOG_COMPONENT_LIST: &str = "logComponentList";
    /// Containers a logs dashboard reads from.
    pub const LOG_CONTAINER_LIST: &str = "logContainerList";
    /// systemd units a node-journal panel reads from.
    pub const LOG_UNIT_LIST: &str = "logUnitList";
    /// Collection jobs a logs dashboard reads from.
    ///
    /// Also the matcher that keeps a log stream selector parseable — see
    /// [`variable::log_jobs`](crate::grafana::variable::log_jobs).
    pub const LOG_JOB_LIST: &str = "logJobList";
    /// Selected deployment generations, for blue/green.
    ///
    /// Optional, like [`OPERATOR_NAMESPACE`]: only a dashboard that reasons about
    /// rollouts defines it. Held as generation *numbers*, which reach a query as a
    /// fragment of a name pattern rather than as a label value — see
    /// [`variable::generations`](crate::grafana::variable::generations).
    pub const MZ_GENERATION_LIST: &str = "mzGenerationList";
    /// Selected node-exporter instances.
    ///
    /// Unlike the others, this one is not supplied by any parameter: the
    /// node-exporter query families write `instance=~"$nodeList"` literally in
    /// their templates (220 occurrences across `node-health.yaml` and
    /// `node-debug.yaml`), a convention inherited from the Node Exporter Full
    /// dashboard. A dashboard that uses those queries must define it; nothing in
    /// the render context can.
    pub const NODE_LIST: &str = "nodeList";
}

/// Variables a dashboard must define for [`dashboard_context`] to render usefully.
///
/// Not including the datasource variable: that is referenced by the dataquery's
/// `datasource` field (see [`crate::grafana::query`]), not by a rendered
/// expression.
pub const REQUIRED_VARIABLES: &[&str] = &[
    variables::ENVIRONMENT_NAME_LIST,
    variables::MZ_NAMESPACE_LIST,
    variables::MZ_CLUSTER_LIST,
    variables::MZ_REPLICA_LIST,
];

/// Variables required only by the node-exporter query families, which reference
/// them literally rather than through a parameter.
///
/// No dashboard defines `nodeList` today — the pre-rendered `env-top` defines the
/// four in [`REQUIRED_VARIABLES`] plus `includeSystemClusters`, `metricAdhoc` and
/// the datasource — so a node dashboard has to add it. Kept separate so a
/// dashboard that uses no node queries is not asked for a variable it has no use
/// for.
///
/// **The node families need vetting before a dashboard uses them.** `node-health`
/// and `node-debug` were authored separately from the dashboards, so their
/// conventions were not held to the same standard as the Materialize families —
/// treat their content as unreviewed rather than as a baseline to build on.
pub const NODE_VARIABLES: &[&str] = &[variables::NODE_LIST];

/// Variables required only by a dashboard that scopes itself to the operator with
/// [`DashboardScope::operator_variable`].
///
/// Separate from [`REQUIRED_VARIABLES`] because a dashboard that leaves the
/// operator namespace pinned never references it — `mzDeploymentNamespaceFilter`
/// then renders the pinned value on the left of the alternation and no `$…` at
/// all.
pub const OPERATOR_VARIABLES: &[&str] = &[variables::OPERATOR_NAMESPACE];

/// Variables required only by a dashboard that filters by deployment generation.
///
/// Unlike [`OPERATOR_VARIABLES`], no scope flag gates these: the two generation
/// parameters always reference `$mzGenerationList`, so a dashboard using a query
/// that names one must define it. Only queries about rollouts do.
pub const GENERATION_VARIABLES: &[&str] = &[variables::MZ_GENERATION_LIST];

/// Variables required only by a logs dashboard.
///
/// Its scope is Loki-discovered end to end, so it shares none of
/// [`REQUIRED_VARIABLES`] — a dashboard defining these need not define those, and
/// vice versa.
pub const LOG_VARIABLES: &[&str] = &[
    variables::LOG_NAMESPACE_LIST,
    variables::LOG_APP_LIST,
    variables::LOG_LEVEL_LIST,
    variables::LOG_JOB_LIST,
    variables::LOG_COMPONENT_LIST,
    variables::LOG_CONTAINER_LIST,
    variables::LOG_UNIT_LIST,
];

/// The object-name pattern a generation appears in, as a regex with the
/// generation itself left as `{}`.
///
/// Two shapes, because orchestratord names the two workloads differently:
/// `…-environmentd-<generation>-<ordinal>` and, for a replica,
/// `…-gen-<generation>-<ordinal>`. Both are matched, so one filter covers
/// environmentd and clusterd rather than the caller picking.
///
/// The generation is deliberately *not* a label anywhere — orchestratord records
/// it as a Kubernetes annotation, which neither kube-state-metrics nor the event
/// pipeline surfaces. The name is the only place it reaches a query.
const GENERATION_NAME_PATTERN: &str = r".*-(environmentd|gen)-({})-[0-9]+";

/// The same pattern as a *capture*, for `label_replace` to lift the generation
/// out of a pod name into a label of its own.
///
/// Non-capturing on the workload alternation so `$1` is the generation and
/// nothing else. Kept beside [`GENERATION_NAME_PATTERN`] because the two must
/// agree: a filter that admits a pod shape the capture cannot parse produces
/// series with an empty `generation` label, which reads as a legend of blanks.
const GENERATION_CAPTURE_PATTERN: &str = r".*-(?:environmentd|gen)-([0-9]+)-[0-9]+";

/// Render a scope's namespace value as a regex-alternation fragment.
///
/// A pinned namespace is a literal and goes in as it is. A `$variable` needs
/// Grafana's `:regex` format for the same reason the namespace list does — see
/// `mzDeploymentNamespaceFilter` in [`dashboard_context`] — and the format
/// suffix has to go inside the braces, so `$name` becomes `${name:regex}`.
fn regex_form(namespace: &str) -> String {
    match namespace.strip_prefix('$') {
        Some(name) => format!("${{{name}:regex}}"),
        None => namespace.to_string(),
    }
}

/// [`GENERATION_NAME_PATTERN`] with the selected generations spliced in.
fn generation_pattern_for_selection() -> String {
    GENERATION_NAME_PATTERN.replace(
        "{}",
        &format!("${{{}:regex}}", variables::MZ_GENERATION_LIST),
    )
}

/// The `%%{interval}` window for `engine`, as that datasource spells it.
///
/// Not cosmetic: an engine handed the other's spelling passes it through to the
/// backend as literal text, and both backends reject it as a duration. The engines
/// with no Grafana datasource never reach a dataquery (see
/// [`crate::grafana::query::data_query`]), so what they get here only has to be
/// something that renders.
fn rate_interval(engine: QueryEngine) -> &'static str {
    match engine {
        QueryEngine::LogQl => "[$__auto]",
        _ => "[$__rate_interval]",
    }
}

/// The PromQL fragment scoping a query to the selected environment(s).
///
/// Inlined rather than routed through a Grafana constant variable: constant
/// interpolation does not recursively resolve the nested `$environmentNameList`
/// reference, and it mangles the embedded commas and quotes when spliced into a
/// label matcher.
pub fn environment_filter() -> String {
    format!(
        r#"materialize_cloud_organization_name=~"${}""#,
        variables::ENVIRONMENT_NAME_LIST
    )
}

/// Deployment-specific values a dashboard context needs that are not dashboard
/// variables.
#[derive(Debug, Clone)]
pub struct DashboardScope {
    /// `%%{mzSqlPrefix}` — `mz_` on self-managed, `v2_mz_` on Cloud. Not a
    /// version: the two prefixes are two deployments of the SQL-based metric
    /// endpoints.
    pub sql_metric_prefix: String,
    /// Namespace selector value for the Materialize operator.
    ///
    /// A plain value by default, since referencing a variable no dashboard defines
    /// would match nothing. A dashboard that defines
    /// [`variable::operator_namespace`](crate::grafana::variable::operator_namespace)
    /// sets this to `$operatorNamespace` — see [`Self::operator_variable`].
    pub operator_namespace: String,
    /// Namespace selector value for system / infrastructure components.
    pub system_namespace: String,
    /// Optional environment-exclusion fragment appended to selectors, to drop
    /// environments a deployment does not want to see (e.g.
    /// `mz_context_org_type!="e2e_test"`). Empty excludes nothing.
    pub exclude_environments: String,
}

impl Default for DashboardScope {
    fn default() -> Self {
        DashboardScope {
            sql_metric_prefix: "mz_".to_string(),
            operator_namespace: "materialize".to_string(),
            system_namespace: "kube-system".to_string(),
            exclude_environments: String::new(),
        }
    }
}

impl DashboardScope {
    /// Self-managed defaults.
    pub fn self_managed() -> Self {
        Self::default()
    }

    /// Cloud, where the SQL-exporter metrics carry the `v2_mz_` prefix.
    pub fn cloud() -> Self {
        DashboardScope {
            sql_metric_prefix: "v2_mz_".to_string(),
            ..Self::default()
        }
    }

    /// Point the operator-namespace parameter at the `$operatorNamespace`
    /// variable rather than pinning a namespace.
    ///
    /// Only for a dashboard whose variable set includes
    /// [`variable::operator_namespace`](crate::grafana::variable::operator_namespace);
    /// on any other it renders a selector that matches nothing.
    pub fn operator_variable(mut self) -> Self {
        self.operator_namespace = format!("${}", variables::OPERATOR_NAMESPACE);
        self
    }

    /// Whichever of the above matches `prefix`.
    ///
    /// The prefix reaches a dashboard as a plain string from the command line, so
    /// this is the seam between "what was asked for" and the two known scopes.
    /// An unrecognized prefix is carried through rather than rejected: it is a
    /// metric-name prefix, and a deployment may legitimately have its own.
    pub fn for_prefix(prefix: &str) -> Self {
        DashboardScope {
            sql_metric_prefix: prefix.to_string(),
            ..Self::default()
        }
    }
}

/// Build the render context for a Grafana dashboard.
///
/// Only [`QueryEngine::PromQl`] and [`QueryEngine::LogQl`] make sense here — the
/// other engines have no Grafana datasource (see [`crate::grafana::query`]) — but
/// the engine is not restricted, so a caller can render a Datadog expression for
/// display without going through a dataquery.
pub fn dashboard_context<'a>(
    registry: &'a QueryRegistry,
    engine: QueryEngine,
    scope: &DashboardScope,
) -> TemplateContext<'a> {
    let env_filter = environment_filter();

    let namespace_selector = format!(r#"namespace=~"${}""#, variables::MZ_NAMESPACE_LIST);
    let parameters = [
        // Grafana built-ins, and the one place the two engines genuinely differ:
        // `$__rate_interval` is a *Prometheus* datasource variable. Grafana's Loki
        // datasource does not define it, so it reaches Loki uninterpolated and the
        // query fails to parse — "not a valid duration string" — rather than
        // rendering empty. Loki's equivalent is `$__auto`, which resolves to the
        // step for a range query and to the selected range for an instant one.
        //
        // Both adapt to the panel's resolution, which is what a dashboard wants
        // where the extraction contexts hardcode a window.
        ("interval", rate_interval(engine).to_string()),
        // `$__range` is defined by both, and means the same thing in each.
        ("range", "[$__range]".to_string()),
        // The bare window, for a subquery. `range` carries its own brackets, so
        // `%%{range}:1m` would render `[$__range]:1m` -- not valid PromQL. A
        // subquery in a dashboard panel wants the panel's own range, so this is
        // what lets a query follow the time picker instead of hardcoding a window.
        ("rangeWindow", "$__range".to_string()),
        ("mzSqlPrefix", scope.sql_metric_prefix.clone()),
        ("mzEnvironmentFilter", env_filter.clone()),
        ("mzEnvironmentNamespaceFilter", namespace_selector.clone()),
        (
            "mzOperatorNamespaceFilter",
            format!(r#"namespace=~"{}""#, scope.operator_namespace),
        ),
        (
            "mzSystemNamespaceFilter",
            format!(r#"namespace=~"{}""#, scope.system_namespace),
        ),
        // Both namespaces in one matcher. Writing the operator and environment
        // filters side by side would repeat the `namespace` label in one
        // selector, which is an AND: a namespace cannot be two things at once, so
        // it matches nothing and the panel is silently empty.
        //
        // The `:regex` forms are load-bearing here in a way they are not for the
        // two single filters. Each of those *is* the whole matcher, where
        // Grafana's default multi-value interpolation (`{a,b}`) still works; as a
        // fragment of a larger alternation it is a glob, which means nothing to a
        // regex engine.
        (
            "mzDeploymentNamespaceFilter",
            format!(
                r#"namespace=~"{}|${{{}:regex}}""#,
                regex_form(&scope.operator_namespace),
                variables::MZ_NAMESPACE_LIST
            ),
        ),
        ("mzClusterList", format!("${}", variables::MZ_CLUSTER_LIST)),
        ("mzReplicaList", format!("${}", variables::MZ_REPLICA_LIST)),
        // Regex-escaped forms, for a variable interpolated as a *fragment* of a
        // larger regex (`pod=~".*-cluster-%%{mzClusterListRegex}-replica-..."`).
        // Grafana's default interpolation of a multi-valued variable is
        // `{a,b}` -- a glob, meaningless inside a regex -- while `:regex` yields
        // `(a|b)` and escapes metacharacters. Where the variable *is* the whole
        // matcher (`instance_id=~"$mzClusterList"`) either form works, so the
        // plain ones above stay as they are.
        (
            "mzClusterListRegex",
            format!("${{{}:regex}}", variables::MZ_CLUSTER_LIST),
        ),
        (
            "mzReplicaListRegex",
            format!("${{{}:regex}}", variables::MZ_REPLICA_LIST),
        ),
        (
            "mzNamespaceList",
            format!("${}", variables::MZ_NAMESPACE_LIST),
        ),
        // Order matches the `container_filter` helper the dashboards use: extra
        // matchers first, then the two that drop the empty-name cgroup series and
        // the pause container.
        (
            "cAdvisorFilter",
            format!(r#"{namespace_selector},container!="",container!="POD""#),
        ),
        (
            "excludeEnvironmentFilter",
            scope.exclude_environments.clone(),
        ),
        // Generation, which is a *name* pattern rather than a label matcher --
        // see `GENERATION_NAME_PATTERN`. `:regex` for the same reason as the
        // cluster and replica forms: the value is a fragment of a larger regex,
        // and Grafana's default multi-value interpolation is a glob.
        (
            "mzGenerationFilter",
            format!(r#"pod=~"{}""#, generation_pattern_for_selection()),
        ),
        // The same idea for events, where the generation is in the object's name
        // and the filter is a pipeline stage rather than a stream selector.
        //
        // The `or` is load-bearing. Only a handful of the objects a rollout
        // touches carry a generation at all -- on this deployment, 6 of 70 event
        // names -- and every operator lifecycle event is filed against the
        // `Materialize` resource, which carries none. A bare `name=~` would drop
        // the entire narrative and keep the pod noise. So: keep what belongs to a
        // selected generation, and keep what belongs to no generation.
        // The capture form, for a query that needs the generation as a *label*.
        // A parameter rather than a template function because the `label_replace`
        // has to wrap an inner selector, while a function wraps the whole
        // template -- `label_replace(count by (generation) (...))` would be
        // backwards.
        // Log scope, all three Loki-discovered. Separate from the metric
        // namespace filter: a logs dashboard is often the tool for working out
        // why the metrics pipeline is broken, so it must not depend on it.
        (
            "mzLogNamespaceFilter",
            format!(r#"namespace=~"${}""#, variables::LOG_NAMESPACE_LIST),
        ),
        (
            "mzLogAppFilter",
            format!(r#"app=~"${}""#, variables::LOG_APP_LIST),
        ),
        (
            "mzLogLevelFilter",
            format!(r#"level=~"${}""#, variables::LOG_LEVEL_LIST),
        ),
        // Load-bearing beyond being a filter: LogQL rejects a stream selector
        // whose every matcher can match the empty string, and a dashboard of
        // `=~` pickers is exactly that. This one's "All" is `.+` rather than the
        // discovered values, so it always contributes a non-empty matcher and the
        // selector parses whatever the other pickers are set to. Log queries only
        // -- the event queries pin `job="loki.source.kubernetes_events"`, which
        // already anchors them, and a second `job` matcher would AND with it.
        (
            "mzLogComponentFilter",
            format!(r#"component=~"${}""#, variables::LOG_COMPONENT_LIST),
        ),
        (
            "mzLogContainerFilter",
            format!(r#"container=~"${}""#, variables::LOG_CONTAINER_LIST),
        ),
        // Journal lines carry no namespace, so `unit` is their anchor the way
        // `job` is for container logs -- hence `.+` for its "All".
        (
            "mzLogUnitFilter",
            format!(r#"unit=~"${}""#, variables::LOG_UNIT_LIST),
        ),
        (
            "mzLogJobFilter",
            format!(r#"job=~"${}""#, variables::LOG_JOB_LIST),
        ),
        // A line filter rather than a label matcher, so it goes after a `|` in the
        // pipeline. Empty is the resting state: `|~ "(?i)"` matches every line,
        // which is what keeps an untouched search box from blanking the panel.
        ("mzLogSearchFilter", r#"|~ "(?i)$logSearch""#.to_string()),
        (
            "mzGenerationPattern",
            GENERATION_CAPTURE_PATTERN.to_string(),
        ),
        (
            "mzGenerationEventFilter",
            format!(
                r#"name=~"{}" or name!~"{}""#,
                generation_pattern_for_selection(),
                GENERATION_NAME_PATTERN.replace("{}", "[0-9]+"),
            ),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();

    let mut functions: HashMap<String, TemplateFn> = HashMap::new();
    functions.insert("orZero".to_string(), Box::new(promql_or_zero));

    // Unlike the extraction contexts, these do the real catalog joins: a
    // dashboard legend should read `my_source`, not `u1234`.
    let env = env_filter.clone();
    functions.insert(
        "mzObjectName".to_string(),
        Box::new(move |base: &str, args: &[String]| {
            let id_label = args.first().map(String::as_str).unwrap_or("id");
            let extra = args.get(1).map(String::as_str);
            enrich::with_object_name(base, id_label, extra, &env)
        }),
    );

    let env = env_filter.clone();
    functions.insert(
        "mzClusterName".to_string(),
        Box::new(move |base: &str, args: &[String]| {
            let id_label = args.first().map(String::as_str).unwrap_or("instance_id");
            enrich::with_cluster_name(base, id_label, &env)
        }),
    );

    // `mzEnvironmentName` is declared by the registry and used by 25 queries, but
    // no implementation exists on either side: there is no verified info metric
    // mapping a namespace to an environment name, and the Python never exercised
    // it because its dashboards hand-write their PromQL. Left as the identity so
    // those queries still render -- their legends show the namespace rather than a
    // friendly name. Replace this once the info metric is settled; do not treat
    // the identity as intended behavior.
    functions.insert(
        "mzEnvironmentName".to_string(),
        Box::new(|base: &str, _args: &[String]| base.to_string()),
    );

    TemplateContext {
        engine,
        parameters,
        functions,
        registry: Some(registry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::render::doc_context;

    #[test]
    fn parameters_cover_every_known_parameter() {
        // The schema's `knownParameter` enum is the authoritative list; a missing
        // one is a render error the moment a query starts using it.
        const KNOWN: &[&str] = &[
            "interval",
            "range",
            "mzSqlPrefix",
            "cAdvisorFilter",
            "mzOperatorNamespaceFilter",
            "mzEnvironmentNamespaceFilter",
            "mzSystemNamespaceFilter",
            "mzDeploymentNamespaceFilter",
            "mzGenerationFilter",
            "mzGenerationEventFilter",
            "mzGenerationPattern",
            "mzLogNamespaceFilter",
            "mzLogAppFilter",
            "mzLogLevelFilter",
            "mzLogJobFilter",
            "mzLogComponentFilter",
            "mzLogContainerFilter",
            "mzLogUnitFilter",
            "mzLogSearchFilter",
            "mzEnvironmentFilter",
            "excludeEnvironmentFilter",
            "mzClusterList",
            "mzReplicaList",
            "mzNamespaceList",
        ];
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        let missing: Vec<_> = KNOWN
            .iter()
            .filter(|k| !ctx.parameters.contains_key(**k))
            .collect();
        assert!(missing.is_empty(), "unsupplied parameters: {missing:?}");
    }

    #[test]
    fn every_dollar_reference_is_a_grafana_builtin_or_a_required_variable() {
        // An undefined variable interpolates to nothing and the selector matches
        // no series, so this is the check that keeps a silently-empty panel from
        // shipping. Both engines, since they do not share every built-in.
        let registry = QueryRegistry::new();
        let builtins = ["__rate_interval", "__auto", "__range"];

        for engine in [QueryEngine::PromQl, QueryEngine::LogQl] {
            let ctx = dashboard_context(&registry, engine, &DashboardScope::default());
            for (name, value) in &ctx.parameters {
                for reference in dollar_references(value) {
                    let known = builtins.contains(&reference.as_str())
                        || REQUIRED_VARIABLES.contains(&reference.as_str())
                        || NODE_VARIABLES.contains(&reference.as_str())
                        || OPERATOR_VARIABLES.contains(&reference.as_str())
                        || GENERATION_VARIABLES.contains(&reference.as_str())
                        || LOG_VARIABLES.contains(&reference.as_str())
                        || reference == "logSearch";
                    assert!(
                        known,
                        "{engine} parameter {name} references unknown ${reference}"
                    );
                }
            }
        }
    }

    #[test]
    fn each_engine_gets_its_own_datasources_spelling_of_the_interval() {
        // `$__rate_interval` is a Prometheus datasource variable and `$__auto` is
        // Loki's. Handing an engine the other one does not degrade to an empty
        // panel: the literal text reaches the backend and is rejected as a
        // duration, so the panel errors.
        let registry = QueryRegistry::new();
        let scope = DashboardScope::default();

        let prom = dashboard_context(&registry, QueryEngine::PromQl, &scope);
        assert_eq!(prom.parameters["interval"], "[$__rate_interval]");

        let loki = dashboard_context(&registry, QueryEngine::LogQl, &scope);
        assert_eq!(loki.parameters["interval"], "[$__auto]");

        // `$__range` is spelled the same by both, so it is the one that must NOT
        // diverge.
        assert_eq!(prom.parameters["range"], loki.parameters["range"]);
    }

    #[test]
    fn the_interval_is_the_only_parameter_that_varies_by_engine() {
        // Everything else is a label matcher or a metric-name prefix, which the
        // engine has no bearing on. A second divergence should be a deliberate
        // edit here rather than a surprise.
        let registry = QueryRegistry::new();
        let scope = DashboardScope::default();
        let prom = dashboard_context(&registry, QueryEngine::PromQl, &scope);
        let loki = dashboard_context(&registry, QueryEngine::LogQl, &scope);

        let differing: Vec<&String> = prom
            .parameters
            .keys()
            .filter(|k| prom.parameters[*k] != loki.parameters[*k])
            .collect();
        assert_eq!(differing, vec!["interval"], "unexpected engine divergence");
    }

    /// Every `$name` in a string.
    fn dollar_references(value: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = value;
        while let Some(pos) = rest.find('$') {
            let after = &rest[pos + 1..];
            let len = after
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            // `$1` and friends are `label_replace` capture-group references --
            // PromQL syntax, not Grafana variables. The enrichment joins emit
            // them, and so do queries that do their own label_replace.
            if len > 0 && !after[..len].bytes().all(|b| b.is_ascii_digit()) {
                out.push(after[..len].to_string());
            }
            rest = &after[len..];
        }
        out
    }

    #[test]
    fn node_is_declared_but_not_supplied_by_any_parameter() {
        // The node families write `$nodeList` literally, so no parameter can carry
        // it; the point of declaring it is that a dashboard must define it.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        assert!(NODE_VARIABLES.contains(&"nodeList"));
        for value in ctx.parameters.values() {
            assert!(
                !dollar_references(value).contains(&"nodeList".to_string()),
                "no parameter should supply $nodeList"
            );
        }
    }

    #[test]
    fn the_operator_namespace_is_pinned_by_default_and_a_variable_on_request() {
        let registry = QueryRegistry::new();
        let pinned = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        assert_eq!(
            pinned.parameters["mzOperatorNamespaceFilter"],
            r#"namespace=~"materialize""#
        );

        let scope = DashboardScope::default().operator_variable();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &scope);
        assert_eq!(
            ctx.parameters["mzOperatorNamespaceFilter"],
            r#"namespace=~"$operatorNamespace""#
        );
        // The switch is scoped to that one parameter: the system namespace stays
        // pinned, since no dashboard defines a variable for it.
        assert_eq!(
            ctx.parameters["mzSystemNamespaceFilter"],
            pinned.parameters["mzSystemNamespaceFilter"]
        );
    }

    #[test]
    fn the_deployment_filter_is_one_matcher_over_both_namespaces() {
        let registry = QueryRegistry::new();
        let scope = DashboardScope::default().operator_variable();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &scope);
        assert_eq!(
            ctx.parameters["mzDeploymentNamespaceFilter"],
            r#"namespace=~"${operatorNamespace:regex}|${mzNamespaceList:regex}""#
        );
        // One `namespace=` matcher, not two: repeating the label ANDs it.
        assert_eq!(
            ctx.parameters["mzDeploymentNamespaceFilter"]
                .matches("namespace=")
                .count(),
            1
        );
    }

    #[test]
    fn the_deployment_filter_inlines_a_pinned_operator_namespace() {
        // With the operator namespace pinned there is no variable to format, and
        // the literal goes straight into the alternation.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        assert_eq!(
            ctx.parameters["mzDeploymentNamespaceFilter"],
            r#"namespace=~"materialize|${mzNamespaceList:regex}""#
        );
    }

    #[test]
    fn the_generation_filters_match_both_workload_naming_shapes() {
        // environmentd is `…-environmentd-<gen>-<ordinal>` and a replica is
        // `…-gen-<gen>-<ordinal>`. One filter has to cover both, or a tab about a
        // rollout shows environmentd and silently omits its replicas.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        let filter = &ctx.parameters["mzGenerationFilter"];
        assert_eq!(
            filter,
            r#"pod=~".*-(environmentd|gen)-(${mzGenerationList:regex})-[0-9]+""#
        );
    }

    #[test]
    fn the_generation_event_filter_keeps_objects_that_have_no_generation() {
        // Most objects a rollout touches carry no generation, and every operator
        // lifecycle event is filed against the `Materialize` resource, which
        // carries none. A bare `name=~` would drop the whole narrative and keep
        // the pod noise, so the `or` arm is the point of this parameter.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::LogQl, &DashboardScope::default());
        let filter = &ctx.parameters["mzGenerationEventFilter"];
        assert_eq!(
            filter,
            concat!(
                r#"name=~".*-(environmentd|gen)-(${mzGenerationList:regex})-[0-9]+""#,
                r#" or name!~".*-(environmentd|gen)-([0-9]+)-[0-9]+""#
            )
        );
        // The escape hatch is a negated matcher, not a second positive one --
        // two positives would keep everything and the control would do nothing.
        assert!(filter.contains(" or name!~"), "{filter}");
    }

    #[test]
    fn the_generation_filters_interpolate_as_a_regex_fragment() {
        // The value is spliced into a larger pattern, where Grafana's default
        // multi-value interpolation (`{a,b}`) is a glob and matches nothing.
        let registry = QueryRegistry::new();
        for engine in [QueryEngine::PromQl, QueryEngine::LogQl] {
            let ctx = dashboard_context(&registry, engine, &DashboardScope::default());
            for key in ["mzGenerationFilter", "mzGenerationEventFilter"] {
                let value = &ctx.parameters[key];
                assert!(
                    value.contains("${mzGenerationList:regex}"),
                    "{key} does not use the regex form: {value}"
                );
                assert!(
                    !value.contains("\"$mzGenerationList\""),
                    "{key} uses the plain form: {value}"
                );
            }
        }
    }

    #[test]
    fn the_cloud_scope_only_changes_the_sql_prefix() {
        let registry = QueryRegistry::new();
        let self_managed = dashboard_context(
            &registry,
            QueryEngine::PromQl,
            &DashboardScope::self_managed(),
        );
        let cloud = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::cloud());

        assert_eq!(self_managed.parameters["mzSqlPrefix"], "mz_");
        assert_eq!(cloud.parameters["mzSqlPrefix"], "v2_mz_");
        for (key, value) in &self_managed.parameters {
            if key != "mzSqlPrefix" {
                assert_eq!(&cloud.parameters[key], value, "{key} should not vary");
            }
        }
    }

    #[test]
    fn enrichment_functions_are_real_joins_not_the_identity() {
        // This is the substantive difference from the extraction contexts, where
        // all three are the identity.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());

        let cluster = ctx.functions["mzClusterName"]("up", &["instance_id".to_string()]);
        assert!(cluster.contains("mz_cluster_info"), "{cluster}");
        assert!(cluster.contains("group_left(cluster_name)"), "{cluster}");

        let object = ctx.functions["mzObjectName"]("up", &["sink_id".to_string()]);
        assert!(object.contains("mz_object_info"), "{object}");
        assert!(object.contains("global_id"), "{object}");

        // The doc context is the identity for the same names -- confirming the two
        // contexts genuinely differ rather than sharing a stub.
        let doc = doc_context(&registry, QueryEngine::PromQl, "mz_");
        assert_eq!(
            doc.functions["mzClusterName"]("up", &["instance_id".to_string()]),
            "up"
        );
    }

    #[test]
    fn enrichment_scopes_the_info_metric_to_the_environment() {
        // An unscoped join matches the same id across orgs in multi-tenant cloud.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        let joined = ctx.functions["mzObjectName"]("up", &["sink_id".to_string()]);
        assert!(
            joined.contains(&environment_filter()),
            "info metric was not scoped to the environment:\n{joined}"
        );
    }

    #[test]
    fn object_name_passes_an_extra_label_through() {
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        let joined = ctx.functions["mzObjectName"]("up", &["id".to_string(), "type".to_string()]);
        assert!(joined.contains("group_left(name, type)"), "{joined}");
    }

    #[test]
    fn exclude_environments_defaults_to_excluding_nothing() {
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        assert_eq!(ctx.parameters["excludeEnvironmentFilter"], "");

        let scope = DashboardScope {
            exclude_environments: r#"mz_context_org_type!="e2e_test""#.to_string(),
            ..DashboardScope::default()
        };
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &scope);
        assert_eq!(
            ctx.parameters["excludeEnvironmentFilter"],
            r#"mz_context_org_type!="e2e_test""#
        );
    }
}
