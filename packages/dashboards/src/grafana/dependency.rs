// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Presentation shared by the two client-side dependency dashboards,
//! `env-persist` and `env-consensus`.
//!
//! Both read persist's own instrumentation, break it down the same way, and
//! reach the same empty states, so the legend and the empty-state text are
//! written once. `infra-cloud` shares none of this: its series carry a
//! provider's resource names, not Materialize's processes.

use mzmon_lib::grafana::generated::{dashboardv2, stat::BigValueGraphMode};
use mzmon_lib::grafana::panel::{NoValue, Panel, Stat};
use mzmon_lib::grafana::{palette, threshold};

/// The legend for a per-process breakdown.
///
/// `cluster_name` comes from the `mzClusterName` join on the long-form cluster
/// id, which only `clusterd` carries — so environmentd reads as itself and a
/// replica as its cluster.
pub(crate) const PROCESS: &str = "{{app}} {{cluster_name}}";

/// What a panel shows when this environment reports nothing at all.
///
/// Every counter these dashboards read exists from the moment a process starts,
/// at zero, so an empty panel is never "no failures" — it is no process
/// reporting.
pub(crate) fn not_collected() -> NoValue {
    NoValue::Custom(
        "No metrics from this environment. Check the environment picker, and that Materialize is running."
            .to_string(),
    )
}

/// A count stat whose only healthy reading is zero.
pub(crate) fn zero_is_healthy(title: &str) -> Panel<Stat> {
    Panel::stat(title)
        .graph_mode(BigValueGraphMode::Area)
        .color_background()
        .min(0.0)
}

/// A latency ladder: healthy below `degraded`, unhealthy from `unhealthy`.
pub(crate) fn latency_ladder(degraded: f64, unhealthy: f64) -> dashboardv2::ThresholdsConfig {
    threshold::Ladder::new(palette::tri_health::HEALTHY)
        .step(degraded, palette::tri_health::DEGRADED)
        .step(unhealthy, palette::tri_health::UNHEALTHY)
        .build()
}
