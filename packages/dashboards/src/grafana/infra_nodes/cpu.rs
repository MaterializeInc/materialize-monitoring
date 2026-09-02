// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The CPU tab: where the processors' time went, and who waited for it.
//!
//! Ordered as the question narrows. The first row says whether the machine is
//! busy; the second says doing what; the third says whether anything was made to
//! wait, which is the part that actually hurts a query.
//!
//! **Utilization and pressure disagree usefully.** A node pinned at 100% with no
//! pressure is a node doing exactly as much work as it has cores for, which is
//! efficient rather than sick. Pressure above zero means threads were runnable
//! and had nowhere to run — that is the number worth escalating.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::Panel;
use mzmon_lib::grafana::threshold;

use super::theme;
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![load(q), breakdown(q), waiting(q)]
}

fn load(q: &Queries) -> Row {
    Row::new("Processor Load").grid(
        AutoGrid::new(3)
            .panel("cpu-utilization", utilization(q))
            .panel("cpu-load", load_normalized(q))
            .panel("cpu-pressure", pressure(q)),
    )
}

fn breakdown(q: &Queries) -> Row {
    Row::new("Where the Time Went").grid(
        AutoGrid::new(2)
            .panel("cpu-by-mode", by_mode(q))
            .panel("cpu-per-core", per_core(q)),
    )
}

fn waiting(q: &Queries) -> Row {
    Row::new("Scheduling").grid(
        AutoGrid::new(2)
            .panel("cpu-runqueue", run_queue(q))
            .panel("cpu-context-switches", context_switches(q)),
    )
}

fn utilization(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Utilization")
        .query(q.get("node.cpu.utilization").legend("busy"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .shade(theme::CPU.shade)
        .build(0)
}

fn load_normalized(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Load Average (normalized)")
        .query(q.get("node.load.normalized").legend("load / cores"))
        .unit("short")
        .min(0.0)
        .shade(theme::CPU.shade)
        .build(0)
}

fn pressure(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Pressure")
        .query(q.get("node.cpu.pressure").legend("stalled"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .shade(theme::CPU.shade)
        .build(0)
}

fn by_mode(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU by Mode")
        .query(q.legended(
            "node.debug.cpu.by_mode",
            &["system", "user", "iowait", "irq", "steal"],
        ))
        .unit("percentunit")
        .min(0.0)
        .build(0)
}

fn per_core(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU per Core")
        .query(q.get("node.debug.cpu.per_core").legend("core {{cpu}}"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .build(0)
}

fn run_queue(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Run Queue Wait")
        .query(q.get("node.debug.schedstat.waiting").legend("core {{cpu}}"))
        .unit("s")
        .min(0.0)
        .build(0)
}

fn context_switches(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Context Switches and Interrupts")
        .query(q.legended(
            "node.debug.context_switches",
            &["context switches", "interrupts"],
        ))
        .unit("cps")
        .min(0.0)
        .build(0)
}
