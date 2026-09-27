// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Metric Pipeline tab: a sample's path from a target to the stores.
//!
//! Scraping first, then the two ways out: the remote-write queue with the
//! write-ahead log behind it, and the OpenTelemetry exporters for any extra
//! destination. The exporter row is collapsed, since a default install
//! configures none and its panels are empty. Samples pushed into the gateway
//! rather than scraped are on the Ingest tab.
//!
//! # Target-wide panels
//!
//! Four panels read `up` and `scrape_*`, which describe each *target* the
//! gateway scrapes rather than the gateway. Those series carry the target's
//! labels, so the collector pickers cannot narrow them. The dashboard's
//! anchoring test lists them by name, so it can tell them apart from a query
//! that lost its scope by accident.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::overview::{self, SAMPLES_PER_SECOND, not_collected};
use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::METRIC_PIPELINE.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        flow(q),
        scraping(q),
        heaviest(q),
        remote_write(q),
        wal(q),
        exporters(q),
    ]
}

fn flow(q: &Queries) -> Row {
    Row::new("Flow").hide_header().grid(
        AutoGrid::new(5)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("metrics-flow-targets", targets(q))
            .panel("metrics-flow-targets-down", targets_down(q))
            .panel("metrics-flow-samples", samples_sent(q))
            .panel("metrics-flow-lost", overview::samples_lost(q))
            .panel("metrics-flow-delay", worst_send_delay(q)),
    )
}

fn scraping(q: &Queries) -> Row {
    Row::new("Scraping").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("metrics-scrape-down", down_targets(q))
            .panel("metrics-scrape-by-monitor", targets_by_monitor(q))
            .panel("metrics-scrape-rejected", rejected_scrapes(q))
            .panel("metrics-scrape-rejected-samples", rejected_samples(q))
            .panel("metrics-scrape-samples", samples_scraped(q)),
    )
}

fn heaviest(q: &Queries) -> Row {
    Row::new("Heaviest Targets").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("metrics-heavy-slowest", slowest_scrapes(q))
            .panel("metrics-heavy-largest", largest_scrapes(q)),
    )
}

fn remote_write(q: &Queries) -> Row {
    Row::new("Remote Write").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("metrics-rw-samples", remote_write_samples(q))
            .panel("metrics-rw-delay", send_delay(q))
            .panel("metrics-rw-shards", shards(q))
            .panel("metrics-rw-latency", send_latency(q)),
    )
}

fn wal(q: &Queries) -> Row {
    Row::new("Write-Ahead Log").grid(
        AutoGrid::new(3)
            .panel("metrics-wal-series", active_series(q))
            .panel("metrics-wal-size", wal_size(q))
            .panel("metrics-wal-errors", wal_errors(q)),
    )
}

/// Collapsed: a default install configures no OpenTelemetry destination, and
/// four empty panels on the open page would read as four problems.
fn exporters(q: &Queries) -> Row {
    Row::new("OpenTelemetry Exporters").collapsed().grid(
        AutoGrid::new(2)
            .panel("metrics-otel-sent", otel_sent(q))
            .panel("metrics-otel-failed", otel_failed(q))
            .panel("metrics-otel-queue", otel_queue(q))
            .panel("metrics-otel-filtered", otel_filtered(q)),
    )
}

/// What an OpenTelemetry panel shows on an install with no such destination.
fn no_exporter() -> NoValue {
    NoValue::Custom("No OpenTelemetry destination is configured".to_string())
}

fn targets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Targets Scraped")
        .query(
            q.get("infra.alloy.metric_pipeline.targets")
                .legend("targets"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn targets_down(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Targets Down")
        .query(
            q.get("infra.alloy.metric_pipeline.targets_down")
                .legend("down"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .color_background()
        .thresholds(threshold::errors(1.0, 5.0).build())
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn samples_sent(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Samples Written")
        .query(
            q.get("infra.alloy.metric_pipeline.samples_sent")
                .legend("samples/s"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn worst_send_delay(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Worst Send Delay")
        .query(
            q.get("infra.alloy.metric_pipeline.send_delay.worst")
                .legend("behind"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

/// Value and time dropped: every row's value is 0 by construction, and the
/// labels are the finding.
fn down_targets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Targets Down")
        .query(
            q.get("infra.alloy.metric_pipeline.down_targets")
                .table_format(),
        )
        .transformations(vec![transform::organize(
            &["Time", "Value"],
            &["job", "namespace", "pod", "instance"],
        )])
        // Empty when every target is up, and also when nothing is scraped at
        // all. The Targets Scraped stat above tells the two apart.
        .no_value(NoValue::Custom(
            "No target is down. If Targets Scraped is empty too, nothing is being collected."
                .to_string(),
        ))
        .build(0)
}

fn targets_by_monitor(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Targets by Monitor")
        .query(
            q.get("infra.alloy.metric_pipeline.targets_by_monitor")
                .table_format(),
        )
        .transformations(vec![
            transform::organize_full(
                &["Time"],
                &["scrape_job", "Value"],
                &[("scrape_job", "monitor"), ("Value", "targets")],
            ),
            transform::sort_by("targets", false),
        ])
        .unit("short")
        .no_value(not_collected())
        .build(0)
}

fn rejected_scrapes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rejected Scrapes")
        .query(q.legended(
            "infra.alloy.metric_pipeline.rejected_scrapes",
            &[
                "sample limit",
                "body size limit",
                "target limit",
                "label limits",
            ],
        ))
        .unit("suffix:scrapes/s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn rejected_samples(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rejected Samples")
        .query(q.legended(
            "infra.alloy.metric_pipeline.rejected_samples",
            &["out of order", "duplicate timestamp", "out of bounds"],
        ))
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn slowest_scrapes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Slowest Scrapes (Top 10)")
        .query(
            q.get("infra.alloy.metric_pipeline.slowest_scrapes")
                .legend("{{job}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn largest_scrapes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Largest Scrapes (Top 10)")
        .query(
            q.get("infra.alloy.metric_pipeline.largest_scrapes")
                .legend("{{job}}"),
        )
        .unit("suffix:samples")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn samples_scraped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Samples Scraped by Source")
        .query(
            q.get("infra.alloy.metric_pipeline.samples_scraped")
                .legend("{{component_id}}"),
        )
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn remote_write_samples(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Samples Sent, Failed and Retried")
        .query(q.legended(
            "infra.alloy.metric_pipeline.remote_write",
            &[
                "{{component_id}} sent",
                "{{component_id}} failed",
                "{{component_id}} retried",
            ],
        ))
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn send_delay(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Send Delay")
        .query(
            q.get("infra.alloy.metric_pipeline.send_delay")
                .legend("{{pod}} {{component_id}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn shards(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Shards")
        .query(q.legended(
            "infra.alloy.metric_pipeline.shards",
            &[
                "{{component_id}} running",
                "{{component_id}} desired",
                "{{component_id}} max",
            ],
        ))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn send_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Send Latency (p99)")
        .query(
            q.get("infra.alloy.metric_pipeline.send_latency")
                .legend("{{component_id}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn active_series(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Active Series")
        .query(
            q.get("infra.alloy.metric_pipeline.active_series")
                .legend("{{pod}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn wal_size(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write-Ahead Log Size")
        .query(
            q.get("infra.alloy.metric_pipeline.wal_size")
                .legend("{{pod}}"),
        )
        .unit("bytes")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn wal_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write-Ahead Log Errors")
        .query(q.legended(
            "infra.alloy.metric_pipeline.wal_errors",
            &[
                "corruptions",
                "failed writes",
                "failed checkpoints",
                "failed truncations",
            ],
        ))
        .unit("suffix:errors/s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn otel_sent(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Points Exported")
        .query(q.legended(
            "infra.alloy.metric_pipeline.otel_sent",
            &["{{component_id}} metrics", "{{component_id}} logs"],
        ))
        .unit("suffix:points/s")
        .min(0.0)
        .no_value(no_exporter())
        .build(0)
}

fn otel_failed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Export Failures")
        .query(q.legended(
            "infra.alloy.metric_pipeline.otel_failed",
            &["{{component_id}} metrics", "{{component_id}} logs"],
        ))
        .unit("suffix:points/s")
        .min(0.0)
        .no_value(no_exporter())
        .build(0)
}

/// A bounded fraction whose nominal is zero, so the ceiling is pinned.
fn otel_queue(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Export Queue Fill")
        .query(
            q.get("infra.alloy.metric_pipeline.otel_queue")
                .legend("{{component_id}} {{data_type}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(no_exporter())
        .build(0)
}

fn otel_filtered(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Points Filtered")
        .query(
            q.get("infra.alloy.metric_pipeline.otel_filtered")
                .legend("{{component_id}}"),
        )
        .unit("suffix:points/s")
        .min(0.0)
        .no_value(no_exporter())
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 23);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_tables_of_current_facts_are_instant_and_render_as_one_table() {
        let q = &test_queries();
        for panel in [down_targets(q), targets_by_monitor(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{json}");
            assert!(json.contains(r#""format":"table""#), "{json}");
        }
    }

    #[test]
    fn the_remote_write_panels_exclude_other_senders() {
        // `thanos-ruler` publishes the same family, and an unanchored query
        // would add its queue to the gateway's.
        let q = &test_queries();
        for panel in [remote_write_samples(q), send_delay(q), shards(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains("$alloyRole"), "{json}");
        }
    }
}
