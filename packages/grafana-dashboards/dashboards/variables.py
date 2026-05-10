"""Common variables across dashboards."""

from __future__ import annotations

from typing import Final

from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import METRICS_DATASOURCE_VAR_NAME, promql_query


class VariableNames:
    """Common variable names."""

    METRIC_DS: Final[str] = METRICS_DATASOURCE_VAR_NAME  # metricsDatasource
    ENVIRONMENT_ID: Final[str] = "environmentId"
    ENVIRONMENT_ID_LIST: Final[str] = "environmentIdList"
    NAMESPACE: Final[str] = "namespace"
    NAMESPACE_LIST: Final[str] = "namespaceList"
    MZ_NAMESPACE: Final[str] = "mzNamespace"
    MZ_NAMESPACE_LIST: Final[str] = "mzNamespaceList"
    CONTAINER_FILTER: Final[str] = "containerFilter"


def environment_namespace(*, multi: bool = False) -> dashboardv2_builders.QueryVariable:
    """Get a variable for where materialize environments live."""
    name = VariableNames.MZ_NAMESPACE_LIST if multi else VariableNames.MZ_NAMESPACE
    return (
        dashboardv2_builders.QueryVariable(name)
        .label("Materialize Namespace")
        .description("The current materialize namespace where environments live")
        .allow_custom_value(True)
        .multi(multi)
        .definition("label_values(v2_mz_can_connect, namespace)")
        .query(promql_query("label_values(v2_mz_can_connect, namespace)"))
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
        .definition('query_result(v2_mz_can_connect{namespace="$mzNamespace"})')
        .query(
            promql_query('query_result(v2_mz_can_connect{namespace="$mzNamespace"})')
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
        dashboardv2_builders.ConstantVariable(VariableNames.CONTAINER_FILTER)
        .label("Container Filter")
        .description(
            "A filter to apply to cAdvisor queries to remove irrelevant series"
        )
        .query(",".join(filter_list))
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
