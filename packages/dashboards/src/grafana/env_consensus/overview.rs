// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is the metadata database working for this environment.
//!
//! The verdict row answers in seven numbers. Four are health — failures,
//! connection errors, the commit tail, and callers queued on a pool — and three
//! are scale: connections, commits, and the durable collections that drive
//! them. "Is 80 commits a second a lot" and "is 60 connections a lot" are the
//! first things asked when the database's own dashboard looks busy, and the
//! collection count is the number the database's sizing guidance is stated in.
//!
//! The Logs row closes the tab because it is the one place the database's own
//! error text reaches this dashboard. A failure counter says *that* calls are
//! failing; `connection refused`, `remaining connection slots are reserved` and
//! `password authentication failed` say which of three different problems it is.
//! All three have been seen on the reference installs.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::{ORACLE, ORACLE_OP, PROCESS, latency_ladder, not_collected, theme, zero_is_healthy};
use crate::grafana::queries::Queries;

const SHADE: &str = theme::OVERVIEW.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), latency(q), failures(q), logs(q)]
}

/// The seven numbers worth reading first: four about health, three about scale.
fn verdict(q: &Queries) -> Row {
    Row::new("Verdict").hide_header().grid(
        AutoGrid::new(7)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-failed-ops", failed_ops(q))
            .panel("overview-connection-errors", connection_errors(q))
            .panel("overview-commit-p99", commit_p99(q))
            .panel("overview-waiting", waiting(q))
            .panel("overview-connections-held", connections_held(q))
            .panel("overview-commits", commits(q))
            .panel("overview-collections", collections(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Latency").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-commit-latency", commit_latency(q))
            .panel("overview-round-trip", round_trip(q)),
    )
}

fn failures(q: &Queries) -> Row {
    Row::new("Failures").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-failures-by-op", failures_by_operation(q))
            .panel(
                "overview-connection-errors-by-process",
                connection_errors_by_process(q),
            ),
    )
}

fn logs(q: &Queries) -> Row {
    Row::new("Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("overview-retry-logs", retry_logs(q)),
    )
}

fn failed_ops(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Failed Operations")
        .query(
            q.get("materialize.consensus.health.failed_ops")
                .legend("failed/s"),
        )
        .thresholds(threshold::errors(0.01, 1.0).build())
        .unit("ops")
        .no_value(not_collected())
        .build(0)
}

fn connection_errors(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Connection Errors")
        .query(
            q.get("materialize.consensus.health.connection_errors")
                .legend("errors/s"),
        )
        .thresholds(threshold::errors(0.01, 1.0).build())
        .unit("cps")
        .no_value(not_collected())
        .build(0)
}

fn commit_p99(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Commit Latency (p99)")
        .query(
            q.get("materialize.consensus.health.commit_latency_p99")
                .legend("p99"),
        )
        .color_background()
        .thresholds(latency_ladder(0.1, 1.0))
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn waiting(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Waiting for a Connection")
        .query(
            q.get("materialize.consensus.health.waiting")
                .legend("waiting"),
        )
        .thresholds(threshold::errors(1.0, 10.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_collected())
        .build(0)
}

fn connections_held(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Connections Held")
        .query(
            q.get("materialize.consensus.health.connections_held")
                .legend("connections"),
        )
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn commits(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Commits")
        .query(
            q.get("materialize.consensus.health.commits")
                .legend("commits/s"),
        )
        .shade(SHADE)
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn commit_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Commit Latency")
        .query(q.legended("materialize.consensus.latency.commit", &["p50", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn round_trip(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Round-Trip Time by Process")
        .query(
            q.get("materialize.consensus.latency.round_trip")
                .legend(PROCESS),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn failures_by_operation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed Operations by Type")
        .query(q.legended(
            "materialize.consensus.failures.by_operation",
            &["{{op}}", ORACLE_OP],
        ))
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn connection_errors_by_process(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Errors by Process")
        .query(q.legended(
            "materialize.consensus.failures.connection_errors",
            &[PROCESS, ORACLE],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn retry_logs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Retrying Metadata Database Operations")
        .query(q.logs("materialize.consensus.logs.retries"))
        .no_value(NoValue::Custom(
            "No retried metadata database calls were logged in this time range.".to_string(),
        ))
        .build(0)
}

fn collections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Durable Collections")
        .query(
            q.get("materialize.consensus.ops.collections")
                .legend("collections"),
        )
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}
