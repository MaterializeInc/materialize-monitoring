// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The environment overview dashboard.
//!
//! A high-level read on one Materialize environment, with the Summary tab
//! answering "is it healthy" and the rest explaining why. Ported from
//! `dashboards.mz_environment.overview`.
//!
//! # Layout
//!
//! Tabs of rows of auto-grids. [`theme`] owns the per-tab colours; [`selector`]
//! owns the PromQL fragments that reference dashboard variables, so no panel
//! query names a variable directly.
//!
//! # Parity
//!
//! All 69 panels of the pre-rendered baseline are here, one module per tab.
//! `tests/env_top_parity.rs` holds the line: it compares every panel's title,
//! description, plugin, unit and queries against the baseline, and enumerates the
//! handful of places this deliberately differs. Adding a tab is a module plus one
//! line in [`tabs`].

pub mod clusters;
pub mod compute;
pub mod connections;
pub mod field_override;
pub mod kubernetes;
pub mod selector;
pub mod sources_sinks;
pub mod summary;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-env-top";

/// Artifact filename stem, which is *not* the resource name.
///
/// The chart globs `pre-rendered/dashboards/grafana/<stem>.yaml` and the docsite
/// serves `<stem>.json`, while `metadata.name` is the in-Grafana identity. Two
/// separate identifiers that happen to describe the same dashboard.
pub const NAME_STEM: &str = "env-top";

/// Dashboard title.
pub const TITLE: &str = "Materialize Environment Overview";

/// Minimum Materialize version this dashboard's metrics require.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.29.0";

/// The Currently Hydrating panel, which the Summary and Compute Objects tabs both
/// show.
///
/// One query, one panel definition, two placements: the Summary copy is a pointer
/// into Compute Objects, and a second copy could only drift from what it points at.
/// `shade` is the only difference — Summary borrows Compute's colour, Compute uses
/// its own.
pub(super) fn currently_hydrating(
    q: &Queries,
    shade: &str,
) -> mzmon_lib::grafana::generated::dashboardv2::PanelKind {
    use mzmon_lib::grafana::panel::{NoValue, Panel};

    Panel::stat("Currently Hydrating")
        .query(
            q.get("materialize.compute.hydration.currently_hydrating")
                .legend("hydrating"),
        )
        .shade(shade)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

/// The tabs, in order.
///
/// One entry per tab module. Tabs not yet ported are simply absent — see the
/// porting status in the module docs.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::SUMMARY_TITLE).rows(summary::rows(q)),
        Tab::new(theme::KUBERNETES.title).rows(kubernetes::rows(q)),
        Tab::new(theme::CONNECTIONS.title).rows(connections::rows(q)),
        Tab::new(theme::CLUSTERS.title).rows(clusters::rows(q)),
        Tab::new(theme::COMPUTE.title).rows(compute::rows(q)),
        Tab::new(theme::SOURCES_SINKS.title).rows(sources_sinks::rows(q)),
    ]
}

/// The export target this crate produces.
///
/// The Python also emits a `cloud_monitoring` export — a Google Cloud Monitoring
/// dashboard rather than a Grafana one — which has no Rust equivalent yet. When one
/// lands this becomes a field on [`crate::grafana::Options`]; until then a constant
/// is the honest answer.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` is `mz_` on self-managed and `v2_mz_` on Cloud; it reaches
/// the cluster-discovery variable, which reads a SQL-derived metric.
///
/// Every panel's expression and description come from `registry` — see
/// [`crate::grafana::queries`]. A panel naming an id that does not exist is
/// reported here rather than by the panel, so a typo lists every bad id at once.
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
            "Overview of a Materialize Environment.\n\nThis provides a high-level summary to \
             catch more obvious issues\nthat may require further investigation.",
        )
        .tags(["materialize", "monitoring"])
        // The baseline left this Off. On a dashboard built for correlating across
        // panels, a shared crosshair is the whole point.
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::environment_scoped(sql_metric_prefix))
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
///
/// Every cloud renders the same panels. The Python branched here: GKE's *managed*
/// cAdvisor and kube-state-metrics shipped a reduced allowlist that omitted the
/// container limit and spec series, so the percent-of-limit gauges had no
/// denominator and fell back to absolute cores and bytes, under different titles.
/// That gap is closed — `alloy-gateway` scrapes `/metrics/cadvisor` on every
/// kubelet directly rather than consuming GKE's subset, and every `container_*`
/// series these queries reference is present. See `docs/content/metrics/scraping.md`.
///
/// So `cloud` reaches only the `target-cloud` metadata annotation, which records
/// what the artifact was rendered for. If a cloud ever diverges in panel content
/// again, this is where the branch goes back.
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

    #[test]
    fn it_builds_for_both_deployments() {
        for prefix in ["mz_", "v2_mz_"] {
            let resource = build(prefix, crate::grafana::queries::test_registry()).expect("build");
            assert_eq!(resource.metadata.name, NAME);
            assert_eq!(resource.spec.title, TITLE);
        }
    }

    #[test]
    fn the_sql_prefix_reaches_the_cluster_variable_and_nothing_else() {
        let cloud = build("v2_mz_", crate::grafana::queries::test_registry()).expect("build");
        let json = serde_json::to_string(&cloud.spec.variables).expect("serialize");
        assert!(
            json.contains("v2_mz_compute_cluster_status"),
            "cluster query not prefixed"
        );
        // The info metric the other variables read is genuine instrumentation.
        assert!(!json.contains("v2_mz_compute_commands_total"));
    }
    #[test]
    fn the_metadata_annotations_the_docsite_reads_are_present() {
        // Consumed by the grafana-dashboards Hugo shortcode. Grafana drops them on
        // a UI save, so they are informational rather than a gate.
        let resource = build("mz_", crate::grafana::queries::test_registry()).expect("build");
        for key in [
            "monitoring.materialize.cloud/min-mz-version",
            "monitoring.materialize.cloud/rec-mz-version",
            "monitoring.materialize.cloud/sql-metric-prefix",
        ] {
            assert!(
                resource.metadata.annotations.contains_key(key),
                "missing {key}"
            );
        }
    }
}
