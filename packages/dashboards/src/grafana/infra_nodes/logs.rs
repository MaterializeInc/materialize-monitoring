// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Logs & Events tab: what the node said, and what Kubernetes said about it.
//!
//! The only tab here that reads Loki rather than Thanos, which is what makes this
//! a mixed-datasource dashboard.
//!
//! **Two different accounts of the same machine.** The events are Kubernetes'
//! verdicts about the node — registered, rebooted, went NotReady — and they come
//! first because they are usually what the reader came for: few, dated, and each
//! one a decision somebody or something made. The journal is the node narrating
//! itself, `kubelet` failing to mount a volume or the runtime failing to start a
//! container, and it is where the *explanation* is once an event has said where
//! to look.
//!
//! Both are scoped to the selected node, but not the same way. `node` is
//! structured metadata on journal lines rather than a stream label, so the
//! journal filters in the pipeline; events are matched on the involved object's
//! name, since an event about a node *is* an event whose object is that node.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::queries::Queries;

/// Lines per second, matching the logs dashboards' unit.
const LINES_PER_SECOND: &str = "suffix:logs/s";

/// The fields worth a column on an event feed, in reading order.
///
/// `name` before `msg` for the same reason as on the logs dashboards: the
/// message is free text of no fixed width, so anything to its right is pushed off
/// the visible line.
const EVENT_FIELDS: [&str; 3] = ["reason", "name", "msg"];

fn no_journal() -> NoValue {
    NoValue::Custom("No journal lines from this node — is journal collection enabled?".to_string())
}

fn quiet_events() -> NoValue {
    NoValue::Custom("No events for this node, which is the healthy reading.".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![events(q), journal_volume(q), journal(q)]
}

fn journal_volume(q: &Queries) -> Row {
    Row::new("Journal Volume").grid(AutoGrid::new(1).panel("node-journal-rate", rate_by_unit(q)))
}

fn journal(q: &Queries) -> Row {
    Row::new("Node Journal").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("node-journal-warnings", warnings(q))
            .panel("node-journal-feed", feed(q)),
    )
}

fn events(q: &Queries) -> Row {
    Row::new("Kubernetes Events").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("node-event-rate", event_rate(q))
            .panel("node-event-feed", event_feed(q)),
    )
}

fn rate_by_unit(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Journal Rate by Unit")
        .query(
            q.logs("infra.nodes.journal.rate.by_unit")
                .legend("{{unit}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(no_journal())
        .build(0)
}

fn warnings(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Journal Warnings and Errors")
        .query(q.logs("infra.nodes.journal.warnings"))
        .no_value(NoValue::Custom(
            "No warnings from this node's journal.".to_string(),
        ))
        .build(0)
}

fn feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Node Journal")
        .query(q.logs("infra.nodes.journal.stream"))
        .no_value(no_journal())
        .build(0)
}

fn event_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Event Rate by Reason")
        .query(
            q.logs("infra.nodes.events.rate.by_reason")
                .legend("{{reason}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(quiet_events())
        .build(0)
}

fn event_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Node Events")
        .query(q.logs("infra.nodes.events.stream"))
        .displayed_fields(EVENT_FIELDS)
        .no_value(quiet_events())
        .build(0)
}
