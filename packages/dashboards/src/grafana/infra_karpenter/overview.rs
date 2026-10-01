// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is Karpenter keeping up, and with how much.
//!
//! The verdict answers the three questions a reader arrives with, in order:
//! can Karpenter act at all (NodePools Ready, state synced), is it failing to
//! place pods, and has EC2 refused it. Capacity follows, by NodePool, then the
//! NodePool and EC2NodeClass conditions, which are where a Karpenter that
//! launches nothing usually says why.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueTextMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::Panel;
use mzmon_lib::grafana::{palette, threshold};

use super::{karpenter_row, no_karpenter_row, not_reported, theme};
use crate::grafana::dependency::zero_is_healthy;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::OVERVIEW.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        verdict(q),
        capacity(q),
        conditions(q),
        no_karpenter_row("overview-no-karpenter"),
    ]
}

fn verdict(q: &Queries) -> Row {
    karpenter_row("Verdict").hide_header().grid(
        AutoGrid::new(7)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-nodepools-not-ready", nodepools_not_ready(q))
            .panel("overview-synced", synced(q))
            .panel("overview-unplaceable", unplaceable(q))
            .panel("overview-launch-errors", launch_errors(q))
            .panel("overview-nodes", nodes(q))
            .panel("overview-disrupted", disrupted(q))
            .panel("overview-version", version(q)),
    )
}

fn capacity(q: &Queries) -> Row {
    karpenter_row("Capacity").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-nodes-by-nodepool", nodes_by_nodepool(q))
            .panel("overview-utilization", utilization(q))
            .panel("overview-nodepool-cpu", nodepool_cpu(q))
            .panel("overview-nodepool-memory", nodepool_memory(q)),
    )
}

fn conditions(q: &Queries) -> Row {
    karpenter_row("NodePool and NodeClass Conditions")
        .grid(AutoGrid::new(1).panel("overview-conditions", conditions_table(q)))
}

fn nodepools_not_ready(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("NodePools Not Ready")
        .query(
            q.get("infra.karpenter.health.nodepools_not_ready")
                .legend("not ready"),
        )
        .thresholds(threshold::errors(1.0, 1.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_reported())
        .build(0)
}

fn synced(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Cluster State")
        .query(q.get("infra.karpenter.health.synced").legend("state"))
        .color_background()
        .mappings(vec![
            value_map(1.0, "Synced", palette::tri_health::HEALTHY, 1),
            value_map(0.0, "Not Synced", palette::tri_health::UNHEALTHY, 2),
        ])
        .thresholds(
            threshold::Ladder::new(palette::tri_health::UNHEALTHY)
                .step(1.0, palette::tri_health::HEALTHY)
                .build(),
        )
        .no_value(not_reported())
        .build(0)
}

fn unplaceable(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Pods Karpenter Cannot Place")
        .query(
            q.get("infra.karpenter.health.pods_unplaceable")
                .legend("pods"),
        )
        .thresholds(threshold::errors(1.0, 5.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_reported())
        .build(0)
}

fn launch_errors(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Launch Errors (Select Time Range)")
        .query(
            q.get("infra.karpenter.health.launch_errors")
                .legend("errors"),
        )
        .thresholds(threshold::errors(1.0, 5.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_reported())
        .build(0)
}

fn nodes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Nodes")
        .query(q.get("infra.karpenter.health.nodes").legend("nodes"))
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn disrupted(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Nodes Disrupted (Select Time Range)")
        .query(
            q.get("infra.karpenter.health.disrupted")
                .legend("disrupted"),
        )
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn version(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Version")
        .query(
            q.get("infra.karpenter.health.version")
                .legend("{{version}}"),
        )
        .shade(SHADE)
        .text_mode(BigValueTextMode::Name)
        .no_value(not_reported())
        .build(0)
}

fn nodes_by_nodepool(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes by NodePool")
        .query(
            q.get("infra.karpenter.nodes.by_nodepool")
                .legend("{{nodepool}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn utilization(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Requested Share of Karpenter's Nodes")
        .query(
            q.get("infra.karpenter.utilization")
                .legend("{{resource_type}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_reported())
        .build(0)
}

fn nodepool_cpu(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("NodePool CPU Against Limit")
        .query(q.legended(
            "infra.karpenter.nodepools.cpu",
            &["{{nodepool}}", "{{nodepool}} limit"],
        ))
        .unit("short")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn nodepool_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("NodePool Memory Against Limit")
        .query(q.legended(
            "infra.karpenter.nodepools.memory",
            &["{{nodepool}}", "{{nodepool}} limit"],
        ))
        .unit("bytes")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn conditions_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("NodePool and NodeClass Conditions")
        .query(q.get("infra.karpenter.conditions").table_format())
        .no_value(not_reported())
        .transformations(vec![
            transform::organize_full(
                &["Time", "Value", "__name__"],
                &["kind", "name", "type", "status", "reason"],
                &[
                    ("kind", "Kind"),
                    ("name", "Name"),
                    ("type", "Condition"),
                    ("status", "Status"),
                    ("reason", "Reason"),
                ],
            ),
            transform::sort_by("Status", false),
        ])
        .build(0)
}

/// One exact-value mapping, as Infrastructure Node Detail's cordon cell uses.
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
