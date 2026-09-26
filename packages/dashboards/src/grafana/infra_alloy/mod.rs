// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Meta monitoring for the collectors.
//!
//! The second meta-monitoring dashboard, after `infra-loki`, and the one whose
//! subject is most entangled with its own instrument.
//!
//! # Everything here travels through the gateway
//!
//! Alloy runs in two roles. The agent is a DaemonSet that reads each node's
//! container logs and journal. The gateway receives those logs, collects
//! Kubernetes events, scrapes every metrics target in the cluster, and delivers
//! all of it to Thanos and Loki. That includes Alloy's own metrics, which the
//! gateway scrapes, and Alloy's own logs, which reach Loki through it.
//!
//! So a gateway outage empties this dashboard entirely, and every other
//! dashboard with it. Nothing can report that from inside the stack: no `up`
//! series reads 0, because the component that would write it is the one that is
//! down. The consequences are the ones `infra-loki` draws, applied harder:
//!
//! * **Overview leads with whether the collectors are being heard from**, and
//!   every panel writes its own empty-state text. A test enforces the second.
//! * **Counts that should be zero are built to distinguish zero from absent.**
//!   The registry's `sum(A) or 0 * sum(B)` form, described at the top of
//!   `infra-alloy.yaml`, reads zero only when the collectors reported.
//! * **The Logs tab says where to go when it is empty.** The agents' own logs
//!   are read by the gateway over the Kubernetes API rather than by an agent,
//!   so an agent problem does not hide the agent's account of it. A gateway
//!   problem hides both, and `kubectl logs` is then the tool.
//!
//! # Why not the dashboards already in circulation
//!
//! The Alloy mixin's dashboards, and the internal copies made from them, were
//! read panel by panel before this was built. Their subject matter is right and
//! is covered here. Several of their queries are not:
//!
//! * **Success rates built from `_bucket` series.** Summing a histogram's
//!   buckets counts each request once per bucket. The status-code panels here
//!   read `_count`.
//! * **Scrape health filtered by the collector's own `job`.** `up` for a
//!   scraped target carries the target's `job`, so that filter selects the
//!   collectors' own scrape and nothing they scrape.
//! * **Memory as resident set size.** RSS counts reclaimable file pages. On a
//!   reference install an agent read 285 MB of RSS against a 300 MiB limit
//!   while its working set, which the limit is enforced against, was 92 MB.
//! * **Heap in use against nothing.** The chart sets `GOMEMLIMIT` so that the
//!   runtime slows before the kernel kills it. The number that matters is Go's
//!   memory against that limit, and heap alone understates it: the same
//!   install's gateways read 62–71% by heap and 93% by the limit's own measure.
//!
//! # Scoping
//!
//! Three pickers, and all three reach both engines. `app`, `namespace` and
//! `pod` hold the same values on a metric and on a log line, because the metrics
//! and logs pipelines both take them from the same pod metadata. The role
//! picker's "All" is the literal `alloy-agent|alloy-gateway`, which keeps every
//! query to this chart's collectors; see
//! [`variable::alloy_roles`](mzmon_lib::grafana::variable::alloy_roles).

pub mod components;
pub mod log_pipeline;
pub mod logs;
pub mod metric_pipeline;
pub mod overview;
pub mod resources;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-alloy";

/// Artifact filename stem, which is *not* the resource name.
///
/// `infra-` puts it in the family `dashboards.selected` ships by default.
pub const NAME_STEM: &str = "infra-alloy";

/// Dashboard title.
pub const TITLE: &str = "Alloy Meta Monitoring";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular: nothing on it reads a Materialize signal. Every series is
/// the monitoring stack describing itself.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order.
///
/// Overview first because it is the verdict. The two pipelines next, logs before
/// metrics because the log path has more hops and so more places to break.
/// Components after both, since an unhealthy component surfaces first as a
/// pipeline stage that stopped moving. Resources after that, because resource
/// pressure is usually the explanation for what the earlier tabs showed rather
/// than the first thing seen. Logs last, where an investigation ends up.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::LOG_PIPELINE.title).rows(log_pipeline::rows(q)),
        Tab::new(theme::METRIC_PIPELINE.title).rows(metric_pipeline::rows(q)),
        Tab::new(theme::COMPONENTS.title).rows(components::rows(q)),
        Tab::new(theme::RESOURCES.title).rows(resources::rows(q)),
        Tab::new(theme::LOGS.title).rows(logs::rows(q)),
    ]
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here, as on `infra-loki`, and stays in
/// the signature so every dashboard is built the same way.
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
            "Is telemetry collection healthy, and if not, which stage of it broke.\n\n\
             The collectors watching themselves: the per-node agents, the gateway, \
             both pipelines, and Alloy's own logs. Everything here reaches the \
             dashboard through the gateway, so a dashboard that is empty throughout \
             most likely means the gateway is down.",
        )
        .tags([tags::INFRA, tags::MZMON, tags::content::META])
        .folder(Folder::MetaO11y)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::alloy_scoped())
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
    use mzmon_lib::grafana::generated::dashboardv2;

    fn built() -> Resource {
        build("mz_", test_registry()).expect("build")
    }

    /// Every panel's rendered expressions, keyed by element name, with the
    /// plugin each query targets.
    fn expressions(resource: &Resource) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for (name, element) in &resource.spec.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = query.spec.query.spec.as_ref().expect("spec")["expr"]
                    .as_str()
                    .expect("expr")
                    .to_string();
                out.push((name.clone(), query.spec.query.group.clone(), expr));
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
    fn it_defines_both_datasources() {
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
    fn it_defines_every_variable_its_queries_reference() {
        // An undefined Grafana variable interpolates to nothing, the selector
        // matches no series, and the panel renders empty and correct-looking.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        for required in [
            "alloyNamespace",
            "alloyRole",
            "alloyPod",
            "logLevelList",
            "logSearch",
        ] {
            assert!(names.contains(&required), "missing {required}: {names:?}");
        }
    }

    #[test]
    fn every_collector_query_is_anchored_to_this_charts_collectors() {
        // `thanos-ruler` publishes `prometheus_remote_storage_*` too. A query
        // that lost its anchor would add the ruler's queue to the gateway's and
        // read as a gateway problem.
        let resource = built();
        for (name, _, expr) in expressions(&resource) {
            if metric_pipeline::TARGET_WIDE.contains(&name.as_str()) {
                continue;
            }
            assert!(
                expr.contains("$alloyRole") || expr.contains(r#"app="alloy-agent""#),
                "{name}: {expr}"
            );
            assert!(expr.contains("$alloyNamespace"), "{name}: {expr}");
        }
    }

    #[test]
    fn target_wide_panels_are_exactly_the_declared_ones() {
        // The exemption above is a list, so it has to stay true in both
        // directions: each exempt panel exists, and none of them narrows by a
        // collector picker it cannot honour.
        let resource = built();
        let exprs = expressions(&resource);
        for exempt in metric_pipeline::TARGET_WIDE {
            let found: Vec<_> = exprs.iter().filter(|(n, _, _)| n == exempt).collect();
            assert!(!found.is_empty(), "{exempt} is not on the dashboard");
            for (_, _, expr) in found {
                assert!(!expr.contains("$alloy"), "{exempt}: {expr}");
            }
        }
    }

    #[test]
    fn every_log_query_filters_the_pod_as_metadata() {
        // `pod` is structured metadata on these streams, not a stream label. In
        // the selector it would match no stream at all.
        let resource = built();
        for (name, group, expr) in expressions(&resource) {
            if group != "loki" {
                continue;
            }
            assert!(expr.contains(r#"| pod=~"$alloyPod""#), "{name}: {expr}");
            let selector = &expr[..expr.find('}').expect("a stream selector")];
            assert!(!selector.contains("pod"), "{name}: {expr}");
        }
    }

    #[test]
    fn the_role_picker_is_the_anchor() {
        let resource = built();
        let role = resource
            .spec
            .variables
            .iter()
            .find(|v| variable::name_of(v) == "alloyRole")
            .expect("alloyRole");
        let json = serde_json::to_string(role).expect("serialize");
        assert!(
            json.contains(r#""allValue":"alloy-agent|alloy-gateway""#),
            "{json}"
        );
    }

    #[test]
    fn it_is_filed_under_meta_observability() {
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
        assert!(json.contains("meta-o11y"), "{json}");
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        // "Nothing to report" and "nothing was collected" have to read
        // differently, and here the second is always reachable.
        let resource = built();
        for (name, element) in &resource.spec.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            if panel.spec.data.spec.queries.is_empty() {
                continue;
            }
            let json = serde_json::to_string(&panel.spec.viz_config).expect("serialize");
            assert!(json.contains("noValue"), "{name} has no empty-state text");
        }
    }
}
