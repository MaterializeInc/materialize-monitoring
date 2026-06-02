// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Shared `rule` typed sugar for the `*.relabel` family.
//!
//! Both `loki.relabel` and `discovery.relabel` accept a list of `rule` blocks
//! with the same shape — defined here so they share one source of truth.

use crate::alloy::ast::{AttributeValue, Block, ToBlock};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Sub-block under a relabel body (used by both `loki.relabel` and
/// `discovery.relabel`).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RelabelSubBlock {
    #[serde(rename = "rule")]
    Rule(RelabelRule),
    #[serde(rename = "raw")]
    Raw(Block),
}

impl ToBlock for RelabelSubBlock {
    fn to_block(&self) -> Result<Block> {
        match self {
            Self::Rule(r) => r.to_block(),
            Self::Raw(b) => Ok(b.clone()),
        }
    }
}

/// One relabel step, applied in document order.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/loki/loki.relabel/#rule-block
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct RelabelRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulus: Option<f64>,
}

impl ToBlock for RelabelRule {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(action) = &self.action {
            attributes.insert("action".into(), AttributeValue::String(action.clone()));
        }
        if !self.source_labels.is_empty() {
            attributes.insert(
                "source_labels".into(),
                AttributeValue::Array(
                    self.source_labels
                        .iter()
                        .map(|s| AttributeValue::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(separator) = &self.separator {
            attributes.insert(
                "separator".into(),
                AttributeValue::String(separator.clone()),
            );
        }
        if let Some(target_label) = &self.target_label {
            attributes.insert(
                "target_label".into(),
                AttributeValue::String(target_label.clone()),
            );
        }
        if let Some(regex) = &self.regex {
            attributes.insert("regex".into(), AttributeValue::String(regex.clone()));
        }
        if let Some(replacement) = &self.replacement {
            attributes.insert(
                "replacement".into(),
                AttributeValue::String(replacement.clone()),
            );
        }
        if let Some(modulus) = self.modulus {
            attributes.insert("modulus".into(), AttributeValue::Number(modulus));
        }
        Ok(Block {
            component: "rule".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}
