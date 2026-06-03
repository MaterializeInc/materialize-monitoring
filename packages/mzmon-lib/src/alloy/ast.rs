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

impl ToBlock for Block {
    fn to_block(&self) -> Result<Block> {
        Ok(self.clone())
    }
}

/// Support converting an enum with ToBlock traits into a Block via `to_block()`
macro_rules! impl_to_block_dispatch {
    ($enum_name:ident { $($variant:ident),+ $(,)? }) => {
        impl $crate::alloy::ast::ToBlock for $enum_name {
            fn to_block(&self) -> $crate::alloy::error::Result<$crate::alloy::ast::Block> {
                match self {
                    $(Self::$variant(inner) => inner.to_block(),)*
                }
            }
        }
    };
}
pub(crate) use impl_to_block_dispatch;

/// Expressions
///
/// `deny_unknown_fields` is load-bearing for `AttributeValue` untagged dispatch:
/// without it, a generic object like `{mapping: ...}` would silently deserialize
/// as an `Expression` with all heads `None` (because no fields are required and
/// unknown fields are tolerated by default), beating the `Object` variant.
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Expression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub function: Option<String>,
    // NOTE: ref is a reserved keyword in rust
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref", default)]
    pub ref_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub arguments: Vec<AttributeValue>,
}

impl Expression {
    /// Generate an Expression for a ref by its name.
    pub fn name_to_ref(name: impl Into<Identifier>) -> Self {
        Self {
            ref_name: Some(name.into()),
            ..Default::default()
        }
    }
}

/// The scalar literal types an alloy attribute can hold.
/// Sealed (see `private`), so `Expressable<T>` is effectively
/// `Expressable<String | bool | f64>` — no other T compiles.
pub trait LiteralScalar: private::Sealed {
    fn to_attribute_value(&self) -> Result<AttributeValue>;
}

mod private {
    pub trait Sealed {}
    impl Sealed for String {}
    impl Sealed for bool {}
    impl Sealed for f64 {}
}

impl LiteralScalar for String {
    fn to_attribute_value(&self) -> Result<AttributeValue> {
        Ok(AttributeValue::String(self.clone()))
    }
}
impl LiteralScalar for bool {
    fn to_attribute_value(&self) -> Result<AttributeValue> {
        Ok(AttributeValue::Bool(*self))
    }
}
impl LiteralScalar for f64 {
    fn to_attribute_value(&self) -> Result<AttributeValue> {
        Ok(AttributeValue::Number(*self))
    }
}

/// The RHS of a "value" which could be a simple literal or an expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Expressable<T: LiteralScalar> {
    Literal(T),
    Expr(Expression),
}

impl<T: LiteralScalar> Expressable<T> {
    pub fn to_attribute_value(&self) -> Result<AttributeValue> {
        match self {
            Expressable::Literal(value) => value.to_attribute_value(),
            Expressable::Expr(expr) => Ok(AttributeValue::Expression(expr.clone())),
        }
    }
}

// The RHS "value" of an assignment
//
// Variant order matters for `#[serde(untagged)]` dispatch: serde tries each
// variant top-to-bottom and picks the first that deserializes.
//
// `String` and `Array` MUST come before `Expression`, because serde's struct
// deserializer accepts a *sequence* by positional-field assignment by default.
// Without that order, `["a", "b"]` would deserialize as `Expression { raw: Some("a"),
// env: Some("b"), ... }` instead of `Array([String("a"), String("b")])`.
//
// `Expression` must still come before `Object` so structured-shape objects
// (`{ref: "..."}`, `{env: "..."}`, ...) are recognized as expressions rather
// than swallowed by the catch-all map; `deny_unknown_fields` on `Expression`
// keeps generic maps (`{mapping: ...}`) from matching.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<AttributeValue>),
    Expression(Expression),
    Object(IndexMap<Identifier, AttributeValue>),
}

/// Convert a label/expression map to an `AttributeValue::Object` of string values.
pub fn string_map(map: &IndexMap<String, String>) -> AttributeValue {
    AttributeValue::Object(
        map.iter()
            .map(|(k, v)| (k.clone(), AttributeValue::String(v.clone())))
            .collect(),
    )
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

    /// Regression: serde's struct deserializer accepts a sequence by positional
    /// field assignment by default. Without the right variant order on
    /// `AttributeValue`, the array `["a", "b"]` would be misrouted to
    /// `Expression { raw: Some("a"), env: Some("b"), ... }`. This pins
    /// the order so that arrays land in `AttributeValue::Array`.
    #[test]
    fn string_array_value_deserializes_as_array_not_expression() {
        let value: AttributeValue = serde_json::from_str(r#"["a", "b"]"#).unwrap();
        match value {
            AttributeValue::Array(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0], AttributeValue::String(ref s) if s == "a"));
                assert!(matches!(items[1], AttributeValue::String(ref s) if s == "b"));
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }

    /// Regression: a generic object whose keys aren't Expression heads must
    /// land in `AttributeValue::Object`, not in `Expression` (which would
    /// silently match because of all-optional fields). `deny_unknown_fields`
    /// on `Expression` is what makes the dispatch fall through.
    #[test]
    fn generic_object_value_deserializes_as_object_not_expression() {
        let value: AttributeValue =
            serde_json::from_str(r#"{"msg": "message", "level": "level"}"#).unwrap();
        match value {
            AttributeValue::Object(map) => {
                assert_eq!(map.len(), 2);
                assert!(map.contains_key("msg"));
                assert!(map.contains_key("level"));
            }
            other => panic!("expected Object, got {other:?}"),
        }
    }

    /// Expression-shaped objects (matching the known head set) still dispatch
    /// to `AttributeValue::Expression` — only generic objects fall through.
    #[test]
    fn ref_shaped_object_deserializes_as_expression() {
        let value: AttributeValue =
            serde_json::from_str(r#"{"ref": "loki.write.gateway.receiver"}"#).unwrap();
        match value {
            AttributeValue::Expression(expr) => {
                assert_eq!(
                    expr.ref_name.as_deref(),
                    Some("loki.write.gateway.receiver")
                );
            }
            other => panic!("expected Expression, got {other:?}"),
        }
    }
}
