// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `grafana`: Grafana's state database lands as one piece.
//!
//! Three things have to land together and each is written to a different
//! subchart path, so any one of them missing is a Grafana that comes up on
//! SQLite — or crash-loops on a half-written config — while the plan looks
//! entirely correct. The password never reaches `grafana.ini` as a literal by
//! design, so the mount and the Secret are what make the `[database]` block
//! usable.

use anyhow::{Context, Result};
use serde_json::Value;

use super::report;
use crate::terraform_render::ctx::{Ctx, str_at};

const MOUNT: &str = "/etc/secrets/grafana-db";

pub fn database(ctx: &Ctx) -> Result<()> {
    let grafana = ctx
        .rendered
        .find("Deployment", "grafana")
        .context("no grafana Deployment rendered")?;
    let ini = ctx
        .rendered
        .config_entry("grafana", "grafana.ini")
        .unwrap_or_default();

    let mut missing = Vec::new();
    if !ini.lines().any(|line| line.trim() == "type = postgres") {
        missing.push("grafana.ini [database] type = postgres");
    }
    // `$__file{...}` is Grafana's own expansion syntax, read by Grafana at
    // startup.
    let password = format!("password = $__file{{{MOUNT}/password}}");
    if !ini.lines().any(|line| line.trim() == password) {
        missing.push("grafana.ini's password file reference");
    }

    let mounted = grafana
        .containers()
        .filter_map(|container| container.get("volumeMounts")?.as_array())
        .flatten()
        .find(|mount| str_at(mount, "mountPath") == Some(MOUNT));
    let volume = mounted
        .and_then(|mount| str_at(mount, "name"))
        .and_then(|name| {
            grafana
                .pod_spec()?
                .get("volumes")?
                .as_array()?
                .iter()
                .find(|volume| str_at(volume, "name") == Some(name))
        });
    match (
        mounted,
        volume.and_then(|volume| str_at(volume, "secret.secretName")),
    ) {
        (None, _) => missing.push("the Secret mount"),
        (Some(_), None) => missing.push("a Secret behind the mount"),
        (Some(_), Some(secret)) => {
            // The Secret itself is a Terraform resource, so it is never in `helm
            // template` output. What the chart can prove is that the mount names
            // the one Terraform creates — a mismatch there is a pod stuck on a
            // volume that does not exist.
            let planned = ctx
                .planned("kubernetes_secret", "grafana_database")
                .and_then(|planned| planned.pointer("/metadata/0/name"))
                .and_then(Value::as_str);
            if planned.is_some_and(|planned| planned != secret) {
                missing.push("a mount naming the Secret Terraform creates");
            }
        }
    }

    report(
        missing
            .into_iter()
            .map(|what| format!("a Grafana database is configured but missing {what}"))
            .collect(),
        "",
    )
}
