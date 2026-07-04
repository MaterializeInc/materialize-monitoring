"""Compatibility for Grafana Foundation SDK builders, between v2 stable and v2beta1."""

from __future__ import annotations

from grafana_foundation_sdk.builders import (
    dashboardv2,
)

HAS_V2_STABLE = True

__all__ = [
    "HAS_V2_STABLE",
    "dashboardv2",
]
