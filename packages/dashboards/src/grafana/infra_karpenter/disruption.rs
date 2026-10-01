// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Disruption tab: removing and replacing nodes, and what stops it.
//!
//! On a Materialize cluster the most common reading here is *nothing
//! disrupted*, and that is by design: the operator marks every Materialize pod
//! `karpenter.sh/do-not-disrupt`, because moving a replica means rehydrating it,
//! and the examples set the Materialize NodePool's `expireAfter` to `Never`.
//! So the tab leads with what Karpenter did, then with why it did not, broken
//! out of the `DisruptionBlocked` and `Unconsolidatable` events: a
//! `do-not-disrupt` annotation is the design working, a PodDisruptionBudget
//! blocking for hours is not.
//!
//! Interruptions close the tab, since an interruption is a disruption AWS
//! starts rather than Karpenter.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{karpenter_row, no_karpenter_row, not_reported, quiet};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        activity(q),
        blocked(q),
        draining(q),
        ages(q),
        interruptions(q),
        no_karpenter_row("disruption-no-karpenter"),
    ]
}

fn activity(q: &Queries) -> Row {
    karpenter_row("Disruptions").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("disruption-disrupted", disrupted(q))
            .panel("disruption-decisions", decisions(q))
            .panel("disruption-eligible", eligible(q)),
    )
}

fn blocked(q: &Queries) -> Row {
    karpenter_row("What Blocks Disruption").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .row_height(RowHeight::Tall)
            .panel("disruption-blocked", why_blocked(q))
            .panel("disruption-blocked-events", blocked_events(q)),
    )
}

fn draining(q: &Queries) -> Row {
    karpenter_row("Draining").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("disruption-budget", budget(q))
            .panel("disruption-evictions", evictions(q))
            .panel("disruption-termination", termination(q)),
    )
}

fn ages(q: &Queries) -> Row {
    karpenter_row("Node Ages").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("disruption-node-age", node_age(q))
            .panel("disruption-timeouts", consolidation_timeouts(q)),
    )
}

fn interruptions(q: &Queries) -> Row {
    karpenter_row("Interruptions").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("disruption-interruptions", interruptions_received(q))
            .panel("disruption-queue-delay", queue_delay(q)),
    )
}

fn disrupted(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("NodeClaims Disrupted per Hour")
        .query(
            q.get("infra.karpenter.disruption.disrupted")
                .legend("{{nodepool}} {{reason}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No nodes disrupted".to_string()))
        .build(0)
}

fn decisions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disruption Decisions per Hour")
        .query(
            q.get("infra.karpenter.disruption.decisions")
                .legend("{{reason}} {{decision}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No disruption decisions".to_string()))
        .build(0)
}

fn eligible(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes Eligible for Disruption")
        .query(
            q.get("infra.karpenter.disruption.eligible")
                .legend("{{reason}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn why_blocked(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Why Disruption Is Blocked")
        .query(
            q.logs("infra.karpenter.disruption.blocked")
                .legend("{{why}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(quiet("blocked disruptions"))
        .build(0)
}

fn blocked_events(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Blocked Disruption Events")
        .query(q.logs("infra.karpenter.disruption.blocked_stream"))
        .dedup_by_signature()
        .no_value(quiet("blocked disruptions"))
        .build(0)
}

fn budget(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disruptions the Budget Allows")
        .query(
            q.get("infra.karpenter.disruption.budget")
                .legend("{{nodepool}} {{reason}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn evictions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Eviction Requests per Minute")
        .query(
            q.get("infra.karpenter.disruption.evictions")
                .legend("{{code}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No evictions requested".to_string()))
        .build(0)
}

fn termination(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Time to Terminate a Node (p99)")
        .query(
            q.get("infra.karpenter.disruption.termination")
                .legend("p99"),
        )
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No nodes terminated in the last hour".to_string(),
        ))
        .build(0)
}

fn node_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Karpenter's Nodes")
        .query(q.get("infra.karpenter.nodes.age").table_format())
        .unit("s")
        .no_value(not_reported())
        .transformations(vec![
            transform::organize_full(
                &["Time", "__name__"],
                &["node_name", "nodepool", "instance_type", "zone", "Value"],
                &[
                    ("node_name", "Node"),
                    ("nodepool", "NodePool"),
                    ("instance_type", "Instance Type"),
                    ("zone", "Zone"),
                    ("Value", "Age"),
                ],
            ),
            // Youngest first: churn is what this table is for.
            transform::sort_by("Age", false),
        ])
        .build(0)
}

fn consolidation_timeouts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Consolidation Timeouts per Hour")
        .query(
            q.get("infra.karpenter.disruption.consolidation_timeouts")
                .legend("{{consolidation_type}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn interruptions_received(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Interruption Notices per Hour")
        .query(
            q.get("infra.karpenter.interruption.received")
                .legend("{{message_type}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("No interruption notices".to_string()))
        .build(0)
}

fn queue_delay(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Notice Wait in Queue (p99)")
        .query(
            q.get("infra.karpenter.interruption.queue_delay")
                .legend("p99"),
        )
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No interruption notices in the last hour".to_string(),
        ))
        .build(0)
}
