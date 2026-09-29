// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Compaction tab: the background work that keeps reads fast and storage
//! small, and the writes it competes with.
//!
//! Every write uploads a new, small data file. Compaction merges them into
//! larger ones and deletes what they replaced, so when it falls behind two
//! things degrade slowly and together: reads get slower as the files pile up,
//! and the bucket grows because nothing is deleted. Neither shows up as an
//! error, which is why this tab reads queue wait and busy time rather than only
//! failures.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::Panel;

use super::{PROCESS, not_collected};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![compaction(q), writes(q)]
}

fn compaction(q: &Queries) -> Row {
    Row::new("Compaction").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("compaction-outcomes", outcomes(q))
            .panel("compaction-queue-wait", queue_wait(q))
            .panel("compaction-busy", busy(q)),
    )
}

fn writes(q: &Queries) -> Row {
    Row::new("Writes").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("writes-amplification", amplification(q))
            .panel("writes-stalls", stalls(q)),
    )
}

fn outcomes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Compaction Outcomes")
        .query(q.legended(
            "materialize.persist.compaction.outcomes",
            &[
                "requested",
                "started",
                "applied",
                "failed",
                "timed out",
                "dropped",
            ],
        ))
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn queue_wait(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Queue Wait (mean)")
        .query(
            q.get("materialize.persist.compaction.queue_wait")
                .legend("waiting"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn busy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Compactions Running by Process")
        .query(q.get("materialize.persist.compaction.busy").legend(PROCESS))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn amplification(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("New Data and Compaction Rewrites")
        .query(q.legended(
            "materialize.persist.writes.amplification",
            &["new data", "compaction rewrites"],
        ))
        .unit("Bps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn stalls(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write Stalls by Process")
        .query(
            q.get("materialize.persist.writes.stalls_by_process")
                .legend(PROCESS),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}
