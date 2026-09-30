// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Controller tab: Karpenter itself.
//!
//! Where to look when the other tabs say Karpenter is not acting and do not say
//! why: its reconcile errors, its queues, the two APIs it depends on, which
//! replica is leading, and whether it has the memory to keep going. Every
//! generic family here is scoped by `app="karpenter"`, since the EBS CSI driver
//! and the Load Balancer Controller publish the same ones.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{karpenter_row, no_karpenter_row, not_reported};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        work(q),
        apis(q),
        process(q),
        no_karpenter_row("controller-no-karpenter"),
    ]
}

fn work(q: &Queries) -> Row {
    karpenter_row("Reconciliation").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("controller-reconcile-errors", reconcile_errors(q))
            .panel("controller-workqueue", workqueue(q)),
    )
}

fn apis(q: &Queries) -> Row {
    karpenter_row("API Calls").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("controller-aws-errors", aws_errors(q))
            .panel("controller-kube-errors", kube_errors(q)),
    )
}

fn process(q: &Queries) -> Row {
    karpenter_row("Replicas").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("controller-leader", leader(q))
            .panel("controller-cpu", cpu(q))
            .panel("controller-memory", memory(q)),
    )
}

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

fn leader(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Leader")
        .query(q.get("infra.karpenter.controller.leader").legend("{{pod}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(not_reported())
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
