// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `secret_filter`: the mode reached the gateway the way the chart renders it.
//!
//! Neither half is visible to `terraform validate`. `shadow` is a forward the
//! chart edits into the gateway config as it renders, and `on` is two
//! environment variables the pipeline reads. A mode that lands nowhere leaves a
//! gateway that scans nothing while the input says it does, or one that scans
//! every line while the input says it's off.
//!
//! Both checks apply to every example: one that declares nothing is the `off`
//! case, which is the module's default.

use anyhow::{Result, bail};

use super::shown;
use crate::terraform_render::ctx::Ctx;

/// What the gateway config forwards to when shadow mode is wired.
const SHADOW_RECEIVER: &str = "loki.secretfilter.shadow.receiver";

/// The two instances' scan rates, which the pipeline reads from the env.
const RATES: [&str; 2] = [
    "GATEWAY_SECRETFILTER_RATE",
    "GATEWAY_SECRETFILTER_BULK_RATE",
];

/// The declared mode, `off` when the example declared none.
pub fn mode(ctx: &Ctx) -> &str {
    ctx.declared_str("secret_filter").unwrap_or("off")
}

/// The shadow branch is wired exactly when the mode is `shadow`.
pub fn shadow_branch_matches_mode(ctx: &Ctx) -> Result<()> {
    let wired = ctx.rendered.gateway_config()?.contains(SHADOW_RECEIVER);
    let want = mode(ctx) == "shadow";
    if wired != want {
        bail!(
            "secret_filter={} but the gateway config {} {SHADOW_RECEIVER}",
            mode(ctx),
            if wired { "forwards to" } else { "has no" }
        );
    }
    Ok(())
}

/// Both scan rates are `"1"` exactly when the mode is `on`, and unset otherwise.
pub fn rates_match_mode(ctx: &Ctx) -> Result<()> {
    let env = ctx.rendered.gateway_env()?;
    let on = mode(ctx) == "on";
    let wrong: Vec<String> = RATES
        .iter()
        .filter_map(|key| {
            let got = env.get(*key).map(String::as_str);
            let fine = if on { got == Some("1") } else { got.is_none() };
            (!fine).then(|| format!("{key} is {}", shown(got)))
        })
        .collect();
    if !wrong.is_empty() {
        bail!(
            "secret_filter={} but in the gateway env {}",
            mode(ctx),
            wrong.join(", ")
        );
    }
    Ok(())
}
