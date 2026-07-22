// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Raw YAML shapes for the query registry, plus their conversion into the
//! resolved [`crate::query::model`] types.
//!
//! These `*Def` structs mirror `mzmon-query.schema.yaml` and the Python
//! `TypedDict`s / `from_entry` classmethods one-to-one. The loader is
//! deliberately lenient (no `deny_unknown_fields`) — structural strictness is
//! the schema validator's job ([`crate::query::validate`]), matching the Python
//! split where `ajv` validates and the registry merely loads.
//!
//! The four template fields (`promQL`, `datadogSQL`, `honeycombSQL`, `logQL`)
//! are kept as untyped [`serde_json::Value`] and lowered by
//! [`template_exprs_from_value`], a direct port of
//! `TemplateExpr.from_entry` / `TemplateFunction.from_entry`. That value can be
//! a bare string, an object, or a list of either, which is far cleaner to walk
//! by hand than to model with nested `#[serde(untagged)]` enums.

use serde::Deserialize;
use serde_json::Value;

use crate::query::error::{Error, Result};
use crate::query::importance::Importance;
use crate::query::model::{Description, TemplateExpr, TemplateFunction};
use crate::query::stability::Stability;

/// A whole registry file: a description, a metric-importance hint, and any of
/// queries / rules / alerts / metric overrides. (The schema requires the hint and
/// at least one content branch; the loader does not, matching the Python `load()`
/// which simply iterates whichever keys are present.)
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryDoc {
    #[serde(default)]
    pub description: String,
    /// Default importance for every metric this file's queries reference. The
    /// schema requires it; a missing hint defaults to [`Importance::default`].
    #[serde(default, rename = "metricImportanceHint")]
    pub metric_importance_hint: Importance,
    #[serde(default)]
    pub queries: Vec<QueryDef>,
    #[serde(default)]
    pub rules: Vec<RuleDef>,
    #[serde(default)]
    pub alerts: Vec<AlertDef>,
    #[serde(default, rename = "metricOverrides")]
    pub metric_overrides: Vec<MetricOverrideDef>,
}

/// A metric-importance override: set every metric matching `metric_pattern` to
/// `importance` outright. See [`crate::query::registry::MetricOverride`].
#[derive(Debug, Clone, Deserialize)]
pub struct MetricOverrideDef {
    #[serde(rename = "metricPattern")]
    pub metric_pattern: String,
    pub importance: Importance,
    #[serde(default)]
    pub priority: i64,
}

impl RegistryDoc {
    /// Parse a registry file's YAML text.
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        Ok(serde_yaml_ng::from_str(yaml)?)
    }
}

/// The structured description block shared by queries, rules, and alerts.
#[derive(Debug, Clone, Deserialize)]
pub struct DescriptionDef {
    pub summary: String,
    #[serde(default)]
    pub nominal: Option<String>,
    #[serde(default)]
    pub degraded: Option<String>,
    #[serde(default)]
    pub unhealthy: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl From<DescriptionDef> for Description {
    fn from(d: DescriptionDef) -> Self {
        Description {
            summary: d.summary,
            nominal: d.nominal,
            degraded: d.degraded,
            unhealthy: d.unhealthy,
            notes: d.notes,
        }
    }
}

/// A single query definition. The irregular `promQL` / `datadogSQL` /
/// `honeycombSQL` / `logQL` key casings are renamed explicitly (a blanket
/// `rename_all` can't produce them).
#[derive(Debug, Clone, Deserialize)]
pub struct QueryDef {
    pub id: String,
    pub description: DescriptionDef,
    pub stability: Stability,
    #[serde(default)]
    pub dependencies: Vec<DependencyDef>,
    #[serde(default, rename = "promQL")]
    pub promql: Option<Value>,
    #[serde(default, rename = "datadogSQL")]
    pub datadog_sql: Option<Value>,
    #[serde(default, rename = "honeycombSQL")]
    pub honeycomb_sql: Option<Value>,
    #[serde(default, rename = "logQL")]
    pub logql: Option<Value>,
    #[serde(default)]
    pub instant: Option<bool>,
}

/// A dependency: either a bare query id, or an inline query definition that gets
/// promoted to a top-level registry entry at load time.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DependencyDef {
    Id(String),
    Inline(Box<QueryDef>),
}

/// A recording rule. Its dependency is supplied as exactly one of `queryId`
/// (reference) or `query` (inline, promoted at load time).
#[derive(Debug, Clone, Deserialize)]
pub struct RuleDef {
    pub record: String,
    pub group: String,
    pub description: DescriptionDef,
    pub stability: Stability,
    #[serde(default)]
    pub labels: indexmap::IndexMap<String, String>,
    #[serde(default, rename = "queryId")]
    pub query_id: Option<String>,
    #[serde(default)]
    pub query: Option<Box<QueryDef>>,
}

/// An alerting rule. Like [`RuleDef`], its dependency is one of `queryId` /
/// `query`.
#[derive(Debug, Clone, Deserialize)]
pub struct AlertDef {
    pub alert: String,
    pub group: String,
    pub description: DescriptionDef,
    pub stability: Stability,
    #[serde(rename = "for")]
    pub for_: String,
    #[serde(default, rename = "keepFiringFor")]
    pub keep_firing_for: Option<String>,
    #[serde(default)]
    pub labels: indexmap::IndexMap<String, String>,
    #[serde(default)]
    pub annotations: indexmap::IndexMap<String, String>,
    #[serde(default, rename = "queryId")]
    pub query_id: Option<String>,
    #[serde(default)]
    pub query: Option<Box<QueryDef>>,
}

/// Lower a raw template value into a list of [`TemplateExpr`].
///
/// Direct port of `TemplateExpr.from_entry`: `None`/null → empty; a list flattens
/// (recursively); a string is a bare inline template; an object carries
/// `template` **or** `queryId` plus optional `functions`. Exactly one of
/// `template` / `queryId` must be present on an object, matching the Python
/// `__post_init__` invariant.
pub fn template_exprs_from_value(entry: Option<&Value>) -> Result<Vec<TemplateExpr>> {
    let Some(entry) = entry else {
        return Ok(Vec::new());
    };
    match entry {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                out.extend(template_exprs_from_value(Some(item))?);
            }
            Ok(out)
        }
        Value::String(s) => Ok(vec![TemplateExpr::template(s.clone())]),
        Value::Object(map) => {
            let template = string_field(map, "template")?;
            let query_id = string_field(map, "queryId")?;
            if template.is_some() == query_id.is_some() {
                // Both set, or neither set.
                return Err(Error::InvalidTemplateExpr);
            }
            let functions = match map.get("functions") {
                Some(Value::Array(fns)) => fns
                    .iter()
                    .map(template_function_from_value)
                    .collect::<Result<Vec<_>>>()?,
                None | Some(Value::Null) => Vec::new(),
                Some(other) => {
                    return Err(Error::Schema {
                        path: "functions".to_string(),
                        message: format!("expected a list of functions, got {other}"),
                    });
                }
            };
            Ok(vec![TemplateExpr {
                template,
                query_id,
                functions,
            }])
        }
        other => Err(Error::Schema {
            path: "template".to_string(),
            message: format!("expected a string, object, or list, got {other}"),
        }),
    }
}

/// Lower one entry of a template's `functions` list. Port of
/// `TemplateFunction.from_entry`: a bare string is the function name (no args);
/// an object carries `name` plus optional `args` (themselves template values).
fn template_function_from_value(entry: &Value) -> Result<TemplateFunction> {
    match entry {
        Value::String(name) => Ok(TemplateFunction {
            name: name.clone(),
            args: Vec::new(),
        }),
        Value::Object(map) => {
            let name = string_field(map, "name")?.ok_or_else(|| Error::Schema {
                path: "functions[].name".to_string(),
                message: "function entry is missing required `name`".to_string(),
            })?;
            let args = template_exprs_from_value(map.get("args"))?;
            Ok(TemplateFunction { name, args })
        }
        other => Err(Error::Schema {
            path: "functions[]".to_string(),
            message: format!("expected a string or object, got {other}"),
        }),
    }
}

/// Read an optional string field from a JSON object, erroring if present but not
/// a string.
fn string_field(map: &serde_json::Map<String, Value>, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(Error::Schema {
            path: key.to_string(),
            message: format!("expected a string, got {other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn none_and_null_yield_empty() {
        assert!(template_exprs_from_value(None).unwrap().is_empty());
        assert!(
            template_exprs_from_value(Some(&Value::Null))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bare_string_is_an_inline_template() {
        let exprs = template_exprs_from_value(Some(&json(r#""up{}""#))).unwrap();
        assert_eq!(exprs, vec![TemplateExpr::template("up{}")]);
    }

    #[test]
    fn list_flattens_recursively() {
        let exprs = template_exprs_from_value(Some(&json(r#"["a", ["b", "c"]]"#))).unwrap();
        assert_eq!(
            exprs,
            vec![
                TemplateExpr::template("a"),
                TemplateExpr::template("b"),
                TemplateExpr::template("c"),
            ]
        );
    }

    #[test]
    fn object_with_template_and_functions() {
        let exprs = template_exprs_from_value(Some(&json(
            r#"{"template": "x{}", "functions": [{"name": "orZero"}]}"#,
        )))
        .unwrap();
        assert_eq!(exprs.len(), 1);
        assert_eq!(exprs[0].template.as_deref(), Some("x{}"));
        assert_eq!(exprs[0].functions.len(), 1);
        assert_eq!(exprs[0].functions[0].name, "orZero");
        assert!(exprs[0].functions[0].args.is_empty());
    }

    #[test]
    fn function_with_args_and_bare_string_name() {
        let exprs = template_exprs_from_value(Some(&json(
            r#"{"template": "x{}", "functions": ["orZero", {"name": "mzClusterName", "args": ["instance_id"]}]}"#,
        )))
        .unwrap();
        let fns = &exprs[0].functions;
        assert_eq!(fns[0].name, "orZero");
        assert_eq!(fns[1].name, "mzClusterName");
        assert_eq!(fns[1].args, vec![TemplateExpr::template("instance_id")]);
    }

    #[test]
    fn query_id_reference() {
        let exprs = template_exprs_from_value(Some(&json(r#"{"queryId": "some.other"}"#))).unwrap();
        assert_eq!(exprs, vec![TemplateExpr::reference("some.other")]);
    }

    #[test]
    fn both_template_and_query_id_is_rejected() {
        let err = template_exprs_from_value(Some(&json(r#"{"template": "x", "queryId": "y"}"#)))
            .unwrap_err();
        assert!(matches!(err, Error::InvalidTemplateExpr));
    }

    #[test]
    fn neither_template_nor_query_id_is_rejected() {
        let err = template_exprs_from_value(Some(&json(r#"{"functions": []}"#))).unwrap_err();
        assert!(matches!(err, Error::InvalidTemplateExpr));
    }

    #[test]
    fn function_missing_name_is_rejected() {
        let err = template_exprs_from_value(Some(&json(
            r#"{"template": "x", "functions": [{"args": ["a"]}]}"#,
        )))
        .unwrap_err();
        assert!(matches!(err, Error::Schema { .. }));
    }
}
