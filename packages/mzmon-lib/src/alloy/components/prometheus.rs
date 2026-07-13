// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Typed sugar for prometheus.* components.
//!
//! Mirrors the per-component schemas in `schemas/alloy/prometheus.schema.yaml`.
//! Each block deserializes from the flat `{prometheus.X: {label, attrs..., blocks}}`
//! form and converts to a generic [`Block`] via [`ToBlock`].
//!
//! Reuses shared machinery: [`MetricsReceiver`] (the metrics analog of
//! `LogsReceiver`) for `forward_to`, [`TargetEntry`]/[`target_list`] for
//! `prometheus.scrape` `targets`, and [`RelabelRule`]/[`RelabelSubBlock`]
//! (`components/relabel.rs`) for the `rule` blocks shared with the loki side.

use crate::alloy::ast::{
    AttributeValue, Block, Expressable, GoDuration, Identifier, ToBlock, impl_to_block_dispatch,
    string_map,
};
use crate::alloy::components::capsule::{
    MetricsReceiver, TargetEntry, metrics_receiver_list, target_list,
};
use crate::alloy::components::relabel::{RelabelRule, RelabelSubBlock};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A nested-block list that only supports the `raw:` escape (no typed
/// sub-blocks yet). Used by `endpoint` (remote_write) and `selector`
/// (operator), where the surrounding block's scalars are typed but its nested
/// blocks are deferred to raw.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RawOnlySubBlock {
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(RawOnlySubBlock { Raw });

/// Collect a `Vec` of `ToBlock` sub-blocks into rendered `Block`s.
fn to_blocks<T: ToBlock>(blocks: &[T]) -> Result<Vec<Block>> {
    blocks.iter().map(ToBlock::to_block).collect()
}

// ============================================================
// prometheus.echo
// ============================================================

/// A `prometheus.echo` block — prints incoming samples to stdout for debugging.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.echo/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusEchoBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Output encoding: `text` (default) or `openmetrics`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl ToBlock for PrometheusEchoBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(format) = &self.format {
            attributes.insert("format".into(), AttributeValue::String(format.clone()));
        }
        Ok(Block {
            component: "prometheus.echo".into(),
            label: self.label.clone(),
            attributes,
            ..Default::default()
        })
    }
}

// ============================================================
// prometheus.relabel
// ============================================================

/// A `prometheus.relabel` block — rewrites metric labels via `rule` sub-blocks
/// before forwarding downstream.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.relabel/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusRelabelBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Metrics receivers to forward relabeled samples to. Required by the schema.
    pub forward_to: Vec<MetricsReceiver>,
    /// Maximum number of entries in the relabeling result cache. Defaults to 100,000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cache_size: Option<f64>,
    /// How long a relabeling result stays cached. Defaults to `0` (no expiry).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<GoDuration>,
    /// `rule` sub-blocks applied in document order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<RelabelSubBlock>,
}

impl ToBlock for PrometheusRelabelBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("forward_to".into(), metrics_receiver_list(&self.forward_to));
        if let Some(mc) = self.max_cache_size {
            attributes.insert("max_cache_size".into(), AttributeValue::Number(mc));
        }
        if let Some(ttl) = &self.cache_ttl {
            attributes.insert("cache_ttl".into(), AttributeValue::String(ttl.clone()));
        }
        Ok(Block {
            component: "prometheus.relabel".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

// ============================================================
// prometheus.scrape  (+ basic_auth / tls_config / clustering sub-blocks)
// ============================================================

/// A `prometheus.scrape` block — scrapes metrics from `targets` and forwards them.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.scrape/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusScrapeBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Targets to scrape (label maps, or a ref to another component's targets
    /// export). Required by the schema.
    pub targets: Vec<TargetEntry>,
    /// Metrics receivers to forward scraped samples to. Required by the schema.
    pub forward_to: Vec<MetricsReceiver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_interval: Option<GoDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_timeout: Option<GoDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_labels: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_timestamps: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_compression: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_http2: Option<bool>,
    /// Bearer token for target auth (a secret; often a `sys.env(...)` expression).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<Expressable<String>>,
    /// Path to a file containing a bearer token for target auth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token_file: Option<String>,
    /// Optional nested blocks (`basic_auth`, `tls_config`, `clustering`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<ScrapeSubBlock>,
}

impl ToBlock for PrometheusScrapeBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("targets".into(), target_list(&self.targets));
        attributes.insert("forward_to".into(), metrics_receiver_list(&self.forward_to));
        if let Some(v) = &self.scrape_interval {
            attributes.insert("scrape_interval".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.scrape_timeout {
            attributes.insert("scrape_timeout".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.metrics_path {
            attributes.insert("metrics_path".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.scheme {
            attributes.insert("scheme".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.job_name {
            attributes.insert("job_name".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = self.honor_labels {
            attributes.insert("honor_labels".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.honor_timestamps {
            attributes.insert("honor_timestamps".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.follow_redirects {
            attributes.insert("follow_redirects".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.enable_compression {
            attributes.insert("enable_compression".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.enable_http2 {
            attributes.insert("enable_http2".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = &self.bearer_token {
            attributes.insert("bearer_token".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.bearer_token_file {
            attributes.insert(
                "bearer_token_file".into(),
                AttributeValue::String(v.clone()),
            );
        }
        Ok(Block {
            component: "prometheus.scrape".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// Sub-block under a `prometheus.scrape` body. `Raw` is the escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScrapeSubBlock {
    #[serde(rename = "basic_auth")]
    BasicAuth(BasicAuthBlock),
    #[serde(rename = "tls_config")]
    TlsConfig(TlsConfigBlock),
    #[serde(rename = "clustering")]
    Clustering(ClusteringBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(ScrapeSubBlock {
    BasicAuth,
    TlsConfig,
    Clustering,
    Raw
});

// ============================================================
// prometheus.receive_http  (+ http sub-block)
// ============================================================

/// A `prometheus.receive_http` block — serves a Prometheus remote-write
/// endpoint and forwards received samples downstream.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.receive_http/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusReceiveHttpBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Metrics receivers to forward received samples to. Required by the schema.
    pub forward_to: Vec<MetricsReceiver>,
    /// Optional nested blocks (`http`; a `tls` block uses `raw:`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<ReceiveHttpSubBlock>,
}

impl ToBlock for PrometheusReceiveHttpBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("forward_to".into(), metrics_receiver_list(&self.forward_to));
        Ok(Block {
            component: "prometheus.receive_http".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// Sub-block under a `prometheus.receive_http` body. `Raw` is the escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiveHttpSubBlock {
    #[serde(rename = "http")]
    Http(HttpServerBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(ReceiveHttpSubBlock { Http, Raw });

/// An `http` sub-block — configures the HTTP server `prometheus.receive_http` runs.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.receive_http/#http-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpServerBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conn_limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_idle_timeout: Option<GoDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_read_timeout: Option<GoDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_write_timeout: Option<GoDuration>,
}

impl ToBlock for HttpServerBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.listen_address {
            attributes.insert("listen_address".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = self.listen_port {
            attributes.insert("listen_port".into(), AttributeValue::Number(v));
        }
        if let Some(v) = self.conn_limit {
            attributes.insert("conn_limit".into(), AttributeValue::Number(v));
        }
        if let Some(v) = &self.server_idle_timeout {
            attributes.insert(
                "server_idle_timeout".into(),
                AttributeValue::String(v.clone()),
            );
        }
        if let Some(v) = &self.server_read_timeout {
            attributes.insert(
                "server_read_timeout".into(),
                AttributeValue::String(v.clone()),
            );
        }
        if let Some(v) = &self.server_write_timeout {
            attributes.insert(
                "server_write_timeout".into(),
                AttributeValue::String(v.clone()),
            );
        }
        Ok(Block {
            component: "http".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

// ============================================================
// prometheus.remote_write  (+ endpoint sub-block)
// ============================================================

/// A `prometheus.remote_write` block — delivers metrics to remote-write endpoints.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.remote_write/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusRemoteWriteBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Labels added to every metric before it is sent to the endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_labels: Option<IndexMap<String, String>>,
    /// `endpoint` sub-blocks describing where to send metrics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<RemoteWriteSubBlock>,
}

impl ToBlock for PrometheusRemoteWriteBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(labels) = &self.external_labels {
            attributes.insert("external_labels".into(), string_map(labels));
        }
        Ok(Block {
            component: "prometheus.remote_write".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// Sub-block under a `prometheus.remote_write` body. `Raw` is the escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteWriteSubBlock {
    #[serde(rename = "endpoint")]
    Endpoint(RemoteWriteEndpointBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(RemoteWriteSubBlock { Endpoint, Raw });

/// An `endpoint` sub-block — one remote-write destination.
///
/// Auth (`basic_auth`, `tls_config`), `queue_config`, and `write_relabel_config`
/// are reachable via a `raw:` block in `blocks` — no need to rawify the whole
/// endpoint just to add auth.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.remote_write/#endpoint-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteWriteEndpointBlock {
    /// Full URL of the remote-write endpoint. Required by the schema.
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_timeout: Option<GoDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_exemplars: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_native_histograms: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_http2: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
    /// Nested endpoint blocks (`basic_auth`, `tls_config`, `queue_config`, ...)
    /// via the `raw:` escape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<RawOnlySubBlock>,
}

impl ToBlock for RemoteWriteEndpointBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("url".into(), AttributeValue::String(self.url.clone()));
        if let Some(v) = &self.name {
            attributes.insert("name".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.remote_timeout {
            attributes.insert("remote_timeout".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.headers {
            attributes.insert("headers".into(), string_map(v));
        }
        if let Some(v) = self.send_exemplars {
            attributes.insert("send_exemplars".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.send_native_histograms {
            attributes.insert("send_native_histograms".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.enable_http2 {
            attributes.insert("enable_http2".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.follow_redirects {
            attributes.insert("follow_redirects".into(), AttributeValue::Bool(v));
        }
        Ok(Block {
            component: "endpoint".into(),
            label: None,
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

// ============================================================
// prometheus.operator.podmonitors / .servicemonitors
// (+ shared operator sub-blocks)
// ============================================================

/// A `prometheus.operator.podmonitors` block — discovers PodMonitors and scrapes
/// the pods they select.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.podmonitors/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusOperatorPodMonitorsBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Metrics receivers to forward scraped samples to. Required by the schema.
    pub forward_to: Vec<MetricsReceiver>,
    /// Namespaces to search for PodMonitors in. Defaults to all namespaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub namespaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub informer_sync_timeout: Option<GoDuration>,
    /// Optional nested blocks (`clustering`, `selector`, `scrape`, `rule`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<OperatorSubBlock>,
}

impl ToBlock for PrometheusOperatorPodMonitorsBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("forward_to".into(), metrics_receiver_list(&self.forward_to));
        if !self.namespaces.is_empty() {
            attributes.insert("namespaces".into(), string_array(&self.namespaces));
        }
        if let Some(v) = &self.informer_sync_timeout {
            attributes.insert(
                "informer_sync_timeout".into(),
                AttributeValue::String(v.clone()),
            );
        }
        Ok(Block {
            component: "prometheus.operator.podmonitors".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// A `prometheus.operator.servicemonitors` block — discovers ServiceMonitors and
/// scrapes the endpoints they select.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.servicemonitors/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusOperatorServiceMonitorsBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Metrics receivers to forward scraped samples to. Required by the schema.
    pub forward_to: Vec<MetricsReceiver>,
    /// Namespaces to search for ServiceMonitors in. Defaults to all namespaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub namespaces: Vec<String>,
    /// Kubernetes role used to discover targets. Defaults to `endpoints`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kubernetes_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub informer_sync_timeout: Option<GoDuration>,
    /// Optional nested blocks (`clustering`, `selector`, `scrape`, `rule`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<OperatorSubBlock>,
}

impl ToBlock for PrometheusOperatorServiceMonitorsBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("forward_to".into(), metrics_receiver_list(&self.forward_to));
        if !self.namespaces.is_empty() {
            attributes.insert("namespaces".into(), string_array(&self.namespaces));
        }
        if let Some(v) = &self.kubernetes_role {
            attributes.insert("kubernetes_role".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.informer_sync_timeout {
            attributes.insert(
                "informer_sync_timeout".into(),
                AttributeValue::String(v.clone()),
            );
        }
        Ok(Block {
            component: "prometheus.operator.servicemonitors".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// Sub-block under a `prometheus.operator.*` body. `Raw` is the escape hatch
/// (notably for the `client` Kubernetes API block). The `rule` variant reuses
/// the shared [`RelabelRule`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatorSubBlock {
    #[serde(rename = "clustering")]
    Clustering(ClusteringBlock),
    #[serde(rename = "selector")]
    Selector(OperatorSelectorBlock),
    #[serde(rename = "scrape")]
    Scrape(OperatorScrapeBlock),
    #[serde(rename = "rule")]
    Rule(RelabelRule),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(OperatorSubBlock {
    Clustering,
    Selector,
    Scrape,
    Rule,
    Raw
});

/// A `selector` sub-block — restricts which PodMonitor/ServiceMonitor resources
/// the operator component picks up. `match_expression` blocks use `raw:`.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.podmonitors/#selector-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSelectorBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_labels: Option<IndexMap<String, String>>,
    /// `match_expression` blocks via the `raw:` escape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<RawOnlySubBlock>,
}

impl ToBlock for OperatorSelectorBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.match_labels {
            attributes.insert("match_labels".into(), string_map(v));
        }
        Ok(Block {
            component: "selector".into(),
            label: None,
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// A `scrape` sub-block — default scrape settings for operator-discovered targets.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.podmonitors/#scrape-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorScrapeBlock {
    /// Default scrape interval; a literal Go duration or an expression
    /// (e.g. `{env: METRICS_SCRAPE_INTERVAL}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_scrape_interval: Option<Expressable<String>>,
    /// Default scrape timeout; a literal Go duration or an expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_scrape_timeout: Option<Expressable<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sample_limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_metadata: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_native_histograms: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_type_and_unit_labels: Option<bool>,
}

impl ToBlock for OperatorScrapeBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.default_scrape_interval {
            attributes.insert("default_scrape_interval".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.default_scrape_timeout {
            attributes.insert("default_scrape_timeout".into(), v.to_attribute_value()?);
        }
        if let Some(v) = self.default_sample_limit {
            attributes.insert("default_sample_limit".into(), AttributeValue::Number(v));
        }
        if let Some(v) = self.honor_metadata {
            attributes.insert("honor_metadata".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.scrape_native_histograms {
            attributes.insert("scrape_native_histograms".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.enable_type_and_unit_labels {
            attributes.insert(
                "enable_type_and_unit_labels".into(),
                AttributeValue::Bool(v),
            );
        }
        Ok(Block {
            component: "scrape".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

// ============================================================
// Shared blocks: clustering / basic_auth / tls_config
// ============================================================

/// A `clustering` sub-block — distributes scrape targets across an Alloy cluster.
/// Shared by `prometheus.scrape` and the operator components.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.scrape/#clustering-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusteringBlock {
    pub enabled: bool,
}

impl ToBlock for ClusteringBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("enabled".into(), AttributeValue::Bool(self.enabled));
        Ok(Block {
            component: "clustering".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// A `basic_auth` sub-block — HTTP Basic authentication for scrape requests.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.scrape/#basic_auth-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasicAuthBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Password (a secret; often a `sys.env(...)` expression).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<Expressable<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_file: Option<String>,
}

impl ToBlock for BasicAuthBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.username {
            attributes.insert("username".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.password {
            attributes.insert("password".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.password_file {
            attributes.insert("password_file".into(), AttributeValue::String(v.clone()));
        }
        Ok(Block {
            component: "basic_auth".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// A `tls_config` sub-block — TLS settings for the scrape connection.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.scrape/#tls_config-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfigBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_pem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_pem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_pem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insecure_skip_verify: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
}

impl ToBlock for TlsConfigBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        for (key, val) in [
            ("ca_file", &self.ca_file),
            ("ca_pem", &self.ca_pem),
            ("cert_file", &self.cert_file),
            ("cert_pem", &self.cert_pem),
            ("key_file", &self.key_file),
            ("key_pem", &self.key_pem),
            ("server_name", &self.server_name),
            ("min_version", &self.min_version),
        ] {
            if let Some(v) = val {
                attributes.insert(key.into(), AttributeValue::String(v.clone()));
            }
        }
        if let Some(v) = self.insecure_skip_verify {
            attributes.insert("insecure_skip_verify".into(), AttributeValue::Bool(v));
        }
        Ok(Block {
            component: "tls_config".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// Convert a `Vec<String>` to an `AttributeValue::Array` of string literals.
fn string_array(values: &[String]) -> AttributeValue {
    AttributeValue::Array(
        values
            .iter()
            .map(|s| AttributeValue::String(s.clone()))
            .collect(),
    )
}

// ============================================================
// tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::alloy::error::Error;
    use crate::alloy::pipeline::Pipeline;
    use crate::alloy::test_support::assert_renders;

    #[test]
    fn prometheus_echo_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.echo:
                  label: debug
                  format: openmetrics
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.echo \"debug\" {\n",
                "\tformat = \"openmetrics\"\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn prometheus_relabel_round_trips() {
        // `forward_to` renders as a bare ref (not quoted); the single trailing
        // scalar `max_cache_size` sits beside it canonically; a shared `rule`
        // sub-block renders like the loki side.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.relabel:
                  label: metrics
                  forward_to: ["prometheus.remote_write.default.receiver"]
                  max_cache_size: 100000
                  blocks:
                    - rule:
                        action: labeldrop
                        regex: "__meta_.*"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.relabel \"metrics\" {\n",
                "\tforward_to = [\n",
                "\t\tprometheus.remote_write.default.receiver,\n",
                "\t]\n",
                "\tmax_cache_size = 100000\n",
                "\n",
                "\trule {\n",
                "\t\taction = \"labeldrop\"\n",
                "\t\tregex  = \"__meta_.*\"\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn prometheus_scrape_with_auth_and_tls_sub_blocks() {
        // A realistic kubelet scrape: a single list-valued `targets` ref assigned
        // directly (not `[…]`-wrapped), a bare-ref `forward_to`, and typed
        // `tls_config` + `clustering` sub-blocks. Byte-checked with `assert_eq!`
        // rather than `assert_renders`: the trailing single-line scalars beside
        // the multi-line `forward_to` array hit the renderer's known alignment
        // divergence (alloy fmt aligns their `=`); the output is valid alloy,
        // just not fmt-canonical.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.scrape:
                  label: kubelet
                  targets: ["discovery.relabel.kubelet.output"]
                  forward_to: ["prometheus.relabel.metrics.receiver"]
                  scrape_interval: "30s"
                  metrics_path: /metrics/cadvisor
                  scheme: https
                  honor_labels: true
                  bearer_token_file: /var/run/secrets/kubernetes.io/serviceaccount/token
                  blocks:
                    - tls_config:
                        insecure_skip_verify: true
                    - clustering:
                        enabled: true
            "#,
        )
        .unwrap();
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "prometheus.scrape \"kubelet\" {\n",
                "\ttargets = discovery.relabel.kubelet.output\n",
                "\tforward_to = [\n",
                "\t\tprometheus.relabel.metrics.receiver,\n",
                "\t]\n",
                "\tscrape_interval = \"30s\"\n",
                "\tmetrics_path = \"/metrics/cadvisor\"\n",
                "\tscheme = \"https\"\n",
                "\thonor_labels = true\n",
                "\tbearer_token_file = \"/var/run/secrets/kubernetes.io/serviceaccount/token\"\n",
                "\n",
                "\ttls_config {\n",
                "\t\tinsecure_skip_verify = true\n",
                "\t}\n",
                "\n",
                "\tclustering {\n",
                "\t\tenabled = true\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn prometheus_receive_http_with_http_block() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.receive_http:
                  label: ingest
                  forward_to: ["prometheus.relabel.metrics.receiver"]
                  blocks:
                    - http:
                        listen_address: 0.0.0.0
                        listen_port: 9090
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.receive_http \"ingest\" {\n",
                "\tforward_to = [\n",
                "\t\tprometheus.relabel.metrics.receiver,\n",
                "\t]\n",
                "\n",
                "\thttp {\n",
                "\t\tlisten_address = \"0.0.0.0\"\n",
                "\t\tlisten_port    = 9090\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn prometheus_remote_write_typed_endpoint_with_raw_auth() {
        // The auth-via-raw property: a typed `endpoint` (url + scalars) carries a
        // `raw:` `basic_auth` sub-block, so auth is reachable WITHOUT rawifying
        // the whole endpoint. `external_labels` renders as an object literal.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.remote_write:
                  label: default
                  external_labels:
                    cluster: example_cluster
                  blocks:
                    - endpoint:
                        url: http://mimir:9009/api/v1/push
                        remote_timeout: "30s"
                        blocks:
                          - raw:
                              component: basic_auth
                              attributes:
                                username: mimir
                                password: { env: MIMIR_PASSWORD }
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.remote_write \"default\" {\n",
                "\texternal_labels = {\n",
                "\t\tcluster = \"example_cluster\",\n",
                "\t}\n",
                "\n",
                "\tendpoint {\n",
                "\t\turl            = \"http://mimir:9009/api/v1/push\"\n",
                "\t\tremote_timeout = \"30s\"\n",
                "\n",
                "\t\tbasic_auth {\n",
                "\t\t\tusername = \"mimir\"\n",
                "\t\t\tpassword = sys.env(\"MIMIR_PASSWORD\")\n",
                "\t\t}\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn prometheus_operator_podmonitors_with_all_sub_blocks() {
        // Exercises every typed operator sub-block in one body: selector
        // (match_labels), scrape (defaults), clustering, and a shared `rule`.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.operator.podmonitors:
                  label: pods
                  forward_to: ["prometheus.relabel.metrics.receiver"]
                  namespaces: ["mz-system", "mz-environment"]
                  informer_sync_timeout: "1m"
                  blocks:
                    - selector:
                        match_labels:
                          team: storage
                    - scrape:
                        default_scrape_interval: "60s"
                        default_scrape_timeout: "10s"
                    - clustering:
                        enabled: true
                    - rule:
                        action: keep
                        regex: "alloy"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.operator.podmonitors \"pods\" {\n",
                "\tforward_to = [\n",
                "\t\tprometheus.relabel.metrics.receiver,\n",
                "\t]\n",
                "\tnamespaces = [\n",
                "\t\t\"mz-system\",\n",
                "\t\t\"mz-environment\",\n",
                "\t]\n",
                "\tinformer_sync_timeout = \"1m\"\n",
                "\n",
                "\tselector {\n",
                "\t\tmatch_labels = {\n",
                "\t\t\tteam = \"storage\",\n",
                "\t\t}\n",
                "\t}\n",
                "\n",
                "\tscrape {\n",
                "\t\tdefault_scrape_interval = \"60s\"\n",
                "\t\tdefault_scrape_timeout  = \"10s\"\n",
                "\t}\n",
                "\n",
                "\tclustering {\n",
                "\t\tenabled = true\n",
                "\t}\n",
                "\n",
                "\trule {\n",
                "\t\taction = \"keep\"\n",
                "\t\tregex  = \"alloy\"\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn operator_scrape_defaults_accept_env_expression() {
        // `default_scrape_interval` / `default_scrape_timeout` are `Expressable`:
        // here the interval is wired to an env var while the timeout stays a
        // literal duration. Both render side by side.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.operator.podmonitors:
                  forward_to: ["prometheus.remote_write.default.receiver"]
                  blocks:
                    - scrape:
                        default_scrape_interval: {env: METRICS_SCRAPE_INTERVAL}
                        default_scrape_timeout: "10s"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.operator.podmonitors {\n",
                "\tforward_to = [\n",
                "\t\tprometheus.remote_write.default.receiver,\n",
                "\t]\n",
                "\n",
                "\tscrape {\n",
                "\t\tdefault_scrape_interval = sys.env(\"METRICS_SCRAPE_INTERVAL\")\n",
                "\t\tdefault_scrape_timeout  = \"10s\"\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn prometheus_operator_servicemonitors_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.operator.servicemonitors:
                  label: svcs
                  forward_to: ["prometheus.relabel.metrics.receiver"]
                  kubernetes_role: endpoints
                  blocks:
                    - clustering:
                        enabled: true
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.operator.servicemonitors \"svcs\" {\n",
                "\tforward_to = [\n",
                "\t\tprometheus.relabel.metrics.receiver,\n",
                "\t]\n",
                "\tkubernetes_role = \"endpoints\"\n",
                "\n",
                "\tclustering {\n",
                "\t\tenabled = true\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn unknown_attribute_on_prometheus_scrape_is_rejected_by_schema() {
        // Typed blocks are strict: an undocumented attribute must use `raw:`.
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.scrape:
                  targets: []
                  forward_to: []
                  mystery_attr: 42
            "#,
        )
        .unwrap_err();
        let paths: Vec<String> = match err {
            Error::Multiple(errs) => errs
                .iter()
                .filter_map(|e| match e {
                    Error::Schema { path, .. } => Some(path.clone()),
                    _ => None,
                })
                .collect(),
            other => panic!("expected Multiple([Schema, ...]), got {other:?}"),
        };
        assert!(
            !paths.is_empty() && paths.iter().any(|p| p.starts_with("/blocks/0")),
            "expected a /blocks/0 schema violation, got {paths:?}"
        );
    }
}
