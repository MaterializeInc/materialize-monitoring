// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The infrastructure logs and events dashboard.
//!
//! The first of the `infra-*` family: scoped to the cluster rather than to a
//! Materialize environment. `env-logs` answers "what did my deployment say";
//! this one answers "what did the platform underneath it say", which is a
//! different question with a different audience and — the part that forced a
//! second dashboard rather than a wider first one — a different label set.
//!
//! # Why not just widen `env-logs`
//!
//! Two things it cannot reach, however its pickers are set.
//!
//! **The node journal.** Journal lines carry `unit` and no `namespace`, `app` or
//! `container`, because they come from the node rather than from a pod. Every
//! `env-logs` selector requires a namespace, so those lines are excluded by
//! construction. That is the Nodes tab, and it is the sharpest reason this
//! dashboard exists.
//!
//! **Sub-components.** `component` splits `loki` into eight processes and
//! `thanos` into three. `env-logs` has no picker for it and no reason to: a
//! Materialize environment has no sub-components. Adding one there would oblige
//! that dashboard to carry a control that does nothing.
//!
//! # What it does share
//!
//! The variable *names* are `env-logs`'s, and the Kubernetes-event queries are
//! `env-logs`'s. Only the namespace picker's opening selection differs — every
//! namespace here, the deployment's own there — so the events half is one set of
//! definitions serving both rather than two that drift.
//!
//! # Loki end to end
//!
//! Like `env-logs`, this defines no metrics datasource. An infrastructure
//! operator reading logs is often doing so *because* the metrics pipeline is the
//! thing that broke, and a dashboard that needed Prometheus to render its own
//! pickers would go blind at exactly that moment.

pub mod events;
pub mod logs;
pub mod nodes;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-logs";

/// Artifact filename stem, which is *not* the resource name.
///
/// **`infra-` rather than `env-`, and that decides whether it ships.**
/// `dashboards.selected` defaults to `["env-*"]`, so nothing in this family is
/// installed until a release widens that. Deliberate: these dashboards are for
/// whoever operates the cluster, which is not always whoever runs Materialize on
/// it.
pub const NAME_STEM: &str = "infra-logs";

/// Dashboard title.
pub const TITLE: &str = "Infrastructure Logs and Events";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular, and less than any other dashboard here: nothing on it
/// reads a Materialize signal at all. Every line it shows is produced by the
/// monitoring stack, the platform, or the nodes.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order.
///
/// Logs first because it is where an investigation starts, Nodes second because
/// it is where one ends up when the answer is not in a pod, and Events last as
/// the cross-cutting record of what Kubernetes decided.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::LOGS.title).rows(logs::rows(q)),
        Tab::new(theme::NODES.title).rows(nodes::rows(q)),
        Tab::new(theme::EVENTS.title).rows(events::rows(q)),
    ]
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here — no query on this dashboard is a
/// metric, let alone a SQL-derived one — but it stays in the signature so every
/// dashboard is built the same way and the renderer needs no special case.
pub fn build(sql_metric_prefix: &str, registry: &QueryRegistry) -> dashboard::Result<Resource> {
    // Subtract Materialize by default: this dashboard opens on every namespace,
    // and the deployment's own logs would otherwise drown the platform's.
    let scope = DashboardScope::for_prefix(sql_metric_prefix).exclude_materialize_variable();
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
            "Logs and Kubernetes events for the platform a Materialize deployment runs on.\n\n\
             The monitoring stack, the Kubernetes system components, and the node journal.",
        )
        .tags(["infrastructure", "monitoring", "logs"])
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::logs_infra_scoped())
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
    fn it_defines_no_metrics_datasource() {
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        assert!(names.contains(&"logsDatasource"));
        assert!(!names.contains(&"metricsDatasource"), "{names:?}");
    }

    #[test]
    fn it_offers_the_axes_the_platform_needs() {
        // The three `env-logs` has no use for, and the reason this is a separate
        // dashboard rather than a wider one.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        for axis in ["logComponentList", "logContainerList", "logUnitList"] {
            assert!(names.contains(&axis), "missing {axis}: {names:?}");
        }
    }

    #[test]
    fn the_namespace_picker_opens_on_everything() {
        // `env-logs` opens on the Materialize namespaces; this one must not, or
        // the platform's own events and logs -- the majority of both -- start
        // hidden on a dashboard whose whole subject is the platform.
        let resource = built();
        let namespaces = resource
            .spec
            .variables
            .iter()
            .find(|v| variable::name_of(v) == "logNamespaceList")
            .expect("a namespace picker");
        let json = serde_json::to_string(namespaces).expect("serialize");
        assert!(
            json.contains(r#"["·+"]"#.replace('·', ".").as_str()),
            "{json}"
        );
        assert!(
            !json.contains("materialize"),
            "opens on a Materialize scope: {json}"
        );
    }

    #[test]
    fn materialize_is_excluded_by_default() {
        // The picker opening on everything (above) is only half the scope. The
        // other half subtracts the deployment, because `environmentd` alone
        // out-logs every platform component combined and would otherwise be all
        // a volume panel shows.
        let resource = built();
        let switch = resource
            .spec
            .variables
            .iter()
            .find(|v| variable::name_of(v) == "excludeMaterialize")
            .expect("an exclusion switch");
        let json = serde_json::to_string(switch).expect("serialize");
        // Defaulting to true is the request: `current` has to be the *enabled*
        // value, not the disabled one.
        assert!(json.contains(r#""current":".*materialize"#), "{json}");
        assert!(json.contains(r#""disabledValue":"a^""#), "{json}");
    }

    #[test]
    fn the_exclusion_reaches_every_namespace_scoped_query() {
        // A switch wired into some of the selectors and not others is worse than
        // no switch: the volume panels would disagree with the streams below
        // them and neither would look wrong.
        let resource = built();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = serde_json::to_string(&query.spec.query.spec).expect("serialize");
                if !expr.contains("$logNamespaceList") {
                    // Node journals carry no namespace, so nothing to subtract.
                    continue;
                }
                assert!(
                    expr.contains(r#"namespace!~\"$excludeMaterialize"#),
                    "{name} scopes by namespace but ignores the switch: {expr}"
                );
            }
        }
    }

    #[test]
    fn every_query_is_loki() {
        let resource = built();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                assert_eq!(query.spec.query.group, "loki", "{name}");
            }
        }
    }
}
