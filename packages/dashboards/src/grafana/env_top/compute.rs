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
use mzmon_lib::grafana::query::{PromQuery, query_group};
use mzmon_lib::grafana::threshold;
use mzmon_lib::query::enrich;

use super::{selector, theme, transform};

/// The tab's theme.
const SHADE: &str = theme::COMPUTE.shade;

pub fn rows() -> Vec<Row> {
    vec![
        summary(),
        freshness(),
        hydration(),
        dataflows(),
        arrangements(),
    ]
}

fn summary() -> Row {
    Row::new("Compute Objects Summary").hide_header().grid(
        AutoGrid::new(5)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("active-mzd-views", active_materialized_views())
            .panel("active-indexes", active_indexes())
            .panel("active-views", active_views())
            .panel("active-subscribes", active_subscribes())
            .panel("index-types", index_types()),
    )
}

fn freshness() -> Row {
    Row::new("Freshness").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("freshness-lag-by-cluster", lag_by_cluster())
            .panel("freshness-top-collections", top_lagged_collections()),
    )
}

fn hydration() -> Row {
    Row::new("Hydration").grid(
        AutoGrid::new(3)
            .panel(
                "hydration-unhydrated-count",
                super::currently_hydrating(SHADE),
            )
            .panel("hydration-queue-size", hydration_queue_size())
            .panel(
                "hydration-slowest-collections",
                slowest_hydrating_collections(),
            ),
    )
}

fn dataflows() -> Row {
    Row::new("Dataflows").grid(
        AutoGrid::new(3)
            .panel("dataflow-count", dataflow_count(Split::Replica))
            .panel("dataflow-count-by-worker", dataflow_count(Split::Worker))
            .panel("dataflow-elapsed-rate", dataflow_elapsed_rate()),
    )
}

fn arrangements() -> Row {
    Row::new("Arrangements").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("arrangement-rate", arrangement_rate(Split::Replica))
            .panel(
                "arrangement-rate-by-worker",
                arrangement_rate(Split::Worker),
            )
            .panel(
                "arrangement-records-system",
                record_counts(Collections::System),
            )
            .panel("arrangement-records-user", record_counts(Collections::User))
            .panel(
                "arrangement-records-transient",
                record_counts(Collections::Transient),
            ),
    )
}

/// Environment scope, which the catalog-count panels use alone.
///
/// These read environmentd's catalog rather than a replica, so the cluster and
/// replica selectors deliberately do not apply — the object exists whichever
/// cluster you are looking at.
fn env() -> String {
    selector::environment()
}

fn active_materialized_views() -> dashboardv2::PanelKind {
    let expr = format!("\nmax(mz_mzd_views_count{{{env}}})\n", env = env());
    Panel::stat("Active Materialized Views")
        .description(
            "**Number of materialized views actively maintained by Materialize.** Each \
             materialized view is a persistent compute object that incrementally keeps a \
             query's result up to date — so this count is roughly proportional to how much work \
             the cluster is doing. Nominal: stable in steady state; expected to step on `CREATE \
             MATERIALIZED VIEW` / `DROP MATERIALIZED VIEW`. Sustained drift suggests automated \
             DDL activity. Environment-scoped — not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("materialized-views").build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_indexes() -> dashboardv2::PanelKind {
    // `sum by (instance)` then `max`: the count is reported per scrape target, so
    // summing within a target and taking the max across them avoids double
    // counting when two jobs scrape the same environmentd.
    let expr = format!(
        "\nmax(sum by (instance) (mz_indexes_count{{{env}}})) or vector(0)\n",
        env = env()
    );
    Panel::stat("Active Indexes")
        .description(
            "**Number of indexes in the catalog.** An index is an in-memory arrangement that \
             makes `SELECT`s against its underlying relation effectively instant — at the cost \
             of memory. Rapid growth here typically pairs with growing memory usage on the \
             cluster's pods (see _Kubernetes Workloads -> Pod Memory Usage_). For the \
             table/view/materialized-view split see _Index Types_. Environment-scoped — not \
             affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("indexes").build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_views() -> dashboardv2::PanelKind {
    let expr = format!("\nmax(mz_views_count{{{env}}})\n", env = env());
    Panel::stat("Active Views")
        .description(
            "**Views (non-materialized) in the catalog.** Views are query templates evaluated \
             on demand — they don't consume compute or memory until something queries them. \
             Mostly a catalog-shape signal; for read-side activity see _Connections / Activity \
             -> Query Rate_. Environment-scoped — not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("views").build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_subscribes() -> dashboardv2::PanelKind {
    let expr = format!(
        "\nsum by (session_type) (\n    mz_active_subscribes{{{env}}}\n)\n",
        env = env()
    );
    Panel::piechart("Active Subscribes")
        .description(
            "**Live SUBSCRIBE sessions — long-running queries that push updates to a client as \
             the underlying data changes.** `system` subscribes are Materialize's internal \
             probing / health checks (always a few); `user` subscribes come from client \
             connections. A persistently high `user` count is often a leaked-connection signal. \
             Environment-scoped — not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{session_type}}")
                .build(),
        ]))
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

fn index_types() -> dashboardv2::PanelKind {
    let expr = format!(
        "\nsum by (relation_type) (\n    mz_indexes_count{{{env}}}\n)\n",
        env = env()
    );
    Panel::piechart("Index Relationship Types")
        .description(
            "**Indexes by the underlying relation type** (view / table / materialized-view). \
             Most workloads heavily favor indexes on views — that's the canonical 'maintain a \
             query's result' pattern. A high ratio of indexes on tables is unusual and worth \
             understanding (often a misunderstanding of materialization). Environment-scoped — \
             not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{relation_type}}")
                .build(),
        ]))
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

fn lag_by_cluster() -> dashboardv2::PanelKind {
    let base = format!(
        r#"
max by (instance_id) (
    mz_dataflow_wallclock_lag_seconds{{
        {env},
        {cluster},
        instance_id!="",
        quantile="1"
    }} < {ceiling}
)
"#,
        env = env(),
        cluster = selector::cluster(),
        ceiling = selector::FINITE_LAG_CEILING
    );
    let expr = enrich::with_cluster_name(&base, "instance_id", &env());
    Panel::timeseries("Freshness Lag by Cluster")
        .description(
            "**How far behind real time each cluster's most-lagged collection is** — the \
             worst-case freshness across all indexes, materialized views, and sources on the \
             cluster. Nominal: low and flat (sub-second to a few seconds) for a keeping-up \
             cluster. A sustained climb means a collection can't keep pace with its input rate \
             (under-provisioned cluster or an expensive dataflow); a sharp spike that decays \
             back down is normal right after a restart or DDL while dataflows re-hydrate. The \
             catalog cluster (`s2`) sits a couple seconds back as its baseline. For which \
             collection is responsible see _Most-Lagged Collections_; for hydration \
             specifically see the _Hydration_ row.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{cluster_name}}").build(),
        ]))
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

fn top_lagged_collections() -> dashboardv2::PanelKind {
    let base = format!(
        r#"
max by (instance_id, collection_id) (
    mz_dataflow_wallclock_lag_seconds{{
        {env},
        {cluster},
        instance_id!="",
        {replica},
        quantile="1"
    }} < {ceiling}
)
"#,
        env = env(),
        cluster = selector::cluster(),
        replica = selector::replica(),
        ceiling = selector::FINITE_LAG_CEILING
    );
    // Both joins: the collection's own name, then the cluster it runs on. Object
    // first so `cluster_name` cannot collide with the object's `name`.
    let named = enrich::with_object_name(&base, "collection_id", None, &env());
    let enriched = enrich::with_cluster_name(&named, "instance_id", &env());
    let expr = format!("topk(15,\n{enriched}\n)");

    Panel::barchart("Most-Lagged Collections")
        .description(
            "**The 15 collections whose output frontier is furthest behind real time.** This is \
             the per-collection breakdown behind _Frontier Lag by Cluster_, labeled by object \
             name (resolved via `mz_object_info`). Collections with no frontier yet (idle or \
             still hydrating) report a sentinel and are filtered out here — check hydration \
             state directly with `SELECT * FROM mz_internal.mz_hydration_statuses WHERE NOT \
             hydrated`. `s2` (catalog) collections commonly appear with a small baseline lag; \
             that's expected.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{cluster_name}} / {{name}}")
                .build(),
        ]))
        .unit("s")
        .custom(bar_custom(false))
        .no_value(NoValue::FilterMismatch)
        .shade(SHADE)
        .build(0)
}

fn hydration_queue_size() -> dashboardv2::PanelKind {
    let base = format!(
        r#"
sum by (instance_id, replica_id) (
    mz_compute_controller_hydration_queue_size{{
        {env},
        {cluster},
        {replica}
    }}
) > 0
"#,
        env = env(),
        cluster = selector::cluster(),
        replica = selector::replica()
    );
    let expr = enrich::with_cluster_name(&base, "instance_id", &env());
    Panel::timeseries("Hydration Queue Size")
        .description(
            "**Collections waiting in the compute controller's hydration queue, per replica.** \
             environmentd schedules hydration work in batches; backlog here means it's queueing \
             faster than replicas can complete it — typical during mass cluster restarts, \
             atypical otherwise. Nominal: 0. Sustained non-zero means the replica is undersized \
             or one slow-hydrating collection is blocking the line.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .legend("{{cluster_name}} / r{{replica_id}}")
                .build(),
        ]))
        .unit("short")
        .min(0.0)
        // Nominal is zero, so "empty" is the healthy reading, not a filter miss.
        .no_value(NoValue::Custom("Hydration Queue is empty".to_string()))
        .build(0)
}

fn slowest_hydrating_collections() -> dashboardv2::PanelKind {
    let base = format!(
        r#"
mz_compute_hydration_time_seconds{{
    {env},
    {cluster},
    {replica},
    hydrated="1"
}}
"#,
        env = env(),
        cluster = selector::cluster(),
        replica = selector::replica()
    );
    let named = enrich::with_object_name(&base, "collection_id", None, &env());
    let enriched = enrich::with_cluster_name(&named, "instance_id", &env());
    let expr = format!("topk(15,\n{enriched}\n)");

    Panel::barchart("Slowest Hydrating Collections")
        .description(
            "**The 15 collections that took the longest to finish hydrating** in the current \
             time range. Hydration time scales roughly with the size of the maintained state, \
             so large materialized views and indexes naturally lead the list. **Note \
             (self-managed): per-collection hydration time is not exposed as a metric here** \
             (the cloud-only `v2_mz_compute_hydration_time_seconds` has no equivalent), so this \
             is blank. Get it from SQL: `SELECT object_id, time_ns/1e9 AS seconds FROM \
             mz_internal.mz_compute_hydration_times ORDER BY time_ns DESC`. The live \
             metric-side proxy for what's behind is _Freshness -> Most-Lagged Collections_.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{cluster_name}} / r{{replica_id}} / {{name}}")
                .build(),
        ]))
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
    /// The grouping labels, and the legend that names them.
    fn grouping(self) -> String {
        let base = format!(
            "\n    {cluster},\n    {replica}",
            cluster = selector::HISTORY_CLUSTER_LABEL,
            replica = selector::HISTORY_REPLICA_LABEL
        );
        match self {
            Split::Replica => base,
            Split::Worker => format!("{base},\n    worker_id"),
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

/// The replica-history scope, in the label spelling those metrics use.
fn history_scope() -> String {
    format!(
        "{env}, {cluster}, {replica}",
        env = env(),
        cluster = selector::history_cluster(),
        replica = selector::history_replica()
    )
}

fn dataflow_count(split: Split) -> dashboardv2::PanelKind {
    let base = format!(
        r#"
max by ({grouping}
) (
    mz_compute_replica_history_dataflow_count{{{scope}}}
)
"#,
        grouping = split.grouping(),
        scope = history_scope()
    );
    let expr = enrich::with_cluster_name(&base, selector::HISTORY_CLUSTER_LABEL, &env());
    let (title, description) = match split {
        Split::Replica => (
            "Dataflow Count",
            "**Number of active dataflows running on each replica.** Every index, materialized \
             view, and live `SUBSCRIBE` manifests as one or more dataflows on a replica — so \
             this count rises with DDL and SUBSCRIBE activity. Nominal: stable for steady \
             workloads. A sharp drop without correlating DDL usually means a replica restart \
             (cross-check _Kubernetes Workloads_); a sharp rise typically means bulk object \
             creation.",
        ),
        Split::Worker => (
            "Dataflow Count (per worker)",
            "**Per-worker view of the dataflow count.** Workers within the same replica run in \
             lockstep and should always see the same dataflows — so the worker series for a \
             given replica should overlap exactly. Visible divergence between worker series \
             within a single (cluster, replica) tuple is a bug-class signal and worth filing.",
        ),
    };
    Panel::timeseries(title)
        .description(description)
        .data(query_group(vec![
            PromQuery::new(expr).legend(split.legend()).build(),
        ]))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn dataflow_elapsed_rate() -> dashboardv2::PanelKind {
    // `max without (job)` before the sum: a replica scraped by two jobs would
    // otherwise contribute twice.
    let base = format!(
        r#"
sum by (instance_id) (
    max without (job) (
        rate(
            mz_dataflow_elapsed_seconds_total{{
                {env},
                {cluster},
                {replica}
            }}[$__rate_interval]
        )
    )
)
"#,
        env = env(),
        cluster = selector::cluster(),
        replica = selector::replica()
    );
    let expr = enrich::with_cluster_name(&base, "instance_id", &env());
    Panel::timeseries("Dataflow Elapsed Rate")
        .description(
            "**CPU-cores busy inside dataflows, per cluster.** Covers all dataflow work — \
             arrangement maintenance, evaluation, and hydration. Capped by cluster size: a \
             `400cc` cluster can't exceed 400 cores. Nominal: well below cluster size; \
             sustained near-max means the cluster is CPU-bound and a candidate for upsizing. \
             The catalog cluster (`s2`) is typically the busiest by far in any environment. Log \
             Y-axis so idle and busy clusters share the chart. For maintenance-only breakdown \
             see the _Arrangements_ row below.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{cluster_name}}").build(),
        ]))
        .unit("none")
        .log_scale(10.0)
        .build(0)
}

fn arrangement_rate(split: Split) -> dashboardv2::PanelKind {
    let base = format!(
        r#"
sum by ({grouping}
) (
    max without (job) (
        rate(
            mz_arrangement_maintenance_seconds_total{{{scope}}}[$__rate_interval]
        )
    )
)
"#,
        grouping = split.grouping(),
        scope = history_scope()
    );
    let expr = enrich::with_cluster_name(&base, selector::HISTORY_CLUSTER_LABEL, &env());
    let (title, description, unit) = match split {
        Split::Replica => (
            "Arrangement Maintenance Rate",
            "**CPU-cores spent maintaining arrangements** — the in-memory indexed snapshots \
             that back every index and materialized view. Summed across workers in each \
             replica, so an N-worker replica can hit N. Nominal: well below worker count. \
             Sustained near-max indicates the replica is bottlenecked on maintenance work — \
             usually high upstream change rate or many heavy indexes. The catalog cluster's \
             normal baseline is higher than typical user clusters.",
            "none",
        ),
        Split::Worker => (
            "Arrangement Maintenance Rate (per worker)",
            "**Same metric as the aggregate, but split per worker** — each worker tops out at \
             1.0. Watch for skew: if one worker series sits near 1.0 while siblings sit near 0, \
             the cluster is bottlenecked on that single worker (a 'hot key' or hot \
             arrangement). Skew is the most common reason an 8-worker cluster behaves like a \
             1-worker one. If you see it, the next step is usually `EXPLAIN PHYSICAL PLAN` on \
             the offending object to find the heavy operator.",
            // Per worker the value is a fraction of one core, so it reads as a
            // percentage rather than a core count.
            "percentunit",
        ),
    };
    Panel::timeseries(title)
        .description(description)
        .data(query_group(vec![
            PromQuery::new(expr).legend(split.legend()).build(),
        ]))
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
    fn pattern(self) -> &'static str {
        match self {
            Collections::System => selector::SYSTEM_COLLECTION_PATTERN,
            Collections::User => selector::USER_COLLECTION_PATTERN,
            Collections::Transient => selector::TRANSIENT_COLLECTION_PATTERN,
        }
    }

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

    fn description(self) -> &'static str {
        match self {
            Collections::System => {
                "**Row counts of arrangements maintained for Materialize's internal system \
                 collections** (`collection_id` starts with `s`). These back catalog tables, \
                 internal probes, and other infrastructure — they're not user data and \
                 shouldn't grow with workload. Columns are Min / Max / Last over the selected \
                 time range, sorted by Last (largest current arrangements first). Useful for \
                 spotting unexpected growth in system collections, which can indicate a \
                 Materialize bug."
            }
            Collections::User => {
                "**Row counts of arrangements maintained for user-created compute objects** \
                 (`collection_id` starts with `u`). This is the row count of every index and \
                 materialized view on the selected clusters — the primary driver of memory \
                 consumption. Sudden growth on a specific collection tracks the size of its \
                 underlying data. Columns are Min / Max / Last over the time range; sorted by \
                 Last desc so the largest current arrangements are at the top. Rows are labeled \
                 by object name (resolved via `mz_object_info`)."
            }
            Collections::Transient => {
                "**Row counts of arrangements with `collection_id` starting with `t` \
                 (transient) or labeled `none`.** Transient arrangements are short-lived \
                 intermediates created during query optimization and dataflow execution; the \
                 `none` sentinel is for arrangements whose collection is unidentified. Both are \
                 usually small and ephemeral. Sustained non-trivial entries here are worth \
                 investigating — they may indicate stuck or leaked dataflows."
            }
        }
    }
}

fn record_counts(collections: Collections) -> dashboardv2::PanelKind {
    let base = format!(
        r#"
max by (collection_id) (
    mz_arrangement_record_count{{
        {env},
        {cluster},
        {replica},
        collection_id=~"{pattern}"
    }}
)
"#,
        env = env(),
        cluster = selector::cluster(),
        replica = selector::replica(),
        pattern = collections.pattern()
    );
    let expr = if collections.named() {
        enrich::with_object_name(&base, "collection_id", None, &env())
    } else {
        base
    };

    Panel::table(collections.title())
        .description(collections.description())
        .data(query_group(vec![
            PromQuery::new(expr).legend(collections.legend()).build(),
        ]))
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
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 18);
    }

    #[test]
    fn the_catalog_counts_ignore_the_cluster_selectors() {
        // These read environmentd's catalog: an index exists whichever cluster you
        // are looking at, so narrowing the selector must not change the count.
        for panel in [
            active_materialized_views(),
            active_indexes(),
            active_views(),
            active_subscribes(),
            index_types(),
        ] {
            let expr = expr_of(&panel);
            assert!(!expr.contains("mzClusterList"), "{expr}");
            assert!(!expr.contains("mzReplicaList"), "{expr}");
        }
    }

    #[test]
    fn each_enriched_panel_joins_on_the_label_its_metric_actually_uses() {
        // Three spellings of a cluster id; joining on the wrong one silently
        // produces an empty join and a legend full of raw ids.
        for (panel, label) in [
            (lag_by_cluster(), "instance_id"),
            (dataflow_elapsed_rate(), "instance_id"),
            (hydration_queue_size(), "instance_id"),
            (
                dataflow_count(Split::Replica),
                selector::HISTORY_CLUSTER_LABEL,
            ),
            (
                arrangement_rate(Split::Replica),
                selector::HISTORY_CLUSTER_LABEL,
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
        // The two variants must stay the same query, or the per-worker view stops
        // being comparable with the aggregate.
        for (replica, worker) in [
            (
                dataflow_count(Split::Replica),
                dataflow_count(Split::Worker),
            ),
            (
                arrangement_rate(Split::Replica),
                arrangement_rate(Split::Worker),
            ),
        ] {
            let a = expr_of(&replica).replace(",\n    worker_id", "");
            let b = expr_of(&worker).replace(",\n    worker_id", "");
            assert_eq!(a, b, "the worker split changed more than the grouping");
        }
    }

    #[test]
    fn the_lag_panels_exclude_hydrating_collections() {
        // A collection without a frontier reports a far-future sentinel; leaving it
        // in makes every lag panel read as hours behind.
        for panel in [lag_by_cluster(), top_lagged_collections()] {
            assert!(
                expr_of(&panel).contains(&format!("< {}", selector::FINITE_LAG_CEILING)),
                "{}",
                expr_of(&panel)
            );
        }
        // And the hydration count is the same sentinel read the other way.
        assert!(
            expr_of(&super::super::currently_hydrating(SHADE))
                .contains(&format!("> {}", selector::HYDRATING_SENTINEL))
        );
    }

    #[test]
    fn transient_collections_are_not_name_enriched() {
        // They have no catalog entry to resolve, and the column header says so.
        let transient = record_counts(Collections::Transient);
        assert!(!expr_of(&transient).contains("mz_object_info"));
        assert_eq!(Collections::Transient.row_label(), "Collection ID");

        for named in [Collections::System, Collections::User] {
            assert!(expr_of(&record_counts(named)).contains("mz_object_info"));
            assert_eq!(named.row_label(), "Collection");
        }
    }

    #[test]
    fn the_record_tables_reduce_to_one_row_per_collection() {
        let panel = record_counts(Collections::User);
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
        // Both pull a name; running the cluster join first would have it land in
        // `name` and be overwritten by the object's.
        let expr = expr_of(&top_lagged_collections());
        let object = expr.find("group_left(name)").expect("an object join");
        let cluster = expr
            .find("group_left(cluster_name)")
            .expect("a cluster join");
        assert!(object < cluster, "the object join must be innermost");
    }
}
