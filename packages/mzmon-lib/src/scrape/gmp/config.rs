// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Google Managed Service for Prometheus (GMP) output model.
//!
//! `monitoring.googleapis.com/v1` `PodMonitoring` (namespaced) and
//! `ClusterPodMonitoring` (cluster-scoped). This is an **output-only** model —
//! the transpiler emits these from prometheus-operator inputs; we never parse
//! GMP. We deliberately do NOT embed the GMP CRD schemas (unlike the operator
//! inputs): correctness here is by construction, and the structs cover only the
//! subset we emit.
//!
//! The two kinds share a spec, so one struct carries a `kind` discriminator and
//! an optional `metadata.namespace` (set for `PodMonitoring`, omitted for the
//! cluster-scoped variant).
//!
//! Several shapes are structurally identical to k8s / prometheus-operator types,
//! so we reuse them rather than redefine: `ObjectMeta` and `LabelSelector` are
//! universal, and GMP's relabeling rule matches `monitoring.coreos.com`'s
//! `RelabelConfig` field-for-field.
//!
//! See: <https://github.com/GoogleCloudPlatform/prometheus-engine/blob/main/doc/api.md>

use serde::{Deserialize, Serialize};

use crate::scrape::operator::common::{LabelSelector, ObjectMeta, RelabelConfig};

/// The `apiVersion` all GMP monitoring resources carry.
pub const API_VERSION: &str = "monitoring.googleapis.com/v1";

/// A GMP `PodMonitoring` or `ClusterPodMonitoring` resource. The `kind` field
/// selects which; `metadata.namespace` is set only for the namespaced variant.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodMonitoring {
    pub api_version: String,
    /// `PodMonitoring` or `ClusterPodMonitoring`.
    pub kind: String,
    pub metadata: ObjectMeta,
    pub spec: PodMonitoringSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodMonitoringSpec {
    pub selector: LabelSelector,
    pub endpoints: Vec<ScrapeEndpoint>,
    /// Emitted only when the source declares `podTargetLabels`. We leave GMP's
    /// `metadata` default (`container`, `pod`, `top_level_controller_*`) intact
    /// by omitting it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_labels: Option<TargetLabels>,
}

/// GMP `spec.targetLabels`. We only populate `fromPod`; `metadata` is omitted so
/// the CRD default applies.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TargetLabels {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from_pod: Vec<LabelMapping>,
}

/// One `fromPod` mapping: copy pod label `from` onto metric label `to`
/// (`to` must be a valid Prometheus label name).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LabelMapping {
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// One `spec.endpoints[]` entry. Subset of the GMP `ScrapeEndpoint`.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeEndpoint {
    /// Port name or number. GMP models this as a single int-or-string field
    /// (unlike the operator's split `port` / `portNumber`).
    pub port: IntOrString,
    /// Scrape interval. Required by GMP; the transpiler fills a default when the
    /// source endpoint omits it.
    pub interval: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    /// GMP exposes only metric relabeling (singular `metricRelabeling`); it has
    /// no target-relabeling surface, so the operator's `relabelings` are dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric_relabeling: Vec<RelabelConfig>,
}

/// A value that serializes as either an integer or a string (k8s
/// `IntOrString`). Used for GMP's `port`.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum IntOrString {
    Int(i64),
    Str(String),
}
