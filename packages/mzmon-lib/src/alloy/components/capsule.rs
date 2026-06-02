// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! First-class types for alloy values that components exchange by reference:
//! *capsules* (`loki.LogsReceiver`, `RelabelRules`, ...) and targets
//! (documented upstream as `list(map(string))`).
//!
//! These exist to make the "ref-valued attributes render as bare refs, never
//! quoted strings" invariant unrepresentable to violate: the only way to turn
//! one of these into an [`AttributeValue`] is through a conversion that emits
//! an `Expression::ref_name`.
//!
//! Naming follows the alloy reference docs' type vocabulary, so a field
//! declared `forward_to: Vec<LogsReceiver>` reads exactly like the upstream
//! component documentation.
//!
//! See: https://grafana.com/docs/alloy/latest/get-started/expressions/types_and_values/#capsules

use crate::alloy::ast::{AttributeValue, Expression, Identifier, string_map};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Reference to a component's exported `loki.LogsReceiver`
/// (e.g. `loki.write.gateway.receiver`).
///
/// Deserializes from a plain YAML string; renders as a bare ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogsReceiver(pub Identifier);

impl LogsReceiver {
    pub fn new(name: impl Into<Identifier>) -> Self {
        Self(name.into())
    }
}

/// Reference to a component's exported `RelabelRules`
/// (e.g. `loki.relabel.filtered.rules`).
///
/// Deserializes from a plain YAML string; renders as a bare ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelabelRules(pub Identifier);

impl RelabelRules {
    pub fn new(name: impl Into<Identifier>) -> Self {
        Self(name.into())
    }
}

/// One element of a `targets` list.
///
/// Upstream documents targets as `list(map(string))` — a list of maps with
/// string values. At validate time alloy implements target elements as
/// capsules (`alloy validate` errors say `expected capsule`) and *flattens*
/// list-valued elements — behavior we rely on but which is not documented
/// upstream (verified against `alloy validate`; pinned by the round-trip
/// tests). A single array may therefore legally mix references to
/// `discovery.*` exports with inline literal targets:
///
/// Some targets have required keys (like `__path__` for file targets),
/// so those are represented only in the schema and not the rust type.
///
/// ```text
/// targets = [
///     discovery.relabel.pods.output,
///     {__path__ = "/var/log/app.log", job = "app"},
/// ]
/// ```
///
/// In YAML, a plain string element is a ref and a map element is a literal
/// target. There is no ambiguity: targets are always maps, never strings.
//
// NOTE: like `AttributeValue`, this needs the right serde representation so
// a plain string lands in `Ref` and a map lands in `Literal`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TargetEntry {
    /// Reference to a targets export (`list(map(string))` upstream),
    /// e.g. `discovery.kubernetes.pods.targets`.
    Ref(Identifier),
    /// Inline literal string map.
    /// This may have additional required keys in the schema (like `__path__` for file targets).
    Literal(IndexMap<String, String>),
}

impl From<&LogsReceiver> for AttributeValue {
    fn from(receiver: &LogsReceiver) -> Self {
        AttributeValue::Expression(Expression::name_to_ref(&receiver.0))
    }
}

impl From<&RelabelRules> for AttributeValue {
    fn from(rules: &RelabelRules) -> Self {
        AttributeValue::Expression(Expression::name_to_ref(&rules.0))
    }
}

impl From<&TargetEntry> for AttributeValue {
    fn from(entry: &TargetEntry) -> Self {
        match entry {
            TargetEntry::Ref(r) => AttributeValue::Expression(Expression::name_to_ref(r)),
            TargetEntry::Literal(m) => string_map(m),
        }
    }
}

/// Wrap a `forward_to`-style list as an `AttributeValue::Array` of bare refs.
///
/// Replaces the per-file `target_refs()` helpers.
pub fn logs_receiver_list(receivers: &[LogsReceiver]) -> AttributeValue {
    AttributeValue::Array(receivers.iter().map(AttributeValue::from).collect())
}

/// Wrap a `targets` list as an `AttributeValue::Array` of ref/literal entries.
pub fn target_list(targets: &[TargetEntry]) -> AttributeValue {
    AttributeValue::Array(targets.iter().map(AttributeValue::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloy::ast::AttributeValue;

    // ----- LogsReceiver -----

    #[test]
    fn logs_receiver_deserializes_from_plain_yaml_string() {
        let r: LogsReceiver = serde_yaml_ng::from_str("loki.write.gateway.receiver").unwrap();
        assert_eq!(r, LogsReceiver("loki.write.gateway.receiver".into()));
    }

    #[test]
    fn logs_receiver_serializes_as_plain_string() {
        // The YAML shape is a bare string, not a wrapper map — the newtype
        // must stay invisible in the document.
        let yaml =
            serde_yaml_ng::to_string(&LogsReceiver("loki.write.gateway.receiver".into())).unwrap();
        assert_eq!(yaml.trim(), "loki.write.gateway.receiver");
    }

    #[test]
    fn logs_receiver_converts_to_bare_ref_expression() {
        let v = AttributeValue::from(&LogsReceiver("loki.write.gateway.receiver".into()));
        match v {
            AttributeValue::Expression(e) => {
                assert_eq!(e.ref_name.as_deref(), Some("loki.write.gateway.receiver"));
            }
            other => panic!("expected Expression with ref_name, got {other:?}"),
        }
    }

    #[test]
    fn logs_receiver_list_wraps_each_as_bare_ref() {
        let v = logs_receiver_list(&[
            LogsReceiver("loki.process.a.receiver".into()),
            LogsReceiver("loki.write.b.receiver".into()),
        ]);
        match v {
            AttributeValue::Array(items) => {
                assert_eq!(items.len(), 2);
                for (item, expected) in items
                    .iter()
                    .zip(["loki.process.a.receiver", "loki.write.b.receiver"])
                {
                    match item {
                        AttributeValue::Expression(e) => {
                            assert_eq!(e.ref_name.as_deref(), Some(expected));
                        }
                        other => panic!("expected Expression, got {other:?}"),
                    }
                }
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }

    // ----- RelabelRules -----

    #[test]
    fn relabel_rules_converts_to_bare_ref_expression() {
        let v = AttributeValue::from(&RelabelRules("loki.relabel.filtered.rules".into()));
        match v {
            AttributeValue::Expression(e) => {
                assert_eq!(e.ref_name.as_deref(), Some("loki.relabel.filtered.rules"));
            }
            other => panic!("expected Expression with ref_name, got {other:?}"),
        }
    }

    // ----- TargetEntry -----

    #[test]
    fn target_entry_plain_string_deserializes_as_ref() {
        let e: TargetEntry = serde_yaml_ng::from_str("discovery.relabel.pods.output").unwrap();
        assert_eq!(e, TargetEntry::Ref("discovery.relabel.pods.output".into()));
    }

    #[test]
    fn target_entry_map_deserializes_as_literal() {
        let e: TargetEntry =
            serde_yaml_ng::from_str("__path__: /var/log/app.log\njob: app").unwrap();
        match e {
            TargetEntry::Literal(m) => {
                assert_eq!(
                    m.get("__path__").map(String::as_str),
                    Some("/var/log/app.log")
                );
                assert_eq!(m.get("job").map(String::as_str), Some("app"));
            }
            other => panic!("expected Literal, got {other:?}"),
        }
    }

    #[test]
    fn target_entry_ref_converts_to_bare_ref_expression() {
        let v = AttributeValue::from(&TargetEntry::Ref(
            "discovery.kubernetes.pods.targets".into(),
        ));
        match v {
            AttributeValue::Expression(e) => {
                assert_eq!(
                    e.ref_name.as_deref(),
                    Some("discovery.kubernetes.pods.targets")
                );
            }
            other => panic!("expected Expression with ref_name, got {other:?}"),
        }
    }

    #[test]
    fn target_entry_literal_converts_to_string_object() {
        let mut m = IndexMap::new();
        m.insert("__path__".to_string(), "/var/log/app.log".to_string());
        let v = AttributeValue::from(&TargetEntry::Literal(m));
        match v {
            AttributeValue::Object(o) => {
                assert!(
                    matches!(o.get("__path__"), Some(AttributeValue::String(s)) if s == "/var/log/app.log")
                );
            }
            other => panic!("expected Object, got {other:?}"),
        }
    }

    #[test]
    fn target_list_mixes_refs_and_literals() {
        let mut m = IndexMap::new();
        m.insert("__path__".to_string(), "/var/log/app.log".to_string());
        let v = target_list(&[
            TargetEntry::Ref("discovery.relabel.pods.output".into()),
            TargetEntry::Literal(m),
        ]);
        match v {
            AttributeValue::Array(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], AttributeValue::Expression(_)));
                assert!(matches!(&items[1], AttributeValue::Object(_)));
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }
}
