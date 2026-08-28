// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Kubernetes Workloads tab: the pods underneath Materialize.
//!
//! Reads cAdvisor and kube-state-metrics, neither of which knows anything about
//! Materialize's catalog. So the cluster and replica selection has to be applied
//! to pod *names* rather than to a cluster-id label, and every per-pod panel is
//! split in two: the selected cluster replicas, and everything else
//! (environmentd, the balancer, the exporter). See [`split_by_pod`].
//!
//! Unlike the Summary tab, capacity here *includes* the monitoring exporter — the
//! question on this tab is what is actually running, not what the user workload
//! was given.

use mzmon_lib::grafana::generated::{dashboardv2, stat};
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;

/// The tab's theme.
const SHADE: &str = theme::KUBERNETES.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        resources_summary(q),
        workload_readiness(q),
        pod_metrics(q),
        pod_networking(q),
    ]
}

fn resources_summary(q: &Queries) -> Row {
    Row::new("Resources Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("k8s-res-cpu-total", cpu_total(q))
            .panel("k8s-res-memory-total", memory_total(q)),
    )
}

fn workload_readiness(q: &Queries) -> Row {
    Row::new("Workload Readiness").hide_header().grid(
        AutoGrid::new(5)
            .row_height(RowHeight::Short)
            .panel("resource-pod-status", pod_status(q))
            .panel("resource-statefulset-status", statefulset_status(q))
            .panel("resource-deployment-status", deployment_status(q)),
    )
}

fn pod_metrics(q: &Queries) -> Row {
    Row::new("Pod Metrics").grid(
        AutoGrid::new(3)
            .panel("pod-cpu-percent", pod_cpu_percent(q))
            .panel("pod-memory-percent", pod_memory_percent(q)),
    )
}

fn pod_networking(q: &Queries) -> Row {
    Row::new("Pod Networking").grid(
        AutoGrid::new(2)
            .panel("pod-network-rx", network_rx(q))
            .panel("pod-network-tx", network_tx(q))
            .panel("pod-network-errors", network_errors(q))
            .panel("pod-network-drops", network_drops(q)),
    )
}

fn cpu_total(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Total CPU Capacity")
        .query(
            q.get("materialize.kubernetes.cpu.capacity.all_containers")
                .legend("CPUs ({{container}})"),
        )
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("cores")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_total(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Total Memory")
        .query(
            q.get("materialize.kubernetes.memory.capacity.all_containers")
                .legend("Memory ({{container}})"),
        )
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("bytes")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn pod_status(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Pod Readiness")
        .query(
            q.get("materialize.kubernetes.pods.readiness")
                .legend("{{phase}}"),
        )
        .shade(SHADE)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn statefulset_status(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("StatefulSet Readiness")
        .query(
            q.get("materialize.kubernetes.statefulsets.ready")
                .legend("Ready"),
        )
        .shade(SHADE)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn deployment_status(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Deployment Readiness")
        .query(q.legended(
            "materialize.kubernetes.deployments.readiness",
            &["Ready", "Unavailable"],
        ))
        .shade(SHADE)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn pod_cpu_percent(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod CPU Usage")
        .query(
            q.get("materialize.kubernetes.pods.cpu_usage")
                .legend("{{pod}} / {{container}}"),
        )
        .unit("percentunit")
        // Needs both exporters: the numerator is cAdvisor, the denominator
        // kube-state-metrics.
        .no_value(NoValue::RequiresCAdvisorAndKubeStateMetrics)
        .build(0)
}

fn pod_memory_percent(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Memory Usage")
        .query(
            q.get("materialize.kubernetes.pods.memory_usage")
                .legend("{{pod}} / {{container}}"),
        )
        .unit("percentunit")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn network_rx(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Rx")
        .query(
            q.get("materialize.kubernetes.pods.network_rx")
                .legend("{{pod}}"),
        )
        .unit("Bps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn network_tx(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Tx")
        .query(
            q.get("materialize.kubernetes.pods.network_tx")
                .legend("{{pod}}"),
        )
        .unit("Bps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn network_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Errors")
        .query(q.legended(
            "materialize.kubernetes.pods.network_errors",
            &["{{pod}} rx", "{{pod}} rx", "{{pod}} tx", "{{pod}} tx"],
        ))
        .unit("cps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn network_drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Packet Drops")
        .query(q.legended(
            "materialize.kubernetes.pods.network_drops",
            &["{{pod}} rx", "{{pod}} rx", "{{pod}} tx", "{{pod}} tx"],
        ))
        .unit("pps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_has_four_rows_and_eleven_panels() {
        let q = &crate::grafana::queries::test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 11);
    }

    #[test]
    fn every_pod_panel_splits_replicas_from_the_rest() {
        let q = &crate::grafana::queries::test_queries();
        // The split is the tab's defining shape; a panel that lost one half would
        // silently stop showing either the replicas or everything else.
        for (name, panel, expected) in [
            ("pod-cpu-percent", pod_cpu_percent(q), 2),
            ("pod-memory-percent", pod_memory_percent(q), 2),
            ("pod-network-rx", network_rx(q), 2),
            ("pod-network-tx", network_tx(q), 2),
            ("pod-network-errors", network_errors(q), 4),
            ("pod-network-drops", network_drops(q), 4),
        ] {
            assert_eq!(
                panel.spec.data.spec.queries.len(),
                expected,
                "{name} query count"
            );
            let exprs: Vec<String> = panel
                .spec
                .data
                .spec
                .queries
                .iter()
                .map(|q| {
                    q.spec
                        .query
                        .spec
                        .as_ref()
                        .and_then(|s| s.get("expr"))
                        .and_then(|e| e.as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect();
            // Half select replica pods, half exclude them.
            let selected = exprs.iter().filter(|e| e.contains("pod=~")).count();
            let excluded = exprs.iter().filter(|e| e.contains("pod!~")).count();
            assert_eq!(selected, expected / 2, "{name} replica-pod queries");
            assert_eq!(excluded, expected / 2, "{name} non-replica-pod queries");
        }
    }

    #[test]
    fn the_non_replica_half_is_not_narrowed_by_the_cluster_selectors() {
        let q = &crate::grafana::queries::test_queries();
        // environmentd and the balancer belong to no cluster; narrowing them by the
        // cluster selection would make them disappear when you focus on one.
        let panel = network_rx(q);
        let excluded = panel
            .spec
            .data
            .spec
            .queries
            .iter()
            .filter_map(|q| {
                q.spec
                    .query
                    .spec
                    .as_ref()
                    .and_then(|s| s.get("expr"))
                    .and_then(|e| e.as_str())
            })
            .find(|e| e.contains("pod!~"))
            .expect("a non-replica query");
        assert!(!excluded.contains("mzClusterList"), "{excluded}");
        assert!(!excluded.contains("mzReplicaList"), "{excluded}");
    }

    #[test]
    fn capacity_here_includes_the_monitoring_exporter() {
        let q = &crate::grafana::queries::test_queries();
        // The Summary tab excludes it to read as user-workload capacity; this tab
        // is about what is actually running.
        for panel in [cpu_total(q), memory_total(q)] {
            let expr = panel.spec.data.spec.queries[0]
                .spec
                .query
                .spec
                .as_ref()
                .and_then(|s| s.get("expr"))
                .and_then(|e| e.as_str())
                .unwrap_or_default();
            assert!(
                !expr.contains(super::super::selector::SQL_EXPORTER_CONTAINER),
                "the exporter should not be excluded here:\n{expr}"
            );
        }
    }

    #[test]
    fn the_cpu_panel_needs_both_exporters() {
        let q = &crate::grafana::queries::test_queries();
        // Numerator from cAdvisor, denominator from kube-state-metrics: naming only
        // one would send someone to fix the wrong scrape target.
        let panel = pod_cpu_percent(q);
        assert_eq!(
            panel
                .spec
                .viz_config
                .spec
                .field_config
                .defaults
                .no_value
                .as_deref(),
            Some("No metrics: cadvisor and kube-state-metrics are required")
        );
    }
}
