// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Karpenter tab, and the rows Karpenter adds to the other tabs.
//!
//! Karpenter says far more about itself than GKE's and AKS's cluster
//! autoscalers, which run where nothing in the cluster can scrape them. Where
//! what it says answers a question another tab asks, it is a row on that tab:
//! its verdict on Overview, its NodePools' limits on Node Pools, how long its
//! launches took on Pending Pods, and what EC2 refused it on Cloud Capacity.
//! This tab holds the rest: what it replaced and removed, what held nodes in
//! place, and the controller itself.
//!
//! Karpenter acts a few times a day, and between those its counters stand
//! still. So this tab reads them over the selected time range, as counts and
//! percentiles, rather than as rates that are zero but for a spike; when each
//! thing happened is on the Events tab.
//!
//! # Rows render on Karpenter being there
//!
//! Every Karpenter row renders on `$karpenterDetected`, discovered from `up`.
//! On the shared tabs a missing row needs no explanation, since the tab's
//! other rows still answer its question. This tab has nothing else, so it
//! carries one negated fallback that says why it is empty. The condition
//! matches the word `karpenter` rather than anything at all, because a
//! variable with no value can reach a row condition as the word `undefined`,
//! which `.+` matches.
//!
//! # The scrape is Terraform's
//!
//! The ServiceMonitor comes from the self-managed Terraform's `karpenter`
//! module, which also trims it: Karpenter publishes its price and availability
//! for every instance type in the region, and the module keeps availability for
//! the node pools' types only. `infra-karpenter.yaml` records what that leaves.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueTextMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::variable::extra;

use super::{quiet, theme};
use crate::grafana::queries::Queries;
use crate::grafana::{field_override, transform};

const SHADE: &str = theme::KARPENTER.shade;

/// The variable every Karpenter row renders on.
pub(crate) const KARPENTER: &str = extra::KARPENTER_DETECTED;

/// What `$karpenterDetected` holds when Karpenter is scraped: the `app` label.
///
/// Not `.+`: a variable whose query returned nothing can reach the row
/// condition as `undefined`, which `.+` matches.
pub(crate) const DETECTED: &str = "karpenter";

/// A row that renders only where Karpenter is running.
pub(crate) fn row(title: &str) -> Row {
    Row::new(title).only_when_variable(KARPENTER, DETECTED)
}

/// What a Karpenter panel shows when its row rendered but the series is absent.
///
/// The row rendering means Karpenter is scraped, so the likely cause is a
/// family the ServiceMonitor's relabeling drops, or one Karpenter has not
/// written yet.
pub(crate) fn not_reported() -> NoValue {
    NoValue::Custom("Karpenter has not reported this yet.".to_string())
}

/// What a percentile over the selected range shows when nothing happened in it.
fn none_in_range(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} in this time range"))
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![changes(q), draining(q), controller(q), logs(q), fallback()]
}

fn changes(q: &Queries) -> Row {
    row("Replacements and Removals").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("karpenter-nodeclaims", nodeclaims(q))
            .panel("karpenter-held", held(q)),
    )
}

fn draining(q: &Queries) -> Row {
    row("Draining and Interruptions").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("karpenter-evictions", evictions(q))
            .panel("karpenter-termination", termination(q))
            .panel("karpenter-queue-delay", queue_delay(q)),
    )
}

fn controller(q: &Queries) -> Row {
    row("Controller").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("karpenter-reconcile-errors", reconcile_errors(q))
            .panel("karpenter-aws-errors", aws_errors(q))
            .panel("karpenter-kube-errors", kube_errors(q))
            .panel("karpenter-workqueue", workqueue(q))
            .panel("karpenter-cpu", cpu(q))
            .panel("karpenter-memory", memory(q)),
    )
}

fn logs(q: &Queries) -> Row {
    row("Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("karpenter-problems", problems(q)),
    )
}

// --- Replacements and removals -----------------------------------------------

const CREATED: &str = "Created";
const DISRUPTED: &str = "Disrupted";

/// One row per NodePool and reason, created beside disrupted.
///
/// The two expressions arrive as separate frames, joined on the label columns
/// by `merge`, as on the Workload Autoscaling tab's table.
fn nodeclaims(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("NodeClaims by Reason (Select Time Range)")
        .query(
            q.legended(
                "infra.karpenter.nodeclaims.by_reason",
                &[CREATED, DISRUPTED],
            )
            .table_format(),
        )
        .unit("short")
        .decimals(0.0)
        .no_value(none_in_range("nodes created or disrupted"))
        .transformations(vec![
            transform::merge(),
            transform::organize_full(
                &["Time"],
                &[
                    "nodepool",
                    "reason",
                    CREATED,
                    DISRUPTED,
                    "Value #query-0",
                    "Value #query-1",
                ],
                &[
                    ("nodepool", "NodePool"),
                    ("reason", "Reason"),
                    ("Value #query-0", CREATED),
                    ("Value #query-1", DISRUPTED),
                ],
            ),
        ])
        // A reason with nodes created and none disrupted leaves a gap, which the
        // panel's own empty-state text would otherwise fill.
        .overrides(vec![zero_when_empty(CREATED), zero_when_empty(DISRUPTED)])
        .build(0)
}

/// One row per node and what held it, from Karpenter's own events.
///
/// Loki answers an instant query with a frame per series, so the labels are
/// promoted to columns and the frames merged into one table.
fn held(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("What Held Nodes in Place (Select Time Range)")
        .query(q.logs("infra.karpenter.disruption.blocked"))
        .no_value(NoValue::Custom(
            "Karpenter reported no node held in place in this time range.".to_string(),
        ))
        .transformations(vec![
            transform::labels_to_fields(&["name", "why", "holder"]),
            transform::merge(),
            transform::organize_full(
                &["Time", "Value"],
                &["name", "why", "holder"],
                &[("name", "Node"), ("why", "Why"), ("holder", "Held By")],
            ),
            transform::sort_by("Node", false),
        ])
        .build(0)
}

fn zero_when_empty(field: &str) -> dashboardv2::FieldConfigSourceOverridesItem {
    field_override::by_name(field)
        .property("noValue", serde_json::Value::String("0".to_string()))
        .build()
}

// --- Draining and interruptions ----------------------------------------------

fn evictions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Evictions (Select Time Range)")
        .query(q.legended(
            "infra.karpenter.disruption.evictions",
            &["allowed", "refused"],
        ))
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn termination(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Time to Terminate a Node (p99)")
        .query(
            q.get("infra.karpenter.disruption.termination")
                .legend("p99"),
        )
        .shade(SHADE)
        .unit("s")
        .no_value(none_in_range("nodes terminated"))
        .build(0)
}

fn queue_delay(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Notice Wait in Queue (p99)")
        .query(
            q.get("infra.karpenter.interruption.queue_delay")
                .legend("p99"),
        )
        .shade(SHADE)
        .unit("s")
        .no_value(none_in_range("interruption notices"))
        .build(0)
}

// --- Controller --------------------------------------------------------------

fn reconcile_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Reconcile Errors per Minute")
        .query(
            q.get("infra.karpenter.controller.reconcile_errors")
                .legend("{{controller}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No reconcile errors".to_string()))
        .build(0)
}

fn aws_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed AWS Calls per Minute")
        .query(
            q.get("infra.karpenter.controller.aws_errors")
                .legend("{{exported_service}} {{action}} {{code}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No failed AWS calls".to_string()))
        .build(0)
}

fn kube_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed Kubernetes API Calls per Minute")
        .query(
            q.get("infra.karpenter.controller.kube_errors")
                .legend("{{method}} {{code}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No failed Kubernetes API calls".to_string(),
        ))
        .build(0)
}

fn workqueue(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Work Queue Depth")
        .query(
            q.get("infra.karpenter.controller.workqueue")
                .legend("{{name}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::Custom("Every queue is empty".to_string()))
        .build(0)
}

fn cpu(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU by Replica")
        .query(q.get("infra.karpenter.controller.cpu").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory by Replica")
        .query(q.legended(
            "infra.karpenter.controller.memory",
            &["{{pod}}", "{{pod}} limit"],
        ))
        .unit("bytes")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisorAndKubeStateMetrics)
        .build(0)
}

// --- Logs --------------------------------------------------------------------

fn problems(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warning and Error Logs")
        .query(q.logs("infra.karpenter.logs.problems"))
        .prettify()
        .no_value(quiet("warnings or errors"))
        .build(0)
}

// --- Fallback ----------------------------------------------------------------

/// Why this tab has no rows on it.
pub(crate) const NO_KARPENTER: &str = "**Karpenter is not reporting on this cluster.**\n\n\
     Karpenter adds nodes on EKS in the self-managed Terraform. GKE and AKS use their managed \
     cluster autoscaler instead, which runs in the control plane where nothing in the cluster \
     can scrape it: the other tabs cover it from the pods, nodes and events it leaves behind.\n\n\
     On EKS, this tab needs the ServiceMonitor that the Terraform `karpenter` module creates \
     with `enable_service_monitor`, and the monitoring CRDs installed before Karpenter, which the \
     examples' `monitoring-crds` module does. Karpenter's chart leaves the ServiceMonitor out when \
     the CRDs are not there yet, and does not add it later on its own; re-apply once they are.";

fn fallback() -> Row {
    Row::new("Karpenter Not Detected")
        .only_unless_variable(KARPENTER, DETECTED)
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "karpenter-not-detected",
                Panel::text("Karpenter Not Detected", NO_KARPENTER)
                    .description("Why this tab has no rows on it.")
                    .build(0),
            ),
        )
}
