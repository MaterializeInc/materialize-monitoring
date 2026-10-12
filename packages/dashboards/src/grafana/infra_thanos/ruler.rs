// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Ruler tab: whether rules are being evaluated, whether their alerts reach
//! Alertmanager, and whether their results get written back.
//!
//! Every failure here is silent everywhere else. An alert that cannot be
//! evaluated, or one that is evaluated and then dropped, looks exactly like an
//! alert whose condition is false.
//!
//! Rule groups are named by the group alone. The ruler's own `rule_group` label
//! prefixes the full path of the generated rule file, which is the same for
//! every group a `PrometheusRule` holds and pushes the group name off the end of
//! any legend.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::not_collected;
use super::overview::SAMPLES_PER_SECOND;
use crate::grafana::dependency::zero_is_healthy;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![evaluation(q), rules(q), notifications(q), write_back(q)]
}

fn evaluation(q: &Queries) -> Row {
    Row::new("Evaluation").grid(
        AutoGrid::new(3)
            .panel("ruler-evaluations", evaluations(q))
            .panel("ruler-failures", failures(q))
            .panel("ruler-missed", missed(q)),
    )
}

fn rules(q: &Queries) -> Row {
    Row::new("Rules").grid(
        AutoGrid::new(3)
            .panel("ruler-slowest", slowest_groups(q))
            .panel("ruler-loaded", loaded(q))
            .panel("ruler-config-failed", config_failed(q)),
    )
}

fn notifications(q: &Queries) -> Row {
    Row::new("Notifications").grid(
        AutoGrid::new(4)
            .panel("ruler-alerts-sent", alerts_sent(q))
            .panel("ruler-alerts-dropped", alerts_dropped(q))
            .panel("ruler-queue", queue(q))
            .panel("ruler-sender-latency", sender_latency(q)),
    )
}

fn write_back(q: &Queries) -> Row {
    Row::new("Write-back").grid(
        AutoGrid::new(2)
            .panel("ruler-remote-write-lag", remote_write_lag(q))
            .panel("ruler-remote-write-failures", remote_write_failures(q)),
    )
}

fn no_ruler() -> NoValue {
    NoValue::Custom("No ruler is reporting".to_string())
}

fn evaluations(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rule Evaluations")
        .query(q.get("infra.thanos.rule.evaluations").legend("{{pod}}"))
        .unit("suffix:/s")
        .min(0.0)
        .no_value(no_ruler())
        .build(0)
}

/// Filtered to groups that failed, so a quiet panel means none did.
fn failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed Evaluations by Group")
        .query(q.get("infra.thanos.rule.failures").legend("{{group}}"))
        .unit("suffix:/s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No rule group failed to evaluate in this range. If Rule Evaluations is empty too, no ruler is reporting."
                .to_string(),
        ))
        .build(0)
}

fn missed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Missed Evaluations by Group")
        .query(q.get("infra.thanos.rule.missed").legend("{{group}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No rule group missed an evaluation in this range".to_string(),
        ))
        .build(0)
}

fn slowest_groups(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Slowest Groups (share of interval)")
        .query(q.get("infra.thanos.rule.slowest_groups").table_format())
        .transformations(vec![
            transform::organize_full(
                &["Time"],
                &["group", "Value"],
                &[("Value", "share of interval")],
            ),
            transform::sort_by("share of interval", true),
        ])
        .unit("percentunit")
        .no_value(no_ruler())
        .build(0)
}

fn loaded(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rules Loaded")
        .query(q.get("infra.thanos.rule.loaded").legend("{{pod}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_ruler())
        .build(0)
}

fn config_failed(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Config Reload Failures")
        .query(q.get("infra.thanos.rule.config_failed").legend("rulers"))
        .thresholds(threshold::errors(1.0, 2.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(no_ruler())
        .build(0)
}

fn alerts_sent(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Alerts Sent by Alertmanager")
        .query(
            q.get("infra.thanos.rule.alerts_sent")
                .legend("{{alertmanager}}"),
        )
        .unit("suffix:alerts/s")
        .min(0.0)
        .no_value(no_ruler())
        .build(0)
}

fn alerts_dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Alerts Dropped")
        .query(q.legended(
            "infra.thanos.rule.alerts_dropped",
            &["queue full", "send failed"],
        ))
        .unit("suffix:alerts/s")
        .min(0.0)
        .no_value(no_ruler())
        .build(0)
}

fn queue(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Alert Queue")
        .query(q.legended("infra.thanos.rule.queue", &["waiting", "capacity"]))
        .unit("short")
        .min(0.0)
        .no_value(no_ruler())
        .build(0)
}

fn sender_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Alertmanager Latency (p99)")
        .query(
            q.get("infra.thanos.rule.sender_latency")
                .legend("{{alertmanager}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No alerts were sent in this range".to_string(),
        ))
        .build(0)
}

fn remote_write_lag(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write-back Lag")
        .query(
            q.get("infra.thanos.rule.remote_write_lag")
                .legend("{{pod}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn remote_write_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write-back Failures")
        .query(
            q.get("infra.thanos.rule.remote_write_failures")
                .legend("{{pod}}"),
        )
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(no_ruler())
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
        assert_eq!(assembled.elements.len(), 12);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn rule_groups_are_named_by_the_group_alone() {
        // The raw label is the generated file's full path, a semicolon, then
        // the group. Legended raw, every line on a group panel starts with the
        // same sixty characters.
        let q = &test_queries();
        for panel in [failures(q), missed(q), slowest_groups(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains("label_replace"), "{json}");
            assert!(!json.contains("{{rule_group}}"), "{json}");
        }
    }
}
