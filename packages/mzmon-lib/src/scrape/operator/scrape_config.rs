// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `monitoring.coreos.com/v1alpha1` ScrapeConfig (input model, transpiled subset).
//!
//! Named `ScrapeConfigCrd` to avoid colliding with the classic output type
//! `scrape::config::ScrapeJob` and the broader notion of a "scrape config". A
//! ScrapeConfig is the closest-to-classic CRD: its `kubernetesSDConfigs` and
//! `relabelings` translate almost 1:1 to a classic scrape job (modulo casing,
//! e.g. role `Node` → `node`).
//!
//! See: <https://prometheus-operator.dev/docs/api-reference/api/#monitoring.coreos.com/v1alpha1.ScrapeConfig>

use serde::{Deserialize, Serialize};

use super::common::{ObjectMeta, RelabelConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeConfigCrd {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default)]
    pub metadata: ObjectMeta,
    pub spec: ScrapeConfigSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeConfigSpec {
    /// Note the SD/configs casing: the CRD field is `kubernetesSDConfigs`.
    #[serde(
        default,
        rename = "kubernetesSDConfigs",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub kubernetes_sd_configs: Vec<KubernetesSdConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relabelings: Vec<RelabelConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric_relabelings: Vec<RelabelConfig>,
    // Other SD config kinds (static, http, file, ...) are out of scope for the
    // first pass; add typed fields here as they are needed.
}

/// Subset of `v1alpha1.KubernetesSDConfig` (operator input). `role` is
/// PascalCase here (`Node`, `Pod`, ...); the transpiler lowercases it for the
/// classic config.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesSdConfig {
    pub role: String,
    // namespaces / selectors / apiServer omitted in the first pass (the default
    // in-cluster apiServer is what the Materialize scrapers rely on).
}
