// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! In-cluster TLS.
//!
//! These exist because of three failures that a running cluster reports as
//! healthy, each of which was hit by hand before it was written down:
//!
//! * **A component whose TLS config is rejected does not necessarily crash.**
//!   `otelcol.receiver.otlp` given a `min_version` it does not understand goes
//!   unhealthy and never binds its listener, while the pod stays `Running` and
//!   `Ready`. Neither pod status nor `alloy validate` shows it; only the
//!   component-health endpoint does.
//! * **cert-manager can stop renewing and keep saying everything is fine.** With
//!   `renewBefore` close to `duration` the controller livelocked, stopped
//!   issuing, and left `Ready=True: "Certificate is up to date and has not
//!   expired"` on certificates that had expired 45 minutes earlier.
//! * **A hop configured for TLS can still be serving plaintext**, when a value
//!   lands on a path nothing reads. A successful query proves data flowed, not
//!   how it travelled.
//!
//! Deliberately **not** here yet: presenting a client certificate, and presenting
//! one signed by the wrong CA. Both need a TLS client over the forwarded stream.
//! Until that lands, "phase 3 refuses an anonymous client" is verified by hand.
//! A green run here means encrypted, issued and healthy — **not authenticated**.

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::cluster::ServiceTarget;
use crate::ctx::Ctx;
use crate::retry::retry_until;

/// Alloy's own HTTP port, which serves the component-health API.
const ALLOY_ADMIN_PORT: u16 = 12345;

/// Every `Certificate` is Ready, and none has expired.
///
/// Both halves are load-bearing. The Ready condition alone is not enough:
/// cert-manager has been observed reporting `Ready=True` with the message
/// "Certificate is up to date and has not expired" about a certificate whose
/// `notAfter` was already in the past. Comparing `notAfter` against the clock is
/// the part a wedged controller cannot fake.
pub async fn certificates_ready(ctx: &Ctx) -> Result<()> {
    let certs = ctx
        .cluster
        .list_custom("cert-manager.io", "v1", "Certificate")
        .await
        .context("listing Certificate resources")?;

    if certs.is_empty() {
        bail!(
            "certificates are enabled in the release values but no Certificate resources exist. \
             A missing CRD would have failed the apply, so this means the values did not reach \
             the chart."
        );
    }

    let mut problems = Vec::new();
    for cert in &certs {
        let name = cert
            .metadata
            .name
            .clone()
            .unwrap_or_else(|| "<unnamed>".into());
        let status = cert.data.get("status");

        let ready = status
            .and_then(|s| s.get("conditions"))
            .and_then(Value::as_array)
            .and_then(|cs| {
                cs.iter()
                    .find(|c| c.get("type").and_then(Value::as_str) == Some("Ready"))
            });

        match ready.and_then(|c| c.get("status")).and_then(Value::as_str) {
            Some("True") => {}
            other => {
                let msg = ready
                    .and_then(|c| c.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("no message");
                problems.push(format!(
                    "{name}: Ready={} ({msg})",
                    other.unwrap_or("absent")
                ));
                continue;
            }
        }

        // Ready says the controller is content. Check what it actually issued.
        if let Some(not_after) = status
            .and_then(|s| s.get("notAfter"))
            .and_then(Value::as_str)
            .filter(|t| is_past(t))
        {
            {
                problems.push(format!(
                    "{name}: cert-manager reports Ready=True but notAfter is {not_after}, which \
                     has passed. That combination is the signature of a livelocked controller — \
                     check that renewBefore is well under duration."
                ));
            }
        }
    }

    if !problems.is_empty() {
        bail!(
            "{} of {} certificates unhealthy:\n  {}",
            problems.len(),
            certs.len(),
            problems.join("\n  ")
        );
    }
    Ok(())
}

/// Every Alloy component is healthy.
///
/// The assertion that catches a rejected TLS config. Alloy keeps running when a
/// component fails to build: the process is up, the pod is Ready, and the
/// listener that component owns is simply never bound. The gateway's two OTLP
/// ports were dark across a whole deploy cycle that way.
pub async fn alloy_components_healthy(ctx: &Ctx) -> Result<()> {
    for role in ["alloy-gateway", "alloy-agent"] {
        if !ctx.features.enabled(role) {
            continue;
        }
        let target = ServiceTarget::new(role, ALLOY_ADMIN_PORT);

        retry_until(
            &format!("{role} component health"),
            ctx.deadline,
            ctx.interval,
            || async {
                let components: Vec<Value> = ctx
                    .cluster
                    .get_json(&target, "api/v0/web/components")
                    .await
                    .map(|v| serde_json::from_value(v).unwrap_or_default())?;

                if components.is_empty() {
                    bail!("{role} reported no components at all");
                }

                let unhealthy: Vec<String> = components
                    .iter()
                    .filter_map(|c| {
                        let state = c.pointer("/health/state")?.as_str()?;
                        // `unknown` is the pre-evaluation state; only genuine
                        // failures are interesting.
                        if state == "healthy" || state == "unknown" {
                            return None;
                        }
                        let id = c
                            .get("localID")
                            .and_then(Value::as_str)
                            .unwrap_or("<unnamed>");
                        // The same exemption list the rest of the suite honours.
                        // `loki.source.journal.node_logs` fails to start on
                        // every cluster the Alloy image runs on, and an
                        // exemption honoured here is printed in the run output
                        // rather than hidden.
                        if ctx.allow_unhealthy.iter().any(|a| a == id) {
                            return None;
                        }
                        let msg = c
                            .pointer("/health/message")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        Some(format!("{id} is {state}: {msg}"))
                    })
                    .collect();

                if !unhealthy.is_empty() {
                    bail!(
                        "{role} has {} unhealthy component(s) while its pod reports Ready:\n  {}",
                        unhealthy.len(),
                        unhealthy.join("\n  ")
                    );
                }
                Ok(())
            },
        )
        .await?;
    }
    Ok(())
}

/// Delivery survives a certificate renewal.
///
/// The failure a freshly-installed cluster cannot see, and the one the design
/// gates every hop on: material is renewed, and a process that read it once at
/// startup keeps using the old copy until something restarts it.
///
/// Renewal is **forced** by deleting the Secret rather than provoked with a short
/// `renewBefore`. Provoking it is slower, and at lifetimes short enough to fit a
/// test run it has been observed to livelock cert-manager — a test that breaks
/// the mechanism it measures is worse than no test.
pub async fn survives_renewal(ctx: &Ctx) -> Result<()> {
    // Loki's, because the logging round trip below is what proves delivery, and
    // Loki is on both ends of it.
    let target_secret = "mzmon-loki-tls";

    let before = ctx
        .cluster
        .secret_value(target_secret, "tls.crt")
        .await
        .with_context(|| format!("reading {target_secret} before renewal"))?;

    ctx.cluster.delete_secret(target_secret).await?;

    // Poll for material that is genuinely different rather than merely present:
    // a Secret that reappears with identical bytes means the read raced the
    // delete and nothing was actually renewed.
    retry_until(
        "certificate reissued",
        ctx.deadline,
        ctx.interval,
        || async {
            let after = ctx.cluster.secret_value(target_secret, "tls.crt").await?;
            if after == before {
                bail!("{target_secret} still holds the pre-renewal certificate");
            }
            Ok(())
        },
    )
    .await?;

    // The point of the whole assertion: the pipeline keeps delivering across it.
    // Reuses the logging round trip rather than inventing a second one, so a
    // failure here means renewal specifically — the same check passed before.
    super::loki::recent_query(ctx).await.context(
        "logs stopped flowing after a forced certificate renewal. This is the reload failure the \
         file-mount convention exists to prevent: something is still presenting or trusting the \
         material it read at startup. Check alloy_components_healthy and the Loki server logs \
         before turning any hop's default on.",
    )
}

/// Whether an RFC3339 timestamp is in the past.
fn is_past(ts: &str) -> bool {
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) else {
        // A format this cannot read is not evidence of expiry; downgrade to a
        // no-op rather than a false alarm.
        return false;
    };
    // `SystemTime` rather than `Utc::now()`, which needs chrono's `clock`
    // feature and the dependencies behind it for one comparison. The crate
    // already reads the wall clock this way.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    parsed.timestamp() < now
}
