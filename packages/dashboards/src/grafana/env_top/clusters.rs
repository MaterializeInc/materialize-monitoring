// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Cluster Objects / Replicas tab: what compute exists, and how it is shaped.
//!
//! Everything here reads `mz_compute_cluster_status`, a SQL-derived metric whose
//! cluster id arrives under `compute_cluster_id` rather than the `instance_id`
//! most compute metrics use — hence [`selector::compute_cluster`].

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::query::{PromQuery, query_group};

use super::{selector, theme, transform};

/// The tab's theme, applied to every shaded panel here.
const SHADE: &str = theme::CLUSTERS.shade;

/// The status metric this whole tab reads.
///
/// Not SQL-prefixed in the baseline's expressions: the panels were authored
/// against self-managed, where the prefix is `mz_`. A Cloud render needs the
/// prefix threaded through, which is tracked as a gap in the tab docs.
const STATUS: &str = "mz_compute_cluster_status";

/// The cluster and replica scope every query on this tab shares.
fn scope() -> String {
    format!(
        "{env}, {cluster}, {replica}",
        env = selector::environment(),
        cluster = selector::compute_cluster(),
        replica = selector::compute_replica()
    )
}

/// Cluster scope only, for panels that count clusters rather than replicas.
fn cluster_scope() -> String {
    format!(
        "{env}, {cluster}",
        env = selector::environment(),
        cluster = selector::compute_cluster()
    )
}

pub fn rows() -> Vec<Row> {
    vec![cluster_summary(), replication(), cluster_information()]
}

fn cluster_summary() -> Row {
    Row::new("Cluster Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("cluster-count", cluster_count())
            .panel("replica-count", replica_count()),
    )
}

fn replication() -> Row {
    Row::new("Replication / Availability")
        .grid(AutoGrid::new(3).panel("replica-sizes", replica_sizes()))
}

fn cluster_information() -> Row {
    Row::new("Cluster Information").grid(AutoGrid::new(3).panel("cluster-table", cluster_table()))
}

fn cluster_count() -> dashboardv2::PanelKind {
    // Two series on one stat: the total, and the system subset. Their difference
    // is what the operator actually created.
    let total = format!(
        r#"
count(
    group by (compute_cluster_id) (
        {STATUS}{{{scope}}}
    )
)
"#,
        scope = cluster_scope()
    );
    let system = format!(
        r#"
count(
    group by (compute_cluster_id) (
        {STATUS}{{{scope}, compute_cluster_id=~"{system}"}}
    )
)
"#,
        scope = cluster_scope(),
        system = selector::SYSTEM_CLUSTER_PATTERN
    );
    Panel::stat("Cluster Count")
        .description(
            "**Number of clusters in the environment, split into \"Total\" and \"System\".** \
             System clusters are Materialize-managed (e.g., `mz_catalog_server`, `mz_system`, \
             `mz_probe`) and exist in every env; the difference between Total and System is the \
             user clusters you've created. Stable in steady state; expected to step on `CREATE \
             CLUSTER` / `DROP CLUSTER`. Scoped to the selected clusters.",
        )
        .data(query_group(vec![
            PromQuery::new(total).legend("Total Clusters").build(),
            PromQuery::new(system).legend("System Clusters").build(),
        ]))
        .text_mode(mzmon_lib::grafana::generated::stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn replica_count() -> dashboardv2::PanelKind {
    let total = format!(
        r#"
count(
    group by (compute_cluster_id, compute_replica_id) (
        {STATUS}{{{scope}}}
    )
)
"#,
        scope = scope()
    );
    // `r1` is the first replica every cluster gets, so excluding it counts the
    // redundancy on top rather than the replicas themselves. `or vector(0)`
    // keeps the panel reading 0 instead of blank when there is none.
    let additional = format!(
        r#"
count(
    group by (compute_cluster_id, compute_replica_id) (
        {STATUS}{{{scope}, compute_replica_name!="r1"}}
    )
) or vector(0)
"#,
        scope = scope()
    );
    Panel::stat("Replica Count")
        .description(
            "**Number of replicas across the selected clusters, with \"Additional Replicas\" \
             calling out those beyond the first.** Every cluster needs at least one replica to \
             run; \"Additional\" counts the redundancy on top of that — non-zero means at least \
             one cluster has been configured for higher availability or extra capacity. \
             Expected to step on `CREATE CLUSTER REPLICA` / `DROP CLUSTER REPLICA`. Scoped to \
             the selected clusters.",
        )
        .data(query_group(vec![
            PromQuery::new(total).legend("Total Replicas").build(),
            PromQuery::new(additional)
                .legend("Additional Replicas")
                .build(),
        ]))
        .text_mode(mzmon_lib::grafana::generated::stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn replica_sizes() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
count by (size) (
    {STATUS}{{{scope}}}
)
"#,
        scope = scope()
    );
    Panel::piechart("Replica Sizes")
        .description(
            "**Replicas grouped by their configured size.** Most workloads cluster around a \
             small number of sizes; a long tail of one-off sizes usually means experimentation \
             or migration in progress. The total here matches the Replica Count panel. Scoped \
             to the selected clusters.",
        )
        .data(query_group(vec![
            PromQuery::new(expr).instant().legend("{{size}}").build(),
        ]))
        // A full pie rather than the default donut: sizes partition the replicas,
        // and a pie reads as a partition.
        .full_pie()
        .shade(SHADE)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn cluster_table() -> dashboardv2::PanelKind {
    let expr = format!("{STATUS}{{{scope}}}", scope = scope());
    // The metric's value says nothing here; the labels are the content. Promote
    // them to columns, collapse the per-series frames into one table, then drop
    // the value column and order what is left.
    const COLUMNS: &[&str] = &[
        "compute_cluster_name",
        "compute_replica_name",
        "compute_cluster_id",
        "compute_replica_id",
        "mz_version",
        "size",
        "materialize_cloud_availability_zone",
        "topology_kubernetes_io_region",
        "topology_kubernetes_io_zone",
    ];
    Panel::table("Cluster Information")
        .description(
            "**A row per (cluster, replica) tuple, with cluster_id / cluster_name / replica \
             metadata and size / AZ / region info.** Operator's \"what does my fleet look \
             like\" reference. The column-header filters let you narrow without changing the \
             dashboard's cluster/replica selectors. Useful for copying a `cluster_id` or \
             `replica_id` into the dashboard selectors to scope the rest of the dashboard.",
        )
        .data(query_group(vec![PromQuery::new(expr).instant().build()]))
        .transformations(vec![
            transform::labels_to_fields(COLUMNS),
            transform::merge(),
            transform::organize(
                // Both prefixes are excluded so the same table works on
                // self-managed and Cloud, where the metric arrives prefixed.
                &["Time", STATUS, "v2_mz_compute_cluster_status"],
                COLUMNS,
            ),
            transform::sort_by("compute_cluster_name", false),
        ])
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_has_three_rows_and_four_panels() {
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 4);
    }

    #[test]
    fn the_cluster_and_replica_counts_carry_two_series_each() {
        // A single-series stat would lose the Total-versus-System contrast that is
        // the whole point of these two panels.
        for panel in [cluster_count(), replica_count()] {
            assert_eq!(panel.spec.data.spec.queries.len(), 2);
        }
    }

    #[test]
    fn the_table_drops_both_metric_name_spellings() {
        // Self-managed emits `mz_…`, Cloud `v2_mz_…`; leaving either in shows a
        // meaningless value column on one of them.
        let panel = cluster_table();
        let organize = panel
            .spec
            .data
            .spec
            .transformations
            .iter()
            .find(|t| t.group == "organize")
            .expect("an organize transform");
        let options = serde_json::to_value(&organize.spec.options).expect("serialize");
        assert_eq!(options["excludeByName"]["mz_compute_cluster_status"], true);
        assert_eq!(
            options["excludeByName"]["v2_mz_compute_cluster_status"],
            true
        );
    }
}
