// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Events and Logs tab: Karpenter's record, in its own words.
//!
//! Events first, because each is a decision about a node and they are few.
//! The logs are where the error text is once an event or a panel has said
//! where to look.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::Panel;

use super::{karpenter_row, no_karpenter_row, quiet};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        events(q),
        event_feed(q),
        logs(q),
        no_karpenter_row("events-no-karpenter"),
    ]
}

fn events(q: &Queries) -> Row {
    karpenter_row("Activity").grid(
        AutoGrid::new(2)
            .panel("events-by-reason", events_by_reason(q))
            .panel("events-log-rate", log_rate(q)),
    )
}

fn event_feed(q: &Queries) -> Row {
    karpenter_row("Events").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("events-stream", stream(q)),
    )
}

fn logs(q: &Queries) -> Row {
    karpenter_row("Warnings and Errors").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("events-problems", problems(q)),
    )
}

fn events_by_reason(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Events by Reason")
        .query(
            q.logs("infra.karpenter.events.rate_by_reason")
                .legend("{{reason}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(quiet("Karpenter events"))
        .build(0)
}

fn log_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Lines by Level")
        .query(q.logs("infra.karpenter.logs.rate").legend("{{level}}"))
        .unit("short")
        .min(0.0)
        .no_value(quiet("Karpenter logs"))
        .build(0)
}

fn stream(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Karpenter Events")
        .query(q.logs("infra.karpenter.events.stream"))
        .dedup_by_signature()
        .no_value(quiet("Karpenter events"))
        .build(0)
}

fn problems(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warning and Error Logs")
        .query(q.logs("infra.karpenter.logs.problems"))
        .prettify()
        .no_value(quiet("warnings or errors"))
        .build(0)
}
