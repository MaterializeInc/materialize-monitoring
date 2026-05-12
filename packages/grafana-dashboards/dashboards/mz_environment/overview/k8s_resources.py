"""Kubernetes Resources Tab for the Overview Dashboard."""

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

from dashboards import threshold

CADVISOR_MISSING = "No metrics: cadvisor/node-exporter is required"


class KubeResourcesMixin:
    """Shared panels between KubeResources tab and Overview Summary tab."""

    dashboard: MzDashboard
    panel_id_prefix: str

    def cpu_total_panel(self, *, include_monitoring: bool = False):
        """Get a panel showing a holistic summary of compute (CPUs).

        We show a stat for total cores available.
        """
        panel_id = f"{self.panel_id_prefix}-cpu-total"
        filter = "$containerFilter"
        if not include_monitoring:
            filter += ', container!="new-promsql-exporter"'
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (container) (
                        container_spec_cpu_quota{{ {filter} }}
                        / container_spec_cpu_period{{ {filter} }}
                    )
                    """
                ),
            )
            .legend_format("CPUs ({{container}})")
            .instant()
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Total CPU Capacity")
            .description("Total CPU cores available.")
            .data(query)
            .visualization(
                stat.Visualization()
                .color_mode(common.BigValueColorMode.NONE)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .graph_mode(common.BigValueGraphMode.NONE)
                .unit("cores")
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
        filter = "$containerFilter"
        if not include_monitoring:
            filter += ', container!="new-promsql-exporter"'
        query = query_group(
            promql_query(
                # 'sum by (container) (mz_memory_limiter_memory_limit_bytes{materialize_cloud_organization_id="$environmentId"})'
                textwrap.dedent(
                    f"""
                    sum by (container) (
                        container_spec_memory_limit_bytes{{ {filter} }}
                    )
                    """
                ),
            )
            .legend_format("Memory ({{container}})")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Total Memory")
            .description(
                "Total memory available in the environment (excluding monitoring)."
            )
            .data(query)
            .visualization(
                stat.Visualization()
                .color_mode(common.BigValueColorMode.NONE)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .graph_mode(common.BigValueGraphMode.NONE)
                .unit("bytes")
                .no_value(CADVISOR_MISSING)
            ),
        )
        return panel_id


class KubeResourcesTab(KubeResourcesMixin):
    """Kubernetes resources tab on Overview Dashboard."""

    panel_id_prefix = "k8s-res"

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

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

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a summary tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Kubernetes Workloads")
            .layout(
                dashboardv2_builders.Rows().row(self.build_k8s_resources_summary_row())
                # .row(self.build_info_row())
            )
        )
