// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is collection working, and is this dashboard hearing from
//! the collectors at all.
//!
//! The verdict row is six counts that are zero on a healthy install, each built
//! so that zero and absent read differently. Three are about the collectors
//! themselves, and three about data being lost: log lines and samples the
//! collectors failed to deliver, and pushes the gateway refused to take.
//!
//! Collection Health follows, because it is what says whether the verdict can
//! be believed: a collector the gateway cannot scrape contributes nothing to
//! any count above it.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// Grafana has no unit for log lines or samples, so these are its custom-suffix
/// form.
pub(super) const LINES_PER_SECOND: &str = "suffix:lines/s";
pub(super) const SAMPLES_PER_SECOND: &str = "suffix:samples/s";

/// What a panel shows when the collectors are not reporting at all.
///
/// The one empty state every metrics panel on this dashboard can reach, and the
/// one that means something different here from anywhere else: the gateway is
/// the component that writes these series, so their absence usually means the
/// gateway is down.
pub(super) fn not_collected() -> NoValue {
    NoValue::Custom("No data from the collectors. Check that the gateway is running.".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), collection(q), throughput(q)]
}

/// The six numbers worth reading before anything else.
///
/// Header hidden and half height, as on `infra-loki`: this row is the answer,
/// and Collection Health beneath it has to stay above the fold.
fn verdict(q: &Queries) -> Row {
    Row::new("Verdict").hide_header().grid(
        AutoGrid::new(6)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-agents-missing", agents_missing(q))
            .panel("overview-unhealthy-components", unhealthy_components(q))
            .panel("overview-config-failed", config_failed(q))
            .panel("overview-log-lines-lost", log_lines_lost(q))
            .panel("overview-samples-lost", samples_lost(q))
            .panel("overview-pushes-refused", pushes_refused(q)),
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
            .panel("overview-lines-delivered", lines_delivered(q))
            .panel("overview-samples-delivered", samples_delivered(q)),
    )
}

/// A count stat whose only healthy reading is zero.
///
/// Health colours rather than the tab shade, since the number's whole meaning is
/// whether it is zero.
fn zero_is_healthy(title: &str) -> Panel<mzmon_lib::grafana::panel::Stat> {
    Panel::stat(title)
        .graph_mode(BigValueGraphMode::Area)
        .color_background()
        .min(0.0)
}

fn agents_missing(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Agents Not Reporting")
        .query(q.get("infra.alloy.health.agents_missing").legend("agents"))
        .thresholds(threshold::errors(1.0, 3.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn unhealthy_components(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Unhealthy Components")
        .query(
            q.get("infra.alloy.health.unhealthy_components")
                .legend("components"),
        )
        .thresholds(threshold::errors(1.0, 5.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_collected())
        .build(0)
}

fn config_failed(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Failed Config Loads")
        .query(
            q.get("infra.alloy.health.config_failed")
                .legend("collectors"),
        )
        .thresholds(threshold::errors(1.0, 3.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_collected())
        .build(0)
}

pub(super) fn log_lines_lost(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Log Lines Lost")
        .query(q.get("infra.alloy.health.log_lines_lost").legend("lost/s"))
        .thresholds(threshold::errors(0.01, 10.0).build())
        .unit(LINES_PER_SECOND)
        .no_value(not_collected())
        .build(0)
}

pub(super) fn samples_lost(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Samples Lost")
        .query(q.get("infra.alloy.health.samples_lost").legend("lost/s"))
        .thresholds(threshold::errors(0.01, 100.0).build())
        .unit(SAMPLES_PER_SECOND)
        .no_value(not_collected())
        .build(0)
}

/// Whether the rest of the dashboard can be believed.
///
/// `table_format` and a dropped `Time` column, for the reason `infra-loki`'s
/// Scrape Health gives: without them a table of seven collectors renders as a
/// frame picker of seven one-row tables.
fn scrape_health(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Collector Scrape Health")
        .query(q.get("infra.alloy.health.up").table_format())
        .transformations(vec![transform::organize(
            &["Time"],
            &["app", "pod", "Value"],
        )])
        .unit("short")
        .no_value(not_collected())
        .build(0)
}

fn restarts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Container Restarts")
        .query(q.get("infra.alloy.health.restarts").table_format())
        .transformations(vec![transform::organize(
            &["Time", "namespace"],
            &["pod", "Value"],
        )])
        .unit("short")
        .no_value(NoValue::Custom(
            "No collector has restarted in this range".to_string(),
        ))
        .build(0)
}

fn versions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Running Versions")
        .query(q.get("infra.alloy.health.versions").table_format())
        .transformations(vec![transform::organize_full(
            &["Time"],
            &["app", "version", "Value"],
            &[("Value", "collectors")],
        )])
        .unit("short")
        .no_value(not_collected())
        .build(0)
}

/// Refusals on the gateway's push listeners, in requests.
///
/// Empty rather than zero when nothing pushes at all, which on a default
/// install means the agents are not reaching the gateway.
pub(super) fn pushes_refused(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Push Requests Refused")
        .query(
            q.get("infra.alloy.health.pushes_refused")
                .legend("refused/s"),
        )
        .thresholds(threshold::errors(0.01, 10.0).build())
        .unit("reqps")
        .no_value(NoValue::Custom(
            "Nothing is pushing to the gateway".to_string(),
        ))
        .build(0)
}

fn lines_delivered(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Lines Delivered")
        .query(
            q.get("infra.alloy.health.lines_delivered")
                .legend("{{app}} to {{host}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn samples_delivered(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Samples Delivered")
        .query(q.legended(
            "infra.alloy.health.samples_delivered",
            &["{{component_id}}", "{{component_id}}"],
        ))
        .unit(SAMPLES_PER_SECOND)
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
        assert_eq!(assembled.elements.len(), 11);
        // The TLS panel moved to the Ingest tab, beside the listeners it is
        // about, and a refusals stat took its place in the verdict.
        assert!(!assembled.elements.contains_key("overview-tls-rejections"));
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_tables_of_current_facts_are_instant_and_render_as_one_table() {
        // Evaluated over a range, each collector repeats once per step. Without
        // `table_format`, the Table becomes a frame picker whose first frame
        // looks correct.
        let q = &test_queries();
        for panel in [scrape_health(q), restarts(q), versions(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{json}");
            assert!(json.contains(r#""format":"table""#), "{json}");
        }
    }

    #[test]
    fn the_verdict_counts_use_health_colours() {
        let q = &test_queries();
        for panel in [
            agents_missing(q),
            unhealthy_components(q),
            config_failed(q),
            log_lines_lost(q),
            samples_lost(q),
            pushes_refused(q),
        ] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""colorMode":"background""#), "{json}");
            assert!(
                !json.contains(super::super::theme::OVERVIEW.shade),
                "{json}"
            );
        }
    }
}
