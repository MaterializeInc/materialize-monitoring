"""Additional context for building dashboards."""

from __future__ import annotations

import dataclasses

from py_mzmon_lib.config import GLOBAL_DASHBOARD_CONFIG
from py_mzmon_lib.context import BuildContext, CloudHint
from py_mzmon_lib.dashboard import MzDashboard


@dataclasses.dataclass(frozen=True)
class MzBuildContext(BuildContext):
    """Context for building Materialize dashboards.

    This can be used to pass state to dashboards that control how things
    are rendered. This is an extension of the more general BuildContext.
    """

    @property
    def sql_metric_prefix(self) -> str:
        """Get the SQL metric prefix from the context.

        This is `mz_` in almost every case, except `new-promsql-exporter`
        (which may go away in the future).
        """
        if GLOBAL_DASHBOARD_CONFIG.sql_metric_prefix != "mz_":
            return GLOBAL_DASHBOARD_CONFIG.sql_metric_prefix
        if self.cloud_hint == CloudHint.MZ_K8S:
            return "v2_mz_"
        return "mz_"


class BaseMzContextTab:
    """An abstract tab definition common for Materialize dashboards."""

    dashboard: MzDashboard

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    @property
    def context(self) -> MzBuildContext:
        """Get the build context for the tab."""
        # Ensure we are narrowed
        assert isinstance(self.dashboard.context, MzBuildContext)
        return self.dashboard.context
