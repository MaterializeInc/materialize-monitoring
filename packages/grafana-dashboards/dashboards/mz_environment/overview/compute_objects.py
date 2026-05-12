"""Compute Objects tab on Overview Dashboard.

Compute objects include Indexes, Materialized Views, Subscriptions.
"""

from __future__ import annotations

from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard


class ComputeObjectsTab:
    """Compute Objects tab on Overview Dashboard."""

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a source/compute objects tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Compute Objects")
            .layout(
                dashboardv2_builders.Rows()
                # .row(self.build_replication_summary_row())
                # .row(self.build_availability_summary_row())
                # .row(self.build_replication_details_row())
            )
        )
