"""Render Grafana dashboards from these sources."""

from __future__ import annotations

import argparse
import json
import logging
import pathlib
import sys

import yaml
from py_mzmon_lib.dashboard import MzDashboard

from dashboards.mz_environment.overview.overview_dashboard import (
    EnvironmentOverviewDashboard,
)

AVAIL_DASHBOARDS: dict[str, type[MzDashboard]] = {
    EnvironmentOverviewDashboard.UID: EnvironmentOverviewDashboard,
}

LOGGER = logging.getLogger("dashboards.render")  # sometimes __main__


class RenderArgs(argparse.Namespace):
    """Arguments for rendering dashboards."""

    output: str
    format: str
    dashboards: list[str]


def get_parser() -> argparse.ArgumentParser:
    """Get the argument parser."""
    parser = argparse.ArgumentParser(description=main.__doc__)
    parser.add_argument(
        "--output",
        "-o",
        type=str,
        default=".",
        help="Output directory for generated dashboards.",
    )
    parser.add_argument(
        "--format",
        choices=["json", "yaml"],
        default="json",
        help="Output format for generated dashboards.",
    )
    parser.add_argument(
        "dashboards",
        nargs="*",
        choices=AVAIL_DASHBOARDS.keys(),
        help="Specific dashboards to generate. If not provided, all dashboards will be generated.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    """Render Grafana dashboards."""
    logging.basicConfig(level=logging.INFO)
    parser = get_parser()
    args: RenderArgs = parser.parse_args(argv, namespace=RenderArgs())
    output_dir = pathlib.Path(args.output)
    selected_dashboards = args.dashboards or AVAIL_DASHBOARDS.keys()
    ext = args.format
    LOGGER.info("Output directory: %s", output_dir)
    LOGGER.debug("Selected dashboards: %s", ", ".join(selected_dashboards))
    for dashboard_name in selected_dashboards:
        dashboard_cls = AVAIL_DASHBOARDS[dashboard_name]
        LOGGER.info("Rendering dashboard: %s", dashboard_name)
        rendered_dashboard = dashboard_cls.render()
        output_path = output_dir / f"{dashboard_name}.{ext}"
        with open(output_path, "w") as handle:
            if args.format == "json":
                handle.write(rendered_dashboard)
                handle.write("\n")
            elif args.format == "yaml":
                # HACK: we need the json encoder to handle cog internals
                # but to convert, we decode and re-encode as yaml
                yaml.dump(json.loads(rendered_dashboard), handle)
            else:
                raise ValueError(f"Unsupported format: {args.format}")
        LOGGER.info("Dashboard %s written to %s", dashboard_name, output_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
