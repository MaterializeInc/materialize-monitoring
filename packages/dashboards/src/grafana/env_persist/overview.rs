// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is object storage working for this environment.
//!
//! The verdict row is five health readings and one of scale. Reads and writes
//! get a latency cell each because they fail different things — a slow read
//! stalls hydration and cold queries, a slow write stalls ingestion and every
//! materialized view — and the operator's next question depends on which.
//!
//! The error-code and timeout panels come from the S3 client beneath persist, so
//! they are populated on S3, GCS (reached through its S3-compatible API) and
//! on-premise S3-compatible stores, and empty on Azure Blob. Their empty state
//! says so, rather than reading as "no errors".

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::{PROCESS, latency_ladder, not_collected, s3_only, theme, zero_is_healthy};
use crate::grafana::queries::Queries;

const SHADE: &str = theme::OVERVIEW.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), latency(q), failures(q), logs(q)]
}

fn verdict(q: &Queries) -> Row {
    Row::new("Verdict").hide_header().grid(
        AutoGrid::new(6)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-failed-ops", failed_ops(q))
            .panel("overview-read-p99", read_p99(q))
            .panel("overview-write-p99", write_p99(q))
            .panel("overview-write-stalls", write_stalls(q))
            .panel("overview-compaction-failures", compaction_failures(q))
            .panel("overview-stored", stored(q)),
    )
}

fn latency(q: &Queries) -> Row {
    Row::new("Latency").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-read-write-latency", read_write_latency(q))
            .panel("overview-round-trip", round_trip(q)),
    )
}

fn failures(q: &Queries) -> Row {
    Row::new("Failures").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("overview-failures-by-op", failures_by_operation(q))
            .panel("overview-store-errors", store_errors(q))
            .panel("overview-timeouts", timeouts(q)),
    )
}

fn logs(q: &Queries) -> Row {
    Row::new("Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("overview-retry-logs", retry_logs(q)),
    )
}

fn failed_ops(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Failed Operations")
        .query(
            q.get("materialize.persist.health.failed_ops")
                .legend("failed/s"),
        )
        .thresholds(threshold::errors(0.01, 1.0).build())
        .unit("ops")
        .no_value(not_collected())
        .build(0)
}

fn read_p99(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Read Latency (p99)")
        .query(
            q.get("materialize.persist.health.read_latency_p99")
                .legend("p99"),
        )
        .color_background()
        .thresholds(latency_ladder(0.5, 2.0))
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn write_p99(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Write Latency (p99)")
        .query(
            q.get("materialize.persist.health.write_latency_p99")
                .legend("p99"),
        )
        .color_background()
        .thresholds(latency_ladder(0.5, 2.0))
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn write_stalls(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Write Stalls")
        .query(
            q.get("materialize.persist.health.write_stalls")
                .legend("stalls/s"),
        )
        .thresholds(threshold::errors(0.01, 1.0).build())
        .unit("ops")
        .no_value(not_collected())
        .build(0)
}

fn compaction_failures(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Compaction Failures")
        .query(
            q.get("materialize.persist.health.compaction_failures")
                .legend("failed/s"),
        )
        .thresholds(threshold::errors(0.01, 1.0).build())
        .unit("ops")
        .no_value(not_collected())
        .build(0)
}

fn stored(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Data Stored")
        .query(q.get("materialize.persist.health.stored").legend("stored"))
        .shade(SHADE)
        .unit("bytes")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn read_write_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Read and Write Latency")
        .query(q.legended(
            "materialize.persist.latency.read_write",
            &["read p50", "read p99", "write p50", "write p99"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn round_trip(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Round-Trip Time by Process")
        .query(
            q.get("materialize.persist.latency.round_trip")
                .legend(PROCESS),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn failures_by_operation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed Operations by Type")
        .query(
            q.get("materialize.persist.failures.by_operation")
                .legend("{{op}}"),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

/// The one panel here whose series exist only once something has gone wrong:
/// the S3 client creates an error counter per code on its first error.
fn store_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Object Store Errors by Code")
        .query(
            q.get("materialize.persist.failures.store_errors")
                .legend("{{op}} {{code}}"),
        )
        .unit("ops")
        .min(0.0)
        .no_value(NoValue::Custom(
            "The object store has returned no errors. Always empty on Azure Blob, which reports no error codes."
                .to_string(),
        ))
        .build(0)
}

fn timeouts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Timeouts")
        .query(q.legended(
            "materialize.persist.failures.timeouts",
            &[
                "connecting",
                "waiting for first byte",
                "one attempt",
                "whole operation",
            ],
        ))
        .unit("ops")
        .min(0.0)
        .no_value(s3_only())
        .build(0)
}

fn retry_logs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Retrying Object Storage Operations")
        .query(q.logs("materialize.persist.logs.retries"))
        .no_value(NoValue::Custom(
            "No retried object storage calls were logged in this time range.".to_string(),
        ))
        .build(0)
}
