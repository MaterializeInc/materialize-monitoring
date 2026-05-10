"""Safe palette of colors for Grafana dashboards.

This is meant to prefer color-blind-friendly and print-friendly
(black/white) as much as possible, while not bringing too much toil.
"""

from __future__ import annotations

import enum

# 12-step (11+1) sequence from "good" data to "bad"
# good for color-blind, but not great for print
# fairly intuitive for health meters and thresholds
# safe to interpolate
INCANDESC_SEQUENTIAL = [
    # pale cyan
    "#CEFFFF",
    # light grayish cyan - lime
    "#C6F7D6",
    # soft lime
    "#A2F49B",
    # soft green
    "#BBE453",
    # strong yellow
    "#D5CE04",
    # golden yellow
    "#E7B503",
    # deep warm orange
    "#F19903",
    # vivid orange
    "#F6790B",
    # blood orange
    "#F94902",
    # vivid red
    "#E40515",
    # dark red
    "#AB0003",
]
# gray for invalid data
INCANDESC_INVALID = "#888888"

INCANDESC_SEQUENTIAL_3 = INCANDESC_SEQUENTIAL[2:12:4]  # 3rd, 7th, last
INCANDESC_SEQUENTIAL_4 = INCANDESC_SEQUENTIAL[2:12:3]
INCANDESC_SEQUENTIAL_6 = INCANDESC_SEQUENTIAL[1:12:2]  # evens (or odds, 0 indexed)


class Binary(enum.StrEnum):
    """Two-step palette (single LOW/HIGH)."""

    # cool teal
    LOW = "#009E73"
    GOOD = LOW
    # warm orange
    HIGH = "#D55E00"
    BAD = HIGH


class TriHealth(enum.StrEnum):
    """Three-step palette."""

    HEALTHY = INCANDESC_SEQUENTIAL_3[0]
    DEGRADED = INCANDESC_SEQUENTIAL_3[1]
    UNHEALTHY = INCANDESC_SEQUENTIAL_3[2]

    INVALID = INCANDESC_INVALID
