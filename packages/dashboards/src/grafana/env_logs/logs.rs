// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Logs tab: what the workloads themselves said.
//!
//! A log *explorer* rather than a fixed report. The controls above it do the real
//! work — namespace, app, level, a free-text search — and the panels are there to
//! show what the current selection contains and let you narrow it further.
//!
//! # Reading order
//!
//! 1. **Volume** — how much is arriving and how much of it is a complaint. The
//!    two rate panels are how you decide whether to read the warning feed or the
//!    whole feed.
//! 2. **Warnings** — the warning-and-worse feed, which is the answer often enough
//!    to be worth putting above the general one.
//! 3. **All Logs** — everything matching the selection.
//!
//! The warning panels are deliberately *not* narrowed by the level picker: they
//! answer "is anything wrong", and a selection of `INFO` silently zeroing them
//! would make them lie.

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
///
/// `cps` and `cpm` are the nearest built-ins and render as "c/s" and "c/min",
/// which says *counts* — true but less direct than saying lines.
const LINES_PER_SECOND: &str = "suffix:logs/s";
const LINES_PER_MINUTE: &str = "suffix:logs/min";

/// What a panel shows when the selection matches nothing.
///
/// Unlike the upgrade dashboard's event feeds, silence here is not the healthy
/// reading: workloads that are running produce logs, so an empty panel means the
/// filters exclude everything or collection has stopped.
fn nothing_matched() -> NoValue {
    NoValue::Custom("No logs match the current filters".to_string())
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
    // Guarded on the same threshold as every other volume row -- see
    // `volume_guard`. This dashboard opens on the deployment's own namespaces,
    // which is six times cheaper than the whole cluster, but the picker can be
    // widened to everything and then the cost is identical.
    Row::new("Volume")
        .only_within(volume_guard::THRESHOLD)
        .grid(
            AutoGrid::new(2)
                .panel("log-rate-total", total_rate(q))
                .panel("warning-rate", warning_rate(q))
                .panel("log-rate-by-app", rate_by_app(q))
                .panel("log-rate-by-level", rate_by_level(q)),
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

// -------------------------------------------------------------------- volume

fn total_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Average Log Rate")
        .query(q.logs("materialize.logs.rate.total").legend("lines/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        // Grafana has no logs unit, so this is the custom-suffix form. `cps`
        // would render "c/s", which says counts rather than lines.
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

fn warning_rate(q: &Queries) -> dashboardv2::PanelKind {
    // Per minute, not per second. Warnings are rare enough on a healthy
    // deployment that a per-second rate spends its life showing a small
    // fraction, and "0.6" reads as noise where "38 per minute" reads as a
    // number. The query does the conversion; this only has to agree with it.
    Panel::stat("Average Warning Rate")
        .query(
            q.logs("materialize.logs.warnings.rate")
                .legend("warnings/min"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_MINUTE)
        .min(0.0)
        // Zero warnings is the good reading, so this one says so rather than
        // implying the filters are wrong.
        .no_value(NoValue::Custom(
            "No warnings in this time range".to_string(),
        ))
        .build(0)
}

fn rate_by_app(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Rate by App")
        .query(q.logs("materialize.logs.rate.by_app").legend("{{app}}"))
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

fn rate_by_level(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Rate by Level")
        .query(q.logs("materialize.logs.rate.by_level").legend("{{level}}"))
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(nothing_matched())
        .build(0)
}

// ------------------------------------------------------------------- feeds

fn warning_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Warnings and Errors")
        .query(q.logs("materialize.logs.warnings.stream"))
        .no_value(NoValue::Custom(
            "No warnings in this time range".to_string(),
        ))
        .build(0)
}

fn log_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Logs")
        .query(q.logs("materialize.logs.stream"))
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
        // Six panels plus the stand-in shown when the volume row hides itself.
        assert_eq!(assembled.elements.len(), 7);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_panel_queries_the_logs_datasource() {
        let q = &test_log_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                assert_eq!(query.spec.query.group, "loki", "{name} is not a Loki query");
            }
        }
    }

    #[test]
    fn the_rate_panels_name_their_own_averaging_and_unit() {
        // Two things that drift apart silently: a title that says "average" over
        // a query that is not one, and a unit that disagrees with the query's
        // normalization. The warning rate is the one that has both -- it is
        // per *minute*, converted in the query.
        let q = &test_log_queries();
        for (panel, want_unit) in [
            (total_rate(q), LINES_PER_SECOND),
            (warning_rate(q), LINES_PER_MINUTE),
            (rate_by_app(q), LINES_PER_SECOND),
            (rate_by_level(q), LINES_PER_SECOND),
        ] {
            let title = panel.spec.title.clone();
            let unit = panel
                .spec
                .viz_config
                .spec
                .field_config
                .defaults
                .unit
                .clone()
                .unwrap_or_default();
            assert_eq!(unit, want_unit, "{title}");
        }
        assert_eq!(total_rate(q).spec.title, "Average Log Rate");
        assert_eq!(warning_rate(q).spec.title, "Average Warning Rate");

        // The per-minute unit has to be backed by a query that converts.
        let expr = warning_rate(q).spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .expect("spec")["expr"]
            .as_str()
            .expect("expr")
            .to_string();
        assert!(
            expr.contains("* 60"),
            "warning rate is not normalized: {expr}"
        );
    }

    #[test]
    fn the_warning_panels_ignore_the_level_picker() {
        // They answer "is anything wrong". Narrowing the level selection to `INFO`
        // would zero them silently, which would make them worse than absent.
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
            assert!(
                !expr.contains("$logLevelList"),
                "{title} is narrowed by the level picker: {expr}"
            );
            assert!(expr.contains("level=~\"WARN"), "{title}: {expr}");
        }
    }

    #[test]
    fn every_log_selector_carries_the_job_anchor() {
        // LogQL rejects a stream selector whose every matcher can match the empty
        // string. Every picker here is a `=~`, so without the job matcher --
        // whose "All" is `.+` -- a panel errors instead of returning a wide
        // result. That failure only shows in the browser, which is why it is
        // asserted here.
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
                    expr.contains("job=~\"$logJobList\""),
                    "{name} has no non-empty-compatible matcher: {expr}"
                );
            }
        }
    }

    #[test]
    fn every_panel_honours_the_search_box() {
        // The search is the control an operator reaches for first. A panel that
        // ignored it would keep showing lines the others had filtered away.
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
                assert!(expr.contains("$logSearch"), "{name} ignores the search box");
            }
        }
    }
}
