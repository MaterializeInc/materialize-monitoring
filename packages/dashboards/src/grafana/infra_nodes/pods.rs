// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Pods tab: what is scheduled here, and whether it is well.
//!
//! The Summary tab answers "how full is this node" in three gauges. This one
//! answers the question that follows — *with what* — and it is the tab to open
//! when a replica is missing, restarting, or refusing traffic and the node is a
//! suspect.
//!
//! **Requests and limits are the "budget" half, and they are not the same
//! question.** A request is what the scheduler set aside and can never be
//! reclaimed; a limit is a ceiling the kernel enforces. A pod with a request and
//! no limit can use the whole machine when it is idle; a pod that crosses its
//! memory limit is OOM-killed without warning. The two tables sit side by side
//! because the *difference* between them is what an operator needs to see — and
//! the limits table is normally shorter, since a pod need not set one.
//!
//! kube-state-metrics puts `node` on the resource families directly but not on
//! the status ones, so phase, readiness and restarts reach this node through an
//! intersection with `kube_pod_info` — see `infra-nodes.yaml`.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::field_override;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![health(q), budgets(q)]
}

/// Is everything that landed here actually running.
fn health(q: &Queries) -> Row {
    Row::new("Pod Health").grid(
        AutoGrid::new(3)
            .panel("pods-by-phase", by_phase(q))
            .panel("pods-not-ready", not_ready(q))
            .panel("pods-restarts", restarts(q)),
    )
}

/// What each of them reserved, and what each is capped at.
fn budgets(q: &Queries) -> Row {
    Row::new("Requests and Limits").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("pods-budgets", budgets_table(q)),
    )
}

fn by_phase(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Pods by Phase")
        .query(q.get("infra.nodes.pods.by_phase").legend("{{phase}}"))
        .unit("short")
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn not_ready(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Pods Not Ready")
        // `table_format` for the same reason as the budgets table: without it
        // each pod arrives as its own frame and the panel shows a frame picker
        // instead of one table. A dropdown at the foot of a table is the tell.
        .query(q.get("infra.nodes.pods.not_ready").table_format())
        // Empty is the healthy reading, so it has to say so rather than looking
        // like a panel that failed to load.
        .no_value(NoValue::Custom(
            "Every pod on this node is Ready.".to_string(),
        ))
        // No `labels_to_fields`: the table frame already carries the labels as
        // columns, so it has nothing left to promote.
        .transformations(vec![transform::organize(
            &["Time", "Value", "__name__"],
            &["namespace", "pod"],
        )])
        .build(0)
}

fn restarts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Container Restarts")
        .query(
            q.get("infra.nodes.pods.restarts")
                .legend("{{namespace}}/{{pod}}"),
        )
        .unit("short")
        .min(0.0)
        .decimals(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

/// Requests beside limits, CPU beside memory, one row per pod.
///
/// One table rather than four, because the comparison that matters is a pod's
/// request against its own limit — across four sorted lists that means holding
/// pod names in your head.
///
/// The four expressions arrive as separate frames and are joined on the label
/// columns by `merge`; the value columns are then renamed off their refIds,
/// which the renderer assigns positionally as `query-0`..`query-3`.
fn budgets_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Requests and Limits by Pod")
        .query(
            q.legended(
                "infra.nodes.pods.budgets",
                &[CPU_REQUESTED, CPU_LIMIT, MEMORY_REQUESTED, MEMORY_LIMIT],
            )
            // Without this each query comes back as its own series frame and the
            // merge stacks them into one column instead of four.
            .table_format(),
        )
        // **No `no_value` here, deliberately.** Grafana applies it per *field*,
        // not per panel, so it fills every empty cell rather than standing in for
        // an empty panel — and most cells in the limit columns are legitimately
        // empty, because a pod need not set a limit. A collection-failure message
        // in those cells would be both wrong and unreadable. Blank is the honest
        // rendering of "this pod sets none", and an empty panel still falls back
        // to Grafana's own "No data".
        .transformations(vec![
            transform::merge(),
            transform::organize_full(
                &["Time"],
                &[
                    "namespace",
                    "pod",
                    CPU_REQUESTED,
                    CPU_LIMIT,
                    MEMORY_REQUESTED,
                    MEMORY_LIMIT,
                    // The refId spelling, in case Grafana names the columns that
                    // way instead. Ordering entries that match no field are
                    // ignored, so carrying both costs nothing.
                    "Value #query-0",
                    "Value #query-1",
                    "Value #query-2",
                    "Value #query-3",
                ],
                &[
                    ("Value #query-0", CPU_REQUESTED),
                    ("Value #query-1", CPU_LIMIT),
                    ("Value #query-2", MEMORY_REQUESTED),
                    ("Value #query-3", MEMORY_LIMIT),
                ],
            ),
        ])
        // Mixed units in one table, so they cannot be a panel-level default.
        .overrides(vec![
            unit_override(CPU_REQUESTED, CPU_UNIT),
            unit_override(CPU_LIMIT, CPU_UNIT),
            unit_override(MEMORY_REQUESTED, MEMORY_UNIT),
            unit_override(MEMORY_LIMIT, MEMORY_UNIT),
        ])
        .build(0)
}

/// Grafana's IEC byte unit, which scales into KiB/MiB/GiB on its own.
const MEMORY_UNIT: &str = "bytes";
/// Bare cores; a request of 0.1 is a tenth of one.
const CPU_UNIT: &str = "cores";

const CPU_REQUESTED: &str = "CPU Requested";
const CPU_LIMIT: &str = "CPU Limit";
const MEMORY_REQUESTED: &str = "Memory Requested";
const MEMORY_LIMIT: &str = "Memory Limit";

/// Give one named column its own unit.
fn unit_override(field: &str, unit: &str) -> dashboardv2::FieldConfigSourceOverridesItem {
    field_override::by_name(field)
        .property("unit", serde_json::Value::String(unit.to_string()))
        .build()
}
