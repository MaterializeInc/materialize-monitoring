// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Components tab: the controller running each pipeline, the configuration
//! it was given, and the cluster the gateways form.
//!
//! Alloy runs a pipeline as a graph of components that re-evaluate when their
//! inputs change. Health comes first because an unhealthy component is a stage
//! that has stopped passing data on. Configuration next, because a collector
//! that failed to load a new configuration keeps running the old one and
//! otherwise looks healthy. Evaluation after that, which is where a controller
//! that cannot keep up shows. Clustering last: it applies to the gateway alone,
//! and its failures show first on the Metric Pipeline tab as duplicate samples
//! or missing targets.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::overview::not_collected;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![health(q), configuration(q), evaluation(q), clustering(q)]
}

fn health(q: &Queries) -> Row {
    Row::new("Health").grid(
        AutoGrid::new(2)
            .panel("components-not-healthy", not_healthy(q))
            .panel("components-running", running(q)),
    )
}

fn configuration(q: &Queries) -> Row {
    Row::new("Configuration").grid(
        AutoGrid::new(2)
            .panel("components-config-by-pod", config_by_pod(q))
            .panel("components-distinct-configs", distinct_configs(q)),
    )
}

fn evaluation(q: &Queries) -> Row {
    Row::new("Evaluation").grid(
        AutoGrid::new(2)
            .panel("components-eval-rate", evaluation_rate(q))
            .panel("components-eval-latency", evaluation_latency(q))
            .panel("components-eval-slow", slow(q))
            .panel("components-eval-queue", queue(q)),
    )
}

fn clustering(q: &Queries) -> Row {
    Row::new("Clustering").grid(
        AutoGrid::new(3)
            .panel("cluster-peers", peers(q))
            .panel("cluster-health-score", health_score(q))
            .panel("cluster-targets", targets(q))
            .panel("cluster-transport-failures", transport_failures(q))
            .panel("cluster-members", members(q)),
    )
}

/// What a clustering panel shows when only agents are selected.
fn gateway_only() -> NoValue {
    NoValue::Custom("Only the gateway runs clustered".to_string())
}

fn not_healthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Components Not Healthy")
        .query(
            q.get("infra.alloy.components.not_healthy")
                .legend("{{pod}} {{health_type}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom("Every component is healthy".to_string()))
        .build(0)
}

fn running(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Components Running")
        .query(q.get("infra.alloy.components.running").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn config_by_pod(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Configuration by Collector")
        .query(q.get("infra.alloy.components.config_by_pod").table_format())
        .transformations(vec![transform::organize_full(
            &["Time"],
            &["app", "pod", "sha256", "Value"],
            &[("sha256", "config hash"), ("Value", "last load succeeded")],
        )])
        .unit("bool_yes_no")
        .no_value(not_collected())
        .build(0)
}

fn distinct_configs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Distinct Configurations")
        .query(
            q.get("infra.alloy.components.distinct_configs")
                .legend("{{app}}"),
        )
        .unit("short")
        .min(0.0)
        .decimals(0.0)
        .no_value(not_collected())
        .build(0)
}

fn evaluation_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Evaluation Rate")
        .query(
            q.get("infra.alloy.components.evaluation_rate")
                .legend("{{app}}"),
        )
        .unit("suffix:evaluations/s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn evaluation_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Evaluation Latency (p99)")
        .query(
            q.get("infra.alloy.components.evaluation_latency")
                .legend("{{app}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn slow(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Slow Components")
        .query(
            q.get("infra.alloy.components.slow")
                .legend("{{app}} {{component_id}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No component has evaluated slowly in this range".to_string(),
        ))
        .build(0)
}

fn queue(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Evaluation Queue")
        .query(q.get("infra.alloy.components.queue").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn peers(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Peers Seen")
        .query(q.legended(
            "infra.alloy.cluster.peers",
            &["{{pod}} sees", "gateways running"],
        ))
        .unit("short")
        .min(0.0)
        .decimals(0.0)
        .no_value(gateway_only())
        .build(0)
}

fn health_score(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Gossip Health Score")
        .query(q.get("infra.alloy.cluster.health_score").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(gateway_only())
        .build(0)
}

fn targets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Targets per Gateway")
        .query(q.get("infra.alloy.cluster.targets").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(gateway_only())
        .build(0)
}

fn transport_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Gossip Send Failures")
        .query(
            q.get("infra.alloy.cluster.transport_failures")
                .legend("{{pod}}"),
        )
        .unit("suffix:messages/s")
        .min(0.0)
        .no_value(gateway_only())
        .build(0)
}

fn members(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Cluster Members")
        .query(q.get("infra.alloy.cluster.members").table_format())
        .transformations(vec![transform::organize_full(
            &["Time"],
            &["pod", "state", "Value"],
            &[("Value", "ready for traffic")],
        )])
        .unit("bool_yes_no")
        .no_value(gateway_only())
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
    fn the_tables_of_current_facts_are_instant_and_render_as_one_table() {
        let q = &test_queries();
        for panel in [config_by_pod(q), members(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{json}");
            assert!(json.contains(r#""format":"table""#), "{json}");
        }
    }
}
