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
use mzmon_lib::grafana::query::{PromQuery, query_group};

use super::{selector, theme};

/// The tab's theme.
const SHADE: &str = theme::KUBERNETES.shade;

pub fn rows() -> Vec<Row> {
    vec![
        resources_summary(),
        workload_readiness(),
        pod_metrics(),
        pod_networking(),
    ]
}

/// Build the two-query split every per-pod panel uses.
///
/// `build` receives a pod matcher and returns the expression for it. The first
/// query covers the selected cluster replicas, the second everything that is not
/// a replica pod — which is deliberately *not* narrowed by the cluster selectors,
/// since environmentd and the balancer belong to no cluster and should not vanish
/// when you focus on one.
///
/// The two matchers are the same pattern under `=~` and `!~`, so the split is
/// exhaustive and disjoint: no pod is missed and none is counted twice.
fn split_by_pod<F>(legend: &str, build: F) -> Vec<dashboardv2::DataQueryKind>
where
    F: Fn(&str) -> String,
{
    vec![
        PromQuery::new(build(&selector::replica_pods()))
            .legend(legend)
            .build(),
        PromQuery::new(build(&selector::non_replica_pods()))
            .legend(legend)
            .build(),
    ]
}

fn resources_summary() -> Row {
    Row::new("Resources Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("k8s-res-cpu-total", cpu_total())
            .panel("k8s-res-memory-total", memory_total()),
    )
}

fn workload_readiness() -> Row {
    Row::new("Workload Readiness").hide_header().grid(
        AutoGrid::new(5)
            .row_height(RowHeight::Short)
            .panel("resource-pod-status", pod_status())
            .panel("resource-statefulset-status", statefulset_status())
            .panel("resource-deployment-status", deployment_status()),
    )
}

fn pod_metrics() -> Row {
    Row::new("Pod Metrics").grid(
        AutoGrid::new(3)
            .panel("pod-cpu-percent", pod_cpu_percent())
            .panel("pod-memory-percent", pod_memory_percent()),
    )
}

fn pod_networking() -> Row {
    Row::new("Pod Networking").grid(
        AutoGrid::new(2)
            .panel("pod-network-rx", network_rx())
            .panel("pod-network-tx", network_tx())
            .panel("pod-network-errors", network_errors())
            .panel("pod-network-drops", network_drops()),
    )
}

fn cpu_total() -> dashboardv2::PanelKind {
    // Same shape as the Summary tab's panel, but over `containers()` rather than
    // `workload_containers()`: the exporter counts here.
    let expr = format!(
        r#"
sum by (container) (
    container_spec_cpu_quota{{ {containers} }}
    / container_spec_cpu_period{{ {containers} }}
)
"#,
        containers = selector::containers()
    );
    Panel::stat("Total CPU Capacity")
        .description(super::CPU_CAPACITY_DESCRIPTION)
        .data(query_group(vec![
            PromQuery::new(expr).legend("CPUs ({{container}})").build(),
        ]))
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("cores")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_total() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (container) (
    container_spec_memory_limit_bytes{{ {containers} }}
)
"#,
        containers = selector::containers()
    );
    Panel::stat("Total Memory")
        .description(super::MEMORY_TOTAL_DESCRIPTION)
        .data(query_group(vec![
            PromQuery::new(expr)
                .legend("Memory ({{container}})")
                .build(),
        ]))
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("bytes")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn pod_status() -> dashboardv2::PanelKind {
    // `max by (…)` over a `sum by (…, instance)`: kube-state-metrics can be
    // scraped by more than one job, and summing across `instance` then taking the
    // max keeps one job's view rather than adding the duplicates together.
    let expr = format!(
        r#"
max by (phase, namespace) (
    sum by (phase, namespace, instance) (
        kube_pod_status_phase{{{ns}}}
    )
)
"#,
        ns = selector::namespace()
    );
    Panel::piechart("Pod Readiness")
        .description(
            "**Pods in the Materialize namespace grouped by phase** (Running, Pending, Failed, \
             etc.). Nominal: nearly all `Running`. Pods stuck in `Pending` usually mean \
             Kubernetes can't schedule them (capacity, taints, AZ constraints); `Failed` means \
             a container exited and won't be restarted. Pairs with _Last Restart Time_ on the \
             Summary tab. Requires kube-state-metrics.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).instant().legend("{{phase}}").build(),
        ]))
        .shade(SHADE)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn statefulset_status() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
max by (namespace) (
    sum by (namespace, instance) (
        kube_statefulset_status_replicas_ready{{{ns}}}
    )
)
"#,
        ns = selector::namespace()
    );
    Panel::piechart("StatefulSet Readiness")
        .description(
            "**Number of StatefulSet replicas reporting Ready.** environmentd and Materialize's \
             cluster pods are StatefulSets; this panel counts the replicas that have reached \
             the Ready state. Nominal: matches the configured replica count. A drop indicates a \
             pod stuck in initialization or hydration. Requires kube-state-metrics.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).instant().legend("Ready").build(),
        ]))
        .shade(SHADE)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn deployment_status() -> dashboardv2::PanelKind {
    let ready = format!(
        r#"
max by (namespace) (
    sum by (namespace, instance) (
        kube_deployment_status_replicas_ready{{{ns}}}
    )
)
"#,
        ns = selector::namespace()
    );
    let unavailable = format!(
        r#"
max by (namespace) (
    sum by (namespace, instance) (
        kube_deployment_status_replicas_unavailable{{{ns}}}
    )
)
"#,
        ns = selector::namespace()
    );
    Panel::piechart("Deployment Readiness")
        .description(
            "**Deployment replica health — Ready vs Unavailable.** Deployments back stateless \
             services (e.g., the promsql exporter). Nominal: all replicas Ready, zero \
             Unavailable. Unavailable counts indicate failed rollouts or crashing pods. \
             Requires kube-state-metrics.",
        )
        .data(query_group(vec![
            PromQuery::new(ready).instant().legend("Ready").build(),
            PromQuery::new(unavailable)
                .instant()
                .legend("Unavailable")
                .build(),
        ]))
        .shade(SHADE)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn pod_cpu_percent() -> dashboardv2::PanelKind {
    let build = |pods: &str| {
        format!(
            r#"
sum by (namespace, pod, container) (
    rate(
        container_cpu_usage_seconds_total{{{containers}, {pods}}}[$__rate_interval]
    )
) / sum by (namespace, pod, container) (
    kube_pod_container_resource_limits{{resource="cpu", {ns}, {pods}}}
)
"#,
            containers = selector::containers(),
            ns = selector::namespace()
        )
    };
    Panel::timeseries("Pod CPU Usage")
        .description(
            "**CPU utilization per pod, as a fraction of the pod's CPU limit.** Two-query \
             split: one for cluster replica pods (filtered by the dashboard's cluster/replica \
             selectors), one for everything else (envd, balancer, exporter, etc.). Sustained \
             near 1.0 for a pod means it's CPU-bound. For the Materialize-level cause see \
             _Compute Objects -> Dataflow Elapsed Rate_ or _Arrangements_.",
        )
        .data(query_group(split_by_pod("{{pod}} / {{container}}", build)))
        .unit("percentunit")
        // Needs both exporters: the numerator is cAdvisor, the denominator
        // kube-state-metrics.
        .no_value(NoValue::RequiresCAdvisorAndKubeStateMetrics)
        .build(0)
}

fn pod_memory_percent() -> dashboardv2::PanelKind {
    // Averaged per pod on both sides so a pod scraped twice does not skew the
    // ratio.
    let build = |pods: &str| {
        format!(
            r#"
avg by (namespace, pod, container) (
    container_memory_working_set_bytes{{{containers}, {pods}}}
) / avg by (namespace, pod, container) (
    container_spec_memory_limit_bytes{{{containers}, {pods}}}
)
"#,
            containers = selector::workload_containers()
        )
    };
    Panel::timeseries("Pod Memory Usage")
        .description(
            "**Memory usage per pod, as a fraction of the pod's memory limit (working-set \
             basis).** Same two-query split as Pod CPU Usage. **Sustained climb toward 1.0 is \
             dangerous** — a pod hitting its memory limit gets OOM-killed, which on a compute \
             replica triggers a hydration cycle (in-memory state rebuilt from persistence, \
             often minutes). If a Materialize cluster pod is the offender, _Compute Objects -> \
             Arrangements_ shows which arrangements consume the memory.",
        )
        .data(query_group(split_by_pod("{{pod}} / {{container}}", build)))
        .unit("percentunit")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

/// A per-pod network rate, for one cAdvisor counter.
fn network_rate(metric: &str) -> impl Fn(&str) -> String + '_ {
    move |pods: &str| {
        format!(
            r#"
sum by (namespace, pod) (
    rate(
        {metric}{{{ns}, {pods}}}[$__rate_interval]
    )
)
"#,
            ns = selector::namespace()
        )
    }
}

fn network_rx() -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Rx")
        .description(
            "**Network bytes/sec received per pod**, aggregated across all network interfaces. \
             Same cluster/non-cluster split as the pod CPU/memory panels. For cluster pods, Rx \
             tracks ingest from upstream (Kafka, Postgres, etc.) and inter-pod replication; for \
             envd and the balancer it reflects client SQL traffic. Surges that coincide with \
             _Compute Objects -> Hydration_ activity are normal (catchup); surges otherwise can \
             mean a runaway client or source.",
        )
        .data(query_group(split_by_pod(
            "{{pod}}",
            network_rate("container_network_receive_bytes_total"),
        )))
        .unit("Bps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn network_tx() -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Tx")
        .description(
            "**Network bytes/sec transmitted per pod**, aggregated across interfaces. For \
             cluster pods Tx covers sink output, inter-pod replication, and query results \
             returning to envd; for envd it's client query responses. Pairs with _Sources and \
             Sinks -> Sink Throughput_ when investigating sink-side bandwidth.",
        )
        .data(query_group(split_by_pod(
            "{{pod}}",
            network_rate("container_network_transmit_bytes_total"),
        )))
        .unit("Bps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

/// An rx+tx pair, each split by pod: four queries.
fn rx_tx_split(rx_metric: &str, tx_metric: &str) -> Vec<dashboardv2::DataQueryKind> {
    let mut queries = split_by_pod("{{pod}} rx", network_rate(rx_metric));
    queries.extend(split_by_pod("{{pod}} tx", network_rate(tx_metric)));
    queries
}

fn network_errors() -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Errors")
        .description(
            "**Network rx + tx errors per pod per second** (errors counted at the NIC/kernel \
             level). Nominal: 0. Non-zero is unusual and points at infrastructure problems \
             (faulty NIC, kernel network stack issues, container runtime bugs) — not \
             Materialize-level. If you see persistent non-zero, file an infra ticket; this \
             isn't fixable from within Materialize.",
        )
        .data(query_group(rx_tx_split(
            "container_network_receive_errors_total",
            "container_network_transmit_errors_total",
        )))
        .unit("cps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn network_drops() -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Network Packet Drops")
        .description(
            "**Network packets dropped (rx + tx) per pod per second.** Drops happen when the \
             kernel's network buffers fill up faster than the application can read from them \
             (rx) or when egress rate-limiting kicks in (tx). Nominal: 0. Low-level non-zero \
             drops (single-digit pps) are usually harmless background noise; sustained higher \
             rates indicate the pod is overwhelmed at the network layer — often paired with \
             elevated _Pod CPU Usage_.",
        )
        .data(query_group(rx_tx_split(
            "container_network_receive_packets_dropped_total",
            "container_network_transmit_packets_dropped_total",
        )))
        .unit("pps")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_has_four_rows_and_eleven_panels() {
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 11);
    }

    #[test]
    fn every_pod_panel_splits_replicas_from_the_rest() {
        // The split is the tab's defining shape; a panel that lost one half would
        // silently stop showing either the replicas or everything else.
        for (name, panel, expected) in [
            ("pod-cpu-percent", pod_cpu_percent(), 2),
            ("pod-memory-percent", pod_memory_percent(), 2),
            ("pod-network-rx", network_rx(), 2),
            ("pod-network-tx", network_tx(), 2),
            ("pod-network-errors", network_errors(), 4),
            ("pod-network-drops", network_drops(), 4),
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
        // environmentd and the balancer belong to no cluster; narrowing them by the
        // cluster selection would make them disappear when you focus on one.
        let panel = network_rx();
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
        // The Summary tab excludes it to read as user-workload capacity; this tab
        // is about what is actually running.
        for panel in [cpu_total(), memory_total()] {
            let expr = panel.spec.data.spec.queries[0]
                .spec
                .query
                .spec
                .as_ref()
                .and_then(|s| s.get("expr"))
                .and_then(|e| e.as_str())
                .unwrap_or_default();
            assert!(
                !expr.contains(selector::SQL_EXPORTER_CONTAINER),
                "the exporter should not be excluded here:\n{expr}"
            );
        }
    }

    #[test]
    fn the_cpu_panel_needs_both_exporters() {
        // Numerator from cAdvisor, denominator from kube-state-metrics: naming only
        // one would send someone to fix the wrong scrape target.
        let panel = pod_cpu_percent();
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
