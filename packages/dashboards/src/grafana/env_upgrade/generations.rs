// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Generations tab: the two sides of a blue/green rollout, side by side.
//!
//! A rollout stands a new generation of `environmentd` and its replicas up beside
//! the old one, rehydrates it, and promotes it only once it has caught up. Both
//! are live and scraped at the same time, so every panel elsewhere in this repo
//! sums them together — which is precisely the wrong thing while the question is
//! whether one of them is ready yet. Splitting by generation is all this tab does,
//! and it is the reason it exists.
//!
//! # Reading order
//!
//! 1. **Rollout Status** — how many generations are up, and the headline numbers
//!    for each.
//! 2. **Hydration** — the tab's centrepiece. The new generation's hydrating count
//!    descending toward zero *is* the rollout making progress, and the rate of
//!    that descent is the only honest estimate of how much longer it needs.
//! 3. **Freshness** — hydrated is not the same as caught up. This covers the
//!    collections that have a frontier; Hydration counts the ones that do not.
//! 4. **Footprint** — what running two generations at once costs, which is
//!    roughly double for the duration.
//!
//! # The generation is not a label
//!
//! orchestratord records it as a Kubernetes annotation, which nothing exports.
//! Where it reaches a query is the object *name*, so the filter and the
//! `label_replace` that produces the legend both match on names — see
//! `%%{mzGenerationFilter}` and `%%{mzGenerationPattern}` in the render context.
//! Both live there rather than in the queries so they cannot drift apart.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::{BigValueGraphMode, BigValueTextMode};
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// The tab's theme, applied to every shaded panel here.
const SHADE: &str = theme::GENERATIONS.shade;

/// What a panel shows when no generation matched.
fn no_generations() -> NoValue {
    NoValue::Custom("No generations match the current filters".to_string())
}

/// What the hydration panels show when no generation reports at all.
///
/// Not the same as "nothing is hydrating": the query scores every collection with
/// `> bool`, so a generation that has caught up reports a genuine zero and stays
/// on the graph. Reaching this message means no generation has any collection to
/// score — a brand-new one in its first moments, or a torn-down one.
fn no_collections() -> NoValue {
    NoValue::Custom("No collections reporting for the selected generations".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        rollout_status(q),
        versions(q),
        hydration(q),
        freshness(q),
        footprint(q),
    ]
}

fn rollout_status(q: &Queries) -> Row {
    Row::new("Rollout Status").hide_header().grid(
        AutoGrid::new(4)
            .row_height(RowHeight::Short)
            .panel("active-generations", active_generations(q))
            .panel("hydrating-now", hydrating_now(q))
            .panel("max-lag-now", max_lag_now(q))
            .panel("pods-now", pods_now(q)),
    )
}

fn versions(q: &Queries) -> Row {
    // Its own row rather than a fifth cell in Rollout Status: the value is a
    // version *string*, which a stat cannot show two of legibly, and the pair of
    // rows during a rollout is the headline — this is the upgrade you are doing.
    Row::new("Versions").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Short)
            .panel("version-by-generation", version_by_generation(q)),
    )
}

fn hydration(q: &Queries) -> Row {
    Row::new("Hydration").grid(
        AutoGrid::new(2)
            .row_height(RowHeight::Tall)
            .panel("hydrating-by-generation", hydrating_by_generation(q))
            .panel("collections-by-generation", collections_by_generation(q)),
    )
}

fn freshness(q: &Queries) -> Row {
    Row::new("Freshness").grid(AutoGrid::new(1).panel("lag-by-generation", lag_by_generation(q)))
}

fn footprint(q: &Queries) -> Row {
    Row::new("Footprint").grid(
        AutoGrid::new(2)
            .panel("cpu-by-generation", cpu_by_generation(q))
            .panel("memory-by-generation", memory_by_generation(q)),
    )
}

// ------------------------------------------------------------ rollout status

fn active_generations(q: &Queries) -> dashboardv2::PanelKind {
    // Not threshold-coloured. Two generations is what a rollout looks like, and
    // alarming on it would fire on every routine upgrade — the same reasoning
    // that keeps Environments Needing Update neutral on the Reconciliation tab.
    Panel::stat("Active Generations")
        .query(
            q.get("materialize.generations.active")
                .legend("generations"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .min(0.0)
        .decimals(0.0)
        .no_value(no_generations())
        .build(0)
}

fn hydrating_now(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Currently Hydrating")
        .query(
            q.get("materialize.generations.hydrating")
                .legend("gen {{generation}}"),
        )
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .decimals(0.0)
        .no_value(no_collections())
        .build(0)
}

fn max_lag_now(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Max Frontier Lag")
        .query(
            q.get("materialize.generations.lag.max")
                .legend("gen {{generation}}"),
        )
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(no_generations())
        .build(0)
}

fn pods_now(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Pods")
        .query(
            q.get("materialize.generations.pods")
                .legend("gen {{generation}}"),
        )
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .decimals(0.0)
        .no_value(no_generations())
        .build(0)
}

// ----------------------------------------------------------------- versions

fn version_by_generation(q: &Queries) -> dashboardv2::PanelKind {
    // The metric's value says nothing; the labels are the content. Promote them
    // to columns, collapse the per-series frames into one table, drop the value
    // column, and put the newest generation on top — during a rollout that is the
    // side being asked about.
    //
    // "(Select Time Range)" is not decoration: the query covers the whole picker
    // window rather than the current instant, so widening it is how a finished
    // rollout's old generation comes back into view.
    const COLUMNS: &[&str] = &["generation", "mz_version"];
    Panel::table("Version by Generation (Select Time Range)")
        .query(q.get("materialize.generations.version"))
        .transformations(vec![
            transform::labels_to_fields(COLUMNS),
            transform::merge(),
            // Both spellings excluded so the same table works on self-managed and
            // Cloud, where the metric arrives prefixed.
            // The metric's value is meaningless here and Grafana names that
            // field differently depending on the response shape, so every
            // spelling is excluded. An exclusion that matches no field is a
            // no-op, which makes covering all of them free.
            transform::organize(
                &[
                    "Time",
                    "Value",
                    "mz_compute_cluster_status",
                    "v2_mz_compute_cluster_status",
                ],
                COLUMNS,
            ),
            transform::sort_by("generation", true),
        ])
        .no_value(no_generations())
        .build(0)
}

// ---------------------------------------------------------------- hydration

fn hydrating_by_generation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Hydrating Collections by Generation")
        .query(
            q.get("materialize.generations.hydrating")
                .legend("gen {{generation}}"),
        )
        .min(0.0)
        .no_value(no_collections())
        .build(0)
}

fn collections_by_generation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Collections by Generation")
        .query(
            q.get("materialize.generations.collections")
                .legend("gen {{generation}}"),
        )
        .min(0.0)
        .no_value(no_generations())
        .build(0)
}

// ---------------------------------------------------------------- freshness

fn lag_by_generation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Frontier Lag by Generation")
        .query(
            q.get("materialize.generations.lag.max")
                .legend("gen {{generation}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(no_generations())
        .build(0)
}

// ---------------------------------------------------------------- footprint

fn cpu_by_generation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU by Generation")
        .query(
            q.get("materialize.generations.cpu")
                .legend("gen {{generation}}"),
        )
        .unit("cores")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_by_generation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory by Generation")
        .query(
            q.get("materialize.generations.memory")
                .legend("gen {{generation}}"),
        )
        .unit("bytes")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_operator_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_operator_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 10);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_panel_splits_by_generation() {
        // The tab's entire reason for existing. A panel that lost either the
        // filter or the grouping would silently sum the two sides of a
        // blue/green back together and look completely normal doing it.
        let q = &test_operator_queries();
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
                // Either spelling: the filter splices the value into a wider
                // regex, so it uses the `${…:regex}` form.
                assert!(
                    expr.contains("$mzGenerationList")
                        || expr.contains("${mzGenerationList:regex}"),
                    "{name} is not filtered by generation: {expr}"
                );
                assert!(
                    expr.contains("label_replace") && expr.contains("\"generation\""),
                    "{name} does not lift the generation into a label: {expr}"
                );
            }
        }
    }

    #[test]
    fn no_query_leaves_an_unrendered_placeholder() {
        // `substitute_params` leaves an invalid `%%{...}` verbatim rather than
        // failing, so a mistyped parameter reaches the artifact as literal text
        // and the panel errors in the browser. Nothing else catches it.
        let q = &test_operator_queries();
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
                    !expr.contains("%%{"),
                    "{name} has an unrendered placeholder: {expr}"
                );
            }
        }
    }

    #[test]
    fn the_version_table_shows_the_generation_beside_the_version() {
        // A rollout's headline is "gen N (old version) -> gen N+1 (new version)".
        // A table that lost either column would answer half the question.
        let q = &test_operator_queries();
        let panel = version_by_generation(q);
        let organize = panel
            .spec
            .data
            .spec
            .transformations
            .iter()
            .find(|t| t.group == "organize")
            .expect("an organize transform");
        let options = serde_json::to_value(&organize.spec.options).expect("serialize");
        let order = &options["indexByName"];
        assert!(order.get("generation").is_some(), "{options}");
        assert!(order.get("mz_version").is_some(), "{options}");
        // Both metric-name spellings dropped, so the same table works on
        // self-managed and Cloud.
        assert_eq!(options["excludeByName"]["Value"], true);
        // The query must look back over the picker window. A plain instant query
        // would evaluate at "now", where a promoted generation has already been
        // torn down, and a finished rollout would look like it never happened.
        let expr = panel.spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .expect("spec")["expr"]
            .as_str()
            .expect("expr");
        assert!(expr.contains("max_over_time"), "{expr}");
        assert!(expr.contains("$__range"), "{expr}");
        assert_eq!(options["excludeByName"]["mz_compute_cluster_status"], true);
        assert_eq!(
            options["excludeByName"]["v2_mz_compute_cluster_status"],
            true
        );
    }

    #[test]
    fn hydration_counts_with_bool_so_the_line_reaches_zero() {
        // A filtering `>` drops the non-matching series, so the query emits no
        // sample once a generation finishes and the panel keeps showing the last
        // count it saw. `> bool` scores every collection and `sum` adds them, so
        // the series stays present and descends to zero -- which is the whole
        // reading this tab is built around.
        let q = &test_operator_queries();
        let expr = hydrating_by_generation(q).spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .expect("spec")["expr"]
            .as_str()
            .expect("expr")
            .to_string();
        assert!(expr.contains("> bool 1e15"), "{expr}");
        assert!(expr.starts_with("sum by (generation)"), "{expr}");
        assert!(!expr.contains("count by (generation)"), "{expr}");
    }

    #[test]
    fn the_two_hydration_panels_share_one_query() {
        // The summary stat and the timeseries are the same question at two
        // altitudes. Two definitions could disagree, and the stat is the one an
        // operator reads first.
        let q = &test_operator_queries();
        let expr = |panel: dashboardv2::PanelKind| {
            panel.spec.data.spec.queries[0]
                .spec
                .query
                .spec
                .as_ref()
                .expect("spec")["expr"]
                .as_str()
                .expect("expr")
                .to_string()
        };
        assert_eq!(expr(hydrating_now(q)), expr(hydrating_by_generation(q)));
    }
}
