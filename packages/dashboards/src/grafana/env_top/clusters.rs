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

use super::{theme, transform};
use crate::grafana::queries::Queries;

/// The tab's theme, applied to every shaded panel here.
const SHADE: &str = theme::CLUSTERS.shade;

/// The status metric this whole tab reads.
///
/// Not SQL-prefixed in the baseline's expressions: the panels were authored
/// against self-managed, where the prefix is `mz_`. A Cloud render needs the
/// prefix threaded through, which is tracked as a gap in the tab docs.
const STATUS: &str = "mz_compute_cluster_status";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![cluster_summary(q), replication(q), cluster_information(q)]
}

fn cluster_summary(q: &Queries) -> Row {
    Row::new("Cluster Summary").hide_header().grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("cluster-count", cluster_count(q))
            .panel("replica-count", replica_count(q)),
    )
}

fn replication(q: &Queries) -> Row {
    Row::new("Replication / Availability")
        .grid(AutoGrid::new(3).panel("replica-sizes", replica_sizes(q)))
}

fn cluster_information(q: &Queries) -> Row {
    Row::new("Cluster Information").grid(AutoGrid::new(3).panel("cluster-table", cluster_table(q)))
}

fn cluster_count(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Cluster Count")
        .query(q.legended(
            "materialize.clusters.count",
            &["Total Clusters", "System Clusters"],
        ))
        .text_mode(mzmon_lib::grafana::generated::stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn replica_count(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Replica Count")
        .query(q.legended(
            "materialize.clusters.replicas.count",
            &["Total Replicas", "Redundant Replicas"],
        ))
        .text_mode(mzmon_lib::grafana::generated::stat::BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn replica_sizes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Replica Sizes")
        .query(
            q.get("materialize.clusters.replicas.sizes")
                .legend("{{size}}"),
        )
        // A full pie rather than the default donut: sizes partition the replicas,
        // and a pie reads as a partition.
        .full_pie()
        .shade(SHADE)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn cluster_table(q: &Queries) -> dashboardv2::PanelKind {
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
        .query(q.get("materialize.clusters.info"))
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
        let q = &crate::grafana::queries::test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 4);
    }

    #[test]
    fn the_cluster_and_replica_counts_carry_two_series_each() {
        let q = &crate::grafana::queries::test_queries();
        // A single-series stat would lose the Total-versus-System contrast that is
        // the whole point of these two panels.
        for panel in [cluster_count(q), replica_count(q)] {
            assert_eq!(panel.spec.data.spec.queries.len(), 2);
        }
    }

    #[test]
    fn the_table_drops_both_metric_name_spellings() {
        let q = &crate::grafana::queries::test_queries();
        // Self-managed emits `mz_…`, Cloud `v2_mz_…`; leaving either in shows a
        // meaningless value column on one of them.
        let panel = cluster_table(q);
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
