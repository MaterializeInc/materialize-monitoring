// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The node detail dashboard: one machine, in full.
//!
//! The second of the `infra-*` family. `infra-logs` asks what the platform said;
//! this asks what one machine underneath it is doing — and, above all, whether
//! the answer to a Materialize problem is "the node".
//!
//! # Who it is for
//!
//! Someone operating Materialize who **cannot run `kubectl describe node`**,
//! either for want of access or for want of knowing that is where to look. The
//! whole dashboard is built around one decision: is this mine to fix, or is this
//! the moment to escalate to whoever runs the cluster. So it puts the facts an
//! infrastructure team will ask for — node name, kubelet version, kernel,
//! capacity, conditions — where they can be read off directly, and it says in the
//! panel descriptions which readings are worth reporting.
//!
//! # One node, deliberately
//!
//! [`variable::node`] is single-select with no "All". Averaging a machine's
//! measurements across a fleet makes every panel here ambiguous: a node at 100%
//! beside four idle ones reads as 20%, which is the one answer that is certainly
//! wrong. Fleet views are a separate dashboard, and a genuinely different
//! question — *which* node — rather than a wider version of this one.
//!
//! # Two names for the same machine
//!
//! kube-state-metrics calls a node `node="<name>"`; node-exporter calls it
//! `instance="<ip>:9100"` and carries the name only on `node_uname_info`. The
//! operator picks a name, and the hidden [`variable::node_instance`] resolves it
//! to the address through that metric — so the node-exporter query families back
//! this dashboard unchanged, writing `instance=~"$nodeList"` exactly as they
//! always have. Loki knows the node a third way again, as structured metadata on
//! journal lines. All three agree on the string; only the label differs.
//!
//! # Mixed datasource
//!
//! Thanos for the measurements, Loki for the Logs & Events tab. Unlike the logs
//! dashboards this one *does* define a metrics datasource, because most of it is
//! metrics — which means it degrades differently: if the metrics pipeline breaks,
//! this dashboard's pickers still resolve but its panels empty, and the last tab
//! is the one still worth reading.

pub mod cpu;
pub mod logs;
pub mod memory;
pub mod network;
pub mod pods;
pub mod storage;
pub mod summary;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-nodes";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "infra-nodes";

/// Dashboard title.
pub const TITLE: &str = "Infrastructure Node Detail";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular: nothing here reads a Materialize signal at all. Every
/// panel is fed by node-exporter, kube-state-metrics, or the node's own journal.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order.
///
/// Summary first because it is where the escalate-or-not decision gets made, then
/// the four resources in the order they run out on a database node — CPU, then
/// memory, then network, then disk — then what is actually scheduled here, and
/// the node's own words last, since that is where an investigation goes once a
/// panel has pointed at something.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::SUMMARY.title).rows(summary::rows(q)),
        Tab::new(theme::CPU.title).rows(cpu::rows(q)),
        Tab::new(theme::MEMORY.title).rows(memory::rows(q)),
        Tab::new(theme::NETWORK.title).rows(network::rows(q)),
        Tab::new(theme::STORAGE.title).rows(storage::rows(q)),
        Tab::new(theme::PODS.title).rows(pods::rows(q)),
        Tab::new(theme::LOGS.title).rows(logs::rows(q)),
    ]
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here — no query on this dashboard is a
/// SQL-derived Materialize metric — but it stays in the signature so every
/// dashboard is built the same way and the renderer needs no special case.
pub fn build(sql_metric_prefix: &str, registry: &QueryRegistry) -> dashboard::Result<Resource> {
    let scope = DashboardScope::for_prefix(sql_metric_prefix);
    let queries = Queries::new(registry, &scope);
    let layout = Layout::tabs(tabs(&queries));

    let failures = queries.failures();
    if !failures.is_empty() {
        return Err(dashboard::Error::Registry {
            dashboard: NAME_STEM,
            failures,
        });
    }

    Dashboard::new(NAME, TITLE)
        .description(
            "Everything about one node a Materialize deployment runs on.\n\n\
             What the machine is, how hard it is working, how much of it is already \
             promised to pods, and what it and Kubernetes have said about it.",
        )
        .tags(["infrastructure", "monitoring", "nodes"])
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::node_scoped())
        .metadata_annotation(
            "monitoring.materialize.cloud/min-mz-version",
            MIN_MZ_VERSION,
        )
        .metadata_annotation(
            "monitoring.materialize.cloud/rec-mz-version",
            REC_MZ_VERSION,
        )
        .metadata_annotation(
            "monitoring.materialize.cloud/sql-metric-prefix",
            sql_metric_prefix,
        )
        .metadata_annotation("monitoring.materialize.cloud/target-export", TARGET_EXPORT)
        .layout(layout)
        .build()
}

/// Render for the registry.
pub fn render(
    options: &crate::grafana::Options,
    registry: &QueryRegistry,
) -> crate::grafana::render::Result<Resource> {
    use crate::grafana::render::Error;

    build(&options.sql_metric_prefix, registry).map_err(|source| Error::Build {
        name: NAME_STEM,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_registry;

    fn built() -> Resource {
        build("mz_", test_registry()).expect("build")
    }

    #[test]
    fn it_builds() {
        let resource = built();
        assert_eq!(resource.metadata.name, NAME);
        assert_eq!(resource.spec.title, TITLE);
    }

    #[test]
    fn it_is_mixed_datasource() {
        // Both, unlike the logs dashboards: the measurements are Thanos and the
        // last tab is Loki.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        assert!(names.contains(&"metricsDatasource"), "{names:?}");
        assert!(names.contains(&"logsDatasource"), "{names:?}");
    }

    #[test]
    fn the_node_picker_selects_exactly_one() {
        // Averaging a machine's measurements across a fleet is the one answer
        // that is certainly wrong, so there is no multi and no "All".
        let resource = built();
        let node = resource
            .spec
            .variables
            .iter()
            .find(|v| variable::name_of(v) == "node")
            .expect("a node picker");
        let json = serde_json::to_string(node).expect("serialize");
        assert!(json.contains(r#""multi":false"#), "{json}");
        assert!(json.contains(r#""includeAll":false"#), "{json}");
    }

    #[test]
    fn the_instance_lookup_is_hidden_and_chained() {
        // It is a join, not a control: showing it would offer a second node
        // picker that has to agree with the first.
        let resource = built();
        let instance = resource
            .spec
            .variables
            .iter()
            .find(|v| variable::name_of(v) == "nodeList")
            .expect("an instance lookup");
        let json = serde_json::to_string(instance).expect("serialize");
        assert!(json.contains(r#""hide":"hideVariable""#), "{json}");
        assert!(json.contains("node_uname_info"), "{json}");
        assert!(json.contains("$node"), "{json}");
    }

    #[test]
    fn multi_series_panels_are_not_shaded() {
        // `shade` sets Grafana's `shades` colour mode, which tints every series
        // in a panel from one hue. On a single-value panel that is a deliberate
        // identity; on a graph drawing five modes or one line per core it makes
        // the lines almost impossible to tell apart, which defeats the panel.
        //
        // Graphs only. A stat reducing a templated query to one number has no
        // lines to confuse, and its fixed shade is what makes the info row read
        // as one block.
        let resource = built();
        let json = serde_json::to_value(&resource.spec.elements).expect("serialize");
        for (name, element) in json.as_object().expect("elements") {
            let Some(panel) = element.get("spec") else {
                continue;
            };
            if panel.pointer("/vizConfig/group").and_then(|g| g.as_str()) != Some("timeseries") {
                continue;
            }
            let Some(queries) = panel
                .pointer("/data/spec/queries")
                .and_then(|q| q.as_array())
            else {
                continue;
            };
            let templated = queries.iter().any(|q| {
                q.pointer("/spec/query/spec/legendFormat")
                    .and_then(|l| l.as_str())
                    .is_some_and(|l| l.contains("{{"))
            });
            if queries.len() <= 1 && !templated {
                continue;
            }
            let mode = panel.pointer("/vizConfig/spec/fieldConfig/defaults/color/mode");
            assert_ne!(
                mode.and_then(|m| m.as_str()),
                Some("shades"),
                "{name} draws several series but pins them to one shade"
            );
        }
    }

    #[test]
    fn cordoning_is_read_in_the_right_direction() {
        // `kube_node_spec_unschedulable` is 0 when the node is *accepting* work,
        // which is the opposite of every other health signal here. Read with the
        // usual mapping it labelled a perfectly healthy node "Unhealthy".
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        let panel = json
            .split("summary-node-unschedulable")
            .nth(1)
            .expect("the cordon panel");
        let mappings = &panel[..panel.len().min(2000)];
        assert!(mappings.contains("Schedulable"), "{mappings}");
        assert!(mappings.contains("Cordoned"), "{mappings}");
        assert!(
            !mappings.contains("Unhealthy"),
            "cordon panel still uses the health vocabulary: {mappings}"
        );
    }

    #[test]
    fn a_table_with_sparse_columns_sets_no_placeholder() {
        // Grafana applies `noValue` per *field*, not per panel, so on a table
        // whose columns are legitimately sparse it fills every empty cell instead
        // of standing in for an empty panel. The budgets table joins four queries
        // and most pods set no limit, so a placeholder there put a
        // collection-failure message in the majority of cells.
        //
        // Single-query tables are unaffected -- their columns come from one
        // result, so a row exists in full or not at all -- which is why this is
        // scoped to the panel that joins several.
        let resource = built();
        let json = serde_json::to_value(&resource.spec.elements).expect("serialize");
        for (name, element) in json.as_object().expect("elements") {
            let Some(panel) = element.get("spec") else {
                continue;
            };
            if panel.pointer("/vizConfig/group").and_then(|g| g.as_str()) != Some("table") {
                continue;
            }
            let queries = panel
                .pointer("/data/spec/queries")
                .and_then(|q| q.as_array())
                .map_or(0, Vec::len);
            if queries <= 1 {
                continue;
            }
            assert_eq!(
                panel.pointer("/vizConfig/spec/fieldConfig/defaults/noValue"),
                None,
                "{name} joins several queries, so a noValue would fill its empty cells"
            );
        }
    }

    #[test]
    fn every_table_lands_in_one_frame() {
        // A Prometheus query returns one frame per series unless asked for table
        // format, and a Table panel handed several frames renders a *frame
        // picker* — a dropdown at the foot of the panel — instead of one table.
        // That dropdown is the tell, and it is easy to miss because the first
        // frame renders correctly.
        //
        // Two ways out: ask the datasource for table format, or consolidate with
        // a transformation (`merge` joins frames, `reduce` in `seriesToRows` mode
        // collapses them to a row each). Every table here must do one of them.
        let resource = built();
        let json = serde_json::to_value(&resource.spec.elements).expect("serialize");
        for (name, element) in json.as_object().expect("elements") {
            let Some(panel) = element.get("spec") else {
                continue;
            };
            if panel.pointer("/vizConfig/group").and_then(|g| g.as_str()) != Some("table") {
                continue;
            }
            let queries = panel
                .pointer("/data/spec/queries")
                .and_then(|q| q.as_array())
                .expect("queries");
            let all_table_format = queries.iter().all(|q| {
                q.pointer("/spec/query/spec/format")
                    .and_then(|f| f.as_str())
                    == Some("table")
            });
            let consolidates = panel
                .pointer("/data/spec/transformations")
                .and_then(|t| t.as_array())
                .is_some_and(|ts| {
                    ts.iter().any(|t| {
                        matches!(
                            t.get("group").and_then(|g| g.as_str()),
                            Some("merge") | Some("reduce")
                        )
                    })
                });
            assert!(
                all_table_format || consolidates,
                "{name} is a table whose frames are never joined, so it will render a frame picker"
            );
        }
    }

    #[test]
    fn every_tab_is_present() {
        let resource = built();
        let json = serde_json::to_string(&resource.spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }
}
