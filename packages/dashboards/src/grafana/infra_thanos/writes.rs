// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Writes tab: remote writes into the receivers, and what the receivers do
//! with each sample until it is safely in object storage.
//!
//! Rows follow a sample: the request that brought it, the database that took
//! it, the peers it was replicated to, and the upload that made it durable.
//! Every panel reads the receivers alone, so none honours the component picker.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::not_collected;
use super::overview::SAMPLES_PER_SECOND;
use crate::grafana::dependency::zero_is_healthy;
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![requests(q), database(q), replication(q), uploads(q)]
}

fn requests(q: &Queries) -> Row {
    Row::new("Requests").grid(
        AutoGrid::new(2)
            .panel("writes-requests", write_requests(q))
            .panel("writes-latency", write_latency(q)),
    )
}

fn database(q: &Queries) -> Row {
    Row::new("Database").grid(
        AutoGrid::new(3)
            .panel("writes-head-series", head_series(q))
            .panel("writes-rejected", samples_rejected(q))
            .panel("writes-database-failures", database_failures(q)),
    )
}

fn replication(q: &Queries) -> Row {
    Row::new("Replication").grid(
        AutoGrid::new(4)
            .panel("writes-replication", replication_outcomes(q))
            .panel("writes-forward", forward_outcomes(q))
            .panel("writes-hashring", hashring(q))
            .panel("writes-pending", pending(q)),
    )
}

fn uploads(q: &Queries) -> Row {
    Row::new("Uploads and Configuration").grid(
        AutoGrid::new(3)
            .panel("writes-upload-age", super::storage::upload_age(q))
            .panel("writes-upload-failures", upload_failures(q))
            .panel("writes-config-failed", config_failed(q)),
    )
}

/// What a receiver panel shows when no receiver is reporting.
fn no_receivers() -> NoValue {
    NoValue::Custom("No receiver is reporting".to_string())
}

fn write_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write Requests by Status")
        .query(q.get("infra.thanos.write.requests").legend("{{code}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No remote writes reached the receivers".to_string(),
        ))
        .build(0)
}

fn write_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write Latency")
        .query(q.legended("infra.thanos.write.latency", &["p50", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No remote writes reached the receivers".to_string(),
        ))
        .build(0)
}

fn head_series(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Series in Memory")
        .query(q.get("infra.thanos.write.head_series").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn samples_rejected(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Samples Rejected by the Database")
        .query(q.legended(
            "infra.thanos.write.samples_rejected",
            &["out of order", "out of bounds", "too old"],
        ))
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn database_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Database Failures")
        .query(q.legended(
            "infra.thanos.write.database_failures",
            &[
                "compaction",
                "truncation",
                "reload or checkpoint",
                "WAL corruption",
            ],
        ))
        .unit("suffix:/s")
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn replication_outcomes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Replication")
        .query(q.get("infra.thanos.write.replication").legend("{{result}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn forward_outcomes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Forwarded Requests")
        .query(q.get("infra.thanos.write.forward").legend("{{result}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn hashring(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Hashring Members")
        .query(q.get("infra.thanos.write.hashring").legend("{{pod}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn pending(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Writes in Progress")
        .query(q.get("infra.thanos.write.pending").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn upload_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Block Upload Failures")
        .query(
            q.get("infra.thanos.write.upload_failures")
                .legend("{{pod}}"),
        )
        .unit("suffix:/s")
        .min(0.0)
        .no_value(no_receivers())
        .build(0)
}

fn config_failed(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Hashring Reload Failures")
        .query(
            q.get("infra.thanos.write.config_failed")
                .legend("receivers"),
        )
        .thresholds(threshold::errors(1.0, 2.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_collected())
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
        assert_eq!(assembled.elements.len(), 12);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }
}
