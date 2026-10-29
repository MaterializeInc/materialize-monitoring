// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The SQL and HTTP API tab: environmentd, where balancerd and the console both
//! lead.
//!
//! This is the far side of every hop on the other tabs, so a fault that shows
//! here as well as there is environmentd's, and one that shows only there is
//! the hop's. environmentd's HTTP server also answers the operator and the
//! metric scrapes, so every HTTP panel reads only the `external` listener
//! balancerd forwards to.
//!
//! The last row is the console as environmentd sees it: the SQL it runs to draw
//! each page, and what users type into its shell. The console's own latency
//! cannot show either, because the shell is a WebSocket and nginx does not time
//! those.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{LOG_BASE, RATE_DECIMALS, no_environmentd};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![http(q), sql(q), console_queries(q)]
}

fn http(q: &Queries) -> Row {
    Row::new("HTTP API").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("api-http-requests", http_requests(q))
            .panel("api-http-errors", http_errors(q))
            .panel("api-http-response", http_response(q)),
    )
}

fn sql(q: &Queries) -> Row {
    Row::new("SQL Connections").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("api-sessions", sessions(q))
            .panel("api-pgwire", pgwire_connections(q)),
    )
}

fn console_queries(q: &Queries) -> Row {
    Row::new("Console Queries").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("api-console-commands", console_commands(q))
            .panel("api-console-latency", console_latency(q)),
    )
}

fn http_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("HTTP Requests by Path")
        .query(
            q.get("materialize.ingress.api.http_requests")
                .legend("{{path}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(no_environmentd())
        .build(0)
}

fn http_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("HTTP Server Errors by Path")
        .query(
            q.get("materialize.ingress.api.http_errors")
                .legend("{{path}} {{status}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(NoValue::Custom(
            "environmentd answered no HTTP request with a server error in this time range."
                .to_string(),
        ))
        .build(0)
}

fn http_response(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("HTTP Response Time (p99)")
        .query(
            q.get("materialize.ingress.api.http_response_p99")
                .legend("{{path}}"),
        )
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_environmentd())
        .build(0)
}

fn sessions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Sessions")
        .query(
            q.get("materialize.connections.sessions.active")
                .legend("{{session_type}}"),
        )
        .unit("short")
        .decimals(0.0)
        .log_scale(LOG_BASE)
        .no_value(no_environmentd())
        .build(0)
}

fn pgwire_connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("SQL Connections Completed")
        .query(
            q.get("materialize.ingress.api.pgwire_connections")
                .legend("{{status}}"),
        )
        .unit("cps")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(no_environmentd())
        .build(0)
}

fn console_commands(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Console SQL Commands")
        .query(
            q.get("materialize.ingress.api.console_commands")
                .legend("{{application_name}} {{status}}"),
        )
        .unit("ops")
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(NoValue::Custom(
            "The console ran no SQL in this environment in this time range.".to_string(),
        ))
        .build(0)
}

fn console_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Console Query Time to First Row (p99)")
        .query(
            q.get("materialize.ingress.api.console_query_latency")
                .legend("{{application_name}}"),
        )
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(NoValue::Custom(
            "The console ran no queries in this environment in this time range.".to_string(),
        ))
        .build(0)
}
