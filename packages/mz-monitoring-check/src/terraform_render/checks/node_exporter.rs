// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `node_exporter`: `install_node_exporter` reaches the chart's circuit breaker.
//!
//! The toggle writes the top-level `node-exporter.enabled` rather than a tag,
//! because tags are OR'd and the chart carries node-exporter under `default`.
//! That makes the failure mode specific: `tags.node-exporter = false` is valid
//! YAML, reaches a key the chart really reads, and does nothing at all. The
//! DaemonSet either renders or it does not, so the assertion is on that.

use anyhow::{Result, bail};
use serde_json::Value;

use crate::terraform_render::ctx::Ctx;

pub fn circuit_breaker(ctx: &Ctx) -> Result<()> {
    // The last document to set it wins, as in Helm, so a caller overriding it
    // through `additional_values` is respected.
    let Some(want) = ctx
        .composed("node-exporter.enabled")
        .and_then(Value::as_bool)
    else {
        bail!(
            "the module wrote no top-level node-exporter.enabled\n\
             install_node_exporter has to write the circuit breaker. A tag cannot switch it off, \
             so moving it under tags: makes the input silently inert."
        );
    };
    let rendered = ctx.rendered.find("DaemonSet", "node-exporter").is_some();
    match (want, rendered) {
        (true, false) => {
            bail!("install_node_exporter is true but no node-exporter DaemonSet rendered")
        }
        (false, true) => bail!(
            "install_node_exporter is false but the node-exporter DaemonSet still rendered\n\
             A tag cannot switch it off; the circuit breaker is what does."
        ),
        _ => Ok(()),
    }
}
