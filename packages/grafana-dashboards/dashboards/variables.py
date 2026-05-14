"""Common variables across dashboards."""

from __future__ import annotations

from typing import Final

from grafana_foundation_sdk.cog import builder as cogbuilder
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import METRICS_DATASOURCE_VAR_NAME, promql_query

_MZ_INFO_METRIC = "mz_compute_commands_total"
_MZ_CLUSTER_INFO_METRIC = "mz_tokio_worker_park_count"


class VariableNames:
    """Common variable names."""

    METRIC_DS: Final[str] = METRICS_DATASOURCE_VAR_NAME  # metricsDatasource
    """Metric datasource (prometheus)."""
    METRIC_ADHOC: Final[str] = "metricAdhoc"
    """Adhoc filters for metrics queries."""
    NAMESPACE: Final[str] = "namespace"
    """A standard Kubernetes namespace.

    Prefer MZ_NAMESPACE_LIST for environment-specific dashboards.
    """
    NAMESPACE_LIST: Final[str] = "namespaceList"
    """A list of Kubernetes namespaces."""

    ENVIRONMENT_ID_LIST: Final[str] = "environmentIdList"
    """A list of environment IDs to filter to.

    These do not include the `environment-` prefix.
    """
    MZ_NAMESPACE_LIST: Final[str] = "mzNamespaceList"
    """A list of materialize namespaces associated with selected environments."""
    MZ_INCLUDE_SYSTEM_CLUSTERS: Final[str] = "includeSystemClusters"
    """Whether to include system clusters in cluster variables."""
    MZ_CLUSTER_LIST: Final[str] = "mzClusterList"
    """A list of clusters within the current environment."""
    MZ_REPLICA_LIST: Final[str] = "mzReplicaList"
    """A list of replicas within the current materialize cluster."""


class IntermediateNames:
    """Intermediate variable names used for defining other variables."""

    CONTAINER_FILTER: Final[str] = "containerFilter"
    """A filter to apply to cAdvisor queries to remove irrelevant series."""
    ENVIRONMENT_FILTER: Final[str] = "environmentFilter"
    """A filter to apply to materialize queries to filter to the current environment."""
    MZ_CLUSTER_FILTER: Final[str] = "clusterFilter"
    """A filter to apply to materialize queries to filter to the current cluster."""
    MZ_REPLICA_FILTER: Final[str] = "replicaFilter"
    """A filter to apply to materialize queries to filter to the current replica."""


def environment_id_variable(
    *, multi: bool = False
) -> dashboardv2_builders.QueryVariable:
    """Get a variable for environment_id.

    This supports multi-select but only if the UI supports it.
    Queries should expect a List regardless.

    FIXME: This does not support augmenting with additional metadata
    (YET).
    FIXME: Use a _info metric once we have it.
    """
    name = VariableNames.ENVIRONMENT_ID_LIST
    return (
        dashboardv2_builders.QueryVariable(name)
        .label("Environment")
        .description("The current environment to view")
        .allow_custom_value(True)
        .multi(multi)
        .include_all(multi)
        .all_value(".*")
        # don't use natural for envs since they contain hex
        .sort(dashboardv2.VariableSort.ALPHABETICAL_ASC)
        .definition(f"query_result({_MZ_INFO_METRIC})")
        .query(promql_query(f"query_result({_MZ_INFO_METRIC})"))
        # NB: Grafana formats labels alphabetically by default
        .regex(
            r".*materialize_cloud_organization_id=\"(?<value>[^\"]+)\",.*materialize_cloud_organization_name=\"(?<text>[^\"]+)\",.*",
        )
    )


def environment_namespace() -> dashboardv2_builders.QueryVariable:
    """Get a variable for where materialize environments live."""
    name = VariableNames.MZ_NAMESPACE_LIST
    return (
        dashboardv2_builders.QueryVariable(name)
        .label("Materialize Namespace")
        .description("The current materialize namespace where environments live")
        .allow_custom_value(True)
        .multi(True)
        .include_all(True)
        .sort(dashboardv2.VariableSort.ALPHABETICAL_ASC)
        .skip_url_sync(True)
        .definition(
            f'label_values({_MZ_INFO_METRIC}{{ materialize_cloud_organization_id=~"$environmentIdList" }}, namespace)'
        )
        .query(
            promql_query(
                f'label_values({_MZ_INFO_METRIC}{{ materialize_cloud_organization_id=~"$environmentIdList" }}, namespace)'
            )
        )
        .hide(dashboardv2.VariableHide.HIDE_VARIABLE)
    )


def container_filter_variable(*filters: str) -> dashboardv2_builders.ConstantVariable:
    """Create a hidden variable for fixing cadvisor container queries.

    All queries generally should have a `container!="",container!="POD"` filter.
    """
    filter_list = [*filters, 'container!=""', 'container!="POD"']
    return (
        dashboardv2_builders.ConstantVariable(IntermediateNames.CONTAINER_FILTER)
        .label("Container Filter")
        .description(
            "A filter to apply to cAdvisor queries to remove irrelevant series"
        )
        .query(",".join(filter_list))
        .skip_url_sync(True)
        .hide(dashboardv2.VariableHide.HIDE_VARIABLE)
    )


def environment_filter_variable() -> dashboardv2_builders.ConstantVariable:
    """Create a hidden variable for filtering to the current environment."""
    return (
        dashboardv2_builders.ConstantVariable(IntermediateNames.ENVIRONMENT_FILTER)
        .label("Environment Filter")
        .description(
            "A filter to apply to queries to filter to the current environment"
        )
        .query('materialize_cloud_organization_id=~"$environmentIdList"')
        .skip_url_sync(True)
        .hide(dashboardv2.VariableHide.HIDE_VARIABLE)
    )


def include_system_clusters_variable() -> dashboardv2_builders.SwitchVariable:
    """A variable for whether to include `s*` clusters when filtering for clusters."""
    return (
        dashboardv2_builders.SwitchVariable(VariableNames.MZ_INCLUDE_SYSTEM_CLUSTERS)
        .label("Include System Clusters")
        .description(
            "Whether to include materialize system clusters in the cluster list."
        )
        # inclusion pattern for instance_id=~"$includeSystemClusters"
        .enabled_value(".*")
        # NOTE: negative lookahead is not supported
        .disabled_value("^[^s].*")
        .current(".*")
        .skip_url_sync(True)
        .hide(dashboardv2.VariableHide.IN_CONTROLS_MENU)
    )


def cluster_list_variable() -> dashboardv2_builders.QueryVariable:
    """A list of clusters we should filter on within an environment.

    XXX: Do we want to show name or id in this list?
    """
    return (
        dashboardv2_builders.QueryVariable(VariableNames.MZ_CLUSTER_LIST)
        .label("Cluster")
        .description("The cluster within the current environment to filter to")
        .allow_custom_value(True)
        .multi(True)
        .include_all(True)
        # Use natural for cluster names (u2 < u11)
        .sort(dashboardv2.VariableSort.NATURAL_ASC)
        .definition(
            f'query_result({_MZ_CLUSTER_INFO_METRIC}{{materialize_cloud_organization_id=~"$environmentIdList", cluster_environmentd_materialize_cloud_cluster_id=~"${VariableNames.MZ_INCLUDE_SYSTEM_CLUSTERS}" }})'
        )
        .query(
            promql_query(
                f'query_result({_MZ_CLUSTER_INFO_METRIC}{{materialize_cloud_organization_id=~"$environmentIdList", cluster_environmentd_materialize_cloud_cluster_id=~"${VariableNames.MZ_INCLUDE_SYSTEM_CLUSTERS}" }})'
            )
        )
        .hide(dashboardv2.VariableHide.IN_CONTROLS_MENU)
        # it would be nice if we could show both name and id as text
        #   but we don't get format support
        # NB: Grafana sorts labels alphabetically by default (so this regex is stable)
        .regex(
            r".*cluster_environmentd_materialize_cloud_cluster_id=\"(?<value>[^\"]+)\",.*cluster_environmentd_materialize_cloud_cluster_name=\"(?<text>[^\"]+)\",.*",
        )
    )


def replica_list_variable() -> dashboardv2_builders.QueryVariable:
    """A list of replicas we should filter on within an environment.

    Note that replica_name is almost "r1", which is not very unique
    across clusters, so we use replica_id for the text.
    """
    return (
        dashboardv2_builders.QueryVariable(VariableNames.MZ_REPLICA_LIST)
        .label("Replica")
        .description("The replica within the current cluster to filter to")
        .allow_custom_value(True)
        .multi(True)
        .include_all(True)
        .all_value(".*")
        # Use natural for replica ids (mostly numbers)
        .sort(dashboardv2.VariableSort.NATURAL_ASC)
        .definition(
            f'label_values({_MZ_INFO_METRIC}{{materialize_cloud_organization_id=~"$environmentIdList", instance_id=~"$mzClusterList"}}, replica_id)'
        )
        .query(
            promql_query(
                f'label_values({_MZ_INFO_METRIC}{{materialize_cloud_organization_id=~"$environmentIdList", instance_id=~"$mzClusterList"}}, replica_id)'
            )
        )
        .hide(dashboardv2.VariableHide.IN_CONTROLS_MENU)
    )


def cluster_filter_variable() -> dashboardv2_builders.ConstantVariable:
    """A variable for filtering to the current cluster."""
    return (
        dashboardv2_builders.ConstantVariable(IntermediateNames.MZ_CLUSTER_FILTER)
        .label("Cluster Filter")
        .description("A filter to apply to queries to filter to the current cluster")
        .query(
            'materialize_cloud_organization_id=~"$environmentIdList", instance_id=~"$mzClusterList"'
        )
        .skip_url_sync(True)
        .hide(dashboardv2.VariableHide.HIDE_VARIABLE)
    )


def replica_filter_variable() -> dashboardv2_builders.ConstantVariable:
    """A variable for filtering to the current replica."""
    return (
        dashboardv2_builders.ConstantVariable(IntermediateNames.MZ_REPLICA_FILTER)
        .label("Replica Filter")
        .description("A filter to apply to queries to filter to the current replica")
        .query(
            'materialize_cloud_organization_id=~"$environmentIdList", instance_id=~"$mzClusterList", replica_id=~"$mzReplicaList"'
        )
        .skip_url_sync(True)
        .hide(dashboardv2.VariableHide.HIDE_VARIABLE)
    )


def metrics_datasource() -> dashboardv2_builders.DatasourceVariable:
    """Build the prometheus datasource."""
    return (
        dashboardv2_builders.DatasourceVariable(name=VariableNames.METRIC_DS)
        .label("Metrics Datasource")
        .description("Datasource for metrics queries")
        .plugin_id("prometheus")
        .allow_custom_value(False)
        # FIXME: just while developing!!
        .current(dashboardv2.VariableOption(text="cloud-staging us-east-1"))
    )


def metric_adhoc_variable() -> dashboardv2_builders.AdhocVariable:
    """Build an advanced adhoc variable for all metric queries."""
    # pyright can't detect the type here without us fully defining it -___-
    filters: list[cogbuilder.Builder[dashboardv2.AdHocFilterWithLabels]] = [
        dashboardv2_builders.AdHocFilterWithLabels()
        .key("namespace")
        .operator("=~")
        .value("$mzNamespaceList"),
    ]
    return (
        dashboardv2_builders.AdhocVariable(name=VariableNames.METRIC_ADHOC)
        .label("Advanced Metric Filter")
        .description("Adhoc filters to apply to all metrics queries")
        .datasource(
            dashboardv2_builders.Dashboardv2beta1AdhocVariableKindDatasource().name(
                f"${VariableNames.METRIC_DS}"
            )
        )
        # FIXME: basis filter doesn't actually limit how many labels are shown
        .base_filters(filters)
        # this sets the defaults which look ugly
        # .filters(filters)
        .hide(dashboardv2.VariableHide.IN_CONTROLS_MENU)
    )
