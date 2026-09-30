// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Provisioning tab: adding nodes, what EC2 refused, and how long it took.
//!
//! Rows follow a launch from start to finish: what Karpenter launched, what EC2
//! refused, which capacity it has stopped asking for, then the pods waiting and
//! how long they waited, then where the time went.
//!
//! _Instance Type Availability_ is the one capacity signal Karpenter has that no
//! provider publishes: a zone drops out for an instance type when EC2 refuses a
//! launch there, until Karpenter's few-minute backoff lets it try again.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{karpenter_row, no_karpenter_row, not_reported};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        launches(q),
        waiting(q),
        timing(q),
        no_karpenter_row("provisioning-no-karpenter"),
    ]
}

fn launches(q: &Queries) -> Row {
    karpenter_row("Launches").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("provisioning-created", created(q))
            .panel("provisioning-errors", errors(q))
            .panel("provisioning-offerings", offerings(q)),
    )
}

fn waiting(q: &Queries) -> Row {
    karpenter_row("Pods Waiting").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("provisioning-pods-waiting", pods_waiting(q))
            .panel("provisioning-startup", startup(q)),
    )
}

fn timing(q: &Queries) -> Row {
    karpenter_row("Where the Time Goes").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("provisioning-stages", stages(q))
            .panel("provisioning-scheduling", scheduling(q))
            .panel("provisioning-cloud-latency", cloud_latency(q)),
    )
}

fn created(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("NodeClaims Created per Hour")
        .query(
            q.get("infra.karpenter.provisioning.created")
                .legend("{{nodepool}} {{reason}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No NodeClaims created".to_string()))
        .build(0)
}

fn errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("EC2 Errors per Hour")
        .query(
            q.get("infra.karpenter.provisioning.errors")
                .legend("{{method}} {{error}}"),
        )
        .unit("short")
        .min(0.0)
        // Karpenter creates the counter on the first error.
        .no_value(NoValue::Custom("No errors from EC2".to_string()))
        .build(0)
}

fn offerings(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Instance Type Availability")
        .query(
            q.get("infra.karpenter.provisioning.offerings")
                .legend("{{instance_type}} {{capacity_type}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No availability data. The Terraform karpenter module keeps it only for service_monitor_instance_types."
                .to_string(),
        ))
        .build(0)
}

fn pods_waiting(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pods Waiting for Karpenter")
        .query(q.legended(
            "infra.karpenter.provisioning.pods_waiting",
            &["cannot place", "queued for {{controller}}"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn startup(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Time for a Pod to Start on a New Node")
        .query(q.legended("infra.karpenter.provisioning.startup", &["p50", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No pods needed a new node in the last hour".to_string(),
        ))
        .build(0)
}

fn stages(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Node Launch Stages (p99)")
        .query(
            q.get("infra.karpenter.provisioning.stages")
                .legend("{{type}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No NodeClaims launched in the last hour".to_string(),
        ))
        .build(0)
}

fn scheduling(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Scheduling Pass Duration (p99)")
        .query(
            q.get("infra.karpenter.provisioning.scheduling_duration")
                .legend("{{controller}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn cloud_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("AWS Call Duration (p99)")
        .query(
            q.get("infra.karpenter.provisioning.cloud_latency")
                .legend("{{method}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}
