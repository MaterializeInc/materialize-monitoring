"""Fixes for Transformation builders."""

import typing
import warnings

from .builders_v2 import dashboardv2 as dashboardv2_builders
from .models_v2 import dashboardv2


class CompatTransformationKind(dashboardv2.TransformationKind):
    """Compatibility wrapper around TransformationKind."""

    # v2beta1 api allows setting kind (but it's only "Transformation", it has group's documentation)
    # default is fixed in v2 stable
    kind: typing.Literal["Transformation"]
    # v2beta1 api does not support setting group (but it's required)
    # fixed in v2 stable
    group: str

    def __init__(
        self,
        kind: typing.Literal["Transformation"] = "Transformation",
        group: str = "",
        spec: dashboardv2.TransformationSpec | None = None,
    ):
        self.kind = kind
        self.group = group
        self.spec = spec if spec is not None else dashboardv2.TransformationSpec()

    @classmethod
    def from_json(cls, data: dict[str, typing.Any]) -> typing.Self:
        """Load from a dict."""
        args: dict[str, typing.Any] = {}

        if "kind" in data:
            args["kind"] = data["kind"]
        if "group" in data:
            args["group"] = data["group"]
        if "spec" in data:
            args["spec"] = dashboardv2.TransformationSpec.from_json(data["spec"])

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

    def id(self, id_val: str) -> typing.Self:
        """Provide deprecated compat method."""
        _ = id_val
        warnings.warn(
            "CompatTransformationBuilder.id() is deprecated and does nothing.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self
