// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Reads tab: queries through Thanos Query, and the stores that answer
//! them.
//!
//! Thanos Query fails quietly. A store it cannot reach is left out of the answer
//! rather than failing the query, so the Stores row is the one that says whether
//! an answer was complete.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::not_collected;
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![queries(q), stores(q), store_gateway(q)]
}

fn queries(q: &Queries) -> Row {
    Row::new("Thanos Query").grid(
        AutoGrid::new(3)
            .panel("reads-requests", requests(q))
            .panel("reads-latency", latency(q))
            .panel("reads-concurrency", concurrency(q)),
    )
}

fn stores(q: &Queries) -> Row {
    Row::new("Stores").grid(
        AutoGrid::new(4)
            .panel("reads-endpoints", endpoints(q))
            .panel("reads-store-errors", store_errors(q))
            .panel("reads-series-latency", series_latency(q))
            .panel("reads-dropped", queries_dropped(q)),
    )
}

fn store_gateway(q: &Queries) -> Row {
    Row::new("Store Gateway").grid(
        AutoGrid::new(3)
            .panel("reads-blocks-loaded", blocks_loaded(q))
            .panel("reads-block-failures", block_failures(q))
            .panel("reads-index-cache", index_cache(q)),
    )
}

fn no_store_gateway() -> NoValue {
    NoValue::Custom("No store gateway is reporting".to_string())
}

fn requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Requests by Endpoint and Status")
        .query(
            q.get("infra.thanos.read.requests")
                .legend("{{handler}} {{code}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Query Latency (p99)")
        .query(q.get("infra.thanos.read.latency").legend("{{handler}}"))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom("No queries in this range".to_string()))
        .build(0)
}

fn concurrency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Concurrent Queries")
        .query(q.legended("infra.thanos.read.concurrency", &["in flight", "limit"]))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn endpoints(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Stores Connected")
        .query(
            q.get("infra.thanos.read.endpoints")
                .legend("{{store_type}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn store_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Store Request Errors")
        .query(
            q.get("infra.thanos.read.store_errors")
                .legend("{{container}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_collected())
        .build(0)
}

fn series_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Series Request Latency (p99)")
        .query(
            q.get("infra.thanos.read.series_latency")
                .legend("{{container}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No series requests in this range".to_string(),
        ))
        .build(0)
}

fn queries_dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Requests Refused by Limits")
        .query(q.legended(
            "infra.thanos.read.queries_dropped",
            &["{{container}} selects", "{{container}} series"],
        ))
        .unit("reqps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn blocks_loaded(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Blocks Loaded")
        .query(q.get("infra.thanos.read.blocks_loaded").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(no_store_gateway())
        .build(0)
}

fn block_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Block Load and Drop Failures")
        .query(q.get("infra.thanos.read.block_failures").legend("{{pod}}"))
        .unit("suffix:/s")
        .min(0.0)
        .no_value(no_store_gateway())
        .build(0)
}

/// Bounded at 1, so the ceiling is pinned: a hit ratio of 0.4 has to read as
/// 0.4, not as a line near the top of an autoscaled axis.
fn index_cache(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Index Cache Hit Ratio")
        .query(
            q.get("infra.thanos.read.index_cache_hits")
                .legend("{{item_type}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "No index lookups in this range".to_string(),
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
        assert_eq!(assembled.elements.len(), 10);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }
}
