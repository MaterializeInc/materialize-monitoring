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

use std::sync::LazyLock;

use jsonschema::Validator;
use serde_json::Value;

use crate::query::error::{Error, Result};

/// The embedded query-registry schema (authored as YAML).
const SCHEMA: &str = include_str!("../../schemas/query/mzmon-query.schema.yaml");

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
pub fn validate(instance: &Value) -> Result<()> {
    let errors: Vec<Error> = VALIDATOR
        .iter_errors(instance)
        .map(|err| Error::Schema {
            path: err.instance_path().to_string(),
            message: err.to_string(),
        })
        .collect();

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
