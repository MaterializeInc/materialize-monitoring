// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: can users reach the environment.
//!
//! The verdict row answers in seven numbers. Five are health — each hop's error
//! ratio and latency, and whether balancerd is taking new connections — and two
//! are scale, so that a ratio has something to be a ratio of.
//!
//! Below it, one row per RED signal, each with balancerd on the left and the
//! console on the right. Reading down a column follows one component; reading
//! across a row compares the two, which is what places a fault: both failing
//! points behind them, at environmentd, and one failing points at that one.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::{palette, threshold};

use super::{LOG_BASE, no_balancer, no_console, no_timings, theme};
use crate::grafana::dependency::{latency_ladder, value_map, zero_is_healthy};
use crate::grafana::queries::Queries;

const SHADE: &str = theme::OVERVIEW.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), rate(q), errors(q), latency(q)]
}

/// The seven numbers worth reading first: five about health, two about scale.
fn verdict(q: &Queries) -> Row {
    Row::new("Verdict").hide_header().grid(
        AutoGrid::new(7)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-balancer-error-ratio", balancer_error_ratio(q))
            .panel("overview-connection-setup-p99", connection_setup_p99(q))
            .panel("overview-accepting", accepting(q))
            .panel("overview-console-error-ratio", console_error_ratio(q))
            .panel("overview-console-page-p99", console_page_p99(q))
            .panel("overview-open-connections", open_connections(q))
            .panel("overview-console-requests", console_requests(q)),
    )
}

fn rate(q: &Queries) -> Row {
    Row::new("Rate").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-connections", connections_by_protocol(q))
            .panel("overview-console-traffic", console_requests_by_location(q)),
    )
}

fn errors(q: &Queries) -> Row {
    Row::new("Errors").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-balancer-errors", balancer_errors(q))
            .panel("overview-console-errors", console_errors(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Latency").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-connection-setup", connection_setup(q))
            .panel("overview-console-response", console_response(q)),
    )
}

fn balancer_error_ratio(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Connection Error Ratio")
        .query(
            q.get("materialize.ingress.overview.balancer_error_ratio")
                .legend("errors"),
        )
        .thresholds(threshold::errors(0.01, 0.05).build())
        .unit("percentunit")
        .no_value(no_balancer())
        .build(0)
}

fn connection_setup_p99(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Connection Setup (p99)")
        .query(
            q.get("materialize.ingress.overview.connection_setup_p99")
                .legend("p99"),
        )
        .color_background()
        .thresholds(latency_ladder(0.5, 2.0))
        .unit("s")
        .min(0.0)
        .no_value(no_timings())
        .build(0)
}

fn accepting(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Taking New Connections")
        .query(
            q.get("materialize.ingress.overview.accepting")
                .legend("state"),
        )
        .color_background()
        .mappings(vec![
            value_map(1.0, "Yes", palette::tri_health::HEALTHY, 1),
            value_map(0.0, "At Capacity", palette::tri_health::DEGRADED, 2),
        ])
        .thresholds(
            threshold::Ladder::new(palette::tri_health::DEGRADED)
                .step(1.0, palette::tri_health::HEALTHY)
                .build(),
        )
        .no_value(NoValue::Custom(
            "balancerd does not report its capacity state. Older Materialize versions don't \
             publish it."
                .to_string(),
        ))
        .build(0)
}

fn console_error_ratio(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Console Error Ratio")
        .query(
            q.get("materialize.ingress.overview.console_error_ratio")
                .legend("errors"),
        )
        .thresholds(threshold::errors(0.01, 0.05).build())
        .unit("percentunit")
        .no_value(no_console())
        .build(0)
}

fn console_page_p99(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Console Page Load (p99)")
        .query(
            q.get("materialize.ingress.overview.console_page_p99")
                .legend("p99"),
        )
        .color_background()
        .thresholds(latency_ladder(0.5, 2.0))
        .unit("s")
        .min(0.0)
        .no_value(no_console())
        .build(0)
}

fn open_connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Open Connections")
        .query(
            q.get("materialize.ingress.overview.open_connections")
                .legend("connections"),
        )
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn console_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Console Requests")
        .query(
            q.get("materialize.ingress.overview.console_request_rate")
                .legend("requests/s"),
        )
        .shade(SHADE)
        .unit("reqps")
        .min(0.0)
        .no_value(no_console())
        .build(0)
}

fn connections_by_protocol(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections by Protocol")
        .query(
            q.get("materialize.ingress.overview.connections_by_protocol")
                .legend("{{source}}"),
        )
        .unit("cps")
        .log_scale(LOG_BASE)
        .no_value(no_balancer())
        .build(0)
}

fn console_requests_by_location(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Console Requests by Location")
        .query(
            q.get("materialize.ingress.overview.console_requests_by_location")
                .legend("{{location}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .no_value(no_console())
        .build(0)
}

fn balancer_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Errors by Protocol")
        .query(
            q.get("materialize.ingress.overview.balancer_errors_by_protocol")
                .legend("{{source}}"),
        )
        .unit("cps")
        .log_scale(LOG_BASE)
        .no_value(no_balancer())
        .build(0)
}

fn console_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Console Server Errors by Location")
        .query(
            q.get("materialize.ingress.overview.console_errors_by_location")
                .legend("{{location}}"),
        )
        .unit("reqps")
        .log_scale(LOG_BASE)
        .no_value(NoValue::Custom(
            "No console request ended in a server error in this time range.".to_string(),
        ))
        .build(0)
}

fn connection_setup(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Setup Time")
        .query(q.legended(
            "materialize.ingress.overview.connection_setup",
            &["{{source}} p50", "{{source}} p99"],
        ))
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_timings())
        .build(0)
}

fn console_response(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Console Response Time (p99)")
        .query(
            q.get("materialize.ingress.overview.console_response_time")
                .legend("{{location}}"),
        )
        .unit("s")
        .log_scale(LOG_BASE)
        .no_value(no_console())
        .build(0)
}
