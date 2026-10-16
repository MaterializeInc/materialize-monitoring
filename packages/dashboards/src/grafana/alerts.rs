// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The firing-alerts panels an Overview tab leads with.
//!
//! Three panels, drawn the same way wherever a dashboard's subject has
//! alerting rules: a count of firing alerts for the verdict row, and an Alerts
//! row beneath it holding a table of what is firing or pending now beside a
//! timeseries of what fired over the range. The style guide's "Firing alerts
//! lead an Overview tab" has the rules; this module is what keeps the
//! dashboards from drifting apart on them.
//!
//! A dashboard supplies the registry queries and the table's columns. Each
//! query set lives in its dashboard's registry file, because what selects "this
//! subject's alerts" differs: a name prefix for the Thanos mixin's rules, a
//! `component` for the registry's, a namespace for an environment's.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;
use mzmon_lib::query::rules::render::COMMON_ALERTS_URL;

use crate::grafana::dependency::zero_is_healthy;
use crate::grafana::field_override;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// One dashboard's alert panels.
pub struct Alerts {
    /// The registry ids, `<queries>.alerts_firing`, `<queries>.alerts` and
    /// `<queries>.alert_history`.
    pub queries: &'static str,
    /// What the alerts are about, as the count's title names it: "Loki" makes
    /// "Loki Alerts Firing".
    pub subject: &'static str,
    /// The same, as prose names it: "No alert about Loki fired in this range".
    pub about: &'static str,
    /// The table's columns after alert and state, in order, each a label and
    /// its header. A label the query returns and this omits still shows, after
    /// these.
    pub columns: &'static [(&'static str, &'static str)],
    /// Labels the table hides, beyond `Time` and `Value`.
    pub hidden: &'static [&'static str],
    /// Where to look when the count is empty, because a ruler has stopped
    /// evaluating or its results have stopped reaching Thanos.
    pub silent: &'static str,
    /// Whether an alert's name links to its Common Alerts entry. True for the
    /// registry's rules, whose `runbook_url` points there too.
    pub common_alerts: bool,
}

impl Alerts {
    /// The firing count, for the verdict row.
    pub fn firing(&self, q: &Queries) -> dashboardv2::PanelKind {
        zero_is_healthy(&self.firing_title())
            .query(q.get(&self.id("alerts_firing")).legend("alerts"))
            .thresholds(threshold::errors(1.0, 3.0).build())
            .unit("short")
            .decimals(0.0)
            .no_value(NoValue::Custom(format!(
                "Alerts are not reaching Thanos, so none can show. {}",
                self.silent
            )))
            .build(0)
    }

    /// Every alert firing or pending now.
    ///
    /// `table_format` and a dropped `Time` and `Value`, for the reason
    /// `infra-loki`'s Scrape Health gives. The value of an `ALERTS` series is
    /// always 1, so it carries nothing the state column does not.
    ///
    /// Sparse by nature: each rule carries its own labels, and an alert about a
    /// whole component carries no `pod`. Grafana puts the panel's empty-state
    /// text into every empty cell, so every label column overrides it with a
    /// dash. That has to match by type, since which labels arrive depends on
    /// which alerts are firing.
    pub fn table(&self, q: &Queries) -> dashboardv2::PanelKind {
        let mut exclude = vec!["Time", "Value"];
        exclude.extend_from_slice(self.hidden);
        let mut order = vec!["alertname", "alertstate"];
        let mut renames = vec![("alertname", "alert"), ("alertstate", "state")];
        for &(label, header) in self.columns {
            order.push(label);
            if label != header {
                renames.push((label, header));
            }
        }

        let mut overrides = vec![
            field_override::by_type("string")
                .property("noValue", serde_json::json!("—"))
                .build(),
        ];
        if self.common_alerts {
            overrides.push(
                field_override::by_name("alert")
                    .property("links", serde_json::json!([common_alerts_link()]))
                    .build(),
            );
        }

        Panel::table("Alerts")
            .query(q.get(&self.id("alerts")).table_format())
            .transformations(vec![transform::organize_full(&exclude, &order, &renames)])
            .overrides(overrides)
            .no_value(NoValue::Custom(format!(
                "No alert about {} is firing or pending, or none reached Thanos. {} says which.",
                self.about,
                self.firing_title(),
            )))
            .build(0)
    }

    /// Which alerts fired over the range, one line each.
    pub fn history(&self, q: &Queries) -> dashboardv2::PanelKind {
        Panel::timeseries("Alerts Firing Over Time")
            .query(q.get(&self.id("alert_history")).legend("{{alertname}}"))
            .unit("short")
            .decimals(0.0)
            .min(0.0)
            .no_value(NoValue::Custom(format!(
                "No alert about {} fired in this range",
                self.about
            )))
            .build(0)
    }

    /// The Alerts row: the table beside the history, directly under the
    /// verdict.
    ///
    /// `prefix` starts each element name, as the tab's other panels do:
    /// `overview` gives `overview-alerts` and `overview-alert-history`.
    pub fn row(&self, q: &Queries, prefix: &str) -> Row {
        Row::new("Alerts").grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel(format!("{prefix}-alerts"), self.table(q))
                .panel(format!("{prefix}-alert-history"), self.history(q)),
        )
    }

    fn id(&self, query: &str) -> String {
        format!("{}.{query}", self.queries)
    }

    fn firing_title(&self) -> String {
        format!("{} Alerts Firing", self.subject)
    }
}

/// A link from an alert's name to its entry under Common Alerts, which says
/// what it means and what to check.
///
/// `ALERTS` carries a rule's labels and not its annotations, so the summary and
/// the runbook cannot be columns. The entry is what the rule's own
/// `runbook_url` points at, anchored by alert name.
fn common_alerts_link() -> serde_json::Value {
    serde_json::json!({
        "title": "What this alert means",
        "url": format!("{COMMON_ALERTS_URL}#${{__value.raw}}"),
        "targetBlank": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;
    use crate::grafana::{env_top, infra_alloy, infra_loki, infra_thanos};

    /// Every dashboard's alert panels.
    fn all() -> [(&'static str, &'static Alerts); 4] {
        [
            ("infra-thanos", &infra_thanos::overview::ALERTS),
            ("infra-loki", &infra_loki::overview::ALERTS),
            ("infra-alloy", &infra_alloy::overview::ALERTS),
            ("env-top", &env_top::summary::ALERTS),
        ]
    }

    #[test]
    fn every_query_set_is_in_the_registry() {
        let q = &test_queries();
        for (_, alerts) in all() {
            alerts.firing(q);
            alerts.table(q);
            alerts.history(q);
        }
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_table_is_instant_and_renders_as_one_table() {
        // A table of current facts: over a range each alert repeats once per
        // evaluation, and without `table_format` each series is a frame of its
        // own behind a picker.
        let q = &test_queries();
        for (name, alerts) in all() {
            let json = serde_json::to_string(&alerts.table(q)).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{name}: {json}");
            assert!(json.contains(r#""format":"table""#), "{name}: {json}");
        }
    }

    #[test]
    fn an_alert_without_a_label_does_not_read_as_no_alerts() {
        // Seen on a live install: a component-wide alert's empty `pod` cell
        // read "No Thanos alert is firing or pending" beside a firing alert.
        let q = &test_queries();
        for (name, alerts) in all() {
            let json = serde_json::to_string(&alerts.table(q)).expect("serialize");
            assert!(json.contains(r#""id":"byType""#), "{name}: {json}");
            assert!(json.contains(r#""value":"—""#), "{name}: {json}");
        }
    }

    #[test]
    fn the_firing_count_does_not_claim_zero_from_a_silent_ruler() {
        // `or vector(0)` would read zero with the alert path down. The zero has
        // to come from a ruler that is evaluating, and that is writing its
        // `ALERTS` back: one whose write-back has stalled keeps evaluating
        // while its alerts go stale in Thanos.
        let q = &test_queries();
        for (name, alerts) in all() {
            let json = serde_json::to_string(&alerts.firing(q)).expect("serialize");
            assert!(!json.contains("vector(0)"), "{name}: {json}");
            assert!(
                json.contains("prometheus_rule_group_last_evaluation_timestamp_seconds"),
                "{name}: {json}"
            );
            assert!(
                json.contains("prometheus_remote_storage_queue_highest_sent_timestamp_seconds"),
                "{name}: {json}"
            );
        }
    }

    #[test]
    fn a_registry_alert_count_also_waits_on_the_loki_ruler() {
        // The registry's log-derived rules are the Loki ruler's, and its
        // `ALERTS` reach Thanos by remote write as the Thanos ruler's do. The
        // Thanos mixin's rules are not, so only that count may skip it.
        let q = &test_queries();
        for (name, alerts) in all() {
            if !alerts.common_alerts {
                continue;
            }
            let json = serde_json::to_string(&alerts.firing(q)).expect("serialize");
            assert!(
                json.contains("loki_prometheus_rule_group_last_evaluation_timestamp_seconds"),
                "{name}: {json}"
            );
            assert!(
                json.contains(
                    "loki_ruler_wal_prometheus_remote_storage_queue_highest_sent_timestamp_seconds"
                ),
                "{name}: {json}"
            );
        }
    }

    #[test]
    fn a_registry_alert_links_to_what_it_means() {
        let q = &test_queries();
        for (name, alerts) in all() {
            let json = serde_json::to_string(&alerts.table(q)).expect("serialize");
            assert_eq!(
                json.contains(COMMON_ALERTS_URL),
                alerts.common_alerts,
                "{name}: {json}"
            );
        }
    }

    #[test]
    fn the_verdict_count_uses_health_colours() {
        let q = &test_queries();
        for (name, alerts) in all() {
            let json = serde_json::to_string(&alerts.firing(q)).expect("serialize");
            assert!(json.contains("background"), "{name}: {json}");
        }
    }
}
