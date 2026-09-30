// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The cloud provider dashboard: the metadata database and the buckets, as their
//! provider reports them.
//!
//! The provider's view of Materialize's two external dependencies — what
//! CloudWatch, Cloud Monitoring and Azure Monitor publish about the instances
//! and buckets listed in `pipeline.metrics.provider`. It is the half of the
//! external-dependency picture that can say *why*: a saturated CPU, a volume out
//! of burst credit, a disk filling, a neighbour on a shared instance. The half
//! that says *whether* — what Materialize experienced — is `env-consensus` and
//! `env-persist`, and those are what to open first.
//!
//! # Why `infra-*`
//!
//! A database and a bucket belong to the platform rather than to an
//! environment, and their series carry a provider's resource name and no
//! Materialize identity at all, so there is nothing an environment picker could
//! narrow. The design doc proposes a values-supplied mapping that would make
//! them joinable to an environment; until it exists, the two panels here that
//! draw Materialize's own numbers beside the provider's sum every environment on
//! the cluster, and say so.
//!
//! # Rows render on the provider
//!
//! The same mechanism `infra-net` uses for CNIs: the provider pulls' `up` is
//! discovered into `$cloudProviderList`, and each provider's rows render on it.
//! A negated fallback on each data tab explains the common case — provider
//! collection is opt-in and off by default — rather than leaving empty tabs.
//!
//! # The provider set is fixed by the pipeline
//!
//! Every family here is one `packages/alloy-pipelines/gateway-provider.yaml`
//! pulls, and nothing else. Adding a metric there is not enough to draw it: this
//! dashboard and `infra-cloud.yaml` have to follow.

pub mod collection;
pub mod database;
pub mod object_storage;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Layout, Row, RowHeight, Tab};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::variable::extra;
use mzmon_lib::grafana::{dashboard, folder::Folder, palette, tags, threshold, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-infra-cloud";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "infra-cloud";

/// Dashboard title.
pub const TITLE: &str = "Infrastructure Cloud Provider";

/// Minimum Materialize version this dashboard requires.
///
/// None in particular: two panels read persist's own counters, published since
/// 2023, and everything else is the provider's.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// The variable every provider row is rendered on.
pub(crate) const PROVIDERS: &str = extra::CLOUD_PROVIDER_LIST;

/// CloudWatch, as its pull's `job` names it (`integrations/cloudwatch`).
pub(crate) const AWS: &str = "cloudwatch";
/// Cloud Monitoring, as its pull's `job` names it (`integrations/gcp`).
pub(crate) const GCP: &str = "gcp";
/// Azure Monitor, as its pull's `job` names it (`integrations/azure`).
pub(crate) const AZURE: &str = "azure";

/// Every provider this dashboard has rows for, as one alternation.
///
/// The fallback renders on the *negation* of this, so a provider added above
/// without being added here would leave the fallback on screen beside its rows.
pub(crate) fn known_providers() -> String {
    format!("{AWS}|{GCP}|{AZURE}")
}

/// One legend per provider, in the order every cross-provider database query
/// lists its expressions: CloudWatch, Cloud Monitoring, Azure Monitor.
pub(crate) const DB_LEGENDS: [&str; 3] = [
    "{{dimension_DBInstanceIdentifier}}",
    "{{database_id}}",
    "{{resourceName}}",
];

/// [`DB_LEGENDS`] for the bucket queries.
pub(crate) const BUCKET_LEGENDS: [&str; 3] = [
    "{{dimension_BucketName}}",
    "{{bucket_name}}",
    "{{resourceName}}",
];

/// The tabs, in order.
///
/// The database first: it is the dependency whose provider view matters most,
/// since object storage does not fill up and its health is better read from
/// the client. Collection last, since an empty row already says most of what it
/// would.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::DATABASE.title).rows(database::rows(q)),
        Tab::new(theme::OBJECT_STORAGE.title).rows(object_storage::rows(q)),
        Tab::new(theme::COLLECTION.title).rows(collection::rows(q)),
    ]
}

/// What a provider panel shows when its provider sent nothing.
///
/// Reachable on a row that rendered, which means a pull exists: so the likely
/// causes are a pull still waiting for its first data, or one the provider is
/// refusing.
pub(crate) fn not_pulled() -> NoValue {
    NoValue::Custom(
        "No data from the provider. A new pull takes a few minutes to report; see Collection if it does not."
            .to_string(),
    )
}

/// A ladder for a bounded fraction where high is bad.
pub(crate) fn ratio_ladder(degraded: f64, unhealthy: f64) -> dashboardv2::ThresholdsConfig {
    threshold::Ladder::new(palette::tri_health::HEALTHY)
        .step(degraded, palette::tri_health::DEGRADED)
        .step(unhealthy, palette::tri_health::UNHEALTHY)
        .build()
}

/// Why a tab has no provider rows on it.
///
/// Its job is the reason, not the absence: provider collection is off unless
/// configured, so the common case is not a fault, and the reader needs to know
/// that and where the same question is answered without it.
pub(crate) const NO_PROVIDER: &str = "**No cloud provider metrics are being collected.**\n\n\
     This is the default, not a fault. Pulling from CloudWatch, Cloud Monitoring or Azure Monitor \
     is opt-in, because it needs a cloud identity with read access and every pull is billed by \
     the provider.\n\n\
     - **To turn it on**, list the metadata database and buckets under \
       `pipeline.metrics.provider.cloudwatch`, `.gcp` or `.azure` in the `materialize-monitoring` \
       chart's values, and give the gateway's service account read access: \
       `cloudwatch:GetMetricStatistics` on AWS, `roles/monitoring.viewer` on GCP, or Monitoring \
       Reader on Azure. The Cloud Provider Metrics page of the documentation has the details.\n\
     - **A pull that is configured but not yet working still renders its rows here**, empty, so \
       this note never hides a failing pull.\n\
     - **Self-hosted databases and on-premise object stores** — CNPG, CockroachDB, MinIO — have \
       no cloud provider, and nothing on this dashboard applies to them.\n\n\
     Without provider metrics, Materialize's own view of both dependencies still works on every \
     deployment: **Materialize Consensus** for the metadata database and **Materialize Persist** \
     for object storage. That view says whether a dependency is failing Materialize; this \
     dashboard, once enabled, says why.";

/// The fallback row a data tab carries, on the negation of every provider.
pub(crate) fn no_provider_row(panel_name: &str) -> Row {
    Row::new("No Provider Metrics")
        .only_unless_variable(PROVIDERS, known_providers())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                panel_name,
                Panel::text("No Provider Metrics", NO_PROVIDER)
                    .description("Why this tab has no provider rows on it.")
                    .build(0),
            ),
        )
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
            "The metadata database and the buckets Materialize depends on, as their \
             cloud provider reports them.\n\n\
             CloudWatch, Cloud Monitoring and Azure Monitor metrics pulled by the \
             gateway, with rows for whichever provider is configured. Materialize's \
             own view of the same dependencies is on Materialize Consensus and \
             Materialize Persist.",
        )
        .tags([tags::INFRA, tags::MZMON, tags::content::DEPENDENCIES])
        .folder(Folder::Infra)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::cloud_scoped())
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
    fn it_is_filed_under_infrastructure() {
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
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
    fn every_provider_row_is_rendered_on_a_known_provider() {
        // A provider row without a condition renders on every cloud and shows
        // nothing but empty panels on two of them.
        let conditions = test_support::variable_conditioned_rows(&built());
        let when: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .collect();
        assert!(
            when.len() >= 8,
            "expected a row per provider section: {conditions:?}"
        );
        for (title, _, value) in when {
            assert!(
                value == AWS || value == GCP || value == AZURE || *value == known_providers(),
                "{title} is conditioned on an unknown provider {value}"
            );
        }
    }

    #[test]
    fn selecting_every_provider_still_renders_their_rows() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn each_data_tab_has_exactly_one_fallback_covering_every_provider() {
        let conditions = test_support::variable_conditioned_rows(&built());
        let fallbacks: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        // One on Metadata Database, one on Object Storage.
        assert_eq!(fallbacks.len(), 2, "{fallbacks:?}");
        for (_, _, pattern) in fallbacks {
            for provider in [AWS, GCP, AZURE] {
                assert!(pattern.contains(provider), "{pattern} misses {provider}");
            }
        }
    }

    #[test]
    fn the_provider_names_match_the_pull_jobs() {
        // Alloy names an exporter's job `integrations/<component suffix>`; the
        // row conditions match a substring of it, and the discovery variable
        // reads it back. All three have to agree or a provider's rows never
        // render.
        let q = Queries::new(test_registry(), &DashboardScope::default());
        let pulls = q.get("infra.cloud.collection.pulls");
        let json = serde_json::to_string(&pulls.query_group).expect("serialize");
        let discovery = serde_json::to_string(&variable::cloud_providers()).expect("serialize");
        for provider in [AWS, GCP, AZURE] {
            let job = format!("integrations/({AWS}|{GCP}|{AZURE})");
            assert!(json.contains(&job), "{json} does not select {provider}");
            assert!(
                discovery.contains(&job),
                "{discovery} does not discover {provider}"
            );
        }
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn bucket_sizes_and_counts_are_drawn_on_a_log_axis() {
        // One install's buckets span four orders of magnitude, and on a linear
        // axis all but the largest read as zero. A log axis has no zero, so a
        // `min` of 0 would be meaningless on it too.
        let resource = built();
        let json = serde_json::to_value(&resource.spec.elements).expect("serialize");
        let titles = [
            "Bucket Size",
            "Object Count",
            "Bytes by Object State",
            "Objects by Object State",
        ];
        let mut seen = 0;
        for element in json.as_object().expect("elements").values() {
            let spec = &element["spec"];
            let Some(title) = spec["title"].as_str() else {
                continue;
            };
            if !titles.contains(&title) {
                continue;
            }
            seen += 1;
            let defaults = &spec["vizConfig"]["spec"]["fieldConfig"]["defaults"];
            assert_eq!(
                defaults["custom"]["scaleDistribution"]["type"], "log",
                "{title} is not on a log axis"
            );
            assert!(
                defaults["min"].is_null(),
                "{title} pins a min on a log axis"
            );
        }
        assert_eq!(seen, titles.len(), "a bucket panel went missing");
    }

    #[test]
    fn the_fallback_explains_rather_than_announces() {
        assert!(NO_PROVIDER.contains("not a fault"));
        assert!(NO_PROVIDER.contains("pipeline.metrics.provider"));
        // And where to go instead.
        assert!(NO_PROVIDER.contains("Materialize Consensus"));
        assert!(NO_PROVIDER.contains("Materialize Persist"));
    }

    #[test]
    fn no_provider_counter_is_rated_over_the_dashboard_interval() {
        // `$__rate_interval` is sized from the datasource's scrape interval,
        // which is far shorter than a five-minute pull: a `rate()` over it sees
        // one sample and draws nothing.
        let resource = built();
        let json = serde_json::to_string(&resource.spec.elements).expect("serialize");
        for family in ["stackdriver_", "aws_", "azure_"] {
            let mut rest = json.as_str();
            while let Some(pos) = rest.find(family) {
                let tail = &rest[pos..];
                let end = tail.find(')').unwrap_or(tail.len());
                assert!(
                    !tail[..end].contains("$__rate_interval"),
                    "a provider family is rated over the dashboard interval: {}",
                    &tail[..end]
                );
                rest = &rest[pos + 1..];
            }
        }
    }
}
