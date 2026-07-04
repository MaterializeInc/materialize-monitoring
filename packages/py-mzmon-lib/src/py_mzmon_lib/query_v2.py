"""Compatibility around Prometheus queries.

For some reason, grafana-foundation-sdk uses a different schema
for prometheus Dataquery's which causes some data to not be pushed.
"""

from __future__ import annotations

import typing

from grafana_foundation_sdk.builders import prometheus as prometheus_builder
from grafana_foundation_sdk.models import prometheus

from .models_v2 import dashboardv2

QRY_TYPE_TO_NAME: dict[int, str] = {
    0: prometheus.PromQueryFormat.TIME_SERIES,  # 0 isn't really valid
    1: prometheus.PromQueryFormat.TIME_SERIES,
    2: prometheus.PromQueryFormat.TABLE,
    3: prometheus.PromQueryFormat.HEATMAP,
}
NAME_TO_QRY_TYPE: dict[str | None, int] = {
    None: 1,
    "": 1,
    prometheus.PromQueryFormat.TIME_SERIES: 1,
    prometheus.PromQueryFormat.TABLE: 2,
    prometheus.PromQueryFormat.HEATMAP: 3,
}

# fail hard if the schema changes under us
assert not hasattr(prometheus_builder.Query, "query")
assert hasattr(prometheus_builder.Query, "expr")


class CompatPrometheusDataQuery(prometheus.Dataquery):
    """Fixed compatibility wrapper around prometheus dataqueries."""

    def to_json(self) -> dict[str, object]:
        """Ensure missing properties are present.

        We do keep the alternative properties for compatibility.
        """
        data = super().to_json()
        data["query"] = data["expr"]
        data["qryType"] = NAME_TO_QRY_TYPE[self.query_type]
        return data

    @classmethod
    def from_json(cls, data: dict[str, typing.Any]) -> typing.Self:
        """Ensure alternative properties are accepted.

        Unknown fields are silently discarded.
        """
        if data.get("query") and not data.get("expr"):
            data["expr"] = data.pop("query")
        if data.get("qryType") and not data.get("queryType"):
            data["queryType"] = QRY_TYPE_TO_NAME[data.pop("qryType")]
        return super().from_json(data)


class CompatPrometheusQueryBuilder(prometheus_builder.QueryV2):
    """Fixed compatibility wrapper around prometheus dataqueries."""

    def __init__(self) -> None:
        self._internal = dashboardv2.DataQueryKind(spec=CompatPrometheusDataQuery())
        self._internal.kind = "DataQuery"
        self._internal.group = "prometheus"

    def query(self, expr: str) -> typing.Self:
        """Support setting expr by alternative method name."""
        return self.expr(expr)
