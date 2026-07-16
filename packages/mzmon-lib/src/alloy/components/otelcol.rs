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
//! Covers the stream-shaping processors — `batch`, `memory_limiter`,
//! `attributes`, `groupbyattrs` — and the OTTL processors `filter` and
//! `transform`. Every otelcol component forwards through an `output` block whose
//! `metrics` / `logs` / `traces` lists are [`OtelcolConsumer`] capsules (bare
//! refs), so `output` is a shared typed sub-block here. OTTL statements and
//! conditions are carried as [`Ottl`] strings (rendered verbatim, escaped) and
//! are `Expressable`, so a statement can also be sourced from an expression; we
//! do not model OTTL's function library.

use crate::alloy::ast::{
    AttributeValue, Block, Expressable, GoDuration, Identifier, Ottl, ToBlock,
    impl_to_block_dispatch,
};
use crate::alloy::components::capsule::{OtelcolConsumer, otelcol_consumer_list};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Collect a `Vec` of `ToBlock` sub-blocks into rendered `Block`s.
fn to_blocks<T: ToBlock>(blocks: &[T]) -> Result<Vec<Block>> {
    blocks.iter().map(ToBlock::to_block).collect()
}

/// Render a list of OTTL statements/conditions as an `AttributeValue::Array`.
/// Each element is a raw OTTL string or an expression that yields one.
fn ottl_list(items: &[Expressable<Ottl>]) -> Result<AttributeValue> {
    Ok(AttributeValue::Array(
        items
            .iter()
            .map(Expressable::to_attribute_value)
            .collect::<Result<Vec<_>>>()?,
    ))
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
    /// Instance label — required: alloy rejects an unlabeled otelcol component.
    pub label: Identifier,
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
            label: Some(self.label.clone()),
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
    /// Instance label — required: alloy rejects an unlabeled otelcol component.
    pub label: Identifier,
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
            label: Some(self.label.clone()),
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
    /// Instance label — required: alloy rejects an unlabeled otelcol component.
    pub label: Identifier,
    /// `action` blocks (applied in order) plus `output`. `include`/`exclude`
    /// match blocks use `raw:`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<AttributesSubBlock>,
}

impl ToBlock for OtelcolProcessorAttributesBlock {
    fn to_block(&self) -> Result<Block> {
        Ok(Block {
            component: "otelcol.processor.attributes".into(),
            label: Some(self.label.clone()),
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
    /// Instance label — required: alloy rejects an unlabeled otelcol component.
    pub label: Identifier,
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
            label: Some(self.label.clone()),
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
// otelcol.processor.filter  (+ *_conditions sub-blocks)
// ============================================================

/// An `otelcol.processor.filter` block — drops telemetry matching OTTL
/// conditions.
///
/// Uses the inferred-context `*_conditions` blocks; the deprecated per-signal
/// `traces`/`metrics`/`logs` blocks are intentionally not typed (use `raw:`).
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.filter/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolProcessorFilterBlock {
    /// Instance label — required: alloy rejects an unlabeled otelcol component.
    pub label: Identifier,
    /// How to react to errors while evaluating a condition (`ignore`/`silent`/`propagate`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_mode: Option<String>,
    /// Nested blocks (`output`, `trace_conditions`, `metric_conditions`,
    /// `log_conditions`; `debug_metrics` uses `raw:`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<FilterSubBlock>,
}

impl ToBlock for OtelcolProcessorFilterBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.error_mode {
            attributes.insert("error_mode".into(), AttributeValue::String(v.clone()));
        }
        Ok(Block {
            component: "otelcol.processor.filter".into(),
            label: Some(self.label.clone()),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// A `*_conditions` sub-block — inferred-context OTTL conditions for one signal.
/// Shared shape behind `trace_conditions` / `metric_conditions` /
/// `log_conditions`; the enclosing enum supplies the block name.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.filter/#trace_conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterConditionsBlock {
    /// OTTL context for evaluating the conditions (e.g. `span`, `metric`, `log`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// OTTL conditions; any one matching drops the telemetry.
    pub conditions: Vec<Expressable<Ottl>>,
}

impl FilterConditionsBlock {
    fn to_named_block(&self, component: &str) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(ctx) = &self.context {
            attributes.insert("context".into(), AttributeValue::String(ctx.clone()));
        }
        attributes.insert("conditions".into(), ottl_list(&self.conditions)?);
        Ok(Block {
            component: component.into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// Sub-block under an `otelcol.processor.filter` body. `Raw` is the escape hatch
/// (e.g. for `debug_metrics` or the deprecated `traces`/`metrics`/`logs` blocks).
///
/// The three `*_conditions` variants share [`FilterConditionsBlock`], so the
/// dispatch is hand-written (rather than via `impl_to_block_dispatch!`) to pass
/// each its block name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterSubBlock {
    #[serde(rename = "output")]
    Output(OtelcolOutputBlock),
    #[serde(rename = "trace_conditions")]
    TraceConditions(FilterConditionsBlock),
    #[serde(rename = "metric_conditions")]
    MetricConditions(FilterConditionsBlock),
    #[serde(rename = "log_conditions")]
    LogConditions(FilterConditionsBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}

impl ToBlock for FilterSubBlock {
    fn to_block(&self) -> Result<Block> {
        match self {
            Self::Output(b) => b.to_block(),
            Self::TraceConditions(b) => b.to_named_block("trace_conditions"),
            Self::MetricConditions(b) => b.to_named_block("metric_conditions"),
            Self::LogConditions(b) => b.to_named_block("log_conditions"),
            Self::Raw(b) => b.to_block(),
        }
    }
}

// ============================================================
// otelcol.processor.transform  (+ *_statements / statements sub-blocks)
// ============================================================

/// An `otelcol.processor.transform` block — modifies telemetry with OTTL.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.transform/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtelcolProcessorTransformBlock {
    /// Instance label — required: alloy rejects an unlabeled otelcol component.
    pub label: Identifier,
    /// How to react to errors while processing a statement (`ignore`/`silent`/`propagate`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_mode: Option<String>,
    /// Nested blocks (`output`, `trace_statements`, `metric_statements`,
    /// `log_statements`, `statements`; `debug_metrics` uses `raw:`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<TransformSubBlock>,
}

impl ToBlock for OtelcolProcessorTransformBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.error_mode {
            attributes.insert("error_mode".into(), AttributeValue::String(v.clone()));
        }
        Ok(Block {
            component: "otelcol.processor.transform".into(),
            label: Some(self.label.clone()),
            attributes,
            blocks: to_blocks(&self.blocks)?,
        })
    }
}

/// A `*_statements` sub-block — a context-scoped group of OTTL statements.
/// Shared shape behind `trace_statements` / `metric_statements` /
/// `log_statements`; the enclosing enum supplies the block name.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.transform/#trace_statements
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformStatementsBlock {
    /// OTTL context the statements run in (e.g. `resource`, `span`, `datapoint`).
    pub context: String,
    /// OTTL statements, applied in order.
    pub statements: Vec<Expressable<Ottl>>,
    /// Optional guard conditions (ORed) gating the statements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Expressable<Ottl>>,
    /// Per-block error handling; overrides the component-level `error_mode`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_mode: Option<String>,
}

impl TransformStatementsBlock {
    fn to_named_block(&self, component: &str) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "context".into(),
            AttributeValue::String(self.context.clone()),
        );
        attributes.insert("statements".into(), ottl_list(&self.statements)?);
        if !self.conditions.is_empty() {
            attributes.insert("conditions".into(), ottl_list(&self.conditions)?);
        }
        if let Some(em) = &self.error_mode {
            attributes.insert("error_mode".into(), AttributeValue::String(em.clone()));
        }
        Ok(Block {
            component: component.into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// A `statements` sub-block — OTTL statements with context inferred per signal.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.processor.transform/#statements
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformInferredStatementsBlock {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<Expressable<Ottl>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metric: Vec<Expressable<Ottl>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<Expressable<Ottl>>,
}

impl ToBlock for TransformInferredStatementsBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if !self.trace.is_empty() {
            attributes.insert("trace".into(), ottl_list(&self.trace)?);
        }
        if !self.metric.is_empty() {
            attributes.insert("metric".into(), ottl_list(&self.metric)?);
        }
        if !self.log.is_empty() {
            attributes.insert("log".into(), ottl_list(&self.log)?);
        }
        Ok(Block {
            component: "statements".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// Sub-block under an `otelcol.processor.transform` body. `Raw` is the escape
/// hatch (e.g. for `debug_metrics`).
///
/// The three `*_statements` variants share [`TransformStatementsBlock`], so the
/// dispatch is hand-written to pass each its block name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformSubBlock {
    #[serde(rename = "output")]
    Output(OtelcolOutputBlock),
    #[serde(rename = "trace_statements")]
    TraceStatements(TransformStatementsBlock),
    #[serde(rename = "metric_statements")]
    MetricStatements(TransformStatementsBlock),
    #[serde(rename = "log_statements")]
    LogStatements(TransformStatementsBlock),
    #[serde(rename = "statements")]
    Statements(TransformInferredStatementsBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}

impl ToBlock for TransformSubBlock {
    fn to_block(&self) -> Result<Block> {
        match self {
            Self::Output(b) => b.to_block(),
            Self::TraceStatements(b) => b.to_named_block("trace_statements"),
            Self::MetricStatements(b) => b.to_named_block("metric_statements"),
            Self::LogStatements(b) => b.to_named_block("log_statements"),
            Self::Statements(b) => b.to_block(),
            Self::Raw(b) => b.to_block(),
        }
    }
}

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
                  label: mem
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
                "otelcol.processor.memory_limiter \"mem\" {\n",
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
                  label: mem
                  check_interval: "1s"
                  limit: "1GiB"
                  spike_limit: "256MiB"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.memory_limiter \"mem\" {\n",
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
                  label: group
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
                "otelcol.processor.groupbyattrs \"group\" {\n",
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
                  label: batch
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
                "otelcol.processor.batch \"batch\" {\n",
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

    #[test]
    fn filter_metric_conditions_round_trips() {
        // A `metric_conditions` block: the OTTL condition is a raw string
        // (rendered as an escaped alloy string), and a second condition is
        // sourced from an env var — proving `conditions` is `Expressable<Ottl>`.
        //
        // Byte-checked with `assert_eq!` (not `assert_renders`): `context` sits in
        // an attribute group with the multi-line `conditions` array, hitting the
        // renderer's known alignment quirk (alloy fmt aligns `context =`). Valid
        // alloy, just not fmt-canonical — see the renderer-alignment cleanup note.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.filter:
                  label: drop
                  error_mode: ignore
                  blocks:
                    - metric_conditions:
                        context: datapoint
                        conditions:
                          - 'name == "up"'
                          - env: EXTRA_DROP
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "otelcol.processor.filter \"drop\" {\n",
                "\terror_mode = \"ignore\"\n",
                "\n",
                "\tmetric_conditions {\n",
                "\t\tcontext = \"datapoint\"\n",
                "\t\tconditions = [\n",
                "\t\t\t\"name == \\\"up\\\"\",\n",
                "\t\t\tsys.env(\"EXTRA_DROP\"),\n",
                "\t\t]\n",
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
    fn transform_metric_statements_round_trips() {
        // Context-scoped OTTL statements: quotes inside each statement render as
        // escaped alloy strings.
        //
        // Byte-checked with `assert_eq!` (not `assert_renders`): `context` beside
        // the multi-line `statements` array hits the renderer's alignment quirk
        // (alloy fmt aligns `context =`). Valid alloy, just not fmt-canonical.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.transform:
                  label: shape
                  error_mode: ignore
                  blocks:
                    - metric_statements:
                        context: datapoint
                        statements:
                          - 'set(attributes["env"], "prod")'
                          - 'delete_key(attributes, "service_instance_id")'
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "otelcol.processor.transform \"shape\" {\n",
                "\terror_mode = \"ignore\"\n",
                "\n",
                "\tmetric_statements {\n",
                "\t\tcontext = \"datapoint\"\n",
                "\t\tstatements = [\n",
                "\t\t\t\"set(attributes[\\\"env\\\"], \\\"prod\\\")\",\n",
                "\t\t\t\"delete_key(attributes, \\\"service_instance_id\\\")\",\n",
                "\t\t]\n",
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
    fn transform_inferred_statements_round_trips() {
        // The context-inference `statements` block (no explicit context).
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.transform:
                  label: shape
                  blocks:
                    - statements:
                        metric:
                          - 'set(resource.attributes["dropped"], true)'
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "otelcol.processor.transform \"shape\" {\n",
                "\tstatements {\n",
                "\t\tmetric = [\n",
                "\t\t\t\"set(resource.attributes[\\\"dropped\\\"], true)\",\n",
                "\t\t]\n",
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
    fn missing_label_is_rejected_by_schema() {
        // otelcol components require a label (alloy rejects an unlabeled one), so
        // the schema requires it — a labelless block fails validation rather than
        // rendering un-loadable alloy.
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - otelcol.processor.batch:
                  blocks:
                    - output:
                        metrics: ["otelcol.exporter.prometheus.bridge.input"]
            "#,
        )
        .unwrap_err();
        assert!(
            matches!(err, crate::alloy::error::Error::Multiple(_)),
            "expected a schema rejection for the missing label, got {err:?}"
        );
    }
}
