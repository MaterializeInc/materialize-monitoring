// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Storage tab: what this environment's data in object storage is for.
//!
//! This is Materialize's own accounting, and it answers a question the bucket
//! cannot: of the bytes stored, which are current data and which are kept only
//! for a reader still looking at an older version. The bucket's own size is the
//! provider's number, on `infra-cloud`, and for GCS on Object Store Internals
//! too. The gap between the two is soft-deleted objects, noncurrent versions,
//! uploads not yet committed and orphaned data, which this tab cannot see.
//!
//! Every `mz_persist_shard_usage_*` total takes `max by (shard)` first, because
//! each process reports every shard it has open. The tab reads three of the
//! five families: environmentd never sets `not_leaked_not_referenced_bytes` or
//! `leaked_bytes`, which only `persistcli inspect` computes.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::Panel;

use super::{not_collected, theme};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::STORAGE.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![summary(q), stored(q), collections(q)]
}

/// The one number: what the environment needs.
fn summary(q: &Queries) -> Row {
    Row::new("Storage Summary").hide_header().grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Short)
            .panel("storage-current", current(q)),
    )
}

fn stored(q: &Queries) -> Row {
    Row::new("Stored Data").grid(
        AutoGrid::new(1)
            .column_width(ColumnWidth::Wide)
            .panel("storage-by-state", by_state(q)),
    )
}

fn collections(q: &Queries) -> Row {
    Row::new("By Object").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .row_height(RowHeight::Tall)
            .panel("storage-largest", largest(q))
            .panel("storage-held", held(q)),
    )
}

fn by_state(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Stored Data by State")
        .query(q.legended(
            "materialize.persist.usage.by_state",
            &["current data", "current rollups", "held for readers"],
        ))
        .unit("bytes")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn current(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Data Stored")
        .query(q.get("materialize.persist.health.stored").legend("stored"))
        .shade(SHADE)
        .unit("bytes")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

/// A current fact, so an instant query: over a range, each shard would repeat
/// once per step and the table would be hundreds of near-identical rows.
fn largest(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Largest Objects")
        .query(q.get("materialize.persist.usage.largest").table_format())
        .transformations(vec![transform::organize_full(
            &["Time"],
            &["shard", "Value"],
            &[("Value", "stored")],
        )])
        .unit("bytes")
        .no_value(not_collected())
        .build(0)
}

fn held(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Data Held for Readers by Object")
        .query(q.get("materialize.persist.usage.held").legend("{{shard}}"))
        .unit("bytes")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}
