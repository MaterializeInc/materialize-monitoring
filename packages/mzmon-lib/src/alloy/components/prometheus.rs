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
    AttributeValue, Block, Expressable, ExpressableList, GoDuration, Identifier, RawOnlySubBlock,
    ToBlock, expressable_string_map, impl_to_block_dispatch, string_map,
};
use crate::alloy::components::capsule::{
    MetricsReceiver, TargetEntry, metrics_receiver_list, target_list,
};
use crate::alloy::components::relabel::{RelabelRule, RelabelSubBlock};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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
    /// `Expressable` so the interval can come from the environment; it is the
    /// dominant cost lever on a high-cardinality target like a per-node cAdvisor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_interval: Option<Expressable<GoDuration>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrape_timeout: Option<Expressable<GoDuration>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_labels: Option<bool>,
    /// `Expressable` so a `declare` body can take it as an argument: a pull
    /// whose points are hours apart is stamped at the scrape instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honor_timestamps: Option<Expressable<bool>>,
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
            attributes.insert("scrape_interval".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.scrape_timeout {
            attributes.insert("scrape_timeout".into(), v.to_attribute_value()?);
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
        if let Some(v) = &self.honor_timestamps {
            attributes.insert("honor_timestamps".into(), v.to_attribute_value()?);
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
    /// `Expressable`, so it can be wired to an environment variable with an
    /// in-cluster default.
    pub url: Expressable<String>,
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
        attributes.insert("url".into(), self.url.to_attribute_value()?);
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
    /// `Expressable`: kubelet serving certs verify against the cluster CA on
    /// some distributions and not others, and getting it wrong stops collection
    /// silently — so it needs to be settable per deployment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insecure_skip_verify: Option<Expressable<bool>>,
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
        if let Some(v) = &self.insecure_skip_verify {
            attributes.insert("insecure_skip_verify".into(), v.to_attribute_value()?);
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
// prometheus.exporter.cadvisor
// ============================================================

/// A `prometheus.exporter.cadvisor` block — runs cAdvisor in-process and
/// exports per-container resource metrics as scrape targets.
///
/// Two things about the collector arguments are easy to get wrong and are worth
/// knowing before setting them:
///
/// * A **non-empty** `enabled_metrics` replaces `disabled_metrics` outright
///   rather than merging per collector. It is the exhaustive list, not extras on
///   top. Empty means "unset", which leaves `disabled_metrics` in charge.
/// * `allowlisted_container_labels` is honored **only** when
///   `store_container_labels` is false. Leaving the latter at its `true` default
///   promotes every container label to a metric label, which in Kubernetes is
///   every pod label on every container series.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.cadvisor/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterCadvisorBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Collectors to enable. A non-empty list overrides `disabled_metrics`
    /// entirely. `ExpressableList`, so it can come from an environment variable
    /// via `encoding.from_json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_metrics: Option<ExpressableList>,
    /// Collectors to disable, overriding cAdvisor's own default-disabled set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_metrics: Option<ExpressableList>,
    /// Promote every container label and env var to metric labels. Defaults to
    /// true upstream; false is almost always what you want in Kubernetes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_container_labels: Option<bool>,
    /// Container labels to promote. Only honored when `store_container_labels`
    /// is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowlisted_container_labels: Option<ExpressableList>,
    /// Environment-variable prefixes to collect for containers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_metadata_allowlist: Option<ExpressableList>,
    /// cgroup path prefixes to collect even under `docker_only`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_cgroup_prefix_allowlist: Option<ExpressableList>,
    /// Report only Docker containers plus root stats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docker_only: Option<bool>,
    /// Skip the root cgroup's own stats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_root_cgroup_stats: Option<bool>,
    /// Docker endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docker_host: Option<Expressable<String>>,
    /// containerd socket path. The default is `/run/containerd/containerd.sock`,
    /// which must be mounted into the pod or container-to-pod identity is lost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub containerd_host: Option<Expressable<String>>,
    /// containerd namespace to read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub containerd_namespace: Option<Expressable<String>>,
    /// How long to retain samples in memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_duration: Option<GoDuration>,
    /// Interval for refreshing resctrl monitoring groups.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resctrl_interval: Option<GoDuration>,
    /// Path to a perf-events configuration file, required by the `perf_event`
    /// collector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perf_events_config: Option<Expressable<String>>,
}

impl ToBlock for PrometheusExporterCadvisorBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        // Emitted in declaration order rather than grouped by type, so the
        // rendered block reads the way the arguments relate: the collector
        // lists together, then `store_container_labels` immediately before the
        // allowlist it gates.
        macro_rules! put_list {
            ($name:literal, $field:expr) => {
                if let Some(v) = &$field {
                    attributes.insert($name.into(), v.to_attribute_value()?);
                }
            };
        }
        macro_rules! put_bool {
            ($name:literal, $field:expr) => {
                if let Some(v) = $field {
                    attributes.insert($name.into(), AttributeValue::Bool(v));
                }
            };
        }
        macro_rules! put_str {
            ($name:literal, $field:expr) => {
                if let Some(v) = &$field {
                    attributes.insert($name.into(), v.to_attribute_value()?);
                }
            };
        }
        put_list!("enabled_metrics", self.enabled_metrics);
        put_list!("disabled_metrics", self.disabled_metrics);
        put_bool!("store_container_labels", self.store_container_labels);
        put_list!(
            "allowlisted_container_labels",
            self.allowlisted_container_labels
        );
        put_list!("env_metadata_allowlist", self.env_metadata_allowlist);
        put_list!(
            "raw_cgroup_prefix_allowlist",
            self.raw_cgroup_prefix_allowlist
        );
        put_bool!("docker_only", self.docker_only);
        put_bool!("disable_root_cgroup_stats", self.disable_root_cgroup_stats);
        put_str!("docker_host", self.docker_host);
        put_str!("containerd_host", self.containerd_host);
        put_str!("containerd_namespace", self.containerd_namespace);
        if let Some(v) = &self.storage_duration {
            attributes.insert("storage_duration".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.resctrl_interval {
            attributes.insert("resctrl_interval".into(), AttributeValue::String(v.clone()));
        }
        put_str!("perf_events_config", self.perf_events_config);
        Ok(Block {
            component: "prometheus.exporter.cadvisor".into(),
            label: self.label.clone(),
            attributes,
            blocks: Vec::new(),
        })
    }
}

// ============================================================
// prometheus.exporter.cloudwatch  (+ static / metric / role sub-blocks)
// ============================================================

/// A `prometheus.exporter.cloudwatch` block — pulls CloudWatch metrics through
/// the embedded YACE exporter and exports them as a scrape target.
///
/// Three behaviours decide how this is used on a clustered gateway:
///
/// * The AWS calls happen **on scrape**, so a clustered `prometheus.scrape`
///   that owns the target is the only replica that calls AWS. The
///   `decoupled_scraping` block (reachable only through `raw:`) polls on a
///   timer in **every** replica, owner or not.
/// * The exported target's `instance` is a hash of these arguments. It is the
///   same on every replica, which is what clustering needs, and it changes
///   whenever an argument does.
/// * `nil_to_zero` defaults to **true** in Alloy, which reports a value-less
///   datapoint as zero.
///
/// `custom_namespace` and `decoupled_scraping` are deferred to the `raw:`
/// escape.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.cloudwatch/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterCloudwatchBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Region for STS, which role assumption and the `account_id` label use.
    /// Required by the schema.
    pub sts_region: Expressable<String>,
    /// Defaults to true upstream, meaning FIPS endpoints are not used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fips_disabled: Option<bool>,
    /// Snake-case the `dimension_*` label names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels_snake_case: Option<bool>,
    /// Per discovery namespace, the tags copied onto that namespace's series as
    /// `tag_<key>`. Every discovery job also emits an `aws_<service>_info`
    /// series with all of a resource's tags, whatever this says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_exported_tags: Option<IndexMap<String, Vec<String>>>,
    /// `static` and `discovery` jobs, plus anything else via `raw:`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<CloudwatchSubBlock>,
}

impl ToBlock for PrometheusExporterCloudwatchBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("sts_region".into(), self.sts_region.to_attribute_value()?);
        if let Some(v) = self.fips_disabled {
            attributes.insert("fips_disabled".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.labels_snake_case {
            attributes.insert("labels_snake_case".into(), AttributeValue::Bool(v));
        }
        if let Some(tags) = &self.discovery_exported_tags {
            attributes.insert(
                "discovery_exported_tags".into(),
                AttributeValue::Object(
                    tags.iter()
                        .map(|(namespace, keys)| (namespace.clone(), string_array(keys)))
                        .collect(),
                ),
            );
        }
        Ok(Block {
            component: "prometheus.exporter.cloudwatch".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// Sub-block under a `prometheus.exporter.cloudwatch` body. `Raw` is the
/// escape hatch, and the way to reach `custom_namespace` and
/// `decoupled_scraping`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudwatchSubBlock {
    #[serde(rename = "static")]
    // Boxed: expression-capable `regions` and `dimensions` make it far wider
    // than `raw` (clippy::large_enum_variant).
    Static(Box<CloudwatchStaticBlock>),
    #[serde(rename = "discovery")]
    Discovery(Box<CloudwatchDiscoveryBlock>),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(CloudwatchSubBlock {
    Static,
    Discovery,
    Raw
});

/// A `static "<name>"` job — one resource, addressed by its exact dimensions.
///
/// Static jobs go through `GetMetricStatistics`. The label becomes the series'
/// `name` label and must be an identifier. `period` and `length` belong on each
/// `metric`: the exporter ignores them on the job.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.cloudwatch/#static-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchStaticBlock {
    /// The job name. Required, and written to every series as `name`.
    pub label: Identifier,
    /// Regions to query. Required by the schema. A list whose members may be
    /// expressions, so a `declare` body can pass `[argument.region.value]`.
    pub regions: ExpressableList,
    /// CloudWatch namespace, such as `AWS/RDS`. Required by the schema.
    pub namespace: String,
    /// The exact dimension set of the resource. Required by the schema. Values
    /// may be expressions, which is how a `declare` body names its resource.
    pub dimensions: IndexMap<String, Expressable<String>>,
    /// Extra labels, written as `custom_tag_<key>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_tags: Option<IndexMap<String, String>>,
    /// Report a value-less datapoint as zero. Defaults to true in Alloy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nil_to_zero: Option<bool>,
    /// `metric` and `role` blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<CloudwatchJobSubBlock>,
}

impl ToBlock for CloudwatchStaticBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("regions".into(), self.regions.to_attribute_value()?);
        attributes.insert(
            "namespace".into(),
            AttributeValue::String(self.namespace.clone()),
        );
        if let Some(v) = self.nil_to_zero {
            attributes.insert("nil_to_zero".into(), AttributeValue::Bool(v));
        }
        attributes.insert(
            "dimensions".into(),
            expressable_string_map(&self.dimensions)?,
        );
        if let Some(v) = &self.custom_tags {
            attributes.insert("custom_tags".into(), string_map(v));
        }
        Ok(Block {
            component: "static".into(),
            label: Some(self.label.clone()),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// A `discovery` job — every resource of a namespace whose tags match.
///
/// Resources come from the Resource Groups Tagging API, or from
/// `DescribeAutoScalingGroups` for `AWS/AutoScaling`, and their metrics from
/// `ListMetrics` and `GetMetricData`. A metric whose dimensions name no
/// discovered resource is kept as a `name="global"` series, which for
/// `AWS/EC2` is every Auto Scaling group's aggregate in the region;
/// `dimension_name_requirements` keeps those out.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.cloudwatch/#discovery-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchDiscoveryBlock {
    /// The namespace to discover, such as `AWS/EC2`. Required by the schema.
    #[serde(rename = "type")]
    pub namespace: String,
    /// Regions to search. Required by the schema.
    pub regions: ExpressableList,
    /// Tag key to unanchored value regex, all of which must match. Values may
    /// be expressions, which is how a `declare` body names its resources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_tags: Option<IndexMap<String, Expressable<String>>>,
    /// Extra labels, written as `custom_tag_<key>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_tags: Option<IndexMap<String, String>>,
    /// Keep only metrics whose dimension names are exactly these.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_name_requirements: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recently_active_only: Option<bool>,
    /// Report a value-less datapoint as zero. Defaults to true in Alloy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nil_to_zero: Option<bool>,
    /// `metric` and `role` blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<CloudwatchJobSubBlock>,
}

impl ToBlock for CloudwatchDiscoveryBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "type".into(),
            AttributeValue::String(self.namespace.clone()),
        );
        attributes.insert("regions".into(), self.regions.to_attribute_value()?);
        if let Some(v) = &self.search_tags {
            attributes.insert("search_tags".into(), expressable_string_map(v)?);
        }
        if let Some(v) = &self.custom_tags {
            attributes.insert("custom_tags".into(), string_map(v));
        }
        if let Some(v) = &self.dimension_name_requirements {
            attributes.insert("dimension_name_requirements".into(), string_array(v));
        }
        if let Some(v) = self.recently_active_only {
            attributes.insert("recently_active_only".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.nil_to_zero {
            attributes.insert("nil_to_zero".into(), AttributeValue::Bool(v));
        }
        Ok(Block {
            component: "discovery".into(),
            label: None,
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// Sub-block under a CloudWatch job. `Raw` is the escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudwatchJobSubBlock {
    #[serde(rename = "metric")]
    Metric(CloudwatchMetricBlock),
    #[serde(rename = "role")]
    Role(CloudwatchRoleBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(CloudwatchJobSubBlock { Metric, Role, Raw });

/// A `metric` block — one CloudWatch metric and the statistics to request.
///
/// Each statistic is a separate billed series. On a `static` job the list needs
/// at least one of `Average`, `Minimum`, `Maximum`, `Sum` or `SampleCount`
/// beside any percentiles, because the exporter indexes that list
/// unconditionally.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.cloudwatch/#metric-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchMetricBlock {
    /// The CloudWatch metric name. Required by the schema.
    pub name: String,
    /// Statistics to request. Required by the schema.
    pub statistics: Vec<String>,
    /// Statistic period. Defaults to 300s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<GoDuration>,
    /// How far back to request. Defaults to `period`, and must not be shorter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<GoDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nil_to_zero: Option<bool>,
    /// Stamp samples with CloudWatch's own timestamp rather than the scrape's.
    /// A daily metric stamped a day back is older than a remote-write receiver
    /// will accept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_cloudwatch_timestamp: Option<bool>,
}

impl ToBlock for CloudwatchMetricBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("name".into(), AttributeValue::String(self.name.clone()));
        attributes.insert("statistics".into(), string_array(&self.statistics));
        if let Some(v) = &self.period {
            attributes.insert("period".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.length {
            attributes.insert("length".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = self.nil_to_zero {
            attributes.insert("nil_to_zero".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.add_cloudwatch_timestamp {
            attributes.insert("add_cloudwatch_timestamp".into(), AttributeValue::Bool(v));
        }
        Ok(Block {
            component: "metric".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// A `role` block — an IAM role to assume for the job. Without one the job
/// uses the ambient AWS credential chain (IRSA, EKS Pod Identity, static keys).
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.cloudwatch/#role-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchRoleBlock {
    /// Required by the schema.
    pub role_arn: Expressable<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<Expressable<String>>,
}

impl ToBlock for CloudwatchRoleBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("role_arn".into(), self.role_arn.to_attribute_value()?);
        if let Some(v) = &self.external_id {
            attributes.insert("external_id".into(), v.to_attribute_value()?);
        }
        Ok(Block {
            component: "role".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

// ============================================================
// prometheus.exporter.gcp
// ============================================================

/// A `prometheus.exporter.gcp` block — pulls Cloud Monitoring time series
/// through the embedded `stackdriver_exporter` and exports them as a scrape
/// target.
///
/// Four behaviours decide how this is used:
///
/// * Each entry in `metrics_prefixes` is a **prefix**, so
///   `.../postgresql/num_backends` also pulls `num_backends_by_state`.
/// * An `extra_filters` entry is `<prefix>:<filter>`, split on the first colon,
///   and ANDed onto every metric type under that prefix. It must be a prefix of
///   some `metrics_prefixes` entry, or the config is refused.
/// * Samples carry Cloud Monitoring's own timestamps, which are minutes old.
/// * DELTA metrics become counters built from the newest point of each pull
///   only, so a DELTA sampled more often than the scrape under-counts.
///
/// Calls happen on scrape, like the CloudWatch exporter's.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.gcp/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterGcpBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Projects to read. Required by the schema.
    pub project_ids: ExpressableList,
    /// Metric-type prefixes to pull. Required by the schema.
    pub metrics_prefixes: ExpressableList,
    /// `<prefix>:<filter>` entries narrowing what each prefix pulls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_filters: Option<ExpressableList>,
    /// How far back each pull reaches. Defaults to 5m. `Expressable`, so a
    /// `declare` body can take it as an argument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_interval: Option<Expressable<GoDuration>>,
    /// Shift the window back by this much. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_offset: Option<GoDuration>,
    /// Shift the window back by each metric type's published ingest delay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest_delay: Option<bool>,
    /// Drop series from projects other than the one being read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_delegated_projects: Option<bool>,
    /// HTTP timeout for the Cloud Monitoring client. Defaults to 15s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gcp_client_timeout: Option<GoDuration>,
}

impl ToBlock for PrometheusExporterGcpBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("project_ids".into(), self.project_ids.to_attribute_value()?);
        attributes.insert(
            "metrics_prefixes".into(),
            self.metrics_prefixes.to_attribute_value()?,
        );
        if let Some(v) = &self.extra_filters {
            attributes.insert("extra_filters".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.request_interval {
            attributes.insert("request_interval".into(), v.to_attribute_value()?);
        }
        for (name, value) in [
            ("request_offset", &self.request_offset),
            ("gcp_client_timeout", &self.gcp_client_timeout),
        ] {
            if let Some(v) = value {
                attributes.insert(name.into(), AttributeValue::String(v.clone()));
            }
        }
        if let Some(v) = self.ingest_delay {
            attributes.insert("ingest_delay".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = self.drop_delegated_projects {
            attributes.insert("drop_delegated_projects".into(), AttributeValue::Bool(v));
        }
        Ok(Block {
            component: "prometheus.exporter.gcp".into(),
            label: self.label.clone(),
            attributes,
            blocks: Vec::new(),
        })
    }
}

// ============================================================
// prometheus.exporter.azure
// ============================================================

/// A `prometheus.exporter.azure` block — pulls Azure Monitor metrics through
/// the embedded `azure-metrics-exporter` and exports them as a scrape target.
///
/// Four behaviours decide how this is used:
///
/// * Each scrape runs one Resource Graph query for `resource_type`, narrowed by
///   `resource_graph_query_filter`, then one metrics call per resource per
///   twenty metrics. Without a filter it pulls every resource of that type the
///   identity can read.
/// * `metric_aggregations` applies to every metric in the block. A metric that
///   needs a different aggregation from the others gets it by asking for both
///   and dropping the unwanted series downstream.
/// * Only the newest datapoint of `timespan` with a value is kept, at
///   `interval` granularity. The window ends at the scrape, so that datapoint
///   is usually a partial bucket. Samples carry the scrape's timestamp.
/// * `included_resource_tags` defaults to `["owner"]`, which turns a resource
///   tag into a label on every series.
///
/// Calls happen on scrape, like the CloudWatch exporter's. The credential is
/// resolved on the first call, not when the component is built.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.exporter.azure/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterAzureBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Subscriptions to query. Required by the schema.
    pub subscriptions: ExpressableList,
    /// The Resource Graph resource type, such as
    /// `Microsoft.DBforPostgreSQL/flexibleServers`. Required by the schema.
    pub resource_type: String,
    /// Azure Monitor metric names, up to twenty per call. Required by the schema.
    pub metrics: ExpressableList,
    /// A Kusto clause appended to the Resource Graph query after a `|`.
    /// `Expressable`, so a `declare` body can build it from an argument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_graph_query_filter: Option<Expressable<String>>,
    /// Aggregations to request for every metric: `average`, `minimum`,
    /// `maximum`, `total` or `count`. Defaults to each metric's primary one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_aggregations: Option<ExpressableList>,
    /// ISO 8601 window each pull reads, ending at the scrape. Defaults to `PT5M`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timespan: Option<String>,
    /// ISO 8601 datapoint granularity. Defaults to `PT1M`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Sub-namespace for resource types with several, such as
    /// `Microsoft.Storage/storageAccounts/blobServices`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_namespace: Option<String>,
    /// Dimensions to split each metric by. Every one applies to every metric.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_dimensions: Option<ExpressableList>,
    /// Resource tags to copy onto every series. Defaults to `["owner"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub included_resource_tags: Option<ExpressableList>,
    /// Regions to read at subscription scope. Cannot be combined with
    /// `resource_graph_query_filter`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regions: Option<ExpressableList>,
    /// `azurecloud`, `azurechinacloud` or `azureusgovernmentcloud`. Defaults to
    /// `azurecloud`. `Expressable`, so a `declare` body can take it as an
    /// argument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_cloud_environment: Option<Expressable<String>>,
    /// Refuse a dimension a metric does not have. Defaults to false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validate_dimensions: Option<bool>,
}

impl ToBlock for PrometheusExporterAzureBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "subscriptions".into(),
            self.subscriptions.to_attribute_value()?,
        );
        attributes.insert(
            "resource_type".into(),
            AttributeValue::String(self.resource_type.clone()),
        );
        attributes.insert("metrics".into(), self.metrics.to_attribute_value()?);
        if let Some(v) = &self.resource_graph_query_filter {
            attributes.insert(
                "resource_graph_query_filter".into(),
                v.to_attribute_value()?,
            );
        }
        if let Some(v) = &self.metric_aggregations {
            attributes.insert("metric_aggregations".into(), v.to_attribute_value()?);
        }
        for (name, value) in [
            ("timespan", &self.timespan),
            ("interval", &self.interval),
            ("metric_namespace", &self.metric_namespace),
        ] {
            if let Some(v) = value {
                attributes.insert(name.into(), AttributeValue::String(v.clone()));
            }
        }
        for (name, value) in [
            ("included_dimensions", &self.included_dimensions),
            ("included_resource_tags", &self.included_resource_tags),
            ("regions", &self.regions),
        ] {
            if let Some(v) = value {
                attributes.insert(name.into(), v.to_attribute_value()?);
            }
        }
        if let Some(v) = &self.azure_cloud_environment {
            attributes.insert("azure_cloud_environment".into(), v.to_attribute_value()?);
        }
        if let Some(v) = self.validate_dimensions {
            attributes.insert("validate_dimensions".into(), AttributeValue::Bool(v));
        }
        Ok(Block {
            component: "prometheus.exporter.azure".into(),
            label: self.label.clone(),
            attributes,
            blocks: Vec::new(),
        })
    }
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
    fn scrape_honor_timestamps_accepts_an_expression() {
        // Inside a `declare` body the choice is an argument: a pull whose points
        // are hours apart is stamped at the scrape rather than refused as stale.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.scrape:
                  targets: ["discovery.relabel.pin.output"]
                  forward_to: ["argument.forward_to.value"]
                  honor_timestamps: {ref: argument.honor_timestamps.value}
            "#,
        )
        .unwrap();
        assert!(
            pipeline
                .render()
                .unwrap()
                .contains("honor_timestamps = argument.honor_timestamps.value\n"),
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

    /// The collector allowlists are list-or-expression: a literal YAML list and
    /// an `encoding.from_json(sys.env(...))` expression both have to work, and
    /// the untagged dispatch has to tell them apart.
    #[test]
    fn cadvisor_collector_lists_accept_literals_and_expressions() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cadvisor:
                  label: local
                  disabled_metrics:
                    function: encoding.from_json
                    arguments:
                      - function: coalesce
                        arguments:
                          - env: ALLOY_CADVISOR_DISABLED
                          - '["percpu"]'
                  store_container_labels: false
                  allowlisted_container_labels:
                    - io.kubernetes.pod.name
            "#,
        )
        .unwrap();
        // Plain byte assert rather than `assert_renders`: same alignment gap as
        // the loki.source.kubernetes case — alloy fmt pads the scalar attributes
        // to line up with the multi-line `allowlisted_container_labels`.
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "prometheus.exporter.cadvisor \"local\" {\n",
                "\tdisabled_metrics = encoding.from_json(coalesce(sys.env(\"ALLOY_CADVISOR_DISABLED\"), \"[\\\"percpu\\\"]\"))\n",
                "\tstore_container_labels = false\n",
                "\tallowlisted_container_labels = [\n",
                "\t\t\"io.kubernetes.pod.name\",\n",
                "\t]\n",
                "}\n",
            ),
        );
    }

    /// Paths of the schema violations in a rejected pipeline.
    fn schema_violation_paths(err: Error) -> Vec<String> {
        match err {
            Error::Multiple(errs) => errs
                .iter()
                .filter_map(|e| match e {
                    Error::Schema { path, .. } => Some(path.clone()),
                    _ => None,
                })
                .collect(),
            other => panic!("expected Multiple([Schema, ...]), got {other:?}"),
        }
    }

    /// A static job carries the label, the attributes in their declared order,
    /// and its `role` and `metric` blocks in YAML order.
    #[test]
    fn cloudwatch_static_job_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  label: provider
                  sts_region: us-east-1
                  blocks:
                    - static:
                        label: rds_example_db
                        regions: [us-east-1]
                        namespace: AWS/RDS
                        nil_to_zero: false
                        dimensions:
                          DBInstanceIdentifier: example-db
                        blocks:
                          - role:
                              role_arn: arn:aws:iam::123456789012:role/reader
                          - metric:
                              name: CPUUtilization
                              statistics: [Average, p99]
                              period: 5m
                              length: 10m
            "#,
        )
        .unwrap();
        // Plain byte assert: alloy fmt aligns the scalars beside the multi-line
        // `regions` and `dimensions`, which the renderer does not.
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "prometheus.exporter.cloudwatch \"provider\" {\n",
                "\tsts_region = \"us-east-1\"\n",
                "\n",
                "\tstatic \"rds_example_db\" {\n",
                "\t\tregions = [\n",
                "\t\t\t\"us-east-1\",\n",
                "\t\t]\n",
                "\t\tnamespace = \"AWS/RDS\"\n",
                "\t\tnil_to_zero = false\n",
                "\t\tdimensions = {\n",
                "\t\t\tDBInstanceIdentifier = \"example-db\",\n",
                "\t\t}\n",
                "\n",
                "\t\trole {\n",
                "\t\t\trole_arn = \"arn:aws:iam::123456789012:role/reader\"\n",
                "\t\t}\n",
                "\n",
                "\t\tmetric {\n",
                "\t\t\tname = \"CPUUtilization\"\n",
                "\t\t\tstatistics = [\n",
                "\t\t\t\t\"Average\",\n",
                "\t\t\t\t\"p99\",\n",
                "\t\t\t]\n",
                "\t\t\tperiod = \"5m\"\n",
                "\t\t\tlength = \"10m\"\n",
                "\t\t}\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    /// A discovery job is unlabelled. A tag key full of `:` and `/` is quoted,
    /// and a tag value can be an expression. The exported tags sit on the
    /// exporter, keyed by namespace.
    #[test]
    fn cloudwatch_discovery_job_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  label: eks
                  sts_region: us-east-1
                  discovery_exported_tags:
                    AWS/EC2: [karpenter.sh/nodepool]
                  blocks:
                    - discovery:
                        type: AWS/EC2
                        regions: [us-east-1]
                        search_tags:
                          aws:eks:cluster-name: {raw: 'string.format("^(%s)$", "example")'}
                        dimension_name_requirements: [InstanceId]
                        nil_to_zero: false
                        blocks:
                          - metric:
                              name: StatusCheckFailed_System
                              statistics: [Maximum]
                              period: 5m
                              length: 10m
            "#,
        )
        .unwrap();
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "prometheus.exporter.cloudwatch \"eks\" {\n",
                "\tsts_region = \"us-east-1\"\n",
                "\tdiscovery_exported_tags = {\n",
                "\t\t\"AWS/EC2\" = [\n",
                "\t\t\t\"karpenter.sh/nodepool\",\n",
                "\t\t],\n",
                "\t}\n",
                "\n",
                "\tdiscovery {\n",
                "\t\ttype = \"AWS/EC2\"\n",
                "\t\tregions = [\n",
                "\t\t\t\"us-east-1\",\n",
                "\t\t]\n",
                "\t\tsearch_tags = {\n",
                "\t\t\t\"aws:eks:cluster-name\" = string.format(\"^(%s)$\", \"example\"),\n",
                "\t\t}\n",
                "\t\tdimension_name_requirements = [\n",
                "\t\t\t\"InstanceId\",\n",
                "\t\t]\n",
                "\t\tnil_to_zero = false\n",
                "\n",
                "\t\tmetric {\n",
                "\t\t\tname = \"StatusCheckFailed_System\"\n",
                "\t\t\tstatistics = [\n",
                "\t\t\t\t\"Maximum\",\n",
                "\t\t\t]\n",
                "\t\t\tperiod = \"5m\"\n",
                "\t\t\tlength = \"10m\"\n",
                "\t\t}\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    /// A discovery job cannot be labelled: Alloy's block is unlabelled.
    #[test]
    fn cloudwatch_discovery_job_takes_no_label() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  sts_region: us-east-1
                  blocks:
                    - discovery:
                        label: ec2
                        type: AWS/EC2
                        regions: [us-east-1]
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0")),
        );
    }

    /// `decoupled_scraping` is reachable through `raw:`, and only through it.
    #[test]
    fn cloudwatch_untyped_blocks_use_the_raw_escape() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  sts_region: us-east-1
                  blocks:
                    - raw:
                        component: decoupled_scraping
                        attributes:
                          enabled: false
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "prometheus.exporter.cloudwatch {\n",
                "\tsts_region = \"us-east-1\"\n",
                "\n",
                "\tdecoupled_scraping {\n",
                "\t\tenabled = false\n",
                "\t}\n",
                "}\n",
            ),
        );

        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  sts_region: us-east-1
                  blocks:
                    - decoupled_scraping:
                        enabled: true
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0")),
        );
    }

    /// Alloy refuses a block label that is not an identifier, and resource
    /// names are full of dashes. Catch it at the schema, not at load.
    #[test]
    fn cloudwatch_static_label_must_be_an_identifier() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  sts_region: us-east-1
                  blocks:
                    - static:
                        label: example-db
                        regions: [us-east-1]
                        namespace: AWS/RDS
                        dimensions:
                          DBInstanceIdentifier: example-db
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0")),
        );
    }

    /// `GetMetricStatistics` takes the five basic statistics and percentiles.
    #[test]
    fn cloudwatch_metric_rejects_an_unknown_statistic() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.cloudwatch:
                  sts_region: us-east-1
                  blocks:
                    - static:
                        label: rds_example_db
                        regions: [us-east-1]
                        namespace: AWS/RDS
                        dimensions:
                          DBInstanceIdentifier: example-db
                        blocks:
                          - metric:
                              name: CPUUtilization
                              statistics: [Mean]
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0")),
        );
    }

    /// The extra filter keeps its embedded quotes and the colon inside the
    /// Cloud SQL `database_id`.
    #[test]
    fn gcp_exporter_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.gcp:
                  label: provider
                  project_ids: [example-project]
                  metrics_prefixes:
                    - cloudsql.googleapis.com/database/cpu/utilization
                  extra_filters:
                    - 'cloudsql.googleapis.com/database:resource.labels.database_id=one_of("example-project:example-pg")'
                  request_interval: 10m
                  ingest_delay: true
            "#,
        )
        .unwrap();
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "prometheus.exporter.gcp \"provider\" {\n",
                "\tproject_ids = [\n",
                "\t\t\"example-project\",\n",
                "\t]\n",
                "\tmetrics_prefixes = [\n",
                "\t\t\"cloudsql.googleapis.com/database/cpu/utilization\",\n",
                "\t]\n",
                "\textra_filters = [\n",
                "\t\t\"cloudsql.googleapis.com/database:resource.labels.database_id=one_of(\\\"example-project:example-pg\\\")\",\n",
                "\t]\n",
                "\trequest_interval = \"10m\"\n",
                "\tingest_delay = true\n",
                "}\n",
            ),
        );
    }

    /// The Kusto filter keeps its single quotes, and an empty tag list renders
    /// as one rather than being dropped, since the default is not empty.
    #[test]
    fn azure_exporter_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.azure:
                  label: postgres
                  subscriptions: [00000000-0000-0000-0000-000000000000]
                  resource_type: Microsoft.DBforPostgreSQL/flexibleServers
                  metrics: [cpu_percent, is_db_alive]
                  resource_graph_query_filter: "where name in~ ('example-pg')"
                  metric_aggregations: [average, minimum]
                  timespan: PT10M
                  interval: PT5M
                  included_resource_tags: []
                  azure_cloud_environment: azurecloud
            "#,
        )
        .unwrap();
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "prometheus.exporter.azure \"postgres\" {\n",
                "\tsubscriptions = [\n",
                "\t\t\"00000000-0000-0000-0000-000000000000\",\n",
                "\t]\n",
                "\tresource_type = \"Microsoft.DBforPostgreSQL/flexibleServers\"\n",
                "\tmetrics = [\n",
                "\t\t\"cpu_percent\",\n",
                "\t\t\"is_db_alive\",\n",
                "\t]\n",
                "\tresource_graph_query_filter = \"where name in~ ('example-pg')\"\n",
                "\tmetric_aggregations = [\n",
                "\t\t\"average\",\n",
                "\t\t\"minimum\",\n",
                "\t]\n",
                "\ttimespan = \"PT10M\"\n",
                "\tinterval = \"PT5M\"\n",
                "\tincluded_resource_tags = []\n",
                "\tazure_cloud_environment = \"azurecloud\"\n",
                "}\n",
            ),
        );
    }

    /// `regions` switches the exporter to subscription scope, where the
    /// Resource Graph filter does not apply; the exporter refuses both.
    #[test]
    fn azure_exporter_refuses_regions_with_a_filter() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.azure:
                  subscriptions: [example]
                  resource_type: Microsoft.Storage/storageAccounts
                  metrics: [UsedCapacity]
                  regions: [eastus2]
                  resource_graph_query_filter: "where name == 'x'"
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0")),
        );
    }

    #[test]
    fn gcp_exporter_requires_prefixes() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - prometheus.exporter.gcp:
                  project_ids: [example-project]
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0")),
        );
    }
}
