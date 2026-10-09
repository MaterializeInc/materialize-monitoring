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

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::terraform_render::ctx::Ctx;

/// The planned `alloy_gateway_env` Secret's data, empty when the module plans
/// none.
pub fn planned(ctx: &Ctx) -> &Map<String, Value> {
    static EMPTY: std::sync::LazyLock<Map<String, Value>> = std::sync::LazyLock::new(Map::new);
    ctx.planned("kubernetes_secret", "alloy_gateway_env")
        .and_then(|secret| secret.get("data"))
        .and_then(Value::as_object)
        .unwrap_or(&EMPTY)
}

/// Every key of the Secret is read by the rendered pipeline.
///
/// That catches a derived name changing on either side, and a Secret key
/// nothing consumes.
pub fn read_by_pipeline(ctx: &Ctx) -> Result<()> {
    let config = ctx.rendered.gateway_config()?;
    let unread: Vec<&str> = planned(ctx)
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

/// No credential reached the rendered release.
///
/// The reason these are not passed as values at all: `values` is recoverable
/// with `helm get values` by anyone who can read the release Secret. A
/// credential that reached the rendered manifests got there through the values,
/// which is the regression.
pub fn absent_from_release(ctx: &Ctx) -> Result<()> {
    let leaked: Vec<&str> = planned(ctx)
        .iter()
        .filter(|(_, value)| {
            value
                .as_str()
                .is_some_and(|value| !value.is_empty() && ctx.rendered.text().contains(value))
        })
        .map(|(key, _)| key.as_str())
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
