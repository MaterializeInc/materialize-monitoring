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

        Everything else resolves the prefix through this property so a single
        `cloud_hint` / config knob drives the whole board.
        """
        if GLOBAL_DASHBOARD_CONFIG.sql_metric_prefix != "mz_":
            return GLOBAL_DASHBOARD_CONFIG.sql_metric_prefix
        if self.cloud_hint == CloudHint.MZ_K8S:
            return "v2_mz_"
        return "mz_"

    @property
    def has_container_resource_limits(self) -> bool:
        """Whether cAdvisor / kube-state-metrics expose container CPU & memory *limits*.

        GKE's managed cAdvisor + kube-state-metrics ship a reduced metric
        allowlist that omits `kube_pod_container_resource_limits` and
        `container_spec_cpu_quota` / `_period` / `container_spec_memory_limit_bytes`
        (and `container_start_time_seconds`). Without a limit denominator the
        usage-vs-limit percentage panels can't be computed, so they fall back to
        absolute usage (cores / bytes). Self-managed AWS/Azure and Materialize's
        own k8s run full collectors, so they keep the percentages.
        """
        return self.cloud_hint != CloudHint.GCP

    def metric_unavailable_note(self, default: str) -> str:
        """Cloud-hint-aware `no_value` message for a metric a deployment may not expose.

        Panels that depend on a metric absent in some environments pass the
        `default` message (used where the metric is normally present, e.g.
        self-managed with full collectors). On GKE, where the managed
        cAdvisor / kube-state-metrics allowlist omits the container limit/spec
        and start-time series, return a message that names the gap and how to
        close it instead of the generic "cAdvisor required" text (cAdvisor *is*
        running on GKE — it just doesn't export these series).
        """
        if self.cloud_hint == CloudHint.GCP:
            return "No data or potentially unavailable in GKE due to limited cAdvisor metrics."
        return default


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
