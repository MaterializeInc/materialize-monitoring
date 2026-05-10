"""Query helpers."""

import typing

from grafana_foundation_sdk.builders import prometheus as prometheus_builder
from grafana_foundation_sdk.cog import builder as cogbuilder

from .builders_v2 import dashboardv2 as dashboardv2_builders
from .models_v2 import dashboardv2
from .query_v2 import CompatPrometheusQueryBuilder as QueryBuilder

METRICS_DATASOURCE_VAR_NAME: typing.Final[str] = "metricsDatasource"


def promql_query(
    expr: str, *, datasource: str = METRICS_DATASOURCE_VAR_NAME
) -> prometheus_builder.Query:
    """Helper to create a prometheus query builder."""
    return (
        QueryBuilder()
        .expr(expr)
        .datasource(
            dashboardv2_builders.Dashboardv2beta1DataQueryKindDatasource().name(
                # e.g., "${metricsDatasource}"
                f"${{{datasource}}}"
            )
        )
    )


def query_group(
    *queries: cogbuilder.Builder[dashboardv2.DataQueryKind],
    promql_expr: typing.LiteralString | list[typing.LiteralString] | None = None,
) -> dashboardv2_builders.QueryGroup:
    """Create a QueryGroup builder with a configured target.

    QueryGroups are the main representation of how data is shown in a panel.
    Having multiple queries in a Query will increase the number of visible series.

    This allows for passing in prebuilt query builders (for max customization)
    and simple string expressions for a configured default datasource.

    WARNING: ref_id is not guaranteed to be stable across implementations
    as new types of queries are added.
    """
    query_list: list[cogbuilder.Builder[dashboardv2.DataQueryKind]] = list(queries)
    if isinstance(promql_expr, str):
        query_list.append(promql_query(promql_expr))
    elif isinstance(promql_expr, list):
        for expr in promql_expr:
            assert isinstance(expr, str)
            query_list.append(promql_query(expr))
    if not query_list:
        raise ValueError("At least one query type must be provided.")
    return dashboardv2_builders.QueryGroup().targets(
        [dashboardv2_builders.Target().query(query) for query in query_list]
    )
