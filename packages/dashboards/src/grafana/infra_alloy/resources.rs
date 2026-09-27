// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Resources tab: whether the collectors have room to do the work.
//!
//! Every panel is a fraction of a limit rather than an absolute amount, because
//! the question is how close each collector is to the point where something
//! gives. The two memory panels measure against different limits on purpose.
//! The container's working set is what the kernel's hard limit is enforced
//! against. Go's own memory is what `GOMEMLIMIT` is enforced against, and the
//! chart sets that below the hard limit so the runtime slows down before the
//! kernel kills it. A gateway near the soft limit is spending its CPU on garbage
//! collection, which is why CPU and memory share this tab.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::overview::not_collected;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![memory(q), cpu(q), runtime(q)]
}

fn memory(q: &Queries) -> Row {
    Row::new("Memory").grid(
        AutoGrid::new(3)
            .panel("resources-memory", memory_used(q))
            .panel("resources-gomemlimit", gomemlimit(q))
            .panel("resources-memory-limiter", memory_limiter(q)),
    )
}

fn cpu(q: &Queries) -> Row {
    Row::new("CPU").grid(
        AutoGrid::new(2)
            .panel("resources-cpu", cpu_used(q))
            .panel("resources-throttling", throttling(q)),
    )
}

fn runtime(q: &Queries) -> Row {
    Row::new("Runtime").grid(
        AutoGrid::new(3)
            .panel("resources-goroutines", goroutines(q))
            .panel("resources-fds", file_descriptors(q))
            .panel("resources-terminations", terminations(q)),
    )
}

/// A fraction of a limit, drawn against the whole range from empty to full.
///
/// Pinned rather than autoscaled so a collector at 20% reads as 20% at a
/// glance. An autoscaled axis would draw it as a line near the top.
fn against_limit(title: &str) -> Panel<mzmon_lib::grafana::panel::Timeseries> {
    Panel::timeseries(title)
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
}

fn memory_used(q: &Queries) -> dashboardv2::PanelKind {
    against_limit("Memory Used Against Limit")
        .query(q.get("infra.alloy.resources.memory").legend("{{pod}}"))
        .no_value(NoValue::RequiresCAdvisorAndKubeStateMetrics)
        .build(0)
}

fn gomemlimit(q: &Queries) -> dashboardv2::PanelKind {
    against_limit("Go Memory Against GOMEMLIMIT")
        .query(q.get("infra.alloy.resources.gomemlimit").legend("{{pod}}"))
        .no_value(not_collected())
        .build(0)
}

fn memory_limiter(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Limiter Refusals")
        .query(
            q.get("infra.alloy.resources.memory_limiter")
                .legend("{{pod}}"),
        )
        .unit("suffix:points/s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn cpu_used(q: &Queries) -> dashboardv2::PanelKind {
    against_limit("CPU Used Against Limit")
        .query(q.get("infra.alloy.resources.cpu").legend("{{pod}}"))
        .no_value(NoValue::RequiresCAdvisorAndKubeStateMetrics)
        .build(0)
}

fn throttling(q: &Queries) -> dashboardv2::PanelKind {
    against_limit("CPU Throttling")
        .query(q.get("infra.alloy.resources.throttling").legend("{{pod}}"))
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn goroutines(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Goroutines")
        .query(q.get("infra.alloy.resources.goroutines").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn file_descriptors(q: &Queries) -> dashboardv2::PanelKind {
    against_limit("File Descriptors Against Limit")
        .query(
            q.get("infra.alloy.resources.file_descriptors")
                .legend("{{pod}}"),
        )
        .no_value(not_collected())
        .build(0)
}

fn terminations(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Last Termination Reason")
        .query(q.get("infra.alloy.resources.terminations").table_format())
        .transformations(vec![transform::organize(
            &["Time", "Value", "namespace"],
            &["pod", "reason"],
        )])
        // A table of events that have not happened has no zero to draw, so
        // the text names both readings rather than claiming the healthy one.
        .no_value(NoValue::Custom(
            "No recorded termination. If Memory Used Against Limit is empty too, nothing is being collected."
                .to_string(),
        ))
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
        assert_eq!(assembled.elements.len(), 8);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_fractions_are_pinned_at_their_ceiling() {
        // A collector at 20% of its limit has to read as 20%, not as a line
        // near the top of an autoscaled axis.
        let q = &test_queries();
        for panel in [
            memory_used(q),
            gomemlimit(q),
            cpu_used(q),
            throttling(q),
            file_descriptors(q),
        ] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""max":1.0"#), "{json}");
        }
    }

    #[test]
    fn memory_is_measured_as_the_limit_measures_it() {
        // RSS counts reclaimable file pages, and read an agent at 95% of its
        // limit while its working set was at 29%.
        let q = &test_queries();
        let json = serde_json::to_string(&memory_used(q)).expect("serialize");
        assert!(
            json.contains("container_memory_working_set_bytes"),
            "{json}"
        );
        assert!(!json.contains("resident_memory"), "{json}");
    }
}
