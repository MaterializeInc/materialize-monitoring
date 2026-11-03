// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The autoscaling dashboard: whether the cluster's nodes are keeping up with
//! its pods, on any cloud.
//!
//! # One dashboard for three autoscalers
//!
//! The self-managed Terraform adds nodes with Karpenter on EKS and with the
//! managed cluster autoscaler on GKE and AKS. The three publish nothing in
//! common, and on GKE and AKS the autoscaler runs in the control plane where it
//! cannot be scraped at all. So this dashboard is built from what every cluster
//! has — kube-state-metrics and Kubernetes events — and each question it asks
//! has the same answer on every cloud: are pods waiting for a node, how full is
//! each pool, what did the autoscaler do about it, and could the cloud supply
//! the node.
//!
//! Karpenter says far more about itself than the other two. Where that answers
//! one of those questions it is a row on the tab that asks it, and the rest is
//! the Karpenter tab; every one of those rows renders only where Karpenter is
//! scraped. `karpenter.rs` says how.
//!
//! # Node pools come from node labels
//!
//! Every per-pool panel reads `%%{nodePools}`, which takes the pool from
//! whichever label the node's provisioner sets. kube-state-metrics publishes
//! those only because the chart names them in `metricLabelsAllowlist`; on an
//! install from before that, every pool panel is empty and says why.
//!
//! # Cloud Capacity renders on the provider
//!
//! The same `$cloudProviderList` rows `infra-cloud` uses. A provider pull that
//! covers only the database and buckets still renders its row here, empty, and
//! the empty state names the value that fills it.

pub mod cloud;
pub mod events;
pub mod karpenter;
pub mod node_pools;
pub mod overview;
pub mod pending;
pub mod theme;
pub mod workloads;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::panel::NoValue;
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-autoscaling";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "infra-autoscaling";

/// Dashboard title.
pub const TITLE: &str = "Infrastructure Autoscaling";

/// Minimum Materialize version this dashboard requires. None in particular:
/// nothing here is Materialize's own.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order: the verdict, then capacity, then demand, then the
/// workloads that scale themselves, then the cloud underneath, then Karpenter's
/// own account, then the record.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::NODE_POOLS.title).rows(node_pools::rows(q)),
        Tab::new(theme::PENDING_PODS.title).rows(pending::rows(q)),
        Tab::new(theme::WORKLOADS.title).rows(workloads::rows(q)),
        Tab::new(theme::CLOUD_CAPACITY.title).rows(cloud::rows(q)),
        Tab::new(theme::KARPENTER.title).rows(karpenter::rows(q)),
        Tab::new(theme::EVENTS.title).rows(events::rows(q)),
    ]
}

/// What a per-pool panel shows with no node labels to group by.
///
/// The one likely cause is an install whose kube-state-metrics does not copy
/// node labels, so the text names the setting rather than guessing further.
pub(crate) fn no_pools() -> NoValue {
    NoValue::Custom(
        "No node labels. kube-state-metrics publishes them only with the chart's metricLabelsAllowlist."
            .to_string(),
    )
}

/// What an event panel shows when nothing happened.
pub(crate) fn quiet(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} in this time range"))
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here, but stays in the signature so every
/// dashboard is built the same way.
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
            "Whether the cluster's nodes are keeping up with its pods, on any cloud.\n\n\
             Pods waiting for a node, how full each node pool is, what the autoscaler \
             did about it, and whether the cloud could supply the node. On EKS, \
             Karpenter's own account adds rows to each tab and a tab of its own.",
        )
        .tags([tags::INFRA, tags::MZMON])
        .folder(Folder::Infra)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::autoscaling_scoped())
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
    use crate::grafana::infra_cloud::{AWS, AZURE, GCP, known_providers};
    use crate::grafana::queries::test_registry;
    use crate::grafana::test_support;

    fn built() -> Resource {
        build("mz_", test_registry()).expect("build")
    }

    fn expressions() -> Vec<String> {
        let json = serde_json::to_value(&built().spec.elements).expect("serialize");
        let mut out = Vec::new();
        for element in json.as_object().expect("elements").values() {
            for query in element["spec"]["data"]["spec"]["queries"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if let Some(expr) = query["spec"]["query"]["spec"]["expr"].as_str() {
                    out.push(expr.to_string());
                }
            }
        }
        out
    }

    #[test]
    fn it_builds() {
        let resource = built();
        assert_eq!(resource.metadata.name, NAME);
        assert_eq!(resource.spec.title, TITLE);
    }

    #[test]
    fn every_tab_is_present() {
        let json = serde_json::to_string(&built().spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }

    #[test]
    fn it_is_filed_under_infrastructure() {
        let json = serde_json::to_string(&built().metadata).expect("serialize");
        assert!(json.contains("\"infra\""), "{json}");
    }

    #[test]
    fn every_query_references_only_variables_the_dashboard_defines() {
        test_support::assert_variables_defined(&built());
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        test_support::assert_every_panel_has_no_value(&built());
    }

    #[test]
    fn multi_series_panels_are_not_shaded() {
        test_support::assert_multi_series_panels_unshaded(&built());
    }

    #[test]
    fn selecting_every_provider_still_renders_their_rows() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn provider_rows_render_only_on_their_provider() {
        let conditions: Vec<_> = test_support::variable_conditioned_rows(&built())
            .into_iter()
            .filter(|(_, _, value)| value != karpenter::DETECTED)
            .collect();
        let when: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .collect();
        assert_eq!(when.len(), 3, "one row per provider: {conditions:?}");
        for (title, _, value) in when {
            assert!(
                value == AWS || value == GCP || value == AZURE,
                "{title} is conditioned on {value}"
            );
        }
        let fallbacks: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        assert_eq!(fallbacks.len(), 1, "{fallbacks:?}");
        assert_eq!(fallbacks[0].2, known_providers());
    }

    #[test]
    fn every_kube_state_metrics_read_is_deduplicated_first() {
        // Each extra kube-state-metrics replica reports every object again, so
        // an un-deduplicated sum doubles. Every `kube_*` selector here sits
        // directly inside a `max by`, or inside `%%{nodePools}`, which is one.
        for expr in expressions() {
            let mut rest = expr.as_str();
            while let Some(pos) = rest.find("kube_") {
                let before = &rest[..pos];
                let deduplicated = before.trim_end().ends_with('(')
                    && (before.contains("max by") || before.contains("label_replace"));
                assert!(
                    deduplicated,
                    "a kube-state-metrics read is not deduplicated: {expr}"
                );
                rest = &rest[pos + 5..];
            }
        }
    }

    #[test]
    fn no_provider_counter_is_rated_over_the_dashboard_interval() {
        // See `infra-cloud`: `$__rate_interval` is far shorter than a pull.
        // Karpenter's own AWS SDK counters are scraped, not pulled, so they are
        // rated like any other.
        for expr in expressions() {
            let pulled = expr.replace("aws_sdk_go_", "");
            for family in ["stackdriver_", "aws_", "azure_"] {
                if pulled.contains(family) {
                    assert!(!expr.contains("$__rate_interval"), "{expr}");
                }
            }
        }
    }

    #[test]
    fn the_noisy_karpenter_reasons_stay_out_of_the_event_panels() {
        // Karpenter repeats `DisruptionBlocked` and `Unconsolidatable` for every
        // node holding a do-not-disrupt pod, which on a Materialize cluster
        // outnumbers every other event here by thousands. The Karpenter tab
        // breaks them down; here they would bury the rest.
        let q = Queries::new(test_registry(), &DashboardScope::default());
        for id in [
            "infra.autoscaling.events.rate_by_reason",
            "infra.autoscaling.events.stream",
        ] {
            let expr = serde_json::to_string(&q.logs(id).query_group).expect("serialize");
            assert!(
                expr.contains(r#"reason!~\"DisruptionBlocked|Unconsolidatable\""#),
                "{id} does not exclude them: {expr}"
            );
        }
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    /// Every row under one tab, as `(title, conditioned)`.
    fn rows_of_tab(title: &str) -> Vec<(String, bool)> {
        fn collect(value: &serde_json::Value, out: &mut Vec<(String, bool)>) {
            match value {
                serde_json::Value::Object(map) => {
                    if map.get("kind").and_then(|k| k.as_str()) == Some("RowsLayoutRow") {
                        let spec = &map["spec"];
                        out.push((
                            spec["title"].as_str().unwrap_or_default().to_string(),
                            spec.get("conditionalRendering").is_some(),
                        ));
                    }
                    for v in map.values() {
                        collect(v, out);
                    }
                }
                serde_json::Value::Array(items) => {
                    for v in items {
                        collect(v, out);
                    }
                }
                _ => {}
            }
        }
        let json = serde_json::to_value(&built().spec.layout).expect("serialize");
        let tab = json["spec"]["tabs"]
            .as_array()
            .expect("tabs")
            .iter()
            .find(|tab| tab["spec"]["title"] == title)
            .unwrap_or_else(|| panic!("no tab {title}"));
        let mut out = Vec::new();
        collect(tab, &mut out);
        out
    }

    #[test]
    fn karpenter_rows_render_on_karpenter_and_only_its_own_tab_has_a_fallback() {
        let conditions = test_support::variable_conditioned_rows(&built());
        let karpenter: Vec<_> = conditions
            .iter()
            .filter(|(_, _, value)| value == karpenter::DETECTED)
            .collect();
        let fallbacks: Vec<_> = karpenter
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        // The shared tabs keep answering their question without Karpenter, so a
        // missing row needs no note. The Karpenter tab has nothing else.
        assert_eq!(fallbacks.len(), 1, "{karpenter:?}");
        assert!(
            karpenter.len() > 1 + 4,
            "a row on each shared tab and the tab's own: {karpenter:?}"
        );
        // Every row on the Karpenter tab is conditioned: a bare row would show
        // empty panels on GKE and AKS beside the fallback that explains them.
        for (title, conditioned) in rows_of_tab(theme::KARPENTER.title) {
            assert!(conditioned, "{title} renders without Karpenter");
        }
    }

    #[test]
    fn karpenter_panels_sit_only_on_karpenter_rows() {
        // A panel reading Karpenter's metrics on an unconditioned row would be
        // empty on GKE and AKS with nothing to say why.
        let json = serde_json::to_value(&built().spec.layout).expect("serialize");
        let elements = serde_json::to_value(&built().spec.elements).expect("serialize");
        // A family name starts an identifier; `%%{nodePools}` reads the node
        // label `label_karpenter_sh_nodepool` on every cloud, which is not one.
        fn reads_family(queries: &str, family: &str) -> bool {
            queries.match_indices(family).any(|(i, _)| {
                i == 0 || {
                    let before = queries.as_bytes()[i - 1];
                    !before.is_ascii_alphanumeric() && before != b'_'
                }
            })
        }
        let karpenter_panels: Vec<String> = elements
            .as_object()
            .expect("elements")
            .iter()
            .filter(|(_, panel)| {
                let queries = panel["spec"]["data"].to_string();
                reads_family(&queries, "karpenter_")
                    || reads_family(&queries, "operator_")
                    || queries.contains(r#"app=\"karpenter\""#)
                    || queries.contains(r#"reportingcontroller=\"karpenter\""#)
            })
            .map(|(name, _)| name.clone())
            .collect();
        assert!(!karpenter_panels.is_empty());

        fn rows_holding(
            value: &serde_json::Value,
            conditioned: bool,
            out: &mut Vec<(String, bool)>,
        ) {
            match value {
                serde_json::Value::Object(map) => {
                    let conditioned = conditioned
                        || (map.get("kind").and_then(|k| k.as_str()) == Some("RowsLayoutRow")
                            && map["spec"].get("conditionalRendering").is_some());
                    if map.get("kind").and_then(|k| k.as_str()) == Some("ElementReference")
                        && let Some(name) = map["name"].as_str()
                    {
                        out.push((name.to_string(), conditioned));
                    }
                    for v in map.values() {
                        rows_holding(v, conditioned, out);
                    }
                }
                serde_json::Value::Array(items) => {
                    for v in items {
                        rows_holding(v, conditioned, out);
                    }
                }
                _ => {}
            }
        }
        let mut placed = Vec::new();
        rows_holding(&json, false, &mut placed);
        for name in karpenter_panels {
            let (_, conditioned) = placed
                .iter()
                .find(|(placed, _)| *placed == name)
                .unwrap_or_else(|| panic!("{name} is not placed"));
            assert!(
                conditioned,
                "{name} reads Karpenter on an unconditioned row"
            );
        }
    }

    #[test]
    fn the_detection_pattern_rejects_an_unset_variable() {
        // The condition is a substring match, so the pattern must not occur in
        // anything an unset variable can stringify to.
        for unset in ["undefined", "", "$__all", "null", "[object Object]"] {
            assert!(
                !unset.contains(karpenter::DETECTED),
                "{unset} would render the rows"
            );
        }
        let discovery = serde_json::to_string(&mzmon_lib::grafana::variable::karpenter_detected())
            .expect("serialize");
        assert!(
            discovery.contains(r#"up{app=\"karpenter\"}"#),
            "{discovery}"
        );
    }

    #[test]
    fn karpenters_generic_families_are_scoped_to_karpenter() {
        // `controller_runtime_*`, `workqueue_*`, `client_go_*` and
        // `aws_sdk_go_*` are published by every controller-runtime process in
        // the cluster, the EBS CSI driver and Load Balancer Controller among
        // them.
        let json = serde_json::to_string(&built().spec.elements).expect("serialize");
        for family in [
            "controller_runtime_",
            "workqueue_",
            "client_go_",
            "aws_sdk_go_",
        ] {
            let selectors = test_support::selectors_of(&json, family);
            assert!(!selectors.is_empty(), "{family} is not read at all");
            for selector in selectors {
                assert!(
                    selector.contains(r#"app=\"karpenter\""#),
                    "{family} is not scoped to Karpenter: {selector}"
                );
            }
        }
    }
}
