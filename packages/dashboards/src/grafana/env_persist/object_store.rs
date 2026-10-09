// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Object Store Internals tab: an on-premise object store, as it reports
//! itself.
//!
//! A cloud bucket does not fill up or lose a disk, and its provider's view is
//! on `infra-cloud`. An object store running on disks someone owns can do both,
//! and is the one deployment shape where the store publishes more about itself
//! than persist can see. Its rows render per store, on a discovered variable;
//! Ceph, run by Rook, is the first.
//!
//! # Scoped by a picker, not by the environment
//!
//! Ceph's series name a Rook namespace and nothing about Materialize, so
//! which Ceph cluster holds this environment's bucket is the reader's to say, on
//! `$cephNamespace`. On the usual single cluster, "All" is already right. The
//! gateway panels sum every client of the store, Materialize among them.
//!
//! # Why these signals
//!
//! Status answers whether the store can take persist's writes right now:
//! Ceph's own verdict, disks down, space left, and data at risk. The gateway
//! rows are the server side of the calls on Operations — the gap between the
//! two is the network in between. Capacity and Disks are the slow failures:
//! a store filling unevenly stops writes at its fullest disk.

use mzmon_lib::grafana::generated::{dashboardv2, stat::BigValueGraphMode};
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::variable::extra;
use mzmon_lib::grafana::{palette, threshold};

use super::{latency_ladder, theme, zero_is_healthy};
use crate::grafana::dependency::{ratio_ladder, value_map};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::OBJECT_STORE.shade;

/// The variable every Ceph row renders on.
pub(crate) const CEPH_DETECTED: &str = extra::CEPH_DETECTED;

/// What `$cephDetected` holds when a Ceph manager is scraped: its `app` label.
///
/// Not `.+`: a variable whose query returned nothing can reach the row
/// condition as `undefined`, which `.+` matches.
pub(crate) const CEPH: &str = "ceph-mgr";

/// Every store this tab has rows for, as one alternation.
///
/// The fallback renders on the negation of this, so a store added without
/// being added here would leave the fallback on screen beside its rows.
pub(crate) fn known_stores() -> String {
    CEPH.to_string()
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        status(q),
        health_checks(q),
        gateway(q),
        capacity(q),
        disks(q),
        no_store_row(),
    ]
}

fn ceph_row(title: &str) -> Row {
    Row::new(title).only_when_variable(CEPH_DETECTED, CEPH)
}

/// What a Ceph panel shows when its row rendered and the series is absent.
///
/// The row rendering means the manager is scraped, so the likely cause of a
/// missing series is the per-node exporter not being scraped.
fn not_reported() -> NoValue {
    NoValue::Custom(
        "Ceph has not reported this. The gateway and disk counters need rook-ceph-exporter scraped as well as the manager."
            .to_string(),
    )
}

/// Why the tab has no store rows on it.
pub(crate) const NO_STORE: &str = "**The object store is not reporting its own metrics.**\n\n\
     This tab shows what an object store running on disks you own says about itself: its \
     health, its free space, its disks, and the server side of every request persist makes. It \
     renders for **Ceph run by Rook**, whose S3-compatible gateway (RGW) can hold the persist \
     bucket.\n\n\
     - **On Rook Ceph**, scrape the manager and the per-node exporter. Setting \
       `monitoring.enabled: true` on the `CephCluster` makes Rook create a `ServiceMonitor` for \
       each; `deploy/examples/monitoring/` in the Rook repository has both as standalone \
       manifests, which leave out the alert rules that setting also installs.\n\
     - **On S3, GCS or Azure Blob**, the bucket cannot fill up or lose a disk, and what the \
       provider reports about it is on the **Infrastructure Cloud Provider** dashboard.\n\n\
     The other tabs here need none of this: they are persist's own measurement of the store, \
     and work on every one.";

fn no_store_row() -> Row {
    Row::new("No Object Store Metrics")
        .only_unless_variable(CEPH_DETECTED, known_stores())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "object-store-none-detected",
                Panel::text("No Object Store Metrics", NO_STORE)
                    .description("Why this tab has no object store rows on it.")
                    .build(0),
            ),
        )
}

// --- Ceph ----------------------------------------------------------------

fn status(q: &Queries) -> Row {
    ceph_row("Ceph: Status").hide_header().grid(
        AutoGrid::new(7)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("ceph-health", health(q))
            .panel("ceph-osds-down", osds_down(q))
            .panel("ceph-raw-used", raw_used(q))
            .panel("ceph-bucket-pool-available", bucket_pool_available(q))
            .panel("ceph-objects-degraded", objects_degraded(q))
            .panel("ceph-rgw-failure-ratio", rgw_failure_ratio(q))
            .panel("ceph-version", version(q)),
    )
}

fn health_checks(q: &Queries) -> Row {
    ceph_row("Ceph: Health Checks").grid(
        AutoGrid::new(1)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-health-checks", checks(q)),
    )
}

fn gateway(q: &Queries) -> Row {
    ceph_row("Ceph: Object Gateway").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-rgw-requests", rgw_requests(q))
            .panel("ceph-rgw-latency", rgw_latency(q))
            .panel("ceph-rgw-throughput", rgw_throughput(q))
            .panel("ceph-rgw-queue", rgw_queue(q)),
    )
}

fn capacity(q: &Queries) -> Row {
    ceph_row("Ceph: Capacity").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-capacity-raw", capacity_raw(q))
            .panel("ceph-capacity-pools", capacity_pools(q)),
    )
}

fn disks(q: &Queries) -> Row {
    ceph_row("Ceph: Disks").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-osd-utilization", osd_utilization(q))
            .panel("ceph-osd-latency", osd_latency(q))
            .panel("ceph-placement-groups", placement_groups(q)),
    )
}

/// Ceph's verdict, as the word Ceph itself uses.
fn health(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Ceph Health")
        .query(q.get("infra.ceph.health.status").legend("health"))
        .color_background()
        .mappings(vec![
            value_map(0.0, "HEALTH_OK", palette::tri_health::HEALTHY, 1),
            value_map(1.0, "HEALTH_WARN", palette::tri_health::DEGRADED, 2),
            value_map(2.0, "HEALTH_ERR", palette::tri_health::UNHEALTHY, 3),
        ])
        .thresholds(
            threshold::Ladder::new(palette::tri_health::HEALTHY)
                .step(1.0, palette::tri_health::DEGRADED)
                .step(2.0, palette::tri_health::UNHEALTHY)
                .build(),
        )
        .no_value(not_reported())
        .build(0)
}

fn osds_down(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Disks (OSDs) Down")
        .query(q.get("infra.ceph.health.osds_down").legend("down"))
        .thresholds(threshold::errors(1.0, 2.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_reported())
        .build(0)
}

fn raw_used(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Raw Capacity Used")
        .query(q.get("infra.ceph.health.raw_used").legend("used"))
        .color_background()
        .thresholds(ratio_ladder(0.75, 0.85))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_reported())
        .build(0)
}

fn bucket_pool_available(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Bucket Space Available")
        .query(
            q.get("infra.ceph.health.bucket_pool_available")
                .legend("{{name}}"),
        )
        .shade(SHADE)
        .unit("bytes")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No RGW bucket data pool in the selected Ceph cluster.".to_string(),
        ))
        .build(0)
}

fn objects_degraded(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Degraded Objects")
        .query(
            q.get("infra.ceph.health.objects_degraded")
                .legend("degraded"),
        )
        .thresholds(threshold::errors(1.0, 1000.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_reported())
        .build(0)
}

fn rgw_failure_ratio(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Gateway Failed Requests")
        .query(
            q.get("infra.ceph.health.rgw_failure_ratio")
                .legend("failed"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .color_background()
        .thresholds(ratio_ladder(0.05, 0.25))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_reported())
        .build(0)
}

fn version(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Ceph Version")
        .query(
            q.get("infra.ceph.health.version")
                .legend("{{ceph_version}}"),
        )
        .graph_mode(BigValueGraphMode::None)
        .value_size(20.0)
        .reduce_fields("/^ceph_version$/")
        .transformations(vec![transform::labels_to_fields(&[])])
        .shade(SHADE)
        .no_value(not_reported())
        .build(0)
}

/// A current fact, so an instant query.
fn checks(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Active Health Checks")
        .query(q.get("infra.ceph.health.checks").table_format())
        .transformations(vec![
            transform::organize_full(
                &["Time", "Value"],
                &["severity", "name"],
                &[("severity", "Severity"), ("name", "Check")],
            ),
            transform::sort_by("Severity", true),
        ])
        .no_value(NoValue::Custom(
            "No health checks raised: Ceph reports HEALTH_OK.".to_string(),
        ))
        .build(0)
}

fn rgw_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Gateway Requests by Operation")
        .query(q.legended(
            "infra.ceph.rgw.requests",
            &["PUT", "GET", "DELETE", "failed"],
        ))
        .unit("reqps")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn rgw_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Gateway Latency by Operation (Mean)")
        .query(q.legended("infra.ceph.rgw.latency", &["PUT", "GET", "DELETE"]))
        .unit("s")
        .min(0.0)
        .thresholds(latency_ladder(0.1, 1.0))
        .no_value(not_reported())
        .build(0)
}

fn rgw_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Gateway Throughput")
        .query(q.legended("infra.ceph.rgw.throughput", &["written", "read"]))
        .unit("Bps")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn rgw_queue(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Gateway Requests in Progress")
        .query(q.legended(
            "infra.ceph.rgw.queue",
            &["active on {{instance_id}}", "queued on {{instance_id}}"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn capacity_raw(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Raw Capacity")
        .query(q.legended("infra.ceph.capacity.raw", &["used", "total"]))
        .unit("bytes")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn capacity_pools(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Stored Data by Pool")
        .query(q.get("infra.ceph.capacity.pools").legend("{{name}}"))
        .unit("bytes")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn osd_utilization(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("OSD Utilization")
        .query(
            q.get("infra.ceph.osd.utilization")
                .legend("{{ceph_daemon}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.75, 0.85))
        .no_value(not_reported())
        .build(0)
}

fn osd_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("OSD Commit Latency")
        .query(q.get("infra.ceph.osd.latency").legend("{{ceph_daemon}}"))
        .unit("s")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn placement_groups(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Placement Groups Not Healthy")
        .query(q.legended(
            "infra.ceph.osd.placement_groups",
            &["not active", "degraded", "undersized", "not clean"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}
