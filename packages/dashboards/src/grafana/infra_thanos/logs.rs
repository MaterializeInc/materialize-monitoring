// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Logs tab: Thanos's own logs.
//!
//! The one tab on this dashboard Thanos does not answer. The lines are in Loki,
//! so this is where to go when every other tab is empty, and where a halted
//! compactor says why it halted.

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

fn nothing_matched() -> NoValue {
    NoValue::Custom("No lines match the current filters".to_string())
}

fn no_warnings() -> NoValue {
    NoValue::Custom("No warnings in this time range".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        volume(q),
        volume_guard::hidden_row("thanos-volume-hidden-note"),
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
            q.logs("infra.thanos.logs.rate.by_component")
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
            q.logs("infra.thanos.logs.warnings.rate")
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
        .query(q.logs("infra.thanos.logs.warnings.stream"))
        .no_value(no_warnings())
        .build(0)
}

fn log_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Logs")
        .query(q.logs("infra.thanos.logs.stream"))
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
    fn the_warning_panels_ignore_the_level_picker() {
        // These answer "is anything wrong", and narrowing the level selection
        // to INFO would silently zero them.
        let q = &test_log_queries();
        for panel in [warning_rate(q), warning_feed(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(!json.contains("$logLevelList"), "{json}");
            assert!(json.contains(r#"level=~\"WARN"#), "{json}");
        }
    }
}
