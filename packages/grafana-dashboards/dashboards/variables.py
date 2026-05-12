"""Common variables across dashboards."""

from __future__ import annotations

from typing import Final

from grafana_foundation_sdk.cog import builder as cogbuilder

from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import METRICS_DATASOURCE_VAR_NAME, promql_query

_MZ_INFO_METRIC = "mz_compute_commands_total"


class VariableNames:
    """Common variable names."""

    METRIC_DS: Final[str] = METRICS_DATASOURCE_VAR_NAME  # metricsDatasource
    """Metric datasource (prometheus)."""
    METRIC_ADHOC: Final[str] = "metricAdhoc"
    """Adhoc filters for metrics queries."""
    NAMESPACE: Final[str] = "namespace"
    """A standard Kubernetes namespace.

    Prefer MZ_NAMESPACE for environment-specific dashboards.
    """
    NAMESPACE_LIST: Final[str] = "namespaceList"
    """A list of Kubernetes namespaces."""

    MZ_NAMESPACE: Final[str] = "mzNamespace"
    """A Kubernetes namespace where a materialize environment lives."""
    MZ_NAMESPACE_LIST: Final[str] = "mzNamespaceList"
    """A list of materialize namespaces."""
    ENVIRONMENT_ID: Final[str] = "environmentId"
    """A single environment ID to filter to.

    This does not include the `environment-` prefix.
    """
    ENVIRONMENT_ID_LIST: Final[str] = "environmentIdList"
    """A list of environment IDs to filter to."""
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


def environment_namespace(*, multi: bool = False) -> dashboardv2_builders.QueryVariable:
    """Get a variable for where materialize environments live."""
    name = VariableNames.MZ_NAMESPACE_LIST if multi else VariableNames.MZ_NAMESPACE
    return (
        dashboardv2_builders.QueryVariable(name)
        .label("Materialize Namespace")
        .description("The current materialize namespace where environments live")
        .allow_custom_value(True)
        .multi(multi)
        .definition(f"label_values({_MZ_INFO_METRIC}, namespace)")
        .query(promql_query(f"label_values({_MZ_INFO_METRIC}, namespace)"))
    )


def environment_id_variable(
    *, multi: bool = False
) -> dashboardv2_builders.QueryVariable:
    """Get a variable for environment_id.

    FIXME: This does not support augmenting with additional metadata
    (YET).
    FIXME: Use a _info metric once we have it.
    """
    name = VariableNames.ENVIRONMENT_ID_LIST if multi else VariableNames.ENVIRONMENT_ID
    return (
        dashboardv2_builders.QueryVariable(name)
        .label("Environment")
        .description("The current environment to view")
        .allow_custom_value(True)
        .multi(multi)
        .definition(f'query_result({_MZ_INFO_METRIC}{{namespace="$mzNamespace"}})')
        .query(
            promql_query(f'query_result({_MZ_INFO_METRIC}{{namespace="$mzNamespace"}})')
        )
        # NB: already alphabetical
        .regex(
            r".*materialize_cloud_organization_id=\"(?<value>[^\"]+)\",.*materialize_cloud_organization_name=\"(?<text>[^\"]+)\",.*",
        )
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
        .query('materialize_cloud_organization_id="$environmentId"')
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
        # exclusion pattern for instance_id!~"$mzClusterList"
        .enabled_value("")
        .disabled_value("^s.*")
        .current("")
        .hide(dashboardv2.VariableHide.IN_CONTROLS_MENU)
    )


def cluster_list_variable() -> dashboardv2_builders.QueryVariable:
    """A list of clusters we should filter on within an environment."""
    return (
        dashboardv2_builders.QueryVariable(VariableNames.MZ_CLUSTER_LIST)
        .label("Cluster")
        .description("The cluster within the current environment to filter to")
        .allow_custom_value(True)
        .multi(True)
        .include_all(True)
        .definition(
            f'label_values({_MZ_INFO_METRIC}{{namespace="$mzNamespace", instance_id!~"${VariableNames.MZ_INCLUDE_SYSTEM_CLUSTERS}" }}, instance_id)'
        )
        .query(
            promql_query(
                f'label_values({_MZ_INFO_METRIC}{{namespace="$mzNamespace", instance_id!~"${VariableNames.MZ_INCLUDE_SYSTEM_CLUSTERS}" }}, instance_id)'
            )
        )
        .hide(dashboardv2.VariableHide.IN_CONTROLS_MENU)
    )


def replica_list_variable() -> dashboardv2_builders.QueryVariable:
    """A list of replicas we should filter on within an environment."""
    return (
        dashboardv2_builders.QueryVariable(VariableNames.MZ_REPLICA_LIST)
        .label("Replica")
        .description("The replica within the current cluster to filter to")
        .allow_custom_value(True)
        .multi(True)
        .include_all(True)
        .definition(
            f'label_values({_MZ_INFO_METRIC}{{namespace="$mzNamespace", instance_id=~"$mzClusterList"}}, replica_id)'
        )
        .query(
            promql_query(
                f'label_values({_MZ_INFO_METRIC}{{namespace="$mzNamespace", instance_id=~"$mzClusterList"}}, replica_id)'
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
            'materialize_cloud_organization_id="$environmentId", instance_id=~"$mzClusterList"'
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
            'materialize_cloud_organization_id="$environmentId", instance_id=~"$mzClusterList", replica_id=~"$mzReplicaList"'
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
        .operator("=")
        .value("$mzNamespace"),
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
