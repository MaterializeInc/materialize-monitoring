// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Summary tab: what this machine is, and whether it is fit to run work.
//!
//! The audience is an operator who cannot run `kubectl describe node` — either
//! because they have no access or because they would not know to look there —
//! and who needs to decide one thing: **is this a Materialize problem or one to
//! hand to whoever runs the cluster.** So the tab is ordered as that decision:
//! what the machine is, how hard it is working, how much of it is already
//! promised, and whether Kubernetes still trusts it.
//!
//! The last two rows are the ones that most often end an investigation. A node
//! can be nearly idle and still refuse work, because scheduling is decided by
//! *requests* rather than usage; and a node can be full of capacity and still
//! run nothing, because it is cordoned or tainted. Neither is visible from the
//! Materialize side at all.

use mzmon_lib::grafana::generated::{dashboardv2, stat};
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::{palette, threshold};

use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// What a panel shows when kube-state-metrics is not reporting this node.
fn no_node() -> NoValue {
    NoValue::Custom("No such node — has it been removed from the cluster?".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![node_info(q), utilization(q), allocation(q), scheduling(q)]
}

/// Identity and size: everything here is static for the life of the machine.
fn node_info(q: &Queries) -> Row {
    Row::new("Node Info").grid(
        // Dense on purpose: ten short facts, none of which needs room to breathe.
        AutoGrid::new(8)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel(
                "summary-kubelet-version",
                label_cell(
                    q,
                    "Kubernetes Version",
                    "infra.nodes.info.kubelet",
                    "kubelet_version",
                ),
            )
            .panel(
                "summary-os-image",
                label_cell(q, "OS Image", "infra.nodes.info.os", "os_image"),
            )
            .panel(
                "summary-kernel-version",
                label_cell(q, "Kernel", "infra.nodes.info.kernel", "kernel_version"),
            )
            .panel(
                "summary-container-runtime",
                label_cell(
                    q,
                    "Container Runtime",
                    "infra.nodes.info.runtime",
                    "container_runtime_version",
                ),
            )
            .panel(
                "summary-internal-ip",
                label_cell(q, "Internal IP", "infra.nodes.info.address", "internal_ip"),
            )
            .panel("summary-cpu-capacity", cpu_capacity(q))
            .panel("summary-memory-capacity", memory_capacity(q))
            .panel("summary-pod-capacity", pod_capacity(q))
            .panel("summary-storage-capacity", storage_capacity(q))
            .panel("summary-node-created", node_created(q)),
    )
}

/// What the machine is doing right now, as sparklines.
fn utilization(q: &Queries) -> Row {
    Row::new("Utilization").grid(
        AutoGrid::new(5)
            .row_height(RowHeight::Short)
            .panel("summary-cpu-usage", cpu_usage(q))
            .panel("summary-memory-available", memory_available(q))
            .panel("summary-swap-used", swap_used(q))
            .panel("summary-network-rx", network_rx(q))
            .panel("summary-network-tx", network_tx(q)),
    )
}

/// How much of the machine the scheduler has already given away.
fn allocation(q: &Queries) -> Row {
    Row::new("Allocation").grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel(
                "summary-cpu-allocation",
                allocation_gauge(q, "CPU Requested", "infra.nodes.allocation.cpu"),
            )
            .panel(
                "summary-memory-allocation",
                allocation_gauge(q, "Memory Requested", "infra.nodes.allocation.memory"),
            )
            .panel(
                "summary-pod-allocation",
                allocation_gauge(q, "Pod Slots Used", "infra.nodes.allocation.pods"),
            ),
    )
}

/// Whether Kubernetes will put work here, and what it has been complaining about.
fn scheduling(q: &Queries) -> Row {
    Row::new("Scheduling and Conditions").grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("summary-node-ready", node_ready(q))
            .panel("summary-node-unschedulable", node_unschedulable(q))
            .panel("summary-node-conditions", node_conditions(q))
            .panel("summary-node-taints", node_taints(q))
            .panel("summary-pods-by-namespace", pods_by_namespace(q)),
    )
}

// --- info cells ------------------------------------------------------------

/// A stat showing one label off `kube_node_info` rather than a number.
///
/// `reduce_fields` picks the label out after `labels_to_fields` promotes it, the
/// same trick `env-top` uses for the Materialize version — a stat renders the
/// *value* of a series by default, and every one of these carries the value 1.
///
/// The description comes from the registry query like every other panel's: each
/// cell has its own `infra.nodes.info.*` definition precisely so the prose has
/// somewhere to live that is not this module.
fn label_cell(q: &Queries, title: &str, id: &str, label: &str) -> dashboardv2::PanelKind {
    Panel::stat(title)
        .query(q.get(id).legend(&format!("{{{{{label}}}}}")))
        // Not a time series: a sparkline of a constant would be a flat line
        // saying nothing.
        .graph_mode(stat::BigValueGraphMode::None)
        .value_size(16.0)
        .reduce_fields(format!("/^{label}$/"))
        .transformations(vec![transform::labels_to_fields(&[])])
        .no_value(no_node())
        // Shaded, unlike the graphs: one value per cell, so a fixed colour makes
        // the info row read as one block instead of tinting series apart.
        .shade(theme::SUMMARY.shade)
        .build(0)
}

fn cpu_capacity(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("CPU Capacity")
        .query(q.get("infra.nodes.capacity.cpu"))
        .graph_mode(stat::BigValueGraphMode::None)
        .unit("cores")
        .no_value(no_node())
        .shade(theme::SUMMARY.shade)
        .build(0)
}

fn memory_capacity(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Memory Capacity")
        .query(q.get("infra.nodes.capacity.memory"))
        .graph_mode(stat::BigValueGraphMode::None)
        .unit("bytes")
        .no_value(no_node())
        .shade(theme::SUMMARY.shade)
        .build(0)
}

fn pod_capacity(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Pod Capacity")
        .query(q.get("infra.nodes.capacity.pods"))
        .graph_mode(stat::BigValueGraphMode::None)
        .unit("short")
        .no_value(no_node())
        .shade(theme::SUMMARY.shade)
        .build(0)
}

fn storage_capacity(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Ephemeral Storage")
        .query(q.get("infra.nodes.capacity.ephemeral_storage"))
        .graph_mode(stat::BigValueGraphMode::None)
        .unit("bytes")
        .no_value(no_node())
        .shade(theme::SUMMARY.shade)
        .build(0)
}

fn node_created(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Joined Cluster")
        .query(q.get("infra.nodes.created"))
        .graph_mode(stat::BigValueGraphMode::None)
        .value_size(16.0)
        .unit("dateTimeAsIso")
        .no_value(no_node())
        .shade(theme::SUMMARY.shade)
        .build(0)
}

// --- utilization sparklines ------------------------------------------------

fn cpu_usage(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("CPU Usage")
        .query(q.get("node.cpu.utilization"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .shade(theme::CPU.shade)
        .build(0)
}

fn memory_available(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Memory Available")
        .query(q.get("node.memory.available.ratio"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        // Low is the bad direction here, unlike every other cell in this row —
        // which is why it reads "available" rather than "used". Inverting the
        // number to match the others would misreport what the metric measures.
        .thresholds(threshold::health(0.1, 0.2).build())
        .shade(theme::MEMORY.shade)
        .build(0)
}

fn swap_used(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Swap Used")
        .query(q.get("node.swap.used.ratio"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.1, 1.0, 0.2).build())
        .no_value(NoValue::Custom(
            "No swap configured on this node.".to_string(),
        ))
        .shade(theme::MEMORY.shade)
        .build(0)
}

fn network_rx(q: &Queries) -> dashboardv2::PanelKind {
    // Summed across interfaces rather than split by device: a stat cannot be
    // hovered, so a per-device query reduced to one number shows one arbitrary
    // device and gives no way to tell which. The Network tab keeps the split.
    Panel::stat("Network Rx")
        .query(q.get("node.network.rx.total"))
        .unit("Bps")
        .min(0.0)
        .shade(theme::NETWORK.shade)
        .build(0)
}

fn network_tx(q: &Queries) -> dashboardv2::PanelKind {
    // Summed, for the same reason as Rx.
    Panel::stat("Network Tx")
        .query(q.get("node.network.tx.total"))
        .unit("Bps")
        .min(0.0)
        .shade(theme::NETWORK.shade)
        .build(0)
}

// --- allocation ------------------------------------------------------------

/// A radial gauge reading a fraction of the node already promised.
fn allocation_gauge(q: &Queries, title: &str, id: &str) -> dashboardv2::PanelKind {
    Panel::gauge(title)
        .query(q.get(id))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .threshold_markers(true)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .no_value(NoValue::RequiresKubeStateMetrics)
        .shade(theme::SUMMARY.shade)
        .build(0)
}

// --- scheduling and conditions ---------------------------------------------

fn node_ready(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Ready")
        .query(q.get("infra.nodes.condition.ready"))
        .color_background()
        .graph_mode(stat::BigValueGraphMode::None)
        .mappings(threshold::health_mapping(1.0, 1.0))
        .thresholds(threshold::health(1.0, 1.0).build())
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn node_unschedulable(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Cordoned")
        .query(q.get("infra.nodes.unschedulable"))
        .color_background()
        .graph_mode(stat::BigValueGraphMode::None)
        .mappings(cordon_mapping())
        .thresholds(cordon_ladder())
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn node_conditions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pressure Conditions")
        .query(q.get("infra.nodes.conditions").legend("{{condition}}"))
        .min(0.0)
        .max(1.0)
        .decimals(0.0)
        // A timeseries rather than a stat: these turn on before Ready turns off,
        // so *when* one appeared is the useful part.
        .no_value(NoValue::Custom(
            "No pressure conditions active — the node is not defending itself.".to_string(),
        ))
        .build(0)
}

fn node_taints(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Taints")
        // One frame per taint without this, which the panel renders as a frame
        // picker rather than as a table.
        .query(q.get("infra.nodes.taints").table_format())
        .no_value(NoValue::Custom(
            "No taints — this node accepts any workload that fits.".to_string(),
        ))
        .transformations(vec![transform::organize(
            &["Time", "Value", "node", "__name__"],
            &["key", "value", "effect"],
        )])
        .build(0)
}

fn pods_by_namespace(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Pods by Namespace")
        .query(
            q.get("infra.nodes.pods.by_namespace")
                .legend("{{namespace}}"),
        )
        .unit("short")
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

/// 0/1 read as words, in the direction this metric actually runs.
///
/// **Not [`threshold::health_mapping`], which reads 1 as healthy.** Cordoning
/// inverts that: `kube_node_spec_unschedulable` is 0 when the node is accepting
/// work, so the health mapping labelled a perfectly good node "Unhealthy". The
/// words matter more than the colour here — "Cordoned" is a state somebody chose,
/// often during a node-pool upgrade, rather than a fault.
fn cordon_mapping() -> Vec<dashboardv2::ValueMapping> {
    vec![
        value_map(0.0, "Schedulable", palette::tri_health::HEALTHY, 1),
        value_map(1.0, "Cordoned", palette::tri_health::DEGRADED, 2),
    ]
}

/// Green when accepting work, amber when cordoned — the opposite direction to
/// every other health ladder here, because 0 is the good value.
fn cordon_ladder() -> dashboardv2::ThresholdsConfig {
    threshold::Ladder::new(palette::tri_health::HEALTHY)
        .step(1.0, palette::tri_health::DEGRADED)
        .build()
}

/// One exact-value mapping.
fn value_map(value: f64, text: &str, colour: &str, index: i64) -> dashboardv2::ValueMapping {
    dashboardv2::ValueMapping::ValueMap(dashboardv2::ValueMap {
        type_: dashboardv2::MappingType::Value,
        options: std::collections::BTreeMap::from([(
            format!("{value}"),
            dashboardv2::ValueMappingResult {
                text: Some(text.to_string()),
                color: Some(colour.to_string()),
                index: Some(index),
                icon: None,
            },
        )]),
    })
}
