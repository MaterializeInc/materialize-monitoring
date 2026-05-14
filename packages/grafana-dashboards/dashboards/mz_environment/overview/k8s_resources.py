"""Kubernetes Resources Tab for the Overview Dashboard."""

from __future__ import annotations

import textwrap

from grafana_foundation_sdk.builders import (
    piechart as piechart_builder,
)
from grafana_foundation_sdk.builders import (
    timeseries,
)
from grafana_foundation_sdk.models import common, piechart
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import promql_query, query_group

from dashboards import palette, visualization

CADVISOR_MISSING = "No metrics: cadvisor/node-exporter is required"
KSM_MISSING = "No metrics: kube-state-metrics is required"

# Pod-name regex matchers for cluster-replica pods vs everything else.
# Used as PromQL label values inside pod=~"…" / pod!~"…" matchers.
# `${var:regex}` expands multi-select dashboard variables into a proper
# `(val1|val2|…)` alternation (bare `$var` does not, when embedded in a
# wider regex string — see Grafana variable interpolation docs).
CLUSTER_POD_RE = ".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"
NONCLUSTER_POD_RE = ".*-cluster-.*-replica-.*"

K8S_THEME = palette.THEME_PALETTE[0]


class KubeResourcesMixin:
    """Shared panels between KubeResources tab and Overview Summary tab."""

    dashboard: MzDashboard
    panel_id_prefix: str

    def cpu_total_panel(self, *, include_monitoring: bool = False):
        """Get a panel showing a holistic summary of compute (CPUs).

        We show a stat for total cores available.
        """
        panel_id = f"{self.panel_id_prefix}-cpu-total"
        metric_filter = "$containerFilter"
        if not include_monitoring:
            metric_filter += ', container!="new-promsql-exporter"'
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (container) (
                        container_spec_cpu_quota{{ {metric_filter} }}
                        / container_spec_cpu_period{{ {metric_filter} }}
                    )
                    """
                ),
            ).legend_format("CPUs ({{container}})")
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Total CPU Capacity")
            .description(
                "**Total CPU cores configured across containers in the "
                "selected scope** (sum of CPU limits from cAdvisor). "
                "Steps correlate with `ALTER CLUSTER REPLICA SIZE`, "
                "`CREATE`/`DROP CLUSTER REPLICA`, or pod restarts. On "
                "the Summary tab the monitoring exporter is excluded "
                "(so this reflects user-workload capacity); on the "
                "Kubernetes Workloads tab it's included."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=K8S_THEME)
                .unit("cores")
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .no_value(CADVISOR_MISSING)
            ),
        )
        return panel_id

    def memory_totals_panel(self, *, include_monitoring: bool = False):
        """Get a panel showing a holistic summary of memory.

        We show a stat for total memory available.

        FIXME: we don't have a swap totals available...
        """
        panel_id = f"{self.panel_id_prefix}-memory-total"
        metric_filter = "$containerFilter"
        if not include_monitoring:
            metric_filter += ', container!="new-promsql-exporter"'
        query = query_group(
            promql_query(
                # 'sum by (container) (mz_memory_limiter_memory_limit_bytes{materialize_cloud_organization_id="$environmentId"})'
                textwrap.dedent(
                    f"""
                    sum by (container) (
                        container_spec_memory_limit_bytes{{ {metric_filter} }}
                    )
                    """
                ),
            ).legend_format("Memory ({{container}})")
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Total Memory")
            .description(
                "**Total memory configured across containers in the "
                "selected scope** (sum of memory limits from "
                "cAdvisor). Memory is the dominant constraint on "
                "Materialize: in-memory arrangements (see _Compute "
                "Objects -> Arrangements_) live in here. Steps "
                "correlate with `ALTER CLUSTER REPLICA SIZE` or pod "
                "restarts."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=K8S_THEME)
                .unit("bytes")
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .no_value(CADVISOR_MISSING)
            ),
        )
        return panel_id


class KubeResourcesTab(KubeResourcesMixin):
    """Kubernetes resources tab on Overview Dashboard."""

    panel_id_prefix = "k8s-res"

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _ready_pods_panel(self):
        """Show a breakdown of Pods by readiness phase.

        We use a donut pie chart to easily denote status.
        """
        panel_id = "resource-pod-status"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    max by (phase, namespace) (
                        sum by (phase, namespace, instance) (
                            kube_pod_status_phase{namespace=~"$mzNamespaceList"}
                        )
                    )
                    """
                )
            )
            .legend_format("{{phase}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod Readiness")
            .description(
                "**Pods in the Materialize namespace grouped by "
                "phase** (Running, Pending, Failed, etc.). Nominal: "
                "nearly all `Running`. Pods stuck in `Pending` usually "
                "mean Kubernetes can't schedule them (capacity, taints, "
                "AZ constraints); `Failed` means a container exited "
                "and won't be restarted. Pairs with _Last Restart "
                "Time_ on the Summary tab. Requires kube-state-metrics."
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(K8S_THEME)
                )
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .no_value(KSM_MISSING)
            ),
        )
        return panel_id

    def _ready_statefulsets_panel(self):
        """Show a breakdown of StatefulSets by readiness."""
        panel_id = "resource-statefulset-status"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    max by (namespace) (
                        sum by (namespace, instance) (
                            kube_statefulset_status_replicas_ready{namespace=~"$mzNamespaceList"}
                        )
                    )
                    """
                )
            )
            .legend_format("Ready")
            .instant(),
            # TODO: statefulset unready
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("StatefulSet Readiness")
            .description(
                "**Number of StatefulSet replicas reporting Ready.** "
                "environmentd and Materialize's cluster pods are "
                "StatefulSets; this panel counts the replicas that "
                "have reached the Ready state. Nominal: matches the "
                "configured replica count. A drop indicates a pod "
                "stuck in initialization or hydration. Requires "
                "kube-state-metrics."
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(K8S_THEME)
                )
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .no_value(KSM_MISSING)
            ),
        )
        return panel_id

    def _ready_deployments_panel(self):
        """Show a breakdown of Deployments by readiness."""
        panel_id = "resource-deployment-status"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    max by (namespace) (
                        sum by (namespace, instance) (
                            kube_deployment_status_replicas_ready{namespace=~"$mzNamespaceList"}
                        )
                    )
                    """
                )
            )
            .legend_format("Ready")
            .instant(),
            promql_query(
                textwrap.dedent(
                    """
                    max by (namespace) (
                        sum by (namespace, instance) (
                            kube_deployment_status_replicas_unavailable{namespace=~"$mzNamespaceList"}
                        )
                    )
                    """
                )
            )
            .legend_format("Unavailable")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Deployment Readiness")
            .description(
                "**Deployment replica health — Ready vs Unavailable.** "
                "Deployments back stateless services (e.g., the "
                "promsql exporter). Nominal: all replicas Ready, zero "
                "Unavailable. Unavailable counts indicate failed "
                "rollouts or crashing pods. Requires kube-state-metrics."
            )
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .pie_type(piechart.PieChartType.DONUT)
                .legend(visualization.PIE_LEGEND_BUILDER)
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(K8S_THEME)
                )
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .no_value(KSM_MISSING)
            ),
        )
        return panel_id

    def _pod_cpu_percent_panel(self):
        """Show CPU usage (used / limit) for pods as a timeseries.

        Split into two queries so the cluster/replica selectors still filter
        cluster-replica pods, while non-cluster pods (envd, balancer, etc.)
        stay visible regardless of the cluster/replica selection.
        """
        panel_id = "pod-cpu-percent"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod, container) (
                        rate(
                            container_cpu_usage_seconds_total{{$containerFilter, pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    ) / sum by (namespace, pod, container) (
                        kube_pod_container_resource_limits{{resource="cpu", namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}
                    )
                    """
                )
            ).legend_format("{{pod}} / {{container}}"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod, container) (
                        rate(
                            container_cpu_usage_seconds_total{{$containerFilter, pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    ) / sum by (namespace, pod, container) (
                        kube_pod_container_resource_limits{{resource="cpu", namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}
                    )
                    """
                )
            ).legend_format("{{pod}} / {{container}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod CPU Usage")
            .description(
                "**CPU utilization per pod, as a fraction of the "
                "pod's CPU limit.** Two-query split: one for cluster "
                "replica pods (filtered by the dashboard's "
                "cluster/replica selectors), one for everything else "
                "(envd, balancer, exporter, etc.). Sustained near 1.0 "
                "for a pod means it's CPU-bound. For the "
                "Materialize-level cause see _Compute Objects -> "
                "Dataflow Elapsed Rate_ or _Arrangements_."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("percentunit")
                .no_value(CADVISOR_MISSING)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _pod_memory_percent_panel(self):
        """Show memory usage (used / limit) for pods as a timeseries.

        Split into two queries so the cluster/replica selectors still filter
        cluster-replica pods, while non-cluster pods (envd, balancer, etc.)
        stay visible regardless of the cluster/replica selection.
        """
        panel_id = "pod-memory-percent"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    avg by (namespace, pod, container) (
                        container_memory_working_set_bytes{{$containerFilter, container!="new-promsql-exporter", pod=~"{CLUSTER_POD_RE}"}}
                    ) / avg by (namespace, pod, container) (
                        container_spec_memory_limit_bytes{{$containerFilter, container!="new-promsql-exporter", pod=~"{CLUSTER_POD_RE}"}}
                    )
                    """
                )
            ).legend_format("{{pod}} / {{container}}"),
            promql_query(
                textwrap.dedent(
                    f"""
                    avg by (namespace, pod, container) (
                        container_memory_working_set_bytes{{$containerFilter, container!="new-promsql-exporter", pod!~"{NONCLUSTER_POD_RE}"}}
                    ) / avg by (namespace, pod, container) (
                        container_spec_memory_limit_bytes{{$containerFilter, container!="new-promsql-exporter", pod!~"{NONCLUSTER_POD_RE}"}}
                    )
                    """
                )
            ).legend_format("{{pod}} / {{container}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod Memory Usage")
            .description(
                "**Memory usage per pod, as a fraction of the pod's "
                "memory limit (working-set basis).** Same two-query "
                "split as Pod CPU Usage. **Sustained climb toward 1.0 "
                "is dangerous** — a pod hitting its memory limit "
                "gets OOM-killed, which on a compute replica triggers "
                "a hydration cycle (in-memory state rebuilt from "
                "persistence, often minutes). If a Materialize cluster "
                "pod is the offender, _Compute Objects -> "
                "Arrangements_ shows which arrangements consume the "
                "memory."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("percentunit")
                .no_value(CADVISOR_MISSING)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _pod_network_rx_panel(self):
        """Show network receive bandwidth per pod as a timeseries."""
        panel_id = "pod-network-rx"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_receive_bytes_total{{namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}}"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_receive_bytes_total{{namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod Network Rx")
            .description(
                "**Network bytes/sec received per pod**, aggregated "
                "across all network interfaces. Same cluster/non-cluster "
                "split as the pod CPU/memory panels. For cluster pods, "
                "Rx tracks ingest from upstream (Kafka, Postgres, "
                "etc.) and inter-pod replication; for envd and the "
                "balancer it reflects client SQL traffic. Surges that "
                "coincide with _Compute Objects -> Hydration_ activity "
                "are normal (catchup); surges otherwise can mean a "
                "runaway client or source."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("Bps")
                .no_value(CADVISOR_MISSING)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _pod_network_tx_panel(self):
        """Show network transmit bandwidth per pod as a timeseries."""
        panel_id = "pod-network-tx"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_transmit_bytes_total{{namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}}"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_transmit_bytes_total{{namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}}"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod Network Tx")
            .description(
                "**Network bytes/sec transmitted per pod**, "
                "aggregated across interfaces. For cluster pods Tx "
                "covers sink output, inter-pod replication, and query "
                "results returning to envd; for envd it's client "
                "query responses. Pairs with _Storage Objects -> Sink "
                "Throughput_ when investigating sink-side bandwidth."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("Bps")
                .no_value(CADVISOR_MISSING)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _pod_network_errors_panel(self):
        """Show combined network rx + tx errors per pod as a timeseries.

        Four queries: rx and tx, each split into cluster-replica pods (filtered
        by selectors) and non-cluster pods (always shown). The legend suffix
        (rx/tx) distinguishes the direction in the chart.
        """
        panel_id = "pod-network-errors"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_receive_errors_total{{namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} rx"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_receive_errors_total{{namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} rx"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_transmit_errors_total{{namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} tx"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_transmit_errors_total{{namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} tx"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod Network Errors")
            .description(
                "**Network rx + tx errors per pod per second** "
                "(errors counted at the NIC/kernel level). Nominal: "
                "0. Non-zero is unusual and points at infrastructure "
                "problems (faulty NIC, kernel network stack issues, "
                "container runtime bugs) — not Materialize-level. If "
                "you see persistent non-zero, file an infra ticket; "
                "this isn't fixable from within Materialize."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("cps")
                .no_value(CADVISOR_MISSING)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def _pod_network_drops_panel(self):
        """Show combined network rx + tx dropped packets per pod.

        Four queries: rx and tx, each split into cluster-replica pods (filtered
        by selectors) and non-cluster pods (always shown). The legend suffix
        (rx/tx) distinguishes the direction in the chart.
        """
        panel_id = "pod-network-drops"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_receive_packets_dropped_total{{namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} rx"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_receive_packets_dropped_total{{namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} rx"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_transmit_packets_dropped_total{{namespace=~"$mzNamespaceList", pod=~"{CLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} tx"),
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (namespace, pod) (
                        rate(
                            container_network_transmit_packets_dropped_total{{namespace=~"$mzNamespaceList", pod!~"{NONCLUSTER_POD_RE}"}}[$__rate_interval]
                        )
                    )
                    """
                )
            ).legend_format("{{pod}} tx"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Pod Network Packet Drops")
            .description(
                "**Network packets dropped (rx + tx) per pod per "
                "second.** Drops happen when the kernel's network "
                "buffers fill up faster than the application can read "
                "from them (rx) or when egress rate-limiting kicks in "
                "(tx). Nominal: 0. Low-level non-zero drops "
                "(single-digit pps) are usually harmless background "
                "noise; sustained higher rates indicate the pod is "
                "overwhelmed at the network layer — often paired with "
                "elevated _Pod CPU Usage_."
            )
            .data(query)
            .visualization(
                timeseries.Visualization()
                .unit("pps")
                .no_value(CADVISOR_MISSING)
                .legend(visualization.TS_LEGEND_BUILDER)
            ),
        )
        return panel_id

    def build_k8s_resources_summary_row(self) -> dashboardv2_builders.Row:
        """Get a row showing a summary of Kubernetes resources."""
        return (
            dashboardv2_builders.Row()
            .title("Resources Summary")
            .hide_header(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .with_item(self.cpu_total_panel(include_monitoring=True))
                .with_item(self.memory_totals_panel(include_monitoring=True))
            )
        )

    def build_k8s_readiness_row(self) -> dashboardv2_builders.Row:
        """Get a row showing Kubernetes resource readiness."""
        return (
            dashboardv2_builders.Row()
            .title("Workload Readiness")
            .hide_header(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .max_column_count(5)  # leave room for Services, etc
                .with_item(self._ready_pods_panel())
                .with_item(self._ready_statefulsets_panel())
                .with_item(self._ready_deployments_panel())
            )
        )

    def build_pod_metrics_row(self) -> dashboardv2_builders.Row:
        """Get a row showing drilldowns into Kubernetes resource metrics."""
        return (
            dashboardv2_builders.Row()
            .title("Pod Metrics")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .with_item(self._pod_cpu_percent_panel())
                .with_item(self._pod_memory_percent_panel())
            )
        )

    def build_pod_network_row(self) -> dashboardv2_builders.Row:
        """Get a row showing per-pod network bandwidth, errors, and drops.

        Rendered as a 2-column auto grid so each panel has room for its
        per-series legend table without overflow; with four panels this lays
        out as a 2x2.
        """
        return (
            dashboardv2_builders.Row()
            .title("Pod Networking")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .max_column_count(2)
                .with_item(self._pod_network_rx_panel())
                .with_item(self._pod_network_tx_panel())
                .with_item(self._pod_network_errors_panel())
                .with_item(self._pod_network_drops_panel())
            )
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a summary tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Kubernetes Workloads")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_k8s_resources_summary_row())
                .row(self.build_k8s_readiness_row())
                .row(self.build_pod_metrics_row())
                .row(self.build_pod_network_row())
            )
        )
