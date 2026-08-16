// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Grafana: did the operator actually push what the chart declared, and can
//! Grafana query the backends it was pointed at.
//!
//! The expected UIDs are read from the operator's own custom resources rather
//! than from a list in this file. A hardcoded list goes stale the moment a
//! dashboard is added, and it goes stale in the direction that passes — the
//! suite would keep asserting the dashboards it knows about and never notice the
//! new one failing to land.

use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use kube::api::DynamicObject;
use serde_json::Value;

use crate::cluster::{ServiceTarget, encode_segment, unix_nanos_ago, unix_nanos_now};
use crate::ctx::Ctx;
use crate::retry::retry_until;

const GROUP: &str = "grafana.integreatly.org";
const VERSION: &str = "v1beta1";

/// Grafana is up and its database is reachable.
///
/// Diagnostic. A healthy Grafana with nothing provisioned into it looks exactly
/// like this.
pub async fn health(ctx: &Ctx) -> Result<()> {
    let target = access(ctx).await?;

    retry_until(
        "grafana /api/health",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx
                .cluster
                .get_authenticated_json(&target, "api/health")
                .await?;
            match body.get("database").and_then(Value::as_str) {
                Some("ok") => Ok(()),
                other => bail!("grafana database is {:?} (full body: {body})", other),
            }
        },
    )
    .await
}

/// Every `GrafanaDatasource` the chart declared exists in Grafana under its
/// declared UID.
///
/// Proves the operator reconciled them. The UID specifically, not the name: the
/// bundled dashboards resolve their datasource by UID, so a datasource that
/// lands under a different one leaves every panel empty with no error anywhere.
pub async fn datasources_provisioned(ctx: &Ctx) -> Result<()> {
    let target = access(ctx).await?;

    let declared = ctx
        .cluster
        .list_custom(GROUP, VERSION, "GrafanaDatasource")
        .await?;
    let expected = collect(&declared, |cr| {
        cr.data
            .pointer("/spec/datasource/uid")
            .and_then(Value::as_str)
            .map(str::to_owned)
    })?;

    // A release that provisions datasources but declares none means the chart
    // rendered nothing — which would otherwise pass here vacuously, since every
    // member of an empty set is present in Grafana.
    if expected.is_empty() {
        bail!(
            "connections.datasources.enabled is true but no GrafanaDatasource \
             resources exist in {}",
            ctx.cluster.namespace()
        );
    }

    retry_until(
        "every declared datasource is provisioned",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx
                .cluster
                .get_authenticated_json(&target, "api/datasources")
                .await?;
            let live: Vec<&str> = body
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|d| d.get("uid").and_then(Value::as_str))
                        .collect()
                })
                .unwrap_or_default();

            let missing: Vec<&str> = expected
                .iter()
                .map(String::as_str)
                .filter(|uid| !live.contains(uid))
                .collect();
            if missing.is_empty() {
                Ok(())
            } else {
                bail!(
                    "datasource UIDs declared but not in Grafana: {} (Grafana has: {})",
                    missing.join(", "),
                    if live.is_empty() {
                        "nothing".to_string()
                    } else {
                        live.join(", ")
                    }
                )
            }
        },
    )
    .await
}

/// Every dashboard the chart declared is in Grafana under its declared UID, and
/// fetchable by that UID.
///
/// `/api/search` proves it was indexed; the fetch proves the UID actually
/// resolves. Both, because a dashboard can be listed and still fail to load.
pub async fn dashboards_provisioned(ctx: &Ctx) -> Result<()> {
    let target = access(ctx).await?;
    let expected = declared_dashboard_uids(ctx).await?;

    if expected.is_empty() {
        bail!(
            "no dashboard resources exist in {} — the chart declared none",
            ctx.cluster.namespace()
        );
    }

    retry_until(
        "every declared dashboard is provisioned",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx
                .cluster
                .get_authenticated_json(&target, "api/search?type=dash-db")
                .await?;
            let live: Vec<&str> = body
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|d| d.get("uid").and_then(Value::as_str))
                        .collect()
                })
                .unwrap_or_default();

            let missing: Vec<&str> = expected
                .iter()
                .map(String::as_str)
                .filter(|uid| !live.contains(uid))
                .collect();
            if !missing.is_empty() {
                bail!(
                    "dashboard UIDs declared but not in Grafana: {} (Grafana has: {})",
                    missing.join(", "),
                    if live.is_empty() {
                        "nothing".to_string()
                    } else {
                        live.join(", ")
                    }
                );
            }

            for uid in &expected {
                let path = format!("api/dashboards/uid/{}", encode_segment(uid));
                let body = ctx
                    .cluster
                    .get_authenticated_json(&target, &path)
                    .await
                    .with_context(|| format!("fetching dashboard {uid}"))?;
                // v1 nests it under `dashboard`, v2 under `spec`. Accept either
                // rather than pinning a schema version the chart may move off.
                let found = body
                    .pointer("/dashboard/uid")
                    .and_then(Value::as_str)
                    .is_some()
                    || body.get("spec").is_some();
                if !found {
                    bail!("dashboard {uid} fetched but has no dashboard body: {body}");
                }
            }

            Ok(())
        },
    )
    .await
}

/// Query Loki *through Grafana's datasource*, not directly.
///
/// This is the assertion that catches the tenant-header wiring. The bundled Loki
/// runs `auth_enabled: true`, so the datasource has to inject `X-Scope-OrgID`;
/// without it Grafana's proxy returns `no org id` and every Loki panel is empty
/// while Loki itself, queried directly, is perfectly healthy. Nothing else in
/// the suite covers that gap — [`crate::checks::loki`] sends the header itself.
pub async fn loki_datasource_query(ctx: &Ctx) -> Result<()> {
    let target = access(ctx).await?;
    let uid = ctx.features.datasource_uid("loki");
    // Time-bounded for the same reason the direct Loki check is: unbounded, the
    // label endpoint fans out across every index period and a distributed Loki
    // answers 504 — which Grafana passes through as a 502.
    let path = format!(
        "api/datasources/proxy/uid/{}/loki/api/v1/labels?start={}&end={}",
        encode_segment(&uid),
        unix_nanos_ago(Duration::from_secs(3600))?,
        unix_nanos_now()?,
    );

    retry_until(
        "loki is queryable through its Grafana datasource",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx.cluster.get_authenticated_json(&target, &path).await?;
            expect_success(&body).context("querying Loki through Grafana")?;

            // Non-empty is legitimate to demand here: these are the stack's own
            // pod logs, which exist without any Materialize instance present.
            let labels = body
                .get("data")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            if labels > 0 {
                Ok(())
            } else {
                bail!("datasource {uid} returned success but no labels")
            }
        },
    )
    .await
}

/// Query Thanos through its Grafana datasource.
///
/// `up` is asserted non-empty because it is self-monitoring: the stack scrapes
/// itself, so the series exists with no Materialize instance in the cluster.
pub async fn thanos_datasource_query(ctx: &Ctx) -> Result<()> {
    let target = access(ctx).await?;
    let uid = ctx.features.datasource_uid("thanos");
    let path = format!(
        "api/datasources/proxy/uid/{}/api/v1/query?query=up",
        encode_segment(&uid)
    );

    retry_until(
        "thanos is queryable through its Grafana datasource",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx.cluster.get_authenticated_json(&target, &path).await?;
            expect_success(&body).context("querying Thanos through Grafana")?;

            let series = body
                .pointer("/data/result")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            if series > 0 {
                Ok(())
            } else {
                bail!("datasource {uid} returned success but no `up` series")
            }
        },
    )
    .await
}

/// The bundled Grafana, with admin credentials pulled from its Secret.
async fn access(ctx: &Ctx) -> Result<ServiceTarget> {
    let (name, port) = ctx.features.grafana_service();
    let (secret, user_key, password_key) = ctx.features.grafana_admin_secret();

    let user = ctx.cluster.secret_value(secret, user_key).await?;
    let password = ctx.cluster.secret_value(secret, password_key).await?;
    let basic = BASE64.encode(format!("{user}:{password}"));

    Ok(ServiceTarget::new(name, port).with_header("Authorization", format!("Basic {basic}")))
}

/// Dashboard UIDs the chart declared, across both resources that can carry one.
///
/// `GrafanaDashboard` is the v1 path; `GrafanaManifest` wrapping a
/// `dashboard.grafana.app` `Dashboard` is the v2 one, and it is what this chart
/// renders today. Reading both means the assertion survives the chart moving
/// between them.
async fn declared_dashboard_uids(ctx: &Ctx) -> Result<Vec<String>> {
    let mut uids = collect(
        &ctx.cluster
            .list_custom(GROUP, VERSION, "GrafanaDashboard")
            .await?,
        |cr| {
            cr.data
                .pointer("/spec/uid")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    // Older shape: the dashboard is an embedded JSON string and
                    // the UID lives inside it.
                    let json = cr.data.pointer("/spec/json")?.as_str()?;
                    let parsed: Value = serde_json::from_str(json).ok()?;
                    parsed.get("uid")?.as_str().map(str::to_owned)
                })
        },
    )?;

    uids.extend(collect(
        &ctx.cluster
            .list_custom(GROUP, VERSION, "GrafanaManifest")
            .await?,
        |cr| {
            // A manifest can wrap any Grafana resource kind; only Dashboards
            // show up in `/api/search`, so filter rather than assume.
            let template = cr.data.get("spec")?.get("template")?;
            if template.get("kind")?.as_str()? != "Dashboard" {
                return None;
            }
            // For a v2 Dashboard the resource's own `metadata.name` *is* the UID.
            template
                .pointer("/metadata/name")
                .and_then(Value::as_str)
                .map(str::to_owned)
        },
    )?);

    uids.sort();
    uids.dedup();
    Ok(uids)
}

/// Pull a UID out of every custom resource, failing on any that has none.
///
/// A resource whose UID cannot be read is an error rather than a skip: silently
/// dropping it shrinks what the assertion covers without changing its result.
fn collect<F>(resources: &[DynamicObject], extract: F) -> Result<Vec<String>>
where
    F: Fn(&DynamicObject) -> Option<String>,
{
    resources
        .iter()
        .map(|cr| {
            extract(cr).ok_or_else(|| {
                let name = cr.metadata.name.as_deref().unwrap_or("<unnamed>");
                anyhow!("could not read a UID out of {name}")
            })
        })
        .collect()
}

/// Grafana's proxied backends answer with Prometheus-style envelopes.
fn expect_success(body: &Value) -> Result<()> {
    match body.get("status").and_then(Value::as_str) {
        Some("success") => Ok(()),
        // The `no org id` case lands here: Grafana passes the backend's error
        // through, so the message names the real cause rather than an empty
        // result set.
        Some(other) => bail!(
            "backend returned status {other:?}: {}",
            body.get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error message")
        ),
        None => bail!("response has no status field: {body}"),
    }
}
