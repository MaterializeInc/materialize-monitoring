"""Summary tab on Overview Dashboard."""

from __future__ import annotations

import textwrap

from grafana_foundation_sdk.builders import common as common_builder
from grafana_foundation_sdk.builders import gauge, stat
from grafana_foundation_sdk.models import common
from py_mzmon_lib import transform as transform_builders
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import promql_query, query_group

from dashboards import threshold, visualization

from .compute_objects import add_currently_hydrating_panel
from .k8s_resources import CADVISOR_MISSING, KubeResourcesMixin


class OverviewSummary(KubeResourcesMixin):
    """Summary tab on Overview Dashboard."""

    panel_id_prefix = "summary"

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _is_healthy_panel(self):
        """Get a panel showing environment status."""
        panel_id = "is-healthy"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    count(
                        v2_mz_compute_cluster_status{$environmentFilter} == 1
                    ) / count(
                        v2_mz_compute_cluster_status{$environmentFilter}
                    ) * 100
                    """
                ),
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Environment Status")
            .description(
                "**High-level environment health based on the fraction "
                "of clusters reporting healthy.** Aggregates "
                "`v2_mz_compute_cluster_status` across the env; the "
                "result is mapped to text via thresholds: Healthy "
                "(100%), Degraded (80-100%), Unhealthy (<80%). When "
                "this turns Degraded or Unhealthy, check _Kubernetes "
                "Workloads_ for pod restart history and _Cluster "
                "Objects / Replicas_ to see which cluster(s) are "
                "affected."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat()
                .color_mode(common.BigValueColorMode.BACKGROUND)
                # since we want to transform our value, we use a mapping
                .mappings(threshold.health_mapping(min_degraded=80, min_healthy=100))
            ),
        )
        return panel_id

    def _availability_panel(self):
        """Get a panel showing availability over the current time range as a percentage."""
        panel_id = "availability-percent"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    avg by (namespace) (
                        avg_over_time(
                            v2_mz_compute_cluster_status{$environmentFilter}[$__range]
                        ) * 100
                    )
                    """
                ),
            ),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Environment Availability (Select Time Range)")
            .description(
                "**Fraction of time the environment was healthy over "
                "the dashboard's selected time range** — computed from "
                "`v2_mz_compute_cluster_status` averaged over "
                "`$__range`. Effectively an SLO snapshot. Nominal: "
                "100.0000% (note the four decimals — five-nines = "
                "99.999%). Sustained dips correlate with cluster "
                "restarts or outages; widen the time range to find "
                "when they happened, then check _Last Restart Time_ "
                "and _Kubernetes Workloads_ for pod restart context."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat()
                .color_mode(common.BigValueColorMode.BACKGROUND)
                .thresholds(
                    threshold.health_thresholds(
                        min_degraded=95.0,
                        min_healthy=99.0,
                        mode=dashboardv2.ThresholdsMode.PERCENTAGE,
                    )
                )
                .decimals(4)  # 100.0000
                .unit("percent")
            ),
        )
        return panel_id

    def _last_restart_panel(self):
        """Get the last time a container was restarted.

        This requires metrics from node-exporter/cadvisor.
        """
        panel_id = "last-restart"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    time()
                    - topk(1,
                        container_start_time_seconds{$containerFilter, container!="new-promsql-exporter"}
                    )
                    """
                )
            )
            .legend_format("{{pod}}")
            .instant()
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Last Restart Time")
            .description(
                "**Seconds since the most recent container restart in "
                "the environment.** Threshold-colored from red "
                "(seconds ago — likely an active incident) through to "
                "gray (days ago, fine). Nominal: hours-to-days. Recent "
                "restarts (red/orange) are worth correlating with "
                "_Environment Availability_ and the _Kubernetes "
                "Workloads_ tab's pod-health panels."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat()
                .color_mode(common.BigValueColorMode.BACKGROUND)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .unit("s")  # Time / seconds (s)
                .thresholds(threshold.time_stable_thresholds(days=2))
                .text(common_builder.VizTextDisplayOptions().value_size(25))
                # FIXME: only centers value
                .justify_mode(common.BigValueJustifyMode.CENTER)
                .no_value(CADVISOR_MISSING)
            ),
        )
        return panel_id

    def _cpu_usage_panel(self):
        """Get a panel with a gauge showing current CPU usage."""
        panel_id = "cpu-usage-current"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (namespace, container) (
                        rate(
                            container_cpu_usage_seconds_total{$containerFilter}[5m]
                        )
                    ) / sum by (namespace, container) (
                        kube_pod_container_resource_limits{resource="cpu", namespace=~"$mzNamespaceList"}
                    )
                    """
                )
            )
            .legend_format("{{container}}")
            .instant()
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Current CPU Usage (5 min)")
            .description(
                "**Current CPU usage as a fraction of each container's "
                "limit, averaged over the last 5 minutes.** "
                "Per-container gauge — shows the worst-loaded container "
                "types in the env. Nominal: well below 1.0; sustained "
                "near 1.0 means a container type is CPU-bound. For "
                "time-resolved per-pod view see _Kubernetes Workloads "
                "-> Pod CPU Usage_; for the Materialize workload "
                "causing it see _Compute Objects -> Dataflow Elapsed "
                "Rate_."
            )
            .data(query)
            .visualization(
                gauge.Visualization()
                .unit("percentunit")
                .no_value(CADVISOR_MISSING)
                .thresholds(threshold.load_thresholds(max_load=1.0))
                .show_threshold_labels(False)  # HACK: options isn't set otherwise
            ),
        )
        return panel_id

    def _memory_usage_panel(self):
        """Get a panel with a gauge showing current memory usage."""
        panel_id = "memory-usage-current"
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (namespace, container) (
                        avg by (namespace, pod, container) (
                            container_memory_working_set_bytes{$containerFilter, container!="new-promsql-exporter"}
                        )
                    ) / sum by (namespace, container) (
                        avg by (namespace, pod, container) (
                            container_spec_memory_limit_bytes{$containerFilter, container!="new-promsql-exporter"}
                        )
                    )
                    """
                ),
            )
            .legend_format("{{container}}")
            .instant()
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Current Memory Usage")
            .description(
                "**Current memory usage as a fraction of each "
                "container's limit.** Per-container gauge — shows the "
                "worst-loaded container types. **Sustained near 1.0 "
                "is dangerous** — OOM-kill triggers a hydration cycle "
                "(in-memory state has to be rebuilt from persisted "
                "storage, taking minutes-to-hours depending on data "
                "size). For time-resolved view see _Kubernetes "
                "Workloads -> Pod Memory Usage_; the offending "
                "workload usually shows in _Compute Objects -> "
                "Arrangements_."
            )
            .data(query)
            .visualization(
                gauge.Visualization()
                .unit("percentunit")
                .no_value(CADVISOR_MISSING)
                .thresholds(threshold.load_thresholds(max_load=1.0))
                .show_threshold_labels(False)  # HACK: options isn't set otherwise
            ),
        )
        return panel_id

    def _currently_hydrating_panel(self):
        """Re-use the Compute Objects "Currently Hydrating" panel.

        Uses a summary-scoped panel_id so the two registrations don't collide.
        """
        return add_currently_hydrating_panel(
            self.dashboard, panel_id="summary-currently-hydrating"
        )

    def build_healthy_row(self) -> dashboardv2_builders.Row:
        """Get a row showing health."""
        return (
            dashboardv2_builders.Row()
            .title("Environment Health")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .with_item(self._is_healthy_panel())
                .with_item(self._availability_panel())
                .with_item(self._last_restart_panel())
                .with_item(self._currently_hydrating_panel())
                .with_item(self._cpu_usage_panel())
                .with_item(self._memory_usage_panel())
            )
        )

    def _materialize_version_panel(self):
        """Get a panel showing the materialize version."""
        panel_id = "materialize-version"
        query = (
            query_group(
                promql_query(
                    textwrap.dedent(
                        """
                        group by (mz_version) (
                            v2_mz_compute_cluster_status{$environmentFilter}
                        )
                        """
                    ),
                )
                .legend_format("{{mz_version}}")
                .instant()
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("labelsToFields")
                .id("labelsToFields")
                .options({})
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("extractFields")
                .id("extractFields")
                .options(
                    {
                        "format": "regexp",
                        "regExp": r"/.+\((?<commit>[a-fA-F0-9]+)\)/",
                        "source": "mz_version",
                        "replace": False,
                    }
                )
            )
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Materialize Version")
            .description(
                "**The version of Materialize currently running in "
                "the environment.** A single version is the steady "
                "state; multiple distinct values typically appear "
                "briefly during a rolling upgrade. Click the value to "
                "open the corresponding commit on GitHub. If the "
                "version doesn't match what you expect, check the "
                "env's manifest in the deployment pipeline."
            )
            .data(query)
            .visualization(
                stat.Visualization()
                .color_mode(common.BigValueColorMode.NONE)
                .text_mode(common.BigValueTextMode.VALUE)
                .graph_mode(common.BigValueGraphMode.NONE)
                # yeah, apparently this new interface is supposed to be more intuitive -___-
                .text(common_builder.VizTextDisplayOptions().value_size(20))
                .data_links(
                    [
                        dashboardv2_builders.DataLink()
                        .title("View Materialize at Commit")
                        .url(
                            "https://github.com/MaterializeInc/materialize/commit/${__data.fields.commit}"
                        )
                        .target_blank(True)
                        .build()
                    ]
                )
                .reduce_options(
                    common_builder.ReduceDataOptions().fields(r"/^mz_version$/")
                )
            ),
        )
        return panel_id

    def build_info_row(self) -> dashboardv2_builders.Row:
        """Get a row showing environment info."""
        return (
            dashboardv2_builders.Row()
            .title("Environment Info")
            .hide_header(False)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .with_item(self._materialize_version_panel())
                .with_item(self.cpu_total_panel())
                .with_item(self.memory_totals_panel())
            )
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a summary tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Summary")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_healthy_row())
                .row(self.build_info_row())
            )
        )
