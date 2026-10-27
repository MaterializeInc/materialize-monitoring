// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Balancer tab: balancerd, which every SQL and HTTPS connection passes
//! through.
//!
//! A row of sparkline stats leads: open connections, then whether the pods are
//! up and how close they run to their CPU request and memory limit. The rows
//! below drill into each.
//!
//! balancerd counts a connection when it closes, so the rates trail long SQL
//! sessions by their whole length; open connections are the live view.
//! Capacity is the failure balancerd causes itself: a pod past its high
//! watermark leaves the load balancer's rotation, and one at its limit refuses
//! connections outright. Latency splits a slow connection into the part
//! balancerd spends and the part environmentd does. Resources and autoscaling
//! are the pods underneath: what they use against what they were given, and,
//! where a HorizontalPodAutoscaler runs them, how it is scaling them. The log
//! feed closes the tab because it is the one place an `error` connection's
//! reason reaches it.
//!
//! # Autoscaling renders on detection
//!
//! Most environments have no autoscaler on balancerd, so the Autoscaling row
//! renders on [`mzmon_lib::grafana::variable::balancerd_autoscaler`], with a
//! fallback that says how replicas are set without one.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;
use mzmon_lib::grafana::variable::extra;

use super::{LOG_BASE, no_balancer, no_timings, theme};
use crate::grafana::dependency::zero_is_healthy;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::BALANCER.shade;

/// The variable the Autoscaling row and its fallback render on.
const AUTOSCALER: &str = extra::BALANCERD_AUTOSCALER;

/// What every value of [`AUTOSCALER`] contains, and what an unset one never
/// does: the detected Deployment's name ends in `-balancerd`.
pub(crate) const AUTOSCALER_DETECTED: &str = "balancerd";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        summary(q),
        connections(q),
        capacity(q),
        latency(q),
        resources(q),
        autoscaling(q),
        no_autoscaler(),
        logs(q),
    ]
}

fn summary(q: &Queries) -> Row {
    Row::new("Balancer Summary").hide_header().grid(
        AutoGrid::new(6)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("balancer-summary-open", open_connections(q))
            .panel("balancer-summary-ready", ready_pods(q))
            .panel("balancer-summary-unavailable", unavailable_pods(q))
            .panel("balancer-summary-cpu", cpu_of_request(q))
            .panel("balancer-summary-memory", memory_of_limit(q))
            .panel("balancer-summary-restart", since_restart(q)),
    )
}

fn connections(q: &Queries) -> Row {
    Row::new("Connections").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-outcomes", outcomes(q))
            .panel("balancer-open-by-pod", open_by_pod(q))
            .panel("balancer-waiting", waiting_for_backend(q)),
    )
}

fn capacity(q: &Queries) -> Row {
    Row::new("Capacity").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-capacity", against_the_limit(q))
            .panel("balancer-accepting", accepting_by_pod(q))
            .panel("balancer-refused", refused(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Connection Timing").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-setup", setup_quantiles(q))
            .panel("balancer-upstream-connect", upstream_connect(q))
            .panel("balancer-lifetime", lifetime(q)),
    )
}

fn resources(q: &Queries) -> Row {
    Row::new("Resources").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-cpu", cpu_by_pod(q))
            .panel("balancer-memory", memory_by_pod(q))
            .panel("balancer-throttling", cpu_throttling(q)),
    )
}

fn autoscaling(q: &Queries) -> Row {
    Row::new("Autoscaling")
        .only_when_variable(AUTOSCALER, AUTOSCALER_DETECTED)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("balancer-autoscaler-replicas", autoscaler_replicas(q))
                .panel("balancer-autoscaler-metrics", autoscaler_metrics(q))
                .panel("balancer-autoscaler-conditions", autoscaler_conditions(q))
                .panel("balancer-autoscaler-events", autoscaler_events(q)),
        )
}

fn logs(q: &Queries) -> Row {
    Row::new("Balancer Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("balancer-logs", warnings(q)),
    )
}

// --- Summary -----------------------------------------------------------------

fn open_connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Open Connections")
        .query(
            q.get("materialize.ingress.overview.open_connections")
                .legend("connections"),
        )
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn ready_pods(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Ready Pods")
        .query(
            q.get("materialize.ingress.balancer.ready_pods")
                .legend("ready"),
        )
        .color_background()
        .thresholds(threshold::health(1.0, 1.0).build())
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn unavailable_pods(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Unavailable Pods")
        .query(
            q.get("materialize.ingress.balancer.unavailable_pods")
                .legend("unavailable"),
        )
        .thresholds(threshold::errors(1.0, 3.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn cpu_of_request(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("CPU vs Request")
        .query(
            q.get("materialize.ingress.balancer.cpu_of_request")
                .legend("cpu"),
        )
        .color_background()
        .thresholds(threshold::utilization(0.8, 1.0, 0.1).build())
        .unit("percentunit")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisorAndKubeStateMetrics)
        .build(0)
}

fn memory_of_limit(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Memory vs Limit")
        .query(
            q.get("materialize.ingress.balancer.memory_of_limit")
                .legend("busiest pod"),
        )
        .color_background()
        .thresholds(threshold::utilization(0.8, 1.0, 0.1).build())
        .unit("percentunit")
        .min(0.0)
        .no_value(NoValue::Custom(
            "balancerd has no memory limit, or the kubelet's container metrics are not scraped."
                .to_string(),
        ))
        .build(0)
}

fn since_restart(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Since Last Restart")
        .query(
            q.get("materialize.ingress.balancer.since_restart")
                .legend("newest pod"),
        )
        .color_background()
        .unit("s")
        // Low is bad here: a restart seconds ago is the alarming case, so the
        // palette runs alarming-to-calm as the duration grows.
        .thresholds(threshold::stability_days(2.0, false).build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn outcomes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Outcomes")
        .query(
            q.get("materialize.ingress.balancer.outcomes")
                .legend("{{source}} {{status}}"),
        )
        .unit("cps")
        .log_scale(LOG_BASE)
        .no_value(no_balancer())
        .build(0)
}

fn open_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections by Pod")
        .query(
            q.get("materialize.ingress.balancer.open_by_pod")
                .legend("{{pod}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn waiting_for_backend(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections Not Yet Routed")
        .query(
            q.get("materialize.ingress.balancer.waiting_for_backend")
                .legend("{{source}}"),
        )
        .unit("short")
        .decimals(0.0)
        .log_scale(LOG_BASE)
        .no_value(no_balancer())
        .build(0)
}

fn against_the_limit(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections Against the Limit")
        .query(q.legended(
            "materialize.ingress.balancer.capacity",
            &["{{pod}}", "limit", "high watermark", "low watermark"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn accepting_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Taking New Connections by Pod")
        .query(
            q.get("materialize.ingress.balancer.accepting_by_pod")
                .legend("{{pod}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "balancerd does not report its capacity state. Older Materialize versions don't \
             publish it."
                .to_string(),
        ))
        .build(0)
}

fn refused(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Refused Connections")
        .query(q.legended(
            "materialize.ingress.balancer.refused",
            &["connection limit", "{{source}} timed out before routing"],
        ))
        .unit("cps")
        .log_scale(LOG_BASE)
        .no_value(no_balancer())
        .build(0)
}

fn setup_quantiles(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Setup Quantiles")
        .query(q.legended(
            "materialize.ingress.balancer.setup_time",
            &["{{source}} p50", "{{source}} p90", "{{source}} p99"],
        ))
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_timings())
        .build(0)
}

fn upstream_connect(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Time to Reach environmentd")
        .query(q.legended(
            "materialize.ingress.balancer.upstream_connect",
            &["{{source}} p50", "{{source}} p99"],
        ))
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_timings())
        .build(0)
}

fn lifetime(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Lifetime")
        .query(q.legended(
            "materialize.ingress.balancer.lifetime",
            &["{{source}} p50", "{{source}} p99"],
        ))
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_timings())
        .build(0)
}

fn warnings(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warnings and Errors")
        .query(q.logs("materialize.ingress.balancer.logs.warnings"))
        .no_value(NoValue::Custom(
            "balancerd logged no warnings or errors in this time range.".to_string(),
        ))
        .build(0)
}

// --- Resources ---------------------------------------------------------------

fn cpu_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU by Pod")
        .query(q.legended(
            "materialize.ingress.balancer.cpu_by_pod",
            &["{{pod}}", "request"],
        ))
        .unit("cores")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory by Pod")
        .query(q.legended(
            "materialize.ingress.balancer.memory_by_pod",
            &["{{pod}}", "limit"],
        ))
        .unit("bytes")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn cpu_throttling(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Throttling")
        .query(
            q.get("materialize.ingress.balancer.cpu_throttling")
                .legend("{{pod}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "balancerd has no CPU limit, so the kernel never throttles it.".to_string(),
        ))
        .build(0)
}

// --- Autoscaling -------------------------------------------------------------

fn autoscaler_replicas(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Autoscaler Replicas")
        .query(q.legended(
            "materialize.ingress.balancer.autoscaler.replicas",
            &[
                "{{horizontalpodautoscaler}} current",
                "{{horizontalpodautoscaler}} desired",
                "{{horizontalpodautoscaler}} minimum",
                "{{horizontalpodautoscaler}} maximum",
            ],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn autoscaler_metrics(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Scaling Metrics Against Their Targets")
        .query(
            q.get("materialize.ingress.balancer.autoscaler.metrics")
                .legend("{{metric_name}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn autoscaler_conditions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Autoscaler Conditions")
        .query(q.legended(
            "materialize.ingress.balancer.autoscaler.conditions",
            &["at its maximum", "cannot read its metrics", "cannot scale"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn autoscaler_events(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Autoscaler Events")
        .query(q.logs("materialize.ingress.balancer.autoscaler.events"))
        .no_value(NoValue::Custom(
            "No autoscaler in this environment recorded an event in this time range.".to_string(),
        ))
        .build(0)
}

// --- Fallback ----------------------------------------------------------------

/// Why the tab has no Autoscaling row, and how to get one.
// TODO: name the first release with `balancerdExternallyScaled`
// (MaterializeInc/materialize#39701) once it ships.
pub(crate) const NO_AUTOSCALER: &str = "**No HorizontalPodAutoscaler scales balancerd in this \
     environment.**\n\n\
     The Materialize operator sets balancerd's replica count from the Materialize resource's \
     `balancerdReplicas`. To scale it with load instead, set `balancerdExternallyScaled: true` on \
     the Materialize resource, so that the operator stops resetting the count, and point a \
     HorizontalPodAutoscaler at the environment's `…-balancerd` Deployment. Materialize versions \
     without that field reset any count an autoscaler sets.\n\n\
     The Resources row shows the CPU an autoscaler would scale on, against each pod's request.";

fn no_autoscaler() -> Row {
    Row::new("No Autoscaler")
        .only_unless_variable(AUTOSCALER, AUTOSCALER_DETECTED)
        .hide_header()
        .grid(
            AutoGrid::new(1).panel(
                "balancer-no-autoscaler",
                Panel::text("No Autoscaler", NO_AUTOSCALER)
                    .description("Why this tab has no autoscaler row, and how to add one.")
                    .build(0),
            ),
        )
}
