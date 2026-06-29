// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Classic Prometheus configuration (the transpiler *output* model).
//!
//! These are plain serde structs serialized straight to YAML — there is no
//! custom renderer. **Field declaration order is the emitted YAML key order**,
//! so the order here is chosen to read like a hand-written `prometheus.yml`
//! (compare `legacy_scrape_config.yaml`).
//!
//! See: <https://prometheus.io/docs/prometheus/latest/configuration/configuration/>

use serde::{Deserialize, Serialize};

use crate::scrape::error::Result;

fn is_false(b: &bool) -> bool {
    !*b
}

/// A full classic Prometheus config document: a `global` block plus the list of
/// scrape jobs. This is the single combined artifact the transpiler emits.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScrapeConfigDocument {
    pub global: GlobalConfig,
    #[serde(default)]
    pub scrape_configs: Vec<ScrapeJob>,
}

impl ScrapeConfigDocument {
    /// Serialize the document to classic-Prometheus YAML.
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml_ng::to_string(self)?)
    }
}

/// The `global:` block. Defaults mirror `legacy_scrape_config.yaml`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalConfig {
    pub evaluation_interval: String,
    pub scrape_interval: String,
    pub scrape_timeout: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            evaluation_interval: "1m".into(),
            scrape_interval: "1m".into(),
            scrape_timeout: "10s".into(),
        }
    }
}

/// One classic `scrape_configs` entry.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ScrapeJob {
    pub job_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_labels: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_interval: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_timeout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic_auth: Option<BasicAuth>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kubernetes_sd_configs: Vec<KubernetesSdConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_config: Option<TlsConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token_file: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relabel_configs: Vec<RelabelConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric_relabel_configs: Vec<RelabelConfig>,
}

/// Classic `kubernetes_sd_configs` entry (the subset we emit).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KubernetesSdConfig {
    /// Lowercase role: `pod`, `node`, `endpoints`, `service`, ...
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespaces: Option<Namespaces>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Namespaces {
    #[serde(default, skip_serializing_if = "is_false")]
    pub own_namespace: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
}

/// Classic `basic_auth` block. Classic Prometheus cannot reference a Kubernetes
/// Secret, so credentials are either inline (`username` / `password`) or read
/// from a mounted file (`password_file`). The transpiler emits inline
/// placeholders; see `docs/content/metrics/scraping.md` for the `password_file`
/// alternative.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BasicAuth {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_file: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TlsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insecure_skip_verify: Option<bool>,
}

/// Classic `relabel_configs` / `metric_relabel_configs` entry (snake_case field
/// names — the operator input form lives in `scrape::monitor::common`).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
