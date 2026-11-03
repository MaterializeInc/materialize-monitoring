// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Console tab: the console's nginx, which serves the web app and forwards
//! its API calls to balancerd.
//!
//! nginx tags each request with the block that served it, `location`, and that
//! is the split every panel here draws: `app` and `static` are the console
//! itself, so their errors and latency are the console pod's, while `api` is the
//! hop to balancerd and everything behind it. Server and client errors are
//! separate panels because a steady trickle of `401` is normal — an expired
//! sign-in — and would hide a `502` drawn beside it.
//!
//! The log feed is the access log filtered to server errors, with nginx's own
//! error lines, which carry the reason a request to balancerd failed. Load
//! balancer health checks are filtered out first: on a reference install they
//! are nearly every line, and half of them are answered with a `400`.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{LOG_BASE, RATE_DECIMALS, no_console};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![traffic(q), errors(q), latency(q), logs(q)]
}

fn traffic(q: &Queries) -> Row {
    Row::new("Traffic").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("console-status-class", by_status_class(q))
            .panel("console-websockets", websockets(q))
            .panel("console-connections", connections(q)),
    )
}

fn errors(q: &Queries) -> Row {
    Row::new("Console Errors").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("console-server-errors", server_errors(q))
            .panel("console-client-errors", client_errors(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Response Time").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("console-response-p50", response_p50(q))
            .panel("console-response-p99", response_p99(q)),
    )
}

fn logs(q: &Queries) -> Row {
    Row::new("Console Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("console-logs", error_logs(q)),
    )
}

fn by_status_class(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Requests by Status Class")
        .query(
            q.get("materialize.ingress.console.by_status_class")
                .legend("{{class}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(no_console())
        .build(0)
}

fn websockets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("WebSocket Sessions Closed")
        .query(
            q.get("materialize.ingress.console.websockets")
                .legend("sessions/s"),
        )
        .unit("ops")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No WebSocket session through the console ended in this time range.".to_string(),
        ))
        .build(0)
}

fn connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Client Connections by State")
        .query(
            q.get("materialize.ingress.console.connections")
                .legend("{{state}}"),
        )
        .unit("short")
        .decimals(0.0)
        .log_scale(LOG_BASE)
        .no_value(no_console())
        .build(0)
}

fn server_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Server Errors by Location")
        .query(
            q.get("materialize.ingress.console.server_errors")
                .legend("{{location}} {{status}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(NoValue::Custom(
            "No console request ended in a server error in this time range.".to_string(),
        ))
        .build(0)
}

fn client_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Client Errors by Location")
        .query(
            q.get("materialize.ingress.console.client_errors")
                .legend("{{location}} {{status}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(NoValue::Custom(
            "No console request was rejected in this time range.".to_string(),
        ))
        .build(0)
}

fn response_p50(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Response Time (p50)")
        .query(
            q.get("materialize.ingress.console.response_p50")
                .legend("{{location}}"),
        )
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_console())
        .build(0)
}

fn response_p99(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Response Time (p99)")
        .query(
            q.get("materialize.ingress.console.response_p99")
                .legend("{{location}}"),
        )
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_console())
        .build(0)
}

fn error_logs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Server Errors")
        .query(q.logs("materialize.ingress.console.logs.errors"))
        .no_value(NoValue::Custom(
            "The console logged no server errors in this time range.".to_string(),
        ))
        .build(0)
}
