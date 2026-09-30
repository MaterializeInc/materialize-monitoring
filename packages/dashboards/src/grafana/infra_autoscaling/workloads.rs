// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Workload Autoscaling tab: workloads that scale by adding pods.
//!
//! HorizontalPodAutoscalers are the other half of autoscaling: they add pods,
//! and the pods are what make an autoscaler add nodes. On a Materialize
//! cluster they run the monitoring stack's own components and anything else
//! the operator scales that way. Materialize's cluster replicas are sized, not
//! autoscaled.
//!
//! The table leads because the four numbers only mean something side by side:
//! desired above current is a scale-up waiting on something, current at the
//! maximum is a workload out of room.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::quiet;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const CURRENT: &str = "Current";
const DESIRED: &str = "Desired";
const MIN: &str = "Min";
const MAX: &str = "Max";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![replicas(q), health(q), events(q)]
}

fn replicas(q: &Queries) -> Row {
    Row::new("Replicas").grid(AutoGrid::new(1).panel("hpa-table", table(q)))
}

/// Two timeseries rather than the Overview's stat: a stat alone in a wide row
/// stretches across the page. The count of autoscalers at their maximum is on
/// Overview; here is which ones, and for how long.
fn health(q: &Queries) -> Row {
    Row::new("Room to Scale").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("hpa-share-of-max", share_of_max(q))
            .panel("hpa-unable", unable_to_scale(q)),
    )
}

fn events(q: &Queries) -> Row {
    Row::new("Events").grid(
        AutoGrid::new(2)
            .row_height(RowHeight::Tall)
            .panel("hpa-events-by-reason", events_by_reason(q))
            .panel("hpa-events", event_feed(q)),
    )
}

/// One row per autoscaler, its four numbers side by side.
///
/// The four expressions arrive as separate frames, joined on the label columns
/// by `merge` and renamed off their refIds, as on Infrastructure Node Detail's
/// Requests and Limits table.
fn table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("HorizontalPodAutoscalers")
        .query(
            q.legended(
                "infra.autoscaling.hpa.replicas",
                &[CURRENT, DESIRED, MIN, MAX],
            )
            .table_format(),
        )
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::Custom("No HorizontalPodAutoscalers".to_string()))
        .transformations(vec![
            transform::merge(),
            transform::organize_full(
                &["Time"],
                &[
                    "namespace",
                    "horizontalpodautoscaler",
                    CURRENT,
                    DESIRED,
                    MIN,
                    MAX,
                    "Value #query-0",
                    "Value #query-1",
                    "Value #query-2",
                    "Value #query-3",
                ],
                &[
                    ("namespace", "Namespace"),
                    ("horizontalpodautoscaler", "Autoscaler"),
                    ("Value #query-0", CURRENT),
                    ("Value #query-1", DESIRED),
                    ("Value #query-2", MIN),
                    ("Value #query-3", MAX),
                ],
            ),
        ])
        .build(0)
}

fn share_of_max(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Replicas as a Share of Maximum")
        .query(
            q.get("infra.autoscaling.hpa.share_of_max")
                .legend("{{namespace}}/{{horizontalpodautoscaler}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom("No HorizontalPodAutoscalers".to_string()))
        .build(0)
}

fn unable_to_scale(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Autoscalers Unable to Scale")
        .query(
            q.get("infra.autoscaling.hpa.unable_to_scale")
                .legend("{{condition}} false"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::Custom("No HorizontalPodAutoscalers".to_string()))
        .build(0)
}

fn events_by_reason(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Autoscaler Events by Reason")
        .query(
            q.logs("infra.autoscaling.events.hpa_rate")
                .legend("{{reason}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(quiet("autoscaler events"))
        .build(0)
}

fn event_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Autoscaler Events")
        .query(q.logs("infra.autoscaling.events.hpa_stream"))
        .dedup_by_signature()
        .no_value(quiet("autoscaler events"))
        .build(0)
}
