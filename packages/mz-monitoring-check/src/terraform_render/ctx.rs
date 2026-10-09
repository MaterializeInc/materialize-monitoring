// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Everything an assertion needs about one example, built once at startup.
//!
//! Three views of the same example, and the checks are deliberate about which
//! they read: what the example **declared** to the module (its input), what the
//! module **composed** for Helm (the module's output), and what the chart
//! **rendered** (what a cluster would run). A check compares one against
//! another; reading the expectation and the result from the same view would
//! assert nothing.

use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::Value;

use super::manifests::Manifests;

pub struct Ctx {
    plan: Value,
    values: Vec<Value>,
    composed: Value,
    pub rendered: Manifests,
    chart_values: Arc<Value>,
}

impl Ctx {
    pub fn new(
        plan: Value,
        values: Vec<Value>,
        rendered: Manifests,
        chart_values: Arc<Value>,
    ) -> Self {
        let mut composed = Value::Null;
        for document in &values {
            merge(&mut composed, document);
        }
        Self {
            plan,
            values,
            composed,
            rendered,
            chart_values,
        }
    }

    /// The literal the example passed for a module input, or `None` when it
    /// passed nothing, passed null, or passed an expression.
    ///
    /// Only literals are visible in a plan's configuration, which is why the
    /// examples spell every input out.
    pub fn declared(&self, input: &str) -> Option<&Value> {
        self.plan
            .pointer(&format!(
                "/configuration/root_module/module_calls/monitoring/expressions/{input}/constant_value"
            ))
            .filter(|value| !value.is_null())
    }

    /// Whether the example declared a non-empty, non-false value for `input`.
    pub fn declares(&self, input: &str) -> bool {
        self.declared(input).is_some_and(truthy)
    }

    pub fn declared_str(&self, input: &str) -> Option<&str> {
        self.declared(input).and_then(Value::as_str)
    }

    /// A field inside a declared object, by dotted path.
    pub fn declared_at(&self, input: &str, path: &str) -> Option<&Value> {
        self.declared(input).and_then(|value| lookup(value, path))
    }

    /// The planned values of one of the module's own resources.
    pub fn planned(&self, kind: &str, name: &str) -> Option<&Value> {
        self.plan
            .pointer("/planned_values/root_module/child_modules")?
            .as_array()?
            .iter()
            .filter_map(|module| module.get("resources")?.as_array())
            .flatten()
            .find(|resource| {
                resource.get("type").and_then(Value::as_str) == Some(kind)
                    && resource.get("name").and_then(Value::as_str) == Some(name)
            })?
            .get("values")
    }

    /// The values documents the module composed, in merge order.
    pub fn value_documents(&self) -> &[Value] {
        &self.values
    }

    /// A key in the composed values, merged the way Helm merges repeated `-f`.
    pub fn composed(&self, path: &str) -> Option<&Value> {
        lookup(&self.composed, path)
    }

    /// A key in the chart's own `values.yaml`.
    pub fn chart_default(&self, path: &str) -> Option<&Value> {
        lookup(&self.chart_values, path)
    }
}

/// Look up a dotted path. Fine for chart keys, which contain `-` but never `.`;
/// annotation and label keys do contain dots and are read with `get` instead.
pub fn lookup<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(value, |node, key| node.get(key))
}

/// Python's truthiness, which is what "the example set this" means for a
/// Terraform literal: null, false, zero and empty are all unset.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// The value at `path` as a string, for checks that compare against one.
pub fn str_at<'a>(value: &'a Value, path: &str) -> Option<&'a str> {
    lookup(value, path).and_then(Value::as_str)
}

pub fn parse_yaml(text: &str) -> Result<Value> {
    serde_yaml_ng::from_str(text).context("parsing YAML")
}

/// Helm's coalescing of one values document over another: maps merge key by
/// key, and anything else — lists included — replaces what was there.
fn merge(into: &mut Value, from: &Value) {
    match (into, from) {
        (Value::Object(into), Value::Object(from)) => {
            for (key, value) in from {
                merge(into.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (into, from) => *into = from.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_coalesces_maps_and_replaces_lists() {
        let mut values = json!({ "a": { "keep": 1, "list": [1, 2] }, "b": true });
        merge(&mut values, &json!({ "a": { "list": [3] }, "b": false }));
        assert_eq!(
            values,
            json!({ "a": { "keep": 1, "list": [3] }, "b": false })
        );
    }

    /// The node-exporter circuit breaker is read through this, and an earlier
    /// document must not win just because it came first.
    #[test]
    fn composed_reads_the_last_document_to_set_a_key() {
        let ctx = Ctx::new(
            json!({}),
            vec![
                json!({ "node-exporter": { "enabled": true } }),
                json!({ "other": 1 }),
                json!({ "node-exporter": { "enabled": false } }),
            ],
            Manifests::parse(String::new()).unwrap(),
            Arc::new(json!({})),
        );
        assert_eq!(ctx.composed("node-exporter.enabled"), Some(&json!(false)));
    }

    #[test]
    fn declared_reads_only_literals() {
        let ctx = Ctx::new(
            json!({ "configuration": { "root_module": { "module_calls": { "monitoring": {
                "expressions": {
                    "literal": { "constant_value": { "nested": "x" } },
                    "null": { "constant_value": null },
                    "expression": { "references": ["var.x"] },
                }
            } } } } }),
            vec![],
            Manifests::parse(String::new()).unwrap(),
            Arc::new(json!({})),
        );
        assert_eq!(
            ctx.declared_at("literal", "nested").and_then(Value::as_str),
            Some("x")
        );
        assert!(ctx.declared("null").is_none());
        assert!(ctx.declared("expression").is_none());
        assert!(!ctx.declares("missing"));
    }
}
