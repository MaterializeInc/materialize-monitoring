// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The cluster networking dashboard: how traffic moves, and what stops it.
//!
//! The third of the `infra-*` family. `infra-logs` asks what the platform said
//! and `infra-nodes` asks what one machine is doing; this asks what the
//! *network* between them is doing.
//!
//! # Who it is for
//!
//! The same operator `infra-nodes` was built for — someone running Materialize
//! who may not have `kubectl` and certainly does not have the cloud console —
//! arriving with one of three questions:
//!
//! - *Something cannot reach something else.* Start at Kubernetes (are there
//!   endpoints behind the Service, is kube-proxy keeping up) and then Security
//!   (is a policy stopping it).
//! - *Something is slow or lossy.* Start at Overview, which puts losses beside
//!   the traffic they happened during, and follow it down to Node Networking.
//! - *Pods will not start, with an address error.* That is the CNI tab, and on
//!   EKS it is almost always address exhaustion.
//!
//! # The dashboard adapts to the cluster
//!
//! **This is the first dashboard here whose layout is not fixed.** A cluster's
//! CNI is not knowable from this repository — EKS defaults to the AWS VPC CNI,
//! GKE to Dataplane V2, AKS to Azure CNI powered by Cilium, and a
//! bring-your-own cluster to anything at all — and the metrics describing each
//! share nothing with the others. `awscni_ip_max` and `cilium_bpf_map_pressure`
//! are different facts about differently-shaped software, not two spellings of
//! one idea.
//!
//! So the CNI monitors stamp every series with a `network_component` label,
//! [`variable::network_components`] discovers it, and the vendor rows are
//! rendered on that variable. A cluster sees its own dataplane and none of the
//! others. The alternative was a dashboard per cloud, which this repo built once
//! for GCP and retired when the two artifacts stopped differing in anything but
//! which panels were blank.
//!
//! Detection reads `up` rather than a vendor metric, which distinguishes the two
//! cases that matter: a target being scraped and returning nothing is a
//! collection bug, and no target at all is a cluster that runs something else.
//!
//! # Two identifier spaces, and why only one is a picker
//!
//! node-exporter names a machine `instance="<ip>:9100"`; kube-state-metrics,
//! cAdvisor and the CNI monitors all name it `node="<kubernetes name>"`.
//! `infra-nodes` bridges them with a hidden lookup because it is scoped to one
//! machine and can afford to resolve one name.
//!
//! A fleet dashboard cannot: the bridge is a per-node join, and doing it for
//! every node in every query would put a `node_uname_info` lookup inside a
//! hundred expressions. So [`variable::node_instances`] is the node-exporter
//! form — which buys the whole of `node-health.yaml` and `node-debug.yaml`
//! unchanged — and **nothing here scopes a `node` label by it**. The CNI and
//! Kubernetes tabs are cluster-wide instead, breaking down *by* node rather than
//! filtering to one. A per-node question is what `infra-nodes` is for, and every
//! panel that raises one says so.
//!
//! # Metrics only
//!
//! No Loki. The network's own account of itself is in the node journal and in
//! Kubernetes events, and both are already on `infra-logs` and `infra-nodes`
//! with better pickers than this dashboard would give them.

pub mod cloud;
pub mod cni;
pub mod kubernetes;
pub mod nodes;
pub mod overview;
pub mod security;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-net";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "infra-net";

/// Dashboard title.
pub const TITLE: &str = "Infrastructure Networking";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular: nothing here reads a Materialize signal at all. Every
/// panel is fed by cAdvisor, kube-state-metrics, node-exporter, or a CNI.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order.
///
/// Overview first because it is where the "is anything wrong" question is
/// answered. Then the stack downward — Kubernetes routing, the CNI under it, the
/// node's own interfaces under that — because that is the order in which a cause
/// gets further from anything the reader controls. Cloud Networking sits outside
/// the cluster entirely, and Security is last because it is a different question
/// rather than a deeper one: not *is the network working* but *is it allowed*.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::KUBERNETES.title).rows(kubernetes::rows(q)),
        Tab::new(theme::CNI.title).rows(cni::rows(q)),
        Tab::new(theme::NODES.title).rows(nodes::rows(q)),
        Tab::new(theme::CLOUD.title).rows(cloud::rows(q)),
        Tab::new(theme::SECURITY.title).rows(security::rows(q)),
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
            "How traffic moves through the cluster Materialize runs on, and what stops it.\n\n\
             Pod and Service networking, the CNI underneath — discovered rather than \
             configured — the nodes' own interfaces, and the NetworkPolicy objects that \
             decide what is allowed.",
        )
        .tags([tags::INFRA, tags::MZMON, tags::content::NETWORK])
        .folder(Folder::Infra)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::network_scoped())
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
    use serde_json::Value;

    fn built() -> Resource {
        build("mz_", test_registry()).expect("build")
    }

    /// Every row in the rendered layout, as JSON.
    fn rows(resource: &Resource) -> Vec<Value> {
        fn collect(value: &Value, out: &mut Vec<Value>) {
            match value {
                Value::Object(map) => {
                    if map.get("kind").and_then(|k| k.as_str()) == Some("RowsLayoutRow") {
                        out.push(value.clone());
                    }
                    for v in map.values() {
                        collect(v, out);
                    }
                }
                Value::Array(items) => {
                    for v in items {
                        collect(v, out);
                    }
                }
                _ => {}
            }
        }
        let json = serde_json::to_value(&resource.spec.layout).expect("serialize");
        let mut out = Vec::new();
        collect(&json, &mut out);
        out
    }

    /// The title and variable-condition of every conditionally rendered row.
    fn conditioned(resource: &Resource) -> Vec<(String, String, String)> {
        rows(resource)
            .iter()
            .filter_map(|row| {
                let spec = row.get("spec")?;
                let title = spec.get("title")?.as_str()?.to_string();
                let item = spec
                    .pointer("/conditionalRendering/spec/items/0")?
                    .get("spec")?;
                // Skips a time-range condition, which carries no variable.
                item.get("variable")?;
                Some((
                    title,
                    item.get("operator")?.as_str()?.to_string(),
                    item.get("value")?.as_str()?.to_string(),
                ))
            })
            .collect()
    }

    #[test]
    fn it_builds() {
        let resource = built();
        assert_eq!(resource.metadata.name, NAME);
        assert_eq!(resource.spec.title, TITLE);
    }

    #[test]
    fn every_tab_is_present() {
        let resource = built();
        let json = serde_json::to_string(&resource.spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }

    #[test]
    fn it_is_metrics_only() {
        // No Loki here, unlike `infra-nodes`: the network's own account of
        // itself is on the logs dashboards, with better pickers than this one
        // would give it. A stray `logsDatasource` would mean a LogQL panel had
        // crept in.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        assert!(names.contains(&"metricsDatasource"), "{names:?}");
        assert!(!names.contains(&"logsDatasource"), "{names:?}");
    }

    #[test]
    fn the_vendor_rows_are_rendered_on_the_detected_dataplane() {
        // The dashboard's whole adaptive behavior. Every vendor-specific row
        // must carry a `matches` on the discovery variable, or it renders on a
        // cluster running something else and shows nothing but empty panels.
        let resource = built();
        let conditions = conditioned(&resource);
        let vendor_rows: Vec<&(String, String, String)> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .collect();
        assert!(
            vendor_rows.len() >= 5,
            "expected a row per vendor section, got {conditions:?}"
        );
        for (title, _, value) in &vendor_rows {
            assert!(
                value == cni::AWS || value == cni::CILIUM,
                "{title} is conditioned on an unknown dataplane {value}"
            );
        }
    }

    #[test]
    fn a_cluster_with_no_dataplane_still_has_something_on_the_cni_tab() {
        // A tab whose every condition failed is indistinguishable from a broken
        // one. Exactly one `notMatches` row covers that case, and its pattern
        // has to name every vendor that has rows -- otherwise it renders
        // *beside* a vendor's panels and contradicts them.
        let resource = built();
        let fallbacks: Vec<&(String, String, String)> = conditioned(&resource)
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .cloned()
            .collect::<Vec<_>>()
            .leak()
            .iter()
            .collect();
        assert_eq!(fallbacks.len(), 1, "{fallbacks:?}");
        let (_, _, pattern) = &fallbacks[0];
        for vendor in [cni::AWS, cni::CILIUM] {
            assert!(
                pattern.contains(vendor),
                "the fallback does not cover {vendor}: {pattern}"
            );
        }
    }

    #[test]
    fn no_panel_scopes_a_node_label_by_the_node_picker() {
        // `$nodeList` holds node-exporter *addresses*; every family carrying a
        // `node` label spells it as the Kubernetes *name*. One picker cannot
        // serve both, and the failure is silent -- the matcher simply matches
        // nothing and the panel reads as "no data".
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        assert!(
            !json.contains(r#"node=~\"$nodeList\""#),
            "a panel filters a node name with the node-exporter address picker"
        );
    }

    #[test]
    fn multi_series_panels_are_not_shaded() {
        // `shade` derives every series in a panel from one hue, which on a graph
        // drawing one line per node makes them indistinguishable. Same rule
        // `infra-nodes` holds, and this dashboard has more per-node graphs than
        // any other.
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
    fn every_table_lands_in_one_frame() {
        // A Table panel handed several frames renders a frame *picker* rather
        // than a table, and the tell is easy to miss because the first frame
        // renders correctly. Either ask for table format or consolidate.
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
}
