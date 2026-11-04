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
//!
//! On EKS, Karpenter also says how long its answer took: how long each stage of
//! a launch ran, and how long the pods it launched for waited to start. A
//! launch is rare, so both are percentiles over the selected time range rather
//! than lines that are empty but for the hour after one.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueTextMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::karpenter;
use super::{overview, quiet, theme};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::PENDING_PODS.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        waiting(q),
        list(q),
        reasons(q),
        response(q),
        launch_times(q),
    ]
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

fn launch_times(q: &Queries) -> Row {
    karpenter::row("How Long Karpenter Took").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .row_height(RowHeight::Short)
            .panel("pending-karpenter-stages", launch_stages(q))
            .panel("pending-karpenter-startup", startup(q)),
    )
}

fn launch_stages(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Node Launch Stages (p99, Select Time Range)")
        .query(q.legended(
            "infra.karpenter.provisioning.stages",
            &["Launched", "Registered", "Initialized"],
        ))
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("s")
        .no_value(NoValue::Custom(
            "Karpenter launched no nodes in this time range".to_string(),
        ))
        .build(0)
}

fn startup(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Time for a Pod to Start on a New Node (Select Time Range)")
        .query(q.legended("infra.karpenter.provisioning.startup", &["p50", "p99"]))
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("s")
        .no_value(NoValue::Custom(
            "No pods needed a new node in this time range".to_string(),
        ))
        .build(0)
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
