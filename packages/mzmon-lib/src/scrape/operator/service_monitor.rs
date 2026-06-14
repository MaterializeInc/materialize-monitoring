// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `monitoring.coreos.com/v1` ServiceMonitor (input model, transpiled subset).
//!
//! ServiceMonitors select *services* by label but scrape their *endpoints*, so
//! they transpile to `role: endpoints` discovery with `keep` relabels on
//! `__meta_kubernetes_service_label_*`.
//!
//! See: <https://prometheus-operator.dev/docs/api-reference/api/#monitoring.coreos.com/v1.ServiceMonitor>

use serde::{Deserialize, Serialize};

use super::common::{LabelSelector, NamespaceSelector, ObjectMeta, RelabelConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMonitor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default)]
    pub metadata: ObjectMeta,
    pub spec: ServiceMonitorSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMonitorSpec {
    /// Selects the services whose endpoints to scrape. Becomes `keep` relabels
    /// on `__meta_kubernetes_service_label_*`.
    #[serde(default)]
    pub selector: LabelSelector,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_selector: Option<NamespaceSelector>,
    /// One classic scrape job is generated per endpoint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<Endpoint>,
}

/// One `endpoints` entry. Subset of `v1.Endpoint`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    /// Named service port. Becomes a `keep` on
    /// `__meta_kubernetes_endpoint_port_name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    /// Target port on the pod (name or number), as an alternative to `port`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_port: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_timeout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_labels: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relabelings: Vec<RelabelConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric_relabelings: Vec<RelabelConfig>,
}
