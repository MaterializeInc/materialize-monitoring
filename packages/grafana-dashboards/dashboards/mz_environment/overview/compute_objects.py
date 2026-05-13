"""Compute Objects tab on Overview Dashboard.

Compute objects include Indexes, Materialized Views, Subscriptions.
"""

from __future__ import annotations

import textwrap

from grafana_foundation_sdk.builders import (
    piechart as piechart_builder,
)
from grafana_foundation_sdk.builders import timeseries
from grafana_foundation_sdk.models import piechart
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import promql_query, query_group

from dashboards import palette, visualization

NO_FILTER_MATCH = "No matches for the current filters"
# Some metrics in this tab are pre-calculated by the promsql-exporter at the
# environment level and don't carry cluster_id / instance_id labels — when
# the user filters to a specific cluster the value is unchanged. Surface this
# in panel descriptions so the lack of reactivity isn't misleading.
ENV_SCOPED_NOTE = "Environment-scoped — not affected by the cluster/replica filters."

COMPUTE_THEME = palette.THEME_PALETTE[3]

# Long-form cluster/replica label names used by mz_arrangement_* and other
# per-replica compute metrics. Materialize's prometheus scraper attaches the
# environmentd-side identifiers under these specific names (separate from the
# shorter `cluster_id` / `instance_id` used on other metric families).
ARRANGEMENT_LABEL_CLUSTER_ID = "cluster_environmentd_materialize_cloud_cluster_id"
ARRANGEMENT_LABEL_CLUSTER_NAME = "cluster_environmentd_materialize_cloud_cluster_name"
ARRANGEMENT_LABEL_REPLICA_ID = "cluster_environmentd_materialize_cloud_replica_id"
ARRANGEMENT_LABEL_REPLICA_NAME = "cluster_environmentd_materialize_cloud_replica_name"

# A short PromQL fragment that filters by the current $environmentFilter,
# $mzClusterList, and $mzReplicaList using the long-form label names.
_ARRANGEMENT_FILTER = (
    f"$environmentFilter, "
    f'{ARRANGEMENT_LABEL_CLUSTER_ID}=~"$mzClusterList", '
    f'{ARRANGEMENT_LABEL_REPLICA_ID}=~"$mzReplicaList"'
)


def _active_objects_query(obj_type: str):
    """Build a count query for `v2_mz_production_object` filtered by type.

    Uses `cluster_id=~"$mzClusterList"` so the value tracks the cluster
    selector. `v2_mz_production_object` has one series per
    (cluster_id, collection_id, name) — counting series gives the object
    count.
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
        """Active materialized views (cluster-filterable via cluster_id)."""
        panel_id = "active-mzd-views"
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Materialized Views")
            .description(
                "Materialized views in the catalog, scoped to the selected clusters."
            )
            .data(_active_objects_query("materialized-view"))
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
            .description("Indexes in the catalog, scoped to the selected clusters.")
            .data(_active_objects_query("index"))
            .visualization(visualization.sparkline_stat(shade=COMPUTE_THEME).min(0)),
        )
        return panel_id

    def _active_views_panel(self):
        """Active (non-materialized) views.

        `v2_mz_views_count` is environment-scoped and has no cluster label,
        so this panel ignores the cluster/replica selectors. Use `max()` to
        collapse the per-exporter-pod duplicate series safely if more than
        one promsql-exporter ends up scraping the same value.
        """
        panel_id = "active-views"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    max(v2_mz_views_count{$environmentFilter})
                    """
                )
            ).legend_format("views"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Views")
            .description(f"Views in the catalog. {ENV_SCOPED_NOTE}")
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
                "Live SUBSCRIBE sessions broken down by session type. "
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
                f"Indexes broken down by the underlying relation type. {ENV_SCOPED_NOTE}"
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
                        {ARRANGEMENT_LABEL_CLUSTER_NAME},
                        {ARRANGEMENT_LABEL_REPLICA_ID},
                        {ARRANGEMENT_LABEL_REPLICA_NAME}
                    ) (
                        rate(
                            mz_arrangement_maintenance_seconds_total{{{_ARRANGEMENT_FILTER}}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format(
                f"{{{{{ARRANGEMENT_LABEL_CLUSTER_NAME}}}}}"
                f" / {{{{{ARRANGEMENT_LABEL_REPLICA_NAME}}}}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Arrangement Maintenance Rate")
            .description(
                "CPU-seconds per second spent maintaining arrangements, "
                "summed across workers in each replica. 1.0 = one CPU "
                "thread fully busy; an 8-worker replica can reach 8.0."
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
                        {ARRANGEMENT_LABEL_CLUSTER_NAME},
                        {ARRANGEMENT_LABEL_REPLICA_ID},
                        {ARRANGEMENT_LABEL_REPLICA_NAME},
                        worker_id
                    ) (
                        rate(
                            mz_arrangement_maintenance_seconds_total{{{_ARRANGEMENT_FILTER}}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format(
                f"{{{{{ARRANGEMENT_LABEL_CLUSTER_NAME}}}}}"
                f" / {{{{{ARRANGEMENT_LABEL_REPLICA_NAME}}}}}"
                " / w{{worker_id}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Arrangement Maintenance Rate (per worker)")
            .description(
                "CPU fraction per worker spent maintaining arrangements. "
                "1.0 = one worker thread fully saturated. Useful for spotting "
                "skew between workers in the same replica."
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

    def build_arrangements_row(self) -> dashboardv2_builders.Row:
        """Arrangements row: aggregate + per-worker maintenance CPU rate."""
        return (
            dashboardv2_builders.Row()
            .title("Arrangements")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(2)
                .with_item(self._arrangement_rate_panel())
                .with_item(self._arrangement_rate_by_worker_panel())
            )
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a compute objects tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Compute Objects")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_summary_row())
                .row(self.build_arrangements_row())
            )
        )
