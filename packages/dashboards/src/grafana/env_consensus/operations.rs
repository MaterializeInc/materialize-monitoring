// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Operations tab: what the environment asks of the database, how long it
//! takes, and what it has to retry.
//!
//! Load first, because the database's provider-side CPU is only interpretable
//! against it. The number the database is sized by — durable collections — is on
//! the Overview's verdict row rather than here, since it is one number and
//! reads as one.
//!
//! The timestamp oracle gets a row of its own. It is the second, smaller client
//! of the same database, runs only in environmentd, and every query waits on it
//! — which makes it the place a database problem reaches query latency first.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{ORACLE_OP, PROCESS, not_collected};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![load(q), latency(q), contention(q), oracle(q)]
}

fn load(q: &Queries) -> Row {
    Row::new("Load").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ops-by-type", by_type(q))
            .panel("ops-commits-by-process", commits_by_process(q))
            .panel("ops-bytes", bytes(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Latency by Operation").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ops-mean-latency", mean_latency(q))
            .panel(
                "ops-commit-latency-by-process",
                commit_latency_by_process(q),
            ),
    )
}

fn contention(q: &Queries) -> Row {
    Row::new("Contention and Retries").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ops-conflicts", conflicts(q))
            .panel("ops-retries", retries(q)),
    )
}

fn oracle(q: &Queries) -> Row {
    Row::new("Timestamp Oracle").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ops-oracle-ops", oracle_ops(q))
            .panel("ops-oracle-latency", oracle_latency(q)),
    )
}

fn by_type(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Operations by Type")
        .query(q.get("materialize.consensus.ops.by_type").legend("{{op}}"))
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn commits_by_process(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Commits by Process")
        .query(
            q.get("materialize.consensus.ops.commits_by_process")
                .legend(PROCESS),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn bytes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Data Exchanged by Operation")
        .query(q.get("materialize.consensus.ops.bytes").legend("{{op}}"))
        .unit("Bps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn mean_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Mean Latency by Operation")
        .query(
            q.get("materialize.consensus.latency.mean_by_operation")
                .legend("{{op}}"),
        )
        .unit("s")
        .min(0.0)
        // A mean is a rate over a rate, and an operation nobody called in the
        // window divides zero by zero -- `consensus_list_keys` is the usual one.
        // Prometheus drops those series rather than drawing zero.
        .no_value(not_collected())
        .build(0)
}

fn commit_latency_by_process(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Commit Latency by Process (p99)")
        .query(
            q.get("materialize.consensus.latency.commit_by_process")
                .legend(PROCESS),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn conflicts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write Conflicts by Process")
        .query(
            q.get("materialize.consensus.contention.conflicts")
                .legend(PROCESS),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn retries(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Retried Operations")
        .query(q.legended(
            "materialize.consensus.retries.by_operation",
            &["{{op}}", ORACLE_OP],
        ))
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn oracle_ops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Timestamp Oracle Operations")
        .query(q.get("materialize.consensus.oracle.ops").legend("{{op}}"))
        .unit("ops")
        .min(0.0)
        .no_value(no_oracle())
        .build(0)
}

fn oracle_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Timestamp Oracle Latency (mean)")
        .query(
            q.get("materialize.consensus.oracle.latency")
                .legend("{{op}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(no_oracle())
        .build(0)
}

/// The oracle runs in environmentd alone, so it goes quiet with environmentd.
fn no_oracle() -> NoValue {
    NoValue::Custom(
        "No timestamp oracle metrics. The oracle runs in environmentd, so check that it is up."
            .to_string(),
    )
}
