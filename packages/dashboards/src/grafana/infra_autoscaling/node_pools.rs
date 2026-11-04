// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Node Pools tab: the nodes, by the pool that owns them.
//!
//! A pool is the unit an autoscaler grows, so it is the unit every panel here
//! breaks down by: how many nodes, of what type, in which zone, and how full.
//! "Full" is requests against allocatable, the number the scheduler places by,
//! and deliberately not usage: a pool can be idle and still turn away a pod.
//!
//! On EKS, Karpenter's NodePools follow the requests: a pool can only grow to
//! the limit its NodePool sets, and a NodePool with a false condition launches
//! nothing at all, which no node label shows.
//!
//! The node table closes the tab because age is the one question a count
//! cannot answer: whether the same nodes are being replaced over and over.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::karpenter::{self, not_reported};
use super::{no_pools, overview};
use crate::grafana::queries::Queries;
use crate::grafana::{field_override, transform};

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        nodes(q),
        requested(q),
        karpenter_nodepools(q),
        health(q),
        table(q),
    ]
}

fn nodes(q: &Queries) -> Row {
    Row::new("Nodes").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("pools-nodes-by-pool", overview::nodes_by_pool(q))
            .panel("pools-nodes-by-type", nodes_by_instance_type(q))
            .panel("pools-nodes-by-zone", nodes_by_zone(q)),
    )
}

fn requested(q: &Queries) -> Row {
    Row::new("Requests Against Allocatable").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("pools-cpu-requested", cpu_requested(q))
            .panel("pools-memory-requested", memory_requested(q)),
    )
}

fn karpenter_nodepools(q: &Queries) -> Row {
    karpenter::row("Karpenter NodePools").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("pools-karpenter-nodepools", nodepools(q))
            .panel("pools-karpenter-conditions", conditions(q)),
    )
}

fn health(q: &Queries) -> Row {
    Row::new("Node Health").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("pools-not-ready", not_ready(q))
            .panel("pools-cordoned", cordoned(q)),
    )
}

fn table(q: &Queries) -> Row {
    Row::new("Nodes by Age").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("pools-node-age", node_age(q)),
    )
}

fn nodes_by_instance_type(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes by Instance Type")
        .query(
            q.get("infra.autoscaling.nodes.by_instance_type")
                .legend("{{instance_type}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_pools())
        .build(0)
}

fn nodes_by_zone(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes by Zone")
        .query(q.get("infra.autoscaling.nodes.by_zone").legend("{{zone}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_pools())
        .build(0)
}

fn cpu_requested(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Requested by Pool")
        .query(
            q.get("infra.autoscaling.pools.cpu_requested")
                .legend("{{pool}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(no_pools())
        .build(0)
}

fn memory_requested(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Requested by Pool")
        .query(
            q.get("infra.autoscaling.pools.memory_requested")
                .legend("{{pool}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(no_pools())
        .build(0)
}

fn not_ready(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes Not Ready by Pool")
        .query(
            q.get("infra.autoscaling.pools.not_ready")
                .legend("{{pool}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_pools())
        .build(0)
}

fn cordoned(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Cordoned Nodes by Pool")
        .query(q.get("infra.autoscaling.pools.cordoned").legend("{{pool}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_pools())
        .build(0)
}

fn node_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Nodes")
        // One frame rather than one per node, as the Pods tab of Infrastructure
        // Node Detail explains.
        .query(q.get("infra.autoscaling.nodes.age").table_format())
        .unit("s")
        .no_value(no_pools())
        .transformations(vec![
            transform::organize_full(
                &["Time", "__name__"],
                &["node", "pool", "instance_type", "zone", "Value"],
                &[
                    ("node", "Node"),
                    ("pool", "Pool"),
                    ("instance_type", "Instance Type"),
                    ("zone", "Zone"),
                    ("Value", "Age"),
                ],
            ),
            // Youngest first: a node that keeps being replaced is at the top.
            transform::sort_by("Age", false),
        ])
        .build(0)
}

const NODES: &str = "Nodes";
const CPU: &str = "CPU";
const CPU_LIMIT: &str = "CPU Limit";
const MEMORY: &str = "Memory";
const MEMORY_LIMIT: &str = "Memory Limit";
const DISRUPTIONS: &str = "Disruptions Allowed";

/// One row per NodePool: what it has beside what it may have.
///
/// The six expressions arrive as separate frames, joined on `nodepool` by
/// `merge`, as on Infrastructure Node Detail's Requests and Limits table.
fn nodepools(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("NodePools")
        .query(
            q.legended(
                "infra.karpenter.nodepools",
                &[NODES, CPU, CPU_LIMIT, MEMORY, MEMORY_LIMIT, DISRUPTIONS],
            )
            .table_format(),
        )
        .unit("short")
        .no_value(not_reported())
        .transformations(vec![
            transform::merge(),
            transform::organize_full(
                &["Time"],
                &[
                    "nodepool",
                    NODES,
                    CPU,
                    CPU_LIMIT,
                    MEMORY,
                    MEMORY_LIMIT,
                    DISRUPTIONS,
                    "Value #query-0",
                    "Value #query-1",
                    "Value #query-2",
                    "Value #query-3",
                    "Value #query-4",
                    "Value #query-5",
                ],
                &[
                    ("nodepool", "NodePool"),
                    ("Value #query-0", NODES),
                    ("Value #query-1", CPU),
                    ("Value #query-2", CPU_LIMIT),
                    ("Value #query-3", MEMORY),
                    ("Value #query-4", MEMORY_LIMIT),
                    ("Value #query-5", DISRUPTIONS),
                ],
            ),
        ])
        // Mixed units in one table. A NodePool without a limit leaves its limit
        // cells empty, and the budget comes from another controller than the
        // rest; the panel's own empty-state text would fill either gap.
        .overrides(vec![
            column(CPU, "cores", None),
            column(CPU_LIMIT, "cores", Some("No limit")),
            column(MEMORY, "bytes", None),
            column(MEMORY_LIMIT, "bytes", Some("No limit")),
            karpenter::empty_when_missing(DISRUPTIONS, "—"),
        ])
        .build(0)
}

fn conditions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("NodePool and NodeClass Conditions That Are False")
        .query(q.get("infra.karpenter.conditions").table_format())
        // Empty is the healthy reading, so it has to say so.
        .no_value(NoValue::Custom(
            "No NodePool or EC2NodeClass condition is False.".to_string(),
        ))
        .transformations(vec![transform::organize_full(
            &["Time", "Value", "__name__"],
            &["kind", "name", "type", "reason"],
            &[
                ("kind", "Kind"),
                ("name", "Name"),
                ("type", "Condition"),
                ("reason", "Reason"),
            ],
        )])
        .build(0)
}

/// Give one named column its own unit, and its own text for an empty cell.
fn column(
    field: &str,
    unit: &str,
    empty: Option<&str>,
) -> dashboardv2::FieldConfigSourceOverridesItem {
    let mut builder = field_override::by_name(field)
        .property("unit", serde_json::Value::String(unit.to_string()));
    if let Some(empty) = empty {
        builder = builder.property("noValue", serde_json::Value::String(empty.to_string()));
    }
    builder.build()
}
