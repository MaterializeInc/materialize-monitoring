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
//! Object Store Internals carries the store's own account, on rows that render
//! only where it is collected — see [`object_store`]. An on-premise store has
//! no provider to ask and can fill up or lose a disk, so Ceph run by Rook gets
//! its health, capacity and disks. A GCS bucket gets its billed size beside
//! this environment's data, which is the comparison above made on one
//! dashboard. S3 and Azure Blob stay on `infra-cloud`.
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
/// own account rather than persist's, and only some stores give one here.
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
             every store. What a Ceph object store reports about itself, and \
             what Cloud Monitoring reports about a GCS bucket, is on the Object \
             Store Internals tab; what the provider reports about an S3 or \
             Azure Blob bucket is on the Infrastructure Cloud Provider \
             dashboard.",
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
    fn every_store_row_renders_only_where_its_store_is_collected() {
        // A store's row without a condition draws empty panels on every
        // deployment that does not use it, which is most installs.
        let conditions = test_support::variable_conditioned_rows(&built());
        let mut when: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .map(|(title, _, value)| (title.as_str(), value.as_str()))
            .collect();
        when.sort();
        // One section per store holds all its rows, so these are the only
        // conditions.
        assert_eq!(
            when,
            vec![
                (object_store::CEPH_SECTION, object_store::CEPH),
                (object_store::GCS_SECTION, object_store::GCS),
            ],
            "{conditions:?}"
        );
    }

    #[test]
    fn the_object_store_tab_has_exactly_one_fallback_covering_every_store() {
        let conditions = test_support::variable_conditioned_rows(&built());
        let fallbacks: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        let titles: std::collections::HashSet<_> = fallbacks.iter().map(|f| &f.0).collect();
        assert_eq!(titles.len(), 1, "{fallbacks:?}");
        let values: Vec<_> = fallbacks.iter().map(|f| f.2.as_str()).collect();
        for (_, value) in object_store::known_stores() {
            assert!(values.contains(&value.as_str()), "{fallbacks:?}");
        }
    }

    #[test]
    fn selecting_every_store_still_renders_the_rows() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn an_unset_detection_variable_renders_no_store_row() {
        // An empty discovery reaches the condition as one of these strings.
        for unset in ["", "undefined", "null", "[object Object]"] {
            for (_, value) in object_store::known_stores() {
                assert!(!unset.contains(value.as_str()), "{unset}");
            }
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
    fn every_gcs_query_is_scoped_by_the_bucket_picker() {
        // A bucket series carries no environment, and the monitoring stack's
        // own buckets are pulled beside the persist bucket, so the picker is
        // the only thing keeping them off these panels once narrowed.
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        let selectors = test_support::selectors_of(&json, "stackdriver_gcs_");
        assert!(!selectors.is_empty(), "no GCS queries found");
        for selector in selectors {
            assert!(selector.contains("$gcsBucket"), "unscoped: {selector}");
        }
    }

    #[test]
    fn each_picker_lives_on_its_section_not_the_dashboard() {
        // A dashboard-level picker sits in the controls of every environment,
        // whether or not it uses this store; on the section, it hides with it.
        let resource = built();
        for picker in ["cephNamespace", "gcsBucket"] {
            assert!(
                !resource
                    .spec
                    .variables
                    .iter()
                    .any(|v| variable::name_of(v) == picker),
                "{picker} is dashboard-level"
            );
        }
        let json = serde_json::to_value(&resource.spec.layout).expect("serialize");
        let mut found = Vec::new();
        find_section_variables(&json, &mut found);
        found.sort();
        assert_eq!(
            found,
            vec![
                (
                    object_store::CEPH_SECTION.to_string(),
                    "cephNamespace".to_string()
                ),
                (
                    object_store::GCS_SECTION.to_string(),
                    "gcsBucket".to_string()
                ),
            ]
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
        assert!(object_store::NO_STORE.contains("gcp.gcs.buckets"));
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
