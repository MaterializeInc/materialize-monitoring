"""Common thresholds.

It'd be nice if we could cache these, but the builders they return
are mutable.
"""

from __future__ import annotations

from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.models_v2 import dashboardv2

from . import palette

THRESHOLD_PALETTE = palette.INCANDESC_SEQUENTIAL
ERROR_PALETTE = palette.SUNSET_ERROR_SEQ

# HACK: null in value mappings is supposed to be infinity,
# .... but it doesn't work
# and we definitely can't use json infinity
_ALMOST_INFINITY = float(0x7FFFFFFF)


def get_high_threshold(
    min_value: float = 80, max_value: float = 100, step: float = 10
) -> dashboardv2_builders.ThresholdsConfig:
    """Get a threshold configuration.

    We map our incandescant palette to the full max range,
    then we select values from where the even step would be.
    """
    steps = []
    value = min_value
    while value < max_value:
        color_idx = int(len(THRESHOLD_PALETTE) * value / max_value)
        steps.append(
            dashboardv2.Threshold(
                value=value,
                color=THRESHOLD_PALETTE[color_idx],
            )
        )
        value += step
    steps.append(
        dashboardv2.Threshold(
            value=max_value,
            color=THRESHOLD_PALETTE[-1],
        )
    )
    return dashboardv2_builders.ThresholdsConfig().steps(steps)


THRESHOLD_80_10 = get_high_threshold(min_value=80, step=10).mode(
    dashboardv2.ThresholdsMode.PERCENTAGE
)


def health_mapping(
    *, min_degraded: float = 80, min_healthy: float = 100
) -> list[dashboardv2.ValueMapping]:
    """Generate a tri-health value at the given values.

    It is preferable to use this in a visualization's `.mapping()`
    as it will have our best practices applied.

    If you set min_degraded==min_healthy, you will only get Healthy and Unhealthy states.

    This uses a colorblind-friendly stoplight palette
    with Healthy/Degraded/Unhealthy/Invalid states.
    """
    value_mappings = [
        dashboardv2.RangeMap(
            dashboardv2.Dashboardv2RangeMapOptions(
                from_val=min_healthy,
                to=_ALMOST_INFINITY,
                result=dashboardv2.ValueMappingResult(
                    text="Healthy",
                    color=palette.TriHealth.HEALTHY,
                    index=1,
                ),
            ),
        ),
        dashboardv2.RangeMap(
            dashboardv2.Dashboardv2RangeMapOptions(
                from_val=min_degraded,
                to=min_healthy,
                result=dashboardv2.ValueMappingResult(
                    text="Degraded",
                    color=palette.TriHealth.DEGRADED,
                    index=2,
                ),
            ),
        ),
        dashboardv2.RangeMap(
            dashboardv2.Dashboardv2RangeMapOptions(
                from_val=-_ALMOST_INFINITY,
                to=min_degraded,
                result=dashboardv2.ValueMappingResult(
                    text="Unhealthy",
                    color=palette.TriHealth.UNHEALTHY,
                    index=3,
                ),
            ),
        ),
        dashboardv2.SpecialValueMap(
            dashboardv2.Dashboardv2SpecialValueMapOptions(
                match=dashboardv2.SpecialValueMatch.NULL_AND_NA_N,
                result=dashboardv2.ValueMappingResult(
                    text="Missing Data",
                    color=palette.TriHealth.INVALID,
                    index=4,
                ),
            ),
        ),
    ]
    return value_mappings


def health_thresholds(
    *,
    min_degraded: float = 80,
    min_healthy: float = 100,
    mode: dashboardv2.ThresholdsMode = dashboardv2.ThresholdsMode.ABSOLUTE,
) -> dashboardv2_builders.ThresholdsConfig:
    """Generate a tri-health threshold at the given values.

    It is preferable to use this in a visualization's `.thresholds()`
    as it will have our best practices applied.

    If you set min_degraded==min_healthy, you will only get Healthy and Unhealthy states.

    This uses a colorblind-friendly stoplight palette
    with Healthy/Degraded/Unhealthy states.
    """
    thresholds = [
        dashboardv2.Threshold(
            value=-_ALMOST_INFINITY,
            color=palette.TriHealth.UNHEALTHY,
        ),
        dashboardv2.Threshold(
            value=min_degraded,
            color=palette.TriHealth.DEGRADED,
        ),
        dashboardv2.Threshold(
            value=min_healthy,
            color=palette.TriHealth.HEALTHY,
        ),
    ]
    return dashboardv2_builders.ThresholdsConfig().mode(mode).steps(thresholds)


def time_stable_thresholds(
    *,
    seconds: float | None = None,
    days: float | None = None,
    divisor: float = 1.0,  # seconds
    high_bad: bool = False,
) -> dashboardv2_builders.ThresholdsConfig:
    """Get thresholds for a visualization with increasing stable time.

    The provided time is how long something needs to be stable for.

    The values in this threshold are subject to change based on implementation.
    We use a log to handle exponential stability (scary math ahead).
    """
    thresholds: list[dashboardv2.Threshold] = []
    stable = 0
    if seconds is not None:
        stable += seconds
    if days is not None:
        stable += days * 24 * 3600
    if not stable:
        raise ValueError("You must provide a time duration")
    stable /= divisor
    total_steps = len(THRESHOLD_PALETTE)
    # We want to pick a factor such that factor**12 ~= stable
    # (12th root is easier to rationalize than 2**(log2(stable)/12))
    factor = stable ** (1 / total_steps)
    value = factor
    colors = THRESHOLD_PALETTE if high_bad else THRESHOLD_PALETTE[::-1]
    for color in colors:
        thresholds.append(
            dashboardv2.Threshold(
                value=int(value),
                color=color,
            )
        )
        value *= factor
    return (
        dashboardv2_builders.ThresholdsConfig()
        .mode(dashboardv2.ThresholdsMode.ABSOLUTE)
        .steps(thresholds)
    )


def error_thresholds(
    *, min_errors: float = 1, max_errors: float = 100
) -> dashboardv2_builders.ThresholdsConfig:
    """Get thresholds for a visualization with increasing errors.

    The provided value is how many errors for the highest color.
    """
    thresholds = []
    step = (max_errors - min_errors) / len(ERROR_PALETTE)
    for step_idx, color in enumerate(ERROR_PALETTE):
        value = min_errors + step * step_idx
        thresholds.append(
            dashboardv2.Threshold(
                value=value,
                color=color,
            )
        )
    return (
        dashboardv2_builders.ThresholdsConfig()
        .mode(dashboardv2.ThresholdsMode.ABSOLUTE)
        .steps(thresholds)
    )


def load_thresholds(
    *, min_load: float = 0.0, max_load: float = 1.0
) -> dashboardv2_builders.ThresholdsConfig:
    """Get thresholds for load average.

    This uses a colorblind-friendly sequential palette
    with low->high load states.
    """
    thresholds = []
    step = (max_load - min_load) / len(THRESHOLD_PALETTE)
    for step_idx, color in enumerate(THRESHOLD_PALETTE):
        value = min_load + step * step_idx
        thresholds.append(
            dashboardv2.Threshold(
                value=value,
                color=color,
            )
        )
    return (
        dashboardv2_builders.ThresholdsConfig()
        .mode(dashboardv2.ThresholdsMode.ABSOLUTE)
        .steps(thresholds)
    )
