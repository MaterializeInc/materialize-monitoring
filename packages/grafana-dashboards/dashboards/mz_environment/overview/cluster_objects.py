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
from py_mzmon_lib.models_v2 import dashboardv2
from py_mzmon_lib.query import promql_query, query_group

from dashboards import palette, variables, visualization
from dashboards.mz_environment.mz_context import BaseMzContextTab

CLUSTERS_THEME = palette.THEME_PALETTE[2]

# Every query in this tab is keyed on the SQL-derived cluster-status metric:
#   {self.context.sql_metric_prefix}compute_cluster_status
#   -> mz_compute_cluster_status (self-managed) / v2_mz_compute_cluster_status (cloud)
# The prefix is baked in at generation time from config (see variables.py).


class ClusterObjectsTab(BaseMzContextTab):
    """Clusters/Replicas/Availability tab on Overview Dashboard."""

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
                    f"""
                    count(
                        group by (compute_cluster_id) (
                            {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList"}}
                        )
                    )
                    """
                )
            ).legend_format("Total Clusters"),
            promql_query(
                textwrap.dedent(
                    f"""
                    count(
                        group by (compute_cluster_id) (
                            {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList", compute_cluster_id=~"^s.*"}}
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
            .description(
                "**Number of clusters in the environment, split into "
                '"Total" and "System".** System clusters are '
                "Materialize-managed (e.g., `mz_catalog_server`, "
                "`mz_system`, `mz_probe`) and exist in every env; the "
                "difference between Total and System is the user clusters "
                "you've created. Stable in steady state; expected to step "
                "on `CREATE CLUSTER` / `DROP CLUSTER`. Scoped to the "
                "selected clusters."
            )
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
                    f"""
                    count(
                        group by (compute_cluster_id, compute_replica_id) (
                            {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}}
                        )
                    )
                    """
                )
            ).legend_format("Total Replicas"),
            promql_query(
                textwrap.dedent(
                    f"""
                    count(
                        group by (compute_cluster_id, compute_replica_id) (
                            {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList", compute_replica_name!="r1"}}
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
                "**Number of replicas across the selected clusters, with "
                '"Additional Replicas" calling out those beyond the '
                "first.** Every cluster needs at least one replica to run; "
                '"Additional" counts the redundancy on top of that — '
                "non-zero means at least one cluster has been configured "
                "for higher availability or extra capacity. Expected to "
                "step on `CREATE CLUSTER REPLICA` / `DROP CLUSTER "
                "REPLICA`. Scoped to the selected clusters."
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
                    f"""
                    count by (size) (
                        {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}}
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
            .description(
                "**Replicas grouped by their configured size.** Most "
                "workloads cluster around a small number of sizes; a long "
                "tail of one-off sizes usually means experimentation or "
                "migration in progress. The total here matches the "
                "Replica Count panel. Scoped to the selected clusters."
            )
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
        # TODO(self-managed): `materialize_cloud_availability_zone` is a cloud-only
        # label and is absent on self-managed instances, so this panel has no data
        # there. Kept for cloud parity; the no_value below explains the blank. When
        # a self-managed AZ/topology signal exists (e.g. a node topology label
        # joined via kube_pod_info), switch this query over to it.
        query = query_group(
            promql_query(
                textwrap.dedent(
                    f"""
                    sum by (materialize_cloud_availability_zone) (
                        count by (compute_cluster_id, compute_replica_id, materialize_cloud_availability_zone) (
                            {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}}
                        )
                        or
                        count by (compute_cluster_id, compute_replica_id, materialize_cloud_availability_zone) (
                            {self.context.sql_metric_prefix}compute_cluster_status{{}}
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
            .description(
                "**Replicas grouped by the cloud availability zone they're "
                "scheduled on.** For HA-sensitive deployments, replicas of "
                "the same cluster should be spread across AZs — heavy "
                "concentration in one AZ means an AZ outage takes more of "
                "the workload down with it. Materialize's scheduler "
                "spreads multi-replica clusters across AZs automatically, "
                "but ad-hoc single-replica clusters can land anywhere. "
                "Scoped to the selected clusters."
            )
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
                dashboardv2_builders.AutoGrid().with_item(self._instance_sizes_panel())
                # FIXME: AZ panel doesn't serve much value in its current form
                # it is also cloud-only
                # .with_item(self._az_distribution_panel())
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
                        f"""
                        {self.context.sql_metric_prefix}compute_cluster_status{{{variables.ENVIRONMENT_FILTER}, compute_cluster_id=~"$mzClusterList", compute_replica_id=~"$mzReplicaList"}}
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
                            # value-field name is the *resolved* metric, so cover
                            # both: mz_compute_cluster_status / v2_mz_compute_cluster_status
                            "mz_compute_cluster_status": True,
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
            .description(
                "**A row per (cluster, replica) tuple, with cluster_id / "
                "cluster_name / replica metadata and size / AZ / region "
                'info.** Operator\'s "what does my fleet look like" '
                "reference. The column-header filters let you narrow "
                "without changing the dashboard's cluster/replica "
                "selectors. Useful for copying a `cluster_id` or "
                "`replica_id` into the dashboard selectors to scope the "
                "rest of the dashboard."
            )
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
