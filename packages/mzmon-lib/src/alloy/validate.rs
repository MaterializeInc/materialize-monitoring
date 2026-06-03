// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! JSONSchema validation of pipeline documents.
//!
//! The schemas under `schemas/alloy/` are embedded at compile time and compiled
//! into a single [`jsonschema::Validator`]. Cross-schema `$ref`s (which use the
//! `$id` URLs below) resolve against the embedded copies, so validation never
//! touches the network or filesystem.

use std::sync::LazyLock;

use jsonschema::Validator;
use serde_json::Value;

use crate::alloy::error::{Error, Result};

// Embedded schema sources (compiled into the binary).
const SCHEMA_MZMON_ALLOY: &str = include_str!("../../schemas/alloy/mzmon-alloy.schema.yaml");
const SCHEMA_TOP: &str = include_str!("../../schemas/alloy/top.schema.yaml");
const SCHEMA_RAW: &str = include_str!("../../schemas/alloy/common/raw.schema.yaml");
const SCHEMA_ATTRIBUTE: &str = include_str!("../../schemas/alloy/common/attribute.schema.yaml");
const SCHEMA_EXPRESSION: &str = include_str!("../../schemas/alloy/common/expression.schema.yaml");
const SCHEMA_LOKI: &str = include_str!("../../schemas/alloy/loki.schema.yaml");
const SCHEMA_DISCOVERY: &str = include_str!("../../schemas/alloy/discovery.schema.yaml");

// The `$id` URLs the schemas reference one another by. These must match the
// `$id` fields in the schema files (and the relative `$ref`s resolve to them).
const ID_TOP: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/top.schema.yaml";
const ID_RAW: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/common/raw.schema.yaml";
const ID_ATTRIBUTE: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/common/attribute.schema.yaml";
const ID_EXPRESSION: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/common/expression.schema.yaml";
const ID_LOKI: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/loki.schema.yaml";
const ID_DISCOVERY: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/discovery.schema.yaml";

/// Parse an embedded schema (authored as YAML) into a JSON value.
fn parse_schema(src: &str) -> Value {
    serde_yaml_ng::from_str(src).expect("embedded schema is valid YAML")
}

/// Build the compiled validator from the embedded schemas. Panics on failure
/// since the schemas are compile-time constants — a failure here is a bug, not
/// a user error.
fn build_validator() -> Validator {
    use jsonschema::{Registry, Resource};

    // Register the cross-referenced schemas by their `$id` so that the relative
    // `$ref`s in the documents resolve against the embedded copies.
    let registry = Registry::new()
        .extend([
            (ID_TOP, Resource::from_contents(parse_schema(SCHEMA_TOP))),
            (ID_RAW, Resource::from_contents(parse_schema(SCHEMA_RAW))),
            (
                ID_ATTRIBUTE,
                Resource::from_contents(parse_schema(SCHEMA_ATTRIBUTE)),
            ),
            (
                ID_EXPRESSION,
                Resource::from_contents(parse_schema(SCHEMA_EXPRESSION)),
            ),
            (ID_LOKI, Resource::from_contents(parse_schema(SCHEMA_LOKI))),
            (
                ID_DISCOVERY,
                Resource::from_contents(parse_schema(SCHEMA_DISCOVERY)),
            ),
        ])
        .expect("register embedded schema resources")
        .prepare()
        .expect("prepare embedded schema registry");

    let root = parse_schema(SCHEMA_MZMON_ALLOY);
    jsonschema::options()
        .with_registry(&registry)
        .build(&root)
        .expect("embedded alloy schemas compile into a validator")
}

static VALIDATOR: LazyLock<Validator> = LazyLock::new(build_validator);

/// Validate a pipeline document against the embedded JSONSchema.
///
/// Collects *all* violations into a single [`Error::Multiple`] so callers can
/// surface every problem at once rather than failing on the first.
///
/// Common violation patterns get an extra `hint:` line explaining the
/// `raw:` escape vs. extend-the-schema choice — see [`schema_hint`].
pub fn validate(instance: &Value) -> Result<()> {
    let errors: Vec<Error> = VALIDATOR
        .iter_errors(instance)
        .map(|err| {
            let path = err.instance_path().to_string();
            let mut message = err.to_string();
            if let Some(hint) = schema_hint(&path, &message) {
                message.push_str("\n  hint: ");
                message.push_str(hint);
            }
            Error::Schema { path, message }
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Multiple(errors))
    }
}

/// If a violation matches a known "schema doesn't cover this" pattern, return
/// a short hint that explains the project's strict-attributes + raw-escape
/// policy. Pattern-matches the underlying jsonschema error text; if the text
/// changes in a future jsonschema release, we just stop emitting the hint
/// (no functional impact).
///
/// See: docs/content/reference/internal/pipelines/authoring.md
fn schema_hint(_path: &str, message: &str) -> Option<&'static str> {
    let msg = message.to_ascii_lowercase();

    // The most actionable case: `additionalProperties: false` rejected an
    // undocumented key (either an unknown attribute on a typed block, or an
    // unknown component / sub-block).
    if msg.contains("additional properties are not allowed")
        || msg.contains("additional properties are not permitted")
    {
        return Some(
            "this key isn't typed in the schema. \
             Either use a `raw:` block for one-off usage, \
             or extend the relevant schema $def to add it. \
             See: docs/content/reference/internal/pipelines/authoring.md",
        );
    }

    // `oneOf` wrapper: the block didn't match any typed branch. The bare jsonschema
    // message ("not valid under any of the schemas listed in the 'oneOf' keyword")
    // doesn't tell the author what to do — surface the same policy here.
    //
    // The exact root cause (unknown component vs. unsupported attribute inside a
    // recognized one) isn't distinguishable without drilling into per-branch
    // errors, which `oneOf` makes noisy. A single shared hint covers both.
    if msg.contains("is not valid under any of the schemas") {
        return Some(
            "this block doesn't match any typed schema. \
             Likely either an unknown component or an attribute outside the documented set. \
             Use a `raw:` block for one-off cases, or extend the relevant schema $def. \
             See: docs/content/reference/internal/pipelines/authoring.md",
        );
    }

    None
}
