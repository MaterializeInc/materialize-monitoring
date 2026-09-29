// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Operations tab: what the environment asks of object storage, and what it
//! costs.
//!
//! Two counts of the same traffic sit side by side on purpose. _Operations by
//! Type_ counts what persist asked for; _Requests to the Object Store_ counts
//! the API calls that produced, which is what the provider bills — a delete is
//! a check and a delete, and a large write is a multipart upload.
//!
//! The cache and hedging panels are about the reads that did *not* reach the
//! store, or reached it twice: the cache is why hydration after the first is
//! cheap, and hedging is the one feature that trades billed requests for tail
//! latency.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{PROCESS, not_collected, s3_only, theme};
use crate::grafana::queries::Queries;

const SHADE: &str = theme::OPERATIONS.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![load(q), latency(q), retries(q)]
}

fn load(q: &Queries) -> Row {
    Row::new("Load").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("ops-by-type", by_type(q))
            .panel("ops-throughput", throughput(q))
            .panel("ops-requests", requests(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Latency by Operation").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ops-mean-latency", mean_latency(q))
            .panel("ops-read-latency-by-process", read_latency_by_process(q)),
    )
}

fn retries(q: &Queries) -> Row {
    Row::new("Retries, Cache and Hedging").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("ops-retries", retried(q))
            .panel("ops-cache", cache(q))
            .panel("ops-hedging", hedging(q)),
    )
}

fn by_type(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Operations by Type")
        .query(q.get("materialize.persist.ops.by_type").legend("{{op}}"))
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput by Operation")
        .query(q.get("materialize.persist.ops.throughput").legend("{{op}}"))
        .unit("Bps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Requests to the Object Store")
        .query(q.get("materialize.persist.ops.requests").legend("{{op}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(s3_only())
        .build(0)
}

fn mean_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Mean Latency by Operation")
        .query(
            q.get("materialize.persist.latency.mean_by_operation")
                .legend("{{op}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn read_latency_by_process(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Read Latency by Process (p99)")
        .query(
            q.get("materialize.persist.latency.read_by_process")
                .legend(PROCESS),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn retried(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Retried Operations")
        .query(
            q.get("materialize.persist.retries.by_operation")
                .legend("{{op}}"),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn cache(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Reads Served from Cache")
        .query(q.get("materialize.persist.cache.hits").legend("from cache"))
        .shade(SHADE)
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "Nothing was read in this window, from the cache or from the store.".to_string(),
        ))
        .build(0)
}

fn hedging(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Hedged Reads")
        .query(q.legended("materialize.persist.hedging", &["sent", "won"]))
        .unit("ops")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No hedging metrics. Hedged reads were added in Materialize v26.41.".to_string(),
        ))
        .build(0)
}
