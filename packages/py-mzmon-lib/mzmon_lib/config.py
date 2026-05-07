"""Dashboard global configuration."""

from __future__ import annotations

from grafana_foundation_sdk.models.dashboard import DashboardDashboardTime
from pydantic import Field
from pydantic_settings import BaseSettings


def _default_tags() -> list[str]:
    """Default tags for dashboards."""
    return ["materialize", "monitoring"]


def _default_time_range() -> DashboardDashboardTime:
    """Default time range for dashboards."""
    return DashboardDashboardTime(
        from_val="now-6h",
        to="now",
    )


class GlobalDashboardConfig(BaseSettings):
    """Global configuration for dashboards."""

    title_prefix: str = Field(
        default="Materialize",
        description="Prefix for all dashboard titles.",
    )

    uid_prefix: str = Field(
        default="mz-mon-",
        description="Prefix for all dashboard UIDs.",
    )

    default_tags: list[str] = Field(
        default_factory=_default_tags,
        description="Default tags to apply to all dashboards.",
    )

    default_refresh: str | None = Field(
        default=None,
        description="Default refresh interval for all dashboards",
    )

    default_time: DashboardDashboardTime | None = Field(
        default_factory=_default_time_range,
        description="Default time range for all dashboards",
    )

    default_timezone: str | None = Field(
        default="browser",
        description="Default timezone for all dashboards",
    )
