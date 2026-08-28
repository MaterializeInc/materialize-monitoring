// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Compute Objects tab: what the clusters are actually doing.
//!
//! The densest tab, and the one where the label-family problem bites hardest. A
//! cluster id arrives under three different label names depending on where the
//! metric came from:
//!
//! * `instance_id` — most compute metrics, scraped from environmentd
//! * `compute_cluster_id` — the SQL-derived status metrics
//! * `cluster_environmentd_materialize_cloud_cluster_id` — replica-history
//!   metrics, scraped from the replica, which prefixes environmentd's own labels
//!
//! [`super::selector`] has one function per spelling, and the enrichment join has
//! to be told which one a given metric uses.
//!
//! Two sentinels also recur. A collection with no output frontier yet reports a
//! lag far in the future rather than nothing, so `> 1e15` counts what is
//! hydrating and `< 1e9` keeps hydration out of the lag panels.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::{selector, theme};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// The tab's theme.
const SHADE: &str = theme::COMPUTE.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        summary(q),
        freshness(q),
        hydration(q),
        dataflows(q),
        arrangements(q),
    ]
}

fn summary(q: &Queries) -> Row {
    Row::new("Compute Objects Summary").hide_header().grid(
        AutoGrid::new(5)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("active-mzd-views", active_materialized_views(q))
            .panel("active-indexes", active_indexes(q))
            .panel("active-views", active_views(q))
            .panel("active-subscribes", active_subscribes(q))
            .panel("index-types", index_types(q)),
    )
}

fn freshness(q: &Queries) -> Row {
    Row::new("Freshness").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("freshness-lag-by-cluster", lag_by_cluster(q))
            .panel("freshness-top-collections", top_lagged_collections(q)),
    )
}

fn hydration(q: &Queries) -> Row {
    Row::new("Hydration").grid(
        AutoGrid::new(3)
            .panel(
                "hydration-unhydrated-count",
                super::currently_hydrating(q, SHADE),
            )
            .panel("hydration-queue-size", hydration_queue_size(q))
            .panel(
                "hydration-slowest-collections",
                slowest_hydrating_collections(q),
            ),
    )
}

fn dataflows(q: &Queries) -> Row {
    Row::new("Dataflows").grid(
        AutoGrid::new(3)
            .panel("dataflow-count", dataflow_count(q, Split::Replica))
            .panel("dataflow-count-by-worker", dataflow_count(q, Split::Worker))
            .panel("dataflow-elapsed-rate", dataflow_elapsed_rate(q)),
    )
}

fn arrangements(q: &Queries) -> Row {
    Row::new("Arrangements").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("arrangement-rate", arrangement_rate(q, Split::Replica))
            .panel(
                "arrangement-rate-by-worker",
                arrangement_rate(q, Split::Worker),
            )
            .panel(
                "arrangement-records-system",
                record_counts(q, Collections::System),
            )
            .panel(
                "arrangement-records-user",
                record_counts(q, Collections::User),
            )
            .panel(
                "arrangement-records-transient",
                record_counts(q, Collections::Transient),
            ),
    )
}

fn active_materialized_views(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Materialized Views")
        .query(
            q.get("materialize.compute.materialized_views.count")
                .legend("materialized-views"),
        )
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_indexes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Indexes")
        .query(q.get("materialize.compute.indexes.count").legend("indexes"))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_views(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Views")
        .query(q.get("materialize.compute.views.count").legend("views"))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_subscribes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Active Subscribes")
        .query(
            q.get("materialize.compute.subscribes.active")
                .legend("{{session_type}}"),
        )
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

fn index_types(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Index Relationship Types")
        .query(
            q.get("materialize.compute.indexes.by_type")
                .legend("{{relation_type}}"),
        )
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

fn lag_by_cluster(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Freshness Lag by Cluster")
        .query(
            q.get("materialize.compute.freshness.lag_by_cluster")
                .legend("{{cluster_name}}"),
        )
        .unit("s")
        .min(0.0)
        .log_scale(10.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

/// The bar-chart `custom` block both lag/hydration bar charts share.
fn bar_custom(threshold_area: bool) -> serde_json::Value {
    let mut custom = serde_json::json!({
        "fillOpacity": 80,
        "gradientMode": "none",
        "lineWidth": 1,
        "scaleDistribution": { "type": "log", "log": 10 },
    });
    if threshold_area {
        // Shades the threshold bands behind the bars, which is how the hydration
        // chart shows "slow" without needing the reader to check the axis.
        custom["thresholdsStyle"] = serde_json::json!({ "mode": "area" });
    }
    custom
}

fn top_lagged_collections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::barchart("Most-Lagged Collections")
        .query(
            q.get("materialize.compute.freshness.top_collections")
                .legend("{{cluster_name}} / {{name}}"),
        )
        .unit("s")
        .custom(bar_custom(false))
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

fn hydration_queue_size(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Hydration Queue Size")
        .query(
            q.get("materialize.compute.hydration.queue_size")
                .legend("{{cluster_name}} / r{{replica_id}}"),
        )
        .unit("short")
        .min(0.0)
        // Nominal is zero, so "empty" is the healthy reading, not a filter miss.
        .no_value(NoValue::Custom("Hydration Queue is empty".to_string()))
        .build(0)
}

fn slowest_hydrating_collections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::barchart("Slowest Hydrating Collections")
        .query(
            q.get("materialize.compute.hydration.slowest_collections")
                .legend("{{cluster_name}} / r{{replica_id}} / {{name}}"),
        )
        .unit("s")
        .custom(bar_custom(true))
        // Six hours, high-is-bad: a collection still hydrating after that long is
        // the alarming case.
        .thresholds(threshold::stability(6.0 * 3600.0, true).build())
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

/// Whether a replica-history panel splits per replica or per worker.
///
/// The two are the same query with one more grouping label, and the per-worker
/// view exists to make *skew* visible — one worker pinned while its siblings idle
/// is the most common reason a wide cluster behaves like a narrow one.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Split {
    Replica,
    Worker,
}

impl Split {
    /// The registry query for the dataflow-count panel at this split.
    fn dataflow_query_id(self) -> &'static str {
        match self {
            Split::Replica => "materialize.compute.dataflows.count",
            Split::Worker => "materialize.compute.dataflows.count_by_worker",
        }
    }

    /// The registry query for the arrangement-maintenance panel at this split.
    fn arrangement_query_id(self) -> &'static str {
        match self {
            Split::Replica => "materialize.compute.arrangements.maintenance_rate",
            Split::Worker => "materialize.compute.arrangements.maintenance_rate_by_worker",
        }
    }

    fn legend(self) -> String {
        let base = format!(
            "{{{{cluster_name}}}} / {{{{{replica}}}}}",
            replica = selector::HISTORY_REPLICA_LABEL
        );
        match self {
            Split::Replica => base,
            Split::Worker => format!("{base} / w{{{{worker_id}}}}"),
        }
    }
}

fn dataflow_count(q: &Queries, split: Split) -> dashboardv2::PanelKind {
    let title = match split {
        Split::Replica => "Dataflow Count",
        Split::Worker => "Dataflow Count (per worker)",
    };
    Panel::timeseries(title)
        .query(q.get(split.dataflow_query_id()).legend(&split.legend()))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn dataflow_elapsed_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Dataflow Elapsed Rate")
        .query(
            q.get("materialize.compute.dataflows.elapsed_rate")
                .legend("{{cluster_name}}"),
        )
        .unit("none")
        .log_scale(10.0)
        .build(0)
}

fn arrangement_rate(q: &Queries, split: Split) -> dashboardv2::PanelKind {
    let (title, unit) = match split {
        Split::Replica => ("Arrangement Maintenance Rate", "none"),
        Split::Worker => ("Arrangement Maintenance Rate (per worker)", "percentunit"),
    };
    Panel::timeseries(title)
        .query(q.get(split.arrangement_query_id()).legend(&split.legend()))
        .unit(unit)
        .build(0)
}

/// Which slice of the catalog an arrangement-record table covers.
///
/// One panel three times, split by collection-id prefix. Transient collections do
/// *not* get name enrichment: they have no catalog entry to resolve, so the table
/// shows ids and says so in its column header.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Collections {
    System,
    User,
    Transient,
}

impl Collections {
    fn named(self) -> bool {
        self != Collections::Transient
    }

    fn title(self) -> &'static str {
        match self {
            Collections::System => "System Collections — Record Counts",
            Collections::User => "User Collections — Record Counts",
            Collections::Transient => "Transient / Uncategorized — Record Counts",
        }
    }

    /// The first column's header, which is what tells the reader whether they are
    /// looking at names or raw ids.
    /// The registry query for this collection class.
    fn query_id(self) -> &'static str {
        match self {
            Collections::System => "materialize.compute.arrangements.records.system",
            Collections::User => "materialize.compute.arrangements.records.user",
            Collections::Transient => "materialize.compute.arrangements.records.transient",
        }
    }

    /// The series label, which is the field `reduce` then turns into the row name.
    ///
    /// Named collections resolve to a `name` via the enrichment join; transient
    /// ones have no catalog entry, so the id is all there is to show.
    fn legend(self) -> &'static str {
        if self.named() {
            "{{name}}"
        } else {
            "{{collection_id}}"
        }
    }

    fn row_label(self) -> &'static str {
        if self.named() {
            "Collection"
        } else {
            "Collection ID"
        }
    }
}

fn record_counts(q: &Queries, collections: Collections) -> dashboardv2::PanelKind {
    Panel::table(collections.title())
        .query(q.get(collections.query_id()).legend(collections.legend()))
        .transformations(vec![
            // One row per collection, with Min / Max / Last as columns.
            transform::reduce(&["min", "max", "lastNotNull"]),
            transform::organize_renamed(&[], &[("Field", collections.row_label())]),
            // `Last *` is what `reduce` names the lastNotNull column.
            transform::sort_by("Last *", true),
        ])
        .unit("short")
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr_of(panel: &dashboardv2::PanelKind) -> String {
        panel.spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .and_then(|s| s.get("expr"))
            .and_then(|e| e.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn the_tab_has_five_rows_and_eighteen_panels() {
        let q = &crate::grafana::queries::test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 18);
    }

    #[test]
    fn the_catalog_counts_ignore_the_cluster_selectors() {
        let q = &crate::grafana::queries::test_queries();
        // These read environmentd's catalog: an index exists whichever cluster you
        // are looking at, so narrowing the selector must not change the count.
        for panel in [
            active_materialized_views(q),
            active_indexes(q),
            active_views(q),
            active_subscribes(q),
            index_types(q),
        ] {
            let expr = expr_of(&panel);
            assert!(!expr.contains("mzClusterList"), "{expr}");
            assert!(!expr.contains("mzReplicaList"), "{expr}");
        }
    }

    #[test]
    fn each_enriched_panel_joins_on_the_label_its_metric_actually_uses() {
        let q = &crate::grafana::queries::test_queries();
        // Three spellings of a cluster id; joining on the wrong one silently
        // produces an empty join and a legend full of raw ids.
        for (panel, label) in [
            (lag_by_cluster(q), "instance_id"),
            (dataflow_elapsed_rate(q), "instance_id"),
            (hydration_queue_size(q), "instance_id"),
            (
                dataflow_count(q, Split::Replica),
                super::super::selector::HISTORY_CLUSTER_LABEL,
            ),
            (
                arrangement_rate(q, Split::Replica),
                super::super::selector::HISTORY_CLUSTER_LABEL,
            ),
        ] {
            let expr = expr_of(&panel);
            assert!(
                expr.contains(&format!("* on ({label}) group_left(cluster_name)")),
                "expected a join on {label} in:\n{expr}"
            );
        }
    }

    #[test]
    fn the_worker_split_only_adds_a_grouping_label() {
        let q = &crate::grafana::queries::test_queries();
        // The split is two registry queries now, not one template with a knob, so
        // this is the check that they have not drifted into different shapes: the
        // per-worker form must be the aggregate plus `worker_id` in the grouping.
        for (replica, worker) in [
            (
                dataflow_count(q, Split::Replica),
                dataflow_count(q, Split::Worker),
            ),
            (
                arrangement_rate(q, Split::Replica),
                arrangement_rate(q, Split::Worker),
            ),
        ] {
            let strip = |e: &str| e.replace(", worker_id", "").replace(",worker_id", "");
            let a = strip(&expr_of(&replica));
            let b = strip(&expr_of(&worker));
            assert!(
                expr_of(&worker).contains("worker_id"),
                "the per-worker query does not group by worker_id"
            );
            assert_eq!(a, b, "the worker split changed more than the grouping");
        }
    }

    #[test]
    fn the_lag_panels_exclude_hydrating_collections() {
        let q = &crate::grafana::queries::test_queries();
        // A collection without a frontier reports a far-future sentinel; leaving it
        // in makes every lag panel read as hours behind.
        for panel in [lag_by_cluster(q), top_lagged_collections(q)] {
            assert!(
                expr_of(&panel)
                    .contains(&format!("< {}", super::super::selector::FINITE_LAG_CEILING)),
                "{}",
                expr_of(&panel)
            );
        }
        // And the hydration count is the same sentinel read the other way.
        assert!(
            expr_of(&super::super::currently_hydrating(q, SHADE))
                .contains(&format!("> {}", super::super::selector::HYDRATING_SENTINEL))
        );
    }

    #[test]
    fn transient_collections_are_not_name_enriched() {
        let q = &crate::grafana::queries::test_queries();
        // They have no catalog entry to resolve, and the column header says so.
        let transient = record_counts(q, Collections::Transient);
        assert!(!expr_of(&transient).contains("mz_object_info"));
        assert_eq!(Collections::Transient.row_label(), "Collection ID");

        for named in [Collections::System, Collections::User] {
            assert!(expr_of(&record_counts(q, named)).contains("mz_object_info"));
            assert_eq!(named.row_label(), "Collection");
        }
    }

    #[test]
    fn the_record_tables_reduce_to_one_row_per_collection() {
        let q = &crate::grafana::queries::test_queries();
        let panel = record_counts(q, Collections::User);
        let reduce = panel
            .spec
            .data
            .spec
            .transformations
            .iter()
            .find(|t| t.group == "reduce")
            .expect("a reduce");
        let options = serde_json::to_value(&reduce.spec.options).expect("serialize");
        assert_eq!(options["mode"], "seriesToRows");
    }

    #[test]
    fn the_object_join_comes_before_the_cluster_join() {
        let q = &crate::grafana::queries::test_queries();
        // Both pull a name; running the cluster join first would have it land in
        // `name` and be overwritten by the object's.
        let expr = expr_of(&top_lagged_collections(q));
        let object = expr.find("group_left(name)").expect("an object join");
        let cluster = expr
            .find("group_left(cluster_name)")
            .expect("a cluster join");
        assert!(object < cluster, "the object join must be innermost");
    }
}
