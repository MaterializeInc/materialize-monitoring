// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Logs tab: what Loki itself had to say about any of it.
//!
//! # The circularity, stated once
//!
//! This tab reads Loki's logs out of Loki. That works right up until the moment
//! it matters most: if the store is broken badly enough, this tab is empty for
//! the same reason every other tab is, and the emptiness is indistinguishable
//! from a quiet cluster.
//!
//! It is still worth having, because most Loki problems are not that problem —
//! a compactor erroring, an ingester refusing a stream, an object-storage
//! credential going stale all leave the store perfectly able to describe them.
//! The tab is for those. For the other case the answer is `kubectl logs`, and the
//! feed's own description says so rather than leaving a reader to work it out
//! during an incident.
//!
//! # Scope
//!
//! `app="loki"` plus the dashboard's component picker, which is the *metrics*
//! picker applied to Loki's `component` log label — the two are the same
//! Kubernetes container name. See
//! [`variable::loki_components`](mzmon_lib::grafana::variable::loki_components)
//! for why one control serves both engines here.
//!
//! The volume row carries the same time-range guard as every other log-volume row
//! in this repository: counting lines means reading them, and the cost is linear
//! in the range.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::volume_guard;

const SHADE: &str = theme::LOGS.shade;

/// Grafana has no unit for log lines, so these are its custom-suffix form.
const LINES_PER_SECOND: &str = "suffix:logs/s";
const LINES_PER_MINUTE: &str = "suffix:logs/min";

/// What a feed shows when the selection matches nothing.
///
/// Silence is not the healthy reading for logs: a running process produces them,
/// so an empty feed means the filters exclude everything, collection has stopped,
/// or — the case specific to this dashboard — the store being read is the store
/// that is down.
fn nothing_matched() -> NoValue {
    NoValue::Custom(
        "No lines match the current filters — or Loki cannot serve its own logs".to_string(),
    )
}

fn no_warnings() -> NoValue {
    NoValue::Custom("No warnings in this time range".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        volume(q),
        volume_guard::hidden_row("loki-volume-hidden-note"),
        warnings(q),
        all_logs(q),
    ]
}

fn volume(q: &Queries) -> Row {
    Row::new("Volume")
        .only_within(volume_guard::THRESHOLD)
        .grid(
            AutoGrid::new(2)
                .panel("logs-rate-by-component", rate_by_component(q))
                .panel("logs-warning-rate", warning_rate(q)),
        )
}

fn warnings(q: &Queries) -> Row {
    Row::new("Warnings").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("logs-warning-feed", warning_feed(q)),
    )
}

fn all_logs(q: &Queries) -> Row {
    Row::new("All Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("logs-feed", log_feed(q)),
    )
}

fn rate_by_component(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Rate by Component")
        .query(
            q.logs("infra.loki.logs.rate.by_component")
                .legend("{{component}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

fn warning_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Warning Rate")
        .query(
            q.logs("infra.loki.logs.warnings.rate")
                .legend("warnings/min"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_MINUTE)
        .min(0.0)
        .no_value(no_warnings())
        .build(0)
}

fn warning_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warnings and Errors")
        .query(q.logs("infra.loki.logs.warnings.stream"))
        .no_value(no_warnings())
        .build(0)
}

fn log_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Logs")
        .query(q.logs("infra.loki.logs.stream"))
        .no_value(nothing_matched())
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
        // Four panels plus the stand-in shown when the volume row hides itself.
        assert_eq!(assembled.elements.len(), 5);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_panel_is_scoped_to_loki_itself() {
        // A meta-monitoring log tab that picked up anything else would be a
        // second `infra-logs` with a worse namespace picker.
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
                assert!(expr.contains(r#"app="loki""#), "{name}: {expr}");
                assert!(expr.contains("$lokiNamespace"), "{name}: {expr}");
                assert!(expr.contains("$lokiComponent"), "{name}: {expr}");
            }
        }
    }

    #[test]
    fn the_warning_panels_ignore_the_level_picker() {
        // Same rule as the other logs dashboards: these answer "is anything
        // wrong", and narrowing the level selection to INFO would silently zero
        // them.
        let q = &test_log_queries();
        for panel in [warning_rate(q), warning_feed(q)] {
            let expr = panel.spec.data.spec.queries[0]
                .spec
                .query
                .spec
                .as_ref()
                .expect("spec")["expr"]
                .as_str()
                .expect("expr")
                .to_string();
            assert!(!expr.contains("$logLevelList"), "{expr}");
            assert!(expr.contains("level=~\"WARN"), "{expr}");
        }
    }
}
