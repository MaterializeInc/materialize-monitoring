// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The metadata database dashboard: consensus, as Materialize experiences it.
//!
//! Persist keeps each durable collection's data in object storage and a small
//! record of its current state in the metadata database, and commits a new
//! version of that record with a compare-and-set on every change. The timestamp
//! oracle keeps every timeline's timestamps in the same database. Those are the
//! two clients this dashboard reads, and the only view of the database that is
//! identical on RDS, Cloud SQL, Flexible Server, CNPG and CockroachDB.
//!
//! # Who it is for, and what it cannot say
//!
//! An operator whose environment is slow or stalled, deciding whether the
//! metadata database is why. Everything here is the client's measurement —
//! which is the SLI, because it includes the network, TLS and the process's
//! own connection pool — and none of it can say *why* the database is slow.
//! That is the provider's view, on `infra-cloud`, and the two are separate
//! dashboards on purpose: the database is shared by every environment on the
//! cluster and its series carry no environment label, while everything here is
//! scoped to one environment.
//!
//! The exception is a database running in the cluster, which has no provider
//! to ask. Database Internals carries its own account, for CloudNativePG, on
//! rows that render only where CNPG is scraped — see [`database`].
//!
//! # Scope
//!
//! The environment picker and nothing narrower. The dependency serves the
//! whole environment, so "which process is suffering" is a legend rather than a
//! filter — see [`variable::dependency_scoped`] for why a cluster picker would
//! cost more than it gives.
//!
//! # Grounded in two incidents
//!
//! Both are in the reference installs' logs and are what the Overview's logs row
//! and the Connections tab were shaped around: a Cloud SQL instance out of
//! connection slots, and an RDS instance refusing connections outright. In the
//! first, the client metrics showed thousands of connection errors in a quarter
//! of an hour and environmentd's pool at seven times its usual size; the log
//! line is what said it was `max_connections`.

pub mod connections;
pub mod database;
pub mod operations;
pub mod overview;
pub mod state;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-env-consensus";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "env-consensus";

/// Dashboard title.
pub const TITLE: &str = "Materialize Consensus (Metadata)";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular: every family it reads has been published since 2023.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

pub(crate) use crate::grafana::dependency::{
    PROCESS, latency_ladder, not_collected, zero_is_healthy,
};

/// The timestamp oracle's own series, which has no process to break down by:
/// it runs in environmentd alone.
pub(crate) const ORACLE: &str = "timestamp oracle";

/// The timestamp oracle's series, labelled by operation.
pub(crate) const ORACLE_OP: &str = "timestamp oracle {{op}}";

/// The tabs, in order.
///
/// Overview answers "is it the database". Operations and Connections are the
/// two places the answer usually is — the database being slow, or the pools in
/// front of it running out. State and Cleanup is the slow failure: nothing is
/// wrong today, and the table is growing. Database Internals is last because
/// it is the database's own account rather than Materialize's, and only a
/// database running in the cluster gives one here.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::OPERATIONS.title).rows(operations::rows(q)),
        Tab::new(theme::CONNECTIONS.title).rows(connections::rows(q)),
        Tab::new(theme::STATE.title).rows(state::rows(q)),
        Tab::new(theme::DATABASE.title).rows(database::rows(q)),
    ]
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
pub fn build(sql_metric_prefix: &str, registry: &QueryRegistry) -> dashboard::Result<Resource> {
    let scope = DashboardScope::for_prefix(sql_metric_prefix);
    let queries = Queries::new(registry, &scope);
    let layout = Layout::tabs(tabs(&queries));

    let failures = queries.failures();
    if !failures.is_empty() {
        return Err(dashboard::Error::Registry {
            dashboard: NAME_STEM,
            failures,
        });
    }

    Dashboard::new(NAME, TITLE)
        .description(
            "The metadata database, as Materialize experiences it.\n\n\
             Commits, failures, latency and connection pools, measured by the \
             processes that use the database — the same on every database \
             flavor. What a CloudNativePG database reports about itself is on \
             the Database Internals tab, and what a managed database's cloud \
             provider reports is on the Infrastructure Cloud Provider dashboard.",
        )
        .tags([tags::MATERIALIZE, tags::MZMON, tags::content::DEPENDENCIES])
        .folder(Folder::Materialize)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::consensus_scoped())
        .metadata_annotation(
            "monitoring.materialize.cloud/min-mz-version",
            MIN_MZ_VERSION,
        )
        .metadata_annotation(
            "monitoring.materialize.cloud/rec-mz-version",
            REC_MZ_VERSION,
        )
        .metadata_annotation(
            "monitoring.materialize.cloud/sql-metric-prefix",
            sql_metric_prefix,
        )
        .metadata_annotation("monitoring.materialize.cloud/target-export", TARGET_EXPORT)
        .layout(layout)
        .build()
}

/// Render for the registry.
pub fn render(
    options: &crate::grafana::Options,
    registry: &QueryRegistry,
) -> crate::grafana::render::Result<Resource> {
    use crate::grafana::render::Error;

    build(&options.sql_metric_prefix, registry).map_err(|source| Error::Build {
        name: NAME_STEM,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_registry;
    use crate::grafana::test_support;

    fn built() -> Resource {
        build("mz_", test_registry()).expect("build")
    }

    #[test]
    fn it_builds() {
        let resource = built();
        assert_eq!(resource.metadata.name, NAME);
        assert_eq!(resource.spec.title, TITLE);
    }

    #[test]
    fn every_tab_is_present() {
        let resource = built();
        let json = serde_json::to_string(&resource.spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }

    #[test]
    fn it_is_filed_under_materialize() {
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
        assert!(json.contains("\"materialize\""), "{json}");
    }

    #[test]
    fn every_query_references_only_variables_the_dashboard_defines() {
        // The dependency set drops the cluster and replica pickers; a query that
        // reached for one would interpolate to nothing and match no series.
        test_support::assert_variables_defined(&built());
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        // Every counter here exists at zero from process start, so an empty
        // panel always means nothing is reporting -- which is worth saying.
        test_support::assert_every_panel_has_no_value(&built());
    }

    #[test]
    fn multi_series_panels_are_not_shaded() {
        test_support::assert_multi_series_panels_unshaded(&built());
    }

    #[test]
    fn every_cnpg_row_renders_only_where_cnpg_is_scraped() {
        // A CNPG row without a condition draws empty panels on every managed
        // database, which is most installs.
        let conditions = test_support::variable_conditioned_rows(&built());
        let when: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .collect();
        // One section holds them all, so its condition is the only one.
        assert_eq!(when.len(), 1, "{conditions:?}");
        assert_eq!(when[0].0, database::CNPG_SECTION);
        assert_eq!(when[0].2, database::CNPG);
    }

    #[test]
    fn the_database_tab_has_exactly_one_fallback_covering_every_flavor() {
        let conditions = test_support::variable_conditioned_rows(&built());
        let fallbacks: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        assert_eq!(fallbacks.len(), 1, "{fallbacks:?}");
        assert!(fallbacks[0].2.contains(database::CNPG), "{fallbacks:?}");
    }

    #[test]
    fn selecting_every_cnpg_cluster_still_renders_the_rows() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn an_unset_detection_variable_renders_no_cnpg_row() {
        // An empty discovery reaches the condition as one of these strings.
        for unset in ["", "undefined", "null", "[object Object]"] {
            assert!(!unset.contains(database::CNPG), "{unset}");
        }
    }

    #[test]
    fn every_cnpg_query_is_scoped_by_the_cluster_picker() {
        // CNPG's series carry no environment, so the picker is the only thing
        // keeping a second CNPG cluster's numbers off these panels.
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        let selectors = test_support::selectors_of(&json, "cnpg_");
        assert!(!selectors.is_empty(), "no CNPG queries found");
        for selector in selectors {
            assert!(
                selector.contains("$cnpgClusterList"),
                "unscoped: {selector}"
            );
        }
    }

    #[test]
    fn the_cluster_picker_lives_on_its_section_not_the_dashboard() {
        // A dashboard-level picker sits in the controls of every environment,
        // whether or not it runs this flavor; on the section, it hides with it.
        let resource = built();
        assert!(
            !resource
                .spec
                .variables
                .iter()
                .any(|v| variable::name_of(v) == "cnpgClusterList"),
            "the picker is dashboard-level"
        );
        let json = serde_json::to_value(&resource.spec.layout).expect("serialize");
        let mut found = Vec::new();
        find_section_variables(&json, &mut found);
        assert_eq!(
            found,
            vec![(
                database::CNPG_SECTION.to_string(),
                "cnpgClusterList".to_string()
            )]
        );
    }

    /// `(row title, variable name)` for every row that declares a variable.
    fn find_section_variables(value: &serde_json::Value, out: &mut Vec<(String, String)>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(spec) = map.get("spec")
                    && let Some(vars) = spec.get("variables").and_then(|v| v.as_array())
                {
                    let title = spec["title"].as_str().unwrap_or_default().to_string();
                    for v in vars {
                        out.push((
                            title.clone(),
                            v["spec"]["name"].as_str().unwrap_or_default().to_string(),
                        ));
                    }
                }
                map.values().for_each(|v| find_section_variables(v, out));
            }
            serde_json::Value::Array(items) => {
                items.iter().for_each(|v| find_section_variables(v, out))
            }
            _ => {}
        }
    }

    #[test]
    fn the_fallback_explains_rather_than_announces() {
        assert!(database::NO_FLAVOR.contains("PodMonitor"));
        assert!(database::NO_FLAVOR.contains("Infrastructure Cloud Provider"));
    }

    #[test]
    fn the_logs_panel_reads_loki_and_only_consensus_work() {
        // The retry line is shared with object storage; which dependency a line
        // is about is only in the name of the work being retried.
        let q = Queries::new(test_registry(), &DashboardScope::default());
        let logs = q.logs("materialize.consensus.logs.retries");
        let json = serde_json::to_string(&logs.query_group).expect("serialize");
        assert!(json.contains("apply_unbatched_cmd::cas"), "{json}");
        assert!(json.contains("read_ts"), "{json}");
        assert!(!json.contains("batch::set"), "{json}");
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }
}
