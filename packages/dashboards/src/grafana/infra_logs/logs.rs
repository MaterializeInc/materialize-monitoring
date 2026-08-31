// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Logs tab: what the platform's own workloads said.
//!
//! The same shape as `env-logs`' Logs tab, with one dimension added that changes
//! what it can answer: **`component`**. `loki` alone splits into `canary`,
//! `querier`, `ingester`, `query-frontend`, `index-gateway`, `compactor`,
//! `distributor` and `ruler`; without that split, "why is Loki slow" is a grep.
//!
//! A `container` picker sits in the controls menu for the other half of the
//! problem: `app` is empty across most of `kube-system`, so container is the only
//! picker that reaches `cilium-agent`, `kubedns` and the rest.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::volume_guard;

/// The tab's theme, applied to every shaded panel here.
const SHADE: &str = theme::LOGS.shade;

/// Grafana has no unit for log lines, so these are its custom-suffix form.
pub(super) const LINES_PER_SECOND: &str = "suffix:logs/s";
pub(super) const LINES_PER_MINUTE: &str = "suffix:logs/min";

/// What a panel shows when the selection matches nothing.
///
/// Silence is not the healthy reading for logs: workloads that are running
/// produce them, so an empty panel means the filters exclude everything or
/// collection has stopped.
pub(super) fn nothing_matched() -> NoValue {
    NoValue::Custom("No logs match the current filters".to_string())
}

/// What a warning panel shows when there is nothing to report.
pub(super) fn no_warnings() -> NoValue {
    NoValue::Custom("No warnings in this time range".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        volume(q),
        volume_guard::hidden_row("volume-hidden-note"),
        warnings(q),
        all_logs(q),
    ]
}

fn volume(q: &Queries) -> Row {
    Row::new("Volume")
        .only_within(volume_guard::THRESHOLD)
        .grid(
            AutoGrid::new(2)
                .panel("log-rate-by-component", rate_by_component(q))
                .panel("log-rate-by-namespace", rate_by_namespace(q))
                .panel("warning-rate", warning_rate(q)),
        )
}

fn warnings(q: &Queries) -> Row {
    Row::new("Warnings").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("warning-feed", warning_feed(q)),
    )
}

fn all_logs(q: &Queries) -> Row {
    Row::new("All Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("log-feed", log_feed(q)),
    )
}

fn rate_by_component(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Rate by Component")
        .query(
            q.logs("infra.logs.rate.by_component")
                .legend("{{app}} / {{component}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

fn rate_by_namespace(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Rate by Namespace")
        .query(
            q.logs("infra.logs.rate.by_namespace")
                .legend("{{namespace}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

fn warning_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Average Warning Rate")
        .query(q.logs("infra.logs.warnings.rate").legend("warnings/min"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_MINUTE)
        .min(0.0)
        .no_value(no_warnings())
        .build(0)
}

fn warning_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warnings and Errors")
        .query(q.logs("infra.logs.warnings.stream"))
        .no_value(no_warnings())
        .build(0)
}

fn log_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Logs")
        .query(q.logs("infra.logs.stream"))
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
        // Five panels plus the stand-in shown when the volume row hides itself.
        assert_eq!(assembled.elements.len(), 6);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_panel_honours_the_component_and_container_pickers() {
        // The reason this tab exists rather than reusing the Materialize one. A
        // panel that ignored either would leave a picker that visibly does
        // nothing, which is worse than not offering it.
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
                assert!(expr.contains("$logComponentList"), "{name}: {expr}");
                assert!(expr.contains("$logContainerList"), "{name}: {expr}");
            }
        }
    }

    #[test]
    fn the_warning_panels_ignore_the_level_picker() {
        let q = &test_log_queries();
        for panel in [warning_rate(q), warning_feed(q)] {
            let title = panel.spec.title.clone();
            let expr = panel.spec.data.spec.queries[0]
                .spec
                .query
                .spec
                .as_ref()
                .expect("spec")["expr"]
                .as_str()
                .expect("expr")
                .to_string();
            assert!(!expr.contains("$logLevelList"), "{title}: {expr}");
            assert!(expr.contains("level=~\"WARN"), "{title}: {expr}");
        }
    }
}
