// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `certificates`: each Certificate points at the issuer the example chose.
//!
//! The class of bug tier 0 exists for: an issuer written to a path the chart does
//! not read is valid HCL, plans clean, and issues nothing. Only a render shows
//! whether the Certificates came out pointing at the right issuer.
//!
//! Two shapes, one per example. With no issuer named the chart bootstraps a
//! self-signed root scoped to the release, and every certificate has to chain to
//! an issuer the release itself renders. With `issuer_ref` (and optionally
//! `internal_issuer_ref`) named, the component certificates go to the internal
//! one and the browser-facing certificate to `issuer_ref`.

use anyhow::{Result, bail};
use serde_json::Value;

use super::report;
use crate::terraform_render::ctx::{Ctx, str_at};
use crate::terraform_render::manifests::Manifest;

/// cert-manager's own group, which the module defaults an unstated one to.
const DEFAULT_GROUP: &str = "cert-manager.io";

/// Every Certificate names an issuer, and it is the one the example chose.
pub fn issuer(ctx: &Ctx) -> Result<()> {
    let certificates = certificates(ctx)?;

    let mut problems = Vec::new();
    for certificate in &certificates {
        let Some(got) = issuer_ref(certificate) else {
            problems.push(format!("{} has no issuerRef", certificate.name()));
            continue;
        };
        match wanted_issuer(ctx, certificate) {
            Some(want) => {
                let want = (
                    str_at(want, "kind").unwrap_or("?"),
                    str_at(want, "name").unwrap_or("?"),
                );
                if (got.kind, got.name) != want {
                    problems.push(format!(
                        "{} is issued by {}/{}, expected {}/{}",
                        certificate.name(),
                        got.kind,
                        got.name,
                        want.0,
                        want.1
                    ));
                }
            }
            // The self-signed root: whatever it points at has to be an issuer
            // this release renders, or nothing signs it.
            None if self_signed(ctx) && ctx.rendered.find(got.kind, got.name).is_none() => {
                problems.push(format!(
                    "{} is issued by {}/{}, which this release does not render, though no \
                     issuer_ref asked for an outside one",
                    certificate.name(),
                    got.kind,
                    got.name
                ));
            }
            None => {}
        }
    }
    if self_signed(ctx) && ctx.rendered.of_kind("ClusterIssuer").next().is_none() {
        problems.push("asked for a self-signed root and no ClusterIssuer rendered".to_owned());
    }
    report(problems, "")
}

/// Every Certificate's issuerRef carries the group the example's issuer lives in.
///
/// cert-manager resolves `issuerRef` by group as well as by kind, so a missing
/// or wrong group silently points at nothing — and an external issuer such as
/// AWS Private CA or Google CAS lives in a group of its own.
pub fn issuer_group(ctx: &Ctx) -> Result<()> {
    let mut problems = Vec::new();
    for certificate in certificates(ctx)? {
        let want = wanted_issuer(ctx, certificate)
            .and_then(|issuer| str_at(issuer, "group"))
            .unwrap_or(DEFAULT_GROUP);
        match issuer_ref(certificate).and_then(|got| got.group) {
            None => problems.push(format!(
                "{} has an issuerRef carrying no group",
                certificate.name()
            )),
            Some(got) if got != want => problems.push(format!(
                "{} has issuerRef group {got}, expected {want}",
                certificate.name()
            )),
            Some(_) => {}
        }
    }
    report(problems, "")
}

/// Loki's certificate carries the full SAN ladder.
///
/// A wrong or short list installs clean and fails at the first handshake with a
/// name mismatch that reads like a broken certificate. The distributor is the
/// name every agent dials, so both ends of its ladder are required.
pub fn loki_san_ladder(ctx: &Ctx) -> Result<()> {
    let Some(certificate) = certificates(ctx)?
        .into_iter()
        .find(|certificate| dns_names(certificate).any(|name| name == "loki-distributor"))
    else {
        bail!("no Certificate names the bare loki-distributor Service");
    };
    let namespace = certificate.namespace().unwrap_or("default");
    let fqdn = format!("loki-distributor.{namespace}.svc.cluster.local");
    if !dns_names(certificate).any(|name| name == fqdn) {
        bail!(
            "{} is missing the fully-qualified SAN rung {fqdn}",
            certificate.name()
        );
    }
    Ok(())
}

/// The browser-facing certificate renders for every external name.
///
/// It is separate because a public issuer cannot sign in-cluster names, and it
/// only exists for Grafana behind an L4 balancer, so it is the one most easily
/// lost.
pub fn external(ctx: &Ctx) -> Result<()> {
    let Some(certificate) = certificates(ctx)?
        .into_iter()
        .find(|certificate| is_external(certificate))
    else {
        bail!("an external issuer and DNS names were set but no external certificate rendered");
    };
    let missing: Vec<&str> = ctx
        .declared("grafana_external_dns_names")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|want| !dns_names(certificate).any(|name| name == *want))
        .collect();
    if !missing.is_empty() {
        bail!("{} does not name {missing:?}", certificate.name());
    }
    Ok(())
}

fn certificates(ctx: &Ctx) -> Result<Vec<&Manifest>> {
    let certificates: Vec<_> = ctx.rendered.of_kind("Certificate").collect();
    if certificates.is_empty() {
        bail!("certificates_enabled is true but no Certificate rendered");
    }
    Ok(certificates)
}

struct IssuerRef<'a> {
    name: &'a str,
    kind: &'a str,
    group: Option<&'a str>,
}

fn issuer_ref(certificate: &Manifest) -> Option<IssuerRef<'_>> {
    Some(IssuerRef {
        name: certificate.str_at("spec.issuerRef.name")?,
        // cert-manager's own default when the kind is left out.
        kind: certificate
            .str_at("spec.issuerRef.kind")
            .unwrap_or("Issuer"),
        group: certificate.str_at("spec.issuerRef.group"),
    })
}

/// The declared issuer a Certificate should name, or `None` when the example
/// named none that applies to it.
fn wanted_issuer<'a>(ctx: &'a Ctx, certificate: &Manifest) -> Option<&'a Value> {
    if is_external(certificate) {
        ctx.declared("issuer_ref")
    } else {
        ctx.declared("internal_issuer_ref")
            .or_else(|| ctx.declared("issuer_ref"))
    }
}

fn self_signed(ctx: &Ctx) -> bool {
    ctx.declared("issuer_ref").is_none() && ctx.declared("internal_issuer_ref").is_none()
}

fn is_external(certificate: &Manifest) -> bool {
    certificate.name().ends_with("grafana-external-tls")
}

fn dns_names(certificate: &Manifest) -> impl Iterator<Item = &str> {
    certificate
        .get("spec.dnsNames")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}
