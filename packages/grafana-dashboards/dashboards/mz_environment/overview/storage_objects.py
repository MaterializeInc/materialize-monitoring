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
            .description(
                "**Number of active sources in the catalog.** Each source "
                "is a continuous ingestion connection from an external "
                "system (Kafka, Postgres, MySQL, S3, etc.) — so this count "
                "is roughly the number of upstream feeds the environment "
                "is maintaining. See _Sources_ row below for type "
                "breakdown and per-source throughput. "
                f"{ENV_SCOPED_NOTE}"
            )
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
            .description(
                "**Number of active sinks in the catalog.** Each sink is "
                "an outbound feed (Kafka, Iceberg, etc.) that emits the "
                "results of a materialized view or query to an external "
                "system. See _Sinks_ row below for per-sink throughput "
                "and lag. "
                f"{ENV_SCOPED_NOTE}"
            )
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
            .description(
                "**Number of user-created tables in the catalog.** Tables "
                "in Materialize are write-once-read-many; `INSERT`s feed "
                "dataflows downstream. Mostly a catalog-shape signal — "
                "for actual ingest activity see _Sources -> Source Bytes "
                "Received_. "
                f"{ENV_SCOPED_NOTE}"
            )
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
            .description(
                "**Sources broken down by source type** (kafka / postgres "
                "/ mysql / etc.). Tells you what flavors of upstream feed "
                "make up your ingest workload. Most environments "
                "concentrate on one or two types. "
                f"{ENV_SCOPED_NOTE}"
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
                "**Per-source status table with `source_name`, type, "
                "current status, and connection info.** Status `running` "
                "is the steady state; `stalled` means the source is "
                "paused (often during catchup or after an error); "
                "`errored` indicates a hard failure. `stalled` isn't "
                "always bad — Postgres sources stall briefly during "
                "their initial snapshot, for example. For active "
                "throughput see _Source Bytes Received (rate)_; for "
                "richer source metadata use `SELECT * FROM mz_sources;`."
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
                "**Inbound throughput per primary source — bytes per "
                "second pulled from upstream.** Subsources (e.g., "
                "per-table Postgres replication subsources) are "
                "aggregated up to their primary, so each line represents "
                "one logical source. Series with a match in "
                "`v2_mz_source_status` show as `source_name`; sources "
                "missing from the status metric fall back to "
                "`parent_source_id` (a metric-side gap that happens for "
                "some source types in some envs, not a problem with the "
                "source itself). Idle sources are filtered out (`> 0`). "
                "Log Y-axis so kB/s and tens-of-MB/s sources share the "
                "chart. Scoped to the selected clusters/replicas."
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
                "**Sinks broken down by (type, envelope_type)** — e.g., "
                "`kafka / upsert`, `kafka / debezium`, `iceberg / "
                "upsert`. The envelope determines how Materialize "
                "encodes changes: `upsert` writes the latest value per "
                "key, `debezium` writes change events with old+new "
                "values. Most envs concentrate on one combination. "
                f"{ENV_SCOPED_NOTE}"
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
                "**Outbound throughput per sink — bytes per second "
                "successfully committed to the downstream system** "
                "(Kafka broker, Iceberg catalog, etc.). Log Y-axis so "
                "low- and high-volume sinks share the chart. Unlike "
                "sources, there's no `v2_mz_sink_status` metric — the "
                "legend uses `sink_id` rather than a friendly name; "
                "look the id up via `SELECT id, name FROM mz_sinks;`. "
                "Idle sinks are filtered out (`> 0`). Scoped to the "
                "selected clusters/replicas."
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
                "**Bytes staged for a sink but not yet committed "
                "downstream — a queue depth in bytes.** Both metrics "
                "are counters; their difference at any moment is the "
                "in-flight write that's been prepared but not yet "
                "acknowledged by the downstream system. Nominal: "
                "oscillates around a small value as commits happen "
                "periodically. **Sustained growth means the sink can't "
                "keep up** — usually downstream back-pressure (broker "
                "overloaded, Iceberg catalog slow) or repeated commit "
                "failures (see _Iceberg Commit Failures & Conflicts_ "
                "or _Kafka TX Error Rate_ in the collapsed rows below). "
                "Scoped to the selected clusters/replicas."
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
                "**Iceberg commit duration percentiles** — how long each "
                "`COMMIT` against the Iceberg catalog takes. Iceberg "
                "writes are batched and committed periodically; the "
                "commit involves writing a snapshot manifest and asking "
                "the catalog to atomically swap it in. Nominal: p50 "
                "sub-second to low seconds; p99 a few seconds even on "
                "healthy systems. Sustained p99 in tens of seconds "
                "points at a slow Iceberg catalog (REST catalog under "
                "load, Glue API throttling) — _Sink Lag_ will be "
                "growing at the same time. Log Y-axis. Scoped to the "
                "selected clusters/replicas."
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
                "**Per-sink rate of failed and conflicting Iceberg "
                "commits.** Conflicts (concurrent-writer races on the "
                "Iceberg snapshot pointer) are recoverable — Materialize "
                "retries — but a high rate signals that something else "
                "is writing to the same Iceberg table. Failures are "
                "commit-side errors (network, auth, schema). **Non-zero "
                "in either dimension is worth investigating.** If "
                "failures are climbing, _Sink Lag_ will follow. The "
                'Errors threshold-coloring is calibrated for "any '
                'non-zero is interesting".'
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
                "**Per-sink rate of files and snapshots written to "
                "Iceberg.** Each commit produces one snapshot containing "
                "some data files (new rows) and delete files (tombstones "
                "for upserts). The data:delete file ratio tells you "
                "about your workload: pure-insert sinks produce ~0 "
                "deletes; upsert-heavy workloads produce roughly 1:1. "
                "Sustained delete-file rate without data files means "
                "the sink is mostly deleting (data evaporating "
                "upstream). Scoped to the selected clusters/replicas."
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
                "**Per-sink rate of TX errors from the librdkafka "
                "client.** Each TX error is one failed produce-request "
                "against the Kafka broker. **Non-zero is a problem** — "
                "likely causes are broker outages, ACL changes, topic "
                "deletion/recreation, or partition rebalancing. If "
                "errors are sustained, _Sink Lag_ will grow and _Kafka "
                "Output Buffer_ may fill. Errors threshold-coloring is "
                'calibrated for "any non-zero is interesting".'
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
                "**Messages currently sitting in the librdkafka output "
                "buffer, waiting to be sent to the broker.** Normal "
                "buffer fluctuates briefly as messages flow through; "
                "sustained high values mean Materialize is producing "
                "faster than the broker is accepting. Often paired "
                "with a non-zero _Kafka TX Error Rate_. If the buffer "
                "hits its bound, the sink stalls and _Sink Lag_ starts "
                "climbing."
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
                "**Connect and disconnect events per sink against the "
                "Kafka broker.** Healthy connections are persistent — "
                "a couple of connects at sink startup and zero "
                "disconnects afterward. **Sustained non-zero disconnect "
                "rate is a sign of unhealthy connectivity** (network "
                "flakiness, broker restarting, auth tokens expiring). "
                "Pairs with _Kafka TX Error Rate_ when the issue is "
                "broker-side rather than purely network."
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
