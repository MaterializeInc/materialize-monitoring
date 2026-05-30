// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::alloy::error::Result;

pub type Identifier = String;
// TODO: struct with more checks
pub type GoDuration = String;

// An Alloy block describing a component and its contents
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    // type of the component
    pub component: String,
    // label (generally recommended, but not technically required for single instances of a component)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<Identifier>,
    // top-level assignments, which could be defined in the body, but are rendered slightly neaterly
    #[serde(default)]
    pub attributes: IndexMap<Identifier, AttributeValue>,
    #[serde(default)]
    pub blocks: Vec<Block>,
}

impl Block {
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            ..Default::default()
        }
    }
}

pub trait ToBlock {
    fn to_block(&self) -> Result<Block>;
}

/// Expressions
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct Expression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref", default)]
    pub ref_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub arguments: Vec<AttributeValue>,
}

// The RHS "value" of an assignment
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Number(f64),
    Expression(Expression),
    String(String),
    Array(Vec<AttributeValue>),
    Object(IndexMap<Identifier, AttributeValue>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_block() {
        let yaml = r#"
            component: loki.echo
            label: stub
            attributes:
              forward_to: ["loki.write.example.receiver"]
        "#;
        let block: Block = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(block.component, "loki.echo");
        assert_eq!(block.label.as_deref(), Some("stub"));
    }
}
