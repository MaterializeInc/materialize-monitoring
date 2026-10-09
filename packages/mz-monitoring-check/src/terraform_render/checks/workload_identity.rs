// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `workload_identity`: every pod bound to an Azure identity carries the
//! webhook's label.
//!
//! The Entra webhook injects the projected token and the `AZURE_*` variables
//! only into pods labelled `azure.workload.identity/use: "true"`, and each
//! subchart takes that label through a different value path (see
//! `terraform/modules/materialize-monitoring/azure.tf`). A label written to a
//! path the subchart does not read renders fine and does nothing: the annotation
//! is there, the pod is not mutated, and the Azure SDK falls through to the
//! node's managed identity. Nothing fails until the first call to Azure.

use std::collections::BTreeSet;

use anyhow::{Result, bail};
use serde_json::Value;

use super::report;
use crate::terraform_render::ctx::{Ctx, truthy};

const CLIENT_ID: &str = "azure.workload.identity/client-id";
const USE: &str = "azure.workload.identity/use";

/// `(namespace, name)` of every ServiceAccount naming an Azure identity.
///
/// An empty annotation names no identity, as the chart's own validator treats
/// it.
pub fn bound_service_accounts(ctx: &Ctx) -> BTreeSet<(Option<&str>, &str)> {
    ctx.rendered
        .of_kind("ServiceAccount")
        .filter(|sa| {
            sa.get("metadata.annotations")
                .and_then(|annotations| annotations.get(CLIENT_ID))
                .is_some_and(truthy)
        })
        .map(|sa| (sa.namespace(), sa.name()))
        .collect()
}

pub fn pods_labelled(ctx: &Ctx) -> Result<()> {
    let bound = bound_service_accounts(ctx);

    let mut checked = 0;
    let mut problems = Vec::new();
    for workload in ctx.rendered.workloads() {
        let Some(sa) = workload.str_at("spec.template.spec.serviceAccountName") else {
            continue;
        };
        if !bound.contains(&(workload.namespace(), sa)) {
            continue;
        }
        checked += 1;
        let label = workload
            .get("spec.template.metadata.labels")
            .and_then(|labels| labels.get(USE))
            .and_then(Value::as_str);
        if label != Some("true") {
            problems.push(format!(
                "{} runs as {sa}, which names an Azure identity, but its pods lack {USE}: \"true\"",
                workload.id()
            ));
        }
    }

    if checked == 0 {
        let names: Vec<_> = bound.iter().map(|(_, name)| *name).collect();
        bail!("ServiceAccounts {names:?} carry {CLIENT_ID} but no workload runs as them");
    }
    report(
        problems,
        "See terraform/modules/materialize-monitoring/azure.tf.",
    )
}
