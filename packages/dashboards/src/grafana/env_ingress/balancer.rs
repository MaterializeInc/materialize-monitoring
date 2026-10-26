// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Balancer tab: balancerd, which every SQL and HTTPS connection passes
//! through.
//!
//! balancerd counts a connection when it closes, so the rates trail long SQL
//! sessions by their whole length; open connections are the live view, and
//! lead the tab. Capacity is the failure balancerd causes itself: a pod past
//! its high watermark leaves the load balancer's rotation, and one at its limit
//! refuses connections outright. Latency splits a slow connection into the part
//! balancerd spends and the part environmentd does. The log feed closes the tab
//! because it is the one place an `error` connection's reason reaches it.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{no_balancer, no_timings};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![connections(q), capacity(q), latency(q), logs(q)]
}

fn connections(q: &Queries) -> Row {
    Row::new("Connections").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-outcomes", outcomes(q))
            .panel("balancer-open-by-pod", open_by_pod(q))
            .panel("balancer-waiting", waiting_for_backend(q)),
    )
}

fn capacity(q: &Queries) -> Row {
    Row::new("Capacity").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-capacity", against_the_limit(q))
            .panel("balancer-accepting", accepting_by_pod(q))
            .panel("balancer-refused", refused(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Connection Timing").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("balancer-setup", setup_quantiles(q))
            .panel("balancer-upstream-connect", upstream_connect(q))
            .panel("balancer-lifetime", lifetime(q)),
    )
}

fn logs(q: &Queries) -> Row {
    Row::new("Balancer Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("balancer-logs", warnings(q)),
    )
}

fn outcomes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Outcomes")
        .query(
            q.get("materialize.ingress.balancer.outcomes")
                .legend("{{source}} {{status}}"),
        )
        .unit("cps")
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn open_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections by Pod")
        .query(
            q.get("materialize.ingress.balancer.open_by_pod")
                .legend("{{pod}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn waiting_for_backend(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections Not Yet Routed")
        .query(
            q.get("materialize.ingress.balancer.waiting_for_backend")
                .legend("{{source}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn against_the_limit(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections Against the Limit")
        .query(q.legended(
            "materialize.ingress.balancer.capacity",
            &["{{pod}}", "limit", "high watermark", "low watermark"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn accepting_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Taking New Connections by Pod")
        .query(
            q.get("materialize.ingress.balancer.accepting_by_pod")
                .legend("{{pod}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "balancerd does not report its capacity state. Older Materialize versions don't \
             publish it."
                .to_string(),
        ))
        .build(0)
}

fn refused(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Refused Connections")
        .query(q.legended(
            "materialize.ingress.balancer.refused",
            &["connection limit", "{{source}} timed out before routing"],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(no_balancer())
        .build(0)
}

fn setup_quantiles(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Setup Quantiles")
        .query(q.legended(
            "materialize.ingress.balancer.setup_time",
            &["{{source}} p50", "{{source}} p90", "{{source}} p99"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(no_timings())
        .build(0)
}

fn upstream_connect(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Time to Reach environmentd")
        .query(q.legended(
            "materialize.ingress.balancer.upstream_connect",
            &["{{source}} p50", "{{source}} p99"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(no_timings())
        .build(0)
}

fn lifetime(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Lifetime")
        .query(q.legended(
            "materialize.ingress.balancer.lifetime",
            &["{{source}} p50", "{{source}} p99"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(no_timings())
        .build(0)
}

fn warnings(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warnings and Errors")
        .query(q.logs("materialize.ingress.balancer.logs.warnings"))
        .no_value(NoValue::Custom(
            "balancerd logged no warnings or errors in this time range.".to_string(),
        ))
        .build(0)
}
