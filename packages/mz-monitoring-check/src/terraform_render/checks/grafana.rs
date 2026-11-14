// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `grafana`: settings that only work as a set land as one.
//!
//! Grafana's state database: the `[database]` block, the password file it
//! reads, and the Secret mounted there are each written to a different subchart
//! path, so any one of them missing is a Grafana that comes up on SQLite — or
//! crash-loops on a half-written config — while the plan looks entirely
//! correct. The password never reaches `grafana.ini` as a literal by design, so
//! the mount and the Secret are what make the `[database]` block usable.
//!
//! Grafana serving HTTPS: the `[server]` block, the certificate mount, the
//! probes, the Service port, and the scrape each sit on their own path, and any
//! one left behind is a Grafana that never starts, never turns Ready, or goes
//! unscraped.
//!
//! The database gates resolve the way the module's `main.tf` does: an explicit
//! flag wins, and null infers from the value it guards.

use anyhow::{Context, Result};
use serde_json::Value;

use super::report;
use crate::terraform_render::ctx::{Ctx, str_at};
use crate::terraform_render::manifests::Manifest;

const MOUNT: &str = "/etc/secrets/grafana-db";
const EXTERNAL_MOUNT: &str = "/etc/mzmon/grafana-external-tls";

/// Whether the example gives Grafana a browser-facing certificate, which is
/// what the module turns HTTPS on for.
pub fn https_enabled(ctx: &Ctx) -> bool {
    ctx.declares("certificates_enabled")
        && ctx.declares("issuer_ref")
        && ctx.declares("grafana_external_dns_names")
}

/// Grafana serves the browser-facing certificate the module issues for it.
pub fn serves_https(ctx: &Ctx) -> Result<()> {
    let grafana = ctx
        .rendered
        .find("Deployment", "grafana")
        .context("no grafana Deployment rendered")?;
    let ini = ctx
        .rendered
        .config_entry("grafana", "grafana.ini")
        .unwrap_or_default();
    let secret = ctx
        .rendered
        .of_kind("Certificate")
        .find(|certificate| certificate.name().ends_with("grafana-external-tls"))
        .and_then(|certificate| certificate.str_at("spec.secretName"))
        .context("no external Grafana certificate rendered")?;
    let host = ctx
        .declared("grafana_external_dns_names")
        .and_then(Value::as_array)
        .and_then(|names| names.first())
        .and_then(Value::as_str)
        .unwrap_or_default();

    let mut missing = Vec::new();
    for line in [
        "protocol = https".to_owned(),
        format!("cert_file = {EXTERNAL_MOUNT}/tls.crt"),
        format!("cert_key = {EXTERNAL_MOUNT}/tls.key"),
        format!("root_url = https://{host}"),
        "cookie_secure = true".to_owned(),
    ] {
        if !ini.lines().any(|candidate| candidate.trim() == line) {
            missing.push(format!("grafana.ini's `{line}`"));
        }
    }

    match mounted_secret(grafana, EXTERNAL_MOUNT) {
        None => missing.push(format!("a Secret mounted at {EXTERNAL_MOUNT}")),
        Some(mounted) if mounted != secret => missing.push(format!(
            "a mount of the certificate's Secret ({mounted} is mounted, the certificate \
             lands in {secret})"
        )),
        Some(_) => {}
    }

    let container = grafana
        .containers()
        .find(|container| str_at(container, "name") == Some("grafana"));
    for probe in ["livenessProbe", "readinessProbe"] {
        let scheme =
            container.and_then(|container| str_at(container, &format!("{probe}.httpGet.scheme")));
        if scheme != Some("HTTPS") {
            missing.push(format!("an HTTPS {probe} (its scheme is {scheme:?})"));
        }
    }

    let port = ctx
        .rendered
        .find("Service", "grafana")
        .and_then(|service| service.get("spec.ports"))
        .and_then(Value::as_array)
        .and_then(|ports| ports.first())
        .and_then(|port| port.get("port"))
        .and_then(Value::as_u64);
    if port != Some(443) {
        missing.push(format!(
            "the grafana Service on port 443 (it is on {port:?})"
        ));
    }

    let endpoint = ctx
        .rendered
        .find("ServiceMonitor", "grafana")
        .and_then(|monitor| monitor.get("spec.endpoints"))
        .and_then(Value::as_array)
        .and_then(|endpoints| endpoints.first());
    if endpoint.and_then(|endpoint| str_at(endpoint, "scheme")) != Some("https")
        || endpoint.and_then(|endpoint| str_at(endpoint, "tlsConfig.serverName")) != Some(host)
    {
        missing.push(format!("an HTTPS scrape of Grafana that verifies {host}"));
    }

    report(
        missing
            .into_iter()
            .map(|what| format!("Grafana has an external certificate but is missing {what}"))
            .collect(),
        "",
    )
}

/// The Secret behind the volume a workload mounts at `path`.
fn mounted_secret<'a>(workload: &'a Manifest, path: &str) -> Option<&'a str> {
    let name = workload
        .containers()
        .filter_map(|container| container.get("volumeMounts")?.as_array())
        .flatten()
        .find(|mount| str_at(mount, "mountPath") == Some(path))
        .and_then(|mount| str_at(mount, "name"))?;
    workload
        .pod_spec()?
        .get("volumes")?
        .as_array()?
        .iter()
        .find(|volume| str_at(volume, "name") == Some(name))
        .and_then(|volume| str_at(volume, "secret.secretName"))
}

/// Whether the example points Grafana at PostgreSQL at all.
pub fn database_enabled(ctx: &Ctx) -> bool {
    match ctx
        .declared("grafana_database_enabled")
        .and_then(Value::as_bool)
    {
        Some(enabled) => enabled,
        None => ctx.declares("grafana_database_host"),
    }
}

/// Whether the module is meant to own the password Secret.
///
/// False is the passwordless shape — a Cloud SQL Auth Proxy sidecar, say — which
/// has no Secret, no `$__file{}` reference and no mount to check.
pub(crate) fn password_secret_expected(ctx: &Ctx) -> bool {
    match ctx
        .declared("grafana_database_manage_password_secret")
        .and_then(Value::as_bool)
    {
        Some(managed) => managed,
        None => ctx.declares("grafana_database_password"),
    }
}

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
        missing.push("grafana.ini [database] type = postgres".to_owned());
    }
    if password_secret_expected(ctx) {
        missing.extend(password_secret_problems(ctx, grafana, &ini));
    }

    report(
        missing
            .into_iter()
            .map(|what| format!("a Grafana database is configured but missing {what}"))
            .collect(),
        "",
    )
}

/// What is missing between `grafana.ini`'s password reference and the Secret
/// Terraform creates for it.
fn password_secret_problems(ctx: &Ctx, grafana: &Manifest, ini: &str) -> Vec<String> {
    let mut missing = Vec::new();
    // `$__file{...}` is Grafana's own expansion syntax, read by Grafana at
    // startup.
    let password = format!("password = $__file{{{MOUNT}/password}}");
    if !ini.lines().any(|line| line.trim() == password) {
        missing.push("grafana.ini's password file reference".to_owned());
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
    let mounted_secret = volume.and_then(|volume| str_at(volume, "secret.secretName"));
    // The Secret itself is a Terraform resource, so it is never in `helm
    // template` output. What the chart can prove is that the mount names the one
    // Terraform creates — a mismatch, or no Secret planned at all, is a pod stuck
    // on a volume that does not exist.
    let planned = ctx
        .planned("kubernetes_secret", "grafana_database")
        .and_then(|planned| planned.pointer("/metadata/0/name"))
        .and_then(Value::as_str);
    match (mounted, mounted_secret, planned) {
        (None, _, _) => missing.push("the Secret mount".to_owned()),
        (Some(_), None, _) => missing.push("a Secret behind the mount".to_owned()),
        (Some(_), Some(_), None) => missing.push(
            "the grafana_database Secret: the module plans none for the mount to find".to_owned(),
        ),
        (Some(_), Some(mounted), Some(planned)) if mounted != planned => missing.push(format!(
            "a mount naming the Secret Terraform creates ({mounted} is mounted, {planned} is \
             planned)"
        )),
        _ => {}
    }
    missing
}
