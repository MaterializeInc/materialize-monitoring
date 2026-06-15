// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Shared input types across the prometheus-operator Monitor kinds.
//!
//! These model the *subset* of the CRDs the transpiler consumes. They are
//! intentionally NOT `deny_unknown_fields`: the real CRDs carry far more fields
//! (auth, proxy, tls, timestamps, ...) and the transpiler simply ignores what it
//! does not translate. Correctness of the input is enforced separately against
//! the upstream CRD JSONSchemas (see `scrape::validate`).
//!
//! Field naming follows the CRDs (`camelCase`), so `serde(rename_all)` is set on
//! every struct.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Subset of `metav1.ObjectMeta` — what the transpiler needs for naming and
/// namespace scoping.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ObjectMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub labels: IndexMap<String, String>,
}

/// Subset of `metav1.LabelSelector`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub match_labels: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_expressions: Vec<LabelSelectorRequirement>,
}

/// One `matchExpressions` entry of a `LabelSelector`.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelectorRequirement {
    pub key: String,
    /// One of `In`, `NotIn`, `Exists`, `DoesNotExist`.
    pub operator: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

/// Subset of a Monitor `namespaceSelector` — picks which namespaces the
/// generated `kubernetes_sd_configs` watches.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub any: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_names: Vec<String>,
}

/// `monitoring.coreos.com/v1` RelabelConfig — the operator *input* form (note
/// the `camelCase` field names, distinct from the classic snake_case output in
/// `scrape::config::RelabelConfig`).
///
/// See: <https://prometheus-operator.dev/docs/api-reference/api/#monitoring.coreos.com/v1.RelabelConfig>
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelabelConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulus: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}
