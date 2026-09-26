// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Logs tab: what the collectors themselves had to say.
//!
//! # How these lines get here
//!
//! The collectors' logs reach Loki through the collectors, by two routes. The
//! agent drops its own pod from the files it tails, and the gateway reads the
//! agents' logs over the Kubernetes API instead. The gateway's own lines are
//! read off disk by the agent on its node, like any other pod's.
//!
//! So an agent in trouble can still describe it, because its lines do not
//! depend on it. A gateway in trouble takes every line with it, the agents'
//! included, and the feed descriptions point at `kubectl logs` for that case.
//!
//! # Error Rate by Component
//!
//! The one panel here that parses lines. Alloy logs in logfmt and names the
//! emitting component in `component_id`, so extracting that one key turns "the
//! gateway is erroring" into "the gateway's Google Cloud exporter is erroring".
//! It reads warning-level lines only, which keeps the parse small, and it sits
//! in the guarded volume row with the other counting panels.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::volume_guard;

const SHADE: &str = theme::LOGS.shade;

const LINES_PER_SECOND: &str = "suffix:logs/s";
const LINES_PER_MINUTE: &str = "suffix:logs/min";

/// What a feed shows when the selection matches nothing.
///
/// A running collector always logs something, so an empty feed means the
/// filters exclude everything or the gateway these lines travel through is down.
fn nothing_matched() -> NoValue {
    NoValue::Custom(
        "No lines match the current filters, or the gateway is not delivering logs".to_string(),
    )
}

fn no_warnings() -> NoValue {
    NoValue::Custom("No warnings in this time range".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        volume(q),
        volume_guard::hidden_row("alloy-volume-hidden-note"),
        warnings(q),
        all_logs(q),
    ]
}

fn volume(q: &Queries) -> Row {
    Row::new("Volume")
        .only_within(volume_guard::THRESHOLD)
        .grid(
            AutoGrid::new(3)
                .panel("alloy-logs-rate", rate_by_role(q))
                .panel("alloy-logs-errors-by-component", errors_by_component(q))
                .panel("alloy-logs-warning-rate", warning_rate(q)),
        )
}

fn warnings(q: &Queries) -> Row {
    Row::new("Warnings").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("alloy-logs-warning-feed", warning_feed(q)),
    )
}

fn all_logs(q: &Queries) -> Row {
    Row::new("All Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("alloy-logs-feed", log_feed(q)),
    )
}

fn rate_by_role(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Rate by Role and Level")
        .query(
            q.logs("infra.alloy.logs.rate.by_role")
                .legend("{{app}} {{level}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

fn errors_by_component(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Error Rate by Component")
        .query(
            q.logs("infra.alloy.logs.errors.by_component")
                .legend("{{component_id}}"),
        )
        .unit(LINES_PER_MINUTE)
        .min(0.0)
        .no_value(no_warnings())
        .build(0)
}

fn warning_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Warning Rate")
        .query(
            q.logs("infra.alloy.logs.warnings.rate")
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
        .query(q.logs("infra.alloy.logs.warnings.stream"))
        .no_value(no_warnings())
        .build(0)
}

fn log_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Logs")
        .query(q.logs("infra.alloy.logs.stream"))
        .no_value(nothing_matched())
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_log_queries;

    fn expr_of(panel: &dashboardv2::PanelKind) -> String {
        panel.spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .expect("spec")["expr"]
            .as_str()
            .expect("expr")
            .to_string()
    }

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_log_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        // Five panels plus the stand-in shown when the volume row hides itself.
        assert_eq!(assembled.elements.len(), 6);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_warning_panels_ignore_the_level_picker() {
        // These answer "is anything wrong", and narrowing the level selection
        // to INFO would silently zero them.
        let q = &test_log_queries();
        for panel in [warning_rate(q), warning_feed(q), errors_by_component(q)] {
            let expr = expr_of(&panel);
            assert!(!expr.contains("$logLevelList"), "{expr}");
            assert!(expr.contains("level=~\"WARN"), "{expr}");
        }
    }

    #[test]
    fn only_the_component_panel_parses_lines() {
        // Parsing reads every line in range. The one panel that needs it reads
        // warning-level lines only; nothing else should parse at all.
        let q = &test_log_queries();
        for panel in [
            rate_by_role(q),
            warning_rate(q),
            warning_feed(q),
            log_feed(q),
        ] {
            assert!(!expr_of(&panel).contains("logfmt"));
        }
        assert!(expr_of(&errors_by_component(q)).contains("| logfmt component_id"));
    }
}
