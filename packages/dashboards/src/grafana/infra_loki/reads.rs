// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Reads tab: what is asked of the store, and how long that takes.
//!
//! The rows separate three things that all present as "Loki is slow" and have
//! different fixes.
//!
//! **Load** is how much is being asked. **Latency** is how long it took, and
//! carries the one panel that splits the question: queue time. A query that
//! spent its time queued was waiting for a querier, and more queriers fix it; a
//! query that spent its time running was not, and more queriers will not. That
//! distinction is most of the diagnostic value on this tab.
//!
//! **Caches** is the third, and the one that most often explains the second
//! without anything being wrong with Loki at all.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::READS.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![load(q), latency(q), caches(q)]
}

fn load(q: &Queries) -> Row {
    Row::new("Query Load").hide_header().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .panel("reads-in-flight", in_flight(q))
            .panel("reads-rate", query_rate(q))
            .panel("reads-bytes", bytes_processed(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Latency").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("reads-latency", query_latency(q))
            .panel("reads-queue", queue_duration(q)),
    )
}

fn caches(q: &Queries) -> Row {
    Row::new("Caches").grid(
        AutoGrid::new(3)
            .panel("reads-cache-requests", cache_requests(q))
            .panel("reads-cache-hit-rate", cache_hit_rate(q))
            .panel("reads-cache-memory", cache_memory(q)),
    )
}

fn in_flight(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Queries In Flight")
        .query(q.get("infra.loki.read.queries_in_flight").legend("queries"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("Nothing is querying the store".to_string()))
        .build(0)
}

fn query_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Query Rate by Route")
        .query(q.get("infra.loki.read.query_rate").legend("{{route}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom("Nothing is querying the store".to_string()))
        .build(0)
}

fn bytes_processed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Bytes Scanned")
        .query(q.get("infra.loki.read.bytes_processed").legend("scanned/s"))
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::Custom("No queries ran in this range".to_string()))
        .build(0)
}

fn query_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Query Latency")
        .query(q.legended("infra.loki.read.query_latency", &["p50", "p90", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom("No queries ran in this range".to_string()))
        .build(0)
}

/// The panel that says whether adding queriers would help.
fn queue_duration(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Scheduler Queue Time")
        .query(q.legended("infra.loki.read.queue_duration", &["p50", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No query waited for a querier.".to_string(),
        ))
        .build(0)
}

fn cache_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Cache Lookups")
        .query(q.get("infra.loki.read.cache_requests").legend("{{name}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No cache lookups in this range".to_string(),
        ))
        .build(0)
}

/// Capped at 1.0: a hit rate is a proportion, and an axis that runs past full
/// makes a well-behaved cache look like it is missing half its lookups.
fn cache_hit_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Cache Hit Rate")
        .query(q.get("infra.loki.read.cache_hit_rate").legend("{{name}}"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "No cache lookups in this range".to_string(),
        ))
        .build(0)
}

/// Depends on the memcached exporter sidecar, which is one of the three
/// plaintext targets. An empty panel here is a scrape problem more often than a
/// cache problem, which is why it points at the Overview tab rather than at the
/// cache configuration.
fn cache_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Cache Memory Used")
        .query(
            q.get("infra.loki.read.memcached_memory")
                .legend("{{service}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "No memcached exporter is being scraped — see Scrape Health on the Overview tab"
                .to_string(),
        ))
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
        assert_eq!(assembled.elements.len(), 8);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_proportion_panels_are_capped_at_their_ceiling() {
        let q = &test_queries();
        for panel in [cache_hit_rate(q), cache_memory(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""max":1.0"#), "{json}");
        }
    }

    #[test]
    fn the_cache_memory_panel_blames_the_scrape_before_the_cache() {
        // It reads the memcached exporter, which is one of the three plaintext
        // targets. Empty here is a collection problem far more often than a
        // sizing problem, and the panel has to say which to check.
        let q = &test_queries();
        let json = serde_json::to_string(&cache_memory(q)).expect("serialize");
        assert!(json.contains("Scrape Health"), "{json}");
    }
}
