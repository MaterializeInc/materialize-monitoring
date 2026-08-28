// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Reconciliation tab: the operator's control loop, as counters and
//! histograms.
//!
//! The Events tab is the rollout's account of *itself* — the phases it moved
//! through and what it said when one failed. This tab is the machine underneath
//! it: is anything reconciling at all, what are the passes concluding, how long
//! do they take, and which phase are they stopping in. Metrics rather than
//! events, so every query here is fetched with [`Queries::get`] and lands on
//! `$metricsDatasource`.
//!
//! # Reading order
//!
//! 1. **Operator Status** — is the loop turning, and how much is left to do.
//!    `Environments Needing Update` is the upgrade's own progress bar and the
//!    reason this row leads.
//! 2. **Reconciliation Passes** — what the passes concluded, and who is failing.
//! 3. **Duration** — how long the work takes, which separates "the operator is
//!    slow" from "the operator is waiting", two things that look identical from
//!    outside.
//! 4. **Steps** — which phase, which is where a failing pass becomes a specific
//!    thing to go and look at.
//!
//! # Not environment-scoped
//!
//! One operator reconciles every environment in the cluster and its metrics carry
//! no organization label, so the environment picker does not narrow anything
//! here. Only `$operatorNamespace` applies. That is a real difference from every
//! other tab in this repo and is stated in the panel descriptions rather than
//! left for a reader to infer from an unchanging graph.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::{palette, threshold};

use super::theme;
use crate::grafana::queries::Queries;

/// The tab's theme, applied to every shaded panel here.
const SHADE: &str = theme::RECONCILIATION.shade;

/// What a panel shows when nothing matched.
///
/// These metrics exist only if the operator is being scraped, so an empty panel
/// here is far more likely to be a missing scrape target than a filter miss —
/// the opposite of the Events tab, where absence is the healthy reading.
fn no_operator_metrics() -> NoValue {
    NoValue::Custom("No metrics: the Materialize operator is not being scraped".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        operator_status(q),
        reconciliation_passes(q),
        duration(q),
        steps(q),
    ]
}

fn operator_status(q: &Queries) -> Row {
    Row::new("Operator Status").hide_header().grid(
        AutoGrid::new(4)
            .row_height(RowHeight::Short)
            .panel("reconciling-replicas", reconciling_replicas(q))
            .panel(
                "environments-needing-update",
                environments_needing_update(q),
            )
            .panel("reconciliation-rate", reconciliation_rate(q))
            .panel("failed-passes", failed_passes(q)),
    )
}

fn reconciliation_passes(q: &Queries) -> Row {
    Row::new("Reconciliation Passes").grid(
        AutoGrid::new(2)
            .panel("pass-outcomes", pass_outcomes(q))
            .panel("failures-by-controller", failures_by_controller(q)),
    )
}

fn duration(q: &Queries) -> Row {
    Row::new("Duration").grid(
        AutoGrid::new(2)
            .panel("pass-duration", pass_duration(q))
            .panel("step-duration", step_duration(q)),
    )
}

fn steps(q: &Queries) -> Row {
    Row::new("Steps").grid(
        AutoGrid::new(2)
            .panel("step-activity", step_activity(q))
            .panel("step-failures", step_failures(q)),
    )
}

// ------------------------------------------------------------ operator status

fn reconciling_replicas(q: &Queries) -> dashboardv2::PanelKind {
    // The one genuinely binary panel on the tab, so it is threshold-coloured
    // rather than shaded: zero is an outage of the control loop, and two is a
    // reading that should not be possible. Both ends are wrong, which is why this
    // is a hand-built ladder rather than `threshold::health` — that one treats
    // "higher" as "better" all the way up.
    Panel::stat("Reconciling Replicas")
        .query(
            q.get("materialize.operator.reconciling.replicas")
                .legend("leaders"),
        )
        .thresholds(
            threshold::Ladder::new(palette::tri_health::UNHEALTHY)
                .step(1.0, palette::tri_health::HEALTHY)
                .step(2.0, palette::tri_health::UNHEALTHY)
                .build(),
        )
        .min(0.0)
        .decimals(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

fn environments_needing_update(q: &Queries) -> dashboardv2::PanelKind {
    // Deliberately not alarm-coloured, on the same reasoning as env-top's
    // Currently Hydrating: a non-zero reading is what a routine upgrade looks
    // like, so colouring it red would fire on every one. The sparkline carries
    // the signal, which is the direction rather than the number.
    Panel::stat("Environments Needing Update")
        .query(
            q.get("materialize.operator.environments.needing_update")
                .legend("outdated"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .min(0.0)
        .decimals(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

fn reconciliation_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Reconciliation Rate")
        .query(
            q.get("materialize.operator.reconciliation.rate")
                .legend("passes"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("reqps")
        .min(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

fn failed_passes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Failed Passes (Select Time Range)")
        .query(
            q.get("materialize.operator.reconciliation.failures.total")
                .legend("failed"),
        )
        .thresholds(threshold::errors_default().build())
        .min(0.0)
        .decimals(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

// -------------------------------------------------------------------- passes

fn pass_outcomes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pass Outcomes")
        .query(
            q.get("materialize.operator.reconciliation.outcomes")
                .legend("{{outcome}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

fn failures_by_controller(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed Passes by Controller")
        .query(
            q.get("materialize.operator.reconciliation.failures.by_controller")
                .legend("{{controller}} / {{event_type}}"),
        )
        .unit("reqps")
        .min(0.0)
        // Empty is the healthy reading on this one, unlike its neighbours.
        .no_value(NoValue::Custom("No failing passes".to_string()))
        .build(0)
}

// ------------------------------------------------------------------ duration

fn pass_duration(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pass Duration")
        .query(q.legended(
            "materialize.operator.reconciliation.duration",
            &["p50", "p90", "p99"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

fn step_duration(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Step Duration (p99)")
        .query(
            q.get("materialize.operator.reconciliation.step.duration.p99")
                .legend("{{step}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

// --------------------------------------------------------------------- steps

fn step_activity(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Step Activity")
        .query(
            q.get("materialize.operator.reconciliation.steps.rate")
                .legend("{{step}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(no_operator_metrics())
        .build(0)
}

fn step_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Step Failures and Abandonments")
        .query(
            q.get("materialize.operator.reconciliation.steps.incomplete")
                .legend("{{step}} / {{outcome}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom("No incomplete steps".to_string()))
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_operator_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_operator_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 10);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_panel_queries_the_metrics_datasource() {
        // The mirror of the Events tab's check. A metrics query fetched with
        // `logs` would render PromQL against Loki and come back empty.
        let q = &test_operator_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                assert_eq!(
                    query.spec.query.group, "prometheus",
                    "{name} is not a Prometheus query"
                );
            }
        }
    }

    #[test]
    fn every_query_is_scoped_to_the_operator_namespace_and_nothing_else() {
        // These metrics carry no organization label, so an environment filter
        // would match nothing rather than narrowing. Catching that here is the
        // point: the panel would render empty with no other clue.
        let q = &test_operator_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = query.spec.query.spec.as_ref().expect("spec")["expr"]
                    .as_str()
                    .expect("expr");
                assert!(
                    expr.contains("$operatorNamespace"),
                    "{name} is not scoped to the operator namespace: {expr}"
                );
                assert!(
                    !expr.contains("materialize_cloud_organization_name"),
                    "{name} filters on an organization label these metrics do not carry: {expr}"
                );
            }
        }
    }

    #[test]
    fn the_replica_count_flags_both_zero_and_two() {
        // Zero is the control loop being down; two should not be reachable. A
        // plain health ladder would call two the healthiest reading of all.
        let q = &test_operator_queries();
        let panel = reconciling_replicas(q);
        let steps = &panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .thresholds
            .as_ref()
            .expect("thresholds")
            .steps;
        let colour_at = |value: f64| {
            steps
                .iter()
                .rfind(|s| s.value.is_none_or(|v| v <= value))
                .map(|s| s.color.clone())
                .expect("a step")
        };
        assert_eq!(colour_at(0.0), palette::tri_health::UNHEALTHY);
        assert_eq!(colour_at(1.0), palette::tri_health::HEALTHY);
        assert_eq!(colour_at(2.0), palette::tri_health::UNHEALTHY);
    }

    #[test]
    fn progress_and_absence_are_not_alarm_coloured() {
        // An upgrade in progress is not a fault. Colouring the outstanding count
        // red would fire on every routine rollout, which is the failure mode that
        // retired env-top's "Stuck Objects" stat.
        let q = &test_operator_queries();
        for panel in [environments_needing_update(q), reconciliation_rate(q)] {
            let defaults = &panel.spec.viz_config.spec.field_config.defaults;
            assert!(
                defaults.thresholds.is_none(),
                "{} is threshold-coloured",
                panel.spec.title
            );
            assert!(
                defaults.color.is_some(),
                "{} has no shade",
                panel.spec.title
            );
        }
    }
}
