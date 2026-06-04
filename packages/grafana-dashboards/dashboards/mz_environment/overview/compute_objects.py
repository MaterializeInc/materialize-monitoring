"""Compute Objects tab on Overview Dashboard.

Compute objects include Indexes, Materialized Views, Subscriptions.
"""

from __future__ import annotations

import textwrap

from grafana_foundation_sdk.builders import (
    barchart as barchart_builder,
)
from grafana_foundation_sdk.builders import (
    common as common_builder,
)
from grafana_foundation_sdk.builders import (
    piechart as piechart_builder,
)
from grafana_foundation_sdk.builders import table, timeseries
from grafana_foundation_sdk.models import common, piechart
from py_mzmon_lib import transform as transform_builders
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import promql_query, query_group

from dashboards import palette, threshold, visualization

NO_FILTER_MATCH = "No matches for the current filters"
# Some metrics in this tab are pre-calculated by the promsql-exporter at the
# environment level and don't carry cluster_id / instance_id labels — when
# the user filters to a specific cluster the value is unchanged. Surface this
# in panel descriptions so the lack of reactivity isn't misleading.
ENV_SCOPED_NOTE = "Environment-scoped — not affected by the cluster/replica filters."

COMPUTE_THEME = palette.THEME_PALETTE[3]

# Long-form cluster/replica label names used by mz_arrangement_* and
# mz_compute_replica_history_* (these metrics carry the environmentd-side
# identifiers under these specific names rather than the shorter
# `instance_id` / `replica_id` used on most other mz_ metric families).
#
# NOTE (self-managed): only the *id* variants exist here; the cloud-only
# `*_cluster_name` / `*_replica_name` companions are absent, so legends and
# group-by use the ids directly.
ARRANGEMENT_LABEL_CLUSTER_ID = "cluster_environmentd_materialize_cloud_cluster_id"
ARRANGEMENT_LABEL_REPLICA_ID = "cluster_environmentd_materialize_cloud_replica_id"

# A short PromQL fragment that filters by the current $environmentFilter,
# $mzClusterList, and $mzReplicaList using the long-form label names.
_ARRANGEMENT_FILTER = (
    f"$environmentFilter, "
    f'{ARRANGEMENT_LABEL_CLUSTER_ID}=~"$mzClusterList", '
    f'{ARRANGEMENT_LABEL_REPLICA_ID}=~"$mzReplicaList"'
)


def add_currently_hydrating_panel(
    dashboard: MzDashboard,
    panel_id: str = "hydration-unhydrated-count",
    *,
    shade: str = COMPUTE_THEME,
) -> str:
    """Add a 'Currently Hydrating' sparkline-stat panel to `dashboard`.

    Module-level so both the Compute Objects tab and the Summary tab's
    Environment Health row can register the same panel under different
    panel_ids without duplicating the query/viz logic.

    `v2_mz_compute_hydration_time_seconds{hydrated="0"}` is a marker series:
    its value stays at 0, but the series only exists while a collection has
    not yet finished hydrating. Counting the series gives a real-time
    "is anything currently hydrating?" signal. The `or vector(0)` keeps the
    panel showing 0 (instead of "no data") when nothing is unhydrated.

    TODO(self-managed): `v2_mz_compute_hydration_time_seconds` is a cloud-only
    promsql-exporter metric with no self-managed equivalent, so this reads a
    constant 0 here. The `or vector(0)` keeps the panel honest (shows 0 rather
    than erroring). Revisit once a self-managed hydration-state metric exists
    (`mz_compute_controller_hydration_queue_size` is the nearest live signal —
    see _Hydration Queue Size_).
    """
    query = query_group(
        promql_query(
            textwrap.dedent(
                """
                count(
                    v2_mz_compute_hydration_time_seconds{
                        $environmentFilter,
                        instance_id=~"$mzClusterList",
                        replica_id=~"$mzReplicaList",
                        hydrated="0"
                    }
                ) or vector(0)
                """
            )
        ).legend_format("unhydrated"),
    )

    dashboard.add_panel(
        panel_id,
        dashboardv2_builders.Panel()
        .title("Currently Hydrating")
        .description(
            "**Compute objects (indexes, materialized views) that haven't "
            "finished hydrating.** Hydration is the process of rebuilding a "
            "dataflow's in-memory state from persisted storage — it runs "
            "after a cluster restart, after creating a replica, and after "
            "some DDL. Nominal: 0 in steady state; non-zero is briefly "
            "expected after restarts. Sustained non-zero (more than a few "
            "minutes) means hydration is stuck — check _Slowest Hydrating "
            "Collections_ next, then pod restart history on _Kubernetes "
            "Workloads_."
        )
        .data(query)
        .visualization(
            visualization.sparkline_stat(shade=shade)
            .min(0)
            .thresholds(
                threshold.time_stable_thresholds(seconds=60 * 60 * 3, high_bad=True)
            )
        ),
    )
    return panel_id


def _active_objects_query(obj_type: str):
    """Build a count query for `v2_mz_production_object` filtered by type.

    Uses `cluster_id=~"$mzClusterList"` so the value tracks the cluster
    selector. `v2_mz_production_object` has one series per
    (cluster_id, collection_id, name) — counting series gives the object
    count.

    TODO(self-managed): `v2_mz_production_object` is the cloud-only catalog
    metric and has no self-managed equivalent (there's `mz_mzd_views_count`
    for materialized views and `mz_views_count` for views, but no per-cluster
    index count). Callers that still rely on this will read "No data" on
    self-managed until a catalog index-count metric is available.
    """
    return query_group(
        promql_query(
            textwrap.dedent(
                f"""
                count(
                    v2_mz_production_object{{$environmentFilter, type="{obj_type}", cluster_id=~"$mzClusterList"}}
                )
                """
            )
        ).legend_format(obj_type),
    )


class ComputeObjectsTab:
    """Compute Objects tab on Overview Dashboard."""

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _active_mzd_views_panel(self):
        """Active materialized views (env-scoped via mz_mzd_views_count)."""
        panel_id = "active-mzd-views"
        # `mz_mzd_views_count` is the self-managed materialized-view count
        # (env-scoped, no cluster label) — the replacement for the cloud-only
        # `v2_mz_production_object{type="materialized-view"}`. `max(...)` dedups
        # across exporter pods.
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    max(mz_mzd_views_count{$environmentFilter})
                    """
                )
            ).legend_format("materialized-views"),
        )
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Materialized Views")
            .description(
                "**Number of materialized views actively maintained by "
                "Materialize.** Each materialized view is a persistent "
                "compute object that incrementally keeps a query's result "
                "up to date — so this count is roughly proportional to "
                "how much work the cluster is doing. Nominal: stable in "
                "steady state; expected to step on `CREATE MATERIALIZED "
                "VIEW` / `DROP MATERIALIZED VIEW`. Sustained drift "
                "suggests automated DDL activity. "
                f"{ENV_SCOPED_NOTE}"
            )
            .data(query)
            .visualization(visualization.sparkline_stat(shade=COMPUTE_THEME).min(0)),
        )
        return panel_id

    def _active_indexes_panel(self):
        """Active indexes (cluster-filterable via cluster_id)."""
        panel_id = "active-indexes"
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Indexes")
            .description(
                "**Number of indexes maintained on the selected clusters.** "
                "An index is an in-memory arrangement that makes `SELECT`s "
                "against its underlying relation effectively instant — at "
                "the cost of memory. Rapid growth here typically pairs with "
                "growing memory usage on the cluster's pods (see "
                "_Kubernetes Workloads -> Pod Memory Usage_). Scoped to the "
                "selected clusters."
            )
            .data(_active_objects_query("index"))
            .visualization(
                visualization.sparkline_stat(shade=COMPUTE_THEME)
                .min(0)
                # TODO(self-managed): no index-count metric exists (see
                # _active_objects_query); reads "No data" until one does.
                .no_value("No index-count metric on self-managed")
            ),
        )
        return panel_id

    def _active_views_panel(self):
        """Active (non-materialized) views.

        `mz_views_count` is environment-scoped and has no cluster label,
        so this panel ignores the cluster/replica selectors. Use `max()` to
        collapse the per-exporter-pod duplicate series safely if more than
        one exporter ends up scraping the same value.
        """
        panel_id = "active-views"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    max(mz_views_count{$environmentFilter})
                    """
                )
            ).legend_format("views"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Views")
            .description(
                "**Views (non-materialized) in the catalog.** Views are "
                "query templates evaluated on demand — they don't consume "
                "compute or memory until something queries them. Mostly a "
                "catalog-shape signal; for read-side activity see "
                "_Connections / Activity -> Query Rate_. "
                f"{ENV_SCOPED_NOTE}"
            )
            .data(query)
            .visualization(visualization.sparkline_stat(shade=COMPUTE_THEME).min(0)),
        )
        return panel_id

    def _active_subscribes_panel(self):
        """Donut: active subscriptions by session_type (system / user).

        Uses `mz_active_subscribes` (which carries `session_type`) rather
        than `mz_compute_controller_subscribe_count` (which has `instance_id`
        but no session_type). The metric is reported by environmentd at the
        env level and does not carry a cluster label, so the cluster/replica
        selectors don't reactively filter this panel.
        """
        panel_id = "active-subscribes"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (session_type) (
                        mz_active_subscribes{$environmentFilter}
                    )
                    """
                )
            )
            .legend_format("{{session_type}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Subscribes")
            .description(
                "**Live SUBSCRIBE sessions — long-running queries that "
                "push updates to a client as the underlying data changes.** "
                "`system` subscribes are Materialize's internal probing / "
                "health checks (always a few); `user` subscribes come from "
                "client connections. A persistently high `user` count is "
                "often a leaked-connection signal. "
                f"{ENV_SCOPED_NOTE}"
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(COMPUTE_THEME)
                )
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .no_value(NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _index_types_panel(self):
        """Donut: indexes by `relation_type` (view, table, materialized-view, …).

        `v2_mz_indexes_count` has the relation_type breakdown but no cluster
        label — this panel is intentionally environment-scoped.

        TODO(self-managed): `v2_mz_indexes_count` is cloud-only with no
        self-managed equivalent, so this donut has no data there. Kept for
        cloud parity; the no_value explains the blank. Fill in once a catalog
        index-count metric (with a relation_type breakdown) lands.
        """
        panel_id = "index-types"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (relation_type) (
                        v2_mz_indexes_count{$environmentFilter}
                    )
                    """
                )
            )
            .legend_format("{{relation_type}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Index Types")
            .description(
                "**Indexes by the underlying relation type** (view / table "
                "/ materialized-view). Most workloads heavily favor indexes "
                "on views — that's the canonical 'maintain a query's "
                "result' pattern. A high ratio of indexes on tables is "
                "unusual and worth understanding (often a misunderstanding "
                "of materialization). "
                f"{ENV_SCOPED_NOTE}"
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(COMPUTE_THEME)
                )
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .no_value(NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _unhydrated_count_panel(self):
        return add_currently_hydrating_panel(self.dashboard)

    def _hydration_queue_panel(self):
        """Timeseries: compute controller hydration queue depth per replica.

        `mz_compute_controller_hydration_queue_size` is reported by
        environmentd with one series per (cluster, replica). When the queue
        is non-zero, work is waiting to be hydrated.
        """
        panel_id = "hydration-queue-size"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (instance_id, replica_id) (
                        mz_compute_controller_hydration_queue_size{
                            $environmentFilter,
                            instance_id=~"$mzClusterList",
                            replica_id=~"$mzReplicaList"
                        }
                    ) > 0
                    """
                )
            ).legend_format("{{instance_id}} / {{replica_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Hydration Queue Size")
            .description(
                "**Collections waiting in the compute controller's "
                "hydration queue, per replica.** environmentd schedules "
                "hydration work in batches; backlog here means it's "
                "queueing faster than replicas can complete it — typical "
                "during mass cluster restarts, atypical otherwise. "
                "Nominal: 0. Sustained non-zero means the replica is "
                "undersized or one slow-hydrating collection is blocking "
                "the line."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("short")
                .min(0)
                .legend(visualization.TS_LEGEND_BUILDER)
                .no_value("Hydration Queue is empty")
            ),
        )
        return panel_id

    def _slowest_hydrating_collections_panel(self):
        """Horizontal bar chart: top-N slowest hydrating collections.

        `v2_mz_compute_hydration_time_seconds{hydrated="1"}` carries the
        seconds it took each collection to hydrate; `topk(N, ...)` keeps the
        N longest individually rather than collapsing per cluster. This
        preserves the within-cluster spread (e.g., the cluster of times
        around 111-112s on the `s2` catalog cluster) and surfaces the
        specific collection_id that was slow.

        Heads-up: `s2` is the `mz_catalog` cluster, which has a very large
        number of internal collections relative to user clusters and tends
        to dominate this chart. If that's noisy in practice, consider
        splitting into "everything except s2" vs "just s2" panels via the
        `instance_id` filter.

        TODO(self-managed): `v2_mz_compute_hydration_time_seconds` is cloud-only
        with no self-managed equivalent, so this chart has no data there. Kept
        for cloud parity (the no_value explains the blank); revisit when a
        per-collection hydration-duration metric exists on self-managed.
        """
        panel_id = "hydration-slowest-collections"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    topk(15,
                        v2_mz_compute_hydration_time_seconds{
                            $environmentFilter,
                            instance_id=~"$mzClusterList",
                            replica_id=~"$mzReplicaList",
                            hydrated="1"
                        }
                    )
                    """
                )
            )
            .legend_format("{{instance_id}} / {{replica_id}} / {{collection_id}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Slowest Hydrating Collections")
            .description(
                "**The 15 collections that took the longest to finish "
                "hydrating** in the current time range. Hydration time "
                "scales roughly with the size of the maintained state, so "
                "large materialized views and indexes naturally lead the "
                "list. Sudden growth on a previously-fast collection "
                "usually means its input data grew. Translate "
                "`collection_id` to a name via `SELECT id, name FROM "
                "mz_internal.mz_indexes` (or `mz_materialized_views`). "
                "Heads-up: `s2` (`mz_catalog_server`) typically dominates "
                "this chart with internal system collections — that's "
                "expected and noise-floor."
            )
            .data(query)
            .visualization(
                barchart_builder.Visualization()
                .orientation(common.VizOrientation.HORIZONTAL)
                .unit("s")
                .scale_distribution(
                    common_builder.ScaleDistributionConfig()
                    .type(common.ScaleDistribution.LOG)
                    .log(10)
                )
                .bar_width(0.8)
                .group_width(0.95)
                .no_value(NO_FILTER_MATCH)
                .x_tick_label_spacing(100)
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(COMPUTE_THEME)
                )
                .thresholds(
                    threshold.time_stable_thresholds(seconds=60 * 60 * 6, high_bad=True)
                )
                .thresholds_style(
                    common_builder.GraphThresholdsStyleConfig().mode(
                        common.GraphThresholdsStyleMode.AREA
                    )
                )
            ),
        )
        return panel_id

    def _arrangement_rate_panel(self):
        """Timeseries: arrangement maintenance CPU summed across workers.

        An arrangement is differential-dataflow's in-memory indexed snapshot
        of a relation. Every index and materialized view has at least one,
        and workers spend CPU maintaining them as input data changes.

        Value semantics: `rate(...seconds_total)` is CPU-seconds per second,
        i.e. fraction of a CPU thread. Aggregated across workers in a
        replica, an N-worker replica fully saturated on arrangement
        maintenance reads as N.0.
        """
        panel_id = "arrangement-rate"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (
                        {ARRANGEMENT_LABEL_CLUSTER_ID},
                        {ARRANGEMENT_LABEL_REPLICA_ID}
                    ) (
                        rate(
                            mz_arrangement_maintenance_seconds_total{{{_ARRANGEMENT_FILTER}}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format(
                f"{{{{{ARRANGEMENT_LABEL_CLUSTER_ID}}}}}"
                f" / {{{{{ARRANGEMENT_LABEL_REPLICA_ID}}}}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Arrangement Maintenance Rate")
            .description(
                "**CPU-cores spent maintaining arrangements** — the "
                "in-memory indexed snapshots that back every index and "
                "materialized view. Summed across workers in each "
                "replica, so an N-worker replica can hit N. Nominal: "
                "well below worker count. Sustained near-max indicates "
                "the replica is bottlenecked on maintenance work — "
                "usually high upstream change rate or many heavy "
                "indexes. The catalog cluster's normal baseline is "
                "higher than typical user clusters."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("none")
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _arrangement_rate_by_worker_panel(self):
        """Timeseries: arrangement maintenance CPU per worker.

        Same metric as the aggregate panel above but one series per
        (cluster, replica, worker_id) — useful for spotting work imbalance
        between workers within a replica.
        """
        panel_id = "arrangement-rate-by-worker"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (
                        {ARRANGEMENT_LABEL_CLUSTER_ID},
                        {ARRANGEMENT_LABEL_REPLICA_ID},
                        worker_id
                    ) (
                        rate(
                            mz_arrangement_maintenance_seconds_total{{{_ARRANGEMENT_FILTER}}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format(
                f"{{{{{ARRANGEMENT_LABEL_CLUSTER_ID}}}}}"
                f" / {{{{{ARRANGEMENT_LABEL_REPLICA_ID}}}}}"
                " / w{{worker_id}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Arrangement Maintenance Rate (per worker)")
            .description(
                "**Same metric as the aggregate, but split per worker** — "
                "each worker tops out at 1.0. Watch for skew: if one "
                "worker series sits near 1.0 while siblings sit near 0, "
                "the cluster is bottlenecked on that single worker (a "
                "'hot key' or hot arrangement). Skew is the most common "
                "reason an 8-worker cluster behaves like a 1-worker one. "
                "If you see it, the next step is usually `EXPLAIN "
                "PHYSICAL PLAN` on the offending object to find the "
                "heavy operator."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("percentunit")
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def build_summary_row(self) -> dashboardv2_builders.Row:
        """Summary row: 4 active-counts + 1 index-types donut, single auto-row."""
        return (
            dashboardv2_builders.Row()
            .title("Compute Objects Summary")
            .hide_header(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .column_width_mode("narrow")
                .max_column_count(5)
                .with_item(self._active_mzd_views_panel())
                .with_item(self._active_indexes_panel())
                .with_item(self._active_views_panel())
                .with_item(self._active_subscribes_panel())
                .with_item(self._index_types_panel())
            )
        )

    def build_freshness_row(self) -> dashboardv2_builders.Row:
        """Freshness row — stub.

        Reserved for end-to-end freshness lag (how far behind real-time
        each materialized view / index is). Title-only row for now so
        the section slot exists; panels will be added in a follow-up.
        """
        return (
            dashboardv2_builders.Row()
            .title("Freshness")
            .hide_header(False)
            .layout(dashboardv2_builders.AutoGrid())
        )

    def build_hydration_row(self) -> dashboardv2_builders.Row:
        """Hydration row: currently-hydrating stat, queue depth, slowest per cluster."""
        return (
            dashboardv2_builders.Row()
            .title("Hydration")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .with_item(self._unhydrated_count_panel())
                .with_item(self._hydration_queue_panel())
                .with_item(self._slowest_hydrating_collections_panel())
            )
        )

    def _dataflow_count_panel(self):
        """Timeseries: per-replica dataflow count.

        `mz_compute_replica_history_dataflow_count` is reported per
        (cluster, replica, worker). Dataflows are replicated across all
        workers in a replica, so each worker reports the same count
        (8 workers x 7 dataflows is not 56 dataflows; it is just 7). We take
        `max by (cluster, replica)` so the panel surfaces the actual
        per-replica count rather than a worker-multiplied sum.

        Dataflows are the underlying execution units for indexes,
        materialized views, and subscribes — each compute object becomes
        one or more dataflows on its replica.
        """
        panel_id = "dataflow-count"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    max by (
                        {ARRANGEMENT_LABEL_CLUSTER_ID},
                        {ARRANGEMENT_LABEL_REPLICA_ID}
                    ) (
                        mz_compute_replica_history_dataflow_count{{{_ARRANGEMENT_FILTER}}}
                    )
                    """
                )
            ).legend_format(
                f"{{{{{ARRANGEMENT_LABEL_CLUSTER_ID}}}}}"
                f" / {{{{{ARRANGEMENT_LABEL_REPLICA_ID}}}}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Dataflow Count")
            .description(
                "**Number of active dataflows running on each replica.** "
                "Every index, materialized view, and live `SUBSCRIBE` "
                "manifests as one or more dataflows on a replica — so "
                "this count rises with DDL and SUBSCRIBE activity. "
                "Nominal: stable for steady workloads. A sharp drop "
                "without correlating DDL usually means a replica restart "
                "(cross-check _Kubernetes Workloads_); a sharp rise "
                "typically means bulk object creation."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("short")
                .min(0)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _dataflow_count_by_worker_panel(self):
        """Timeseries: dataflow count broken out per worker.

        Workers in the same replica should always agree on the count.
        Visible divergence between worker series here is a signal that
        something has gone wrong with the dataflow replication.
        """
        panel_id = "dataflow-count-by-worker"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    max by (
                        {ARRANGEMENT_LABEL_CLUSTER_ID},
                        {ARRANGEMENT_LABEL_REPLICA_ID},
                        worker_id
                    ) (
                        mz_compute_replica_history_dataflow_count{{{_ARRANGEMENT_FILTER}}}
                    )
                    """
                )
            ).legend_format(
                f"{{{{{ARRANGEMENT_LABEL_CLUSTER_ID}}}}}"
                f" / {{{{{ARRANGEMENT_LABEL_REPLICA_ID}}}}}"
                " / w{{worker_id}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Dataflow Count (per worker)")
            .description(
                "**Per-worker view of the dataflow count.** Workers within "
                "the same replica run in lockstep and should always see "
                "the same dataflows — so the worker series for a given "
                "replica should overlap exactly. Visible divergence "
                "between worker series within a single (cluster, replica) "
                "tuple is a bug-class signal and worth filing."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("short")
                .min(0)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _dataflow_elapsed_rate_panel(self):
        """Timeseries: total cores busy in dataflows per cluster (log Y-axis).

        `mz_dataflow_elapsed_seconds_total` is a per-(collection,
        replica, worker) counter of cumulative CPU-seconds inside
        dataflows. `sum by (instance_id) (rate(...))` gives total cores
        busy per cluster — broader than the arrangement maintenance rate
        panel (which is just the maintenance subset of dataflow work).

        Aggregating away `collection_id`, `replica_id`, and `worker_id`
        is deliberate: at scale (hundreds of collections * replicas *
        workers), keeping that cardinality has made graphs fail to load
        on larger customer environments. Specialists can drill down via
        ad-hoc queries when needed; the dashboard prioritizes
        reliability at high granularity over drill-down convenience.

        Log Y-axis keeps idle clusters near zero visible alongside busy
        ones at >1 core — common pattern in this environment is one
        cluster (e.g. mz_catalog_server) sitting at 1-3 cores while
        everything else is in the 0.001-0.01 range.
        """
        panel_id = "dataflow-elapsed-rate"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (instance_id) (
                        rate(
                            mz_dataflow_elapsed_seconds_total{
                                $environmentFilter,
                                instance_id=~"$mzClusterList",
                                replica_id=~"$mzReplicaList"
                            }[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{instance_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Dataflow Elapsed Rate")
            .description(
                "**CPU-cores busy inside dataflows, per cluster.** Covers "
                "all dataflow work — arrangement maintenance, evaluation, "
                "and hydration. Capped by cluster size: a `400cc` cluster "
                "can't exceed 400 cores. Nominal: well below cluster "
                "size; sustained near-max means the cluster is CPU-bound "
                "and a candidate for upsizing. The catalog cluster (`s2`) "
                "is typically the busiest by far in any environment. Log "
                "Y-axis so idle and busy clusters share the chart. For "
                "maintenance-only breakdown see the _Arrangements_ row "
                "below."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("none")
                .scale_distribution(
                    common_builder.ScaleDistributionConfig()
                    .type(common.ScaleDistribution.LOG)
                    .log(10)
                )
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def build_dataflows_row(self) -> dashboardv2_builders.Row:
        """Dataflows row: counts (per-replica + per-worker) + elapsed rate."""
        return (
            dashboardv2_builders.Row()
            .title("Dataflows")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .with_item(self._dataflow_count_panel())
                .with_item(self._dataflow_count_by_worker_panel())
                .with_item(self._dataflow_elapsed_rate_panel())
            )
        )

    def build_arrangements_row(self) -> dashboardv2_builders.Row:
        """Arrangements row: aggregate + per-worker maintenance CPU rate.

        Three tables split by collection_id prefix (system / user /
        transient+none). Tables rather than graphs because the values are
        near-static — Min/Max columns surface the occasional spike that
        a time series would otherwise hide.
        """
        return (
            dashboardv2_builders.Row()
            .title("Arrangements")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .column_width_mode("wide")
                .with_item(self._arrangement_rate_panel())
                .with_item(self._arrangement_rate_by_worker_panel())
                .with_item(self._arrangement_records_system_panel())
                .with_item(self._arrangement_records_user_panel())
                .with_item(self._arrangement_records_transient_panel())
            )
        )

    def _arrangement_records_table(
        self,
        panel_id: str,
        title: str,
        collection_id_regex: str,
        description: str,
    ):
        """Build a table of `mz_arrangement_record_count` per collection.

        Records per collection are nearly static — graphs are uninteresting,
        but Min/Max over $__range catch occasional spikes that Last alone
        would miss.

        `max by (collection_id)` collapses the per-(replica, worker)
        duplicates (workers in a replica agree; replicas of a cluster agree
        when hydrated).

        The Reduce transformation in `seriesToRows` mode produces one row
        per series with the three calc columns; SortBy puts the biggest
        current value at the top.
        """
        query = (
            query_group(
                promql_query(
                    textwrap.dedent(
                        f"""
                        max by (collection_id) (
                            mz_arrangement_record_count{{
                                $environmentFilter,
                                instance_id=~"$mzClusterList",
                                replica_id=~"$mzReplicaList",
                                collection_id=~"{collection_id_regex}"
                            }}
                        )
                        """
                    )
                ).legend_format("{{collection_id}}"),
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("reduce")
                .id("reduce")
                .options(
                    {
                        "reducers": ["min", "max", "lastNotNull"],
                        "mode": "seriesToRows",
                    }
                )
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("organize")
                .id("organize")
                .options({"renameByName": {"Field": "Collection ID"}})
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("sortBy")
                .id("sortBy")
                .options(
                    {
                        "fields": {},
                        "sort": [{"field": "Last *", "desc": True}],
                    }
                )
            )
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title(title)
            .description(description)
            .data(query)
            .visualization(
                table.Visualization()
                .show_header(True)
                .filterable(True)
                .unit("short")
                .no_value(NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _arrangement_records_system_panel(self):
        return self._arrangement_records_table(
            panel_id="arrangement-records-system",
            title="System Collections — Record Counts",
            collection_id_regex="s.*",
            description=(
                "**Row counts of arrangements maintained for Materialize's "
                "internal system collections** (`collection_id` starts with "
                "`s`). These back catalog tables, internal probes, and "
                "other infrastructure — they're not user data and "
                "shouldn't grow with workload. Columns are Min / Max / "
                "Last over the selected time range, sorted by Last "
                "(largest current arrangements first). Useful for spotting "
                "unexpected growth in system collections, which can "
                "indicate a Materialize bug."
            ),
        )

    def _arrangement_records_user_panel(self):
        return self._arrangement_records_table(
            panel_id="arrangement-records-user",
            title="User Collections — Record Counts",
            collection_id_regex="u.*",
            description=(
                "**Row counts of arrangements maintained for user-created "
                "compute objects** (`collection_id` starts with `u`). "
                "This is the row count of every index and materialized "
                "view on the selected clusters — the primary driver of "
                "memory consumption. Sudden growth on a specific "
                "collection tracks the size of its underlying data. "
                "Columns are Min / Max / Last over the time range; sorted "
                "by Last desc so the largest current arrangements are at "
                "the top. Translate `collection_id` to a name via `SELECT "
                "id, name FROM mz_internal.mz_indexes` (or "
                "`mz_materialized_views`)."
            ),
        )

    def _arrangement_records_transient_panel(self):
        return self._arrangement_records_table(
            panel_id="arrangement-records-transient",
            title="Transient / Uncategorized — Record Counts",
            collection_id_regex="t.*|none",
            description=(
                "**Row counts of arrangements with `collection_id` "
                "starting with `t` (transient) or labeled `none`.** "
                "Transient arrangements are short-lived intermediates "
                "created during query optimization and dataflow "
                "execution; the `none` sentinel is for arrangements "
                "whose collection is unidentified. Both are usually "
                "small and ephemeral. Sustained non-trivial entries "
                "here are worth investigating — they may indicate stuck "
                "or leaked dataflows."
            ),
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a compute objects tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Compute Objects")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_summary_row())
                .row(self.build_freshness_row())
                .row(self.build_hydration_row())
                .row(self.build_dataflows_row())
                .row(self.build_arrangements_row())
            )
        )
