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
use mzmon_lib::grafana::query::{PromQuery, query_group};
use mzmon_lib::grafana::threshold;
use mzmon_lib::query::enrich;

use super::{field_override, selector, theme, transform};

/// The tab's theme.
const SHADE: &str = theme::CONNECTIONS.shade;

pub fn rows() -> Vec<Row> {
    vec![connection_summary(), queries(), applications()]
}

fn connection_summary() -> Row {
    Row::new("Connection Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("connections-active-sessions", active_sessions())
            .panel("connections-active-queries", active_queries())
            .panel("connections-adapter-command-rate", adapter_command_rate()),
    )
}

fn queries() -> Row {
    Row::new("Queries").grid(
        AutoGrid::new(3)
            .panel("queries-distribution", query_distribution())
            .panel("queries-rate-by-statement", query_rate())
            .panel("queries-peek-latency-p50", peek_latency(Quantile::P50))
            .panel("queries-peek-latency-p90", peek_latency(Quantile::P90))
            .panel("queries-peek-latency-p99", peek_latency(Quantile::P99)),
    )
}

fn applications() -> Row {
    // One column: the panel is a wide table whose columns need the room. A
    // three-column grid would render it at a third of the width.
    Row::new("SQL Control Plane Commands")
        .grid(AutoGrid::new(1).panel("adapter-commands-by-app", commands_by_application()))
}

fn active_sessions() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (session_type) (
    mz_active_sessions{{{env}}}
)
"#,
        env = selector::environment()
    );
    Panel::stat("Active Sessions")
        .description(
            "**Currently-open SQL sessions, broken down by session type (`system` vs \
             `user`).** `system` sessions come from Materialize's internal probing (a few are \
             always present); `user` sessions come from client connections. Nominal: a small \
             steady `system` count and a variable `user` count tracking your client activity. \
             Sustained high `user` count is often a leaked-connection signal — sanity-check by \
             seeing whether _Active Queries_ shows commensurate activity. Environment-scoped.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{session_type}}").build(),
        ]))
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_queries() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (session_type) (
    rate(mz_query_total{{{env}}}[$__rate_interval])
)
"#,
        env = selector::environment()
    );
    Panel::stat("Active Queries")
        .description(
            "**Queries per second by session type, rated from the `mz_query_total` counter.** \
             Bursty in normal operation — `user` tracks your client traffic shape, `system` \
             reflects internal health-checks (typically a steady single-digit baseline). Use \
             _Query Distribution_ to see *what kinds* of queries make up the rate, and _Peek \
             Latency_ to confirm the queries are running fast enough. Environment-scoped.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{session_type}}").build(),
        ]))
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .unit("cps")
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn adapter_command_rate() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum(
    rate(mz_adapter_commands{{{env}}}[$__rate_interval])
)
"#,
        env = selector::environment()
    );
    Panel::stat("SQL Control Plane Command Rate")
        .description(
            "**Commands per second across the adapter** — the SQL protocol layer that handles \
             parse, execute, prepare, fetch, etc. Usually higher than the query rate because \
             each query produces several commands. Sudden flat-line on a usually-busy env is \
             unusual (could indicate adapter trouble). Use _Adapter Commands by Application_ \
             below to see which clients dominate, and watch its Errors column for failed \
             commands. Environment-scoped.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("commands").build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .unit("cps")
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn query_distribution() -> dashboardv2::PanelKind {
    // `increase(…[$__range])` rather than a rate: the donut's slices are totals
    // over the selected window, which is what makes them comparable as shares.
    // `> 0` drops statement types with no traffic, which would otherwise crowd the
    // legend with zero-width slices.
    let expr = format!(
        r#"
sum by (statement_type) (
    increase(mz_query_total{{{env}}}[$__range])
) > 0
"#,
        env = selector::environment()
    );
    Panel::piechart("Query Distribution (by statement_type)")
        .description(
            "**Share of queries by statement type over the dashboard's time range** — uses \
             `increase()`, so the slice sizes are total counts over the time selector, not \
             per-second rates. Workload-shape signal. Heavy `set_variable` / `reset_variable` / \
             `fetch` traffic is normal — that's how PostgreSQL clients manage session state. \
             Heavy `insert` / `update` / `delete` on a service you think of as read-mostly is \
             worth investigating. Idle statement types are filtered out (`> 0`). \
             Environment-scoped.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{statement_type}}")
                .build(),
        ]))
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn query_rate() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (statement_type, session_type) (
    rate(mz_query_total{{{env}}}[$__rate_interval])
) > 0
"#,
        env = selector::environment()
    );
    Panel::timeseries("Query Rate (by statement_type / session_type)")
        .description(
            "**Queries per second broken down by statement type AND session type, fully \
             time-resolved.** Pairs with the _Query Distribution_ donut (which shows the \
             time-range total): this panel shows *how those slices move over time*. Watch for \
             sudden spikes in `select / user` — pair with _Peek Latency (p99)_ to see if the \
             system kept up. Idle (statement, session) tuples are filtered out. \
             Environment-scoped.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .legend("{{statement_type}} / {{session_type}}")
                .build(),
        ]))
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
    fn value(self) -> f64 {
        match self {
            Quantile::P50 => 0.5,
            Quantile::P90 => 0.9,
            Quantile::P99 => 0.99,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Quantile::P50 => "Peek Latency (p50)",
            Quantile::P90 => "Peek Latency (p90)",
            Quantile::P99 => "Peek Latency (p99)",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Quantile::P50 => {
                "**Median read-query latency** — the typical time it takes to look up the \
                 current state of an arrangement (the operation behind every `SELECT … FROM \
                 <view>` against an index). p50 is your \"what does a normal query feel like\" \
                 number. Nominal: typically a few milliseconds on a healthy cluster. Sustained \
                 multi-second p50 means the cluster is overwhelmed. One line per cluster — \
                 narrow the cluster selector to focus. Log Y-axis. See also: _Dataflow Elapsed \
                 Rate_ and _Arrangement Maintenance Rate_ on the _Compute Objects_ tab."
            }
            Quantile::P90 => {
                "**90th-percentile read-query latency** — \"how slow do my slowest 10% of \
                 queries feel?\" Catches contention bursts and rarely-hit cold paths that p50 \
                 hides. Nominal: usually a small multiple of p50 (2-5x). If p90 is 10-100x p50, \
                 your latency distribution is bimodal — typically cold-cache effects on \
                 infrequently-queried indexes or contention. Same per-cluster split as p50."
            }
            Quantile::P99 => {
                "**Tail read-query latency (99th percentile)** — the slowest 1% of queries, the \
                 ones users complain about. Nominal: a small multiple of p50 (typically 2-10x), \
                 with occasional spikes during query plan recompilation or hydration. Sustained \
                 p99 in the seconds range — especially when *not* paired with elevated p50/p90 \
                 — points at a single bad query or a tail-latency-sensitive use case worth \
                 investigating directly. Pair with _Query Rate_ above to confirm the latency is \
                 happening on actual traffic, not just idle scrapes."
            }
        }
    }
}

fn peek_latency(quantile: Quantile) -> dashboardv2::PanelKind {
    // Peeks are per-cluster, and the metric carries only `instance_id`. The
    // enrichment joins `mz_cluster_info` so the legend reads a cluster name
    // instead of `u3` -- see `mzmon_lib::query::enrich`.
    let histogram = format!(
        r#"
histogram_quantile({q},
    sum by (le, instance_id) (
        rate(
            mz_compute_peek_duration_seconds_bucket{{{env}, {cluster}}}[$__rate_interval]
        )
    )
)
"#,
        q = quantile.value(),
        env = selector::environment(),
        cluster = selector::cluster()
    );
    let expr = enrich::with_cluster_name(&histogram, "instance_id", &selector::environment());

    Panel::timeseries(quantile.title())
        .description(quantile.description())
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{cluster_name}}").build(),
        ]))
        .unit("s")
        // Log axis: peek latency spans milliseconds to seconds, and a linear axis
        // flattens everything below the worst spike into the baseline.
        .log_scale(10.0)
        .build(0)
}

fn commands_by_application() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (application_name, status) (
    increase(mz_adapter_commands{{{env}}}[$__range])
)
"#,
        env = selector::environment()
    );
    // Pivot `status` into columns so each application is one row with Success and
    // Errors side by side. `groupingToMatrix` names the row column
    // `application_name\status`, hence the rename.
    const ROW_COLUMN: &str = r"application_name\status";
    Panel::table("SQL Control Plane Commands by Application")
        .description(
            "**SQL control plane command totals per `application_name` over the dashboard time \
             range, split into Success and Errors columns.** Rows sorted by Errors (descending) \
             so anything bad floats to the top. The Errors column is threshold-colored — \
             non-zero jumps out visually. Most clients set `application_name` via the \
             PostgreSQL connection string; clients that don't are bucketed as `unrecognized` or \
             `unspecified` (normal). Sustained non-zero Errors on a real application means that \
             app is consistently failing — investigate by correlating with that application's \
             own logs, and inspect recent failures via Materialize's `mz_internal` \
             activity-log views.",
        )
        .data(query_group(vec![PromQuery::new(expr).instant().build()]))
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
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 9);
    }

    #[test]
    fn the_three_percentiles_differ_only_in_the_quantile() {
        // They are one panel three times; if the shapes ever diverge, the shared
        // builder has stopped being shared.
        let exprs: Vec<String> = [Quantile::P50, Quantile::P90, Quantile::P99]
            .into_iter()
            .map(|q| {
                let panel = peek_latency(q);
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
            .zip([0.5, 0.9, 0.99])
            .map(|(e, q)| e.replace(&format!("histogram_quantile({q}"), "histogram_quantile(Q"))
            .collect();
        assert_eq!(normalized[0], normalized[1]);
        assert_eq!(normalized[1], normalized[2]);
    }

    #[test]
    fn the_percentiles_enrich_the_cluster_name() {
        // The metric carries only `instance_id`; without the join the legend reads
        // `u3` rather than the cluster's name.
        let expr = peek_latency(Quantile::P99).spec.data.spec.queries[0]
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
        assert!(expr.contains(&selector::environment()));
    }

    #[test]
    fn only_the_errors_column_is_coloured() {
        let panel = commands_by_application();
        assert_eq!(panel.spec.viz_config.spec.field_config.overrides.len(), 1);
        let override_ = &panel.spec.viz_config.spec.field_config.overrides[0];
        assert_eq!(override_.matcher.options, Some(serde_json::json!("Errors")));
    }

    #[test]
    fn the_pivot_fills_missing_statuses_with_zero() {
        // An application with no errors has no `status="error"` series, and a blank
        // cell would read as "unknown" rather than "none".
        let panel = commands_by_application();
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
        // Sessions and adapter commands belong to environmentd, so narrowing the
        // cluster selector must not change them.
        for panel in [active_sessions(), active_queries(), adapter_command_rate()] {
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
