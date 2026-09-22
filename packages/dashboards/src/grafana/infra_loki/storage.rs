// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Storage tab: where it all ends up, and what tidies it away.
//!
//! Three rows, and they fail on three different timescales — which is the reason
//! they are separate rather than one long list.
//!
//! **Object storage** fails in seconds and loudly: a credential expires, and
//! uploads stop. **The index** fails in minutes and quietly: data keeps arriving
//! and stops being findable, which looks like a query problem. **Maintenance**
//! fails over days and silently: retention stops running and the only symptom is
//! the storage bill, weeks later.
//!
//! The last of those is why this tab leads with two "how long ago" stats rather
//! than with a rate. A compactor that has not run is not a rate of zero — it is
//! the absence of a rate, which no rate panel can draw.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::STORAGE.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![maintenance(q), object_store(q), index(q), deletion(q)]
}

fn maintenance(q: &Queries) -> Row {
    Row::new("Maintenance").hide_header().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .panel("storage-retention-age", retention_age(q))
            .panel("storage-compaction-age", compaction_age(q))
            .panel("storage-compaction-runs", compaction_runs(q)),
    )
}

fn object_store(q: &Queries) -> Row {
    Row::new("Object Storage").grid(
        AutoGrid::new(3)
            .panel("storage-operations", operations(q))
            .panel("storage-failures", failures(q))
            .panel("storage-latency", latency(q)),
    )
}

fn index(q: &Queries) -> Row {
    Row::new("Index").grid(
        AutoGrid::new(2)
            .panel("storage-index-sync", index_sync(q))
            .panel("storage-index-wait", index_wait(q)),
    )
}

fn deletion(q: &Queries) -> Row {
    Row::new("Deletion Requests")
        .grid(AutoGrid::new(1).panel("storage-deletes", deletes_pending(q)))
}

/// A sawtooth is the healthy shape here, not a flat line — the value counts up
/// between runs and drops each time one completes. Said on the registry query, so
/// the panel tooltip carries it.
fn retention_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Retention Last Ran")
        .query(q.get("infra.loki.store.retention_age").legend("ago"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        // Absent means the compactor has never completed a retention pass, which
        // on a new install is ordinary and on an old one is the finding.
        .no_value(NoValue::Custom(
            "Retention has never completed — expected on a new install, a finding on an old one"
                .to_string(),
        ))
        .build(0)
}

fn compaction_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Compaction Last Ran")
        .query(q.get("infra.loki.store.compaction_age").legend("ago"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Compaction has never completed — expected on a new install, a finding on an old one"
                .to_string(),
        ))
        .build(0)
}

fn compaction_runs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Compaction Runs by Outcome")
        .query(
            q.get("infra.loki.store.compaction_runs")
                .legend("{{status}}"),
        )
        .unit("short")
        .min(0.0)
        // "No runs" and "compactor is not running" look identical on a rate
        // panel, so the empty text has to name the second possibility.
        .no_value(NoValue::Custom(
            "No compaction ran in this range — check Scrape Health if that is unexpected"
                .to_string(),
        ))
        .build(0)
}

fn operations(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Operations by Kind")
        .query(q.get("infra.loki.store.operations").legend("{{operation}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No object-storage traffic in this range".to_string(),
        ))
        .build(0)
}

fn failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failures by Kind")
        .query(q.get("infra.loki.store.failures").legend("{{operation}}"))
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No object-storage operation failed.".to_string(),
        ))
        .build(0)
}

fn latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Operation Latency (p99)")
        .query(q.get("infra.loki.store.latency").legend("{{operation}}"))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No object-storage traffic in this range".to_string(),
        ))
        .build(0)
}

fn index_sync(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Index Table Operations")
        .query(q.legended("infra.loki.store.index_sync", &["sync", "upload"]))
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No index tables were synced or uploaded in this range".to_string(),
        ))
        .build(0)
}

fn index_wait(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Query Wait for Index (p99)")
        .query(q.get("infra.loki.store.index_wait").legend("wait"))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No query waited on an index table.".to_string(),
        ))
        .build(0)
}

/// Both series on one panel because neither is readable alone: a count with no
/// age does not say whether anything is stuck, and an age with no count does not
/// say how much is.
fn deletes_pending(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pending Delete Requests")
        .query(q.legended(
            "infra.loki.store.deletes_pending",
            &["pending requests", "oldest request age"],
        ))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No delete requests are pending.".to_string(),
        ))
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
        assert_eq!(assembled.elements.len(), 9);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_maintenance_stats_are_instant_queries() {
        // "How long since the last successful run" is a current fact. Over a
        // range it draws a sawtooth per series and reduces to whichever point
        // the reducer happened to land on, which is not the question.
        let q = &test_queries();
        for panel in [retention_age(q), compaction_age(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains(r#""instant":true"#), "{json}");
        }
    }

    #[test]
    fn the_maintenance_panels_distinguish_never_ran_from_zero() {
        // The failure this tab exists for is silent: retention stops and nothing
        // else changes. An empty panel must not read as "nothing to report".
        let q = &test_queries();
        for panel in [retention_age(q), compaction_age(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains("never completed"), "{json}");
        }
    }
}
