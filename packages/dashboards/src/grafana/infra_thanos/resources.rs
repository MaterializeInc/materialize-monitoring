// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Resources tab: whether the Thanos processes have room to do the work.
//!
//! Absolute figures rather than fractions of a limit, unlike `infra-alloy`. The
//! chart sets no CPU or memory limit on Thanos, so a fraction of the limit would
//! be empty on every default install.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::not_collected;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![usage(q), runtime(q)]
}

fn usage(q: &Queries) -> Row {
    Row::new("Usage").grid(
        AutoGrid::new(2)
            .panel("resources-memory", memory(q))
            .panel("resources-cpu", cpu(q)),
    )
}

fn runtime(q: &Queries) -> Row {
    Row::new("Runtime").grid(
        AutoGrid::new(2)
            .panel("resources-goroutines", goroutines(q))
            .panel("resources-terminations", terminations(q)),
    )
}

fn memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory")
        .query(q.get("infra.thanos.resources.memory").legend("{{pod}}"))
        .unit("bytes")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn cpu(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU")
        .query(q.get("infra.thanos.resources.cpu").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn goroutines(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Goroutines")
        .query(q.get("infra.thanos.resources.goroutines").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn terminations(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Last Termination Reason")
        .query(q.get("infra.thanos.resources.terminations").table_format())
        .transformations(vec![transform::organize(
            &["Time", "Value"],
            &["container", "pod", "reason"],
        )])
        // A table of events that have not happened has no zero to draw, so
        // the text names both readings rather than claiming the healthy one.
        .no_value(NoValue::Custom(
            "No recorded termination. If Memory is empty too, nothing is being collected."
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
        assert_eq!(assembled.elements.len(), 4);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }
}
