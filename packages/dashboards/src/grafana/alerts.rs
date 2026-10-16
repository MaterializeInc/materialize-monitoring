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
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
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
    ///
    /// The registry marks the query instant, so the stat has no sparkline. Over
    /// a range, Grafana's last-non-null reduction would keep showing the zero
    /// from before a ruler went silent, which is the reading this panel exists
    /// to withhold. The history beside the table is the range view.
    pub fn firing(&self, q: &Queries) -> dashboardv2::PanelKind {
        zero_is_healthy(&self.firing_title())
            .graph_mode(BigValueGraphMode::None)
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
    fn the_firing_count_is_a_current_value() {
        // Over a range, the stat's last-non-null reduction would keep the zero
        // from before a ruler went silent for the rest of the window.
        let q = &test_queries();
        for (name, alerts) in all() {
            let json = serde_json::to_string(&alerts.firing(q)).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{name}: {json}");
            assert!(json.contains(r#""graphMode":"none""#), "{name}: {json}");
        }
    }

    #[test]
    fn the_table_keeps_an_instance_the_rule_named() {
        // `environmentd-cpu-throttled` keeps the node's `instance`. Dropping the
        // label merged two throttled containers into one row; only the value
        // the gateway stamps on a result without one says nothing.
        let q = &test_queries();
        for (name, alerts) in all() {
            if !alerts.common_alerts {
                continue;
            }
            let expr = expression(&alerts.table(q));
            assert!(!expr.contains("ruler_replica, instance"), "{name}: {expr}");
            assert!(
                expr.contains(r#""instance", "", "instance", "thanos-ruler|loki-ruler""#),
                "{name}: {expr}"
            );
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

    /// A panel's one rendered expression, with the dashboard variables the
    /// alert queries read set to the values the series below carry.
    fn expression(panel: &dashboardv2::PanelKind) -> String {
        let json = serde_json::to_value(panel).expect("serialize");
        json["spec"]["data"]["spec"]["queries"][0]["spec"]["query"]["spec"]["expr"]
            .as_str()
            .expect("expr")
            .replace("$mzNamespaceList", "materialize-environment")
            .replace("$thanosNamespace", "monitoring")
    }

    /// One input series for `promtool test rules`.
    fn series(series: &str, values: &str) -> serde_json::Value {
        serde_json::json!({ "series": series, "values": values })
    }

    /// A ruler metric's series, on the pod and rule group given.
    fn ruler(metric: &str, labels: &str, values: &str) -> serde_json::Value {
        series(
            &format!(r#"{metric}{{container="ruler", namespace="monitoring", {labels}}}"#),
            values,
        )
    }

    /// Both rulers healthy: every rule group evaluated each minute, and every
    /// remote-write queue sent up to its newest sample. Values are timestamps
    /// in seconds, so `0+60x20` is a group evaluated on schedule.
    fn healthy_rulers() -> Vec<serde_json::Value> {
        let mut out = vec![
            ruler(
                "prometheus_rule_group_last_evaluation_timestamp_seconds",
                r#"app="thanos", pod="thanos-ruler-0", rule_group="r.yaml;g""#,
                "0+60x20",
            ),
            ruler(
                "prometheus_remote_storage_highest_timestamp_in_seconds",
                r#"app="thanos", pod="thanos-ruler-0""#,
                "0+60x20",
            ),
            ruler(
                "prometheus_remote_storage_queue_highest_sent_timestamp_seconds",
                r#"app="thanos", pod="thanos-ruler-0""#,
                "0+60x20",
            ),
        ];
        for (pod, group) in [("loki-ruler-0", "panics"), ("loki-ruler-1", "correctness")] {
            let group =
                format!(r#"app="loki", pod="{pod}", rule_group="l.yaml;{group}", user="loki""#);
            out.push(ruler(
                "loki_prometheus_rule_group_last_evaluation_timestamp_seconds",
                &group,
                "0+60x20",
            ));
            out.push(ruler(
                "loki_prometheus_rule_group_interval_seconds",
                &group,
                "60x20",
            ));
            let queue = format!(r#"app="loki", pod="{pod}", tenant="loki""#);
            for metric in [
                "loki_ruler_wal_prometheus_remote_storage_highest_timestamp_in_seconds",
                "loki_ruler_wal_prometheus_remote_storage_queue_highest_sent_timestamp_seconds",
            ] {
                out.push(ruler(metric, &queue, "0+60x20"));
            }
        }
        out
    }

    /// Replace the values of every input series whose text contains each key.
    fn with(mut input: Vec<serde_json::Value>, changes: &[(&str, &str)]) -> Vec<serde_json::Value> {
        for (needle, values) in changes {
            let mut found = false;
            for s in &mut input {
                if s["series"]
                    .as_str()
                    .is_some_and(|text| text.contains(needle))
                {
                    s["values"] = serde_json::json!(values);
                    found = true;
                }
            }
            assert!(found, "no input series contains {needle}");
        }
        input
    }

    /// Run expression tests through `promtool test rules`, or skip when it is
    /// not installed. CI installs it for the cargo tests.
    fn promtool(name: &str, tests: Vec<serde_json::Value>) {
        use std::process::Command;
        if Command::new("promtool").arg("--version").output().is_err() {
            eprintln!("skipping {name}: `promtool` not found on PATH");
            return;
        }
        // JSON is YAML, so the document needs no serializer of its own.
        let doc = serde_json::json!({
            "rule_files": [],
            "evaluation_interval": "1m",
            "tests": tests,
        });
        let path = std::env::temp_dir().join(format!(
            "mzmon-dashboards-{name}-{}.test.yaml",
            std::process::id()
        ));
        std::fs::write(&path, doc.to_string()).expect("write the promtool test");
        let output = Command::new("promtool")
            .args(["test", "rules"])
            .arg(&path)
            .output()
            .expect("run promtool");
        let _ = std::fs::remove_file(&path);
        assert!(
            output.status.success(),
            "{name}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn the_firing_count_reads_zero_only_from_rulers_that_deliver() {
        // Each case is the rendered expression against synthetic rulers,
        // evaluated at 15m. `Some(n)` is the count shown; `None` is the empty
        // panel that says the alert path is down.
        let q = &test_queries();
        let stalled = "0+60x5 300x15";
        let firing = [
            series(
                r#"ALERTS{alertname="ThanosCompactHalted", alertstate="firing", job="thanos-compact"}"#,
                "1x20",
            ),
            series(
                r#"ALERTS{alertname="loki-panics", alertstate="firing", component="loki", namespace="monitoring"}"#,
                "1x20",
            ),
            series(
                r#"ALERTS{alertname="clusterd-metrics-missing", alertstate="firing", component="monitoring", namespace="materialize-environment"}"#,
                "1x20",
            ),
        ];
        let loki_ruler_only = |s: &serde_json::Value| {
            s["series"]
                .as_str()
                .is_some_and(|text| !text.contains(r#"app="loki""#))
        };

        for (name, alerts) in all() {
            let waits_on_loki = alerts.common_alerts;
            let loki = |stale: Option<f64>| if waits_on_loki { stale } else { Some(0.0) };
            let cases: Vec<(&str, Vec<serde_json::Value>, Option<f64>)> = vec![
                ("both rulers healthy", healthy_rulers(), Some(0.0)),
                (
                    "no Loki ruler",
                    healthy_rulers()
                        .into_iter()
                        .filter(loki_ruler_only)
                        .collect(),
                    Some(0.0),
                ),
                (
                    "Thanos ruler stopped evaluating",
                    with(
                        healthy_rulers(),
                        &[(
                            "prometheus_rule_group_last_evaluation_timestamp_seconds{container=\"ruler\", namespace=\"monitoring\", app=\"thanos\"",
                            stalled,
                        )],
                    ),
                    None,
                ),
                (
                    "Thanos ruler's write-back stalled",
                    with(
                        healthy_rulers(),
                        &[(
                            "prometheus_remote_storage_queue_highest_sent_timestamp_seconds{container=\"ruler\", namespace=\"monitoring\", app=\"thanos\"",
                            stalled,
                        )],
                    ),
                    None,
                ),
                (
                    "one Loki ruler's groups stalled while the other runs",
                    with(healthy_rulers(), &[("correctness\", user", stalled)]),
                    loki(None),
                ),
                (
                    "one Loki ruler's write-back stalled",
                    with(
                        healthy_rulers(),
                        &[(
                            "queue_highest_sent_timestamp_seconds{container=\"ruler\", namespace=\"monitoring\", app=\"loki\", pod=\"loki-ruler-1\"",
                            stalled,
                        )],
                    ),
                    loki(None),
                ),
                (
                    "a Loki group loaded and not yet evaluated",
                    {
                        let mut input = healthy_rulers();
                        input.push(ruler(
                            "loki_prometheus_rule_group_last_evaluation_timestamp_seconds",
                            r#"app="loki", pod="loki-ruler-0", rule_group="l.yaml;new", user="loki""#,
                            "0x20",
                        ));
                        input
                    },
                    Some(0.0),
                ),
                (
                    "a restarted Loki ruler's previous series still in the lookback",
                    {
                        let mut input = healthy_rulers();
                        input.push(ruler(
                            "loki_prometheus_rule_group_last_evaluation_timestamp_seconds",
                            r#"app="loki", pod="loki-ruler-1", rule_group="l.yaml;correctness", user="loki", instance="old""#,
                            "300x20",
                        ));
                        input
                    },
                    Some(0.0),
                ),
                (
                    "an alert firing",
                    healthy_rulers().into_iter().chain(firing.clone()).collect(),
                    Some(1.0),
                ),
            ];

            let expr = expression(&alerts.firing(q));
            let tests = cases
                .into_iter()
                .map(|(case, input, want)| {
                    let samples = match want {
                        Some(value) => serde_json::json!([{ "labels": "{}", "value": value }]),
                        None => serde_json::json!([]),
                    };
                    serde_json::json!({
                        "name": case,
                        "interval": "1m",
                        "input_series": input,
                        "promql_expr_test": [{
                            "expr": expr,
                            "eval_time": "15m",
                            "exp_samples": samples,
                        }],
                    })
                })
                .collect();
            promtool(name, tests);
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
