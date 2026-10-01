// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Karpenter dashboard: how Karpenter adds, replaces and removes the
//! cluster's nodes on EKS.
//!
//! Infrastructure Autoscaling answers whether pods are waiting and whether the
//! cloud could supply a node, the same way on every cloud. This dashboard is
//! Karpenter's own account, which says far more: which NodePool launched what,
//! what EC2 refused and why, how long each stage of a launch took, and what is
//! keeping nodes from being consolidated.
//!
//! # Rows render on Karpenter being there
//!
//! `infra-*` dashboards install on every cluster, and only EKS runs Karpenter.
//! Every row renders on `$karpenterDetected`, discovered from `up`, and each tab
//! carries one negated fallback that says why it is empty and where to look
//! instead. The condition matches the word `karpenter` rather than anything at
//! all, because a variable with no value can reach a row condition as the word
//! `undefined`, which `.+` matches.
//!
//! # The scrape is Terraform's
//!
//! The ServiceMonitor comes from the self-managed Terraform's `karpenter`
//! module, which also trims it: Karpenter publishes its price and availability
//! for every instance type in the region, and the module keeps availability for
//! the node pools' types only. `infra-karpenter.yaml` records what that leaves.

pub mod controller;
pub mod disruption;
pub mod events;
pub mod overview;
pub mod provisioning;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{AutoGrid, Layout, Row, RowHeight, Tab};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::variable::extra;
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-karpenter";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "infra-karpenter";

/// Dashboard title.
pub const TITLE: &str = "Karpenter";

/// Minimum Materialize version this dashboard requires. None in particular:
/// nothing here is Materialize's own.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The variable every row renders on.
pub(crate) const KARPENTER: &str = extra::KARPENTER_DETECTED;

/// What `$karpenterDetected` holds when Karpenter is scraped: the `app` label.
///
/// Not `.+`: a variable whose query returned nothing can reach the row
/// condition as `undefined`, which `.+` matches.
pub(crate) const DETECTED: &str = "karpenter";

/// The tabs, in order: the verdict, then adding nodes, then removing them, then
/// the controller, then the record.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::PROVISIONING.title).rows(provisioning::rows(q)),
        Tab::new(theme::DISRUPTION.title).rows(disruption::rows(q)),
        Tab::new(theme::CONTROLLER.title).rows(controller::rows(q)),
        Tab::new(theme::EVENTS.title).rows(events::rows(q)),
    ]
}

/// A row that renders only where Karpenter is running.
pub(crate) fn karpenter_row(title: &str) -> Row {
    Row::new(title).only_when_variable(KARPENTER, DETECTED)
}

/// What a Karpenter panel shows when its row rendered but the series is absent.
///
/// The row rendering means Karpenter reported its version, so the likely cause
/// is a family the ServiceMonitor's relabeling drops, or one Karpenter has not
/// written yet.
pub(crate) fn not_reported() -> NoValue {
    NoValue::Custom("Karpenter has not reported this yet.".to_string())
}

/// What an event or log panel shows when nothing happened.
pub(crate) fn quiet(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} in this time range"))
}

/// Why a tab has no rows on it.
pub(crate) const NO_KARPENTER: &str = "**Karpenter is not reporting on this cluster.**\n\n\
     Karpenter adds nodes on EKS in the self-managed Terraform. GKE and AKS use their managed \
     cluster autoscaler instead, which has no equivalent of this dashboard: \
     **Infrastructure Autoscaling** covers every cloud.\n\n\
     On EKS, this dashboard needs the ServiceMonitor that the Terraform `karpenter` module creates \
     with `enable_service_monitor`, and the monitoring CRDs installed before Karpenter, which the \
     examples' `monitoring-crds` module does. Karpenter's chart leaves the ServiceMonitor out when \
     the CRDs are not there yet, and does not add it later on its own; re-apply once they are.";

/// The fallback row a tab carries, on the negation of Karpenter being there.
pub(crate) fn no_karpenter_row(panel_name: &str) -> Row {
    Row::new("Karpenter Not Detected")
        .only_unless_variable(KARPENTER, DETECTED)
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                panel_name,
                Panel::text("Karpenter Not Detected", NO_KARPENTER)
                    .description("Why this tab has no rows on it.")
                    .build(0),
            ),
        )
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here, but stays in the signature so every
/// dashboard is built the same way.
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
            "How Karpenter adds, replaces and removes the cluster's nodes on EKS.\n\n\
             Launches and their errors, capacity EC2 refused, how long nodes take to join, \
             what keeps nodes from being consolidated, and the controller's health. The \
             cloud-agnostic view is Infrastructure Autoscaling.",
        )
        .tags([tags::INFRA, tags::MZMON])
        .folder(Folder::Infra)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::karpenter_scoped())
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
        let json = serde_json::to_string(&built().spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }

    #[test]
    fn it_is_filed_under_infrastructure() {
        let json = serde_json::to_string(&built().metadata).expect("serialize");
        assert!(json.contains("\"infra\""), "{json}");
    }

    #[test]
    fn every_query_references_only_variables_the_dashboard_defines() {
        test_support::assert_variables_defined(&built());
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        test_support::assert_every_panel_has_no_value(&built());
    }

    #[test]
    fn multi_series_panels_are_not_shaded() {
        test_support::assert_multi_series_panels_unshaded(&built());
    }

    #[test]
    fn selecting_every_node_pool_still_renders_the_rows() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn every_row_renders_on_karpenter_and_each_tab_has_one_fallback() {
        let conditions = test_support::variable_conditioned_rows(&built());
        let fallbacks = conditions
            .iter()
            .filter(|(_, op, value)| op == "notMatches" && value == DETECTED)
            .count();
        assert_eq!(fallbacks, theme::THEMED.len(), "{conditions:?}");
        for (title, op, value) in &conditions {
            assert_eq!(value, DETECTED, "{title} renders on something else");
            assert!(op == "matches" || op == "notMatches", "{title}: {op}");
        }
        // Every row that is not a fallback is conditioned: a bare row would show
        // empty panels on GKE and AKS beside the fallback that explains them.
        let json = serde_json::to_value(&built().spec.layout).expect("serialize");
        let rows = json.to_string().matches("\"RowsLayoutRow\"").count();
        assert_eq!(
            rows,
            conditions.len(),
            "an unconditioned row: {conditions:?}"
        );
    }

    #[test]
    fn the_detection_pattern_rejects_an_unset_variable() {
        // The condition is a substring match, so the pattern must not occur in
        // anything an unset variable can stringify to.
        for unset in ["undefined", "", "$__all", "null", "[object Object]"] {
            assert!(!unset.contains(DETECTED), "{unset} would render the rows");
        }
        let discovery = serde_json::to_string(&variable::karpenter_detected()).expect("serialize");
        assert!(
            discovery.contains(r#"up{app=\"karpenter\"}"#),
            "{discovery}"
        );
    }

    #[test]
    fn karpenters_generic_families_are_scoped_to_karpenter() {
        // `controller_runtime_*`, `workqueue_*`, `client_go_*`, `aws_sdk_go_*`
        // and `leader_election_*` are published by every controller-runtime
        // process in the cluster, and the EBS CSI driver and Load Balancer
        // Controller among them.
        let json = serde_json::to_string(&built().spec.elements).expect("serialize");
        for family in [
            "controller_runtime_",
            "workqueue_",
            "client_go_",
            "aws_sdk_go_",
            "leader_election_",
        ] {
            let mut rest = json.as_str();
            let mut seen = false;
            while let Some(pos) = rest.find(family) {
                seen = true;
                let tail = &rest[pos..];
                let end = tail.find('}').unwrap_or(tail.len());
                assert!(
                    tail[..end].contains(r#"app=\"karpenter\""#),
                    "{family} is not scoped to Karpenter: {}",
                    &tail[..end]
                );
                rest = &rest[pos + 1..];
            }
            assert!(seen, "{family} is not read at all");
        }
    }
}
