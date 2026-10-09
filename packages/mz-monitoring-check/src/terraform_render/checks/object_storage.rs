// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `object_storage`: the buckets' endpoint and credentials reach both backends.
//!
//! Loki and Thanos take object storage through different subchart paths, and
//! Thanos's arrives as a YAML document inside a string, so a value that lands in
//! one says nothing about the other. Every failure here happens at pod start or
//! later, never at plan time.

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

use super::{report, shown, shown_json};
use crate::terraform_render::ctx::{Ctx, parse_yaml, str_at};
use crate::terraform_render::manifests::Manifest;

/// Whether the example's buckets are S3, the one dialect whose endpoint the
/// caller picks.
pub fn on_s3(ctx: &Ctx) -> bool {
    ctx.declared_at("object_storage", "cloud")
        .and_then(Value::as_str)
        == Some("aws")
}

/// Static object-storage credentials reach Loki's s3 block and the Thanos
/// objstore document.
///
/// Asserted against the composed *values*, not the rendered manifests: once the
/// configuration lands in a Secret it is encoded, so a plaintext search of the
/// render would fail for the very reason the composition is correct. A backend
/// that misses the key falls back to the default credential chain, and chunk
/// writes, receive and store all fail with a 403.
pub fn static_credentials_land(ctx: &Ctx) -> Result<()> {
    let want = ctx
        .declared_str("object_storage_access_key_id")
        .context("object_storage_access_key_id is not a string")?;

    let mut problems = Vec::new();
    let loki = ctx
        .composed("loki.loki.storage.object_store.s3.access_key_id")
        .and_then(Value::as_str);
    if loki != Some(want) {
        problems.push(format!(
            "loki: loki.storage.object_store.s3.access_key_id is {}",
            shown(loki)
        ));
    }
    let thanos = thanos_objstore(ctx)?;
    let thanos = thanos
        .as_ref()
        .and_then(|config| str_at(config, "access_key"));
    if thanos != Some(want) {
        problems.push(format!(
            "thanos: the objstore config's access_key is {}",
            shown(thanos)
        ));
    }
    report(
        problems,
        "Those backends fall back to the default credential chain and fail to authenticate at \
         pod start, not at plan time.",
    )
}

/// The secret key appears in Secrets and nowhere else.
///
/// Helm writes Secrets with `stringData`, so cleartext there is normal and it is
/// still a Secret at rest. The chart defaults Loki's `configStorageType` to
/// ConfigMap and the rendered Loki config carries `secret_access_key` verbatim,
/// so a regression puts it in a ConfigMap — which works perfectly and publishes
/// the key to anyone with `get` on ConfigMaps. That is exactly the kind of thing
/// that needs an assertion rather than trust.
pub fn secret_key_stays_in_secrets(ctx: &Ctx) -> Result<()> {
    let secret = ctx
        .declared_str("object_storage_secret_access_key")
        .context("object_storage_secret_access_key is not a string")?;
    let mut leaked: Vec<String> = ctx
        .rendered
        .iter()
        .filter(|doc| doc.kind() != "Secret" && doc.text.contains(secret))
        .map(Manifest::id)
        .collect();
    leaked.sort();
    leaked.dedup();
    if !leaked.is_empty() {
        bail!(
            "the secret access key reached non-Secret objects: {}\n\
             Check that Loki's configStorageType is Secret; the chart default is ConfigMap, which \
             works but publishes the key to the namespace.",
            leaked.join(", ")
        );
    }
    Ok(())
}

/// An `http://` endpoint reaches both backends bare, with the insecure flag set.
///
/// Read per backend rather than searched for, because the two write the same
/// key names in different places, and a flat search is satisfied by either one
/// alone. Both halves fail without naming the value that caused them: a
/// surviving scheme is "Endpoint url cannot have fully qualified paths" at
/// startup, and a missing insecure flag is a TLS handshake against a plaintext
/// port.
pub fn http_endpoint_bare_and_insecure(ctx: &Ctx) -> Result<()> {
    let given = declared_endpoint(ctx).context("object_storage.endpoint is not a string")?;
    let bare = given.split_once("://").map_or(given, |(_, rest)| rest);

    let loki = ctx.composed("loki.loki.storage.object_store.s3").cloned();
    let thanos = thanos_objstore(ctx)?;

    let mut problems = Vec::new();
    for (name, block) in [("loki", loki), ("thanos", thanos)] {
        let Some(block) = block else {
            problems.push(format!("{name}: no s3 config in the composed values"));
            continue;
        };
        let endpoint = str_at(&block, "endpoint").unwrap_or("");
        if endpoint.contains("://") {
            problems.push(format!("{name}: endpoint kept its scheme ({endpoint})"));
        } else if endpoint != bare {
            problems.push(format!(
                "{name}: endpoint is {endpoint:?}, expected {bare:?}"
            ));
        }
        if block.get("insecure") != Some(&Value::Bool(true)) {
            problems.push(format!(
                "{name}: insecure is {}, expected true",
                shown_json(block.get("insecure"))
            ));
        }
    }
    report(problems, "")
}

/// The Loki NetworkPolicy that admits egress to object storage, when one
/// renders.
pub fn loki_egress_policy(ctx: &Ctx) -> Option<&Manifest> {
    ctx.rendered
        .find("NetworkPolicy", "loki-egress-external-storage")
}

/// Loki's egress policy names the port the endpoint is on, and keeps 443 for
/// STS.
///
/// Hardcoded at 443 it blocks any self-hosted store, and the symptom names
/// nothing useful: a bare `i/o timeout` in the index gateway and every query
/// hanging to a 504. Swapped to only the store's port it would break workload
/// identity instead, since the same rule is what reaches STS. Read from the
/// rendered policy, which is what the CNI enforces.
pub fn loki_egress_covers_endpoint(ctx: &Ctx) -> Result<()> {
    let given = declared_endpoint(ctx).unwrap_or("");
    let tail = given.rsplit(':').next().unwrap_or("");
    let want = match tail.parse::<u64>() {
        Ok(port) => port,
        Err(_) if given.starts_with("http://") => 80,
        Err(_) => 443,
    };

    let policy =
        loki_egress_policy(ctx).context("no loki-egress-external-storage NetworkPolicy")?;
    let ports: Vec<u64> = policy
        .get("spec.egress")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|rule| rule.get("ports")?.as_array())
        .flatten()
        .filter_map(|port| port.get("port")?.as_u64())
        .collect();

    let mut problems = Vec::new();
    if !ports.contains(&want) {
        problems.push(format!(
            "object storage is on :{want} but Loki's egress allows {ports:?}"
        ));
    }
    if !ports.contains(&443) {
        problems.push(format!(
            "443 is missing from {ports:?}; STS is always 443, so workload identity breaks"
        ));
    }
    report(problems, "")
}

/// Loki's S3 endpoint reaches both clients built from it.
///
/// It has no default inside Loki and is not derivable from the bucket or the
/// region: the client rejects an empty one up front ("create bucket: no s3
/// endpoint in config file") instead of falling through to the AWS SDK, and every
/// component that touches storage crash-loops. The chart ships a default, so what
/// this proves is that the module's own `object_store` document did not
/// overwrite it — which is also why the endpoint the module wrote is what must
/// appear, not merely *an* endpoint: the chart's default would satisfy a bare
/// presence check even if the module contributed nothing.
pub fn loki_s3_endpoint(ctx: &Ctx) -> Result<()> {
    let Some(want) = ctx
        .composed("loki.loki.storage.object_store.s3.endpoint")
        .and_then(Value::as_str)
    else {
        bail!("Loki is on s3 but the module wrote no loki.storage.object_store.s3.endpoint");
    };

    let config = ctx.rendered.loki_config()?;
    let mut problems = Vec::new();
    for (client, path) in [
        ("chunk client", "common.storage.object_store.s3.endpoint"),
        ("ruler storage", "ruler_storage.s3.endpoint"),
    ] {
        let got = str_at(&config, path);
        if got != Some(want) {
            problems.push(format!(
                "{client} ({path}) has endpoint {}, expected {want:?}",
                shown(got)
            ));
        }
    }
    report(
        problems,
        "Loki does not default one, so those components crash-loop with \"create bucket: no s3 \
         endpoint in config file\".",
    )
}

fn declared_endpoint(ctx: &Ctx) -> Option<&str> {
    ctx.declared_at("object_storage", "endpoint")
        .and_then(Value::as_str)
}

/// The `config` block of the composed Thanos objstore document, which the
/// chart takes as a string holding YAML.
fn thanos_objstore(ctx: &Ctx) -> Result<Option<Value>> {
    let Some(text) = ctx
        .composed("thanos.global.objstore.config")
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let document = parse_yaml(text).context("parsing the composed Thanos objstore config")?;
    Ok(Some(
        document
            .get("config")
            .cloned()
            .unwrap_or(Value::Object(Map::new())),
    ))
}
