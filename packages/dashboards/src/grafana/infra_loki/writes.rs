// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Writes tab: what arrives, and what becomes of it.
//!
//! The write path in the order a line travels it — distributor, ingester, chunk,
//! object storage — because that is also the order in which the failures nest.
//! A discard at the distributor never reaches an ingester, so the rejection row
//! comes before the ingester rows rather than after them.
//!
//! **The rejection row is the one to read first.** Everything else here degrades
//! visibly: streams climb, chunks pile up, the write-ahead log fills. A discard
//! is silent at both ends — the collector was told the line was delivered, and
//! the store never had it — so nothing else on this dashboard will tell you
//! about it.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::overview::LINES_PER_SECOND;
use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::WRITES.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![ingest(q), rejected(q), ingesters(q), chunks(q), wal(q)]
}

fn ingest(q: &Queries) -> Row {
    Row::new("Ingest").hide_header().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .panel("writes-lines", lines(q))
            .panel("writes-bytes", bytes(q))
            .panel("writes-push-latency", push_latency(q)),
    )
}

fn rejected(q: &Queries) -> Row {
    Row::new("Rejected").grid(
        AutoGrid::new(2)
            .panel("writes-discarded-lines", discarded_lines(q))
            .panel("writes-discarded-bytes", discarded_bytes(q)),
    )
}

fn ingesters(q: &Queries) -> Row {
    Row::new("Ingesters").grid(
        AutoGrid::new(3)
            .panel("writes-streams", streams(q))
            .panel("writes-chunks-memory", chunks_in_memory(q))
            .panel("writes-chunks-flushed", chunks_flushed(q)),
    )
}

fn chunks(q: &Queries) -> Row {
    Row::new("Chunks").grid(
        AutoGrid::new(2)
            .panel("writes-chunk-utilization", chunk_utilization(q))
            .panel("writes-chunk-age", chunk_age(q)),
    )
}

fn wal(q: &Queries) -> Row {
    Row::new("Write-Ahead Log").grid(
        AutoGrid::new(3)
            .panel("writes-wal-disk", wal_disk(q))
            .panel("writes-wal-full", wal_full(q))
            .panel("writes-wal-replay", wal_replay(q)),
    )
}

fn lines(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Lines Received")
        .query(q.get("infra.loki.write.lines").legend("lines/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing is reaching the distributor".to_string(),
        ))
        .build(0)
}

fn bytes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Bytes Received")
        .query(q.get("infra.loki.write.bytes").legend("bytes/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing is reaching the distributor".to_string(),
        ))
        .build(0)
}

fn push_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Push Latency (p99)")
        .query(q.legended("infra.loki.write.push_latency", &["p50", "p99"]))
        .reduce_fields("/p99/")
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom("No pushes in this range".to_string()))
        .build(0)
}

/// Not shaded with the tab colour. Zero is the only healthy reading, and a
/// number whose whole meaning is "this should be zero" gets health colours.
fn discarded_lines(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Discarded Lines by Reason")
        .query(q.get("infra.loki.write.discarded").legend("{{reason}}"))
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom("Nothing is being discarded.".to_string()))
        .build(0)
}

fn discarded_bytes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Discarded Bytes by Reason")
        .query(
            q.get("infra.loki.write.discarded_bytes")
                .legend("{{reason}}"),
        )
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::Custom("Nothing is being discarded.".to_string()))
        .build(0)
}

fn streams(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Active Streams")
        .query(q.get("infra.loki.write.streams").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn chunks_in_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Chunks in Memory")
        .query(q.get("infra.loki.write.chunks_in_memory").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn chunks_flushed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Chunks Flushed")
        .query(q.get("infra.loki.write.chunks_flushed").legend("chunks/s"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing has been flushed to object storage in this range".to_string(),
        ))
        .build(0)
}

/// Capped at 1.0, because the metric is a ratio and an axis that autoscales past
/// full makes a healthy chunk look half empty.
fn chunk_utilization(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Chunk Utilization on Flush")
        .query(q.legended("infra.loki.write.chunk_utilization", &["p50", "p10"]))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "No chunks were flushed in this range".to_string(),
        ))
        .build(0)
}

fn chunk_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Chunk Age on Flush")
        .query(q.legended("infra.loki.write.chunk_age", &["p50", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No chunks were flushed in this range".to_string(),
        ))
        .build(0)
}

fn wal_disk(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("WAL Disk Usage")
        .query(q.get("infra.loki.write.wal_disk").legend("{{pod}}"))
        .unit("percent")
        .min(0.0)
        .max(100.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn wal_full(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("WAL Disk Full Failures")
        .query(q.get("infra.loki.write.wal_disk_full").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No ingester has run out of write-ahead log space.".to_string(),
        ))
        .build(0)
}

fn wal_replay(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("WAL Replay Active")
        .query(q.get("infra.loki.write.wal_replay").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom("No ingester is replaying.".to_string()))
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
        assert_eq!(assembled.elements.len(), 13);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_discard_panels_say_zero_is_the_healthy_reading() {
        // Silent data loss: the collector believes the line was delivered and
        // the store never had it. An empty panel here must read as "nothing was
        // lost" rather than as an unloaded panel.
        let q = &test_queries();
        for panel in [discarded_lines(q), discarded_bytes(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains("Nothing is being discarded"), "{json}");
        }
    }

    #[test]
    fn the_ratio_panels_are_capped_at_their_ceiling() {
        // Utilization is a proportion; an autoscaling axis past 1.0 makes a full
        // chunk look half empty, which inverts the reading.
        let q = &test_queries();
        let json = serde_json::to_string(&chunk_utilization(q)).expect("serialize");
        assert!(json.contains(r#""max":1.0"#), "{json}");
    }
}
