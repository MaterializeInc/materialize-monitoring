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
use crate::grafana::dependency::{value_map, zero_is_healthy};
use crate::grafana::field_override;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::OVERVIEW.shade;

/// Grafana has no unit for samples, so this is its custom-suffix form.
pub(super) const SAMPLES_PER_SECOND: &str = "suffix:samples/s";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), alerts(q), collection(q), throughput(q)]
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
            .panel("overview-alerts-firing", alerts_firing(q))
            .panel("overview-components-down", components_down(q))
            .panel("overview-write-errors", write_errors(q))
            .panel("overview-query-errors", query_errors(q))
            .panel("overview-rule-failures", rule_failures(q))
            .panel("overview-compactor", compactor(q)),
    )
}

fn alerts(q: &Queries) -> Row {
    Row::new("Alerts").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-alerts", alert_table(q))
            .panel("overview-alert-history", alert_history(q)),
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

/// What the alert panels show when the alert path itself is silent.
///
/// The firing count and the table can both be empty for the healthy reason and
/// for the broken one, so the text names both.
fn no_alerts() -> NoValue {
    NoValue::Custom(
        "No Thanos alert is firing or pending, or none reached Thanos. The Ruler tab's Evaluation and Write-back rows say which."
            .to_string(),
    )
}

fn alerts_firing(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Thanos Alerts Firing")
        .query(q.get("infra.thanos.health.alerts_firing").legend("alerts"))
        .thresholds(threshold::errors(1.0, 3.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::Custom(
            "No ruler has evaluated and written back in five minutes, so no alert can show. Check the Ruler tab."
                .to_string(),
        ))
        .build(0)
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

/// Every Thanos alert firing or pending now.
///
/// `table_format` and a dropped `Time` and `Value`, for the reason
/// `infra-loki`'s Scrape Health gives. The value of an `ALERTS` series is
/// always 1, so it carries nothing the state column does not.
///
/// One query, and still sparse: an alert about a whole component, such as
/// `ThanosCompactHasNotRun`, carries no `pod`. Grafana puts the panel's
/// empty-state text into every empty cell, so the label columns override it
/// with a dash.
pub(super) fn alert_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Alerts")
        .query(q.get("infra.thanos.health.alerts").table_format())
        .transformations(vec![transform::organize_full(
            &["Time", "Value"],
            &["alertname", "alertstate", "severity", "job", "pod"],
            &[("alertname", "alert"), ("alertstate", "state")],
        )])
        .overrides(
            ["severity", "job", "pod"]
                .into_iter()
                .map(|field| {
                    field_override::by_name(field)
                        .property("noValue", serde_json::json!("—"))
                        .build()
                })
                .collect(),
        )
        .no_value(no_alerts())
        .build(0)
}

pub(super) fn alert_history(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Alerts Firing Over Time")
        .query(
            q.get("infra.thanos.health.alert_history")
                .legend("{{alertname}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No Thanos alert fired in this range".to_string(),
        ))
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
        for panel in [alert_table(q), scrape_health(q), restarts(q), versions(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{json}");
            assert!(json.contains(r#""format":"table""#), "{json}");
        }
    }

    #[test]
    fn an_alert_without_a_pod_does_not_read_as_no_alerts() {
        // Seen on a live install: the component-wide alert's empty `pod` cell
        // read "No Thanos alert is firing or pending" beside a firing alert.
        let q = &test_queries();
        let json = serde_json::to_string(&alert_table(q)).expect("serialize");
        assert!(json.contains(r#""options":"pod""#), "{json}");
        assert!(json.contains(r#""value":"—""#), "{json}");
    }

    #[test]
    fn the_firing_count_does_not_claim_zero_from_a_silent_ruler() {
        // `or vector(0)` would read zero with the alert path down. The zero
        // has to come from a ruler that is evaluating.
        let q = &test_queries();
        let json = serde_json::to_string(&alerts_firing(q)).expect("serialize");
        assert!(!json.contains("vector(0)"), "{json}");
        assert!(
            json.contains("prometheus_rule_group_last_evaluation_timestamp_seconds"),
            "{json}"
        );
        // Nor from a ruler that evaluates and cannot deliver: its `ALERTS`
        // would go stale in Thanos while its own metrics stay fresh.
        assert!(
            json.contains("prometheus_remote_storage_queue_highest_sent_timestamp_seconds"),
            "{json}"
        );
    }

    #[test]
    fn the_verdict_uses_health_colours() {
        let q = &test_queries();
        for panel in [
            alerts_firing(q),
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
