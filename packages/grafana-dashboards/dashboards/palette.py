"""Safe palette of colors for Grafana dashboards.

This is meant to prefer color-blind-friendly and print-friendly
(black/white) as much as possible, while not bringing too much toil.

These palettes generally come from the works of Paul Tol.

See Also:
    <https://sronpersonalpages.nl/~pault/>
"""

from __future__ import annotations

import enum

# 12-step (11+1) sequence from "good" data to "bad"
# good for color-blind, but slightly less great for black and white printing
# fairly intuitive for health meters and thresholds
# safe to interpolate
# https://sronpersonalpages.nl/~pault/#fig:scheme_incandescent
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


SUNSET_DIVERGING_SEQ = [
    "#364B9A",  # dark blue
    "#4A7BB7",
    "#6EA6CD",
    "#98CAE1",
    "#C2E4EF",
    "#EAECCC",  # light yellow
    "#FEDA8B",  # light orange
    "#FDB366",  # orange
    "#F67E4B",  # red-orange
    "#DD3D2D",  # red
    "#A50026",  # dark red
]

# white for invalid data
SUNSET_INVALID = "#FFFFFF"
# not an error
SUNSET_NOMINAL = SUNSET_DIVERGING_SEQ[5]
# increasing errors
SUNSET_ERROR_SEQ = SUNSET_DIVERGING_SEQ[6:]


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


# Theme colors
# https://sronpersonalpages.nl/~pault/#fig:scheme_light
# These can be used for cases where health isn't important
LIGHT_QUALITATIVE_NONSEQ = [
    "#77AADD",  # light blue
    # "#99DDFF",  # light cyan  (too close to light blue)
    # "#44BB99",  # mint  (staying away from greens)
    "#BBCC33",  # pear
    "#AAAA00",  # olive
    "#EEDD88",  # light yellow
    "#EE8866",  # orange
    "#FFAABB",  # pink
    "#DDDDDD",  # light gray
]
BRIGHT_QUALITATIVE_NONSEQ = [
    "#0077BB",  # blue
    "#33BBEE",  # cyan
    "#009988",  # teal
    "#EE7733",  # orange
    "#CCBB44",  # yellow
    # "#CC3311",  # red  (health color)
    "#EE3377",  # magenta
    "#BBBBBB",  # gray
]
THEME_PALETTE = BRIGHT_QUALITATIVE_NONSEQ
