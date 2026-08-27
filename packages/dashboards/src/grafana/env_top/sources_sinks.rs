// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Sources and Sinks tab: what flows in, and what flows out.
//!
//! Storage metrics come from the replica's own scrape, so they carry the
//! replica-history label spelling (`cluster_environmentd_materialize_cloud_*`) and
//! are keyed by object id — `parent_source_id` for sources, `sink_id` for sinks.
//! Every panel that shows a name gets it from [`enrich`]; without that the legends
//! read `u42`.
//!
//! `max without (job)` appears throughout, before any aggregation. A replica
//! scraped by two Prometheus jobs reports the same counter twice, and summing
//! would double every rate on this tab.
//!
//! The two connector-specific rows are collapsed by default: most environments run
//! one of Kafka or Iceberg, so the other is noise until you go looking.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::query::{PromQuery, query_group};
use mzmon_lib::grafana::threshold;
use mzmon_lib::query::enrich;

use super::{selector, theme, transform};

/// The tab's theme.
const SHADE: &str = theme::SOURCES_SINKS.shade;

/// The catalog metric describing every storage object.
const STORAGE_OBJECTS: &str = "mz_storage_objects";

pub fn rows() -> Vec<Row> {
    vec![summary(), sources(), sinks(), iceberg(), kafka()]
}

fn summary() -> Row {
    Row::new("Storage Objects Summary").hide_header().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("storage-active-sources", active_sources())
            .panel("storage-active-sinks", active_sinks())
            .panel("storage-active-tables", active_tables()),
    )
}

fn sources() -> Row {
    Row::new("Sources").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sources-types", source_types())
            .panel("sources-status-table", source_catalog())
            .panel("sources-bytes-received-rate", source_bytes_received())
            .panel("sources-ingestion-by-replica", ingestion_by_replica())
            .panel("sources-errors", source_errors()),
    )
}

fn sinks() -> Row {
    Row::new("Sinks").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sinks-types", sink_types())
            .panel("sinks-throughput", sink_throughput())
            .panel("sinks-lag", sink_lag()),
    )
}

fn iceberg() -> Row {
    // Collapsed: an environment with no Iceberg sinks would otherwise pay for
    // three empty panels on every load.
    Row::new("Iceberg Sinks").collapsed().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sinks-iceberg-commit-latency", iceberg_commit_latency())
            .panel("sinks-iceberg-failures", iceberg_failures())
            .panel("sinks-iceberg-files", iceberg_files()),
    )
}

fn kafka() -> Row {
    Row::new("Kafka Sinks").collapsed().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sinks-kafka-tx-errors", kafka_tx_errors())
            .panel("sinks-kafka-outbuf", kafka_output_buffer())
            .panel("sinks-kafka-connects", kafka_connects()),
    )
}

/// Environment scope, for the catalog panels.
fn env() -> String {
    selector::environment()
}

/// The replica-history scope the storage metrics carry.
fn scope() -> String {
    format!(
        "{env}, {cluster}, {replica}",
        env = env(),
        cluster = selector::history_cluster(),
        replica = selector::history_replica()
    )
}

/// `count(group by (id) (…))` over one kind of storage object.
///
/// Grouping by `id` first is what excludes the hidden `_progress` subsources
/// every source and sink carries — otherwise the count is roughly double what
/// `mz_sources` shows.
fn object_count(kind: &str) -> String {
    format!(
        r#"
count(
    group by (id) (
        {STORAGE_OBJECTS}{{{env}, type="{kind}"}}
    )
) or vector(0)
"#,
        env = env()
    )
}

fn active_sources() -> dashboardv2::PanelKind {
    Panel::stat("Active Sources")
        .description(
            "**Number of active sources in the catalog.** Each source is a continuous ingestion \
             connection from an external system (Kafka, Postgres, MySQL, S3, etc.) — so this \
             count is roughly the number of upstream feeds the environment is maintaining. \
             Counts distinct source objects (the hidden per-source `_progress` subsources are \
             excluded), so it matches what you'd see in `mz_sources`. See _Sources_ row below \
             for type breakdown and per-source throughput. Environment-scoped — not affected by \
             the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(object_count("source"))
                .legend("sources")
                .build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_sinks() -> dashboardv2::PanelKind {
    Panel::stat("Active Sinks")
        .description(
            "**Number of active sinks in the catalog.** Each sink is an outbound feed (Kafka, \
             Iceberg, etc.) that emits the results of a materialized view or query to an \
             external system. Counts distinct sink objects (excluding hidden `_progress` \
             subsources), matching `mz_sinks`. See _Sinks_ row below for per-sink throughput \
             and lag. Environment-scoped — not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(object_count("sink")).legend("sinks").build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_tables() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
max(
    sum by (instance) (mz_tables_count{{{env}}})
) or vector(0)
"#,
        env = env()
    );
    Panel::stat("Active Tables")
        .description(
            "**Number of user-created tables in the catalog.** Tables in Materialize are \
             write-once-read-many; `INSERT`s feed dataflows downstream. Mostly a catalog-shape \
             signal — for actual ingest activity see _Sources -> Source Bytes Received_. \
             Environment-scoped — not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("mz_tables_count").build(),
        ]))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn source_types() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
count by (object_type) (
    group by (id, object_type) (
        {STORAGE_OBJECTS}{{{env}, type="source"}}
    )
) > 0
"#,
        env = env()
    );
    Panel::piechart("Source Types")
        .description(
            "**Sources broken down by source type** (kafka / postgres / mysql / etc.). Tells \
             you what flavors of upstream feed make up your ingest workload. Most environments \
             concentrate on one or two types. Environment-scoped — not affected by the \
             cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{object_type}}")
                .build(),
        ]))
        .shade(SHADE)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn sink_types() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
count by (object_type, envelope_type) (
    group by (id, object_type, envelope_type) (
        {STORAGE_OBJECTS}{{{env}, type="sink"}}
    )
) > 0
"#,
        env = env()
    );
    Panel::piechart("Sink Types")
        .description(
            "**Sinks broken down by (type, envelope_type)** — e.g., `kafka / upsert`, `kafka / \
             debezium`, `iceberg / upsert`. The envelope determines how Materialize encodes \
             changes: `upsert` writes the latest value per key, `debezium` writes change events \
             with old+new values. Most envs concentrate on one combination. Environment-scoped \
             — not affected by the cluster/replica filters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .instant()
                .legend("{{object_type}} / {{envelope_type}}")
                .build(),
        ]))
        .shade(SHADE)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn source_catalog() -> dashboardv2::PanelKind {
    let base = format!(
        r#"
group by (id, object_type, connection_type, envelope_type, cluster_id) (
    {STORAGE_OBJECTS}{{{env}, type="source"}}
)
"#,
        env = env()
    );
    // The catalog metric is keyed by `id`, so the name join goes through that
    // rather than a source-specific label.
    let expr = enrich::with_object_name(&base, "id", None, &env());
    const COLUMNS: &[&str] = &[
        "name",
        "id",
        "object_type",
        "connection_type",
        "envelope_type",
        "cluster_id",
    ];
    Panel::table("Sources")
        .description(
            "**Catalog of sources running in the environment — one row per source (by name) \
             with its connector type, envelope, and the cluster it ingests on.** Names are \
             resolved via `mz_object_info`. Self-managed Materialize exposes no source *status* \
             metric, so running/stalled/errored isn't shown here — check live status with \
             `SELECT name, type, status FROM mz_internal.mz_source_statuses;`, and use _Source \
             Bytes Received (rate)_ to confirm a source is actively ingesting. The hidden \
             `_progress` subsources are excluded, so the row count matches _Active Sources_.",
        )
        .data(query_group(vec![PromQuery::new(expr).instant().build()]))
        .transformations(vec![
            transform::labels_to_fields(COLUMNS),
            transform::merge(),
            transform::organize_full(
                // `Value` is the info metric's `1`, which says nothing here.
                &["Time", "Value"],
                COLUMNS,
                &[
                    ("name", "Name"),
                    ("id", "Source ID"),
                    ("object_type", "Type"),
                    ("connection_type", "Connection"),
                    ("envelope_type", "Envelope"),
                    ("cluster_id", "Cluster"),
                ],
            ),
            transform::sort_by("Name", false),
        ])
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

/// A per-object rate: `sum by (<key>) (max without (job) (rate(<metric>)))`.
///
/// `max without (job)` first: a replica scraped by two jobs reports the counter
/// twice, and summing those would double the rate.
fn object_rate(metric: &str, key: &str, extra_group: Option<&str>) -> String {
    let grouping = match extra_group {
        Some(extra) => format!("{key}, {extra}"),
        None => key.to_string(),
    };
    format!(
        r#"
sum by ({grouping}) (
    max without (job) (
        rate({metric}{{{scope}}}[$__rate_interval])
    )
)
"#,
        scope = scope()
    )
}

/// [`object_rate`], dropping idle objects so the legend stays readable.
fn active_object_rate(metric: &str, key: &str) -> String {
    format!("{} > 0", object_rate(metric, key, None).trim_end())
}

/// A per-object gauge, with the same double-scrape guard.
fn object_gauge(metric: &str, key: &str) -> String {
    format!(
        r#"
sum by ({key}) (
    max without (job) (
        {metric}{{{scope}}}
    )
)
"#,
        scope = scope()
    )
}

fn source_bytes_received() -> dashboardv2::PanelKind {
    // Grouped by `parent_source_id`, which rolls per-table subsources up into the
    // logical source the operator created.
    let base = active_object_rate("mz_source_bytes_received", "parent_source_id");
    let expr = enrich::with_object_name(&base, "parent_source_id", None, &env());
    Panel::timeseries("Source Bytes Received (rate)")
        .description(
            "**Inbound throughput per primary source — bytes per second pulled from \
             upstream.** Subsources (e.g., per-table Postgres replication subsources) are \
             aggregated up to their primary, so each line represents one logical source, \
             labeled by source name (resolved via `mz_object_info`). Idle sources are filtered \
             out (`> 0`). Log Y-axis so kB/s and tens-of-MB/s sources share the chart. Scoped \
             to the selected clusters/replicas.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{name}}").build(),
        ]))
        .unit("Bps")
        .log_scale(10.0)
        .build(0)
}

fn ingestion_by_replica() -> dashboardv2::PanelKind {
    let base = object_rate(
        "mz_source_messages_received",
        "parent_source_id",
        Some(selector::HISTORY_REPLICA_LABEL),
    );
    let expr = enrich::with_object_name(&base, "parent_source_id", None, &env());
    Panel::timeseries("Source Ingestion by Replica")
        .description(
            "**Messages ingested per second, split per source and replica.** Replicas read \
             their upstream independently and should track together. **A replica flat at 0 \
             while its siblings keep ingesting has lost its upstream connection** (e.g. it was \
             restarted and couldn't resume pulling from Kafka) — the source still shows \
             `Running` overall and the aggregate _Source Bytes Received_ hides it, so this \
             split is where it surfaces. Legends are ids; map with `SELECT id, name FROM \
             mz_sources` and the replica via `SELECT id, name FROM mz_cluster_replicas`. When \
             you see a replica drop out, _Compute Objects -> Freshness_ frontier lag will be \
             climbing too; restarting that replica usually clears the stale connection.",
        )
        .data(query_group(vec![
            PromQuery::new(expr)
                .legend(format!(
                    "{{{{name}}}} / r{{{{{replica}}}}}",
                    replica = selector::HISTORY_REPLICA_LABEL
                ))
                .build(),
        ]))
        .unit("cps")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn source_errors() -> dashboardv2::PanelKind {
    let commit_failures = active_object_rate("mz_source_offset_commit_failures", "source_id");
    // Two counters compared, not a rate: `committed > known` means the source has
    // lost sight of its upstream, which the commit-failure counter cannot catch
    // because the source never reaches the commit step. `> bool` turns the
    // comparison into a 0/1 indicator, and the trailing `> 0` drops the healthy
    // sources so the panel is empty when all is well.
    let disconnected = format!(
        r#"
(
    max by (source_id) (
        max without (job) (mz_source_offset_committed{{{scope}}})
    ) > bool max by (source_id) (
        max without (job) (mz_source_offset_known{{{scope}}})
    )
) > 0
"#,
        scope = scope()
    );
    Panel::timeseries("Source Upstream Errors")
        .description(
            "**Per-source upstream health — healthy sources are filtered out (`> 0`), so this \
             panel is empty when all is well and any series at all means a source needs \
             attention.** Two signals: **commit failures** (`mz_source_offset_commit_failures` \
             rate) fire when the upstream is reachable but rejects the offset / \
             replication-slot commit (auth/ACL, broker rejecting); the **disconnected** \
             indicator flips to **1** when the source has lost sight of its upstream \
             (`offset_known` fell below `offset_committed`) — the broker/DB-unreachable case \
             (`BrokerTransportFailure`, severed security group, DNS) that the commit-failure \
             counter can't catch because the source never reaches the commit step. When \
             `disconnected` is 1, _Source Bytes Received_ flat-lines and _Compute Objects -> \
             Freshness_ frontier lag climbs. Legend is `source_id` — name it with `SELECT id, \
             name FROM mz_sources` and read the exact error via `SELECT name, status, error \
             FROM mz_internal.mz_source_statuses WHERE status != 'running'`. (Data-decode \
             errors are separate: `mz_source_error_inserts`.)",
        )
        .data(query_group(vec![
            PromQuery::new(commit_failures)
                .legend("{{source_id}} commit failures")
                .build(),
            PromQuery::new(disconnected)
                .legend("{{source_id}} disconnected")
                .build(),
        ]))
        .unit("short")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 1.0).build())
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

/// A per-sink series, named via the catalog.
fn named_sink(expr: String) -> String {
    enrich::with_object_name(&expr, "sink_id", None, &env())
}

fn sink_throughput() -> dashboardv2::PanelKind {
    let expr = named_sink(active_object_rate("mz_sink_bytes_committed", "sink_id"));
    Panel::timeseries("Sink Throughput (committed)")
        .description(
            "**Outbound throughput per sink — bytes per second successfully committed to the \
             downstream system** (Kafka broker, Iceberg catalog, etc.). Log Y-axis so low- and \
             high-volume sinks share the chart. Labeled by sink name (resolved via \
             `mz_object_info`). Idle sinks are filtered out (`> 0`). Scoped to the selected \
             clusters/replicas.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{name}}").build(),
        ]))
        .unit("Bps")
        .log_scale(10.0)
        .build(0)
}

fn sink_lag() -> dashboardv2::PanelKind {
    // Both sides are counters; their difference is what is staged but not yet
    // acknowledged. `clamp_min(…, 0)` because the two counters are scraped
    // independently and can momentarily disagree in the wrong direction.
    let base = format!(
        r#"
clamp_min(
    sum by (sink_id) (max without (job) (mz_sink_bytes_staged{{{scope}}}))
    - sum by (sink_id) (max without (job) (mz_sink_bytes_committed{{{scope}}})),
    0
)
"#,
        scope = scope()
    );
    Panel::timeseries("Sink Lag (staged minus committed)")
        .description(
            "**Bytes staged for a sink but not yet committed downstream — a queue depth in \
             bytes.** Both metrics are counters; their difference at any moment is the \
             in-flight write that's been prepared but not yet acknowledged by the downstream \
             system. Nominal: oscillates around a small value as commits happen periodically. \
             **Sustained growth means the sink can't keep up** — usually downstream \
             back-pressure (broker overloaded, Iceberg catalog slow) or repeated commit \
             failures (see _Iceberg Commit Failures & Conflicts_ or _Kafka TX Error Rate_ in \
             the collapsed rows below). Scoped to the selected clusters/replicas.",
        )
        .data(query_group(vec![
            PromQuery::new(named_sink(base)).legend("{{name}}").build(),
        ]))
        .unit("bytes")
        .min(0.0)
        .build(0)
}

fn iceberg_commit_latency() -> dashboardv2::PanelKind {
    // Not per-sink: the histogram is aggregated across sinks, so the percentiles
    // describe the environment's Iceberg commit behaviour rather than one table's.
    let quantile = |q: f64| {
        format!(
            r#"
histogram_quantile({q},
    sum by (le) (
        max without (job) (
            rate(mz_sink_iceberg_commit_duration_seconds_bucket{{{scope}}}[$__rate_interval])
        )
    )
)
"#,
            scope = scope()
        )
    };
    Panel::timeseries("Iceberg Commit Latency (p50 / p90 / p99)")
        .description(
            "**Iceberg commit duration percentiles** — how long each `COMMIT` against the \
             Iceberg catalog takes. Iceberg writes are batched and committed periodically; the \
             commit involves writing a snapshot manifest and asking the catalog to atomically \
             swap it in. Nominal: p50 sub-second to low seconds; p99 a few seconds even on \
             healthy systems. Sustained p99 in tens of seconds points at a slow Iceberg catalog \
             (REST catalog under load, Glue API throttling) — _Sink Lag_ will be growing at the \
             same time. Log Y-axis. Scoped to the selected clusters/replicas.",
        )
        .data(query_group(vec![
            PromQuery::new(quantile(0.5)).legend("p50").build(),
            PromQuery::new(quantile(0.9)).legend("p90").build(),
            PromQuery::new(quantile(0.99)).legend("p99").build(),
        ]))
        .unit("s")
        .log_scale(10.0)
        .build(0)
}

fn iceberg_failures() -> dashboardv2::PanelKind {
    Panel::timeseries("Iceberg Commit Failures & Conflicts")
        .description(
            "**Per-sink rate of failed and conflicting Iceberg commits.** Conflicts \
             (concurrent-writer races on the Iceberg snapshot pointer) are recoverable — \
             Materialize retries — but a high rate signals that something else is writing to \
             the same Iceberg table. Failures are commit-side errors (network, auth, schema). \
             **Non-zero in either dimension is worth investigating.** If failures are climbing, \
             _Sink Lag_ will follow. The Errors threshold-coloring is calibrated for \"any \
             non-zero is interesting\".",
        )
        .data(query_group(vec![
            PromQuery::new(named_sink(object_rate(
                "mz_sink_iceberg_commit_failures",
                "sink_id",
                None,
            )))
            .legend("{{name}} failures")
            .build(),
            PromQuery::new(named_sink(object_rate(
                "mz_sink_iceberg_commit_conflicts",
                "sink_id",
                None,
            )))
            .legend("{{name}} conflicts")
            .build(),
        ]))
        .unit("cps")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 10.0).build())
        .build(0)
}

fn iceberg_files() -> dashboardv2::PanelKind {
    let rate = |metric: &str| named_sink(object_rate(metric, "sink_id", None));
    Panel::timeseries("Iceberg File & Snapshot Rate")
        .description(
            "**Per-sink rate of files and snapshots written to Iceberg.** Each commit produces \
             one snapshot containing some data files (new rows) and delete files (tombstones \
             for upserts). The data:delete file ratio tells you about your workload: \
             pure-insert sinks produce ~0 deletes; upsert-heavy workloads produce roughly 1:1. \
             Sustained delete-file rate without data files means the sink is mostly deleting \
             (data evaporating upstream). Scoped to the selected clusters/replicas.",
        )
        .data(query_group(vec![
            PromQuery::new(rate("mz_sink_iceberg_data_files_written"))
                .legend("{{name}} data")
                .build(),
            PromQuery::new(rate("mz_sink_iceberg_delete_files_written"))
                .legend("{{name}} deletes")
                .build(),
            PromQuery::new(rate("mz_sink_iceberg_snapshots_committed"))
                .legend("{{name}} snapshots")
                .build(),
        ]))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn kafka_tx_errors() -> dashboardv2::PanelKind {
    let expr = named_sink(object_rate("mz_sink_rdkafka_txerrs", "sink_id", None));
    Panel::timeseries("Kafka TX Error Rate")
        .description(
            "**Per-sink rate of TX errors from the librdkafka client.** Each TX error is one \
             failed produce-request against the Kafka broker. **Non-zero is a problem** — \
             likely causes are broker outages, ACL changes, topic deletion/recreation, or \
             partition rebalancing. If errors are sustained, _Sink Lag_ will grow and _Kafka \
             Output Buffer_ may fill. Errors threshold-coloring is calibrated for \"any \
             non-zero is interesting\".",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{name}}").build(),
        ]))
        .unit("cps")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 10.0).build())
        .build(0)
}

fn kafka_output_buffer() -> dashboardv2::PanelKind {
    // A gauge, not a rate: the question is how deep the queue is right now.
    let expr = named_sink(object_gauge("mz_sink_rdkafka_outbuf_msg_cnt", "sink_id"));
    Panel::timeseries("Kafka Output Buffer (messages)")
        .description(
            "**Messages currently sitting in the librdkafka output buffer, waiting to be sent \
             to the broker.** Normal buffer fluctuates briefly as messages flow through; \
             sustained high values mean Materialize is producing faster than the broker is \
             accepting. Often paired with a non-zero _Kafka TX Error Rate_. If the buffer hits \
             its bound, the sink stalls and _Sink Lag_ starts climbing.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).legend("{{name}}").build(),
        ]))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn kafka_connects() -> dashboardv2::PanelKind {
    let rate = |metric: &str| named_sink(object_rate(metric, "sink_id", None));
    Panel::timeseries("Kafka Connect / Disconnect Rate")
        .description(
            "**Connect and disconnect events per sink against the Kafka broker.** Healthy \
             connections are persistent — a couple of connects at sink startup and zero \
             disconnects afterward. **Sustained non-zero disconnect rate is a sign of unhealthy \
             connectivity** (network flakiness, broker restarting, auth tokens expiring). Pairs \
             with _Kafka TX Error Rate_ when the issue is broker-side rather than purely \
             network.",
        )
        .data(query_group(vec![
            PromQuery::new(rate("mz_sink_rdkafka_connects"))
                .legend("{{name}} connects")
                .build(),
            PromQuery::new(rate("mz_sink_rdkafka_disconnects"))
                .legend("{{name}} disconnects")
                .build(),
        ]))
        .unit("cps")
        .min(0.0)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exprs(panel: &dashboardv2::PanelKind) -> Vec<String> {
        panel
            .spec
            .data
            .spec
            .queries
            .iter()
            .map(|q| {
                q.spec
                    .query
                    .spec
                    .as_ref()
                    .and_then(|s| s.get("expr"))
                    .and_then(|e| e.as_str())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn the_tab_has_five_rows_and_seventeen_panels() {
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 17);
    }

    #[test]
    fn every_storage_rate_guards_against_a_double_scrape() {
        // A replica scraped by two jobs reports each counter twice; summing without
        // collapsing `job` first would double every rate on this tab.
        for panel in [
            source_bytes_received(),
            ingestion_by_replica(),
            sink_throughput(),
            sink_lag(),
            iceberg_failures(),
            iceberg_files(),
            kafka_tx_errors(),
            kafka_output_buffer(),
            kafka_connects(),
        ] {
            for expr in exprs(&panel) {
                assert!(
                    expr.contains("max without (job)"),
                    "missing the double-scrape guard in {}:\n{expr}",
                    panel.spec.title
                );
            }
        }
    }

    #[test]
    fn the_object_counts_exclude_progress_subsources() {
        // Without the inner `group by (id)` these counts roughly double, because
        // every source and sink carries a hidden `_progress` subsource.
        for panel in [active_sources(), active_sinks()] {
            let expr = exprs(&panel).remove(0);
            assert!(expr.contains("group by (id)"), "{expr}");
        }
    }

    #[test]
    fn per_object_panels_resolve_names_from_the_catalog() {
        // Storage metrics are keyed by id; without the join every legend reads
        // `u42` and the panel is unusable during an incident.
        for (panel, key) in [
            (source_bytes_received(), "parent_source_id"),
            (ingestion_by_replica(), "parent_source_id"),
            (sink_throughput(), "sink_id"),
            (sink_lag(), "sink_id"),
            (kafka_tx_errors(), "sink_id"),
        ] {
            let expr = exprs(&panel).remove(0);
            assert!(
                expr.contains(&format!("* on ({key}) group_left(name)")),
                "expected a name join on {key} in {}:\n{expr}",
                panel.spec.title
            );
        }
    }

    #[test]
    fn the_source_error_panel_carries_both_signals() {
        // The commit-failure counter cannot see a source that never reaches the
        // commit step, which is what the disconnected indicator is for.
        let panel = source_errors();
        let queries = exprs(&panel);
        assert_eq!(queries.len(), 2);
        assert!(queries[0].contains("mz_source_offset_commit_failures"));
        assert!(queries[1].contains("> bool"), "{}", queries[1]);
        assert!(queries[1].contains("mz_source_offset_known"));
    }

    #[test]
    fn the_error_panels_are_empty_when_healthy() {
        // `> 0` on the rate is what makes "any series at all" the signal.
        for expr in exprs(&source_errors()) {
            assert!(expr.trim_end().ends_with("> 0"), "{expr}");
        }
    }

    #[test]
    fn the_connector_rows_are_collapsed() {
        // Most environments run one of Kafka or Iceberg; the other is noise.
        for row in [iceberg(), kafka()] {
            let assembled = mzmon_lib::grafana::layout::Layout::rows(vec![row])
                .assemble()
                .expect("assemble");
            let dashboardv2::DashboardLayout::RowsLayoutKind(rows) = &assembled.layout else {
                panic!("expected rows");
            };
            assert_eq!(rows.spec.rows[0].spec.collapse, Some(true));
        }
    }

    #[test]
    fn iceberg_latency_is_aggregated_across_sinks() {
        // Deliberately not per-sink: the percentiles describe the environment's
        // Iceberg commit behaviour, and a per-sink histogram would be too sparse to
        // read.
        for expr in exprs(&iceberg_commit_latency()) {
            assert!(expr.contains("sum by (le)"), "{expr}");
            assert!(!expr.contains("sink_id"), "{expr}");
        }
    }
}
