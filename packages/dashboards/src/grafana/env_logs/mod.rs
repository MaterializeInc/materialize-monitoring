// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The logs and events dashboard.
//!
//! A general-purpose reader for what the deployment *said*, as opposed to what it
//! measured. `env-top` and `env-upgrade` answer questions with metrics and reach
//! for logs only where a metric cannot say it; this one is the other way round.
//!
//! # Loki end to end
//!
//! The only datasource is `$logsDatasource`, and the scope — namespace, app,
//! level — is discovered from Loki's own label values rather than from the
//! metrics pipeline. That is the dashboard's defining constraint rather than a
//! convenience: reading logs is frequently how an operator works out why the
//! metrics pipeline is broken, and one that derived its scope from Prometheus
//! would go blind exactly when it is most needed. Nothing here defines a metrics
//! datasource variable.
//!
//! # Materialize-first, not Materialize-only
//!
//! The namespace picker discovers every namespace and merely *opens* on the
//! deployment's own, so the monitoring stack, `kube-system` and the platform
//! underneath are one selection away. A default rather than a hard scope, because
//! the monitoring stack's own logs are what you need when telemetry itself is
//! failing — and because the narrow value is a naming convention rather than a
//! derived fact, so being wrong about it has to be recoverable.
//!
//! `infra-logs` is the mirror image: it opens on everything and subtracts these
//! same namespaces by default.
//!
//! # Tabs
//!
//! **Logs** is the workloads' own account of themselves. **Events** is what
//! Kubernetes said about them. They are separate tabs because they are separate
//! sources with different shapes, not two views of one thing.

pub mod events;
pub mod logs;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-env-logs";

/// Artifact filename stem, which is *not* the resource name. See
/// [`crate::grafana::env_top::NAME_STEM`] for why the two are separate.
pub const NAME_STEM: &str = "env-logs";

/// Dashboard title.
pub const TITLE: &str = "Materialize Logs and Events";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular. Every signal here is produced by the monitoring stack —
/// the agent tailing container logs, the gateway reading Kubernetes events — and
/// none of it depends on what Materialize itself exports. The floor is therefore
/// the same one the scrapers already state.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::LOGS.title).rows(logs::rows(q)),
        Tab::new(theme::EVENTS.title).rows(events::rows(q)),
    ]
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here — no query on this dashboard is
/// SQL-derived, or a metric at all — but it stays in the signature so every
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
            "Logs and Kubernetes events for a Materialize deployment.\n\nWhat the workloads said, \
             as opposed to what they measured.",
        )
        .tags(["materialize", "monitoring", "logs"])
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::logs_scoped())
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
        // The dashboard's defining constraint: it must keep working when the
        // metrics pipeline is the thing that is broken.
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
    fn every_query_is_loki_and_none_reads_a_metric_variable() {
        let resource = built();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                assert_eq!(query.spec.query.group, "loki", "{name}");
                let expr = query.spec.query.spec.as_ref().expect("spec")["expr"]
                    .as_str()
                    .expect("expr");
                for metric_scoped in ["$mzNamespaceList", "$mzClusterList", "$environmentNameList"]
                {
                    assert!(
                        !expr.contains(metric_scoped),
                        "{name} reads the metric-side {metric_scoped}: {expr}"
                    );
                }
            }
        }
    }
}
