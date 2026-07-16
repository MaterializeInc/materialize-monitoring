// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Typed sugar for otelcol.* components.
//!
//! Mirrors the per-component schemas in `schemas/alloy/otelcol.schema.yaml`.
//! Each block deserializes from the flat `{otelcol.X: {label, attrs..., blocks}}`
//! form and converts to a generic [`Block`] via [`ToBlock`].
//!
//! This first tranche covers the stream-shaping processors: `batch`,
//! `memory_limiter`, `attributes`, and `groupbyattrs`. Every otelcol component
//! forwards through an `output` block whose `metrics` / `logs` / `traces` lists
//! are [`OtelcolConsumer`] capsules (bare refs), so `output` is a shared typed
//! sub-block here. The intricate OTTL processors (`filter`, `transform`) stay on
//! the `raw:` escape for now.

use crate::alloy::ast::{
    AttributeValue, Block, Expressable, GoDuration, Identifier, ToBlock, impl_to_block_dispatch,
};
use crate::alloy::components::capsule::{OtelcolConsumer, otelcol_consumer_list};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Collect a `Vec` of `ToBlock` sub-blocks into rendered `Block`s.
fn to_blocks<T: ToBlock>(blocks: &[T]) -> Result<Vec<Block>> {
    blocks.iter().map(ToBlock::to_block).collect()
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
// otelcol.processor.batch
// ============================================================

/// An `otelcol.processor.batch` block — batches telemetry before forwarding.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.batch/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolProcessorBatchBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Flush a batch after this long regardless of size. Defaults to `200ms`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<GoDuration>,
    /// Number of items to accumulate before flushing. Defaults to `8192`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_batch_size: Option<f64>,
    /// Hard upper bound on a batch's size (larger batches split). Defaults to `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_batch_max_size: Option<f64>,
    /// Request-metadata keys that each get their own batcher.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_keys: Vec<String>,
    /// Maximum number of distinct metadata-key combinations to track.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_cardinality_limit: Option<f64>,
    /// Optional nested blocks (`output`; `debug_metrics` uses `raw:`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<ProcessorSubBlock>,
}

impl ToBlock for OtelcolProcessorBatchBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.timeout {
            attributes.insert("timeout".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = self.send_batch_size {
            attributes.insert("send_batch_size".into(), AttributeValue::Number(v));
        }
        if let Some(v) = self.send_batch_max_size {
            attributes.insert("send_batch_max_size".into(), AttributeValue::Number(v));
        }
        if !self.metadata_keys.is_empty() {
            attributes.insert("metadata_keys".into(), string_array(&self.metadata_keys));
        }
        if let Some(v) = self.metadata_cardinality_limit {
            attributes.insert(
                "metadata_cardinality_limit".into(),
                AttributeValue::Number(v),
            );
        }
        Ok(Block {
            component: "otelcol.processor.batch".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

// ============================================================
// otelcol.processor.memory_limiter
// ============================================================

/// An `otelcol.processor.memory_limiter` block — refuses telemetry when memory
/// crosses configured limits. The limit knobs are `Expressable` so they can be
/// wired to environment variables for per-deployment sizing.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.memory_limiter/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolProcessorMemoryLimiterBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// How often to check memory usage (e.g. `1s`). Required.
    pub check_interval: Expressable<String>,
    /// Hard memory limit as a byte size (e.g. `1GiB`). Exclusive with `limit_percentage`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Expressable<String>>,
    /// Spike headroom as a byte size. Exclusive with `spike_limit_percentage`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spike_limit: Option<Expressable<String>>,
    /// Hard memory limit as a percentage of total memory. Exclusive with `limit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_percentage: Option<Expressable<f64>>,
    /// Spike headroom as a percentage of total memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spike_limit_percentage: Option<Expressable<f64>>,
    /// Optional nested blocks (`output`; `debug_metrics` uses `raw:`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<ProcessorSubBlock>,
}

impl ToBlock for OtelcolProcessorMemoryLimiterBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "check_interval".into(),
            self.check_interval.to_attribute_value()?,
        );
        if let Some(v) = &self.limit {
            attributes.insert("limit".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.spike_limit {
            attributes.insert("spike_limit".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.limit_percentage {
            attributes.insert("limit_percentage".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.spike_limit_percentage {
            attributes.insert("spike_limit_percentage".into(), v.to_attribute_value()?);
        }
        Ok(Block {
            component: "otelcol.processor.memory_limiter".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

// ============================================================
// otelcol.processor.attributes  (+ action sub-block)
// ============================================================

/// An `otelcol.processor.attributes` block — reshapes attributes via ordered
/// `action` sub-blocks.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.attributes/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolProcessorAttributesBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// `action` blocks (applied in order) plus `output`. `include`/`exclude`
    /// match blocks use `raw:`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<AttributesSubBlock>,
}

impl ToBlock for OtelcolProcessorAttributesBlock {
    fn to_block(&self) -> Result<Block> {
        Ok(Block {
            component: "otelcol.processor.attributes".into(),
            label: self.label.clone(),
            attributes: IndexMap::new(),
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// An `action` sub-block — one attribute operation.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.attributes/#action-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributesActionBlock {
    /// The operation: insert / update / upsert / delete / hash / extract / convert.
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Value to set (a literal string or an expression, e.g. `{env: CLUSTER}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Expressable<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converted_type: Option<String>,
}

impl ToBlock for AttributesActionBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.key {
            attributes.insert("key".into(), AttributeValue::String(v.clone()));
        }
        attributes.insert("action".into(), AttributeValue::String(self.action.clone()));
        if let Some(v) = &self.value {
            attributes.insert("value".into(), v.to_attribute_value()?);
        }
        if let Some(v) = &self.pattern {
            attributes.insert("pattern".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.from_attribute {
            attributes.insert("from_attribute".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.from_context {
            attributes.insert("from_context".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.converted_type {
            attributes.insert("converted_type".into(), AttributeValue::String(v.clone()));
        }
        Ok(Block {
            component: "action".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

// ============================================================
// otelcol.processor.groupbyattrs
// ============================================================

/// An `otelcol.processor.groupbyattrs` block — regroups records by attribute.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.groupbyattrs/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolProcessorGroupByAttrsBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Attribute keys to group records by. Empty = compaction only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
    /// Optional nested blocks (`output`; `debug_metrics` uses `raw:`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<ProcessorSubBlock>,
}

impl ToBlock for OtelcolProcessorGroupByAttrsBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if !self.keys.is_empty() {
            attributes.insert("keys".into(), string_array(&self.keys));
        }
        Ok(Block {
            component: "otelcol.processor.groupbyattrs".into(),
            label: self.label.clone(),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

// ============================================================
// Shared sub-blocks: output + sub-block enums
// ============================================================

/// An `output` sub-block — wires this component to downstream consumers.
/// Each signal is a list of [`OtelcolConsumer`] bare refs; omitted signals drop.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.batch/#output-block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolOutputBlock {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<OtelcolConsumer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<OtelcolConsumer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traces: Vec<OtelcolConsumer>,
}

impl ToBlock for OtelcolOutputBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if !self.metrics.is_empty() {
            attributes.insert("metrics".into(), otelcol_consumer_list(&self.metrics));
        }
        if !self.logs.is_empty() {
            attributes.insert("logs".into(), otelcol_consumer_list(&self.logs));
        }
        if !self.traces.is_empty() {
            attributes.insert("traces".into(), otelcol_consumer_list(&self.traces));
        }
        Ok(Block {
            component: "output".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// Sub-block under a batch / memory_limiter / groupbyattrs body. `Raw` is the
/// escape hatch (e.g. for `debug_metrics`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessorSubBlock {
    #[serde(rename = "output")]
    Output(OtelcolOutputBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(ProcessorSubBlock { Output, Raw });

/// Sub-block under an `otelcol.processor.attributes` body. `Raw` is the escape
/// hatch (e.g. for `include`/`exclude` match blocks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributesSubBlock {
    #[serde(rename = "action")]
    Action(AttributesActionBlock),
    #[serde(rename = "output")]
    Output(OtelcolOutputBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(AttributesSubBlock {
    Action,
    Output,
    Raw
});

// ============================================================
// tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::alloy::pipeline::Pipeline;
    use crate::alloy::test_support::assert_renders;

    #[test]
    fn batch_with_output_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.batch:
                  label: default
                  timeout: "200ms"
                  send_batch_size: 8192
                  blocks:
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.batch \"default\" {\n",
                "\ttimeout         = \"200ms\"\n",
                "\tsend_batch_size = 8192\n",
                "\n",
                "\toutput {\n",
                "\t\tmetrics = [\n",
                "\t\t\totelcol.exporter.prometheus.bridge.input,\n",
                "\t\t]\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn memory_limiter_accepts_env_expressions() {
        // The limit knobs are `Expressable`: `check_interval` stays a literal
        // duration while the percentage limits are wired to env vars (with the
        // `encoding.from_json` string->int coercion, since `sys.env` yields a
        // string).
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.memory_limiter:
                  check_interval: "1s"
                  limit_percentage:
                    function: encoding.from_json
                    arguments:
                      - function: coalesce
                        arguments:
                          - env: MEMORY_LIMIT_PERCENTAGE
                          - "80"
                  spike_limit_percentage: 20
                  blocks:
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.memory_limiter {\n",
                "\tcheck_interval         = \"1s\"\n",
                "\tlimit_percentage       = encoding.from_json(coalesce(sys.env(\"MEMORY_LIMIT_PERCENTAGE\"), \"80\"))\n",
                "\tspike_limit_percentage = 20\n",
                "\n",
                "\toutput {\n",
                "\t\tmetrics = [\n",
                "\t\t\totelcol.exporter.prometheus.bridge.input,\n",
                "\t\t]\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn memory_limiter_with_byte_size_limit() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.memory_limiter:
                  check_interval: "1s"
                  limit: "1GiB"
                  spike_limit: "256MiB"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.memory_limiter {\n",
                "\tcheck_interval = \"1s\"\n",
                "\tlimit          = \"1GiB\"\n",
                "\tspike_limit    = \"256MiB\"\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn attributes_with_action_blocks_round_trips() {
        // Ordered `action` blocks plus an `output`: an upsert sourced from an env
        // var, and a regex delete of a high-cardinality attribute.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.attributes:
                  label: reshape
                  blocks:
                    - action:
                        key: cluster
                        action: upsert
                        value: { env: CLUSTER }
                    - action:
                        action: delete
                        pattern: "service_instance_id"
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.attributes \"reshape\" {\n",
                "\taction {\n",
                "\t\tkey    = \"cluster\"\n",
                "\t\taction = \"upsert\"\n",
                "\t\tvalue  = sys.env(\"CLUSTER\")\n",
                "\t}\n",
                "\n",
                "\taction {\n",
                "\t\taction  = \"delete\"\n",
                "\t\tpattern = \"service_instance_id\"\n",
                "\t}\n",
                "\n",
                "\toutput {\n",
                "\t\tmetrics = [\n",
                "\t\t\totelcol.exporter.prometheus.bridge.input,\n",
                "\t\t]\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn groupbyattrs_with_keys_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.groupbyattrs:
                  keys: ["namespace", "cluster"]
                  blocks:
                    - output:
                        metrics: ["otelcol.processor.batch.default.input"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.groupbyattrs {\n",
                "\tkeys = [\n",
                "\t\t\"namespace\",\n",
                "\t\t\"cluster\",\n",
                "\t]\n",
                "\n",
                "\toutput {\n",
                "\t\tmetrics = [\n",
                "\t\t\totelcol.processor.batch.default.input,\n",
                "\t\t]\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn debug_metrics_uses_raw_escape() {
        // A sub-block we don't type yet (`debug_metrics`) still works via `raw:`.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.batch:
                  blocks:
                    - raw:
                        component: debug_metrics
                        attributes:
                          disable_high_cardinality_metrics: true
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.batch {\n",
                "\tdebug_metrics {\n",
                "\t\tdisable_high_cardinality_metrics = true\n",
                "\t}\n",
                "\n",
                "\toutput {\n",
                "\t\tmetrics = [\n",
                "\t\t\totelcol.exporter.prometheus.bridge.input,\n",
                "\t\t]\n",
                "\t}\n",
                "}\n",
            ),
        );
    }
}
