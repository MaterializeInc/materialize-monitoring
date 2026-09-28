// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! JSONSchema validation of registry files against
//! `schemas/query/mzmon-query.schema.yaml`.
//!
//! This is the hand-authored, strict schema (`additionalProperties` /
//! `unevaluatedProperties: false`) that today's pre-commit runs via `ajv`. The
//! embedded copy is the single source of truth so the Rust check and the
//! editor's `yaml-language-server` hint point at the same file. Format
//! assertions (`format: go-duration`) are advisory and not enforced, matching
//! the `ajv --validate-formats=false` invocation.
//!
//! Wiring the validator here is the groundwork for the future
//! `mz-monitoring-check check-queries` command that will replace the `ajv` hook.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use jsonschema::Validator;
use regex::Regex;
use serde_json::Value;

use crate::query::error::{Error, Result};

/// The embedded query-registry schema (authored as YAML).
pub(crate) const SCHEMA: &str = include_str!("../../schemas/query/mzmon-query.schema.yaml");

/// Compile the embedded schema into a validator. Panics on failure since the
/// schema is a compile-time constant — a failure here is a bug, not user error.
static VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let schema: Value =
        serde_yaml_ng::from_str(SCHEMA).expect("embedded query schema is valid YAML");
    jsonschema::options()
        .build(&schema)
        .expect("embedded query schema compiles into a validator")
});

/// Validate a parsed registry file against the schema, collecting *all*
/// violations into a single [`Error::Multiple`].
/// The schema's `knownParameter` enum: every `%%{name}` a template may use.
pub fn known_parameters() -> &'static BTreeSet<String> {
    static KNOWN: LazyLock<BTreeSet<String>> = LazyLock::new(|| {
        let schema: Value =
            serde_yaml_ng::from_str(SCHEMA).expect("embedded query schema is valid YAML");
        schema["$defs"]["knownParameter"]["enum"]
            .as_array()
            .expect("knownParameter is an enum")
            .iter()
            .map(|v| {
                v.as_str()
                    .expect("knownParameter values are strings")
                    .to_string()
            })
            .collect()
    });
    &KNOWN
}

/// Every `%%{name}` in any string of `value` whose `name` is not a known
/// parameter, with the JSON pointer of the string it appears in.
///
/// Template strings are free text as far as JSON Schema is concerned, so the
/// enum cannot be enforced by the schema itself; this is what makes a misspelt
/// placeholder a validation error rather than a render error in whichever
/// consumer first meets it.
fn unknown_placeholders(value: &Value, path: &str, out: &mut Vec<Error>) {
    static PLACEHOLDER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"%%\{([A-Za-z0-9_]+)\}").unwrap());
    match value {
        Value::String(s) => {
            for capture in PLACEHOLDER.captures_iter(s) {
                let name = &capture[1];
                if !known_parameters().contains(name) {
                    out.push(Error::Schema {
                        path: path.to_string(),
                        message: format!(
                            "`%%{{{name}}}` is not a known parameter (see `knownParameter` in the schema)"
                        ),
                    });
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                unknown_placeholders(item, &format!("{path}/{i}"), out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                unknown_placeholders(item, &format!("{path}/{key}"), out);
            }
        }
        _ => {}
    }
}

pub fn validate(instance: &Value) -> Result<()> {
    let mut errors: Vec<Error> = VALIDATOR
        .iter_errors(instance)
        .map(|err| Error::Schema {
            path: err.instance_path().to_string(),
            message: err.to_string(),
        })
        .collect();
    unknown_placeholders(instance, "", &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Multiple(errors))
    }
}

/// Parse `yaml` and validate it against the schema.
pub fn validate_yaml_str(yaml: &str) -> Result<()> {
    let instance: Value = serde_yaml_ng::from_str(yaml)?;
    validate(&instance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_compiles() {
        LazyLock::force(&VALIDATOR);
    }

    #[test]
    fn all_real_query_files_validate() {
        for (name, yaml) in crate::query::test_support::FIXTURES {
            validate_yaml_str(yaml)
                .unwrap_or_else(|err| panic!("fixture {name} failed schema validation: {err}"));
        }
    }

    #[test]
    fn missing_required_id_is_rejected() {
        let err = validate_yaml_str(
            r#"
description: test
queries:
  - stability: best-effort
    description: {summary: s}
    promQL: 'up{}'
"#,
        )
        .unwrap_err();
        assert!(matches!(err, Error::Multiple(_)));
    }

    #[test]
    fn unknown_stability_is_rejected() {
        let err = validate_yaml_str(
            r#"
description: test
queries:
  - id: q
    stability: super-stable
    description: {summary: s}
    promQL: 'up{}'
"#,
        )
        .unwrap_err();
        assert!(matches!(err, Error::Multiple(_)));
    }

    #[test]
    fn every_file_in_the_registry_validates() {
        // The fixture set leaves the alert files out; the pre-commit hook does
        // not, so neither does this.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../queries");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            let yaml = std::fs::read_to_string(&path).unwrap();
            validate_yaml_str(&yaml)
                .unwrap_or_else(|err| panic!("{} failed validation: {err}", path.display()));
            checked += 1;
        }
        assert!(
            checked > 10,
            "expected the whole registry, found {checked} files"
        );
    }

    #[test]
    fn unknown_placeholders_are_rejected() {
        let err = validate_yaml_str(
            r#"
description: test
metricImportanceHint: essential
queries:
  - id: q
    stability: best-effort
    description: {summary: s}
    promQL: 'up{%%{mzEnvironmentNamspaceFilter}}'
"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("mzEnvironmentNamspaceFilter"),
            "{err}"
        );
    }

    #[test]
    fn stray_alert_keys_are_rejected() {
        let err = validate_yaml_str(
            r#"
description: test
metricImportanceHint: essential
alerts:
  - alert: a
    group: g
    stability: best-effort
    for: 5m
    requries: [cilium]
    description: {summary: s}
    queryId: q
"#,
        )
        .unwrap_err();
        assert!(matches!(err, Error::Multiple(_)));
    }

    #[test]
    fn stray_top_level_key_is_rejected() {
        // `unevaluatedProperties: false` should reject unknown top-level keys.
        let err = validate_yaml_str(
            r#"
description: test
queries: []
bogusKey: nope
"#,
        )
        .unwrap_err();
        assert!(matches!(err, Error::Multiple(_)));
    }
}
