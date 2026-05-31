// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{AttributeValue, Block, Expression, GoDuration, Identifier, ToBlock};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

type TargetRef = String;

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
