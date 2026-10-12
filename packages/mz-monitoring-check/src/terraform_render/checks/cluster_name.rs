// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `cluster_name`: the name reaches everything that stamps it.
//!
//! That is both Alloy env ConfigMaps and the rulers' one. The agent stamps it on
//! pod logs, the gateway uses it as the `cluster` fallback for every other log
//! source and as the metrics `external_labels`, and both rulers stamp it on
//! every alert. A value that lands in only some of them splits a cluster's
//! signals across two label values with nothing said.

use anyhow::{Context, Result, bail};

use crate::terraform_render::ctx::Ctx;

pub fn reaches_every_stamp(ctx: &Ctx) -> Result<()> {
    let want = ctx
        .declared_str("cluster_name")
        .context("cluster_name is not a string")?;

    let (mut alloy, mut rulers) = (Vec::new(), Vec::new());
    for configmap in ctx.rendered.of_kind("ConfigMap") {
        let Some(value) = configmap.entry("CLUSTER_NAME") else {
            continue;
        };
        let stamp = (configmap.name().to_owned(), value);
        if configmap.name() == "ruler-env" {
            rulers.push(stamp);
        } else {
            alloy.push(stamp);
        }
    }

    let wrong: Vec<String> = alloy
        .iter()
        .chain(&rulers)
        .filter(|(_, value)| value != want)
        .map(|(name, value)| format!("{name}={value:?}"))
        .collect();
    if alloy.len() != 2 || rulers.is_empty() || !wrong.is_empty() {
        bail!(
            "cluster_name did not reach everything that stamps it: {} of 2 Alloy env ConfigMaps \
             and {} ruler-env ConfigMap(s); wrong: {wrong:?}",
            alloy.len(),
            rulers.len(),
        );
    }
    Ok(())
}
