// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Summary tab: is the environment healthy, and where do I go next.
//!
//! Every panel here is a pointer. It answers a question in one number and names
//! the tab that explains it, which is why the panels borrow their shades from the
//! tabs they point at rather than having one of their own — see
//! [`super::theme`].

use mzmon_lib::grafana::generated::{dashboardv2, stat};
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::theme;
use super::transform;
use crate::grafana::queries::Queries;

/// The Summary tab's rows.
pub fn rows(q: &Queries) -> Vec<Row> {
    vec![environment_health(q), environment_info(q)]
}

/// Health at a glance: is it up, has it been up, and is anything behind.
fn environment_health(q: &Queries) -> Row {
    Row::new("Environment Health").grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("is-healthy", is_healthy(q))
            .panel("availability-percent", availability_percent(q))
            .panel("last-restart", last_restart(q))
            .panel(
                "summary-currently-hydrating",
                super::currently_hydrating(q, theme::COMPUTE.shade),
            )
            .panel("summary-max-lag", max_lag(q))
            .panel("cpu-usage-current", cpu_usage_current(q))
            .panel("memory-usage-current", memory_usage_current(q)),
    )
}

/// What is running, and how much of it there is.
fn environment_info(q: &Queries) -> Row {
    Row::new("Environment Info").grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("materialize-version", materialize_version(q))
            .panel("summary-cpu-total", cpu_total(q))
            .panel("summary-memory-total", memory_total(q)),
    )
}

fn is_healthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Environment Status")
        .query(q.get("materialize.health.clusters.status.percentage"))
        .color_background()
        // Mapped to words rather than threshold-coloured: "Degraded" says more
        // than "87".
        .mappings(threshold::health_mapping(80.0, 100.0))
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn availability_percent(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Environment Availability (Select Time Range)")
        .query(q.get("materialize.health.environment.availability.percentage"))
        .color_background()
        .unit("percent")
        // Four decimals: five-nines and 100% are different stories.
        .decimals(4.0)
        .thresholds(threshold::health(95.0, 99.0).percentage().build())
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn last_restart(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Last Restart Time")
        .query(
            q.get("materialize.kubernetes.last_restart")
                .legend("{{pod}}"),
        )
        .color_background()
        .justify_center()
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .value_size(25.0)
        .unit("s")
        // Low is bad here: a restart seconds ago is the alarming case, so the
        // palette runs alarming-to-calm as the duration grows.
        .thresholds(threshold::stability_days(2.0, false).build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn max_lag(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Max Lag (Select Time Range)")
        .query(q.get("materialize.info.max_lag").legend("max lag"))
        .color_background()
        .unit("s")
        // High is bad: an hour of lag is the alarming end.
        .thresholds(threshold::stability(3600.0, true).build())
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn cpu_usage_current(q: &Queries) -> dashboardv2::PanelKind {
    Panel::gauge("Current CPU Usage (5 min)")
        .query(
            q.get("materialize.kubernetes.cpu.usage.percent")
                .legend("{{container}}"),
        )
        .unit("percentunit")
        .thresholds(threshold::load_default().build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_usage_current(q: &Queries) -> dashboardv2::PanelKind {
    Panel::gauge("Current Memory Usage")
        .query(
            q.get("materialize.kubernetes.memory.usage.percent")
                .legend("{{container}}"),
        )
        .unit("percentunit")
        .thresholds(threshold::load_default().build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn materialize_version(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Materialize Version")
        .query(q.get("materialize.info.version").legend("{{mz_version}}"))
        // No sparkline: a version is not a time series.
        .graph_mode(stat::BigValueGraphMode::None)
        .value_size(20.0)
        // Reduce over the version label only, so the panel shows the version
        // string rather than the metric's value.
        .reduce_fields("/^mz_version$/")
        .links(vec![commit_link()])
        // The version arrives as a label, so promote it to a field, then split the
        // trailing `(<commit>)` out into a `commit` field for the data link below.
        .transformations(vec![
            transform::labels_to_fields(&[]),
            transform::extract_fields_regex("mz_version", COMMIT_PATTERN),
        ])
        .build(0)
}

/// Pull the commit hash out of a version string like `v0.150.1 (abc1234)`.
///
/// Grafana's `extractFields` regex form wants the delimiting slashes, and the
/// named group is what becomes the `commit` field.
const COMMIT_PATTERN: &str = r"/.+\((?<commit>[a-fA-F0-9]+)\)/";

/// Link the displayed version to its commit on GitHub.
///
/// `${__data.fields.commit}` is a Grafana data link, interpolated from the row's
/// `commit` field rather than from a dashboard variable.
fn commit_link() -> serde_json::Map<String, serde_json::Value> {
    match serde_json::json!({
        "title": "View Materialize at Commit",
        "url": "https://github.com/MaterializeInc/materialize/commit/${__data.fields.commit}",
        "targetBlank": true,
    }) {
        serde_json::Value::Object(map) => map,
        _ => unreachable!("a json! object literal is an object"),
    }
}

fn cpu_total(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Total CPU Capacity")
        .query(
            q.get("materialize.kubernetes.cpu.capacity")
                .legend("CPUs ({{container}})"),
        )
        .text_mode(stat::BigValueTextMode::ValueAndName)
        // Points at Kubernetes Workloads.
        .shade(theme::KUBERNETES.shade)
        .unit("cores")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_total(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Total Memory")
        .query(
            q.get("materialize.kubernetes.memory.capacity")
                .legend("{{container}}"),
        )
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(theme::KUBERNETES.shade)
        .unit("bytes")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_has_both_rows_and_all_ten_panels() {
        let q = &crate::grafana::queries::test_queries();
        let rows = rows(q);
        assert_eq!(rows.len(), 2);
        // Panel count is checked end to end against the golden in
        // tests/env_top_parity.rs; this just guards the row split.
        let json = serde_json::to_value(
            mzmon_lib::grafana::layout::Layout::rows(rows)
                .assemble()
                .expect("assemble")
                .layout,
        )
        .expect("serialize");
        let text = json.to_string();
        assert_eq!(text.matches("ElementReference").count(), 10);
    }

    #[test]
    fn every_shade_is_a_theme_colour() {
        let q = &crate::grafana::queries::test_queries();
        // Checked on the built output rather than the source text: what matters is
        // that no panel ends up with a colour outside the qualitative palette,
        // however it got there.
        use mzmon_lib::grafana::palette;
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        let mut shaded = 0usize;
        for element in assembled.elements.values() {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            let Some(colour) = &panel.spec.viz_config.spec.field_config.defaults.color else {
                continue;
            };
            let Some(hex) = &colour.fixed_color else {
                continue;
            };
            shaded += 1;
            assert!(
                palette::THEME.contains(&hex.as_str()),
                "{hex} is not a THEME colour"
            );
        }
        // Three panels borrow a shade in the baseline; if that drops to zero the
        // assertion above would pass vacuously.
        assert_eq!(shaded, 3, "expected three borrowed shades");
    }

    #[test]
    fn every_panel_query_is_environment_scoped_or_namespace_scoped() {
        let q = &crate::grafana::queries::test_queries();
        // The property the selectors exist to guarantee: no panel queries the
        // whole fleet. Asserted on the rendered expressions, not the source.
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = query.spec.query.spec.as_ref().and_then(|s| s.get("expr"));
                let expr = expr.and_then(|e| e.as_str()).unwrap_or_default();
                assert!(
                    expr.contains("$environmentNameList") || expr.contains("$mzNamespaceList"),
                    "{name} is not scoped to the environment:\n{expr}"
                );
            }
        }
    }
}
