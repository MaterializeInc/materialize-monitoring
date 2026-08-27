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
//! # Porting status
//!
//! The baseline carries 69 panels across six tabs. Tabs land one at a time, and
//! `tests/env_top_parity.rs` is the ledger: it compares this dashboard against
//! the pre-rendered baseline and reports, per tab, which panels are still
//! missing. Adding a tab is a module plus one line in [`tabs`].

pub mod clusters;
pub mod selector;
pub mod summary;
pub mod theme;
pub mod transform;

use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-env-top";

/// Dashboard title.
pub const TITLE: &str = "Materialize Environment Overview";

/// Minimum Materialize version this dashboard's metrics require.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.29.0";

/// The tabs, in order.
///
/// One entry per tab module. Tabs not yet ported are simply absent — see the
/// porting status in the module docs.
fn tabs() -> Vec<Tab> {
    vec![
        Tab::new(theme::SUMMARY_TITLE).rows(summary::rows()),
        Tab::new(theme::CLUSTERS.title).rows(clusters::rows()),
    ]
}

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` is `mz_` on self-managed and `v2_mz_` on Cloud; it reaches
/// the cluster-discovery variable, which reads a SQL-derived metric.
pub fn build(sql_metric_prefix: &str) -> dashboard::Result<Resource> {
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
        .layout(Layout::tabs(tabs()))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_builds_for_both_deployments() {
        for prefix in ["mz_", "v2_mz_"] {
            let resource = build(prefix).expect("build");
            assert_eq!(resource.metadata.name, NAME);
            assert_eq!(resource.spec.title, TITLE);
        }
    }

    #[test]
    fn the_sql_prefix_reaches_the_cluster_variable_and_nothing_else() {
        let cloud = build("v2_mz_").expect("build");
        let json = serde_json::to_string(&cloud.spec.variables).expect("serialize");
        assert!(
            json.contains("v2_mz_compute_cluster_status"),
            "cluster query not prefixed"
        );
        // The info metric the other variables read is genuine instrumentation.
        assert!(!json.contains("v2_mz_compute_commands_total"));
    }

    #[test]
    fn no_description_points_at_a_tab_that_does_not_exist() {
        // Panel descriptions cross-reference tabs as `_Tab -> Section_`, which is
        // how a reader navigates. The baseline has two that name tabs which do not
        // exist (`_Storage -> Sources_`, `_Compute -> Freshness_`), so the pointer
        // dead-ends. `theme` knows every real title, which is what makes this
        // checkable at all.
        let resource = build("mz_").expect("build");
        let titles: Vec<&str> = std::iter::once(theme::SUMMARY_TITLE)
            .chain(theme::THEMED.iter().map(|t| t.title))
            .collect();

        let mut broken = Vec::new();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for tab in tab_references(&panel.spec.description) {
                if !titles.contains(&tab.as_str()) {
                    broken.push(format!("{name}: _{tab} -> …_ is not a tab"));
                }
            }
        }
        assert!(
            broken.is_empty(),
            "{} broken cross-reference(s):\n  {}",
            broken.len(),
            broken.join("\n  ")
        );
    }

    /// Tab names from `_Tab -> Section_` italics.
    ///
    /// Only the arrow form is checked. A bare `_Currently Hydrating_` names a
    /// *panel*, not a tab, and validating those needs every tab ported first —
    /// until then a reference to a panel on an unported tab would read as broken
    /// when it is merely ahead of us.
    fn tab_references(description: &str) -> Vec<String> {
        // Code spans are stripped first: `mz_compute_cluster_status` is full of
        // underscores that would otherwise parse as italic delimiters.
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
            if let Some((tab, _section)) = italic.split_once("->") {
                out.push(tab.trim().to_string());
            }
            rest = &after[end + 1..];
        }
        out
    }

    #[test]
    fn the_metadata_annotations_the_docsite_reads_are_present() {
        // Consumed by the grafana-dashboards Hugo shortcode. Grafana drops them on
        // a UI save, so they are informational rather than a gate.
        let resource = build("mz_").expect("build");
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
