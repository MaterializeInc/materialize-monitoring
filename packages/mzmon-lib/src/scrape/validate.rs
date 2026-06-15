// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! JSONSchema validation of Monitor documents against the upstream CRD schemas.
//!
//! Unlike `alloy::validate`, we do NOT hand-author these schemas: they are the
//! real `prometheus-operator` CRD OpenAPI v3 schemas, extracted from the
//! vendored chart by `bin/extract-crd-schemas.sh` and checked in under
//! `schemas/scrape/`. Each is self-contained (no cross-file `$ref`s), so there
//! is one standalone validator per kind — no `Registry` wiring needed.
//!
//! Caveat: these are Kubernetes *structural* schemas (OpenAPI v3 with
//! `x-kubernetes-*` extensions). The `jsonschema` crate ignores unknown
//! keywords, and the CRDs do not set `additionalProperties: false`, so this
//! catches bad enums / types / missing required fields but not stray keys —
//! the typed `serde` deserialize in `transpile::Monitor::from_yaml_str` is the
//! backstop for those.

use std::sync::LazyLock;

use jsonschema::Validator;
use serde_json::Value;

use crate::scrape::error::{Error, Result};

const SCHEMA_PODMONITOR: &str = include_str!("../../schemas/scrape/podmonitor.schema.yaml");
const SCHEMA_SERVICEMONITOR: &str = include_str!("../../schemas/scrape/servicemonitor.schema.yaml");
const SCHEMA_SCRAPECONFIG: &str = include_str!("../../schemas/scrape/scrapeconfig.schema.yaml");

/// The Monitor kinds we validate. Selects which embedded schema to use.
#[derive(Clone, Copy, Debug)]
pub enum MonitorKind {
    PodMonitor,
    ServiceMonitor,
    ScrapeConfig,
}

/// Parse an embedded schema (authored as YAML) into a JSON value.
fn parse_schema(src: &str) -> Value {
    serde_yaml_ng::from_str(src).expect("embedded CRD schema is valid YAML")
}

/// Compile one embedded CRD schema into a validator. Panics on failure since
/// the schemas are compile-time constants — a failure here is a bug, not a user
/// error.
fn build_validator(src: &str) -> Validator {
    jsonschema::options()
        .build(&parse_schema(src))
        .expect("embedded CRD schema compiles into a validator")
}

static PODMONITOR_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| build_validator(SCHEMA_PODMONITOR));
static SERVICEMONITOR_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| build_validator(SCHEMA_SERVICEMONITOR));
static SCRAPECONFIG_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| build_validator(SCHEMA_SCRAPECONFIG));

/// Validate a Monitor document against its CRD schema, collecting *all*
/// violations into a single [`Error::Multiple`].
pub fn validate(kind: MonitorKind, instance: &Value) -> Result<()> {
    let validator: &Validator = match kind {
        MonitorKind::PodMonitor => &PODMONITOR_VALIDATOR,
        MonitorKind::ServiceMonitor => &SERVICEMONITOR_VALIDATOR,
        MonitorKind::ScrapeConfig => &SCRAPECONFIG_VALIDATOR,
    };

    let errors: Vec<Error> = validator
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: all three embedded CRD schemas compile into validators. This
    /// is the early canary for the "does the `jsonschema` crate accept k8s
    /// OpenAPI v3 schemas" risk.
    #[test]
    fn all_crd_schemas_compile() {
        LazyLock::force(&PODMONITOR_VALIDATOR);
        LazyLock::force(&SERVICEMONITOR_VALIDATOR);
        LazyLock::force(&SCRAPECONFIG_VALIDATOR);
    }
}
