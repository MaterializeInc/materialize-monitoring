// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Events tab: what the operator and the cluster reported while an upgrade
//! ran.
//!
//! Every panel here reads Kubernetes events out of Loki rather than metrics out
//! of Thanos, so every query is fetched with [`Queries::logs`] and lands on
//! `$logsDatasource`. That is the whole reason this tab exists as its own thing:
//! a rollout's account of itself is a sequence of discrete things that happened,
//! and a time series can only tell you how many of them there were.
//!
//! # Reading order
//!
//! The rows narrow from verdict to cause, which is the order an operator asks the
//! questions in:
//!
//! 1. **Event Summary** — is anything complaining at all.
//! 2. **Rollout** — the operator's own account of the upgrade, phase by phase.
//!    This is the tab's subject; everything below it explains a rollout that did
//!    not finish.
//! 3. **Operator Health** — why reconciliation could not proceed, with the error's
//!    cause chain.
//! 4. **Kubernetes Activity** — what the cluster did underneath. The operator can
//!    be working perfectly and the rollout still stall because the new pods have
//!    nowhere to run, and this row is where that shows.
//! 5. **All Events** — collapsed, for a cause none of the above names.
//!
//! Each rate panel is paired with the feed it summarizes, in the same row: the
//! chart says when, and the feed beneath it says what. Splitting them across rows
//! would make the reader hold a timestamp in their head while they scrolled.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::{BigValueGraphMode, BigValueTextMode};
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;

/// The tab's theme, applied to every shaded panel here.
const SHADE: &str = theme::EVENTS.shade;

/// What a panel shows when the range holds no matching events.
///
/// Absence is the *good* reading on most of this tab — no warnings and no
/// reconciliation failures is what a healthy deployment looks like — so the
/// message says so rather than implying something is missing. `env-top`'s
/// `FilterMismatch` wording would read as a broken panel here.
fn quiet(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} in this time range"))
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        event_summary(q),
        rollout(q),
        operator_health(q),
        kubernetes_activity(q),
        all_events(q),
    ]
}

fn event_summary(q: &Queries) -> Row {
    // Header hidden: three stats under a title saying "summary" would only repeat
    // what the panels already say, and the row sits at the top where it needs no
    // introduction.
    Row::new("Event Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("warning-events", warning_events(q))
            .panel("reconciliation-failures", reconciliation_failure_count(q))
            .panel("lifecycle-transitions", lifecycle_transition_count(q)),
    )
}

fn rollout(q: &Queries) -> Row {
    Row::new("Rollout").grid(
        AutoGrid::new(2)
            .row_height(RowHeight::Tall)
            .panel("lifecycle-rate", lifecycle_rate(q))
            .panel("lifecycle-events", lifecycle_events(q)),
    )
}

fn operator_health(q: &Queries) -> Row {
    Row::new("Operator Health").grid(
        AutoGrid::new(2)
            .row_height(RowHeight::Tall)
            .panel(
                "reconciliation-failure-rate",
                reconciliation_failure_rate(q),
            )
            .panel(
                "reconciliation-failure-events",
                reconciliation_failure_events(q),
            ),
    )
}

fn kubernetes_activity(q: &Queries) -> Row {
    Row::new("Kubernetes Activity").grid(
        AutoGrid::new(2)
            .row_height(RowHeight::Tall)
            .panel("event-rate-by-reason", event_rate_by_reason(q))
            .panel("warning-event-feed", warning_event_feed(q)),
    )
}

fn all_events(q: &Queries) -> Row {
    // Collapsed: it is the same events as every other feed on the tab, unfiltered.
    // Useful when the cause is something none of the filters name, and noise the
    // rest of the time.
    Row::new("All Events").collapsed().grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("all-event-feed", all_event_feed(q)),
    )
}

// ------------------------------------------------------------------ summary

fn warning_events(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Warning Events")
        .query(
            q.logs("materialize.events.deployment.warning.rate")
                .legend("warnings"),
        )
        // A sparkline rather than a bare number: the question this panel answers
        // is whether the warnings are still arriving, which a count cannot say.
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .min(0.0)
        .no_value(quiet("warnings"))
        .build(0)
}

fn reconciliation_failure_count(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Reconciliation Failures")
        .query(
            q.logs("materialize.events.operator.reconciliation.failures.rate")
                .legend("{{kind}}"),
        )
        // Named values: which resource is failing is most of the diagnosis, and
        // `Balancer` failing while `Materialize` is fine is a different morning
        // from the reverse.
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(quiet("reconciliation failures"))
        .build(0)
}

fn lifecycle_transition_count(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Lifecycle Transitions")
        .query(
            q.logs("materialize.events.operator.lifecycle.rate")
                .legend("{{reason}}"),
        )
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(quiet("rollout activity"))
        .build(0)
}

// ------------------------------------------------------------------ rollout

fn lifecycle_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Lifecycle Transitions")
        .query(
            q.logs("materialize.events.operator.lifecycle.rate")
                .legend("{{reason}}"),
        )
        .min(0.0)
        .no_value(quiet("rollout activity"))
        .build(0)
}

fn lifecycle_events(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Lifecycle Events")
        .query(q.logs("materialize.events.operator.lifecycle"))
        .no_value(quiet("rollout activity"))
        .build(0)
}

// ---------------------------------------------------------- operator health

fn reconciliation_failure_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Reconciliation Failures")
        .query(
            q.logs("materialize.events.operator.reconciliation.failures.rate")
                .legend("{{kind}}"),
        )
        .min(0.0)
        .no_value(quiet("reconciliation failures"))
        .build(0)
}

fn reconciliation_failure_events(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Reconciliation Failure Events")
        .query(q.logs("materialize.events.operator.reconciliation.failures"))
        .no_value(quiet("reconciliation failures"))
        .build(0)
}

// ------------------------------------------------------- kubernetes activity

fn event_rate_by_reason(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Event Rate by Reason")
        .query(
            q.logs("materialize.events.deployment.rate.by_reason")
                .legend("{{reason}}"),
        )
        .min(0.0)
        .no_value(quiet("events"))
        .build(0)
}

fn warning_event_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warning Events")
        .query(q.logs("materialize.events.deployment.warnings"))
        .no_value(quiet("warnings"))
        .build(0)
}

// --------------------------------------------------------------- all events

fn all_event_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Events")
        .query(q.logs("materialize.events.deployment.stream"))
        .no_value(quiet("events"))
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 10);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_panel_queries_the_logs_datasource() {
        // The check that matters on this tab: a query fetched with `get` rather
        // than `logs` would render PromQL against Loki, which fails as an empty
        // panel rather than an error.
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                assert_eq!(query.spec.query.group, "loki", "{name} is not a Loki query");
                assert_eq!(
                    query
                        .spec
                        .query
                        .datasource
                        .as_ref()
                        .and_then(|d| d.name.as_deref()),
                    Some("${logsDatasource}"),
                    "{name} does not name the logs datasource"
                );
            }
        }
    }

    #[test]
    fn no_query_uses_the_prometheus_spelling_of_the_interval() {
        // `$__rate_interval` is defined by Grafana's Prometheus datasource and not
        // by its Loki one, so a LogQL query carrying it reaches Loki as literal
        // text and fails to parse. That is louder than an empty panel but only
        // visible in the browser, which is why it is asserted here — the backend
        // API does not interpolate either spelling, so a live query check cannot
        // tell them apart.
        let q = &test_queries();
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
                    !expr.contains("$__rate_interval"),
                    "{name} uses the Prometheus interval variable in a LogQL query: {expr}"
                );
            }
        }
    }

    #[test]
    fn each_rate_panel_sits_beside_the_feed_it_summarizes() {
        // The pairing is the tab's whole layout argument: a chart saying "when"
        // above a feed saying "what". A row that lost one half would leave the
        // other unreadable.
        let q = &test_queries();
        for row in [rollout(q), operator_health(q), kubernetes_activity(q)] {
            let assembled = mzmon_lib::grafana::layout::Layout::rows(vec![row])
                .assemble()
                .expect("assemble");
            let plugins: Vec<String> = assembled
                .elements
                .values()
                .filter_map(|e| match e {
                    dashboardv2::Element::PanelKind(p) => Some(p.spec.viz_config.group.clone()),
                    _ => None,
                })
                .collect();
            assert!(plugins.contains(&"timeseries".to_string()), "{plugins:?}");
            assert!(plugins.contains(&"logs".to_string()), "{plugins:?}");
        }
    }

    #[test]
    fn absence_reads_as_quiet_rather_than_as_a_broken_panel() {
        // No warnings and no reconciliation failures is the healthy reading, so a
        // "no matches for the current filters" message would be actively wrong.
        let q = &test_queries();
        let panel = warning_events(q);
        let no_value = panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .no_value
            .as_deref()
            .expect("a no-value message");
        assert_eq!(no_value, "No warnings in this time range");
    }
}
