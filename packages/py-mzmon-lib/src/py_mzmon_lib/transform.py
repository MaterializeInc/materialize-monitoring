"""Fixes for Transformation builders."""

import typing

from .builders_v2 import dashboardv2 as dashboardv2_builders
from .models_v2 import dashboardv2


class CompatTransformationKind(dashboardv2.TransformationKind):
    """Compatibility wrapper around TransformationKind."""

    # v2beta1 api allows setting kind (but it's only "Transformation", it has group's documentation)
    # default is fixed in v2 stable
    kind: str
    # v2beta1 api does not support setting group (but it's required)
    # fixed in v2 stable
    group: str

    def __init__(
        self,
        kind: str = "Transformation",
        group: str = "",
        spec: dashboardv2.DataTransformerConfig | None = None,
    ):
        self.kind = kind
        self.group = group
        self.spec = spec if spec is not None else dashboardv2.DataTransformerConfig()

    def to_json(self) -> dict[str, object]:
        """Dump to a json-serializable dict.

        This adds a missing property.
        """
        payload: dict[str, object] = super().to_json()
        assert "group" not in payload, "fixer not needed"
        payload["group"] = self.group
        return payload

    @classmethod
    def from_json(cls, data: dict[str, typing.Any]) -> typing.Self:
        """Load from a dict."""
        args: dict[str, typing.Any] = {}

        if "kind" in data:
            args["kind"] = data["kind"]
        if "group" in data:
            args["group"] = data["group"]
        if "spec" in data:
            args["spec"] = dashboardv2.DataTransformerConfig.from_json(data["spec"])

        return cls(**args)


class CompatTransformationBuilder(dashboardv2_builders.Transformation):
    """Compatibility wrapper around Transformation builders."""

    def __init__(self) -> None:
        self._internal = CompatTransformationKind()

    def group(self, group: str) -> typing.Self:
        """Ensure group is always set."""
        assert isinstance(self._internal, CompatTransformationKind)
        self._internal.group = group
        return self
