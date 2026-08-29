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
use mzmon_lib::grafana::threshold;
use mzmon_lib::query::enrich;

use super::{selector, theme};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// The tab's theme.
const SHADE: &str = theme::SOURCES_SINKS.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![summary(q), sources(q), sinks(q), iceberg(q), kafka(q)]
}

fn summary(q: &Queries) -> Row {
    Row::new("Storage Objects Summary").hide_header().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("storage-active-sources", active_sources(q))
            .panel("storage-active-sinks", active_sinks(q))
            .panel("storage-active-tables", active_tables(q)),
    )
}

fn sources(q: &Queries) -> Row {
    Row::new("Sources").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sources-types", source_types(q))
            .panel("sources-status-table", source_catalog(q))
            .panel("sources-bytes-received-rate", source_bytes_received(q))
            .panel("sources-ingestion-by-replica", ingestion_by_replica(q))
            .panel("sources-errors", source_errors(q)),
    )
}

fn sinks(q: &Queries) -> Row {
    Row::new("Sinks").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sinks-types", sink_types(q))
            .panel("sinks-throughput", sink_throughput(q))
            .panel("sinks-lag", sink_lag(q)),
    )
}

fn iceberg(q: &Queries) -> Row {
    // Collapsed: an environment with no Iceberg sinks would otherwise pay for
    // three empty panels on every load.
    Row::new("Iceberg Sinks").collapsed().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sinks-iceberg-commit-latency", iceberg_commit_latency(q))
            .panel("sinks-iceberg-failures", iceberg_failures(q))
            .panel("sinks-iceberg-files", iceberg_files(q)),
    )
}

fn kafka(q: &Queries) -> Row {
    Row::new("Kafka Sinks").collapsed().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("sinks-kafka-tx-errors", kafka_tx_errors(q))
            .panel("sinks-kafka-outbuf", kafka_output_buffer(q))
            .panel("sinks-kafka-connects", kafka_connects(q)),
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

fn active_sources(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Sources")
        .query(q.get("materialize.storage.sources.count").legend("sources"))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_sinks(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Sinks")
        .query(q.get("materialize.storage.sinks.count").legend("sinks"))
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn active_tables(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Active Tables")
        .query(
            q.get("materialize.storage.tables.count")
                .legend("mz_tables_count"),
        )
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn source_types(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Source Types")
        .query(
            q.get("materialize.storage.sources.by_type")
                .legend("{{object_type}}"),
        )
        .shade(SHADE)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn sink_types(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Sink Types")
        .query(
            q.get("materialize.storage.sinks.by_type")
                .legend("{{object_type}} / {{envelope_type}}"),
        )
        .shade(SHADE)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn source_catalog(q: &Queries) -> dashboardv2::PanelKind {
    // The catalog metric is keyed by `id`, so the name join goes through that
    // rather than a source-specific label.
    const COLUMNS: &[&str] = &[
        "name",
        "id",
        "object_type",
        "connection_type",
        "envelope_type",
        "cluster_id",
    ];
    Panel::table("Sources")
        .query(q.get("materialize.storage.sources.catalog"))
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

fn source_bytes_received(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Source Bytes Received (rate)")
        .query(
            q.get("materialize.storage.sources.bytes_received")
                .legend("{{name}}"),
        )
        .unit("Bps")
        .log_scale(10.0)
        .build(0)
}

/// `{{name}} / r{{<replica label>}}`, in the label spelling the replica-history
/// metrics use.
///
/// Built rather than written literally because the label name differs across the
/// three cluster-id families — see [`super::selector`].
fn replica_legend() -> String {
    format!(
        "{{{{name}}}} / r{{{{{replica}}}}}",
        replica = selector::HISTORY_REPLICA_LABEL
    )
}

fn ingestion_by_replica(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Source Ingestion by Replica")
        .query(
            q.get("materialize.storage.sources.ingestion_by_replica")
                .legend(&replica_legend()),
        )
        .unit("cps")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn source_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Source Upstream Errors")
        .query(q.legended(
            "materialize.storage.sources.upstream_errors",
            &[
                "{{source_id}} commit failures",
                "{{source_id}} disconnected",
            ],
        ))
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

fn sink_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Sink Throughput (committed)")
        .query(
            q.get("materialize.storage.sinks.throughput")
                .legend("{{name}}"),
        )
        .unit("Bps")
        .log_scale(10.0)
        .build(0)
}

fn sink_lag(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Sink Lag (staged minus committed)")
        .query(q.get("materialize.storage.sinks.lag").legend("{{name}}"))
        .unit("bytes")
        .min(0.0)
        .build(0)
}

fn iceberg_commit_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Iceberg Commit Latency (p50 / p90 / p99)")
        .query(q.legended(
            "materialize.storage.sinks.iceberg.commit_latency",
            &["p50", "p90", "p99"],
        ))
        .unit("s")
        .log_scale(10.0)
        .build(0)
}

fn iceberg_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Iceberg Commit Failures & Conflicts")
        .query(q.legended(
            "materialize.storage.sinks.iceberg.commit_failures",
            &["{{name}} failures", "{{name}} conflicts"],
        ))
        .unit("cps")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 10.0).build())
        .build(0)
}

fn iceberg_files(q: &Queries) -> dashboardv2::PanelKind {
    let _rate = |metric: &str| named_sink(object_rate(metric, "sink_id", None));
    Panel::timeseries("Iceberg File & Snapshot Rate")
        .query(q.legended(
            "materialize.storage.sinks.iceberg.file_rate",
            &["{{name}} data", "{{name}} deletes", "{{name}} snapshots"],
        ))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn kafka_tx_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Kafka TX Error Rate")
        .query(
            q.get("materialize.storage.sinks.kafka.tx_errors")
                .legend("{{name}}"),
        )
        .unit("cps")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 10.0).build())
        .build(0)
}

fn kafka_output_buffer(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Kafka Output Buffer (messages)")
        .query(
            q.get("materialize.storage.sinks.kafka.output_buffer")
                .legend("{{name}}"),
        )
        .unit("short")
        .min(0.0)
        .build(0)
}

fn kafka_connects(q: &Queries) -> dashboardv2::PanelKind {
    let _rate = |metric: &str| named_sink(object_rate(metric, "sink_id", None));
    Panel::timeseries("Kafka Connect / Disconnect Rate")
        .query(q.legended(
            "materialize.storage.sinks.kafka.connect_rate",
            &["{{name}} connects", "{{name}} disconnects"],
        ))
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
        let q = &crate::grafana::queries::test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 17);
    }

    #[test]
    fn every_storage_rate_guards_against_a_double_scrape() {
        let q = &crate::grafana::queries::test_queries();
        // A replica scraped by two jobs reports each counter twice; summing without
        // collapsing `job` first would double every rate on this tab.
        for panel in [
            source_bytes_received(q),
            ingestion_by_replica(q),
            sink_throughput(q),
            sink_lag(q),
            iceberg_failures(q),
            iceberg_files(q),
            kafka_tx_errors(q),
            kafka_output_buffer(q),
            kafka_connects(q),
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
        let q = &crate::grafana::queries::test_queries();
        // Without the inner `group by (id)` these counts roughly double, because
        // every source and sink carries a hidden `_progress` subsource.
        for panel in [active_sources(q), active_sinks(q)] {
            let expr = exprs(&panel).remove(0);
            assert!(expr.contains("group by (id)"), "{expr}");
        }
    }

    #[test]
    fn per_object_panels_resolve_names_from_the_catalog() {
        let q = &crate::grafana::queries::test_queries();
        // Storage metrics are keyed by id; without the join every legend reads
        // `u42` and the panel is unusable during an incident.
        for (panel, key) in [
            (source_bytes_received(q), "parent_source_id"),
            (ingestion_by_replica(q), "parent_source_id"),
            (sink_throughput(q), "sink_id"),
            (sink_lag(q), "sink_id"),
            (kafka_tx_errors(q), "sink_id"),
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
        let q = &crate::grafana::queries::test_queries();
        // The commit-failure counter cannot see a source that never reaches the
        // commit step, which is what the disconnected indicator is for.
        let panel = source_errors(q);
        let queries = exprs(&panel);
        assert_eq!(queries.len(), 2);
        assert!(queries[0].contains("mz_source_offset_commit_failures"));
        assert!(queries[1].contains("> bool"), "{}", queries[1]);
        assert!(queries[1].contains("mz_source_offset_known"));
    }

    #[test]
    fn the_error_panels_are_empty_when_healthy() {
        let q = &crate::grafana::queries::test_queries();
        // `> 0` on the rate is what makes "any series at all" the signal.
        for expr in exprs(&source_errors(q)) {
            assert!(expr.trim_end().ends_with("> 0"), "{expr}");
        }
    }

    #[test]
    fn the_connector_rows_are_collapsed() {
        let q = &crate::grafana::queries::test_queries();
        // Most environments run one of Kafka or Iceberg; the other is noise.
        for row in [iceberg(q), kafka(q)] {
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
        let q = &crate::grafana::queries::test_queries();
        // Deliberately not per-sink: the percentiles describe the environment's
        // Iceberg commit behaviour, and a per-sink histogram would be too sparse to
        // read.
        for expr in exprs(&iceberg_commit_latency(q)) {
            assert!(expr.contains("sum by (le)"), "{expr}");
            assert!(!expr.contains("sink_id"), "{expr}");
        }
    }
}
