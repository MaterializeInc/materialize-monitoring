// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Pending Pods tab: which pods are waiting for a node, and why.
//!
//! The metrics say which pods and for how long; only the events say why. The
//! scheduler's `FailedScheduling` message names the constraint no node met, and
//! the autoscaler's reply — Karpenter's `Nominated`, the cluster autoscaler's
//! `TriggeredScaleUp` or `NotTriggerScaleUp` — says whether a node is coming.
//! The two feeds sit one above the other so a pod's name can be followed from
//! the question to the answer.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{overview, quiet};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![waiting(q), list(q), reasons(q), response(q)]
}

fn waiting(q: &Queries) -> Row {
    Row::new("Waiting").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("pending-unscheduled", overview::unscheduled_by_namespace(q))
            .panel("pending-pending", pending_by_namespace(q))
            .panel("pending-failure-rate", failure_rate(q)),
    )
}

fn list(q: &Queries) -> Row {
    Row::new("Pods Waiting Now").grid(AutoGrid::new(1).panel("pending-list", unscheduled_list(q)))
}

fn reasons(q: &Queries) -> Row {
    Row::new("Why They Wait").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("pending-scheduling-failures", scheduling_failures(q)),
    )
}

fn response(q: &Queries) -> Row {
    Row::new("What the Autoscaler Did").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("pending-scale-up-decisions", scale_up_decisions(q)),
    )
}

fn pending_by_namespace(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pods Pending by Namespace")
        .query(
            q.get("infra.autoscaling.pods.pending_by_namespace")
                .legend("{{namespace}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::Custom("No pods pending".to_string()))
        .build(0)
}

fn failure_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Scheduling Failures per Minute")
        .query(
            q.logs("infra.autoscaling.events.scheduling_failures_rate")
                .legend("{{namespace}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(quiet("scheduling failures"))
        .build(0)
}

fn unscheduled_list(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Pods Waiting for a Node")
        .query(
            q.get("infra.autoscaling.pods.unscheduled_list")
                .table_format(),
        )
        .unit("s")
        // Empty is the healthy reading, so it has to say so.
        .no_value(NoValue::Custom("Every pod has a node.".to_string()))
        .transformations(vec![
            transform::organize_full(
                &["Time", "__name__"],
                &["namespace", "pod", "Value"],
                &[("namespace", "Namespace"), ("pod", "Pod"), ("Value", "Age")],
            ),
            transform::sort_by("Age", true),
        ])
        .build(0)
}

fn scheduling_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Scheduling Failures")
        .query(q.logs("infra.autoscaling.events.scheduling_failures"))
        .dedup_by_signature()
        .no_value(quiet("scheduling failures"))
        .build(0)
}

fn scale_up_decisions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Autoscaler Decisions")
        .query(q.logs("infra.autoscaling.events.scale_up_decisions"))
        .dedup_by_signature()
        .no_value(quiet("autoscaler decisions"))
        .build(0)
}
