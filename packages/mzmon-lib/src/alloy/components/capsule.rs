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

/// Reference to a component's exported `MetricsReceiver`
/// (e.g. `prometheus.remote_write.default.receiver`).
///
/// The metrics-side analog of [`LogsReceiver`]: components exchange it through
/// `forward_to`, and `prometheus.echo` / `prometheus.relabel` /
/// `prometheus.remote_write` / `prometheus.receive_http` export a `receiver` of
/// this type. Deserializes from a plain YAML string; renders as a bare ref.
///
/// See: https://grafana.com/docs/alloy/latest/reference/compatibility/#metricsreceiver
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetricsReceiver(pub Identifier);

impl MetricsReceiver {
    pub fn new(name: impl Into<Identifier>) -> Self {
        Self(name.into())
    }
}

/// Reference to an otelcol component's exported consumer `input`
/// (e.g. `otelcol.exporter.prometheus.bridge.input`).
///
/// The otelcol analog of [`LogsReceiver`]/[`MetricsReceiver`]: otelcol
/// components hand telemetry to the next stage through their `output` block's
/// `metrics` / `logs` / `traces` lists, each element being an `otelcol.Consumer`.
/// Deserializes from a plain YAML string; renders as a bare ref.
///
/// See: https://grafana.com/docs/alloy/latest/reference/compatibility/#opentelemetry-collector-consumer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OtelcolConsumer(pub Identifier);

impl OtelcolConsumer {
    pub fn new(name: impl Into<Identifier>) -> Self {
        Self(name.into())
    }
}

/// One element of a `targets` list.
///
/// A `discovery.*` export (e.g. `discovery.kubernetes.pods.targets`) is itself a
/// `list(discovery.Target)`; an inline literal target is a single
/// `discovery.Target` (a string map). alloy does NOT flatten a list literal, so
/// `targets = [discovery.kubernetes.pods.targets]` fails at load with
/// `conversion from '[]discovery.Target' is not supported` — even though `alloy
/// validate` accepts it (only a real config load catches it). [`target_list`]
/// therefore combines list-valued refs with `array.concat(...)` rather than
/// wrapping them in a bare array.
///
/// Some targets have required keys (like `__path__` for file targets),
/// so those are represented only in the schema and not the rust type.
///
/// In YAML, a plain string element is a ref and a map element is a literal
/// target. There is no ambiguity: literal targets are always maps, never strings.
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

impl From<&MetricsReceiver> for AttributeValue {
    fn from(receiver: &MetricsReceiver) -> Self {
        AttributeValue::Expression(Expression::name_to_ref(&receiver.0))
    }
}

impl From<&OtelcolConsumer> for AttributeValue {
    fn from(consumer: &OtelcolConsumer) -> Self {
        AttributeValue::Expression(Expression::name_to_ref(&consumer.0))
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

/// Wrap a `forward_to`-style list of `MetricsReceiver`s as an
/// `AttributeValue::Array` of bare refs.
///
/// Unlike `targets` (list-valued, see [`target_list`]), each `forward_to`
/// element is a single capsule, so list-wrapping is correct here.
pub fn metrics_receiver_list(receivers: &[MetricsReceiver]) -> AttributeValue {
    AttributeValue::Array(receivers.iter().map(AttributeValue::from).collect())
}

/// Wrap an otelcol `output` list (`metrics` / `logs` / `traces`) of
/// [`OtelcolConsumer`]s as an `AttributeValue::Array` of bare refs.
///
/// Like `forward_to`, each element is a single capsule, so list-wrapping is
/// correct here (unlike `targets`, see [`target_list`]).
pub fn otelcol_consumer_list(consumers: &[OtelcolConsumer]) -> AttributeValue {
    AttributeValue::Array(consumers.iter().map(AttributeValue::from).collect())
}

/// Render a `targets` value.
///
/// `discovery.*` refs export `list(discovery.Target)`; literal maps are a single
/// `discovery.Target`. A bare `[ref]` would be `list(list(Target))`, which alloy
/// rejects at load, so list-valued refs are combined with `array.concat`:
///   - one ref, no literals  → the ref directly (`targets = discovery.x.targets`)
///   - only literals         → an array literal (`targets = [{…}, {…}]`)
///   - multiple refs / mixed → `array.concat(ref, ref, [{…}])`
///
/// Order within `targets` is not semantically significant, so refs are grouped
/// ahead of literals rather than preserving interleaved document order.
pub fn target_list(targets: &[TargetEntry]) -> AttributeValue {
    let refs: Vec<AttributeValue> = targets
        .iter()
        .filter_map(|t| match t {
            TargetEntry::Ref(r) => Some(AttributeValue::Expression(Expression::name_to_ref(r))),
            TargetEntry::Literal(_) => None,
        })
        .collect();
    let literals: Vec<AttributeValue> = targets
        .iter()
        .filter_map(|t| match t {
            TargetEntry::Literal(m) => Some(string_map(m)),
            TargetEntry::Ref(_) => None,
        })
        .collect();

    // Only literal targets (or none): a plain array literal is the correct
    // `list(Target)`.
    if refs.is_empty() {
        return AttributeValue::Array(literals);
    }
    // A single list-valued ref: assign it directly — it already is a
    // `list(Target)`, and wrapping it in `[…]` would break at load.
    if refs.len() == 1 && literals.is_empty() {
        return refs.into_iter().next().expect("refs.len() == 1");
    }
    // Otherwise concat each list-valued ref, plus one array-literal arg holding
    // any inline literal targets.
    let mut arguments = refs;
    if !literals.is_empty() {
        arguments.push(AttributeValue::Array(literals));
    }
    AttributeValue::Expression(Expression {
        function: Some("array.concat".into()),
        arguments,
        ..Default::default()
    })
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

    // ----- MetricsReceiver -----

    #[test]
    fn metrics_receiver_deserializes_from_plain_yaml_string() {
        let r: MetricsReceiver =
            serde_yaml_ng::from_str("prometheus.remote_write.default.receiver").unwrap();
        assert_eq!(
            r,
            MetricsReceiver("prometheus.remote_write.default.receiver".into())
        );
    }

    #[test]
    fn metrics_receiver_converts_to_bare_ref_expression() {
        let v = AttributeValue::from(&MetricsReceiver(
            "prometheus.remote_write.default.receiver".into(),
        ));
        match v {
            AttributeValue::Expression(e) => {
                assert_eq!(
                    e.ref_name.as_deref(),
                    Some("prometheus.remote_write.default.receiver")
                );
            }
            other => panic!("expected Expression with ref_name, got {other:?}"),
        }
    }

    #[test]
    fn metrics_receiver_list_wraps_each_as_bare_ref() {
        let v = metrics_receiver_list(&[
            MetricsReceiver("prometheus.relabel.a.receiver".into()),
            MetricsReceiver("prometheus.remote_write.b.receiver".into()),
        ]);
        match v {
            AttributeValue::Array(items) => {
                assert_eq!(items.len(), 2);
                for (item, expected) in items.iter().zip([
                    "prometheus.relabel.a.receiver",
                    "prometheus.remote_write.b.receiver",
                ]) {
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

    // ----- OtelcolConsumer -----

    #[test]
    fn otelcol_consumer_deserializes_from_plain_yaml_string() {
        let c: OtelcolConsumer =
            serde_yaml_ng::from_str("otelcol.exporter.prometheus.bridge.input").unwrap();
        assert_eq!(
            c,
            OtelcolConsumer("otelcol.exporter.prometheus.bridge.input".into())
        );
    }

    #[test]
    fn otelcol_consumer_list_wraps_each_as_bare_ref() {
        let v = otelcol_consumer_list(&[
            OtelcolConsumer("otelcol.processor.batch.default.input".into()),
            OtelcolConsumer("otelcol.exporter.prometheus.bridge.input".into()),
        ]);
        match v {
            AttributeValue::Array(items) => {
                assert_eq!(items.len(), 2);
                for (item, expected) in items.iter().zip([
                    "otelcol.processor.batch.default.input",
                    "otelcol.exporter.prometheus.bridge.input",
                ]) {
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
    fn target_list_single_ref_is_unwrapped() {
        // A lone list-valued ref must be assigned directly — NOT wrapped in an
        // array, which alloy rejects at load ('[]discovery.Target' conversion).
        let v = target_list(&[TargetEntry::Ref("discovery.relabel.pods.output".into())]);
        match v {
            AttributeValue::Expression(e) => {
                assert_eq!(e.ref_name.as_deref(), Some("discovery.relabel.pods.output"));
                assert!(e.function.is_none());
            }
            other => panic!("expected a bare ref Expression, got {other:?}"),
        }
    }

    #[test]
    fn target_list_literals_only_is_an_array() {
        let mut m = IndexMap::new();
        m.insert("__path__".to_string(), "/var/log/app.log".to_string());
        let v = target_list(&[TargetEntry::Literal(m)]);
        match v {
            AttributeValue::Array(items) => {
                assert_eq!(items.len(), 1);
                assert!(matches!(&items[0], AttributeValue::Object(_)));
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }

    #[test]
    fn target_list_mixes_refs_and_literals_via_concat() {
        // A ref (list-valued) mixed with a literal (single Target) can't be a
        // bare array; it combines via `array.concat(ref, [literal])`.
        let mut m = IndexMap::new();
        m.insert("__path__".to_string(), "/var/log/app.log".to_string());
        let v = target_list(&[
            TargetEntry::Ref("discovery.relabel.pods.output".into()),
            TargetEntry::Literal(m),
        ]);
        match v {
            AttributeValue::Expression(e) => {
                assert_eq!(e.function.as_deref(), Some("array.concat"));
                assert_eq!(e.arguments.len(), 2);
                assert!(matches!(&e.arguments[0], AttributeValue::Expression(_)));
                match &e.arguments[1] {
                    AttributeValue::Array(lits) => {
                        assert_eq!(lits.len(), 1);
                        assert!(matches!(&lits[0], AttributeValue::Object(_)));
                    }
                    other => panic!("expected literal array arg, got {other:?}"),
                }
            }
            other => panic!("expected array.concat Expression, got {other:?}"),
        }
    }
}
