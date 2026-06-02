// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{AttributeValue, Block, Expression, GoDuration, Identifier, ToBlock};
use crate::alloy::components::relabel::RelabelSubBlock;
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

type TargetRef = String;

/// Wrap a list of `TargetRef`s as an `AttributeValue::Array` of bare-ref
/// expressions (e.g. `[loki.write.x.receiver]`, not `["loki.write.x.receiver"]`).
fn target_refs(refs: &[TargetRef]) -> AttributeValue {
    AttributeValue::Array(
        refs.iter()
            .map(|s| {
                AttributeValue::Expression(Expression {
                    ref_name: Some(s.clone()),
                    ..Default::default()
                })
            })
            .collect(),
    )
}

/// Convert a label/expression map to an `AttributeValue::Object` of string values.
fn string_map(map: &IndexMap<String, String>) -> AttributeValue {
    AttributeValue::Object(
        map.iter()
            .map(|(k, v)| (k.clone(), AttributeValue::String(v.clone())))
            .collect(),
    )
}

/// A "loki.echo" block, which shows output to stdout for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LokiEchoBlock {
    /// Label for this loki.echo block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
}

impl ToBlock for LokiEchoBlock {
    fn to_block(&self) -> Result<Block> {
        Ok(Block {
            component: "loki.echo".into(),
            label: self.label.clone(),
            ..Default::default()
        })
    }
}

/// A "loki.source.journal" block, which shows output to stdout for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LokiSourceJournalBlock {
    /// Label for this loki.source.journal block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Forward logs to this target
    /// This is required
    pub forward_to: Vec<TargetRef>,
    /// Journal path to read logs from (e.g. /var/log/journal)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Journal selector to filter logs (e.g. _SYSTEMD_UNIT=nginx.service)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<String>,
    /// Static labels to apply to the logs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<IndexMap<String, String>>,
    /// Maximum age of journal entries to read
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age: Option<GoDuration>,
    /// Whether to format logs as JSON
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_as_json: Option<bool>,
    /// Relabeling rules to apply to the logs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relabel_rules: Option<TargetRef>,
}

impl ToBlock for LokiSourceJournalBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if !self.forward_to.is_empty() {
            attributes.insert(
                "forward_to".into(),
                AttributeValue::Array(
                    self.forward_to
                        .iter()
                        .map(|s| {
                            AttributeValue::Expression(Expression {
                                ref_name: Some(s.clone()),
                                ..Default::default()
                            })
                        })
                        .collect(),
                ),
            );
        }
        if let Some(path) = &self.path {
            attributes.insert("path".into(), AttributeValue::String(path.clone()));
        }
        if let Some(matches) = &self.matches {
            attributes.insert("matches".into(), AttributeValue::String(matches.clone()));
        }
        if let Some(labels) = &self.labels {
            attributes.insert(
                "labels".into(),
                AttributeValue::Object(
                    labels
                        .iter()
                        .map(|(k, v)| (k.clone(), AttributeValue::String(v.clone())))
                        .collect(),
                ),
            );
        }
        if let Some(max_age) = &self.max_age {
            attributes.insert("max_age".into(), AttributeValue::String(max_age.clone()));
        }
        if let Some(format_as_json) = self.format_as_json {
            attributes.insert(
                "format_as_json".into(),
                AttributeValue::Bool(format_as_json),
            );
        }
        if let Some(rr) = &self.relabel_rules {
            attributes.insert(
                "relabel_rules".into(),
                AttributeValue::Expression(Expression {
                    ref_name: Some(rr.clone()),
                    ..Default::default()
                }),
            );
        }
        Ok(Block {
            component: "loki.source.journal".into(),
            label: self.label.clone(),
            attributes,
            ..Default::default()
        })
    }
}

// ============================================================
// loki.relabel
// ============================================================

/// A "loki.relabel" block — rewrites log entry labels via `rule` sub-blocks
/// before forwarding downstream.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/loki/loki.relabel/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LokiRelabelBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Loki receivers to forward relabeled entries to. Required by the schema.
    pub forward_to: Vec<TargetRef>,
    /// Maximum number of relabeling results to cache. Defaults to 10,000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cache_size: Option<f64>,
    /// `rule` sub-blocks applied in document order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<RelabelSubBlock>,
}

impl ToBlock for LokiRelabelBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("forward_to".into(), target_refs(&self.forward_to));
        if let Some(mc) = self.max_cache_size {
            attributes.insert("max_cache_size".into(), AttributeValue::Number(mc));
        }
        let mut blocks: Vec<Block> = Vec::with_capacity(self.blocks.len());
        for sb in &self.blocks {
            blocks.push(sb.to_block()?);
        }
        Ok(Block {
            component: "loki.relabel".into(),
            label: self.label.clone(),
            attributes,
            blocks,
        })
    }
}

// ============================================================
// loki.source.file
// ============================================================

/// A "loki.source.file" block — tails log files described by `targets`,
/// forwarding new entries downstream.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/loki/loki.source.file/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LokiSourceFileBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// File targets to tail. Each target is an object with `__path__` and any
    /// label keys to attach. Required by the schema.
    pub targets: Vec<IndexMap<String, String>>,
    /// Loki receivers to forward tailed entries to. Required by the schema.
    pub forward_to: Vec<TargetRef>,
    /// When true, only lines added after start are tailed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail_from_end: Option<bool>,
    /// Character encoding override. Defaults to UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

impl ToBlock for LokiSourceFileBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "targets".into(),
            AttributeValue::Array(self.targets.iter().map(string_map).collect()),
        );
        attributes.insert("forward_to".into(), target_refs(&self.forward_to));
        if let Some(tail) = self.tail_from_end {
            attributes.insert("tail_from_end".into(), AttributeValue::Bool(tail));
        }
        if let Some(enc) = &self.encoding {
            attributes.insert("encoding".into(), AttributeValue::String(enc.clone()));
        }
        Ok(Block {
            component: "loki.source.file".into(),
            label: self.label.clone(),
            attributes,
            ..Default::default()
        })
    }
}

// ============================================================
// loki.process  (+ stage.* sub-blocks)
// ============================================================

/// A "loki.process" block — receives log entries, applies a pipeline of
/// stages, and forwards the results.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/loki/loki.process/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LokiProcessBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Loki receivers to forward processed entries to.
    pub forward_to: Vec<TargetRef>,
    /// Stages applied in document order to each entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<StageBlock>,
}

impl ToBlock for LokiProcessBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("forward_to".into(), target_refs(&self.forward_to));
        let mut blocks: Vec<Block> = Vec::with_capacity(self.blocks.len());
        for sb in &self.blocks {
            blocks.push(sb.to_block()?);
        }
        Ok(Block {
            component: "loki.process".into(),
            label: self.label.clone(),
            attributes,
            blocks,
        })
    }
}

/// Sub-block under a `loki.process` body (or, recursively, a `stage.match`).
/// Externally-tagged; the final `Raw` variant is the escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageBlock {
    #[serde(rename = "stage.match")]
    Match(StageMatchBlock),
    #[serde(rename = "stage.drop")]
    Drop(StageDropBlock),
    #[serde(rename = "stage.limit")]
    Limit(StageLimitBlock),
    #[serde(rename = "stage.regex")]
    Regex(StageRegexBlock),
    #[serde(rename = "stage.replace")]
    Replace(StageReplaceBlock),
    #[serde(rename = "stage.template")]
    Template(StageTemplateBlock),
    #[serde(rename = "stage.logfmt")]
    Logfmt(StageLogfmtBlock),
    #[serde(rename = "stage.json")]
    Json(StageJsonBlock),
    #[serde(rename = "stage.timestamp")]
    Timestamp(StageTimestampBlock),
    #[serde(rename = "stage.labels")]
    Labels(StageLabelsBlock),
    #[serde(rename = "stage.static_labels")]
    StaticLabels(StageStaticLabelsBlock),
    #[serde(rename = "stage.label_drop")]
    LabelDrop(StageLabelDropBlock),
    #[serde(rename = "stage.structured_metadata")]
    StructuredMetadata(StageStructuredMetadataBlock),
    #[serde(rename = "stage.structured_metadata_drop")]
    StructuredMetadataDrop(StageStructuredMetadataDropBlock),
    #[serde(rename = "stage.sampling")]
    Sampling(StageSamplingBlock),
    #[serde(rename = "raw")]
    Raw(Block),
}

impl ToBlock for StageBlock {
    fn to_block(&self) -> Result<Block> {
        match self {
            Self::Match(b) => b.to_block(),
            Self::Drop(b) => b.to_block(),
            Self::Limit(b) => b.to_block(),
            Self::Regex(b) => b.to_block(),
            Self::Replace(b) => b.to_block(),
            Self::Template(b) => b.to_block(),
            Self::Logfmt(b) => b.to_block(),
            Self::Json(b) => b.to_block(),
            Self::Timestamp(b) => b.to_block(),
            Self::Labels(b) => b.to_block(),
            Self::StaticLabels(b) => b.to_block(),
            Self::LabelDrop(b) => b.to_block(),
            Self::StructuredMetadata(b) => b.to_block(),
            Self::StructuredMetadataDrop(b) => b.to_block(),
            Self::Sampling(b) => b.to_block(),
            Self::Raw(b) => Ok(b.clone()),
        }
    }
}

// ----- stage.match -----

/// `stage.match` — conditionally apply nested stages to entries matching a
/// LogQL selector. Recursive: its body is another list of `StageBlock`s.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageMatchBlock {
    pub selector: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_counter_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<StageBlock>,
}

impl ToBlock for StageMatchBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "selector".into(),
            AttributeValue::String(self.selector.clone()),
        );
        if let Some(a) = &self.action {
            attributes.insert("action".into(), AttributeValue::String(a.clone()));
        }
        if let Some(p) = &self.pipeline_name {
            attributes.insert("pipeline_name".into(), AttributeValue::String(p.clone()));
        }
        if let Some(r) = &self.drop_counter_reason {
            attributes.insert(
                "drop_counter_reason".into(),
                AttributeValue::String(r.clone()),
            );
        }
        let mut blocks: Vec<Block> = Vec::with_capacity(self.blocks.len());
        for sb in &self.blocks {
            blocks.push(sb.to_block()?);
        }
        Ok(Block {
            component: "stage.match".into(),
            label: None,
            attributes,
            blocks,
        })
    }
}

// ----- stage.drop -----

/// `stage.drop` — drops log entries matching the configured condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageDropBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Drop entries older than this duration (Go duration syntax).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub older_than: Option<GoDuration>,
    /// Drop entries whose line is longer than this byte length (e.g. `1MB`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longer_than: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_counter_reason: Option<String>,
}

impl ToBlock for StageDropBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = &self.source {
            attributes.insert("source".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.expression {
            attributes.insert("expression".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.value {
            attributes.insert("value".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.older_than {
            attributes.insert("older_than".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.longer_than {
            attributes.insert("longer_than".into(), AttributeValue::String(v.clone()));
        }
        if let Some(v) = &self.drop_counter_reason {
            attributes.insert(
                "drop_counter_reason".into(),
                AttributeValue::String(v.clone()),
            );
        }
        Ok(Block {
            component: "stage.drop".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.limit -----

/// `stage.limit` — rate-limits incoming entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageLimitBlock {
    pub rate: f64,
    pub burst: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_label_name: Option<String>,
}

impl ToBlock for StageLimitBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("rate".into(), AttributeValue::Number(self.rate));
        attributes.insert("burst".into(), AttributeValue::Number(self.burst));
        if let Some(d) = self.drop {
            attributes.insert("drop".into(), AttributeValue::Bool(d));
        }
        if let Some(n) = &self.by_label_name {
            attributes.insert("by_label_name".into(), AttributeValue::String(n.clone()));
        }
        Ok(Block {
            component: "stage.limit".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.regex -----

/// `stage.regex` — extracts named capture groups from a field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRegexBlock {
    pub expression: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl ToBlock for StageRegexBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "expression".into(),
            AttributeValue::String(self.expression.clone()),
        );
        if let Some(s) = &self.source {
            attributes.insert("source".into(), AttributeValue::String(s.clone()));
        }
        Ok(Block {
            component: "stage.regex".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.replace -----

/// `stage.replace` — replaces regex matches in a field with a literal string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageReplaceBlock {
    pub expression: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,
}

impl ToBlock for StageReplaceBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "expression".into(),
            AttributeValue::String(self.expression.clone()),
        );
        if let Some(s) = &self.source {
            attributes.insert("source".into(), AttributeValue::String(s.clone()));
        }
        if let Some(r) = &self.replace {
            attributes.insert("replace".into(), AttributeValue::String(r.clone()));
        }
        Ok(Block {
            component: "stage.replace".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.template -----

/// `stage.template` — sets a field via a Go-template expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageTemplateBlock {
    pub source: String,
    pub template: String,
}

impl ToBlock for StageTemplateBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("source".into(), AttributeValue::String(self.source.clone()));
        attributes.insert(
            "template".into(),
            AttributeValue::String(self.template.clone()),
        );
        Ok(Block {
            component: "stage.template".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.logfmt -----

/// `stage.logfmt` — parses a logfmt-formatted field into the entry's data map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageLogfmtBlock {
    pub mapping: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl ToBlock for StageLogfmtBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("mapping".into(), string_map(&self.mapping));
        if let Some(s) = &self.source {
            attributes.insert("source".into(), AttributeValue::String(s.clone()));
        }
        Ok(Block {
            component: "stage.logfmt".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.json -----

/// `stage.json` — parses a JSON field into the entry's data map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageJsonBlock {
    pub expressions: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_malformed: Option<bool>,
}

impl ToBlock for StageJsonBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("expressions".into(), string_map(&self.expressions));
        if let Some(s) = &self.source {
            attributes.insert("source".into(), AttributeValue::String(s.clone()));
        }
        if let Some(d) = self.drop_malformed {
            attributes.insert("drop_malformed".into(), AttributeValue::Bool(d));
        }
        Ok(Block {
            component: "stage.json".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.timestamp -----

/// `stage.timestamp` — parses a timestamp field and sets the entry's time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageTimestampBlock {
    pub source: String,
    pub format: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_formats: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_on_failure: Option<String>,
}

impl ToBlock for StageTimestampBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("source".into(), AttributeValue::String(self.source.clone()));
        attributes.insert("format".into(), AttributeValue::String(self.format.clone()));
        if !self.fallback_formats.is_empty() {
            attributes.insert(
                "fallback_formats".into(),
                AttributeValue::Array(
                    self.fallback_formats
                        .iter()
                        .map(|s| AttributeValue::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(l) = &self.location {
            attributes.insert("location".into(), AttributeValue::String(l.clone()));
        }
        if let Some(a) = &self.action_on_failure {
            attributes.insert(
                "action_on_failure".into(),
                AttributeValue::String(a.clone()),
            );
        }
        Ok(Block {
            component: "stage.timestamp".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.labels / stage.static_labels / stage.structured_metadata -----
//
// These three all wrap a single required `values: IndexMap<String, String>`.

/// `stage.labels` — promotes extracted fields to indexed Loki labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageLabelsBlock {
    pub values: IndexMap<String, String>,
}

impl ToBlock for StageLabelsBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("values".into(), string_map(&self.values));
        Ok(Block {
            component: "stage.labels".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

/// `stage.static_labels` — attaches constant indexed labels to every entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageStaticLabelsBlock {
    pub values: IndexMap<String, String>,
}

impl ToBlock for StageStaticLabelsBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("values".into(), string_map(&self.values));
        Ok(Block {
            component: "stage.static_labels".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

/// `stage.structured_metadata` — promotes fields to structured (non-indexed) metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageStructuredMetadataBlock {
    pub values: IndexMap<String, String>,
}

impl ToBlock for StageStructuredMetadataBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("values".into(), string_map(&self.values));
        Ok(Block {
            component: "stage.structured_metadata".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.label_drop / stage.structured_metadata_drop -----
//
// Two flavors of "remove N keys"; values is a list of label / metadata names.

/// `stage.label_drop` — removes named indexed labels from each entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageLabelDropBlock {
    pub values: Vec<String>,
}

impl ToBlock for StageLabelDropBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "values".into(),
            AttributeValue::Array(
                self.values
                    .iter()
                    .map(|s| AttributeValue::String(s.clone()))
                    .collect(),
            ),
        );
        Ok(Block {
            component: "stage.label_drop".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

/// `stage.structured_metadata_drop` — removes named structured-metadata fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageStructuredMetadataDropBlock {
    pub values: Vec<String>,
}

impl ToBlock for StageStructuredMetadataDropBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "values".into(),
            AttributeValue::Array(
                self.values
                    .iter()
                    .map(|s| AttributeValue::String(s.clone()))
                    .collect(),
            ),
        );
        Ok(Block {
            component: "stage.structured_metadata_drop".into(),
            label: None,
            attributes,
            ..Default::default()
        })
    }
}

// ----- stage.sampling -----

/// `stage.sampling` — probabilistically drops entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageSamplingBlock {
    pub rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_counter_reason: Option<String>,
}

impl ToBlock for StageSamplingBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("rate".into(), AttributeValue::Number(self.rate));
        if let Some(r) = &self.drop_counter_reason {
            attributes.insert(
                "drop_counter_reason".into(),
                AttributeValue::String(r.clone()),
            );
        }
        Ok(Block {
            component: "stage.sampling".into(),
            label: None,
            attributes,
            ..Default::default()
        })
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
    fn loki_relabel_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - loki.relabel:
                  label: filtered
                  forward_to: ["loki.write.gateway.receiver"]
                  blocks:
                    - rule:
                        action: keep
                        regex: "alloy"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "loki.relabel \"filtered\" {\n",
                "\tforward_to = [\n",
                "\t\tloki.write.gateway.receiver,\n",
                "\t]\n",
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
    fn loki_source_file_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - loki.source.file:
                  targets:
                    - __path__: "/var/log/app.log"
                      job: "app"
                  forward_to: ["loki.write.gateway.receiver"]
            "#,
        )
        .unwrap();
        // Both `targets` and `forward_to` are multi-line — alignment is skipped
        // for this attribute group per the current renderer rule.
        assert_renders(
            pipeline.render(),
            concat!(
                "loki.source.file {\n",
                "\ttargets = [\n",
                "\t\t{\n",
                "\t\t\t__path__ = \"/var/log/app.log\",\n",
                "\t\t\tjob      = \"app\",\n",
                "\t\t},\n",
                "\t]\n",
                "\tforward_to = [\n",
                "\t\tloki.write.gateway.receiver,\n",
                "\t]\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn loki_process_minimal_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - loki.process:
                  label: processor
                  forward_to: ["loki.write.gateway.receiver"]
                  blocks:
                    - stage.drop:
                        older_than: "12h"
                        drop_counter_reason: "backlog > 12hr"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "loki.process \"processor\" {\n",
                "\tforward_to = [\n",
                "\t\tloki.write.gateway.receiver,\n",
                "\t]\n",
                "\n",
                "\tstage.drop {\n",
                "\t\tolder_than          = \"12h\"\n",
                "\t\tdrop_counter_reason = \"backlog > 12hr\"\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn loki_process_kitchen_sink_with_recursive_stage_match() {
        // Exercises the recursive case (stage.match nesting) plus a sample of
        // the typed stages. Each inner stage is kept to one attribute to dodge
        // the multi-line-disables-alignment quirk in the renderer (task #15).
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - loki.process:
                  forward_to: ["loki.write.gateway.receiver"]
                  blocks:
                    - stage.match:
                        selector: '{app="alloy"}'
                        blocks:
                          - stage.regex:
                              expression: '(?P<level>[A-Z]+)'
                          - stage.timestamp:
                              source: ts
                              format: RFC3339Nano
                    - stage.labels:
                        values:
                          level: level
                    - stage.sampling:
                        rate: 0.05
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "loki.process {\n",
                "\tforward_to = [\n",
                "\t\tloki.write.gateway.receiver,\n",
                "\t]\n",
                "\n",
                "\tstage.match {\n",
                "\t\tselector = \"{app=\\\"alloy\\\"}\"\n",
                "\n",
                "\t\tstage.regex {\n",
                "\t\t\texpression = \"(?P<level>[A-Z]+)\"\n",
                "\t\t}\n",
                "\n",
                "\t\tstage.timestamp {\n",
                "\t\t\tsource = \"ts\"\n",
                "\t\t\tformat = \"RFC3339Nano\"\n",
                "\t\t}\n",
                "\t}\n",
                "\n",
                "\tstage.labels {\n",
                "\t\tvalues = {\n",
                "\t\t\tlevel = \"level\",\n",
                "\t\t}\n",
                "\t}\n",
                "\n",
                "\tstage.sampling {\n",
                "\t\trate = 0.05\n",
                "\t}\n",
                "}\n",
            ),
        );
    }
}
