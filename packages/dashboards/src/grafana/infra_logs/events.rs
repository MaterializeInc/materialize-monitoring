// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Events tab: what Kubernetes said about any of it.
//!
//! Shares its queries with `env-logs`' Events tab rather than redefining them.
//! `materialize.events.cluster.*` carries no Materialize-specific filter — it is
//! scoped by the same Loki-discovered namespace picker both dashboards define —
//! so the only thing that differs here is where that picker opens: on every
//! namespace rather than on the deployment's own.
//!
//! That difference is most of the value. On a Materialize-scoped default the
//! platform's own events are invisible, and they are the majority: on a
//! representative install, `FailedScheduling` alone accounts for 4,538 warnings
//! in a week, against a handful from the operator.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::queries::Queries;

/// The three fields worth a column on an event feed.
const EVENT_FIELDS: [&str; 3] = ["reason", "msg", "name"];

/// What a panel shows when nothing matched. Quiet is the healthy reading here,
/// unlike for logs.
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
    fn it_shares_the_event_queries_rather_than_redefining_them() {
        // Same definitions as `env-logs`. If these ever diverge it should be
        // because the scopes genuinely differ, not because two copies drifted --
        // the only intended difference is where the namespace picker opens.
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
                    expr.contains(r#"job="loki.source.kubernetes_events""#),
                    "{name} is not anchored to the events stream: {expr}"
                );
                assert!(
                    expr.contains("$logNamespaceList"),
                    "{name} is not scoped by the shared namespace picker: {expr}"
                );
            }
        }
    }
}
