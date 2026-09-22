// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Meta monitoring for the log store.
//!
//! Every other dashboard here *uses* the monitoring stack. This one watches half
//! of it, and that inversion is what makes it different rather than just another
//! `infra-*` view.
//!
//! # The instrument is the subject
//!
//! On any other dashboard a blank panel means "nothing happened". Here it may
//! mean "the thing that would have told you is the thing that is down", and the
//! two readings call for opposite responses. Three consequences run through the
//! whole dashboard:
//!
//! * **The Overview tab leads with scrape health**, before any measurement. A
//!   Loki component at `up == 0` makes every panel about it empty, and an empty
//!   panel is indistinguishable from a healthy quiet one.
//! * **The canary outranks everything else.** It is the only end-to-end signal:
//!   it writes a line and reads it back, so it is wrong only when the store
//!   really is. Every other panel measures one stage and infers the rest.
//! * **Empty text is written per panel** rather than left to the default,
//!   because "nothing to report" and "nothing was collected" have to read
//!   differently on a dashboard whose subject can take its own instrumentation
//!   with it.
//!
//! # The fault this was built around
//!
//! The Loki subchart emits a single ServiceMonitor with one `scheme` shared by
//! every target it selects. `profiles/mtls` sets that to `https`, which is
//! correct for the Loki processes and wrong for the three targets that never
//! serve TLS — the canary's own `/metrics`, and the two memcached exporters. All
//! three failed their scrape, so `up` read 0 and no series arrived at all.
//!
//! The end-to-end check was therefore silent on exactly the installs that had
//! configured the most carefully, and nothing said so. The chart now splits those
//! three onto a plaintext monitor of its own
//! (`templates/scrapers/monitor-loki-plaintext.yaml`), and the Scrape Health
//! panel is what says the split is working.
//!
//! # Why not the upstream mixin dashboards
//!
//! The Loki chart ships `loki-reads`, `loki-writes` and `loki-operational`, and
//! they are good dashboards. They are also unusable here without new
//! infrastructure: they read `cluster_job_route:loki_request_duration_seconds_*`
//! recording rules, and this stack evaluates no PromQL rules at all — there is no
//! Prometheus and no Thanos Ruler, and Loki's own ruler evaluates LogQL. They
//! further assume a `cluster` variable, where the `cluster` label on these
//! metrics is Loki's own ring name (`default`) rather than a Kubernetes cluster,
//! so the scoping would be silently wrong rather than absent.
//!
//! Bloom panels are excluded for a simpler reason: the feature is experimental
//! and this deployment does not run it.
//!
//! # Both datasources
//!
//! Unlike the other `infra-*` dashboards, this defines a metrics datasource *and*
//! a logs one. The metrics say whether Loki is healthy; its own logs say why.
//! Splitting them across two dashboards would mean reading a symptom here and
//! going elsewhere for the cause, which is the wrong trade for the one subject
//! where the second dashboard might not load.

pub mod logs;
pub mod overview;
pub mod reads;
pub mod storage;
pub mod theme;
pub mod writes;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-loki";

/// Artifact filename stem, which is *not* the resource name.
///
/// `infra-` puts it in the family for whoever operates the cluster rather than
/// whoever runs Materialize on it, and `dashboards.selected` ships that pattern —
/// so this installs by default alongside the rest.
pub const NAME_STEM: &str = "infra-loki";

/// Dashboard title.
pub const TITLE: &str = "Loki Meta Monitoring";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular, and for the strongest reason of any dashboard here:
/// nothing on it reads a Materialize signal at all. Every series it draws is
/// produced by the monitoring stack watching itself.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The tabs, in order.
///
/// Overview first because it is the verdict. Then the write path and the read
/// path, which is the order a line travels and the order in which their failures
/// nest. Storage after both, because it is downstream of each and its failures
/// surface *as* a write or read problem before they surface as themselves. Logs
/// last: it is where an investigation ends up once the numbers have said where to
/// look.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::WRITES.title).rows(writes::rows(q)),
        Tab::new(theme::READS.title).rows(reads::rows(q)),
        Tab::new(theme::STORAGE.title).rows(storage::rows(q)),
        Tab::new(theme::LOGS.title).rows(logs::rows(q)),
    ]
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
///
/// `sql_metric_prefix` reaches nothing here — no query on this dashboard is a
/// Materialize metric, let alone a SQL-derived one — but it stays in the
/// signature so every dashboard is built the same way and the renderer needs no
/// special case.
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
            "Is the log store healthy — and if not, which half of it broke.\n\n\
             The monitoring stack watching itself: ingest, queries, object storage, \
             and Loki's own logs. Start at Scrape Health and the canary; an empty \
             panel here can mean the instrument is down rather than that nothing \
             happened.",
        )
        .tags([
            tags::INFRA,
            tags::MZMON,
            tags::content::META,
            tags::content::LOGS,
        ])
        .folder(Folder::MetaO11y)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::loki_scoped())
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
    fn it_defines_both_datasources() {
        // The one `infra-*` dashboard that does. Losing either silently empties
        // half of it: without the metrics datasource the component and namespace
        // pickers stop resolving, and every log query loses its scope with them.
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
        // The failure this guards is invisible: an undefined Grafana variable
        // interpolates to nothing, the selector matches no series, and the panel
        // renders empty and correct-looking.
        let resource = built();
        let names: Vec<&str> = resource
            .spec
            .variables
            .iter()
            .map(variable::name_of)
            .collect();
        for required in [
            "lokiNamespace",
            "lokiComponent",
            "logLevelList",
            "logSearch",
        ] {
            assert!(names.contains(&required), "missing {required}: {names:?}");
        }
    }

    #[test]
    fn every_metric_query_is_scoped_to_one_loki() {
        // Two `materialize-monitoring` releases on one cluster would otherwise
        // read as a single store, and every count on the dashboard would be the
        // sum of two unrelated ones.
        let resource = built();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                if query.spec.query.group == "loki" {
                    // Log queries carry it too, asserted in `logs.rs` against the
                    // stream selector rather than against the string.
                    continue;
                }
                let expr = query.spec.query.spec.as_ref().expect("spec")["expr"]
                    .as_str()
                    .expect("expr");
                assert!(expr.contains("$lokiNamespace"), "{name}: {expr}");
            }
        }
    }

    #[test]
    fn it_is_the_first_occupant_of_the_meta_observability_folder() {
        // The folder has existed in `values.yaml` since folders did, unused.
        // Stated as a test so that moving this dashboard elsewhere is a decision
        // rather than an accident that leaves the folder empty again.
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
        assert!(json.contains("meta-o11y"), "{json}");
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        // The rule this dashboard turns on: "nothing to report" and "nothing was
        // collected" have to read differently, because both are reachable here
        // and they call for opposite responses.
        let resource = built();
        for (name, element) in &resource.spec.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
            else {
                continue;
            };
            // Text panels carry no query and no field config to put it on.
            if panel.spec.data.spec.queries.is_empty() {
                continue;
            }
            let json = serde_json::to_string(&panel.spec.viz_config).expect("serialize");
            assert!(json.contains("noValue"), "{name} has no empty-state text");
        }
    }
}
