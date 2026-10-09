// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `storage_class`: the class reaches every PVC-backed workload.
//!
//! A `storageClass` written to a path no subchart reads renders fine and is
//! silently ignored, so rendering alone proves nothing. Each subchart takes the
//! class at a different path (see `storage_class.tf`), so the assertion is on
//! the rendered claims: every `volumeClaimTemplate` carries the class the
//! example asked for.

use anyhow::{Context, Result};
use serde_json::Value;

use super::{report, shown};
use crate::terraform_render::ctx::{Ctx, str_at};

pub fn reaches_every_claim(ctx: &Ctx) -> Result<()> {
    let want = ctx
        .declared_str("storage_class")
        .context("storage_class is not a string")?;

    let mut problems = Vec::new();
    for statefulset in ctx.rendered.of_kind("StatefulSet") {
        let claims = statefulset
            .get("spec.volumeClaimTemplates")
            .and_then(Value::as_array)
            .into_iter()
            .flatten();
        for claim in claims {
            let got = str_at(claim, "spec.storageClassName");
            if got != Some(want) {
                problems.push(format!(
                    "{} claim {} has storageClassName {}, not {want:?}",
                    statefulset.id(),
                    str_at(claim, "metadata.name").unwrap_or("?"),
                    shown(got),
                ));
            }
        }
    }
    report(
        problems,
        "A PVC-backed workload is missing from storage_class.tf's fan-out.",
    )
}
