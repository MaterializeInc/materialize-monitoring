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

# Per-percentile descriptions for the Peek Latency panels. Each percentile
# tells the operator something different (typical vs tail vs worst-case),
# so the three panels deliberately carry tailored writeups instead of one
# generic "latency at PN" line.
_PEEK_LATENCY_DESCRIPTIONS: dict[str, str] = {
    "p50": (
        "**Median read-query latency** — the typical time it takes to "
        "look up the current state of an arrangement (the operation "
        "behind every `SELECT … FROM <view>` against an index). p50 is "
        'your "what does a normal query feel like" number. Nominal: '
        "typically a few milliseconds on a healthy cluster. Sustained "
        "multi-second p50 means the cluster is overwhelmed. One line "
        "per cluster — narrow the cluster selector to focus. Log "
        "Y-axis. See also: _Dataflow Elapsed "
        "Rate_ and _Arrangement Maintenance Rate_ on the _Compute "
        "Objects_ tab."
    ),
    "p90": (
        '**90th-percentile read-query latency** — "how slow do my '
        'slowest 10% of queries feel?" Catches contention bursts and '
        "rarely-hit cold paths that p50 hides. Nominal: usually a "
        "small multiple of p50 (2-5x). If p90 is 10-100x p50, your "
        "latency distribution is bimodal — typically cold-cache "
        "effects on infrequently-queried indexes or contention. Same "
        "per-cluster split as p50."
    ),
    "p99": (
        "**Tail read-query latency (99th percentile)** — the slowest "
        "1% of queries, the ones users complain about. Nominal: a "
        "small multiple of p50 (typically 2-10x), with occasional "
        "spikes during query plan recompilation or hydration. "
        "Sustained p99 in the seconds range — especially when *not* "
        "paired with elevated p50/p90 — points at a single bad query "
        "or a tail-latency-sensitive use case worth investigating "
        "directly. Pair with _Query Rate_ above to confirm the "
        "latency is happening on actual traffic, not just idle scrapes."
    ),
}


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
                "**Currently-open SQL sessions, broken down by session "
                "type (`system` vs `user`).** `system` sessions come "
                "from Materialize's internal probing (a few are always "
                "present); `user` sessions come from client connections. "
                "Nominal: a small steady `system` count and a variable "
                "`user` count tracking your client activity. Sustained "
                "high `user` count is often a leaked-connection signal "
                "— sanity-check by seeing whether _Active Queries_ "
                "shows commensurate activity. Environment-scoped."
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
                "**Queries per second by session type, rated from the "
                "`mz_query_total` counter.** Bursty in normal operation "
                "— `user` tracks your client traffic shape, `system` "
                "reflects internal health-checks (typically a steady "
                "single-digit baseline). Use _Query Distribution_ to see "
                "*what kinds* of queries make up the rate, and _Peek "
                "Latency_ to confirm the queries are running fast "
                "enough. Environment-scoped."
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
                "**Commands per second across the adapter** — the SQL "
                "protocol layer that handles parse, execute, prepare, "
                "fetch, etc. Usually higher than the query rate because "
                "each query produces several commands. Sudden flat-line "
                "on a usually-busy env is unusual (could indicate "
                "adapter trouble). Use _Adapter Commands by Application_ "
                "below to see which clients dominate, and watch its "
                "Errors column for failed commands. Environment-scoped."
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
                "**Share of queries by statement type over the "
                "dashboard's time range** — uses `increase()`, so the "
                "slice sizes are total counts over the time selector, "
                "not per-second rates. Workload-shape signal. Heavy "
                "`set_variable` / `reset_variable` / `fetch` traffic is "
                "normal — that's how PostgreSQL clients manage session "
                "state. Heavy `insert` / `update` / `delete` on a "
                "service you think of as read-mostly is worth "
                "investigating. Idle statement types are filtered out "
                "(`> 0`). Environment-scoped."
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
                "**Queries per second broken down by statement type AND "
                "session type, fully time-resolved.** Pairs with the "
                "_Query Distribution_ donut (which shows the time-range "
                "total): this panel shows *how those slices move over "
                "time*. Watch for sudden spikes in `select / user` — "
                "pair with _Peek Latency (p99)_ to see if the system "
                "kept up. Idle (statement, session) tuples are filtered "
                "out. Environment-scoped."
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

        `mz_compute_peek_duration_seconds_*` is a histogram of
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
        # `mz_compute_peek_duration_seconds` is reported environmentd-side and
        # carries `instance_id` (the cluster) but NOT `replica_id` — so peek
        # latency is per-cluster here, and the replica selector does not split
        # it further. (The cloud-only `v2_mz_compute_replica_peek_duration_*`
        # histogram was per-replica; no self-managed equivalent exists.)
        bucket_filter = '$environmentFilter, instance_id=~"$mzClusterList"'
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    histogram_quantile({percentile},
                        sum by (le, instance_id) (
                            rate(
                                mz_compute_peek_duration_seconds_bucket{{{bucket_filter}}}[$__rate_interval]
                            )
                        )
                    )
                    """
                )
            ).legend_format("{{instance_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title(f"Peek Latency ({percentile_label})")
            .description(_PEEK_LATENCY_DESCRIPTIONS[percentile_label])
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
                "**Adapter command totals per `application_name` over "
                "the dashboard time range, split into Success and Errors "
                "columns.** Rows sorted by Errors (descending) so "
                "anything bad floats to the top. The Errors column is "
                "threshold-colored — non-zero jumps out visually. Most "
                "clients set `application_name` via the PostgreSQL "
                "connection string; clients that don't are bucketed as "
                "`unrecognized` or `unspecified` (normal). Sustained "
                "non-zero Errors on a real application means that app "
                "is consistently failing — investigate by correlating "
                "with that application's own logs, and inspect recent "
                "failures via Materialize's `mz_internal` activity-log "
                "views."
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
