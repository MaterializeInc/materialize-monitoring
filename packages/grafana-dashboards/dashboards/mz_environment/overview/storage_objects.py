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

from dashboards import enrich, palette, threshold, visualization

# Clusterd-side storage metrics (mz_source_*, mz_sink_*, etc.) use the
# long-form `cluster_environmentd_materialize_cloud_*` id label family — the
# same convention as the arrangement/dataflow metrics. Verified directly
# against live `mz_source_bytes_received` / `mz_sink_bytes_committed` series.
#
# NOTE: the `$mzClusterList` picker is built from `mz_compute_cluster_status`,
# which lists *compute* clusters only — a storage-only ingest cluster won't
# appear there. With the default "All" (`.*`) selection these panels show
# everything; selecting a specific compute cluster will hide storage objects
# that live on a dedicated ingest cluster.
_COMPUTE_FILTER = (
    "$environmentFilter, "
    'cluster_environmentd_materialize_cloud_cluster_id=~"$mzClusterList", '
    'cluster_environmentd_materialize_cloud_replica_id=~"$mzReplicaList"'
)

# Pre-calc storage counts are env-scoped — the metrics don't carry
# cluster_id/instance_id labels, so the dashboard cluster/replica filters
# don't change these stats. Surfaced in panel descriptions.
ENV_SCOPED_NOTE = "Environment-scoped — not affected by the cluster/replica filters."

# JOB DEDUP: several clusterd metrics (mz_source_*, mz_sink_*, mz_arrangement_*)
# are scraped by more than one Prometheus job hitting the same :6878 endpoint,
# so each underlying series appears N times differing only by `job`. A plain
# `sum(rate(...))` then multiplies the real value by N. Wrap the inner
# counter/gauge in `max without (job) (...)` to collapse those redundant copies
# back to one series BEFORE the outer aggregation; it is a no-op once the scrape
# config is deduped (1 job -> max of 1). `max by (...)` panels already collapse
# `job` implicitly, so they need no wrap.
#
# We deliberately do NOT filter on a job name: the authoritative job name varies
# by deployment, and on this instance several metrics live ONLY on a so-called
# "legacy" job (cluster_status, storage_objects, dataflow_elapsed, the *_count
# metrics), so excluding job names by pattern would blank real panels.

# SQL-derived metric used across this tab's count/type/catalog panels:
#   ${sqlMetricPrefix}storage_objects
#   -> mz_storage_objects (self-managed) / v2_mz_storage_objects (cloud)
# The mz_source_*/mz_sink_* throughput/lag/error metrics below are genuine
# instrumentation (same `mz_` name in both environments) and are NOT prefixed.

STORAGE_THEME = palette.THEME_PALETTE[
    4
]  # yellow — distinct from compute (orange), connections (teal), k8s (cyan)


def _env_total_count_query(metric_name: str):
    """Build a deduped env-wide count query for a pre-calc storage metric.

    Some of these metrics carry breakdown labels like `type`/`size` —
    `sum by (instance)` collapses those into a single per-scrape value, then
    `max(...)` dedups across multiple exporter pods if there's more than one.
    `or vector(0)` keeps the panel showing 0 (not "No data") when no series
    exist for the metric in this env.

    Used here for catalog counts that have no breakdown-doubling concern
    (`mz_tables_count`). For sources/sinks, prefer `_storage_object_count_query`
    — `mz_sources_count` / `mz_sinks_count` fold the hidden `<name>_progress`
    subsources into their per-connector-type counts (e.g. 3 Postgres sources
    report `type="postgres"` = 6), so they over-count actual objects.
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


def _storage_object_count_query(object_kind: str):
    """Count distinct source/sink objects from `${sqlMetricPrefix}storage_objects`.

    Metric: `mz_storage_objects` (self-managed) / `v2_mz_storage_objects` (cloud).

    `mz_storage_objects` emits one series per (object, replica) with a `type`
    label of `source` / `sink`, an `id`, and `object_type` / `connection_type`
    / `envelope_type` describing the connector. Crucially it does **not**
    include the hidden `<name>_progress` subsources, so
    `count(group by (id) (...))` is the true catalog count — progress-free and
    deduped across replicas. `or vector(0)` keeps the stat at 0 (not "No data")
    when none exist. Env-scoped: not filtered by the cluster picker (which only
    lists compute clusters; ingest clusters are storage-only).
    """
    return query_group(
        promql_query(
            textwrap.dedent(
                f"""
                count(
                    group by (id) (
                        ${{sqlMetricPrefix}}storage_objects{{$environmentFilter, type="{object_kind}"}}
                    )
                ) or vector(0)
                """
            )
        ).legend_format(f"{object_kind}s"),
    )


class StorageObjectsTab:
    """Storage Objects tab on Overview Dashboard."""

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _active_sources_panel(self):
        """Active sources count from mz_storage_objects (progress-free)."""
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
                "is maintaining. Counts distinct source objects (the hidden "
                "per-source `_progress` subsources are excluded), so it "
                "matches what you'd see in `mz_sources`. See _Sources_ row "
                "below for type breakdown and per-source throughput. "
                f"{ENV_SCOPED_NOTE}"
            )
            .data(_storage_object_count_query("source"))
            .visualization(visualization.sparkline_stat(shade=STORAGE_THEME).min(0)),
        )
        return panel_id

    def _active_sinks_panel(self):
        """Active sinks count from mz_storage_objects (progress-free)."""
        panel_id = "storage-active-sinks"
        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Active Sinks")
            .description(
                "**Number of active sinks in the catalog.** Each sink is "
                "an outbound feed (Kafka, Iceberg, etc.) that emits the "
                "results of a materialized view or query to an external "
                "system. Counts distinct sink objects (excluding hidden "
                "`_progress` subsources), matching `mz_sinks`. See _Sinks_ "
                "row below for per-sink throughput and lag. "
                f"{ENV_SCOPED_NOTE}"
            )
            .data(_storage_object_count_query("sink"))
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
            # mz_tables_count / v2_mz_tables_count
            .data(_env_total_count_query("${sqlMetricPrefix}tables_count"))
            .visualization(visualization.sparkline_stat(shade=STORAGE_THEME).min(0)),
        )
        return panel_id

    def _source_types_panel(self):
        """Donut: source distribution by connector type (kafka / postgres / ...).

        Counts distinct source `id`s per `object_type` from
        `mz_storage_objects` — `group by (id, object_type)` first dedups the
        per-replica series, then `count by (object_type)` gives one slice per
        connector type. Progress subsources are not present in this metric, so
        the totals match `mz_sources`. Env-scoped.
        """
        panel_id = "sources-types"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    count by (object_type) (
                        group by (id, object_type) (
                            ${sqlMetricPrefix}storage_objects{$environmentFilter, type="source"}
                        )
                    ) > 0
                    """
                )
            )
            .legend_format("{{object_type}}")
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
        """Catalog table of sources from `mz_storage_objects`.

        Self-managed exposes no source *status* metric (the cloud-only
        `v2_mz_source_status` has no equivalent; running/stalled/errored lives
        in `mz_internal.mz_source_statuses` in SQL). `mz_storage_objects` is
        the closest metric-side catalog: one series per (object, replica), with
        `id`, `object_type`, `connection_type`, `envelope_type`, `cluster_id`.
        `group by (...)` collapses the per-replica duplicates to one row per
        source; labelsToFields + organize promote the labels to columns.
        """
        panel_id = "sources-status-table"
        columns = [
            "id",
            "object_type",
            "connection_type",
            "envelope_type",
            "cluster_id",
        ]
        query = (
            query_group(
                promql_query(
                    textwrap.dedent(
                        """
                        group by (id, object_type, connection_type, envelope_type, cluster_id) (
                            ${sqlMetricPrefix}storage_objects{$environmentFilter, type="source"}
                        )
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
                        },
                        "renameByName": {
                            "id": "Source ID",
                            "object_type": "Type",
                            "connection_type": "Connection",
                            "envelope_type": "Envelope",
                            "cluster_id": "Cluster",
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
                        "sort": [{"field": "Source ID"}],
                    }
                )
            )
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Sources")
            .description(
                "**Catalog of sources running in the environment — one row "
                "per source with its connector type, envelope, and the "
                "cluster it ingests on.** Self-managed Materialize exposes no "
                "source *status* metric, so running/stalled/errored isn't "
                "shown here — check live status with `SELECT name, type, "
                "status FROM mz_internal.mz_source_statuses;`, and use _Source "
                "Bytes Received (rate)_ to confirm a source is actively "
                "ingesting. Translate `Source ID` to a name via `SELECT id, "
                "name FROM mz_sources`. The hidden `_progress` subsources are "
                "excluded, so the row count matches _Active Sources_."
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

        TODO(self-managed): the cloud-only `v2_mz_source_status` metric (used
        to enrich `parent_source_id` with a friendly `source_name`) has no
        self-managed equivalent, so the friendly-name outer-join is dropped
        here and the legend is `parent_source_id`. Translate ids via `SELECT
        id, name FROM mz_sources`. The underlying `mz_source_bytes_received`
        metric itself is clusterd-side and DOES exist on self-managed, but the
        test env has no sources so this could not be verified live.

        `> 0` so idle sources don't clutter the chart. Log Y-axis because real
        workloads span kB/s to tens of MB/s, and linear scale flattens the
        smaller sources against the X-axis.
        """
        panel_id = "sources-bytes-received-rate"

        # mz_source_bytes_received (genuine); name resolved via mz_object_info
        bytes_expr = textwrap.dedent(
            f"""
            sum by (parent_source_id) (
                max without (job) (
                    rate(mz_source_bytes_received{{{_COMPUTE_FILTER}}}[$__rate_interval])
                )
            ) > 0
            """
        )
        query = query_group(
            promql_query(
                enrich.with_object_name(bytes_expr, "parent_source_id")
            ).legend_format("{{name}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Source Bytes Received (rate)")
            .description(
                "**Inbound throughput per primary source — bytes per "
                "second pulled from upstream.** Subsources (e.g., "
                "per-table Postgres replication subsources) are "
                "aggregated up to their primary, so each line represents "
                "one logical source, labeled by source name (resolved via "
                "`mz_object_info`). Idle sources are filtered out (`> 0`). "
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

    def _source_ingestion_by_replica_panel(self):
        """Timeseries: source message-ingest rate split per replica.

        Each replica of a multi-replica cluster reads its upstream
        independently, so for a given source the replicas should track each
        other closely. **A replica flat at 0 while its siblings ingest means
        that replica lost its upstream connection** — the classic "restarted a
        replica and it couldn't resume pulling from Kafka" failure. The source
        still reports `Running` (other replicas are fine) and the aggregate
        _Source Bytes Received_ panel hides it (the healthy replica's volume
        masks the dead one), so this per-replica split is the only place on the
        metrics side it shows up — analogous to the per-worker dataflow panel.

        `mz_source_offset_commit_failures` does NOT catch this: a replica that
        silently stops pulling isn't failing to *commit*, so _Source
        Commit-Failure Rate_ stays 0. Watch instead for one replica's line
        dropping to zero here, with _Compute -> Freshness_ frontier lag
        climbing in parallel.
        """
        panel_id = "sources-ingestion-by-replica"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (parent_source_id, cluster_environmentd_materialize_cloud_replica_id) (
                        max without (job) (
                            rate(mz_source_messages_received{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
                    )
                    """
                )
            ).legend_format(
                "{{parent_source_id}} / r{{cluster_environmentd_materialize_cloud_replica_id}}"
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Source Ingestion by Replica")
            .description(
                "**Messages ingested per second, split per source and "
                "replica.** Replicas read their upstream independently and "
                "should track together. **A replica flat at 0 while its "
                "siblings keep ingesting has lost its upstream connection** "
                "(e.g. it was restarted and couldn't resume pulling from "
                "Kafka) — the source still shows `Running` overall and the "
                "aggregate _Source Bytes Received_ hides it, so this split is "
                "where it surfaces. Legends are ids; map with `SELECT id, name "
                "FROM mz_sources` and the replica via `SELECT id, name FROM "
                "mz_cluster_replicas`. When you see a replica drop out, "
                "_Compute -> Freshness_ frontier lag will be climbing too; "
                "restarting that replica usually clears the stale connection."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("cps")
                .min(0)
                .legend(visualization.TS_LEGEND_BUILDER)
                .no_value(visualization.NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def _source_errors_panel(self):
        """Timeseries: per-source upstream health — commit failures + disconnects.

        Two complementary "this source can't deal with its upstream" signals,
        both nominal at 0 so any lift off the floor is a problem:

        1. **Commit-failure rate** — `rate(mz_source_offset_commit_failures)`.
           Non-zero when the upstream is reachable but *rejects* the offset /
           replication-slot commit (auth/ACL change, broker rejecting). This
           does NOT fire for an unreachable broker — the source never gets to
           the commit step (see #2).
        2. **Disconnected indicator (0/1)** — `offset_committed > offset_known`.
           Normally `known >= committed`; when the broker/DB is unreachable the
           source can't fetch metadata so `offset_known` collapses below
           `offset_committed` (a `BrokerTransportFailure`-class stall). This
           catches the common "broker down / security group cut / DNS" case
           that the commit-failure counter misses — verified against a stalled
           Kafka source (`offset_known` -> 0) vs healthy Postgres sources (0).

        Both use `_COMPUTE_FILTER` + `max without (job)` for job-dedup. The 0/1
        disconnect uses source-level `max` of each offset, so it's a coarse
        per-source flag, not a per-partition measure.
        """
        panel_id = "sources-errors"
        commit_failures = promql_query(
            textwrap.dedent(
                f"""
                sum by (source_id) (
                    max without (job) (
                        rate(mz_source_offset_commit_failures{{{_COMPUTE_FILTER}}}[$__rate_interval])
                    )
                ) > 0
                """
            )
        ).legend_format("{{source_id}} commit failures")

        disconnected = promql_query(
            textwrap.dedent(
                f"""
                (
                    max by (source_id) (
                        max without (job) (mz_source_offset_committed{{{_COMPUTE_FILTER}}})
                    ) > bool max by (source_id) (
                        max without (job) (mz_source_offset_known{{{_COMPUTE_FILTER}}})
                    )
                ) > 0
                """
            )
        ).legend_format("{{source_id}} disconnected")

        query = query_group(commit_failures, disconnected)

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Source Upstream Errors")
            .description(
                "**Per-source upstream health — healthy sources are filtered "
                "out (`> 0`), so this panel is empty when all is well and "
                "any series at all means a source needs attention.** Two "
                "signals: "
                "**commit failures** (`mz_source_offset_commit_failures` rate) "
                "fire when the upstream is reachable but rejects the offset / "
                "replication-slot commit (auth/ACL, broker rejecting); the "
                "**disconnected** indicator flips to **1** when the source has "
                "lost sight of its upstream (`offset_known` fell below "
                "`offset_committed`) — the broker/DB-unreachable case "
                "(`BrokerTransportFailure`, severed security group, DNS) that "
                "the commit-failure counter can't catch because the source "
                "never reaches the commit step. When `disconnected` is 1, "
                "_Source Bytes Received_ flat-lines and _Compute -> Freshness_ "
                "frontier lag climbs. Legend is `source_id` — name it with "
                "`SELECT id, name FROM mz_sources` and read the exact error "
                "via `SELECT name, status, error FROM "
                "mz_internal.mz_source_statuses WHERE status != 'running'`. "
                "(Data-decode errors are separate: `mz_source_error_inserts`.)"
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("short")
                .min(0)
                .thresholds(threshold.error_thresholds(max_errors=1))
                .legend(visualization.TS_LEGEND_BUILDER)
                .no_value(visualization.NO_FILTER_MATCH)
            ),
        )
        return panel_id

    def build_sources_row(self) -> dashboardv2_builders.Row:
        """Sources row: type donut + catalog + bytes + per-replica ingest + errors."""
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
                .with_item(self._source_ingestion_by_replica_panel())
                .with_item(self._source_errors_panel())
            )
        )

    # ---- Sinks: universal ----

    def _sink_types_panel(self):
        """Donut: sinks broken down by (connector type, envelope_type).

        Kafka has both `debezium` and `upsert` envelopes in practice;
        Iceberg is upsert-only as of writing. Keying on the pair
        surfaces that mix instead of collapsing it.

        Counts distinct sink `id`s per `(object_type, envelope_type)` from
        `mz_storage_objects` — `group by (id, ...)` first dedups the per-replica
        series. Env-scoped.
        """
        panel_id = "sinks-types"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    count by (object_type, envelope_type) (
                        group by (id, object_type, envelope_type) (
                            ${sqlMetricPrefix}storage_objects{$environmentFilter, type="sink"}
                        )
                    ) > 0
                    """
                )
            )
            .legend_format("{{object_type}} / {{envelope_type}}")
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
        # mz_sink_bytes_committed (genuine); name resolved via mz_object_info
        throughput_expr = textwrap.dedent(
            f"""
            sum by (sink_id) (
                max without (job) (
                    rate(mz_sink_bytes_committed{{{_COMPUTE_FILTER}}}[$__rate_interval])
                )
            ) > 0
            """
        )
        query = query_group(
            promql_query(
                enrich.with_object_name(throughput_expr, "sink_id")
            ).legend_format("{{name}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Sink Throughput (committed)")
            .description(
                "**Outbound throughput per sink — bytes per second "
                "successfully committed to the downstream system** "
                "(Kafka broker, Iceberg catalog, etc.). Log Y-axis so "
                "low- and high-volume sinks share the chart. Labeled by "
                "sink name (resolved via `mz_object_info`). "
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
        # mz_sink_bytes_staged/_committed (genuine); name resolved via mz_object_info
        lag_expr = textwrap.dedent(
            f"""
            clamp_min(
                sum by (sink_id) (max without (job) (mz_sink_bytes_staged{{{_COMPUTE_FILTER}}}))
                - sum by (sink_id) (max without (job) (mz_sink_bytes_committed{{{_COMPUTE_FILTER}}})),
                0
            )
            """
        )
        query = query_group(
            promql_query(enrich.with_object_name(lag_expr, "sink_id")).legend_format(
                "{{name}}"
            ),
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
                            max without (job) (
                                rate(mz_sink_iceberg_commit_duration_seconds_bucket{{{_COMPUTE_FILTER}}}[$__rate_interval])
                            )
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
                        max without (job) (
                            rate(mz_sink_iceberg_commit_failures{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
                    )
                    """
                )
            ).legend_format("{{sink_id}} failures"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        max without (job) (
                            rate(mz_sink_iceberg_commit_conflicts{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
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
                        max without (job) (
                            rate(mz_sink_iceberg_data_files_written{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
                    )
                    """
                )
            ).legend_format("{{sink_id}} data"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        max without (job) (
                            rate(mz_sink_iceberg_delete_files_written{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
                    )
                    """
                )
            ).legend_format("{{sink_id}} deletes"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        max without (job) (
                            rate(mz_sink_iceberg_snapshots_committed{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
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
                        max without (job) (
                            rate(mz_sink_rdkafka_txerrs{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
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
                        max without (job) (
                            mz_sink_rdkafka_outbuf_msg_cnt{{{_COMPUTE_FILTER}}}
                        )
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
                        max without (job) (
                            rate(mz_sink_rdkafka_connects{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
                    )
                    """
                )
            ).legend_format("{{sink_id}} connects"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (sink_id) (
                        max without (job) (
                            rate(mz_sink_rdkafka_disconnects{{{_COMPUTE_FILTER}}}[$__rate_interval])
                        )
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
            .title("Sources and Sinks")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_summary_row())
                .row(self.build_sources_row())
                .row(self.build_sinks_row())
                .row(self.build_iceberg_sinks_row())
                .row(self.build_kafka_sinks_row())
            )
        )
