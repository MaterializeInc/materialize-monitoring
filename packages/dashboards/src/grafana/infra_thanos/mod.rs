// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Meta monitoring for the metrics store.
//!
//! The third dashboard whose subject is the monitoring stack, beside `infra-loki`
//! and `infra-alloy`, and it inherits their two rules: scrape health comes
//! before any measurement, and every panel writes its own empty-state text.
//!
//! # The instrument answers for itself
//!
//! Thanos Query answers every panel on this dashboard, including the ones about
//! Thanos Query. A query outage therefore empties the whole dashboard rather
//! than its Reads tab, which is why the Logs tab exists here at all: Thanos's
//! logs are in Loki, and stay readable when nothing else on this dashboard is.
//!
//! # Alerts lead the Overview
//!
//! The Thanos subchart installs the upstream mixin's alerting rules, and the
//! Thanos ruler evaluates them. Overview shows what is firing before it shows
//! any measurement, so a reader who arrived from a notification lands on the
//! alert, and one who arrived cold learns about it. The panels select
//! `ALERTS{alertname=~"Thanos.*"}`, because the mixin's rules carry nothing but
//! `severity`; their own selectors are cluster-wide, so the namespace picker
//! cannot narrow them either, and the tests exempt those panels by name.
//!
//! The firing count reads zero only while a ruler has evaluated something in
//! the last five minutes. A ruler that has stopped evaluating fires nothing,
//! and "no alerts" from a dead alert path is the reading this dashboard most
//! needs to avoid.
//!
//! # Why not the upstream mixin dashboards
//!
//! The Thanos repository ships dashboards generated from the same mixin as the
//! alerts. Unlike Loki's, they read raw metrics rather than recording rules, so
//! they would run. They were read and not ported:
//!
//! * They scope by `job=~".*thanos-<component>.*"` with no namespace, so two
//!   releases on one cluster read as one store.
//! * Several panels read metric names this Thanos version does not publish:
//!   `thanos_alert_sender_errors_total`, `thanos_compact_downsample_failed_total`,
//!   `thanos_compact_blocks_marked_for_deletion_total`,
//!   `thanos_query_store_apis_dns_*` and
//!   `thanos_bucket_store_series_gate_duration_seconds`. Those panels render
//!   empty, which on a meta-monitoring dashboard reads as healthy.
//! * The Ruler dashboard's "Push Rate" panel reads the queue's *dropped*
//!   counter.
//! * They are one dashboard per component, which answers "how is the store
//!   gateway" well and "is the metrics store healthy" not at all.
//!
//! What they do well is kept: the error-ratio and p99 shape of each process's
//! request panels, and the mixin's gRPC error-code set for store requests.

pub mod logs;
pub mod overview;
pub mod reads;
pub mod resources;
pub mod ruler;
pub mod storage;
pub mod theme;
pub mod writes;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::panel::NoValue;
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-thanos";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "infra-thanos";

/// Dashboard title.
pub const TITLE: &str = "Thanos Meta Monitoring";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular: nothing on it reads a Materialize signal.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// What a metrics panel shows when nothing about Thanos is arriving.
///
/// The one empty state every metrics panel here can reach. Thanos Query answers
/// every panel, so its absence usually means Query is down, and the logs are
/// where to look.
pub(crate) fn not_collected() -> NoValue {
    NoValue::Custom(
        "No data from Thanos. Check Collection Health on the Overview tab, or the Logs tab if that is empty too."
            .to_string(),
    )
}

/// The tabs, in order.
///
/// Overview first because it is the verdict. Then the write path and the read
/// path, the order a sample travels. The ruler after both, since it is a client
/// of the read path and a writer through the gateway, and its failures surface
/// first as alerts that do not fire. Storage after that, because the compactor
/// and the bucket are downstream of everything and fail slowly. Resources is
/// usually the explanation for what the earlier tabs showed. Logs last, where an
/// investigation ends up.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::WRITES.title).rows(writes::rows(q)),
        Tab::new(theme::READS.title).rows(reads::rows(q)),
        Tab::new(theme::RULER.title).rows(ruler::rows(q)),
        Tab::new(theme::STORAGE.title).rows(storage::rows(q)),
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
            "Is the metrics store healthy, and if not, which part of it broke.\n\n\
             The monitoring stack watching itself: remote writes into the receivers, \
             queries through Thanos Query, the ruler, the compactor, and object storage, \
             and any Thanos alert firing now. Thanos answers every panel here, so a \
             dashboard that is empty throughout most likely means Thanos Query is \
             down; the Logs tab reads Loki and still works.",
        )
        .tags([tags::INFRA, tags::MZMON, tags::content::META])
        .folder(Folder::MetaO11y)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::thanos_scoped())
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

    /// Panels whose series no namespace picker can narrow: the alert panels, which
    /// read `ALERTS` for rules whose selectors are cluster-wide.
    const UNSCOPED: &[&str] = &[
        "overview-alerts-firing",
        "overview-alerts",
        "overview-alert-history",
    ];

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
            "metricsDatasource",
            "logsDatasource",
            "thanosNamespace",
            "thanosComponent",
            "logLevelList",
            "logSearch",
        ] {
            assert!(names.contains(&required), "missing {required}: {names:?}");
        }
    }

    #[test]
    fn every_query_is_anchored_to_one_thanos() {
        // `prometheus_tsdb_*`, `prometheus_remote_storage_*` and the gRPC and
        // HTTP families come from other processes in the stack too, and two
        // releases on one cluster would otherwise read as one store. The
        // cAdvisor and kube-state-metrics series carry no `app`, so they reach
        // Thanos pods through the namespace and container instead.
        let resource = built();
        for (name, _, expr) in expressions(&resource) {
            if UNSCOPED.contains(&name.as_str()) {
                continue;
            }
            assert!(expr.contains("$thanosNamespace"), "{name}: {expr}");
            assert!(
                expr.contains(r#"app="thanos""#),
                "{name} is not anchored to Thanos: {expr}"
            );
        }
    }

    #[test]
    fn unscoped_panels_are_exactly_the_declared_ones() {
        // The exemption above is a list, so it has to stay true in both
        // directions: each exempt panel exists, and reads only Thanos alerts.
        let resource = built();
        let exprs = expressions(&resource);
        for exempt in UNSCOPED {
            let found: Vec<_> = exprs.iter().filter(|(n, _, _)| n == exempt).collect();
            assert!(!found.is_empty(), "{exempt} is not on the dashboard");
            for (_, _, expr) in found {
                assert!(
                    expr.contains(r#"ALERTS{alertname=~"Thanos.*""#),
                    "{exempt}: {expr}"
                );
            }
        }
    }

    #[test]
    fn every_metric_query_names_its_process() {
        // A panel either spans processes and honours the component picker, or
        // is about one process and names its container. Neither is a panel
        // that silently mixes processes the reader did not choose.
        let resource = built();
        for (name, group, expr) in expressions(&resource) {
            if group == "loki" || UNSCOPED.contains(&name.as_str()) {
                continue;
            }
            assert!(
                expr.contains("$thanosComponent") || expr.contains(r#"container=""#),
                "{name}: {expr}"
            );
        }
    }

    #[test]
    fn it_is_filed_with_the_other_meta_monitoring_dashboards() {
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
        assert!(json.contains("meta-o11y"), "{json}");
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        // "Nothing to report" and "nothing was collected" have to read
        // differently, because both are reachable here and they call for
        // opposite responses.
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
