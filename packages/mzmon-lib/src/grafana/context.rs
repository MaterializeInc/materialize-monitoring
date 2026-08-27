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
//! operator and system namespaces below are plain values rather than `$…`
//! references, since no dashboard defines a variable for them.

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
    /// Namespace selector value for the Materialize operator. A plain value, not
    /// a `$variable`: no dashboard defines an operator-namespace variable, and
    /// referencing a missing one would match nothing.
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
        // Grafana built-ins: `$__rate_interval` adapts to the panel's resolution
        // and scrape interval, which is what a dashboard wants where the doc
        // context hardcodes a window.
        ("interval", "[$__rate_interval]".to_string()),
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
        // shipping.
        let registry = QueryRegistry::new();
        let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());
        let builtins = ["__rate_interval", "__range"];

        for (name, value) in &ctx.parameters {
            for reference in dollar_references(value) {
                let known = builtins.contains(&reference.as_str())
                    || REQUIRED_VARIABLES.contains(&reference.as_str())
                    || NODE_VARIABLES.contains(&reference.as_str());
                assert!(known, "parameter {name} references unknown ${reference}");
            }
        }
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
