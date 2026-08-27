// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Dashboard variables. Port of `dashboards.variables`.
//!
//! The set here is what [`crate::grafana::context`] requires plus the two
//! controls that shape them, chained so each narrows the last: pick an
//! environment, and the namespace / cluster / replica lists follow from it.
//!
//! Two conventions carried over, both documented in
//! [`crate::grafana::context::variables`]:
//!
//! * A `*List` name means the value is always interpolated as a *pattern* —
//!   `instance_id=~"$mzClusterList"`, never `=`. A single-select variable still
//!   renders as an alternation when "All" is chosen, and `=` against `a|b` matches
//!   nothing, silently.
//! * `Name` versus `Id` records which identifier is stable. [`environments`] holds
//!   organization *names* because self-managed Materialize has no organization id;
//!   [`clusters`] holds cluster *ids* with the name as display text, because
//!   cluster names are neither stable nor unique.
//!
//! Discovery queries read `mz_compute_commands_total` — genuine instrumentation
//! present in every deployment, so never SQL-prefixed.

use crate::grafana::context::variables;
use crate::grafana::generated::dashboardv2;
use crate::grafana::query::{METRICS_DATASOURCE_VAR, promql_data_query};

/// The info metric the discovery queries read.
///
/// Genuine instrumentation (bare `mz_` in every deployment), so it is *not*
/// SQL-prefixed — unlike [`CLUSTER_STATUS_METRIC`].
const INFO_METRIC: &str = "mz_compute_commands_total";

/// The SQL-derived metric [`clusters`] reads, which *is* prefixed.
const CLUSTER_STATUS_SUFFIX: &str = "compute_cluster_status";

/// Grafana plugin id for the metrics datasource.
const PROMETHEUS_PLUGIN: &str = "prometheus";

/// Extra controls the baseline defines beyond what the render context requires.
pub mod extra {
    /// Whether cluster discovery includes Materialize's own system clusters.
    pub const INCLUDE_SYSTEM_CLUSTERS: &str = "includeSystemClusters";
    /// Free-form label filters applied to every metrics query.
    pub const METRIC_ADHOC: &str = "metricAdhoc";
}

/// An empty current selection.
///
/// Grafana fills this in on load from the variable's own query; authoring a value
/// would just pin a stale selection into the dashboard.
fn no_selection() -> dashboardv2::VariableOption {
    dashboardv2::VariableOption {
        selected: None,
        text: dashboardv2::VariableOptionText::String(String::new()),
        value: dashboardv2::VariableOptionValue::String(String::new()),
        properties: Default::default(),
    }
}

/// Shared skeleton for the query variables, which differ only in a few fields.
struct QueryVariable {
    name: &'static str,
    label: &'static str,
    description: &'static str,
    expr: String,
    multi: bool,
    include_all: bool,
    all_value: Option<&'static str>,
    hide: dashboardv2::VariableHide,
    sort: dashboardv2::VariableSort,
    skip_url_sync: bool,
    /// Named-capture regex extracting `value` and `text` from a `query_result`.
    regex: String,
}

impl QueryVariable {
    fn build(self) -> dashboardv2::VariableKind {
        dashboardv2::VariableKind::QueryVariableKind(dashboardv2::QueryVariableKind {
            kind: "QueryVariable".to_string(),
            spec: dashboardv2::QueryVariableSpec {
                name: self.name.to_string(),
                label: Some(self.label.to_string()),
                description: Some(self.description.to_string()),
                // Grafana shows `definition` in the variable editor; it has to
                // repeat the query rather than being derived from it.
                definition: Some(self.expr.clone()),
                query: promql_data_query(&self.expr, METRICS_DATASOURCE_VAR, None),
                multi: self.multi,
                include_all: self.include_all,
                all_value: self.all_value.map(str::to_string),
                hide: self.hide,
                sort: self.sort,
                skip_url_sync: self.skip_url_sync,
                regex: self.regex,
                regex_apply_to: Some(dashboardv2::VariableRegexApplyTo::Value),
                // Custom values let an operator name an environment or cluster the
                // discovery query has not seen yet -- a new environment, or one
                // whose metrics have aged out of the window.
                allow_custom_value: true,
                // `Never`: these are chained, and refreshing on every time-range
                // change re-runs four queries for no new information.
                refresh: dashboardv2::VariableRefresh::Never,
                current: no_selection(),
                options: Vec::new(),
                static_options: Vec::new(),
                static_options_order: None,
                placeholder: None,
                origin: None,
            },
        })
    }
}

/// The metrics datasource, which every other variable's query runs against.
pub fn metrics_datasource() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::DatasourceVariableKind(dashboardv2::DatasourceVariableKind {
        kind: "DatasourceVariable".to_string(),
        spec: dashboardv2::DatasourceVariableSpec {
            name: METRICS_DATASOURCE_VAR.to_string(),
            label: Some("Metrics Datasource".to_string()),
            description: Some("Datasource for metrics queries".to_string()),
            plugin_id: PROMETHEUS_PLUGIN.to_string(),
            multi: false,
            include_all: false,
            all_value: None,
            allow_custom_value: false,
            hide: dashboardv2::VariableHide::DontHide,
            refresh: dashboardv2::VariableRefresh::Never,
            regex: String::new(),
            skip_url_sync: false,
            current: no_selection(),
            options: Vec::new(),
            origin: None,
        },
    })
}

/// Environment selector, by organization *name*.
///
/// **Single-select on purpose.** Object and cluster ids are only unique *within*
/// one environment, and both the registry's aggregations and
/// [`crate::query::enrich`]'s joins match on those ids alone — so selecting two
/// environments can merge unrelated series, or break the enrichment join with a
/// duplicate id on its right-hand side. Multi-select becomes safe once the
/// organization label survives those aggregations and is part of the enrichment
/// join keys; until then this deliberately does not offer it.
///
/// Queries must still treat the value as a *pattern* — hence the `*List` name —
/// because a custom value interpolates as an alternation.
pub fn environments() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::ENVIRONMENT_NAME_LIST,
        label: "Environment",
        description: "The current environment to view",
        expr: format!("label_values({INFO_METRIC}, materialize_cloud_organization_name)"),
        multi: false,
        include_all: false,
        all_value: Some(".*"),
        hide: dashboardv2::VariableHide::DontHide,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// Namespaces the selected environments live in.
///
/// Hidden: it is derived from the environment selection, so exposing it invites
/// an operator to desync the two. `materialize_cloud_organization_namespace` is
/// the only namespace label present in both deployments — self-managed carries
/// `kubernetes_namespace`, Cloud carries `namespace`.
pub fn namespaces() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::MZ_NAMESPACE_LIST,
        label: "Materialize Namespace",
        description: "The current materialize namespace where environments live",
        expr: format!(
            "label_values({INFO_METRIC}{{ {} }}, materialize_cloud_organization_namespace)",
            environment_selector()
        ),
        multi: true,
        include_all: true,
        all_value: None,
        hide: dashboardv2::VariableHide::HideVariable,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        // Derived, so keeping it out of the URL avoids permalinks that pin a
        // namespace inconsistent with their own environment.
        skip_url_sync: true,
        regex: String::new(),
    }
    .build()
}

/// Toggle for whether [`clusters`] includes Materialize's system clusters.
///
/// The values are regexes spliced into the cluster query: system cluster ids
/// start with `s`, so excluding them is `^[^s].*`.
pub fn include_system_clusters() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::SwitchVariableKind(dashboardv2::SwitchVariableKind {
        kind: "SwitchVariable".to_string(),
        spec: dashboardv2::SwitchVariableSpec {
            name: extra::INCLUDE_SYSTEM_CLUSTERS.to_string(),
            label: Some("Include System Clusters".to_string()),
            description: Some(
                "Whether to include materialize system clusters in the cluster list.".to_string(),
            ),
            enabled_value: ".*".to_string(),
            disabled_value: "^[^s].*".to_string(),
            current: ".*".to_string(),
            hide: dashboardv2::VariableHide::InControlsMenu,
            skip_url_sync: true,
            origin: None,
        },
    })
}

/// Cluster selector within the selected environments.
///
/// Reads the SQL-derived `compute_cluster_status`, so `sql_metric_prefix` must
/// match what the panels use (`mz_` self-managed, `v2_mz_` Cloud).
///
/// Uses `query_result` rather than `label_values` so both the id and the name are
/// available: the regex puts `compute_cluster_id` in the value and
/// `compute_cluster_name` in the display text. Grafana sorts labels
/// alphabetically, and `compute_cluster_id` sorts before `compute_cluster_name`,
/// which is what makes that regex stable.
pub fn clusters(sql_metric_prefix: &str) -> dashboardv2::VariableKind {
    let metric = format!("{sql_metric_prefix}{CLUSTER_STATUS_SUFFIX}");
    QueryVariable {
        name: variables::MZ_CLUSTER_LIST,
        label: "Cluster",
        description: "The cluster within the current environment to filter to",
        expr: format!(
            "query_result({metric}{{{}, compute_cluster_id=~\"${}\" }})",
            environment_selector(),
            extra::INCLUDE_SYSTEM_CLUSTERS
        ),
        multi: true,
        include_all: true,
        all_value: None,
        hide: dashboardv2::VariableHide::InControlsMenu,
        // Natural, so u2 sorts before u11.
        sort: dashboardv2::VariableSort::NaturalAsc,
        skip_url_sync: false,
        regex: r#".*compute_cluster_id=\"(?<value>[^\"]+)\",.*compute_cluster_name=\"(?<text>[^\"]+)\",.*"#
            .to_string(),
    }
    .build()
}

/// Replica selector within the selected clusters.
///
/// Replica *ids*, not names: names are almost always `r1`, which says nothing
/// across clusters.
pub fn replicas() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::MZ_REPLICA_LIST,
        label: "Replica",
        description: "The replica within the current cluster to filter to",
        expr: format!(
            "label_values({INFO_METRIC}{{{}, instance_id=~\"${}\"}}, replica_id)",
            environment_selector(),
            variables::MZ_CLUSTER_LIST
        ),
        multi: true,
        include_all: true,
        all_value: Some(".*"),
        hide: dashboardv2::VariableHide::InControlsMenu,
        sort: dashboardv2::VariableSort::NaturalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// Free-form label filters applied to every metrics query.
///
/// Seeded with the namespace selector so an operator's ad-hoc filters compose
/// with the environment scope rather than escaping it.
pub fn metric_adhoc() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::AdhocVariableKind(dashboardv2::AdhocVariableKind {
        kind: "AdhocVariable".to_string(),
        // The ad-hoc filter picks its own label keys from the datasource, so it
        // carries a datasource ref of its own rather than borrowing a query's.
        datasource: Some(dashboardv2::AdhocVariableKindDatasource {
            name: Some(format!("${METRICS_DATASOURCE_VAR}")),
        }),
        // Empty in the baseline: `group` names a datasource *plugin* for
        // dataqueries, and an ad-hoc variable resolves its own through the
        // datasource ref above.
        group: String::new(),
        labels: Default::default(),
        spec: dashboardv2::AdhocVariableSpec {
            name: extra::METRIC_ADHOC.to_string(),
            label: Some("Advanced Metric Filter".to_string()),
            description: Some("Adhoc filters to apply to all metrics queries".to_string()),
            base_filters: vec![dashboardv2::AdHocFilterWithLabels {
                key: "namespace".to_string(),
                operator: "=~".to_string(),
                value: format!("${}", variables::MZ_NAMESPACE_LIST),
                // `FilterOrigin` is a newtype over the string Grafana expects.
                origin: Some(dashboardv2::FilterOrigin("dashboard".to_string())),
                condition: None,
                force_edit: None,
                key_label: None,
                values: Vec::new(),
                value_labels: Vec::new(),
            }],
            filters: Vec::new(),
            default_keys: Vec::new(),
            enable_group_by: false,
            allow_custom_value: true,
            hide: dashboardv2::VariableHide::InControlsMenu,
            skip_url_sync: false,
            origin: None,
        },
    })
}

/// The standard environment-scoped variable set, in dependency order.
///
/// Order matters to a reader, not to Grafana: each variable's query references
/// the one before it, so listing them in that order is what makes the chain
/// legible in the variable editor.
pub fn environment_scoped(sql_metric_prefix: &str) -> Vec<dashboardv2::VariableKind> {
    vec![
        metrics_datasource(),
        environments(),
        namespaces(),
        include_system_clusters(),
        clusters(sql_metric_prefix),
        replicas(),
        metric_adhoc(),
    ]
}

/// The environment scope fragment the chained queries share.
fn environment_selector() -> String {
    format!(
        r#"materialize_cloud_organization_name=~"${}""#,
        variables::ENVIRONMENT_NAME_LIST
    )
}

/// The `name` of a variable, whatever its kind.
pub fn name_of(variable: &dashboardv2::VariableKind) -> &str {
    use dashboardv2::VariableKind as V;
    match variable {
        V::QueryVariableKind(v) => &v.spec.name,
        V::TextVariableKind(v) => &v.spec.name,
        V::ConstantVariableKind(v) => &v.spec.name,
        V::DatasourceVariableKind(v) => &v.spec.name,
        V::IntervalVariableKind(v) => &v.spec.name,
        V::CustomVariableKind(v) => &v.spec.name,
        V::GroupByVariableKind(v) => &v.spec.name,
        V::AdhocVariableKind(v) => &v.spec.name,
        V::SwitchVariableKind(v) => &v.spec.name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::context::REQUIRED_VARIABLES;

    fn names(variables: &[dashboardv2::VariableKind]) -> Vec<&str> {
        variables.iter().map(name_of).collect()
    }

    #[test]
    fn the_standard_set_defines_every_required_variable() {
        // The check that matters: an undefined variable interpolates to nothing
        // and every selector using it silently matches no series.
        let set = environment_scoped("mz_");
        let defined = names(&set);
        for required in REQUIRED_VARIABLES {
            assert!(
                defined.contains(required),
                "the standard set does not define ${required}"
            );
        }
    }

    #[test]
    fn the_standard_set_is_listed_in_dependency_order() {
        assert_eq!(
            names(&environment_scoped("mz_")),
            vec![
                "metricsDatasource",
                "environmentNameList",
                "mzNamespaceList",
                "includeSystemClusters",
                "mzClusterList",
                "mzReplicaList",
                "metricAdhoc",
            ]
        );
    }

    #[test]
    fn no_variable_is_defined_twice() {
        let set = environment_scoped("mz_");
        let mut seen = names(&set);
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "duplicate variable name");
    }

    /// Extract each variable's discovery expression, where it has one.
    fn expr_of(variable: &dashboardv2::VariableKind) -> Option<String> {
        match variable {
            dashboardv2::VariableKind::QueryVariableKind(v) => v
                .spec
                .query
                .spec
                .as_ref()
                .and_then(|s| s.get("expr"))
                .and_then(|e| e.as_str())
                .map(str::to_string),
            _ => None,
        }
    }

    #[test]
    fn every_chained_query_references_only_variables_the_set_defines() {
        // The chain is the part that breaks quietly: `mzReplicaList` reads
        // `$mzClusterList`, which reads `$includeSystemClusters`, and a rename
        // anywhere in that line leaves an empty selector.
        let set = environment_scoped("mz_");
        let defined = names(&set);
        for variable in &set {
            let Some(expr) = expr_of(variable) else {
                continue;
            };
            for reference in dollar_references(&expr) {
                assert!(
                    defined.contains(&reference.as_str()),
                    "{} references undefined ${reference}",
                    name_of(variable)
                );
            }
        }
    }

    #[test]
    fn the_chain_narrows_in_order() {
        let set = environment_scoped("mz_");
        let expr = |name: &str| {
            set.iter()
                .find(|v| name_of(v) == name)
                .and_then(expr_of)
                .unwrap_or_default()
        };
        // Environment discovery is unscoped; everything after it is scoped by the
        // selection before it.
        assert!(!expr("environmentNameList").contains('$'));
        assert!(expr("mzNamespaceList").contains("$environmentNameList"));
        assert!(expr("mzClusterList").contains("$environmentNameList"));
        assert!(expr("mzClusterList").contains("$includeSystemClusters"));
        assert!(expr("mzReplicaList").contains("$mzClusterList"));
    }

    #[test]
    fn the_cluster_query_is_sql_prefixed_and_the_others_are_not() {
        // `compute_cluster_status` is SQL-derived and must match the panels'
        // prefix; `mz_compute_commands_total` is genuine instrumentation and never
        // prefixed.
        let cloud = environment_scoped("v2_mz_");
        let expr = |set: &[dashboardv2::VariableKind], name: &str| {
            set.iter()
                .find(|v| name_of(v) == name)
                .and_then(expr_of)
                .unwrap()
        };
        assert!(expr(&cloud, "mzClusterList").contains("v2_mz_compute_cluster_status"));
        assert!(expr(&cloud, "mzReplicaList").contains("mz_compute_commands_total"));
        assert!(!expr(&cloud, "mzReplicaList").contains("v2_mz_compute_commands_total"));

        let self_managed = environment_scoped("mz_");
        assert!(expr(&self_managed, "mzClusterList").contains("mz_compute_cluster_status"));
    }

    #[test]
    fn the_environment_selector_is_single_select() {
        // Multi-select would let two environments' id-keyed series merge, and would
        // break the enrichment joins, which match on ids alone. See `environments`.
        match environments() {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert!(!v.spec.multi, "multi-select is unsound with id-keyed joins");
                assert!(
                    !v.spec.include_all,
                    "\"All\" is multi-select by another name"
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn derived_variables_are_hidden_and_kept_out_of_the_url() {
        // A permalink that pins a derived namespace inconsistent with its own
        // environment is worse than one that re-derives it.
        let set = environment_scoped("mz_");
        match set
            .iter()
            .find(|v| name_of(v) == "mzNamespaceList")
            .unwrap()
        {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert_eq!(v.spec.hide, dashboardv2::VariableHide::HideVariable);
                assert!(v.spec.skip_url_sync);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_cluster_regex_names_both_captures() {
        match environment_scoped("mz_")
            .into_iter()
            .find(|v| name_of(v) == "mzClusterList")
            .unwrap()
        {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert!(v.spec.regex.contains("(?<value>"), "{}", v.spec.regex);
                assert!(v.spec.regex.contains("(?<text>"), "{}", v.spec.regex);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_adhoc_filter_is_seeded_with_the_namespace_scope() {
        match metric_adhoc() {
            dashboardv2::VariableKind::AdhocVariableKind(v) => {
                let base = &v.spec.base_filters[0];
                assert_eq!(base.key, "namespace");
                assert_eq!(base.operator, "=~");
                assert_eq!(base.value, "$mzNamespaceList");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Every `$name`, skipping `label_replace` capture groups.
    fn dollar_references(value: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = value;
        while let Some(pos) = rest.find('$') {
            let after = &rest[pos + 1..];
            let len = after
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            if len > 0 && !after[..len].bytes().all(|b| b.is_ascii_digit()) {
                out.push(after[..len].to_string());
            }
            rest = &after[len..];
        }
        out
    }
}
