"""Compatibility wrapper around Grafana Foundation SDK Dashboard."""

from __future__ import annotations

import abc
import typing
from collections.abc import Sequence

from grafana_foundation_sdk.cog import builder as cogbuilder
from grafana_foundation_sdk.models.dashboard import Dashboard as DashboardV1

from .config import GLOBAL_DASHBOARD_CONFIG
from .models_v2 import dashboardv2

T = typing.TypeVar("T")


def _unique_list(items: Sequence[T]) -> list[T]:
    """Return a list of unique items."""
    return list(set(items))


@typing.runtime_checkable
class _PanelBuilderProtocol(typing.Protocol):
    def build(self) -> dashboardv2.PanelKind: ...


class MzDashboard(dashboardv2.Dashboard, metaclass=abc.ABCMeta):
    """Compatibility wrapper around Grafana Foundation SDK Dashboard in a declarative manner."""

    TITLE: str
    """Title for the dashboard."""
    DESCRIPTION: str = "__doc__"
    """Description for the dashboard. By default, uses the class docstring."""

    TAGS: list[str]
    """Additional tags to apply to the dashboard."""

    UID: str
    """In v2, UIDs exist on the level above dashboards."""

    def __init__(self, **kwargs):
        """Initialize the MzDashboard."""
        if not kwargs.get("title"):
            kwargs["title"] = f"{GLOBAL_DASHBOARD_CONFIG.title_prefix} {self.TITLE}"
        if not kwargs.get("description"):
            kwargs["description"] = (
                self.__doc__ if self.DESCRIPTION == "__doc__" else self.DESCRIPTION
            )
        if not kwargs.get("tags"):
            kwargs["tags"] = _unique_list(
                GLOBAL_DASHBOARD_CONFIG.default_tags + getattr(self, "TAGS", [])
            )
        self.uid: str = f"{GLOBAL_DASHBOARD_CONFIG.uid_prefix}{self.UID}"
        self._panel_id_counter = 1000
        super().__init__(**kwargs)
        self.configure_datasources()
        self.configure_variables()
        self.layout = self.build_layout()

    def to_v1(self) -> DashboardV1:
        """Generate a V1 dashboard from this dashboard."""
        return DashboardV1(
            title=self.title,
            description=self.description,
            tags=self.tags,
            uid=self.uid,
        )

    def add_panel(
        self, name: str, panel: dashboardv2.PanelKind | _PanelBuilderProtocol
    ) -> None:
        """Add a panel to the dashboard."""
        if name in self.elements:
            raise ValueError(f"Panel with name {name} already exists in the dashboard.")
        if isinstance(panel, _PanelBuilderProtocol):
            element = panel.build()
            if not element.spec.id_val:
                element.spec.id_val = self._panel_id_counter
                self._panel_id_counter += 1
            self.elements[name] = element
        else:
            if not panel.spec.id_val:
                panel.spec.id_val = self._panel_id_counter
                self._panel_id_counter += 1
            self.elements[name] = panel

    def add_variable(
        self,
        variable: dashboardv2.VariableKind
        | cogbuilder.Builder[dashboardv2.VariableKind],
    ) -> None:
        """Add a variable to the dashboard."""
        existing_names = {v.spec.name for v in self.variables}
        if isinstance(variable, cogbuilder.Builder):
            to_insert = variable.build()
            # NB: can't check union at runtime
            # assert isinstance(to_insert, dashboardv2.VariableKind)
        else:
            to_insert = variable
        if to_insert.spec.name in existing_names:
            raise ValueError(
                f"Variable with name {to_insert.spec.name} already exists."
            )
        self.variables.append(to_insert)

    def configure_datasources(self):
        """Add datasources to the dashboard."""
        raise NotImplementedError("Must define default datasources")

    def configure_variables(self) -> None:
        """Add variables to the dashboard."""

    @classmethod
    def build(cls, **kwargs) -> dashboardv2.Dashboard:
        """Build the dashboard with the given kwargs.

        This is the main entrypoint for our generator.
        """
        return cls(**kwargs)

    @abc.abstractmethod
    def build_layout(self):
        """Generate an appropriate layout for this dashboard.

        See Also:
            [Tabs builder](https://grafana.github.io/grafana-foundation-sdk/python/Reference/dashboardv2beta1/builder-Tabs/)
            [Tab builder](https://grafana.github.io/grafana-foundation-sdk/python/Reference/dashboardv2beta1/builder-Tab/)
            [Rows builder](https://grafana.github.io/grafana-foundation-sdk/python/Reference/dashboardv2beta1/builder-Rows/)
            [Row builder](https://grafana.github.io/grafana-foundation-sdk/python/Reference/dashboardv2beta1/builder-Row/)
            [AutoGrid builder](https://grafana.github.io/grafana-foundation-sdk/python/Reference/dashboardv2beta1/builder-AutoGrid/)
            [AutoGridItem builder](https://grafana.github.io/grafana-foundation-sdk/python/Reference/dashboardv2beta1/builder-AutoGridItem/)
        """
        raise NotImplementedError
