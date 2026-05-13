"""Connections / Activity tab on Overview Dashboard.

Focused on automated/persistent connections (sources, sinks) and user
sessions, plus query and adapter activity. Plenty of room to break each
of these down further as the tab fills out.
"""

from __future__ import annotations

import textwrap

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

CONNECTIONS_THEME = palette.THEME_PALETTE[1]


class ConnectionsActivityTab:
    """Connections / Activity tab on Overview Dashboard."""

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _active_sessions_panel(self):
        """Sparkline stat: currently-open sessions by session_type.

        `mz_active_sessions` is a gauge with a `session_type` label
        (system / user). Each session_type renders as its own stat tile
        with its own sparkline. Environment-scoped — not affected by the
        cluster/replica selectors.
        """
        panel_id = "connections-active-sessions"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (session_type) (
                        mz_active_sessions{$environmentFilter}
                    )
                    """
                )
            ).legend_format("{{session_type}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Sessions")
            .description(
                "Currently-open SQL sessions, broken down by session type "
                "(system / user). Multiple stat tiles, one per session type."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=CONNECTIONS_THEME)
                .min(0)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
            ),
        )
        return panel_id

    def _active_queries_panel(self):
        """Sparkline stat: query rate by session_type.

        `mz_query_total` is a counter (queries seen since pod start), so
        we `rate()` it and `sum by (session_type)` to get queries/sec
        broken down by session type.
        """
        panel_id = "connections-active-queries"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (session_type) (
                        rate(mz_query_total{$environmentFilter}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{session_type}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Queries")
            .description(
                "Query rate (queries/sec) by session type. "
                "Rated from the mz_query_total counter."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=CONNECTIONS_THEME)
                .min(0)
                .unit("cps")
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
            ),
        )
        return panel_id

    def _adapter_command_rate_panel(self):
        """Sparkline stat: total adapter command rate.

        `mz_adapter_commands` is a counter with `command_type` and
        `status` labels. We sum across both for the headline number;
        future drilldown panels could break out by command_type
        (parse / execute / prepare / etc.) or status.
        """
        panel_id = "connections-adapter-command-rate"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum(
                        rate(mz_adapter_commands{$environmentFilter}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("commands"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Adapter Command Rate")
            .description(
                "Adapter command rate across the environment "
                "(commands/sec, summed across command_type and status). "
                "Drilldown by command_type/status is a natural follow-up."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=CONNECTIONS_THEME).min(0).unit("cps")
            ),
        )
        return panel_id

    def _query_distribution_panel(self):
        """Donut: query distribution by statement_type over the time range.

        `increase(mz_query_total[$__range])` gives total queries per
        labeled tuple over the dashboard time range. Summing by
        `statement_type` collapses every other dimension — this panel is
        about workload composition, not where the queries came from.
        The `> 0` keeps statement types with zero activity out of the
        legend.
        """
        panel_id = "queries-distribution"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (statement_type) (
                        increase(mz_query_total{$environmentFilter}[$__range])
                    ) > 0
                    """
                )
            )
            .legend_format("{{statement_type}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Query Distribution (by statement_type)")
            .description(
                "Share of queries by statement type over the current time "
                "range. Workload-shape signal — what kinds of queries are "
                "hitting this environment."
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .no_value(visualization.NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _query_rate_panel(self):
        """Timeseries: query rate broken out by (statement_type, session_type).

        Up to 11 statement_types * 2 session_types = 22 potential series,
        but `> 0` drops the empties — in practice closer to one series per
        active statement-type-on-session-type combination.
        """
        panel_id = "queries-rate-by-statement"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (statement_type, session_type) (
                        rate(mz_query_total{$environmentFilter}[$__rate_interval])
                    ) > 0
                    """
                )
            ).legend_format("{{statement_type}} / {{session_type}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Query Rate (by statement_type / session_type)")
            .description(
                "Queries per second per (statement_type, session_type). "
                "Pairs with the distribution donut: the donut shows the "
                "shape over the whole range, this shows it over time and "
                "splits user vs system."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("cps")
                .min(0)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _peek_latency_panel(self, percentile: float, percentile_label: str):
        """Single-percentile peek-latency timeseries, split per (cluster, replica).

        `v2_mz_compute_replica_peek_duration_seconds_*` is a histogram of
        the operation that SELECT-against-an-index actually performs in
        compute. "Peek" is differential-dataflow's name for "look up the
        current state of an arrangement" — for read-heavy workloads this
        is the closest thing to user-facing query latency.

        Called once per percentile so each panel has a single dimension
        (just the per-replica spread for one quantile) rather than mixing
        percentiles into one chart. Title carries the percentile so the
        legend can be just cluster/replica.

        Log Y-axis because peek latencies span orders of magnitude.
        """
        panel_id = f"queries-peek-latency-{percentile_label}"
        bucket_filter = (
            "$environmentFilter, "
            'instance_id=~"$mzClusterList", '
            'replica_id=~"$mzReplicaList"'
        )
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    histogram_quantile({percentile},
                        sum by (le, instance_id, replica_id) (
                            rate(
                                v2_mz_compute_replica_peek_duration_seconds_bucket{{{bucket_filter}}}[$__rate_interval]
                            )
                        )
                    )
                    """
                )
            ).legend_format("{{instance_id}} / {{replica_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title(f"Peek Latency ({percentile_label})")
            .description(
                f"Read-query (peek) latency at {percentile_label}, "
                "per (cluster, replica). Log Y-axis."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("s")
                .scale_distribution(
                    common_builder.ScaleDistributionConfig()
                    .type(common.ScaleDistribution.LOG)
                    .log(10)
                )
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def build_queries_row(self) -> dashboardv2_builders.Row:
        """Queries row: workload distribution + rate breakdown + per-percentile latency."""
        return (
            dashboardv2_builders.Row()
            .title("Queries")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .with_item(self._query_distribution_panel())
                .with_item(self._query_rate_panel())
                .with_item(self._peek_latency_panel(0.50, "p50"))
                .with_item(self._peek_latency_panel(0.90, "p90"))
                .with_item(self._peek_latency_panel(0.99, "p99"))
            )
        )

    def build_summary_row(self) -> dashboardv2_builders.Row:
        """Connection summary: active sessions, query rate, adapter rate."""
        return (
            dashboardv2_builders.Row()
            .title("Connection Summary")
            .hide_header(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .max_column_count(3)
                .with_item(self._active_sessions_panel())
                .with_item(self._active_queries_panel())
                .with_item(self._adapter_command_rate_panel())
            )
        )

    def _adapter_commands_by_application_panel(self):
        """Table: adapter command counts per application_name, split by status.

        Single instant query grouped by (application_name, status), then
        `groupingToMatrix` pivots `status` into columns — so we get one
        row per application with Success and Errors columns side by side.

        The two-queries-plus-joinByField approach is the obvious one but
        bites you on cardinality: joinByField makes one Value column per
        input frame, so N applications + M statuses balloons into N*M
        value columns instead of the 2 you want. Pivoting via
        groupingToMatrix is the cleaner path.

        `or vector(0)` keeps the query non-empty when nothing has
        happened — otherwise the table renders "No matches" everywhere.
        """
        panel_id = "adapter-commands-by-app"
        query = (
            query_group(
                promql_query(
                    textwrap.dedent(
                        """
                        sum by (application_name, status) (
                            increase(mz_adapter_commands{$environmentFilter}[$__range])
                        )
                        """
                    )
                ).instant(),
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("labelsToFields")
                .id("labelsToFields")
                .options({"keepLabels": ["application_name", "status"]})
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("merge")
                .id("merge")
                .options({})
            )
            .transformation(
                # Pivot: rowField becomes the row identifier column,
                # columnField becomes the column names, valueField fills
                # the cells. The row-identifier column comes out named
                # literally `<rowField>\<columnField>`.
                transform_builders.CompatTransformationBuilder()
                .group("groupingToMatrix")
                .id("groupingToMatrix")
                .options(
                    {
                        "rowField": "application_name",
                        "columnField": "status",
                        "valueField": "Value",
                        "emptyValue": "zero",
                    }
                )
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("organize")
                .id("organize")
                .options(
                    {
                        "renameByName": {
                            # The literal backslash-joined row-id column:
                            "application_name\\status": "Application",
                            "success": "Success",
                            "error": "Errors",
                        },
                        "indexByName": {
                            "application_name\\status": 0,
                            "success": 1,
                            "error": 2,
                        },
                    }
                )
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("sortBy")
                .id("sortBy")
                .options(
                    {
                        "fields": {},
                        "sort": [{"field": "Errors", "desc": True}],
                    }
                )
            )
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Adapter Commands by Application")
            .description(
                "Total adapter commands per application_name over the "
                "dashboard time range, split into Success and Errors. "
                "The Errors column is threshold-colored — non-zero errors "
                "should jump out."
            )
            .data(query)
            .visualization(
                table.Visualization()
                .show_header(True)
                .filterable(True)
                .unit("short")
                .no_value(visualization.NO_FILTER_MATCH)
                .override_by_name(
                    "Errors",
                    [
                        dashboardv2.DynamicConfigValue(
                            id_val="thresholds",
                            value=threshold.error_thresholds().build(),
                        ),
                        dashboardv2.DynamicConfigValue(
                            id_val="custom.cellOptions",
                            value={"type": "color-background"},
                        ),
                    ],
                )
            ),
        )
        return panel_id

    def build_adapter_commands_row(self) -> dashboardv2_builders.Row:
        """Adapter Commands row: per-application Success/Errors breakdown table."""
        return (
            dashboardv2_builders.Row()
            .title("Adapter Commands")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(1)
                .with_item(self._adapter_commands_by_application_panel())
            )
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate the Connections / Activity tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Connections / Activity")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_summary_row())
                .row(self.build_queries_row())
                .row(self.build_adapter_commands_row())
            )
        )
