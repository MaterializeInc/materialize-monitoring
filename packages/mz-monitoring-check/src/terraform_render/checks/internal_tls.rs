// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `internal_tls`: the stage composed the chart's mTLS profiles onto every hop.
//!
//! Every part of this is invisible to `terraform validate`: the profiles are
//! files the module reads at plan time, and a stage that failed to compose
//! renders a chart that installs perfectly and speaks plaintext.
//!
//! The stages are `encrypt`, `present` and `authenticate`, and only
//! `authenticate` refuses a client that presents nothing. Asserting the negative
//! for the earlier stages is the point of having stages at all: `present` looks
//! like mTLS in every values file and rejects nothing, so a bug that skipped
//! ahead would otherwise read as the feature working.

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::{alerting, report, shown};
use crate::terraform_render::ctx::{Ctx, parse_yaml, str_at, truthy};
use crate::terraform_render::manifests::Manifest;

/// Where the chart mounts each component's certificate.
const CERT_FILE: &str = "/etc/mzmon/tls/tls.crt";

const RECEIVE_SERVER_CERT: &str = "--remote-write.server-tls-cert=";
const RECEIVE_CLIENT_CA: &str = "--remote-write.server-tls-client-ca=";
const REPLICATION_FACTOR: &str = "--receive.replication-factor=3";
const GATEWAY_REQUIRES: &str = r#"client_auth_type = "RequireAndVerifyClientCert""#;
const ALERTMANAGER_WEB_CONFIG: &str = "--web.config.file=/etc/alertmanager/config/web.yml";

/// The declared stage, `off` when the example declared none.
pub fn stage(ctx: &Ctx) -> &str {
    ctx.declared_str("internal_tls").unwrap_or("off")
}

pub fn enabled(ctx: &Ctx) -> bool {
    stage(ctx) != "off"
}

/// Loki serves TLS on its HTTP port: the floor for every stage.
pub fn loki_serves_tls(ctx: &Ctx) -> Result<()> {
    let config = ctx.rendered.loki_config()?;
    let got = str_at(&config, "server.http_tls_config.cert_file");
    if got != Some(CERT_FILE) {
        bail!(
            "internal_tls={} but Loki's server.http_tls_config.cert_file is {}",
            stage(ctx),
            shown(got)
        );
    }
    Ok(())
}

/// Thanos Receive serves TLS on its remote-write listener: the other floor.
pub fn thanos_receive_serves_tls(ctx: &Ctx) -> Result<()> {
    if !receive_args(ctx).any(|arg| arg.starts_with(RECEIVE_SERVER_CERT)) {
        bail!(
            "internal_tls={} but Thanos Receive has no {RECEIVE_SERVER_CERT}",
            stage(ctx)
        );
    }
    Ok(())
}

/// Composing the mTLS profile did not cost Receive its replication factor.
///
/// The sizing profile also sets `thanos.receive.extraArgs`, and Helm overwrites
/// lists. Whichever document loses that merge does so silently, and losing this
/// flag drops write quorum to 1.
pub fn replication_factor_survives(ctx: &Ctx) -> Result<()> {
    if !receive_args(ctx).any(|arg| arg == REPLICATION_FACTOR) {
        bail!(
            "internal_tls={} clobbered {REPLICATION_FACTOR} — the sizing profile and the mTLS \
             profile are fighting over thanos.receive.extraArgs",
            stage(ctx)
        );
    }
    Ok(())
}

/// The gateway and Thanos Receive require a client certificate at
/// `authenticate`, and only there.
pub fn client_auth_matches_stage(ctx: &Ctx) -> Result<()> {
    let stage = stage(ctx);
    let gateway = ctx.rendered.gateway_config()?.contains(GATEWAY_REQUIRES);
    let receive = receive_args(ctx).any(|arg| arg.starts_with(RECEIVE_CLIENT_CA));

    let mut problems = Vec::new();
    if stage == "authenticate" {
        if !gateway {
            problems.push(
                "internal_tls=authenticate but no gateway listener requires a client certificate"
                    .to_owned(),
            );
        }
        if !receive {
            problems.push(
                "internal_tls=authenticate but Thanos Receive has no client CA, so it \
                 authenticates nobody"
                    .to_owned(),
            );
        }
    } else if gateway || receive {
        problems.push(format!(
            "internal_tls={stage} should still serve a client presenting no certificate, but {} \
             requires one",
            match (gateway, receive) {
                (true, true) => "the gateway and Thanos Receive each",
                (true, false) => "a gateway listener",
                _ => "Thanos Receive",
            }
        ));
    }
    report(problems, "")
}

/// Loki's HTTP port verifies a client certificate when one is presented.
///
/// Loki's own hop tops out at verify-if-given, whatever the stage: the kubelet
/// probes that port and an `httpGet` probe cannot present a certificate.
/// `present` is where it arrives.
pub fn loki_verifies_client_certs(ctx: &Ctx) -> Result<()> {
    let config = ctx.rendered.loki_config()?;
    let got = str_at(&config, "server.http_tls_config.client_auth_type");
    if got != Some("VerifyClientCertIfGiven") {
        bail!(
            "internal_tls={} but Loki's server.http_tls_config.client_auth_type is {}, \
             expected \"VerifyClientCertIfGiven\"",
            stage(ctx),
            shown(got)
        );
    }
    Ok(())
}

/// Alertmanager serves TLS on its API port, at verify-if-given from `present`.
///
/// The listener settings are the chart's own `web.yml`, and they only take
/// effect through the flag, so both are asserted. Like Loki's, the port tops out
/// at verify-if-given.
pub fn alertmanager_serves_tls(ctx: &Ctx) -> Result<()> {
    let secret = alerting::config_secret(ctx).context("no alertmanager-config Secret rendered")?;
    let web = match secret.entry("web.yml") {
        Some(text) => parse_yaml(&text).context("parsing Alertmanager's web.yml")?,
        None => Value::Null,
    };
    let want = if stage(ctx) == "encrypt" {
        "NoClientCert"
    } else {
        "VerifyClientCertIfGiven"
    };
    let got = str_at(&web, "tls_server_config.client_auth_type");
    if got != Some(want) {
        bail!(
            "Alertmanager's web.yml client_auth_type is {}, want {want:?}",
            shown(got)
        );
    }

    let passes_flag = ctx
        .rendered
        .find("StatefulSet", "alertmanager")
        .into_iter()
        .flat_map(Manifest::containers)
        .filter(|container| container.get("name").and_then(Value::as_str) == Some("alertmanager"))
        .filter_map(|container| container.get("args")?.as_array())
        .flatten()
        .any(|arg| arg.as_str() == Some(ALERTMANAGER_WEB_CONFIG));
    if !passes_flag {
        bail!(
            "web.yml rendered but the alertmanager StatefulSet does not pass {ALERTMANAGER_WEB_CONFIG}, \
             so Alertmanager serves plaintext"
        );
    }
    Ok(())
}

/// Both rulers remote-write to the gateway with a CA and a keypair.
///
/// They write to the gateway's metrics listener, which every stage past `off`
/// serves over TLS and `authenticate` requires a certificate on. A ruler without
/// a keypair there retries every batch forever.
pub fn rulers_remote_write_over_tls(ctx: &Ctx) -> Result<()> {
    let mut problems = Vec::new();
    if let Some(clients) = thanos_ruler_clients(ctx)? {
        check_ruler("Thanos ruler", &clients, &mut problems);
    }
    let loki = ctx.rendered.loki_config()?;
    if let Some(clients) = loki
        .pointer("/ruler/remote_write/clients")
        .and_then(Value::as_object)
    {
        let clients: Vec<Value> = clients.values().cloned().collect();
        check_ruler("Loki ruler", &clients, &mut problems);
    }
    report(problems, "")
}

fn check_ruler(who: &str, clients: &[Value], problems: &mut Vec<String>) {
    for client in clients {
        let url = str_at(client, "url").unwrap_or("");
        if !url.contains("alloy-gateway") {
            continue;
        }
        let tls = client.get("tls_config");
        let keyed = ["ca_file", "cert_file", "key_file"]
            .iter()
            .all(|key| tls.and_then(|tls| tls.get(key)).is_some_and(truthy));
        if !url.starts_with("https://") || !keyed {
            problems.push(format!(
                "{who} remote-writes to {url} without a CA and keypair (tls_config: {})",
                tls.unwrap_or(&Value::Null)
            ));
        }
        // The gateway's OTLP bridge refuses samples without job and instance.
        // The Loki ruler fills them itself; the gateway fills the Thanos ruler's,
        // because a replace in Thanos's write_relabel_configs that reads a label
        // panics it.
        let relabels = client
            .get("write_relabel_configs")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let filled: Vec<&str> = relabels
            .iter()
            .filter_map(|rule| str_at(rule, "target_label"))
            .collect();
        if who == "Loki ruler" && !(filled.contains(&"job") && filled.contains(&"instance")) {
            problems.push(format!(
                "{who} remote-writes without filling job and instance, which the gateway refuses \
                 (fills: {filled:?})"
            ));
        }
        if who == "Thanos ruler" && !relabels.is_empty() {
            problems.push(format!(
                "{who} has write_relabel_configs, and Thanos panics on a replace in them that \
                 reads a label"
            ));
        }
    }
}

/// The remote-write clients in the Secret the Thanos ruler actually mounts.
///
/// The chart renders a plaintext and a TLS variant; what matters is the one the
/// StatefulSet's `remote-write` volume names.
fn thanos_ruler_clients(ctx: &Ctx) -> Result<Option<Vec<Value>>> {
    let mounted = ctx
        .rendered
        .find("StatefulSet", "thanos-ruler")
        .and_then(Manifest::pod_spec)
        .and_then(|spec| spec.get("volumes")?.as_array())
        .into_iter()
        .flatten()
        .find(|volume| volume.get("name").and_then(Value::as_str) == Some("remote-write"))
        .and_then(|volume| str_at(volume, "secret.secretName"));
    let Some(mounted) = mounted else {
        return Ok(None);
    };
    let Some(secret) = ctx.rendered.find("Secret", mounted) else {
        return Ok(None);
    };
    let mut clients = Vec::new();
    for text in secret.entries().values() {
        let document = parse_yaml(text).with_context(|| format!("parsing Secret/{mounted}"))?;
        if let Some(remote_write) = document.get("remote_write").and_then(Value::as_array) {
            clients.extend(remote_write.iter().cloned());
        }
    }
    Ok(Some(clients))
}

fn receive_args(ctx: &Ctx) -> impl Iterator<Item = &str> {
    ctx.rendered
        .workloads()
        .filter(|workload| workload.name().starts_with("thanos-receive"))
        .flat_map(Manifest::container_args)
}
