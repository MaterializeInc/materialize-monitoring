// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The object storage dashboard: persist, as Materialize experiences it.
//!
//! Persist stores every durable collection as data files in object storage.
//! Writers upload new files, compaction merges them and deletes what it
//! replaced, and readers fetch them through an in-memory cache to hydrate and to
//! serve queries. This dashboard reads persist's own measurement of all three,
//! which is the only view of the store that is identical on S3, GCS, Azure Blob
//! and an on-premise S3-compatible store, and needs no cloud credentials.
//!
//! # What it cannot say
//!
//! Why the store is slow, and what the bucket holds that Materialize does not
//! know about. The first is the provider's view and the second the provider's
//! accounting, both on `infra-cloud`. The Storage tab here is the other half of
//! that comparison: what Materialize's current state refers to, and what it is
//! keeping only for a reader, which the bucket cannot tell apart.
//!
//! The exception is an on-premise store, which has no provider to ask and can
//! fill up or lose a disk. Object Store Internals carries its own account, for
//! Ceph run by Rook, on rows that render only where Ceph is scraped — see
//! [`object_store`].
//!
//! # Scope
//!
//! The environment picker and nothing narrower, for the reason
//! [`variable::dependency_scoped`] gives.
//!
//! # Two clients beneath one name
//!
//! Persist reaches S3, GCS and S3-compatible stores through one S3 client, and
//! Azure Blob through Azure's own. Everything under `mz_persist_external_*` is
//! above that split and reads the same everywhere; `mz_persist_s3_*` is below it,
//! and is registered on every install but reads zero on Azure. The queries built
//! on it keep a series only while the S3 client is making calls, and the panels
//! say why they are empty, since "no errors" and "no S3 client" would otherwise
//! both draw as a flat zero.

pub mod compaction;
pub mod object_store;
pub mod operations;
pub mod overview;
pub mod storage;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::panel::NoValue;
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

pub(crate) use crate::grafana::dependency::{
    PROCESS, latency_ladder, not_collected, zero_is_healthy,
};

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-env-persist";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "env-persist";

/// Dashboard title.
pub const TITLE: &str = "Materialize Persist (Storage)";

/// Minimum Materialize version this dashboard requires.
///
/// Every family but one has been published since 2023. The exception is hedged
/// reads, from v26.41.0, whose panel says so when it is empty.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version: the one that publishes hedged reads.
pub const REC_MZ_VERSION: &str = "v26.41.0";

/// The tabs, in order.
///
/// Overview answers "is it object storage". Operations and Compaction are the
/// foreground and background traffic, in the order a slowdown is usually
/// traced. Storage is about cost and slow growth rather than about anything
/// being broken today. Object Store Internals is last because it is the store's
/// own account rather than persist's, and only an on-premise store gives one.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::OPERATIONS.title).rows(operations::rows(q)),
        Tab::new(theme::COMPACTION.title).rows(compaction::rows(q)),
        Tab::new(theme::STORAGE.title).rows(storage::rows(q)),
        Tab::new(theme::OBJECT_STORE.title).rows(object_store::rows(q)),
    ]
}

/// The empty state of a panel built on the S3 client's own counters.
///
/// Those counters exist on every install and read zero on Azure Blob, so the
/// queries behind these panels keep a series only while the S3 client is
/// making calls — which is what makes this text reachable there.
pub(crate) fn s3_only() -> NoValue {
    NoValue::Custom(
        "No calls through the S3 client in this window. Azure Blob does not use it; S3, GCS and S3-compatible stores do."
            .to_string(),
    )
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
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
            "Object storage, as Materialize experiences it.\n\n\
             Reads, writes, failures, compaction and what the stored data is \
             for, measured by the processes that use the bucket — the same on \
             every store. What a Ceph object store reports about itself is on \
             the Object Store Internals tab, and what a cloud bucket's provider \
             reports is on the Infrastructure Cloud Provider dashboard.",
        )
        .tags([tags::MATERIALIZE, tags::MZMON, tags::content::DEPENDENCIES])
        .folder(Folder::Materialize)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::persist_scoped())
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
    use crate::grafana::test_support;

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
    fn every_tab_is_present() {
        let resource = built();
        let json = serde_json::to_string(&resource.spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }

    #[test]
    fn it_is_filed_under_materialize() {
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
        assert!(json.contains("\"materialize\""), "{json}");
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
    fn every_shard_total_deduplicates_across_processes() {
        // Each process reports every shard it has open, so a plain sum over a
        // usage family counts a shard once per process -- on the reference
        // installs, 180 series for 100 shards.
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        let mut rest = json.as_str();
        while let Some(pos) = rest.find("mz_persist_shard_usage_") {
            let before = &rest[..pos];
            let window = &before[before.len().saturating_sub(40)..];
            assert!(
                window.contains("max by (shard)"),
                "a shard usage family is aggregated without deduplicating: …{window}"
            );
            rest = &rest[pos + 1..];
        }
    }

    #[test]
    fn every_ceph_row_renders_only_where_ceph_is_scraped() {
        // A Ceph row without a condition draws empty panels on every cloud
        // bucket, which is most installs.
        let conditions = test_support::variable_conditioned_rows(&built());
        let when: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .collect();
        // One section holds them all, so its condition is the only one.
        assert_eq!(when.len(), 1, "{conditions:?}");
        assert_eq!(when[0].0, object_store::CEPH_SECTION);
        assert_eq!(when[0].2, object_store::CEPH);
    }

    #[test]
    fn the_object_store_tab_has_exactly_one_fallback_covering_every_store() {
        let conditions = test_support::variable_conditioned_rows(&built());
        let fallbacks: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        assert_eq!(fallbacks.len(), 1, "{fallbacks:?}");
        assert!(fallbacks[0].2.contains(object_store::CEPH), "{fallbacks:?}");
    }

    #[test]
    fn selecting_every_ceph_cluster_still_renders_the_rows() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn an_unset_detection_variable_renders_no_ceph_row() {
        for unset in ["", "undefined", "null", "[object Object]"] {
            assert!(!unset.contains(object_store::CEPH), "{unset}");
        }
    }

    #[test]
    fn every_ceph_query_is_scoped_by_the_ceph_picker() {
        // Ceph's series carry no environment, so the picker is the only thing
        // keeping a second Ceph cluster's numbers off these panels.
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        let selectors = test_support::selectors_of(&json, "ceph_");
        assert!(!selectors.is_empty(), "no Ceph queries found");
        for selector in selectors {
            assert!(selector.contains("$cephNamespace"), "unscoped: {selector}");
        }
    }

    #[test]
    fn the_cluster_picker_lives_on_its_section_not_the_dashboard() {
        // A dashboard-level picker sits in the controls of every environment,
        // whether or not it runs this flavor; on the section, it hides with it.
        let resource = built();
        assert!(
            !resource
                .spec
                .variables
                .iter()
                .any(|v| variable::name_of(v) == "cephNamespace"),
            "the picker is dashboard-level"
        );
        let json = serde_json::to_value(&resource.spec.layout).expect("serialize");
        let mut found = Vec::new();
        find_section_variables(&json, &mut found);
        assert_eq!(
            found,
            vec![(
                object_store::CEPH_SECTION.to_string(),
                "cephNamespace".to_string()
            )]
        );
    }

    /// `(row title, variable name)` for every row that declares a variable.
    fn find_section_variables(value: &serde_json::Value, out: &mut Vec<(String, String)>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(spec) = map.get("spec")
                    && let Some(vars) = spec.get("variables").and_then(|v| v.as_array())
                {
                    let title = spec["title"].as_str().unwrap_or_default().to_string();
                    for v in vars {
                        out.push((
                            title.clone(),
                            v["spec"]["name"].as_str().unwrap_or_default().to_string(),
                        ));
                    }
                }
                map.values().for_each(|v| find_section_variables(v, out));
            }
            serde_json::Value::Array(items) => {
                items.iter().for_each(|v| find_section_variables(v, out))
            }
            _ => {}
        }
    }

    #[test]
    fn the_fallback_explains_rather_than_announces() {
        assert!(object_store::NO_STORE.contains("monitoring.enabled"));
        assert!(object_store::NO_STORE.contains("Infrastructure Cloud Provider"));
    }

    #[test]
    fn the_logs_panel_reads_only_object_storage_work() {
        let q = Queries::new(test_registry(), &DashboardScope::default());
        let logs = q.logs("materialize.persist.logs.retries");
        let json = serde_json::to_string(&logs.query_group).expect("serialize");
        assert!(json.contains("batch::set"), "{json}");
        assert!(json.contains("fetch_batch::get"), "{json}");
        assert!(!json.contains("apply_unbatched_cmd::cas"), "{json}");
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }
}
