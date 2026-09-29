// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Connections tab: the pools every process keeps to the database.
//!
//! Built around one incident, observed on a reference install: a Cloud SQL
//! instance ran out of connection slots, and every process logged
//! `remaining connection slots are reserved` at once. From the client's side it
//! looked like this tab — environmentd's pool at seven times its usual size, and
//! thousands of connection errors in a quarter of an hour. The pools grow when
//! the database slows and callers hold connections longer, so a pool that is
//! growing is often the first sign of the slowdown rather than its cause.
//!
//! Held, in use and waiting are three different questions: what the database
//! counts against `max_connections`, what is busy, and what is queued. The
//! pool's configured maximum is not published, which is why nothing here draws a
//! ceiling.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::Panel;

use super::{ORACLE, PROCESS, not_collected};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![pools(q), churn(q)]
}

fn pools(q: &Queries) -> Row {
    Row::new("Connection Pools").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("pool-held", held(q))
            .panel("pool-in-use", in_use(q))
            .panel("pool-waiting", waiting(q))
            .panel("pool-acquire-wait", acquire_wait(q)),
    )
}

fn churn(q: &Queries) -> Row {
    Row::new("Connection Churn").grid(
        AutoGrid::new(1)
            .column_width(ColumnWidth::Wide)
            .panel("pool-churn", new_connections(q)),
    )
}

fn held(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections Held by Process")
        .query(q.legended("materialize.consensus.pool.size", &[PROCESS, ORACLE]))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn in_use(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections in Use by Process")
        .query(q.legended("materialize.consensus.pool.in_use", &[PROCESS, ORACLE]))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn waiting(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Waiting for a Connection by Process")
        .query(q.legended("materialize.consensus.pool.waiting", &[PROCESS, ORACLE]))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn acquire_wait(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Wait (mean)")
        .query(q.legended(
            "materialize.consensus.pool.acquire_wait",
            &[PROCESS, ORACLE],
        ))
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn new_connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("New Connections")
        .query(q.legended(
            "materialize.consensus.pool.churn",
            &["opened", "replaced at age limit"],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}
