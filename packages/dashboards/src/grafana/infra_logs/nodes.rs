// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Nodes tab: the systemd journal from the machines everything runs on.
//!
//! **This is the tab no namespace-scoped dashboard can offer.** Journal lines
//! carry `unit`, `component`, `job`, `level` and `service_name` and no
//! `namespace`, `app` or `container` at all, because they come from the node
//! rather than from a pod. Any selector requiring a namespace excludes them by
//! construction — `env-logs` cannot show them however its pickers are set.
//!
//! What lives here is the node's own control plane narrating itself: `kubelet`
//! and `containerd` above all, plus the node problem detector and the cloud
//! provider's guest agent. When a pod will not start, will not stop, or a volume
//! will not attach, the explanation is usually in this tab and nowhere else.
//!
//! The `unit` picker is the axis, and it is also the anchor: a journal selector
//! has no namespace matcher to lean on, so its "All" is `.+` rather than `.*`.
//! The namespace, app, component and container pickers do not apply here — they
//! are the other tabs' controls, and Grafana has no way to scope a variable to
//! one tab.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::logs::{LINES_PER_SECOND, no_warnings};
use crate::grafana::queries::Queries;

/// What a panel shows when no journal line matched.
///
/// Distinct from the container-log message: journal collection is a separate
/// source with its own agent path, so its absence points somewhere else.
fn no_journal() -> NoValue {
    NoValue::Custom("No node journal logs — is journal collection enabled?".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![volume(q), warnings(q), all_logs(q)]
}

fn volume(q: &Queries) -> Row {
    Row::new("Journal Volume").grid(AutoGrid::new(1).panel("node-rate-by-unit", rate_by_unit(q)))
}

fn warnings(q: &Queries) -> Row {
    Row::new("Node Warnings").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("node-warning-feed", warning_feed(q)),
    )
}

fn all_logs(q: &Queries) -> Row {
    Row::new("All Node Logs").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("node-log-feed", log_feed(q)),
    )
}

fn rate_by_unit(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Journal Rate by Unit")
        .query(q.logs("infra.logs.node.rate.by_unit").legend("{{unit}}"))
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(no_journal())
        .build(0)
}

fn warning_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Node Warnings and Errors")
        .query(q.logs("infra.logs.node.warnings"))
        .no_value(no_warnings())
        .build(0)
}

fn log_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Node Journal")
        .query(q.logs("infra.logs.node.stream"))
        .no_value(no_journal())
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_log_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_log_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 3);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn no_panel_here_requires_a_namespace() {
        // The whole reason this tab exists. Journal lines carry no `namespace`,
        // `app` or `container`, so a selector naming any of them matches nothing
        // — and would do so silently, since an empty log panel looks like a quiet
        // one.
        let q = &test_log_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = query.spec.query.spec.as_ref().expect("spec")["expr"]
                    .as_str()
                    .expect("expr");
                for absent in ["namespace=~", "app=~", "container=~"] {
                    assert!(
                        !expr.contains(absent),
                        "{name} selects on {absent}, which no journal line carries: {expr}"
                    );
                }
                // `unit` is the anchor, standing in for the namespace matcher the
                // container-log selectors lean on.
                assert!(
                    expr.contains("$logUnitList"),
                    "{name} has no anchor: {expr}"
                );
            }
        }
    }
}
