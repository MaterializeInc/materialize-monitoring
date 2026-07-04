"""Compatibility for Grafana Foundation SDK models, between v2 stable and v2beta1."""

from __future__ import annotations

from grafana_foundation_sdk.models import (
    dashboardv2,
)

HAS_V2_STABLE = True

__all__ = [
    "HAS_V2_STABLE",
    "dashboardv2",
]
