// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is the metrics store working, is anything about it
//! alerting, and is this dashboard hearing from it at all.
//!
//! The verdict row is six numbers whose healthy reading is zero or "Running",
//! each built so that zero and absent read differently. The alerts row comes
//! next, above Collection Health, because a firing alert is a conclusion
//! somebody already reached and the rest of the tab is evidence.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::{palette, threshold};

use super::{not_collected, theme};
use crate::grafana::alerts::Alerts;
use crate::grafana::dependency::{value_map, zero_is_healthy};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::OVERVIEW.shade;

/// Grafana has no unit for samples, so this is its custom-suffix form.
pub(super) const SAMPLES_PER_SECOND: &str = "suffix:samples/s";

/// The upstream Thanos mixin's alerts, selected by name because its rules carry
/// nothing but `severity`. The mixin's runbook, not Common Alerts, explains
/// them, so their names do not link.
pub(crate) const ALERTS: Alerts = Alerts {
    queries: "infra.thanos.health",
    subject: "Thanos",
    about: "Thanos",
    columns: &[("severity", "severity"), ("job", "job"), ("pod", "pod")],
    hidden: &[],
    silent: "The Ruler tab's Evaluation and Write-back rows say why.",
    common_alerts: false,
};

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        verdict(q),
        ALERTS.row(q, "overview"),
        collection(q),
        throughput(q),
    ]
}

/// The six numbers worth reading before anything else.
///
/// Header hidden and half height, as on `infra-loki`: this row is the answer,
/// and the alerts beneath it have to stay above the fold.
fn verdict(q: &Queries) -> Row {
    Row::new("Verdict").hide_header().grid(
        AutoGrid::new(6)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-alerts-firing", ALERTS.firing(q))
            .panel("overview-components-down", components_down(q))
            .panel("overview-write-errors", write_errors(q))
            .panel("overview-query-errors", query_errors(q))
            .panel("overview-rule-failures", rule_failures(q))
            .panel("overview-compactor", compactor(q)),
    )
}

fn collection(q: &Queries) -> Row {
    Row::new("Collection Health").grid(
        AutoGrid::new(3)
            .panel("overview-scrape", scrape_health(q))
            .panel("overview-restarts", restarts(q))
            .panel("overview-versions", versions(q)),
    )
}

fn throughput(q: &Queries) -> Row {
    Row::new("Throughput").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-samples-ingested", samples_ingested(q))
            .panel("overview-query-rate", query_rate(q)),
    )
}

fn components_down(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Processes Down")
        .query(
            q.get("infra.thanos.health.components_down")
                .legend("processes"),
        )
        .thresholds(threshold::errors(1.0, 2.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::Custom(
            "No Thanos process found. Check the namespace picker.".to_string(),
        ))
        .build(0)
}

fn write_errors(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Write Errors (5xx)")
        .query(
            q.get("infra.thanos.health.write_errors")
                .legend("5xx share"),
        )
        .thresholds(threshold::errors(0.001, 0.05).build())
        .unit("percentunit")
        .no_value(NoValue::Custom(
            "No remote writes reached the receivers".to_string(),
        ))
        .build(0)
}

fn query_errors(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Query Errors (5xx)")
        .query(
            q.get("infra.thanos.health.query_errors")
                .legend("5xx share"),
        )
        .thresholds(threshold::errors(0.001, 0.05).build())
        .unit("percentunit")
        .no_value(NoValue::Custom("No queries in this range".to_string()))
        .build(0)
}

fn rule_failures(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Failed Rule Evaluations")
        .query(
            q.get("infra.thanos.health.rule_failures")
                .legend("failures/s"),
        )
        .thresholds(threshold::errors(0.001, 0.1).build())
        .unit("suffix:/s")
        .no_value(NoValue::Custom("No ruler is reporting".to_string()))
        .build(0)
}

/// A state rather than a count, so it says so in words.
fn compactor(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Compactor")
        .query(
            q.get("infra.thanos.health.compactor_halted")
                .legend("compactor"),
        )
        .color_background()
        .mappings(vec![
            value_map(0.0, "Running", palette::tri_health::HEALTHY, 1),
            value_map(1.0, "Halted", palette::tri_health::UNHEALTHY, 2),
        ])
        .thresholds(
            threshold::Ladder::new(palette::tri_health::HEALTHY)
                .step(1.0, palette::tri_health::UNHEALTHY)
                .build(),
        )
        .no_value(NoValue::Custom("Compactor not collected".to_string()))
        .build(0)
}

/// Whether the rest of the dashboard can be believed.
fn scrape_health(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Scrape Health")
        .query(q.get("infra.thanos.health.up").table_format())
        .transformations(vec![transform::organize(
            &["Time"],
            &["container", "pod", "Value"],
        )])
        .unit("short")
        .no_value(NoValue::Custom(
            "No Thanos targets found at all. Check the namespace picker, or the Logs tab."
                .to_string(),
        ))
        .build(0)
}

fn restarts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Container Restarts")
        .query(q.get("infra.thanos.health.restarts").table_format())
        .transformations(vec![transform::organize(
            &["Time"],
            &["container", "pod", "Value"],
        )])
        .unit("short")
        .no_value(NoValue::Custom(
            "No restarts in this range. If Scrape Health is empty too, nothing is being collected."
                .to_string(),
        ))
        .build(0)
}

fn versions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Running Versions")
        .query(q.get("infra.thanos.health.versions").table_format())
        .transformations(vec![transform::organize_full(
            &["Time"],
            &["container", "version", "Value"],
            &[("Value", "pods")],
        )])
        .unit("short")
        .no_value(not_collected())
        .build(0)
}

fn samples_ingested(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Samples Ingested")
        .query(
            q.get("infra.thanos.health.samples_ingested")
                .legend("accepted"),
        )
        .shade(SHADE)
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No remote writes reached the receivers".to_string(),
        ))
        .build(0)
}

fn query_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Queries Answered")
        .query(
            q.get("infra.thanos.health.query_rate")
                .legend("{{handler}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(not_collected())
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
        assert_eq!(assembled.elements.len(), 13);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_alerts_sit_above_collection_health() {
        // The habit this dashboard sets for every Overview: a firing alert is
        // the first thing below the verdict, not something found by scrolling.
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        let layout = serde_json::to_string(&assembled.layout).expect("serialize");
        let alerts = layout.find(r#""title":"Alerts""#).expect("alerts row");
        let collection = layout
            .find(r#""title":"Collection Health""#)
            .expect("collection row");
        assert!(alerts < collection, "{layout}");
    }

    #[test]
    fn the_tables_of_current_facts_are_instant_and_render_as_one_table() {
        let q = &test_queries();
        for panel in [scrape_health(q), restarts(q), versions(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{json}");
            assert!(json.contains(r#""format":"table""#), "{json}");
        }
    }

    #[test]
    fn the_verdict_uses_health_colours() {
        let q = &test_queries();
        for panel in [
            ALERTS.firing(q),
            components_down(q),
            write_errors(q),
            query_errors(q),
            rule_failures(q),
            compactor(q),
        ] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(!json.contains(SHADE), "{json}");
            assert!(json.contains("background"), "{json}");
        }
    }
}
