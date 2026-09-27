// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Events tab: what Kubernetes reported about the collectors.
//!
//! The answer to questions no collector metric can answer about itself: why a
//! gateway pod was killed, why an agent never scheduled onto a new node, why a
//! certificate volume did not mount, and when the autoscaler resized the
//! gateway. Events before logs, as on `infra-nodes`, because there are few of
//! them, each is dated, and each is a decision.
//!
//! # Matched by name
//!
//! Events carry the *involved object's* name and namespace, not the pod labels
//! the rest of this dashboard scopes on. So the role picker is matched against
//! `name` as a prefix, which reaches the DaemonSet, the Deployment, its
//! ReplicaSets, the autoscaler, and every pod, since all of them are named after
//! the role. The picker is embedded in a longer pattern there, so it is written
//! `${alloyRole:regex}`; its custom "All" value passes through unformatted.
//!
//! The pre-install validation Jobs are the one set of objects not named after a
//! role: they are `<release>-validate-agent` and `<release>-validate-gateway`,
//! and they run in each role's namespace before every install and upgrade. A
//! second pattern matches them. It ignores the role picker, since the two run
//! together and a failed one stops the whole upgrade either way.
//!
//! The feed renders `reason`, `name` and `msg` as columns, the same fields
//! `env-logs` shows, so an operator moving between the two reads one layout.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::queries::Queries;

const EVENT_FIELDS: [&str; 3] = ["reason", "name", "msg"];

fn quiet(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} about the collectors in this time range"))
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![activity(q), warnings(q), all_events(q)]
}

fn activity(q: &Queries) -> Row {
    Row::new("Activity").grid(
        AutoGrid::new(2)
            .panel("alloy-event-rate-by-reason", rate_by_reason(q))
            .panel("alloy-event-rate-by-kind", rate_by_kind(q)),
    )
}

fn warnings(q: &Queries) -> Row {
    Row::new("Warning Events").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("alloy-warning-event-feed", warning_feed(q)),
    )
}

fn all_events(q: &Queries) -> Row {
    Row::new("All Events").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("alloy-event-feed", event_feed(q)),
    )
}

fn rate_by_reason(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Event Rate by Reason")
        .query(
            q.logs("infra.alloy.events.rate.by_reason")
                .legend("{{reason}}"),
        )
        .min(0.0)
        .no_value(quiet("events"))
        .build(0)
}

fn rate_by_kind(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Event Rate by Object Kind")
        .query(q.logs("infra.alloy.events.rate.by_kind").legend("{{kind}}"))
        .min(0.0)
        .no_value(quiet("events"))
        .build(0)
}

fn warning_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warning Events")
        .query(q.logs("infra.alloy.events.warnings"))
        .displayed_fields(EVENT_FIELDS)
        .dedup_by_signature()
        .no_value(quiet("warning events"))
        .build(0)
}

fn event_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Events")
        .query(q.logs("infra.alloy.events.stream"))
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
    fn every_event_query_is_anchored_to_the_event_stream_and_the_collectors() {
        // The event job is the non-empty equality matcher that keeps the
        // selector parseable, and the name prefix is what keeps the feed to
        // the collectors' own objects.
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
                    "{name}: {expr}"
                );
                assert!(
                    expr.contains(
                        r#"| name=~"(${alloyRole:regex}).*|.+-validate-(agent|gateway)(-.*)?""#
                    ),
                    "{name}: {expr}"
                );
                assert!(expr.contains(r#"| name=~"$alloyPod""#), "{name}: {expr}");
            }
        }
    }
}
