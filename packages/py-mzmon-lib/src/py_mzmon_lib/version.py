"""Grafana Versions."""

from __future__ import annotations

import enum


class DashboardAPI(enum.StrEnum):
    """Grafana dashboard API versions."""

    DASHBOARD_V1 = "dashboard.grafana.app/v1"
    """Used for Grafana 10/11."""
    DASHBOARD_V2BETA1 = "dashboard.grafana.app/v2beta1"
    """Used for Grafana 12."""
    DASHBOARD_V2 = "dashboard.grafana.app/v2"
    """Used for Grafana 13."""

    @classmethod
    def from_version(cls, version: str) -> DashboardAPI:
        """Get the API version corresponding to a given Grafana version."""
        mapping = {
            # don't need earlier than 10
            "10": cls.DASHBOARD_V1,
            "11": cls.DASHBOARD_V1,
            "12": cls.DASHBOARD_V2BETA1,
            "13": cls.DASHBOARD_V2,
            # devel
            "14": cls.DASHBOARD_V2,
        }
        major_version = version.split(".", maxsplit=1)[0]
        if major_version not in mapping:
            raise ValueError(f"Unsupported Grafana version: {version}")
        return mapping[major_version]
