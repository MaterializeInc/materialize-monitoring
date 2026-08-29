// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Events tab: what Kubernetes said about the workloads.
//!
//! The general-purpose counterpart to the upgrade dashboard's Events tab. That
//! one is rollout-scoped — it filters by generation and picks the operator out by
//! reporting controller, because it is answering "is this upgrade going through".
//! This one deliberately carries none of those filters: a general event browser
//! should not quietly drop an event for belonging to the wrong side of a rollout.
//!
//! Scoped by the same Loki-discovered namespace picker as the Logs tab, so
//! turning the Materialize-only switch off reaches `kube-system` and the platform
//! underneath — which is where the answer lives when the question is about nodes
//! or storage rather than about Materialize.
//!
//! No panel here takes the tab's shade, because none of them is a stat — the
//! timeseries and log panels colour by series, and forcing one hue across them
//! would make the reasons indistinguishable from each other.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::queries::Queries;

/// The three fields worth a column on an event feed.
///
/// An event carries a dozen more — resource versions, forwarding addresses, the
/// source component — and none of them is what you are reading for. Rendering
/// these as columns is also why the event queries do *not* reformat the line:
/// displayed fields supersede the raw line, so a `line_format` would be work
/// thrown away.
///
/// `reason` first because it is what you scan; `msg` carries the detail; `name`
/// says which object it happened to.
const EVENT_FIELDS: [&str; 3] = ["reason", "name", "msg"];

/// What a panel shows when nothing matched.
///
/// Quiet is the healthy reading for events, unlike for logs.
fn quiet(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} in this time range"))
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![activity(q), warnings(q), all_events(q)]
}

fn activity(q: &Queries) -> Row {
    Row::new("Activity").grid(
        AutoGrid::new(2)
            .panel("event-rate-by-reason", rate_by_reason(q))
            .panel("event-rate-by-namespace", rate_by_namespace(q)),
    )
}

fn warnings(q: &Queries) -> Row {
    Row::new("Warnings").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("warning-event-feed", warning_feed(q)),
    )
}

fn all_events(q: &Queries) -> Row {
    Row::new("All Events").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("event-feed", event_feed(q)),
    )
}

fn rate_by_reason(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Event Rate by Reason")
        .query(
            q.logs("materialize.events.cluster.rate.by_reason")
                .legend("{{reason}}"),
        )
        .min(0.0)
        .no_value(quiet("events"))
        .build(0)
}

fn rate_by_namespace(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Event Rate by Namespace")
        .query(
            q.logs("materialize.events.cluster.rate.by_namespace")
                .legend("{{namespace}}"),
        )
        .min(0.0)
        .no_value(quiet("events"))
        .build(0)
}

fn warning_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warning Events")
        .query(q.logs("materialize.events.cluster.warnings"))
        .displayed_fields(EVENT_FIELDS)
        .dedup_by_signature()
        .no_value(quiet("warning events"))
        .build(0)
}

fn event_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Events")
        .query(q.logs("materialize.events.cluster.stream"))
        .displayed_fields(EVENT_FIELDS)
        .dedup_by_signature()
        .no_value(quiet("events"))
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_log_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_log_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 4);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn nothing_here_inherits_the_rollout_filters() {
        // The whole reason these are separate query definitions from the upgrade
        // dashboard's. A general browser that dropped events for belonging to
        // another generation, or that only showed the operator's own, would be
        // quietly lying about what happened.
        let q = &test_log_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = query.spec.query.spec.as_ref().expect("spec")["expr"]
                    .as_str()
                    .expect("expr");
                assert!(
                    !expr.contains("mzGenerationList"),
                    "{name} inherits the generation filter: {expr}"
                );
                assert!(
                    !expr.contains("reportingcontroller"),
                    "{name} is narrowed to one reporter: {expr}"
                );
                // And it is scoped by the logs picker, not by the deployment's
                // own namespaces.
                assert!(
                    expr.contains("$logNamespaceList"),
                    "{name} is not scoped by the logs namespace picker: {expr}"
                );
                // Anchored by the event job itself rather than by the logs
                // dashboard's job picker: that is already a non-empty equality
                // matcher, and a second `job` matcher would AND with it and zero
                // the panel the moment a container job was picked.
                assert!(
                    expr.contains(r#"job="loki.source.kubernetes_events""#),
                    "{name} is not anchored to the events stream: {expr}"
                );
                assert!(
                    !expr.contains("$logJobList"),
                    "{name} ANDs a second job matcher onto the events stream: {expr}"
                );
            }
        }
    }
}
