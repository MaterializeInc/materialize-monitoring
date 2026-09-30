// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is the cluster keeping up with its pods.
//!
//! The verdict leads with the two pod counts because they are the symptom a
//! reader arrives with: a Materialize replica that will not start is a pod
//! waiting for a node. Pending and waiting-for-a-node are both shown because
//! they differ in the way that matters: a pod can be Pending with a node,
//! pulling its image, and only the second is an autoscaling question.
//!
//! Below the verdict, demand beside capacity: pods waiting against nodes by
//! pool, then how full the cluster is to the scheduler. A full cluster with
//! nothing waiting is an autoscaler doing its job; nothing waiting is the
//! reading that matters.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::{no_pools, quiet, theme};
use crate::grafana::dependency::zero_is_healthy;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::OVERVIEW.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), demand(q), activity(q)]
}

/// Seven numbers: two about pods, four about nodes, one about workloads.
fn verdict(q: &Queries) -> Row {
    Row::new("Verdict").hide_header().grid(
        AutoGrid::new(7)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-pending", pending(q))
            .panel("overview-unscheduled", unscheduled(q))
            .panel("overview-nodes", nodes(q))
            .panel("overview-not-ready", not_ready(q))
            .panel("overview-added", added(q))
            .panel("overview-removed", removed(q))
            .panel("overview-hpa-at-max", hpa_at_max(q)),
    )
}

fn demand(q: &Queries) -> Row {
    Row::new("Demand and Capacity").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel(
                "overview-unscheduled-by-namespace",
                unscheduled_by_namespace(q),
            )
            .panel("overview-nodes-by-pool", nodes_by_pool(q))
            .panel("overview-requested", requested(q)),
    )
}

fn activity(q: &Queries) -> Row {
    Row::new("Activity").grid(
        AutoGrid::new(2)
            .row_height(RowHeight::Tall)
            .panel("overview-events-by-reason", events_by_reason(q))
            .panel("overview-warnings", warnings(q)),
    )
}

fn pending(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Pods Pending")
        .query(q.get("infra.autoscaling.pods.pending").legend("pending"))
        .thresholds(threshold::errors(1.0, 10.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

pub(crate) fn unscheduled(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Pods Waiting for a Node")
        .query(
            q.get("infra.autoscaling.pods.unscheduled")
                .legend("waiting"),
        )
        .thresholds(threshold::errors(1.0, 5.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn nodes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Nodes")
        .query(q.get("infra.autoscaling.nodes.count").legend("nodes"))
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn not_ready(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Nodes Not Ready")
        .query(
            q.get("infra.autoscaling.nodes.not_ready")
                .legend("not ready"),
        )
        .thresholds(threshold::errors(1.0, 2.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn added(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Nodes Added (Select Time Range)")
        .query(q.get("infra.autoscaling.nodes.added").legend("added"))
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn removed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Nodes Removed (Select Time Range)")
        .query(q.logs("infra.autoscaling.nodes.removed").legend("removed"))
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        // No series at all is the reading when nothing was removed.
        .no_value(NoValue::Custom("None".to_string()))
        .build(0)
}

pub(crate) fn hpa_at_max(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Workloads at Max Replicas")
        .query(q.get("infra.autoscaling.hpa.at_max").legend("at maximum"))
        .thresholds(threshold::errors(1.0, 3.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::Custom("No HorizontalPodAutoscalers".to_string()))
        .build(0)
}

pub(crate) fn unscheduled_by_namespace(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pods Waiting for a Node by Namespace")
        .query(
            q.get("infra.autoscaling.pods.unscheduled_by_namespace")
                .legend("{{namespace}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::Custom("No pods waiting for a node".to_string()))
        .build(0)
}

pub(crate) fn nodes_by_pool(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes by Pool")
        .query(q.get("infra.autoscaling.nodes.by_pool").legend("{{pool}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_pools())
        .build(0)
}

fn requested(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Requested Share of Allocatable")
        .query(q.legended("infra.autoscaling.capacity.requested", &["CPU", "Memory"]))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

pub(crate) fn events_by_reason(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Autoscaling Events by Reason")
        .query(
            q.logs("infra.autoscaling.events.rate_by_reason")
                .legend("{{reason}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(quiet("autoscaling events"))
        .build(0)
}

pub(crate) fn warnings(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warning Events")
        .query(q.logs("infra.autoscaling.events.warnings"))
        .dedup_by_signature()
        .no_value(quiet("warning events"))
        .build(0)
}
