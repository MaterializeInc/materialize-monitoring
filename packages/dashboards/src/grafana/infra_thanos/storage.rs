// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Storage tab: the compactor, and object storage as every Thanos process
//! sees it.
//!
//! Both fail slowly. A halted compactor costs nothing for hours and then costs
//! retention and query speed for as long as nobody notices, which is why its
//! backlog is drawn rather than only its state.
//!
//! `thanos_objstore_*` is the bucket client's own measurement, so it says how
//! object storage behaves *from inside the cluster*. What the provider reports
//! about the bucket is `infra-cloud`'s Object Storage tab.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::not_collected;
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![compactor(q), blocks(q), object_storage(q)]
}

fn compactor(q: &Queries) -> Row {
    Row::new("Compactor").grid(
        AutoGrid::new(3)
            .panel("storage-backlog", backlog(q))
            .panel("storage-compactions", compactions(q))
            .panel("storage-compactor-failures", compactor_failures(q)),
    )
}

fn blocks(q: &Queries) -> Row {
    Row::new("Blocks").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("storage-blocks", block_states(q))
            .panel("storage-upload-age", upload_age(q)),
    )
}

fn object_storage(q: &Queries) -> Row {
    Row::new("Object Storage").grid(
        AutoGrid::new(3)
            .panel("storage-operations", operations(q))
            .panel("storage-failures", failures(q))
            .panel("storage-latency", latency(q)),
    )
}

fn no_compactor() -> NoValue {
    NoValue::Custom("Compactor not collected".to_string())
}

fn backlog(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Compaction Backlog")
        .query(q.legended(
            "infra.thanos.storage.backlog",
            &[
                "compactions",
                "blocks to compact",
                "blocks to downsample",
                "blocks to delete",
            ],
        ))
        .unit("short")
        .min(0.0)
        .no_value(no_compactor())
        .build(0)
}

fn compactions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Compactions per Hour by Resolution")
        .query(
            q.get("infra.thanos.storage.compactions")
                .legend("{{resolution}} ms"),
        )
        .unit("short")
        .min(0.0)
        .no_value(no_compactor())
        .build(0)
}

fn compactor_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Compactor Failures")
        .query(q.legended(
            "infra.thanos.storage.compactor_failures",
            &[
                "compaction",
                "downsampling",
                "garbage collection",
                "block deletion",
            ],
        ))
        .unit("suffix:/s")
        .min(0.0)
        .no_value(no_compactor())
        .build(0)
}

fn block_states(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Blocks in the Bucket by State")
        .query(q.get("infra.thanos.storage.blocks").legend("{{state}}"))
        .unit("short")
        .min(0.0)
        .no_value(no_compactor())
        .build(0)
}

/// Shared with the Writes tab, where the receivers' lines are the ones that
/// matter.
pub(super) fn upload_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Time Since Last Upload")
        .query(q.get("infra.thanos.storage.upload_age").legend("{{pod}}"))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No process has uploaded to object storage since it started".to_string(),
        ))
        .build(0)
}

fn operations(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Operations")
        .query(
            q.get("infra.thanos.storage.operations")
                .legend("{{container}} {{operation}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failure Ratio")
        .query(
            q.get("infra.thanos.storage.failures")
                .legend("{{container}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_collected())
        .build(0)
}

fn latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Latency (p99)")
        .query(
            q.get("infra.thanos.storage.latency")
                .legend("{{container}} {{operation}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 8);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn processes_that_never_uploaded_are_left_out() {
        // The store gateway and the ruler report a last-upload time of zero,
        // which drawn as an age reads as fifty-six years.
        let q = &test_queries();
        let json = serde_json::to_string(&upload_age(q)).expect("serialize");
        assert!(json.contains("> 0"), "{json}");
    }
}
