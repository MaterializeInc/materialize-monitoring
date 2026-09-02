// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Memory & Swap tab: what is resident, what spilled, and what was killed.
//!
//! The row order is the escalation path. Available memory and pressure say
//! whether the machine is short; the breakdown says what is holding it; swap and
//! reclaim say what the kernel is doing about it; OOM kills say it already lost.
//!
//! **An OOM kill is the one signal here that is never ambiguous.** Everything
//! else on this tab can look alarming on a healthy machine — Linux is supposed to
//! use free memory for cache — but a kill means a process was terminated for
//! asking, and on a Materialize node that process is usually a replica.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::theme;
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![headroom(q), breakdown(q), swap(q), paging(q)]
}

fn headroom(q: &Queries) -> Row {
    Row::new("Headroom").grid(
        AutoGrid::new(3)
            .panel("memory-available", available(q))
            .panel("memory-pressure", pressure(q))
            .panel("memory-oom-kills", oom_kills(q)),
    )
}

fn breakdown(q: &Queries) -> Row {
    Row::new("What Is Holding It").grid(
        AutoGrid::new(2)
            .panel("memory-breakdown", memory_breakdown(q))
            .panel("memory-kernel", kernel(q)),
    )
}

fn swap(q: &Queries) -> Row {
    Row::new("Swap").grid(
        AutoGrid::new(2)
            .panel("swap-used", swap_used(q))
            .panel("swap-activity", swap_activity(q)),
    )
}

fn paging(q: &Queries) -> Row {
    Row::new("Paging and Reclaim").grid(
        AutoGrid::new(2)
            .panel("memory-page-faults", page_faults(q))
            .panel("memory-reclaim", reclaim(q)),
    )
}

fn available(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Available")
        .query(q.get("node.memory.available.ratio").legend("available"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.1, 0.2).build())
        .shade(theme::MEMORY.shade)
        .build(0)
}

fn pressure(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Pressure")
        .query(q.legended("node.memory.pressure", &["waiting", "stalled"]))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .build(0)
}

fn oom_kills(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("OOM Kills")
        .query(q.get("node.memory.oom_kills").legend("kills"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No OOM kills — nothing has been terminated for memory.".to_string(),
        ))
        .shade(theme::MEMORY.shade)
        .build(0)
}

fn memory_breakdown(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Breakdown")
        .query(q.legended(
            "node.debug.memory.breakdown",
            &["total", "used", "cache and buffers", "free"],
        ))
        .unit("bytes")
        .min(0.0)
        .build(0)
}

fn kernel(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Kernel Memory")
        .query(q.legended(
            "node.debug.memory.kernel",
            &["slab", "reclaimable", "unreclaimable", "committed"],
        ))
        .unit("bytes")
        .min(0.0)
        .build(0)
}

fn swap_used(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Swap Used")
        .query(q.get("node.swap.used.ratio").legend("used"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.1, 1.0, 0.2).build())
        .shade(theme::MEMORY.shade)
        .build(0)
}

fn swap_activity(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Swap Activity")
        .query(q.legended("node.swap.activity", &["swapped in", "swapped out"]))
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No swap activity — nothing is being paged to disk.".to_string(),
        ))
        .build(0)
}

fn page_faults(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Page Faults")
        .query(q.legended("node.debug.memory.page_faults", &["minor", "major (disk)"]))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn reclaim(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Page Reclaim")
        .query(q.legended(
            "node.debug.memory.reclaim",
            &[
                "scanned (background)",
                "scanned (direct)",
                "reclaimed (background)",
                "reclaimed (direct)",
            ],
        ))
        .unit("cps")
        .min(0.0)
        .build(0)
}
