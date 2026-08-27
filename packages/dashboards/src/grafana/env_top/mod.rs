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
pub mod transform;

use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};

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

/// Description shared by the Total CPU Capacity panels on the Summary and
/// Kubernetes Workloads tabs.
///
/// Verbatim identical in the baseline, and for good reason: the text explains the
/// one way the two panels differ (whether the monitoring exporter is counted), so
/// a second copy could only drift away from that explanation.
pub(super) const CPU_CAPACITY_DESCRIPTION: &str = "**Total CPU cores configured across containers in the selected scope** (sum of CPU limits \
     from cAdvisor). Steps correlate with `ALTER CLUSTER REPLICA SIZE`, `CREATE`/`DROP CLUSTER \
     REPLICA`, or pod restarts. On the Summary tab the monitoring exporter is excluded (so this \
     reflects user-workload capacity); on the Kubernetes Workloads tab it's included.";

/// Description shared by the Total Memory panels on both tabs.
pub(super) const MEMORY_TOTAL_DESCRIPTION: &str = "**Total memory configured across containers in the selected scope** (sum of memory limits \
     from cAdvisor). Memory is the dominant constraint on Materialize: in-memory arrangements \
     (see _Compute Objects -> Arrangements_) live in here. Steps correlate with `ALTER CLUSTER \
     REPLICA SIZE` or pod restarts.";

/// The Currently Hydrating panel, which the Summary and Compute Objects tabs both
/// show.
///
/// Identical in the baseline down to the expression, so it lives here rather than
/// being written twice: the Summary copy is a pointer into Compute Objects, and a
/// second copy could only drift from what it points at. `shade` is the only
/// difference — Summary borrows Compute's colour, Compute uses its own.
pub(super) fn currently_hydrating(
    shade: &str,
) -> mzmon_lib::grafana::generated::dashboardv2::PanelKind {
    use mzmon_lib::grafana::panel::{NoValue, Panel};
    use mzmon_lib::grafana::query::{PromQuery, query_group};

    let expr = format!(
        r#"
count(
    max by (instance_id, collection_id) (
        mz_dataflow_wallclock_lag_seconds{{
            {env},
            {cluster},
            instance_id!="",
            quantile="1"
        }} > {sentinel}
    )
) or vector(0)
"#,
        env = selector::environment(),
        cluster = selector::cluster(),
        sentinel = selector::HYDRATING_SENTINEL
    );
    Panel::stat("Currently Hydrating")
        .description(HYDRATING_DESCRIPTION)
        .data(query_group(vec![
            PromQuery::new(expr).legend("hydrating").build(),
        ]))
        .shade(shade)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

/// Prose for [`currently_hydrating`].
///
/// The baseline points at `_Compute -> Freshness_`, which is not a tab; corrected
/// to `_Compute Objects -> Freshness_` here.
pub(super) const HYDRATING_DESCRIPTION: &str = "**Collections still (re)building their in-memory state — a live hydration-queue proxy.** \
     Hydration rebuilds a dataflow's state from persisted storage after a cluster/replica \
     restart, replica creation, or some DDL; until it finishes, the collection has no output \
     frontier, which this counts (via the `mz_dataflow_wallclock_lag_seconds` sentinel). \
     **Nominal: 0, with brief spikes right after a replica restart that drain back to 0 as \
     dataflows catch up — that's healthy.** A count that stays elevated means something can't \
     finish hydrating (e.g. a source whose `CREATE` didn't complete, or a wedged dataflow). \
     Confirm what's stuck with `SELECT * FROM mz_internal.mz_hydration_statuses WHERE NOT \
     hydrated`; watch _Compute Objects -> Freshness_ for the lag those collections accrue.";

/// The tabs, in order.
///
/// One entry per tab module. Tabs not yet ported are simply absent — see the
/// porting status in the module docs.
fn tabs() -> Vec<Tab> {
    vec![
        Tab::new(theme::SUMMARY_TITLE).rows(summary::rows()),
        Tab::new(theme::KUBERNETES.title).rows(kubernetes::rows()),
        Tab::new(theme::CONNECTIONS.title).rows(connections::rows()),
        Tab::new(theme::CLUSTERS.title).rows(clusters::rows()),
        Tab::new(theme::COMPUTE.title).rows(compute::rows()),
        Tab::new(theme::SOURCES_SINKS.title).rows(sources_sinks::rows()),
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
pub fn build(cloud: crate::grafana::Cloud, sql_metric_prefix: &str) -> dashboard::Result<Resource> {
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
        // What the artifact targets. Nothing in the chart reads these today, but
        // they are how a rendered file says which variant it is.
        .metadata_annotation(
            "monitoring.materialize.cloud/target-cloud",
            cloud.to_string(),
        )
        .metadata_annotation("monitoring.materialize.cloud/target-export", TARGET_EXPORT)
        .layout(Layout::tabs(tabs()))
        .build()
}

/// Render for the registry: applies the cloud variant, or refuses.
pub fn render(options: &crate::grafana::Options) -> crate::grafana::render::Result<Resource> {
    use crate::grafana::{Cloud, render::Error};

    match options.cloud {
        Cloud::Generic => {
            build(options.cloud, &options.sql_metric_prefix).map_err(|source| Error::Build {
                name: NAME_STEM,
                source,
            })
        }
        // The GCP variant is not ported. Its CPU and memory gauges are different
        // panels, not the same panels with different metadata, because GCP's
        // cAdvisor does not expose the container memory limit the generic gauges
        // divide by.
        cloud => Err(Error::UnsupportedCloud {
            name: NAME_STEM,
            cloud,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_builds_for_both_deployments() {
        for prefix in ["mz_", "v2_mz_"] {
            let resource = build(crate::grafana::Cloud::Generic, prefix).expect("build");
            assert_eq!(resource.metadata.name, NAME);
            assert_eq!(resource.spec.title, TITLE);
        }
    }

    #[test]
    fn the_sql_prefix_reaches_the_cluster_variable_and_nothing_else() {
        let cloud = build(crate::grafana::Cloud::Generic, "v2_mz_").expect("build");
        let json = serde_json::to_string(&cloud.spec.variables).expect("serialize");
        assert!(
            json.contains("v2_mz_compute_cluster_status"),
            "cluster query not prefixed"
        );
        // The info metric the other variables read is genuine instrumentation.
        assert!(!json.contains("v2_mz_compute_commands_total"));
    }

    #[test]
    fn no_description_points_somewhere_that_does_not_exist() {
        // Descriptions navigate by naming a destination as `_Where -> What_`, and
        // `Where` is either a tab or a row — `_Compute Objects -> Freshness_` names
        // a tab, `_Freshness -> Most-Lagged Collections_` names a row on the
        // current one. Either way it has to exist.
        //
        // The baseline has three that do not: `_Storage -> Sources_`,
        // `_Storage Objects -> Sink Throughput_` and `_Compute -> Freshness_`,
        // against tabs actually called `Sources and Sinks` and `Compute Objects`.
        // It reads like a tab rename that never reached the prose, which is
        // exactly the class of rot a check can hold back.
        let resource = build(crate::grafana::Cloud::Generic, "mz_").expect("build");
        let mut destinations: Vec<String> = std::iter::once(theme::SUMMARY_TITLE.to_string())
            .chain(theme::THEMED.iter().map(|t| t.title.to_string()))
            .collect();
        destinations.extend(row_titles(&resource.spec));

        let mut broken = Vec::new();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for destination in arrow_references(&panel.spec.description) {
                if !destinations.contains(&destination) {
                    broken.push(format!(
                        "{name}: _{destination} -> …_ is neither a tab nor a row"
                    ));
                }
            }
        }
        broken.sort();
        assert!(
            broken.is_empty(),
            "{} broken cross-reference(s):\n  {}",
            broken.len(),
            broken.join("\n  ")
        );
    }

    /// Every row title in the dashboard.
    fn row_titles(spec: &mzmon_lib::grafana::generated::dashboardv2::Dashboard) -> Vec<String> {
        let json = serde_json::to_value(&spec.layout).unwrap_or(serde_json::Value::Null);
        let mut out = Vec::new();
        collect_row_titles(&json, &mut out);
        out
    }

    fn collect_row_titles(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|k| k.as_str()) == Some("RowsLayoutRow")
                    && let Some(title) = map["spec"].get("title").and_then(|t| t.as_str())
                {
                    out.push(title.to_string());
                }
                for v in map.values() {
                    collect_row_titles(v, out);
                }
            }
            serde_json::Value::Array(items) => {
                for v in items {
                    collect_row_titles(v, out);
                }
            }
            _ => {}
        }
    }

    /// The left-hand side of every `_Where -> What_` italic reference.
    ///
    /// Only the arrow form is checked. A bare `_Currently Hydrating_` names a
    /// *panel*, and validating those would need panel titles from tabs that may not
    /// be ported yet — a reference ahead of us would read as broken.
    fn arrow_references(description: &str) -> Vec<String> {
        // Code spans first: `mz_compute_cluster_status` is full of underscores that
        // would otherwise parse as italic delimiters.
        let prose: String = description
            .split('`')
            .step_by(2)
            .collect::<Vec<_>>()
            .join(" ");

        let mut out = Vec::new();
        let mut rest = prose.as_str();
        while let Some(start) = rest.find('_') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('_') else { break };
            let italic = &after[..end];
            if let Some((where_, _what)) = italic.split_once("->") {
                out.push(where_.trim().to_string());
            }
            rest = &after[end + 1..];
        }
        out
    }

    #[test]
    fn the_metadata_annotations_the_docsite_reads_are_present() {
        // Consumed by the grafana-dashboards Hugo shortcode. Grafana drops them on
        // a UI save, so they are informational rather than a gate.
        let resource = build(crate::grafana::Cloud::Generic, "mz_").expect("build");
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
