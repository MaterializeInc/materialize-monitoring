// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `monitoring.coreos.com/v1` PodMonitor (input model, transpiled subset).
//!
//! See: <https://prometheus-operator.dev/docs/api-reference/api/#monitoring.coreos.com/v1.PodMonitor>

use serde::{Deserialize, Serialize};

use super::common::{BasicAuth, LabelSelector, NamespaceSelector, ObjectMeta, RelabelConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodMonitor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default)]
    pub metadata: ObjectMeta,
    pub spec: PodMonitorSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodMonitorSpec {
    /// Selects the pods to scrape. Becomes `keep` relabels on
    /// `__meta_kubernetes_pod_label_*` / `__meta_kubernetes_pod_labelpresent_*`.
    #[serde(default)]
    pub selector: LabelSelector,
    /// Scopes which namespaces are discovered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_selector: Option<NamespaceSelector>,
    /// Pod label keys to copy onto the scraped metrics (e.g.
    /// `materialize.cloud/organization-name`). classic → a per-label `replace`
    /// relabel from `__meta_kubernetes_pod_label_*`; GMP → `targetLabels.fromPod`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pod_target_labels: Vec<String>,
    /// One classic scrape job is generated per endpoint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pod_metrics_endpoints: Vec<PodMetricsEndpoint>,
}

/// One `podMetricsEndpoints` entry. Subset of `v1.PodMetricsEndpoint`.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PodMetricsEndpoint {
    /// Named container port to scrape. Becomes a `keep` on
    /// `__meta_kubernetes_pod_container_port_name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    /// Numeric container port (alternative to `port`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_number: Option<i32>,
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
    /// Basic-auth credentials for the scrape request. Operator form: both the
    /// username and password reference keys in a Kubernetes Secret. The SQL
    /// metrics endpoints (`/metrics/mz_*`) run the request as the authenticated
    /// Materialize user, so this scopes which clusters' metrics are returned. The
    /// classic output emits inline placeholders and GMP carries the password as a
    /// secret reference (see `transpile`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic_auth: Option<BasicAuth>,
    /// Operator-form relabelings, passed through to the job's `relabel_configs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relabelings: Vec<RelabelConfig>,
    /// Operator-form metric relabelings → `metric_relabel_configs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric_relabelings: Vec<RelabelConfig>,
}
