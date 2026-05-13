"""Environment Overview Dashboard."""

from __future__ import annotations

from grafana_foundation_sdk.builders import dashboardv2beta1 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard

from dashboards import variables

from .cluster_objects import ClusterObjectsTab
from .compute_objects import ComputeObjectsTab
from .connections_activity import ConnectionsActivityTab
from .k8s_resources import KubeResourcesTab
from .storage_objects import StorageObjectsTab
from .summary import OverviewSummary


class EnvironmentOverviewDashboard(MzDashboard):
    """Overview of a Materialize Environment."""

    TITLE = "Environment Overview"

    UID = "env-top"

    def configure_datasources(self):
        """Add datasources to the dashboard."""
        self.add_variable(variables.metrics_datasource())
        self.add_variable(variables.metric_adhoc_variable())

    def configure_variables(self) -> None:
        """Add variables to the dashboard."""
        self.add_variable(variables.environment_namespace())
        self.add_variable(variables.environment_id_variable())
        self.add_variable(
            variables.container_filter_variable(
                'namespace=~"$mzNamespaceList"',
            )
        )
        self.add_variable(variables.include_system_clusters_variable())
        self.add_variable(variables.cluster_list_variable())
        self.add_variable(variables.replica_list_variable())

        self.add_variable(variables.environment_filter_variable())
        self.add_variable(variables.cluster_filter_variable())
        self.add_variable(variables.replica_filter_variable())

    def build_summary_tab(self) -> dashboardv2_builders.Tab:
        """Get a summary tab."""
        return OverviewSummary(self).build()

    def build_k8s_resources_tab(self) -> dashboardv2_builders.Tab:
        """Get a Kubernetes resources tab."""
        return KubeResourcesTab(self).build()

    def build_cluster_objects_tab(self) -> dashboardv2_builders.Tab:
        """Get a clusters/replicas/availability tab."""
        return ClusterObjectsTab(self).build()

    def build_connections_activity_tab(self) -> dashboardv2_builders.Tab:
        """Get a connections / activity tab."""
        return ConnectionsActivityTab(self).build()

    def build_compute_objects_tab(self) -> dashboardv2_builders.Tab:
        """Get a compute objects tab."""
        return ComputeObjectsTab(self).build()

    def build_storage_objects_tab(self) -> dashboardv2_builders.Tab:
        """Get a storage objects tab."""
        return StorageObjectsTab(self).build()

    def build_layout(self):
        """Get the layout for the dashboard."""
        return (
            dashboardv2_builders.Tabs()
            .tab(self.build_summary_tab())
            .tab(self.build_k8s_resources_tab())
            .tab(self.build_connections_activity_tab())
            .tab(self.build_cluster_objects_tab())
            .tab(self.build_compute_objects_tab())
            .tab(self.build_storage_objects_tab())
        ).build()


if __name__ == "__main__":
    from grafana_foundation_sdk.cog.encoder import JSONEncoder

    print(JSONEncoder(indent=2).encode(EnvironmentOverviewDashboard()))  # noqa: T201
