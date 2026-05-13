"""Clusters/Replicas/Availability tab on Overview Dashboard."""

from __future__ import annotations

import textwrap

from grafana_foundation_sdk.builders import (
    piechart as piechart_builder,
)
from grafana_foundation_sdk.builders import (
    table,
)
from grafana_foundation_sdk.models import common, piechart
from py_mzmon_lib import transform as transform_builders
from py_mzmon_lib.builders_v2 import dashboardv2 as dashboardv2_builders
from py_mzmon_lib.dashboard import MzDashboard
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import promql_query, query_group

from dashboards import palette, visualization

CLUSTERS_THEME = palette.THEME_PALETTE[2]


class ClusterObjectsTab:
    """Clusters/Replicas/Availability tab on Overview Dashboard."""

    def __init__(self, dashboard: MzDashboard) -> None:
        self.dashboard = dashboard

    def _cluster_count_panel(self):
        """Show the number of clusters.

        NOTE: cluster/replica filters are not considered
        """
        panel_id = "cluster-count"
        # v2_mz_clusters_count and v2_mz_cluster_reps_count are weirdly environment scoped
        # so we recalculate them
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    count(
                        group by (compute_cluster_id) (
                            v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList"}
                        )
                    )
                    """
                )
            ).legend_format("Total Clusters"),
            promql_query(
                textwrap.dedent(
                    """
                    count(
                        group by (compute_cluster_id) (
                            v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList", compute_cluster_id=~"^s.*"}
                        )
                    )
                    """
                )
            ).legend_format("System Clusters"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Cluster Count")
            .description("Number of clusters.")
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=CLUSTERS_THEME)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .min(0)
            ),
        )
        return panel_id

    def _replica_count_panel(self):
        """Show the number of replicas.

        NOTE: cluster/replica filters are not considered
        """
        panel_id = "replica-count"
        # v2_mz_clusters_count and v2_mz_cluster_reps_count are weirdly environment scoped
        # so we recalculate them
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    count(
                        group by (compute_cluster_id, compute_replica_id) (
                            v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}
                        )
                    )
                    """
                )
            ).legend_format("Total Replicas"),
            promql_query(
                textwrap.dedent(
                    """
                    count(
                        group by (compute_cluster_id, compute_replica_id) (
                            v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList", compute_replica_name!="r1"}
                        )
                    ) or vector(0)
                    """
                )
            ).legend_format("Additional Replicas"),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Replica Count")
            .description(
                "Number of replicas. Additional replicas are those beyond the first replica of the cluster."
            )
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=CLUSTERS_THEME)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .min(0)
            ),
        )
        return panel_id

    def _instance_sizes_panel(self):
        """Show the sizes of replicas."""
        panel_id = "replica-sizes"
        # TODO: add transformations to allow hovering to show which clusters are each size
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    count by (size) (
                        v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}
                    )
                    """
                )
            )
            .legend_format("{{size}}")
            .instant(),
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Replica Sizes")
            .description("Sizes of replicas.")
            .data(query)
            .visualization(
                piechart_builder.Visualization()
                .display_labels(
                    [piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE]
                )
                .color_scheme(
                    dashboardv2_builders.FieldColor()
                    .mode(dashboardv2.FieldColorModeId.SHADES)
                    .fixed_color(CLUSTERS_THEME)
                )
                .legend(visualization.PIE_LEGEND_BUILDER)
                .no_value("No matches for the current filters")
            ),
        )
        return panel_id

    def _az_distribution_panel(self):
        """Show distribution of replicas across availability zones."""
        panel_id = "replica-azs"
        # FIXME: label name may be updated in later versions
        query = query_group(
            promql_query(
                textwrap.dedent(
                    """
                    sum by (materialize_cloud_availability_zone) (
                        count by (compute_cluster_id, compute_replica_id, materialize_cloud_availability_zone) (
                            v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}
                        )
                        or
                        count by (compute_cluster_id, compute_replica_id, materialize_cloud_availability_zone) (
                            v2_mz_compute_cluster_status{}
                        ) * 0
                    )
                    """
                )
            ).legend_format("{{materialize_cloud_availability_zone}}")
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Replica Availability Zones")
            .description("Distribution of replicas across availability zones.")
            .data(query)
            .visualization(
                visualization.sparkline_stat(shade=CLUSTERS_THEME)
                .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
                .justify_mode(common.BigValueJustifyMode.CENTER)
                .no_value("No matches for the current filters or AZ label missing.")
            ),
        )
        return panel_id

    def build_cluster_summary_row(self) -> dashboardv2_builders.Row:
        """Build a row summarizing clusters/replicas."""
        return (
            dashboardv2_builders.Row()
            .title("Cluster Summary")
            .hide_header(True)
            .layout(
                dashboardv2_builders.AutoGrid()
                .row_height_mode("short")
                .with_item(self._cluster_count_panel())
                .with_item(self._replica_count_panel())
            )
        )

    def build_replication_summary_row(self) -> dashboardv2_builders.Row:
        """Build a row summarizing replication status."""
        return (
            dashboardv2_builders.Row()
            .title("Replication / Availability")
            .layout(
                dashboardv2_builders.AutoGrid()
                .with_item(self._instance_sizes_panel())
                .with_item(self._az_distribution_panel())
            )
        )

    def _cluster_table_panel(self):
        """Show a table of clusters and their statuses."""
        panel_id = "cluster-table"

        columns = [
            "compute_cluster_name",
            "compute_replica_name",
            "compute_cluster_id",
            "compute_replica_id",
            "mz_version",
            "size",
            "materialize_cloud_availability_zone",
            "topology_kubernetes_io_region",
            "topology_kubernetes_io_zone",
        ]
        query = (
            query_group(
                promql_query(
                    textwrap.dedent(
                        """
                        v2_mz_compute_cluster_status{$environmentFilter, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}
                        """
                    )
                ).instant()
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("labelsToFields")
                .id("labelsToFields")
                .options(
                    {
                        "keepLabels": columns,
                    }
                )
            )
            # remove Time and Value basically
            # .transformation(
            #     transform_builders.CompatTransformationBuilder()
            #     .group("filterFieldsByName")
            #     .id("filterFieldsByName")
            #     .options(
            #         {
            #             "include": {
            #                 "names": columns,
            #             }
            #         }
            #     )
            # )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("merge")
                .id("merge")
                .options({})
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("organize")
                .id("organize")
                .options(
                    {
                        "excludeByName": {
                            "Time": True,
                            "v2_mz_compute_cluster_status": True,
                        },
                        "indexByName": {
                            column_name: columns.index(column_name)
                            for column_name in columns
                        },
                    }
                )
            )
            .transformation(
                transform_builders.CompatTransformationBuilder()
                .group("sortBy")
                .id("sortBy")
                .options(
                    {
                        "fields": {},
                        "sort": [
                            {"field": "compute_cluster_name"},
                        ],
                    }
                )
            )
        )

        self.dashboard.add_panel(
            panel_id,
            dashboardv2_builders.Panel()
            .title("Cluster Information")
            .description("Information about clusters and replicas in this environment.")
            .data(query)
            .visualization(
                table.Visualization()
                # at least one option is required to be set to avoid schema error
                .show_header(True)
                .filterable(True)
                .no_value("No matches for the current filters")
            ),
        )
        return panel_id

    def build_cluster_info_row(self) -> dashboardv2_builders.Row:
        """Build a row with cluster info panels."""
        return (
            dashboardv2_builders.Row()
            .title("Cluster Information")
            .layout(
                dashboardv2_builders.AutoGrid().with_item(self._cluster_table_panel())
            )
        )

    def build(self) -> dashboardv2_builders.Tab:
        """Generate a clusters/replicas/availability tab."""
        return (
            dashboardv2_builders.Tab()
            .title("Cluster Objects / Replicas")
            .layout(
                dashboardv2_builders.Rows()
                .row(self.build_cluster_summary_row())
                .row(self.build_replication_summary_row())
                .row(self.build_cluster_info_row())
            )
        )
