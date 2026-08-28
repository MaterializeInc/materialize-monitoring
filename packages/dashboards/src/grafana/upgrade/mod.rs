// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The upgrade dashboard.
//!
//! What happened during a Materialize upgrade, and why it did or did not finish.
//! `env-top` answers "is this environment healthy"; this one answers "is this
//! *change* going through", which is a different question with different
//! evidence. A rollout is a sequence of discrete transitions, and the record of
//! it lives in Kubernetes events rather than in any gauge.
//!
//! # Two datasources
//!
//! This is the first dashboard here to read Loki, and the reason is that events
//! are logs. It defines both datasource variables — `$metricsDatasource` and
//! `$logsDatasource` — because a Grafana `DatasourceVariable` resolves against one
//! plugin id, so a dashboard mixing engines needs one of each.
//!
//! # Two namespaces
//!
//! The operator and the environments it reconciles live in different namespaces,
//! and an upgrade is only legible with both in view: the operator's namespace
//! carries the decision to roll, the environment's carries the pods that have to
//! come up for it to succeed.
//!
//! They are scoped differently on purpose. `$operatorNamespace` is a control an
//! operator sets, because the operator is a cluster-wide singleton that no
//! environment selection narrows. The environment namespace is derived from the
//! environment picker exactly as it is on `env-top`, and stays hidden for the same
//! reason: exposing it invites a selection inconsistent with the environment it is
//! supposed to belong to.
//!
//! # Tabs
//!
//! **Events** is the rollout's account of itself: the phases it moved through and
//! what it said when one failed. **Reconciliation** is the machine underneath —
//! the operator's control loop as counters and histograms.
//!
//! They are separate tabs rather than one because they answer questions at
//! different altitudes, and because they read from different datasources. Events
//! tells you *what happened*; Reconciliation tells you whether the thing that
//! makes it happen is working. An operator usually arrives at the first and
//! descends to the second, which is the order they are in.
//!
//! The two are also scoped differently, which is the subtler reason not to merge
//! them. Events are per-resource and therefore per-environment; the operator's
//! metrics carry no organization label at all, because one operator reconciles
//! every environment in the cluster. A single tab would have to hold both
//! meanings of "what am I looking at" at once.

pub mod events;
pub mod reconciliation;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-upgrade";

/// Artifact filename stem, which is *not* the resource name. See
/// [`crate::grafana::env_top::NAME_STEM`] for why the two are separate.
pub const NAME_STEM: &str = "upgrade";

/// Dashboard title.
pub const TITLE: &str = "Materialize Upgrade";

/// Minimum Materialize version this dashboard's events require.
///
/// The operator publishes lifecycle transitions and reconciliation failures as
/// Kubernetes events, and exports the reconciliation counters and histograms,
/// from this version on. Against an older operator the Events tab's Kubernetes
/// Activity row still works — those events come from the kubelet and the
/// scheduler, which have always emitted them — while the Rollout and Operator
/// Health rows are empty, which is why they carry a "no rollout activity" message
/// rather than a filter-mismatch one. The Reconciliation tab is empty entirely,
/// apart from `Reconciling Replicas` and `Environments Needing Update`, whose two
/// gauges predate the rest.
pub const MIN_MZ_VERSION: &str = "v26.40.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.40.0";

/// The tabs, in order.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::EVENTS.title).rows(events::rows(q)),
        Tab::new(theme::RECONCILIATION.title).rows(reconciliation::rows(q)),
    ]
}

/// The export target this crate produces. See `env_top` for the note on why this
/// is a constant.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// The scope differs from `env-top`'s in one respect: [`DashboardScope::operator_variable`]
/// points the operator-namespace parameter at `$operatorNamespace` rather than
/// pinning `materialize`, which is sound only because [`variable::operator_scoped`]
/// defines that variable. The two have to move together.
pub fn build(
    cloud: crate::grafana::Cloud,
    sql_metric_prefix: &str,
    registry: &QueryRegistry,
) -> dashboard::Result<Resource> {
    let scope = DashboardScope::for_prefix(sql_metric_prefix).operator_variable();
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
            "What happened during a Materialize upgrade.\n\nThe operator's own account of a \
             rollout, alongside\nwhat the cluster did underneath it.",
        )
        .tags(["materialize", "monitoring", "upgrade"])
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::operator_scoped(sql_metric_prefix))
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
        .metadata_annotation(
            "monitoring.materialize.cloud/target-cloud",
            cloud.to_string(),
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

    build(options.cloud, &options.sql_metric_prefix, registry).map_err(|source| Error::Build {
        name: NAME_STEM,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_registry;

    fn built() -> Resource {
        build(crate::grafana::Cloud::Generic, "mz_", test_registry()).expect("build")
    }

    #[test]
    fn it_builds_for_both_deployments() {
        for prefix in ["mz_", "v2_mz_"] {
            let resource =
                build(crate::grafana::Cloud::Generic, prefix, test_registry()).expect("build");
            assert_eq!(resource.metadata.name, NAME);
            assert_eq!(resource.spec.title, TITLE);
        }
    }

    #[test]
    fn it_defines_both_datasource_variables() {
        // A Loki panel on a dashboard defining only `$metricsDatasource` resolves
        // its datasource to nothing and renders empty.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        assert!(names.contains(&"metricsDatasource"));
        assert!(names.contains(&"logsDatasource"));
    }

    #[test]
    fn the_operator_namespace_variable_and_the_scope_agree() {
        // The scope renders `$operatorNamespace` into every operator query; if the
        // variable set stopped defining it, those queries would silently match
        // nothing. This is the pairing that keeps them together.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        assert!(names.contains(&"operatorNamespace"));

        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        // Either spelling: a query naming the variable outright writes
        // `$operatorNamespace`, and one splicing it into a wider regex writes
        // `${operatorNamespace:regex}`.
        assert!(
            json.contains("$operatorNamespace") || json.contains("${operatorNamespace:regex}"),
            "no query references the operator namespace variable"
        );
        // The pinned default must not survive anywhere: a query still carrying it
        // would look right and ignore the control.
        assert!(
            !json.contains(r#"namespace=~\"materialize\""#),
            "a query still pins the operator namespace instead of using the variable"
        );
    }

    #[test]
    fn no_description_points_somewhere_that_does_not_exist() {
        // Same rule as `env_top`: a description navigating by `_Where -> What_`
        // has to name a tab or a row that is really there.
        let resource = built();
        let mut destinations: Vec<String> =
            theme::THEMED.iter().map(|t| t.title.to_string()).collect();
        destinations.extend(row_titles(&resource.spec));

        let mut broken = Vec::new();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for destination in arrow_references(&panel.spec.description) {
                if !destinations.contains(&destination) {
                    broken.push(format!(
                        "{name}: _{destination} -> …_ is neither a tab nor a row"
                    ));
                }
            }
        }
        broken.sort();
        assert!(
            broken.is_empty(),
            "broken references:\n  {}",
            broken.join("\n  ")
        );
    }

    fn row_titles(spec: &mzmon_lib::grafana::generated::dashboardv2::Dashboard) -> Vec<String> {
        let json = serde_json::to_value(&spec.layout).unwrap_or(serde_json::Value::Null);
        let mut out = Vec::new();
        collect_row_titles(&json, &mut out);
        out
    }

    fn collect_row_titles(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|k| k.as_str()) == Some("RowsLayoutRow")
                    && let Some(title) = map["spec"].get("title").and_then(|t| t.as_str())
                {
                    out.push(title.to_string());
                }
                for v in map.values() {
                    collect_row_titles(v, out);
                }
            }
            serde_json::Value::Array(items) => {
                for v in items {
                    collect_row_titles(v, out);
                }
            }
            _ => {}
        }
    }

    /// The left-hand side of every `_Where -> What_` italic reference.
    fn arrow_references(description: &str) -> Vec<String> {
        let prose: String = description
            .split('`')
            .step_by(2)
            .collect::<Vec<_>>()
            .join(" ");

        let mut out = Vec::new();
        let mut rest = prose.as_str();
        while let Some(start) = rest.find('_') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('_') else { break };
            let italic = &after[..end];
            if let Some((where_, _what)) = italic.split_once("->") {
                out.push(where_.trim().to_string());
            }
            rest = &after[end + 1..];
        }
        out
    }
}
