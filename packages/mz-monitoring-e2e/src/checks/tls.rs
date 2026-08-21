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
//! The suite dials TLS hops with its own client, built from a cert-manager
//! Secret — see [`crate::tls`] — so it trusts the same CA the components do and
//! presents an identity that CA signed.
//!
//! With that client, [`gateway_requires_client_certificate`] is the one
//! assertion here about *authentication* rather than encryption — it dials a
//! phase-3 listener twice, once presenting and once not, and the two answers
//! have to differ.
//!
//! Deliberately **not** here yet: a certificate signed by a CA nothing trusts,
//! which is what would prove phase 2's `VerifyClientCertIfGiven` rejects rather
//! than ignores. That needs a second CA, which means provisioning a throwaway
//! issuer rather than reading an existing Secret.

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::checks::loki::LOKI_PORT;
use crate::cluster::ServiceTarget;
use crate::ctx::Ctx;
use crate::forward::is_tls_refusal;
use crate::retry::retry_until;

/// Alloy's own HTTP port, which serves the component-health API.
const ALLOY_ADMIN_PORT: u16 = 12345;

/// The gateway's log-ingest listener — `loki.source.api`, the `loki` port on the
/// `alloy-gateway` Service.
///
/// The one hop in the stack that reaches phase 3 *and* is reachable from here.
/// Thanos Receive's remote-write listener also requires a client certificate,
/// but the suite has no remote-write client to dial it with; Loki's HTTP port
/// stops at phase 2 because the kubelet probes it and a `httpGet` probe cannot
/// present a certificate.
const GATEWAY_LOGS_SERVICE: &str = "alloy-gateway";
const GATEWAY_LOGS_PORT: u16 = 3100;

/// Any path on the ingest listener: the assertion is about the handshake, and a
/// GET at a push endpoint answering `405` is proof the handshake finished.
const GATEWAY_PUSH_PATH: &str = "loki/api/v1/push";

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

/// Loki's HTTP port serves TLS, and *only* TLS.
///
/// Two halves, in this order on purpose. The positive half — a TLS client gets a
/// good answer — has to come first: without it the negative half passes for the
/// wrong reason on any stack where Loki is simply down, and "plaintext was
/// refused" is true of a listener that refuses everything.
///
/// The negative half is the one the module docstring is about. A value that
/// lands on a path nothing reads leaves the port on plaintext, and every other
/// assertion in the suite still passes — data flows, queries answer, the
/// certificates are Ready and mounted. Only asking for plaintext and being
/// turned away distinguishes a TLS listener from a configured intention.
pub async fn loki_refuses_plaintext(ctx: &Ctx) -> Result<()> {
    let service = ctx
        .cluster
        .first_existing_service(&ctx.features.loki_service_candidates().read)
        .await?;

    let secure = ServiceTarget::new(service.clone(), LOKI_PORT).with_tls(ctx.client_tls()?);
    let body = ctx
        .cluster
        .get(&secure, "ready")
        .await
        .context("Loki did not answer a TLS request on its HTTP port")?;
    if body.trim() != "ready" {
        bail!(
            "Loki answered TLS on {service}:{LOKI_PORT} with {:?}",
            body.trim()
        );
    }

    let plaintext = ServiceTarget::new(service.clone(), LOKI_PORT);
    match ctx.cluster.get(&plaintext, "ready").await {
        Err(_) => Ok(()),
        Ok(body) => bail!(
            "{service}:{LOKI_PORT} answered a *plaintext* request with {:?} while also answering \
             TLS. A listener cannot do both, so this is the port being reached by something other \
             than the TLS one the values configure — check that `loki.loki.server.http_tls_config` \
             is under the subchart key the release actually renders, and that no sidecar or proxy \
             is terminating in front of it.",
            body.trim()
        ),
    }
}

/// The gateway's log-ingest listener turns away a client with no certificate.
///
/// **The only assertion in the suite that tests authentication rather than
/// encryption**, and the reason the suite carries a TLS client at all. Every
/// other TLS check would pass identically on a stack where the listener is
/// encrypted and admits the entire cluster.
///
/// Ordered positive-then-negative, and both halves are load-bearing. Without the
/// positive half, a listener that is simply down passes the negative one — "the
/// anonymous client was refused" is true of a port that refuses everything, and
/// that reads as security rather than as an outage.
pub async fn gateway_requires_client_certificate(ctx: &Ctx) -> Result<()> {
    let trusted =
        ServiceTarget::new(GATEWAY_LOGS_SERVICE, GATEWAY_LOGS_PORT).with_tls(ctx.client_tls()?);

    // A non-2xx is expected and fine — `loki.source.api` answers a GET at its
    // push path with a method error. What matters is that the answer is HTTP at
    // all, which cannot happen unless the server accepted the certificate.
    if let Err(err) = ctx.cluster.get(&trusted, GATEWAY_PUSH_PATH).await
        && is_tls_refusal(&err)
    {
        return Err(err).context(
            "the gateway refused a client presenting a certificate from its own CA. The listener \
             requires one, so this is the trust root disagreeing rather than the policy working: \
             the CA the suite read out of its Secret is not the CA in clientCAFile. Check that \
             every component certificate comes from one issuer.",
        );
    }

    let anonymous =
        ServiceTarget::new(GATEWAY_LOGS_SERVICE, GATEWAY_LOGS_PORT).with_tls(ctx.anonymous_tls()?);
    match ctx.cluster.get(&anonymous, GATEWAY_PUSH_PATH).await {
        Err(err) if is_tls_refusal(&err) => Ok(()),
        Err(err) => Err(err).context(format!(
            "the gateway answered a client with no certificate at the HTTP layer, so `clientAuth: \
             {}` is not being enforced. An HTTP error here is not the check passing — the request \
             got past the part that was supposed to stop it.",
            ctx.features.gateway_logs_client_auth().unwrap_or("<unset>"),
        )),
        Ok(_) => bail!(
            "{GATEWAY_LOGS_SERVICE}:{GATEWAY_LOGS_PORT} served a client presenting no \
             certificate, while the values set `clientAuth: {}`. Alloy does not fail a listener \
             whose TLS block it cannot use — it goes unhealthy and binds nothing, or binds \
             without the policy — so check tls::alloy_components_healthy and the rendered \
             `.alloy` config before trusting this hop.",
            ctx.features.gateway_logs_client_auth().unwrap_or("<unset>"),
        ),
    }
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
