"""Storage Objects tab on Overview Dashboard.

Storage Objects include Sources, Sinks, Tables, and Persisted
Materialized Views.
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

# Compute-side storage metrics (mz_source_*, mz_sink_*, etc.) use the
# long-form `cluster_environmentd_materialize_cloud_*` label family — same
# convention as arrangement/dataflow metrics. This is the PromQL fragment
# that filters by env + cluster + replica using those labels.
_COMPUTE_FILTER = (
    "$environmentFilter, "
    'cluster_environmentd_materialize_cloud_cluster_id=~"$mzClusterList", '
    'cluster_environmentd_materialize_cloud_replica_id=~"$mzReplicaList"'
)

# Pre-calc storage counts are env-scoped — the metrics don't carry
# cluster_id/instance_id labels, so the dashboard cluster/replica filters
# don't change these stats. Surfaced in panel descriptions.
ENV_SCOPED_NOTE = "Environment-scoped — not affected by the cluster/replica filters."

STORAGE_THEME = palette.THEME_PALETTE[
    4
]  # yellow — distinct from compute (orange), connections (teal), k8s (cyan)


def _env_total_count_query(metric_name: str):
    """Build a deduped env-wide count query for a pre-calc storage metric.

    Some of these metrics (sources_count, sinks_count) carry breakdown
    labels like `type`/`envelope_type`/`size` — `sum by (instance)`
    collapses those into a single per-scrape value, then `max(...)`
    dedups across multiple promsql-exporter pods if there's more than
    one. `or vector(0)` keeps the panel showing 0 (not "No data") when
    no series exist for the metric in this env (e.g., an env with no
    sinks).
    """
    return query_group(
        promql_query(
            textwrap.dedent(
                f"""
                max(
                    sum by (instance) ({metric_name}{{$environmentFilter}})
                ) or vector(0)
                """
            )
        ).legend_format(metric_name),
    )


class StorageObjectsTab:
    """Storage Objects tab on Overview Dashboard."""

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _active_sources_panel(self):
        """Active sources count (env-scoped, multi-label-dedup)."""
        panel_id = "storage-active-sources"
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Sources")
            .description(f"Sources in the catalog. {ENV_SCOPED_NOTE}")
            .data(_env_total_count_query("v2_mz_sources_count"))
            .visualization(visualization.sparkline_stat(shade=STORAGE_THEME).min(0)),
        )
        return panel_id

    def _active_sinks_panel(self):
        """Active sinks count (env-scoped, multi-label-dedup)."""
        panel_id = "storage-active-sinks"
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Sinks")
            .description(f"Sinks in the catalog. {ENV_SCOPED_NOTE}")
            .data(_env_total_count_query("v2_mz_sinks_count"))
            .visualization(visualization.sparkline_stat(shade=STORAGE_THEME).min(0)),
        )
        return panel_id

    def _active_tables_panel(self):
        """Active tables count (env-scoped)."""
        panel_id = "storage-active-tables"
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Tables")
            .description(f"Tables in the catalog. {ENV_SCOPED_NOTE}")
            .data(_env_total_count_query("v2_mz_tables_count"))
            .visualization(visualization.sparkline_stat(shade=STORAGE_THEME).min(0)),
        )
        return panel_id

    def _source_types_panel(self):
        """Donut: source distribution by `type` (kafka / postgres / mysql / ...).

        `v2_mz_sources_count` is env-scoped pre-calc — no cluster filter.
        """
        panel_id = "sources-types"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (type) (
                        v2_mz_sources_count{$environmentFilter}
                    ) > 0
                    """
                )
            )
            .legend_format("{{type}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Source Types")
            .description(f"Sources broken down by source type. {ENV_SCOPED_NOTE}")
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(STORAGE_THEME)
                )
                .no_value(visualization.NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _source_status_table_panel(self):
        """Table of named sources with their current status.

        `v2_mz_source_status` is an info-style marker metric (value always
        1); the useful information is in the labels — `source_name`,
        `source_type`, `status`, `connection_type`, `source_id`.
        labelsToFields + organize promotes those labels to table columns.
        """
        panel_id = "sources-status-table"
        columns = [
            "source_name",
            "source_type",
            "status",
            "connection_type",
            "source_id",
        ]
        query = (
            query_group(
                promql_query(
                    textwrap.dedent(
                        """
                        v2_mz_source_status{$environmentFilter}
                        """
                    )
                ).instant()
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("labelsToFields")
                .id("labelsToFields")
                .options({"keepLabels": columns})
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("merge")
                .id("merge")
                .options({})
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("organize")
                .id("organize")
                .options(
                    {
                        "excludeByName": {
                            "Time": True,
                            "Value": True,
                            "v2_mz_source_status": True,
                        },
                        "renameByName": {
                            "source_name": "Source Name",
                            "source_type": "Type",
                            "status": "Status",
                            "connection_type": "Connection",
                            "source_id": "Source ID",
                        },
                        "indexByName": {
                            column: idx for idx, column in enumerate(columns)
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
                        "sort": [{"field": "Source Name"}],
                    }
                )
            )
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Sources by Status")
            .description(
                "Named sources with their current status. Useful for spotting "
                "stalled or failing sources at a glance."
            )
            .data(query)
            .visualization(
                table.Visualization()
                .show_header(True)
                .filterable(True)
                .no_value(visualization.NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _source_bytes_received_panel(self):
        """Timeseries: per-source bytes received rate, log Y-axis.

        `mz_source_bytes_received` reports per-(source_id, replica, worker)
        where `source_id` is the *subsource* ID for source types that have
        them (e.g. Postgres sources expose one bytes_received series per
        replicated table, each rolling up to a single `parent_source_id`).
        Aggregating by `parent_source_id` gives one series per *primary*
        source, which is what users actually think of.

        Two-query outer-join pattern for friendly names:

        1. **Named branch** — `parent_source_id` joined to `source_id` from
           `v2_mz_source_status` via `label_replace`, pulling `source_name`
           in via `group_left`. Legend uses `{{source_name}}`.
        2. **Orphan branch** — same aggregate `unless on (parent_source_id)`
           the join's right-hand side. Catches primary sources that don't
           appear in `v2_mz_source_status` (which silently happens for some
           source types in some envs). Legend falls back to
           `{{parent_source_id}}`.

        Both branches apply `> 0` so idle sources don't clutter the chart.
        Log Y-axis because real workloads span kB/s to tens of MB/s, and
        linear scale flattens the smaller sources against the X-axis.
        """
        panel_id = "sources-bytes-received-rate"
        # The status metric is env-scoped; we re-label `source_id` →
        # `parent_source_id` so it can join against the bytes-received
        # aggregate (which is per primary).
        status_with_parent_label = (
            "label_replace("
            "avg by (source_id, source_name) ("
            "v2_mz_source_status{$environmentFilter}"
            "),"
            ' "parent_source_id", "$1", "source_id", "(.*)"'
            ")"
        )

        named_query = promql_query(
            textwrap.dedent(
                f"""
                (
                    sum by (parent_source_id) (
                        rate(mz_source_bytes_received{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    * on (parent_source_id) group_left (source_name)
                    {status_with_parent_label}
                ) > 0
                """
            )
        ).legend_format("{{source_name}}")

        orphan_query = promql_query(
            textwrap.dedent(
                f"""
                (
                    sum by (parent_source_id) (
                        rate(mz_source_bytes_received{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    unless on (parent_source_id)
                    {status_with_parent_label}
                ) > 0
                """
            )
        ).legend_format("{{parent_source_id}}")

        query = query_group(named_query, orphan_query)

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Source Bytes Received (rate)")
            .description(
                "Bytes per second received per primary source (subsources "
                "aggregated). Series with a matching v2_mz_source_status "
                "entry show as source_name; the rest fall back to "
                "parent_source_id. Log Y-axis."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("Bps")
                .scale_distribution(
                    common_builder.ScaleDistributionConfig()
                    .type(common.ScaleDistribution.LOG)
                    .log(10)
                )
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def build_sources_row(self) -> dashboardv2_builders.Row:
        """Sources row: type donut + status table + bytes-received rate."""
        return (
            dashboardv2_builders.Row()
            .title("Sources")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .column_width_mode("wide")
                .with_item(self._source_types_panel())
                .with_item(self._source_status_table_panel())
                .with_item(self._source_bytes_received_panel())
            )
        )

    # ---- Sinks: universal ----

    def _sink_types_panel(self):
        """Donut: sinks broken down by (type, envelope_type).

        Kafka has both `debezium` and `upsert` envelopes in practice;
        Iceberg is upsert-only as of writing. Keying on the pair
        surfaces that mix instead of collapsing it.

        `v2_mz_sinks_count` is env-scoped — no cluster filter.
        """
        panel_id = "sinks-types"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (type, envelope_type) (
                        v2_mz_sinks_count{$environmentFilter}
                    ) > 0
                    """
                )
            )
            .legend_format("{{type}} / {{envelope_type}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Sink Types")
            .description(
                f"Sinks broken down by (type, envelope_type). {ENV_SCOPED_NOTE}"
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(STORAGE_THEME)
                )
                .no_value(visualization.NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _sink_throughput_panel(self):
        """Timeseries: per-sink bytes-committed rate (log Y-axis).

        Unlike sources, sinks have no `v2_mz_sink_status` analog — no
        friendly name is available. Legend is `sink_id`.
        """
        panel_id = "sinks-throughput"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_bytes_committed{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    ) > 0
                    """
                )
            ).legend_format("{{sink_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Sink Throughput (committed)")
            .description(
                "Bytes per second committed by each sink. There's no "
                "v2_mz_sink_status equivalent of source_status, so the "
                "legend is sink_id. Log Y-axis."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("Bps")
                .scale_distribution(
                    common_builder.ScaleDistributionConfig()
                    .type(common.ScaleDistribution.LOG)
                    .log(10)
                )
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _sink_lag_panel(self):
        """Timeseries: per-sink lag in bytes (staged minus committed).

        Both metrics are counters; their difference is "bytes that have
        been prepared but not yet acknowledged downstream" at the moment
        of scrape. Brief negative values can occur from scrape skew
        (committed updates between staged and committed reads), hence
        `clamp_min(..., 0)`.
        """
        panel_id = "sinks-lag"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    clamp_min(
                        sum by (sink_id) (mz_sink_bytes_staged{{{_COMPUTE_FILTER}}})
                        - sum by (sink_id) (mz_sink_bytes_committed{{{_COMPUTE_FILTER}}}),
                        0
                    )
                    """
                )
            ).legend_format("{{sink_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Sink Lag (staged minus committed)")
            .description(
                "Bytes staged but not yet committed per sink. Persistent "
                "growth signals downstream backpressure or recurring "
                "commit failures."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("bytes")
                .min(0)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def build_sinks_row(self) -> dashboardv2_builders.Row:
        """Sinks row: type/envelope donut + throughput + lag (always visible)."""
        return (
            dashboardv2_builders.Row()
            .title("Sinks")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .column_width_mode("wide")
                .with_item(self._sink_types_panel())
                .with_item(self._sink_throughput_panel())
                .with_item(self._sink_lag_panel())
            )
        )

    # ---- Iceberg-specific sinks ----

    def _iceberg_commit_latency_panel(self):
        """Histogram quantile p50/p90/p99 of iceberg commit duration."""
        panel_id = "sinks-iceberg-commit-latency"

        def quantile(p: float, label: str):
            return promql_query(
                textwrap.dedent(
                    f"""
                    histogram_quantile({p},
                        sum by (le) (
                            rate(mz_sink_iceberg_commit_duration_seconds_bucket{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
                    )
                    """
                )
            ).legend_format(label)

        query = query_group(
            quantile(0.50, "p50"),
            quantile(0.90, "p90"),
            quantile(0.99, "p99"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Iceberg Commit Latency (p50 / p90 / p99)")
            .description(
                "Iceberg snapshot commit duration percentiles across the "
                "env. Log Y-axis because commits range from sub-second to "
                "multi-second."
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

    def _iceberg_failures_panel(self):
        """Iceberg commit failures + conflicts rate (threshold-colored)."""
        panel_id = "sinks-iceberg-failures"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_iceberg_commit_failures{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} failures"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_iceberg_commit_conflicts{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} conflicts"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Iceberg Commit Failures & Conflicts")
            .description(
                "Rate of commit failures and conflicts per sink. "
                "Conflicts are usually concurrent-writer races; failures "
                "are commit-side errors. Non-zero is bad."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("cps")
                .min(0)
                .thresholds(threshold.error_thresholds(max_errors=10))
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _iceberg_files_panel(self):
        """Iceberg data/delete files written + snapshots committed rate."""
        panel_id = "sinks-iceberg-files"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_iceberg_data_files_written{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} data"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_iceberg_delete_files_written{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} deletes"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_iceberg_snapshots_committed{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} snapshots"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Iceberg File & Snapshot Rate")
            .description(
                "Rate of data files, delete files, and snapshots per sink. "
                "Their proportions tell you about commit batching and "
                "upsert behavior."
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

    def build_iceberg_sinks_row(self) -> dashboardv2_builders.Row:
        """Iceberg-specific sink panels (collapsed by default)."""
        return (
            dashboardv2_builders.Row()
            .title("Iceberg Sinks")
            .hide_header(False)
            .collapse(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .column_width_mode("wide")
                .with_item(self._iceberg_commit_latency_panel())
                .with_item(self._iceberg_failures_panel())
                .with_item(self._iceberg_files_panel())
            )
        )

    # ---- Kafka-specific sinks ----

    def _kafka_tx_errors_panel(self):
        """Kafka rdkafka TX error rate per sink (threshold-colored)."""
        panel_id = "sinks-kafka-tx-errors"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_rdkafka_txerrs{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Kafka TX Error Rate")
            .description(
                "rdkafka TX error rate per sink. Non-zero indicates "
                "publishing failures against the broker."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("cps")
                .min(0)
                .thresholds(threshold.error_thresholds(max_errors=10))
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _kafka_outbuf_panel(self):
        """Kafka rdkafka outgoing message buffer per sink."""
        panel_id = "sinks-kafka-outbuf"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        mz_sink_rdkafka_outbuf_msg_cnt{{{_COMPUTE_FILTER}}}
                    )
                    """
                )
            ).legend_format("{{sink_id}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Kafka Output Buffer (messages)")
            .description(
                "Messages currently buffered in rdkafka waiting for "
                "transmission. Sustained high values indicate broker-side "
                "back-pressure."
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

    def _kafka_connects_panel(self):
        """Kafka connect & disconnect event rates per sink."""
        panel_id = "sinks-kafka-connects"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_rdkafka_connects{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} connects"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        rate(mz_sink_rdkafka_disconnects{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                    """
                )
            ).legend_format("{{sink_id}} disconnects"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Kafka Connect / Disconnect Rate")
            .description(
                "Connect and disconnect events per sink. A persistently "
                "high disconnect rate is a sign of unhealthy broker "
                "connectivity."
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

    def build_kafka_sinks_row(self) -> dashboardv2_builders.Row:
        """Kafka-specific sink panels (collapsed by default)."""
        return (
            dashboardv2_builders.Row()
            .title("Kafka Sinks")
            .hide_header(False)
            .collapse(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(3)
                .column_width_mode("wide")
                .with_item(self._kafka_tx_errors_panel())
                .with_item(self._kafka_outbuf_panel())
                .with_item(self._kafka_connects_panel())
            )
        )

    def build_summary_row(self) -> dashboardv2_builders.Row:
        """Summary row: source / sink / table counts."""
        return (
            dashboardv2_builders.Row()
            .title("Storage Objects Summary")
            .hide_header(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .column_width_mode("narrow")
                .max_column_count(3)
                .with_item(self._active_sources_panel())
                .with_item(self._active_sinks_panel())
                .with_item(self._active_tables_panel())
            )
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate the Storage Objects tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Storage Objects")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_summary_row())
                .row(self.build_sources_row())
                .row(self.build_sinks_row())
                .row(self.build_iceberg_sinks_row())
                .row(self.build_kafka_sinks_row())
            )
        )
