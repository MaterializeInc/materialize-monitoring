// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The State and Cleanup tab: how much persist keeps in the database, and
//! whether cleanup keeps up.
//!
//! Every commit appends a state version; cleanup truncates the versions no
//! reader needs. In steady state the two rates are equal and the table persist
//! writes to stays the same size. When a reader holds an old version — a
//! subscription that is behind, a replica that went away without releasing its
//! lease — cleanup stops at that version for that shard, and the table grows.
//! On PostgreSQL that growth is also autovacuum's workload, which is the
//! slow-building failure this tab exists to catch before it reaches commit
//! latency or the database's disk.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::Panel;

use super::{not_collected, theme};
use crate::grafana::queries::Queries;

const SHADE: &str = theme::STATE.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![versions(q), cleanup(q)]
}

fn versions(q: &Queries) -> Row {
    Row::new("State Versions").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("state-written-deleted", written_and_deleted(q))
            .panel("state-live", live(q))
            .panel("state-held", held(q)),
    )
}

fn cleanup(q: &Queries) -> Row {
    Row::new("Cleanup").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("state-gc-runs", gc_runs(q))
            .panel("state-lease-timeouts", lease_timeouts(q)),
    )
}

fn written_and_deleted(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Versions Written and Deleted")
        .query(q.legended(
            "materialize.consensus.state.versions",
            &["written", "deleted"],
        ))
        .unit("suffix:versions/s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn live(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Versions Stored")
        .query(q.get("materialize.consensus.state.live").legend("versions"))
        .shade(SHADE)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn held(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Versions Held by Collection")
        .query(
            q.get("materialize.consensus.state.held")
                .legend("{{shard}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        // Shard ids are long and opaque; the legend is a lookup key for the SQL
        // in the description rather than something to read at a glance.
        .no_value(not_collected())
        .build(0)
}

fn gc_runs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Cleanup Runs")
        .query(q.legended(
            "materialize.consensus.gc.runs",
            &["started", "finished", "skipped"],
        ))
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn lease_timeouts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Expired Reader Leases")
        .query(
            q.get("materialize.consensus.gc.lease_timeouts")
                .legend("expired/s"),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}
