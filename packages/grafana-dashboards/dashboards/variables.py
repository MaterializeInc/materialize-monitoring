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
    NAMESPACE: Final[str] = "namespace"
    MZ_NAMESPACE: Final[str] = "mzNamespace"


def environment_namespace() -> dashboardv2_builders.QueryVariable:
    """Get a variable for where materialize environments live."""
    return (
        dashboardv2_builders.QueryVariable(VariableNames.MZ_NAMESPACE)
        .label("Materialize Namespace")
        .description("The current materialize namespace where environments live")
        .allow_custom_value(True)
        .definition("label_values(v2_mz_can_connect, namespace)")
        .query(promql_query("label_values(v2_mz_can_connect, namespace)"))
    )


def environment_id_variable() -> dashboardv2_builders.QueryVariable:
    """Get a variable for environment_id.

    FIXME: This does not support augmenting with additional metadata
    (YET).
    FIXME: Use a _info metric once we have it.
    """
    return (
        dashboardv2_builders.QueryVariable(VariableNames.ENVIRONMENT_ID)
        .label("Environment")
        .description("The current environment to view")
        .allow_custom_value(True)
        .definition('query_result(v2_mz_can_connect{namespace="$mzNamespace"})')
        .query(
            promql_query('query_result(v2_mz_can_connect{namespace="$mzNamespace"})')
        )
        # NB: already alphabetical
        .regex(
            r".*materialize_cloud_organization_id=\"(?<value>[^\"]+)\",.*materialize_cloud_organization_name=\"(?<text>[^\"]+)\",.*",
        )
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
