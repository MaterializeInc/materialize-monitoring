"""Compatibility for Grafana Foundation SDK models, between v2 stable and v2beta1."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from grafana_foundation_sdk.models import dashboardv2beta1 as dashboardv2

    HAS_V2_STABLE = False
else:
    try:
        # 0.0.13 (not released as of April 2026)
        from grafana_foundation_sdk.models import (
            dashboardv2,  # pyright: ignore[reportMissingImports]
        )

        HAS_V2_STABLE = True
    except ImportError:
        # 0.0.12
        from grafana_foundation_sdk.models import dashboardv2beta1 as dashboardv2

        HAS_V2_STABLE = False

__all__ = [
    "HAS_V2_STABLE",
    "dashboardv2",
]
