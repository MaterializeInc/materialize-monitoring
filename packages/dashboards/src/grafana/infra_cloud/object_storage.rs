// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Object Storage tab: each bucket, as its provider bills it.
//!
//! Provider collection for object storage earns its place on capacity and waste
//! rather than health — the design's finding, and the reason there is no S3
//! row: S3's free metrics are size and object count, both on the shared row,
//! and its request metrics are opt-in and billed. The client's view of health is
//! on Materialize Persist and is better than any provider's.
//!
//! The one panel that is not provider data is the point of pulling at all:
//! _Bucket Size_ draws the bytes Materialize accounts for beside the bytes the
//! provider bills. The gap is storage nothing uses, and on the reference GCS
//! install it was a persist bucket holding over a hundred times its live size
//! in soft-deleted objects.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::Panel;

use super::{AZURE, BUCKET_LEGENDS, GCP, PROVIDERS, known_providers, no_provider_row, not_pulled};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        stored(q),
        gcs(q),
        blob(q),
        no_provider_row("bucket-none-detected"),
    ]
}

fn stored(q: &Queries) -> Row {
    Row::new("Stored Data")
        .only_when_variable(PROVIDERS, known_providers())
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("bucket-bytes", bytes(q))
                .panel("bucket-objects", objects(q)),
        )
}

fn gcs(q: &Queries) -> Row {
    Row::new("GCS: Reclaimable Data")
        .only_when_variable(PROVIDERS, GCP)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("gcs-bytes-by-type", gcs_bytes(q))
                .panel("gcs-objects-by-type", gcs_objects(q)),
        )
}

fn blob(q: &Queries) -> Row {
    Row::new("Azure Blob: Availability and Latency")
        .only_when_variable(PROVIDERS, AZURE)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("blob-availability", blob_availability(q))
                .panel("blob-latency", blob_latency(q)),
        )
}

fn bytes(q: &Queries) -> dashboardv2::PanelKind {
    let [aws, gcp, azure] = BUCKET_LEGENDS;
    Panel::timeseries("Bucket Size")
        .query(q.legended(
            "infra.cloud.bucket.bytes",
            &[aws, gcp, azure, "referenced by Materialize"],
        ))
        .unit("bytes")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn objects(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Object Count")
        .query(q.legended("infra.cloud.bucket.objects", &BUCKET_LEGENDS))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcs_bytes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Bytes by Object State")
        .query(
            q.get("infra.cloud.gcs.bytes_by_type")
                .legend("{{bucket_name}} {{type}}"),
        )
        .unit("bytes")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcs_objects(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Objects by Object State")
        .query(
            q.get("infra.cloud.gcs.objects_by_type")
                .legend("{{bucket_name}} {{type}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn blob_availability(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Availability")
        .query(
            q.get("infra.cloud.blob.availability")
                .legend("{{resourceName}}"),
        )
        .unit("percentunit")
        .max(1.0)
        .no_value(not_pulled())
        .build(0)
}

fn blob_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Request Latency")
        .query(q.legended(
            "infra.cloud.blob.latency",
            &["server {{resourceName}}", "end to end {{resourceName}}"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}
