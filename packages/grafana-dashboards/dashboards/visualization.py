"""Common visualizations for use in panels."""

from grafana_foundation_sdk.builders import common as common_builder
from grafana_foundation_sdk.builders import piechart as piechart_builder
from grafana_foundation_sdk.builders import stat
from grafana_foundation_sdk.models import common, piechart
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.models_v2 import dashboardv2

NO_FILTER_MATCH = "No matches for the current filters"


PIE_LEGEND_BUILDER = (
    piechart_builder.PieChartLegendOptions()
    .as_table(True)
    .display_mode(common.LegendDisplayMode.TABLE)
    .placement(common.LegendPlacement.RIGHT)
    .is_visible(True)
    .show_legend(True)
    .values([piechart.PieChartLegendValues.VALUE])
)

# Shared legend for timeseries panels in this tab: render as a table beneath
# the chart, with per-series Max / Avg (mean) / Last (lastNotNull) columns.
TS_LEGEND_BUILDER = (
    common_builder.VizLegendOptions()
    .display_mode(common.LegendDisplayMode.TABLE)
    .placement(common.LegendPlacement.BOTTOM)
    .show_legend(True)
    .calcs(["max", "mean", "lastNotNull"])
)


def sparkline_stat(*, shade: str | None = None) -> stat.Visualization:
    """Generate a stat visualization with a sparkline.

    Tabs should pick a theme background color (from e.g., palette.THEME_PALETTE)
    for non-health metrics.

    Further overrides can be applied afterwards.
    """
    viz = (
        stat.Visualization()
        .color_mode(common.BigValueColorMode.NONE)
        .text_mode(common.BigValueTextMode.VALUE)
        .graph_mode(common.BigValueGraphMode.AREA)
        .no_value(NO_FILTER_MATCH)
    )
    if shade:
        viz = viz.color_scheme(
            dashboardv2_builders.FieldColor()
            .mode(dashboardv2.FieldColorModeId.SHADES)
            .fixed_color(shade)
        )
    return viz
