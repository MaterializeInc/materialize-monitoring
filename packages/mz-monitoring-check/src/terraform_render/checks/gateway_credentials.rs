// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `gateway_credentials`: destination credentials reach the gateway through the
//! module's Secret, and only through it.
//!
//! They never appear in the Helm values, so the coupling that can break is a
//! *name*: the values say which environment variable the pipeline reads, the
//! Secret says which one it sets, and for the OTLP header case the module derives
//! both from the header name. Nothing errors when those drift. The gateway
//! starts, `sys.env(...)` resolves empty, and the destination rejects every
//! request.
//!
//! The expectation is the credentials the example declared, compared by value.
//! That needs no copy of the module's naming scheme, and it keeps a module that
//! stops planning the Secret from switching these checks off.

use anyhow::{Result, bail};
use serde_json::Value;

use crate::terraform_render::ctx::Ctx;

/// Every credential the example declared for a gateway destination, labelled by
/// the input it came from.
pub fn declared(ctx: &Ctx) -> Vec<(String, &str)> {
    let mut out = Vec::new();
    for input in ["datadog_api_key", "otlp_auth_bearer_token"] {
        if let Some(value) = ctx.declared_str(input).filter(|value| !value.is_empty()) {
            out.push((input.to_owned(), value));
        }
    }
    let headers = ctx
        .declared("otlp_auth_header_secrets")
        .and_then(Value::as_object)
        .into_iter()
        .flatten();
    for (header, value) in headers {
        if let Some(value) = value.as_str().filter(|value| !value.is_empty()) {
            out.push((format!("otlp_auth_header_secrets.{header}"), value));
        }
    }
    let destinations = ctx
        .declared("prometheus_remote_write_credentials")
        .and_then(Value::as_object)
        .into_iter()
        .flatten();
    for (destination, credentials) in destinations {
        for field in ["username", "password", "bearer_token"] {
            if let Some(value) = credentials
                .get(field)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                out.push((
                    format!("prometheus_remote_write_credentials.{destination}.{field}"),
                    value,
                ));
            }
        }
    }
    out
}

/// Every declared credential is a value of the planned `alloy_gateway_env`
/// Secret.
pub fn reach_the_secret(ctx: &Ctx) -> Result<()> {
    let data = planned_data(ctx)?;
    let missing: Vec<String> = declared(ctx)
        .into_iter()
        .filter(|(_, value)| !data.values().any(|planned| planned.as_str() == Some(value)))
        .map(|(input, _)| input)
        .collect();
    if !missing.is_empty() {
        bail!(
            "the planned alloy_gateway_env Secret does not carry the value of: {}",
            missing.join(", ")
        );
    }
    Ok(())
}

/// Every key of the Secret is read by the rendered pipeline.
///
/// That catches a derived name changing on either side, and a Secret key
/// nothing consumes.
pub fn read_by_pipeline(ctx: &Ctx) -> Result<()> {
    let data = planned_data(ctx)?;
    let config = ctx.rendered.gateway_config()?;
    let unread: Vec<&str> = data
        .keys()
        .filter(|key| !config.contains(&format!("sys.env(\"{key}\")")))
        .map(String::as_str)
        .collect();
    if !unread.is_empty() {
        bail!(
            "the gateway Secret sets variables the pipeline never reads: {}\n\
             The values and the Secret have to agree on the name. They do not, so the destination \
             authenticates with an empty credential at run time.",
            unread.join(", ")
        );
    }
    Ok(())
}

/// No declared credential reached the rendered release.
///
/// The reason these are not passed as values at all: `values` is recoverable
/// with `helm get values` by anyone who can read the release Secret. A
/// credential that reached the rendered manifests got there through the values,
/// which is the regression.
pub fn absent_from_release(ctx: &Ctx) -> Result<()> {
    let leaked: Vec<String> = declared(ctx)
        .into_iter()
        .filter(|(_, value)| ctx.rendered.text().contains(value))
        .map(|(input, _)| input)
        .collect();
    if !leaked.is_empty() {
        bail!(
            "destination credentials reached the Helm release: {}\n\
             They belong in the module's Secret only — anything in values is readable with \
             'helm get values' and is stored in the release Secret besides.",
            leaked.join(", ")
        );
    }
    Ok(())
}

/// The planned Secret's data. The Secret is a Terraform resource, so the plan
/// is the only place it appears.
fn planned_data(ctx: &Ctx) -> Result<&serde_json::Map<String, Value>> {
    let Some(data) = ctx
        .planned("kubernetes_secret", "alloy_gateway_env")
        .and_then(|secret| secret.get("data"))
        .and_then(Value::as_object)
    else {
        bail!(
            "the example declares destination credentials but the module plans no \
             alloy_gateway_env Secret, so the gateway starts with none of them"
        );
    };
    Ok(data)
}
