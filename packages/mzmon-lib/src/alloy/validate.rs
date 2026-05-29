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
const SCHEMA_RAW: &str = include_str!("../../schemas/alloy/raw.schema.yaml");
const SCHEMA_LOKI: &str = include_str!("../../schemas/alloy/loki.schema.yaml");

// The `$id` URLs the schemas reference one another by. These must match the
// `$id` fields in the schema files (and the relative `$ref`s resolve to them).
const ID_TOP: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/top.schema.yaml";
const ID_RAW: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/raw.schema.yaml";
const ID_LOKI: &str = "https://materializeinc.github.io/materialize-monitoring/reference/internal/schemas/alloy/loki.schema.yaml";

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
            (ID_LOKI, Resource::from_contents(parse_schema(SCHEMA_LOKI))),
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
