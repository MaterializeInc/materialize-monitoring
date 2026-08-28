// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Connections / Activity tab: who is talking to Materialize, and how fast.
//!
//! Everything here is environment-scoped rather than cluster-scoped — sessions and
//! adapter commands belong to environmentd, not to a compute cluster. The peek
//! latency panels are the exception: peeks run on a cluster, so those are the one
//! place on this tab where the cluster selector applies and where catalog
//! enrichment earns its keep.

use mzmon_lib::grafana::generated::{dashboardv2, stat};
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::{field_override, theme, transform};
use crate::grafana::queries::Queries;

/// The tab's theme.
const SHADE: &str = theme::CONNECTIONS.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![connection_summary(q), queries(q), applications(q)]
}

fn connection_summary(q: &Queries) -> Row {
    Row::new("Connection Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("connections-active-sessions", active_sessions(q))
            .panel("connections-active-queries", active_queries(q))
            .panel("connections-adapter-command-rate", adapter_command_rate(q)),
    )
}

fn queries(q: &Queries) -> Row {
    Row::new("Queries").grid(
        AutoGrid::new(3)
            .panel("queries-distribution", query_distribution(q))
            .panel("queries-rate-by-statement", query_rate(q))
            .panel("queries-peek-latency-p50", peek_latency(q, Quantile::P50))
            .panel("queries-peek-latency-p90", peek_latency(q, Quantile::P90))
            .panel("queries-peek-latency-p99", peek_latency(q, Quantile::P99)),
    )
}

fn applications(q: &Queries) -> Row {
    // One column: the panel is a wide table whose columns need the room. A
    // three-column grid would render it at a third of the width.
    Row::new("SQL Control Plane Commands")
        .grid(AutoGrid::new(1).panel("adapter-commands-by-app", commands_by_application(q)))
}

fn active_sessions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Sessions")
        .query(
            q.get("materialize.connections.sessions.active")
                .legend("{{session_type}}"),
        )
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_queries(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Queries")
        .query(
            q.get("materialize.connections.queries.rate")
                .legend("{{session_type}}"),
        )
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .unit("cps")
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn adapter_command_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("SQL Control Plane Command Rate")
        .query(
            q.get("materialize.connections.adapter.command_rate")
                .legend("commands"),
        )
        .shade(SHADE)
        .min(0.0)
        .unit("cps")
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn query_distribution(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Query Distribution (by statement_type)")
        .query(
            q.get("materialize.connections.queries.distribution")
                .legend("{{statement_type}}"),
        )
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn query_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Query Rate (by statement_type / session_type)")
        .query(
            q.get("materialize.connections.queries.rate_by_statement")
                .legend("{{statement_type}} / {{session_type}}"),
        )
        .min(0.0)
        .unit("cps")
        .build(0)
}

/// The three peek-latency percentiles the tab shows.
///
/// They are the same panel three times over, differing only in the quantile and
/// the prose — so the shape lives in [`peek_latency`] and this only carries what
/// varies.
#[derive(Debug, Clone, Copy)]
enum Quantile {
    P50,
    P90,
    P99,
}

impl Quantile {
    fn title(self) -> &'static str {
        match self {
            Quantile::P50 => "Peek Latency (p50)",
            Quantile::P90 => "Peek Latency (p90)",
            Quantile::P99 => "Peek Latency (p99)",
        }
    }

    /// The registry query for this quantile. Three separate ids rather than one
    /// parameterized query: each carries its own prose about what that quantile
    /// means, which is the part a reader needs.
    fn query_id(self) -> &'static str {
        match self {
            Quantile::P50 => "materialize.connections.peek_latency.p50",
            Quantile::P90 => "materialize.connections.peek_latency.p90",
            Quantile::P99 => "materialize.connections.peek_latency.p99",
        }
    }
}

fn peek_latency(q: &Queries, quantile: Quantile) -> dashboardv2::PanelKind {
    Panel::timeseries(quantile.title())
        .query(q.get(quantile.query_id()).legend("{{cluster_name}}"))
        .unit("s")
        // Log axis: peek latency spans milliseconds to seconds, and a linear axis
        // flattens everything below the worst spike into the baseline.
        .log_scale(10.0)
        .build(0)
}

fn commands_by_application(q: &Queries) -> dashboardv2::PanelKind {
    // Pivot `status` into columns so each application is one row with Success and
    // Errors side by side. `groupingToMatrix` names the row column
    // `application_name\status`, hence the rename.
    const ROW_COLUMN: &str = r"application_name\status";
    Panel::table("SQL Control Plane Commands by Application")
        .query(q.get("materialize.connections.adapter.commands_by_application"))
        .transformations(vec![
            transform::labels_to_fields(&["application_name", "status"]),
            transform::merge(),
            // `emptyValue: zero` matters: an application with no errors has no
            // `status="error"` series at all, and a blank cell would read as
            // "unknown" rather than "none".
            transform::grouping_to_matrix("application_name", "status", "Value", "zero"),
            transform::organize_renamed(
                &[ROW_COLUMN, "success", "error"],
                &[
                    (ROW_COLUMN, "Application"),
                    ("success", "Success"),
                    ("error", "Errors"),
                ],
            ),
            transform::sort_by("Errors", true),
        ])
        // Only the Errors column is coloured; colouring Success would make a busy
        // healthy application look alarming.
        .overrides(vec![
            field_override::by_name("Errors")
                .thresholds(threshold::errors_default().build())
                .color_background_cells()
                .build(),
        ])
        .unit("short")
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_has_three_rows_and_nine_panels() {
        let q = &crate::grafana::queries::test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 9);
    }

    #[test]
    fn the_three_percentiles_differ_only_in_the_quantile() {
        let q = &crate::grafana::queries::test_queries();
        // They are one panel three times; if the shapes ever diverge, the shared
        // builder has stopped being shared.
        let exprs: Vec<String> = [Quantile::P50, Quantile::P90, Quantile::P99]
            .into_iter()
            .map(|quantile| {
                let panel = peek_latency(q, quantile);
                panel.spec.data.spec.queries[0]
                    .spec
                    .query
                    .spec
                    .as_ref()
                    .and_then(|s| s.get("expr"))
                    .and_then(|e| e.as_str())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        let normalized: Vec<String> = exprs
            .iter()
            // The registry writes the quantiles as `0.50` / `0.90` / `0.99`, so
            // match the literal text rather than a formatted f64 (`0.5` != `0.50`).
            .zip(["0.50", "0.90", "0.99"])
            .map(|(e, q)| e.replace(&format!("histogram_quantile({q}"), "histogram_quantile(Q"))
            .collect();
        assert_eq!(normalized[0], normalized[1]);
        assert_eq!(normalized[1], normalized[2]);
    }

    #[test]
    fn the_percentiles_enrich_the_cluster_name() {
        let q = &crate::grafana::queries::test_queries();
        // The metric carries only `instance_id`; without the join the legend reads
        // `u3` rather than the cluster's name.
        let expr = peek_latency(q, Quantile::P99).spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .and_then(|s| s.get("expr"))
            .and_then(|e| e.as_str())
            .unwrap_or_default()
            .to_string();
        assert!(expr.contains("mz_cluster_info"), "{expr}");
        assert!(expr.contains("group_left(cluster_name)"), "{expr}");
        // And the info metric must be environment-scoped, or ids collide across
        // organizations.
        assert!(expr.contains(&super::super::selector::environment()));
    }

    #[test]
    fn only_the_errors_column_is_coloured() {
        let q = &crate::grafana::queries::test_queries();
        let panel = commands_by_application(q);
        assert_eq!(panel.spec.viz_config.spec.field_config.overrides.len(), 1);
        let override_ = &panel.spec.viz_config.spec.field_config.overrides[0];
        assert_eq!(override_.matcher.options, Some(serde_json::json!("Errors")));
    }

    #[test]
    fn the_pivot_fills_missing_statuses_with_zero() {
        let q = &crate::grafana::queries::test_queries();
        // An application with no errors has no `status="error"` series, and a blank
        // cell would read as "unknown" rather than "none".
        let panel = commands_by_application(q);
        let pivot = panel
            .spec
            .data
            .spec
            .transformations
            .iter()
            .find(|t| t.group == "groupingToMatrix")
            .expect("a pivot");
        let options = serde_json::to_value(&pivot.spec.options).expect("serialize");
        assert_eq!(options["emptyValue"], "zero");
    }

    #[test]
    fn everything_but_the_percentiles_is_environment_scoped_only() {
        let q = &crate::grafana::queries::test_queries();
        // Sessions and adapter commands belong to environmentd, so narrowing the
        // cluster selector must not change them.
        for panel in [
            active_sessions(q),
            active_queries(q),
            adapter_command_rate(q),
        ] {
            let expr = panel.spec.data.spec.queries[0]
                .spec
                .query
                .spec
                .as_ref()
                .and_then(|s| s.get("expr"))
                .and_then(|e| e.as_str())
                .unwrap_or_default();
            assert!(!expr.contains("mzClusterList"), "{expr}");
        }
    }
}
