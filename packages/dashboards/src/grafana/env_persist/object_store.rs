// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Object Store Internals tab: the object store, as it reports itself.
//!
//! Each store's rows render on a discovered variable, one section per store,
//! with one fallback for a tab where none was found.
//!
//! - **Ceph, run by Rook.** An object store running on disks someone owns can
//!   fill up or lose a disk, and is the one deployment shape where the store
//!   publishes more about itself than persist can see.
//! - **GCS, as Cloud Monitoring reports it.** A cloud bucket does neither, so
//!   its section is about what the bucket holds and is billed for: its live
//!   data beside the environment's own, how that changes day to day, and the
//!   soft-deleted and noncurrent objects billed on top. The same families are
//!   on `infra-cloud`, for every bucket on the cluster; here they sit beside
//!   the environment they serve.
//!
//! S3 and Azure Blob have no section. Their provider's view stays on
//! `infra-cloud`, and the fallback says so.
//!
//! # Scoped by a picker, not by the environment
//!
//! Ceph's series name a Rook namespace and nothing about Materialize, so
//! which Ceph cluster holds this environment's bucket is the reader's to say, on
//! `$cephNamespace`. It is single-select, and on the usual single cluster
//! Grafana's first choice is already right. The
//! gateway panels sum every client of the store, Materialize among them.
//!
//! GCS's series name a bucket and nothing about Materialize either, so the
//! reader picks buckets on `$gcsBucket`. It opens on every bucket, for the
//! reason [`variable::gcs_buckets`] gives. The environment's own tile, beside
//! each bucket's live bytes, is what identifies its bucket among them: the two
//! read the same.
//!
//! Each picker is a section variable on the one row that holds its store's
//! rows, so it renders under that header and hides with it, for the reason the
//! metadata database's CNPG section gives.
//!
//! # Why these signals
//!
//! Status answers whether the store can take persist's writes right now:
//! Ceph's own verdict, disks down, space left, and data at risk. The gateway
//! rows are the server side of the calls on Operations — the gap between the
//! two is the network in between. Capacity and Disks are the slow failures:
//! a store filling unevenly stops writes at its fullest disk.
//!
//! GCS is pulled for size and object count only. Its request counts are a
//! per-minute delta the pull cannot read whole, which the chart's values
//! explain, and persist's own count of the same calls is on Operations.

use mzmon_lib::grafana::generated::{
    dashboardv2,
    stat::{BigValueGraphMode, BigValueTextMode},
};
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::query::PanelQuery;
use mzmon_lib::grafana::variable::{self, extra};
use mzmon_lib::grafana::{palette, threshold};

use super::{latency_ladder, theme, zero_is_healthy};
use crate::grafana::dependency::{ratio_ladder, value_map};
use crate::grafana::field_override;
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

/// The variable every GCS row renders on.
pub(crate) const GCS_DETECTED: &str = extra::GCS_DETECTED;

/// What `$gcsDetected` holds when the GCS pull is up: its scrape's `instance`.
pub(crate) const GCS: &str = "gcs";

/// Every store this tab has rows for, as the detection each one renders on.
///
/// The fallback renders when none of these matches, so a store added without
/// being added here would leave the fallback on screen beside its rows.
pub(crate) fn known_stores() -> Vec<(&'static str, String)> {
    vec![
        (CEPH_DETECTED, CEPH.to_string()),
        (GCS_DETECTED, GCS.to_string()),
    ]
}

/// The title of the section holding every Ceph row.
pub(crate) const CEPH_SECTION: &str = "Ceph";

/// The title of the section holding every GCS row.
pub(crate) const GCS_SECTION: &str = "GCS";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![ceph_section(q), gcs_section(q), no_store_row()]
}

/// Every Ceph row, in one section that renders only where Ceph is scraped and
/// carries the Ceph cluster picker its rows read.
fn ceph_section(q: &Queries) -> Row {
    Row::section(
        CEPH_SECTION,
        vec![
            status(q),
            health_checks(q),
            gateway(q),
            capacity(q),
            disks(q),
        ],
    )
    .only_when_variable(CEPH_DETECTED, CEPH)
    .variables([variable::ceph_namespaces()])
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

/// Every GCS row, in one section that renders only where the GCS pull runs
/// and carries the bucket picker its rows read.
fn gcs_section(q: &Queries) -> Row {
    Row::section(
        GCS_SECTION,
        vec![gcs_stored(q), gcs_growth(q), gcs_reclaimable(q)],
    )
    .only_when_variable(GCS_DETECTED, GCS)
    .variables([variable::gcs_buckets()])
}

/// What a GCS panel shows when its row rendered and the series is absent.
///
/// The row rendering means the pull is up, so the likely causes are a bucket
/// not measured yet or a pull the provider is refusing.
fn not_pulled() -> NoValue {
    NoValue::Custom(
        "Cloud Monitoring has not reported the selected buckets. Google measures a bucket once a day, so a new one has nothing for up to a day; Collection on the Infrastructure Cloud Provider dashboard says whether the pull is working."
            .to_string(),
    )
}

/// Why the tab has no store rows on it.
pub(crate) const NO_STORE: &str = "**The object store is not reporting its own metrics.**\n\n\
     This tab shows what the object store says about itself. It renders for **Ceph run by \
     Rook**, whose S3-compatible gateway (RGW) can hold the persist bucket: its health, its free \
     space, its disks, and the server side of every request persist makes. It also renders for \
     **GCS**, as Cloud Monitoring reports each bucket: what you are billed for beside what \
     Materialize stores.\n\n\
     - **On Rook Ceph**, scrape the manager and the per-node exporter. Setting \
       `monitoring.enabled: true` on the `CephCluster` makes Rook create a `ServiceMonitor` for \
       each; `deploy/examples/monitoring/` in the Rook repository has both as standalone \
       manifests, which leave out the alert rules that setting also installs.\n\
     - **On GCS**, turn on the gateway's Cloud Monitoring pull, list the bucket under \
       `pipeline.metrics.provider.gcp.gcs.buckets` in the `materialize-monitoring` chart's \
       values, and give the gateway's service account `roles/monitoring.viewer`. The Cloud \
       Provider Metrics page of the documentation has the details.\n\
     - **On S3 or Azure Blob**, the bucket cannot fill up or lose a disk, and what the provider \
       reports about it is on the **Infrastructure Cloud Provider** dashboard.\n\n\
     The other tabs here need none of this: they are persist's own measurement of the store, \
     and work on every one.";

fn no_store_row() -> Row {
    Row::new("No Object Store Metrics")
        .only_unless_variables(known_stores())
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
    Row::new("Status").hide_header().grid(
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
    Row::new("Health Checks").grid(
        AutoGrid::new(1)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-health-checks", checks(q)),
    )
}

fn gateway(q: &Queries) -> Row {
    Row::new("Object Gateway").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-rgw-requests", rgw_requests(q))
            .panel("ceph-rgw-latency", rgw_latency(q))
            .panel("ceph-rgw-throughput", rgw_throughput(q))
            .panel("ceph-rgw-queue", rgw_queue(q)),
    )
}

fn capacity(q: &Queries) -> Row {
    Row::new("Capacity").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ceph-capacity-raw", capacity_raw(q))
            .panel("ceph-capacity-pools", capacity_pools(q)),
    )
}

fn disks(q: &Queries) -> Row {
    Row::new("Disks").grid(
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

// --- GCS -----------------------------------------------------------------
//
// Google measures a bucket once a day, so over the dashboard's six hours every
// series is a flat line. The tiles and charts that draw history draw the last
// two weeks instead, which Grafana states in each panel's header. The donuts
// are a single instant and need no range.

/// How far back the GCS panels with history draw.
const GCS_HISTORY: &str = "14d";

/// One full-width panel each, so a tile per bucket has room for its name.
fn gcs_stored(q: &Queries) -> Row {
    Row::new("Stored Data").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Short)
            .panel("gcs-live-bytes", gcs_live_bytes(q))
            .panel("gcs-live-objects", gcs_live_objects(q))
            .panel("gcs-object-size", gcs_object_size(q)),
    )
}

fn gcs_growth(q: &Queries) -> Row {
    Row::new("Growth").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("gcs-live-bytes-change", gcs_live_bytes_change(q))
            .panel("gcs-live-objects-change", gcs_live_objects_change(q)),
    )
}

fn gcs_reclaimable(q: &Queries) -> Row {
    Row::new("Reclaimable Data").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("gcs-bytes-by-type", gcs_bytes_by_type(q))
            .panel("gcs-objects-by-type", gcs_objects_by_type(q)),
    )
}

/// A tile per bucket and one for the environment: the bucket whose live bytes
/// read the same as the environment's is the environment's.
fn gcs_live_bytes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Live Data")
        .query(
            q.legended(
                "infra.cloud.gcs.env.live_bytes",
                &["{{bucket_name}}", "referenced by this environment"],
            )
            .time_from(GCS_HISTORY),
        )
        .shade(SHADE)
        .text_mode(BigValueTextMode::ValueAndName)
        .min_max_per_field()
        .unit("bytes")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcs_live_objects(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Live Objects")
        .query(
            q.get("infra.cloud.gcs.env.live_objects")
                .legend("{{bucket_name}}")
                .time_from(GCS_HISTORY),
        )
        .shade(SHADE)
        .text_mode(BigValueTextMode::ValueAndName)
        .min_max_per_field()
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcs_object_size(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Average Object Size")
        .query(
            q.get("infra.cloud.gcs.env.object_size")
                .legend("{{bucket_name}}")
                .time_from(GCS_HISTORY),
        )
        .shade(SHADE)
        .text_mode(BigValueTextMode::ValueAndName)
        .min_max_per_field()
        .unit("bytes")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

/// One bar a day, stacked, so a day's bar is the selected buckets' total
/// change and a shrinking bucket draws below the axis.
fn daily_change(title: &str, query: PanelQuery, unit: &str) -> dashboardv2::PanelKind {
    Panel::timeseries(title)
        .query(
            query
                .legend("{{bucket_name}}")
                .time_from(GCS_HISTORY)
                .min_interval("1d"),
        )
        .custom(serde_json::json!({
            "drawStyle": "bars",
            "fillOpacity": 80,
            "lineWidth": 1,
            "stacking": { "mode": "normal", "group": "A" },
        }))
        .unit(unit)
        .no_value(not_pulled())
        .build(0)
}

fn gcs_live_bytes_change(q: &Queries) -> dashboardv2::PanelKind {
    daily_change(
        "Live Data Change per Day",
        q.get("infra.cloud.gcs.env.live_bytes_change"),
        "bytes",
    )
}

fn gcs_live_objects_change(q: &Queries) -> dashboardv2::PanelKind {
    daily_change(
        "Live Objects Change per Day",
        q.get("infra.cloud.gcs.env.live_objects_change"),
        "short",
    )
}

/// Google's state names end in `-object`, which the legend does not need.
fn object_state_names() -> dashboardv2::TransformationKind {
    transform::raw(
        "renameByRegex",
        serde_json::json!({ "regex": "(.*)-object", "renamePattern": "$1" }),
    )
}

/// One colour per state on both donuts: live as healthy, and the two states
/// billed without being read warmer.
fn object_state_colors() -> Vec<dashboardv2::FieldConfigSourceOverridesItem> {
    [
        ("live", palette::tri_health::HEALTHY),
        ("noncurrent", palette::tri_health::DEGRADED),
        ("soft-deleted", palette::tri_health::UNHEALTHY),
    ]
    .into_iter()
    .map(|(state, color)| {
        field_override::by_name(state)
            .property(
                "color",
                serde_json::json!({ "mode": "fixed", "fixedColor": color }),
            )
            .build()
    })
    .collect()
}

fn gcs_bytes_by_type(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Billed Data by Object State")
        .query(
            q.get("infra.cloud.gcs.env.bytes_by_type")
                .legend("{{type}}"),
        )
        .transformations(vec![object_state_names()])
        .overrides(object_state_colors())
        .legend_percent()
        .unit("bytes")
        .no_value(not_pulled())
        .build(0)
}

fn gcs_objects_by_type(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Objects by Object State")
        .query(
            q.get("infra.cloud.gcs.env.objects_by_type")
                .legend("{{type}}"),
        )
        .transformations(vec![object_state_names()])
        .overrides(object_state_colors())
        .legend_percent()
        .unit("short")
        .no_value(not_pulled())
        .build(0)
}
