// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Capabilities: what a deployment contains that a rule needs.
//!
//! A rule is installed only where every capability it requires is present. The
//! vocabulary names what a deployment *contains* — a metadata-database flavor, a
//! CNI, an exporter — rather than who operates it, which is what the
//! `deploymentMode: cloud-only` label it replaces got wrong: a CockroachDB rule is
//! for a deployment running CockroachDB, whoever runs it.
//!
//! Most of a rule's requirements are **inferred** from the metrics it reads,
//! through [`capability_for_metric`]. That table is also the build-time
//! applicability check: a metric no entry claims is an error, because nothing in
//! this repository knows what produces it, and a rule reading it may never fire.
//!
//! The same vocabulary is an enum in `mzmon-query.schema.yaml` (for an alert's
//! declared `requires`), and a test keeps the two in agreement.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Something a deployment contains that a rule needs.
///
/// Declaration order is the order capabilities are listed in generated output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    // Derived by the chart from what it deploys.
    Materialize,
    MaterializeSql,
    MaterializeOperator,
    KubeStateMetrics,
    Cadvisor,
    NodeExporter,
    Loki,
    Alloy,
    Cloudwatch,
    CloudMonitoring,
    AzureMonitor,
    // Set explicitly by an operator.
    SyntheticUptime,
    ExternalUptime,
    FeatureFlags,
    FronteggAuth,
    MemoryLimiter,
    CrdbDedicated,
    Cilium,
    Coredns,
    CertManager,
    KubeletMetrics,
    SwapNodes,
    EgressGateway,
}

impl Capability {
    /// Every capability, in declaration order.
    pub const ALL: &'static [Capability] = &[
        Capability::Materialize,
        Capability::MaterializeSql,
        Capability::MaterializeOperator,
        Capability::KubeStateMetrics,
        Capability::Cadvisor,
        Capability::NodeExporter,
        Capability::Loki,
        Capability::Alloy,
        Capability::Cloudwatch,
        Capability::CloudMonitoring,
        Capability::AzureMonitor,
        Capability::SyntheticUptime,
        Capability::ExternalUptime,
        Capability::FeatureFlags,
        Capability::FronteggAuth,
        Capability::MemoryLimiter,
        Capability::CrdbDedicated,
        Capability::Cilium,
        Capability::Coredns,
        Capability::CertManager,
        Capability::KubeletMetrics,
        Capability::SwapNodes,
        Capability::EgressGateway,
    ];

    /// The kebab-case wire value, as written in `requires` and in values.
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::Materialize => "materialize",
            Capability::MaterializeSql => "materialize-sql",
            Capability::MaterializeOperator => "materialize-operator",
            Capability::KubeStateMetrics => "kube-state-metrics",
            Capability::Cadvisor => "cadvisor",
            Capability::NodeExporter => "node-exporter",
            Capability::Loki => "loki",
            Capability::Alloy => "alloy",
            Capability::Cloudwatch => "cloudwatch",
            Capability::CloudMonitoring => "cloud-monitoring",
            Capability::AzureMonitor => "azure-monitor",
            Capability::SyntheticUptime => "synthetic-uptime",
            Capability::ExternalUptime => "external-uptime",
            Capability::FeatureFlags => "feature-flags",
            Capability::FronteggAuth => "frontegg-auth",
            Capability::MemoryLimiter => "memory-limiter",
            Capability::CrdbDedicated => "crdb-dedicated",
            Capability::Cilium => "cilium",
            Capability::Coredns => "coredns",
            Capability::CertManager => "cert-manager",
            Capability::KubeletMetrics => "kubelet-metrics",
            Capability::SwapNodes => "swap-nodes",
            Capability::EgressGateway => "egress-gateway",
        }
    }

    /// Whether the chart derives this capability from what it deploys, rather
    /// than an operator listing it in `rules.capabilities`.
    ///
    /// The chart's `mzmon.rules.derivedCapabilities` helper must derive exactly
    /// these. Nothing checks the two lists against each other, so a capability
    /// added here needs its derivation added there by hand.
    pub fn is_derived(self) -> bool {
        matches!(
            self,
            Capability::Materialize
                | Capability::MaterializeSql
                | Capability::MaterializeOperator
                | Capability::KubeStateMetrics
                | Capability::Cadvisor
                | Capability::NodeExporter
                | Capability::Loki
                | Capability::Alloy
                | Capability::Cloudwatch
                | Capability::CloudMonitoring
                | Capability::AzureMonitor
        )
    }

    /// Whether this capability is a provider pull, whose families are tiered by
    /// `pipeline.metrics.provider.<name>.metricImportance` in values rather
    /// than by the registry queries that read them.
    pub fn is_provider_pull(self) -> bool {
        matches!(
            self,
            Capability::Cloudwatch | Capability::CloudMonitoring | Capability::AzureMonitor
        )
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What produces a metric, as far as rule applicability is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricSource {
    /// Produced where `capability` is present.
    Capability(Capability),
    /// Produced for every scrape target, so its name says nothing about which
    /// one a rule means. A rule reading only neutral metrics has to declare
    /// what it needs.
    Neutral,
}

/// The metric-name table, most specific first: the first matching entry wins.
///
/// Order is load-bearing wherever one family nests inside another:
///
/// - The synthetic-uptime probes are SQL-prefixed like every other
///   `materialize-sql` family, so they come before the generic prefix.
/// - Alloy's `loki.*` components publish `loki_write_*`, `loki_source_*` and so
///   on, inside the name space the bundled Loki uses for its own metrics.
/// - The `mz_*` families that exist only where something optional is running
///   (Frontegg, a feature-flag sync, the external checker, the memory limiter)
///   come before the catch-all `mz_.+`.
///
/// `__mzmon_sql_prefix__` is the alerting context's placeholder for
/// `%%{mzSqlPrefix}`, which is what a rule's metric names read as at build time.
const SOURCES: &[(&str, MetricSource)] = &[
    ("up", MetricSource::Neutral),
    (
        "__mzmon_sql_prefix__(can_connect|envd_up|views_query_successful)",
        MetricSource::Capability(Capability::SyntheticUptime),
    ),
    (
        "__mzmon_sql_prefix__.+",
        MetricSource::Capability(Capability::MaterializeSql),
    ),
    (
        "mz_auth_.+",
        MetricSource::Capability(Capability::FronteggAuth),
    ),
    (
        "mz_parameter_frontend_.+",
        MetricSource::Capability(Capability::FeatureFlags),
    ),
    (
        "mz_external_.+",
        MetricSource::Capability(Capability::ExternalUptime),
    ),
    (
        "mz_memory_limiter_.+",
        MetricSource::Capability(Capability::MemoryLimiter),
    ),
    ("mz_.+", MetricSource::Capability(Capability::Materialize)),
    (
        "(orchestratord|environmentd)_.+",
        MetricSource::Capability(Capability::MaterializeOperator),
    ),
    (
        "kube_.+",
        MetricSource::Capability(Capability::KubeStateMetrics),
    ),
    (
        "(container|machine)_.+",
        MetricSource::Capability(Capability::Cadvisor),
    ),
    (
        "node_.+",
        MetricSource::Capability(Capability::NodeExporter),
    ),
    (
        "kubelet_.+",
        MetricSource::Capability(Capability::KubeletMetrics),
    ),
    (
        "(alloy|loki_(write|source|process|relabel|secretfilter))_.+",
        MetricSource::Capability(Capability::Alloy),
    ),
    ("loki_.+", MetricSource::Capability(Capability::Loki)),
    // The provider pulls (`pipeline.metrics.provider.*`). The families are the
    // ones the chart tiers per provider in
    // `mzmon.alloyGateway.provider.metricPatterns`, and `aws_` alone is too
    // broad: the AWS Load Balancer Controller publishes `aws_api_*` from inside
    // the cluster.
    (
        "aws_(rds|s3|ec2|autoscaling|usage)_.+",
        MetricSource::Capability(Capability::Cloudwatch),
    ),
    (
        "stackdriver_(cloudsql_database|gcs_bucket|compute_googleapis_com_location)_.+",
        MetricSource::Capability(Capability::CloudMonitoring),
    ),
    (
        "azure_microsoft_(dbforpostgresql_flexibleservers|storage_storageaccounts_blobservices|containerservice_managedclusters|compute_virtualmachinescalesets)_.+",
        MetricSource::Capability(Capability::AzureMonitor),
    ),
    (
        "crdb_dedicated_.+",
        MetricSource::Capability(Capability::CrdbDedicated),
    ),
    (
        "(cilium|hubble)_.+",
        MetricSource::Capability(Capability::Cilium),
    ),
    ("coredns_.+", MetricSource::Capability(Capability::Coredns)),
    (
        "certmanager_.+",
        MetricSource::Capability(Capability::CertManager),
    ),
];

static COMPILED: LazyLock<Vec<(Regex, MetricSource)>> = LazyLock::new(|| {
    SOURCES
        .iter()
        .map(|(pattern, source)| {
            let anchored = format!("^(?:{pattern})$");
            let regex = Regex::new(&anchored).expect("capability source patterns compile");
            (regex, *source)
        })
        .collect()
});

/// What produces `metric`, or `None` when no entry claims it.
pub fn capability_for_metric(metric: &str) -> Option<MetricSource> {
    COMPILED
        .iter()
        .find(|(regex, _)| regex.is_match(metric))
        .map(|(_, source)| *source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::validate::SCHEMA;

    fn source(metric: &str) -> Option<MetricSource> {
        capability_for_metric(metric)
    }

    #[test]
    fn nested_families_resolve_to_the_specific_capability() {
        use Capability::*;
        use MetricSource::Capability as C;
        let cases = [
            ("__mzmon_sql_prefix__can_connect", C(SyntheticUptime)),
            (
                "__mzmon_sql_prefix__compute_cluster_status",
                C(MaterializeSql),
            ),
            ("mz_auth_request_count", C(FronteggAuth)),
            (
                "mz_parameter_frontend_last_sse_time_seconds",
                C(FeatureFlags),
            ),
            ("mz_external_envd_up", C(ExternalUptime)),
            ("mz_memory_limiter_memory_usage_bytes", C(MemoryLimiter)),
            ("mz_persist_blob_failures", C(Materialize)),
            (
                "orchestratord_reconciliations_total",
                C(MaterializeOperator),
            ),
            ("loki_write_dropped_entries_total", C(Alloy)),
            ("loki_request_duration_seconds_count", C(Loki)),
            ("container_memory_working_set_bytes", C(Cadvisor)),
            ("kube_pod_status_phase", C(KubeStateMetrics)),
            ("aws_rds_free_storage_space_minimum", C(Cloudwatch)),
            (
                "stackdriver_cloudsql_database_cloudsql_googleapis_com_database_up",
                C(CloudMonitoring),
            ),
            (
                "azure_microsoft_dbforpostgresql_flexibleservers_is_db_alive_minimum_count",
                C(AzureMonitor),
            ),
        ];
        for (metric, expected) in cases {
            assert_eq!(source(metric), Some(expected), "{metric}");
        }
    }

    #[test]
    fn up_is_neutral_and_strangers_are_unclaimed() {
        assert_eq!(source("up"), Some(MetricSource::Neutral));
        // Anchored: a name that merely contains `up` is not neutral.
        assert_eq!(source("setup_total"), None);
        assert_eq!(source("some_vendor_thing_total"), None);
        // The load balancer controller's own client metrics, not the pull.
        assert_eq!(source("aws_api_calls_total"), None);
    }

    #[test]
    fn wire_values_round_trip() {
        for capability in Capability::ALL {
            let json = serde_json::to_string(capability).unwrap();
            assert_eq!(json, format!("\"{}\"", capability.as_str()));
            let back: Capability = serde_json::from_str(&json).unwrap();
            assert_eq!(back, *capability);
        }
    }

    /// The schema's `capability` enum is what an alert's `requires` is validated
    /// against; this enum is what the renderer and the chart index use. A
    /// capability in one and not the other either rejects a valid rule or ships
    /// one the chart cannot select.
    #[test]
    fn schema_vocabulary_matches() {
        let schema: serde_json::Value = serde_yaml_ng::from_str(SCHEMA).unwrap();
        let mut from_schema: Vec<String> = schema["$defs"]["capability"]["oneOf"]
            .as_array()
            .expect("capability is a oneOf of consts")
            .iter()
            .map(|entry| entry["const"].as_str().unwrap().to_string())
            .collect();
        let mut from_rust: Vec<String> = Capability::ALL
            .iter()
            .map(|c| c.as_str().to_string())
            .collect();
        from_schema.sort();
        from_rust.sort();
        assert_eq!(from_schema, from_rust);
    }

    #[test]
    fn every_source_capability_is_in_the_vocabulary() {
        // `ALL` is hand-maintained beside the enum; a variant missing from it
        // would never be listed in generated output.
        for (_, source) in SOURCES {
            if let MetricSource::Capability(c) = source {
                assert!(Capability::ALL.contains(c), "{c} missing from ALL");
            }
        }
        assert_eq!(Capability::ALL.len(), 23);
    }
}
