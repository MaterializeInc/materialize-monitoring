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

// Identifiers are used as LHS or labels and may get special treatment
type Identifier = String;
// Durations are a common type of string with temporal units
type Duration = String;

// An Alloy block describing a component and its contents
#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    // type of the component
    pub component: String,
    // label (generally recommended, but not technically required for single instances of a component)
    pub label: Option<Identifier>,
    // top-level assignments, which could be defined in the body, but are rendered slightly neaterly
    #[serde(default)]
    pub attributes: IndexMap<Identifier, AttributeValue>,
    // Nested blocks and assignments
    #[serde(default)]
    pub body: Body,
}

// The RHS "value" of an assignment
#[derive(Serialize, Deserialize, Debug)]
pub enum AttributeValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Duration(Duration),
    Array(Vec<AttributeValue>),
    Empty,
    Object(IndexMap<Identifier, AttributeValue>),
    // TODO: expression
}

// The content of a Block
#[derive(Serialize, Deserialize, Debug, Default)]
pub enum Body {
    #[default]
    Empty,
    Blocks(Vec<Block>),
    Assignment(IndexMap<Identifier, AttributeValue>),
}
