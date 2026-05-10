"""Environment Overview Dashboard."""

from __future__ import annotations

from grafana_foundation_sdk.builders import dashboardv2beta1 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard

from dashboards import variables

from .summary import OverviewSummary


class EnvironmentOverviewDashboard(MzDashboard):
    """Overview of a Materialize Environment."""

    TITLE = "Environment Overview"

    UID = "env-top"

    def configure_datasources(self):
        """Add datasources to the dashboard."""
        self.add_variable(variables.metrics_datasource())

    def configure_variables(self) -> None:
        """Add variables to the dashboard."""
        self.add_variable(variables.environment_namespace())
        self.add_variable(variables.environment_id_variable())

    def build_summary_tab(self) -> dashboardv2_builders.Tab:
        """Get a summary tab."""
        return OverviewSummary(self).build()

    def build_layout(self):
        """Get the layout for the dashboard."""
        return (dashboardv2_builders.Tabs().tab(self.build_summary_tab())).build()


if __name__ == "__main__":
    from grafana_foundation_sdk.cog.encoder import JSONEncoder

    print(JSONEncoder(indent=2).encode(EnvironmentOverviewDashboard()))
