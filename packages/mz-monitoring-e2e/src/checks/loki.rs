// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The logging round trip: agent → gateway → Loki → query.
//!
//! Only one of these four assertions detects a broken write path
//! ([`recent_query`]). The other three narrow down *where* a failure is; they do
//! not find one. Both kinds are worth having, but do not mistake a green
//! `/ready` for evidence that logs are arriving.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::cluster::{ServiceTarget, encode, unix_nanos_ago, unix_nanos_now};
use crate::ctx::Ctx;
use crate::promtext::sum_samples;
use crate::retry::retry_until;

pub(super) const LOKI_PORT: u16 = 3100;
const STREAMS_METRIC: &str = "loki_ingester_streams_created_total";

/// How far back the label queries look.
///
/// **Bounding them is not optional.** Unbounded, Loki's label endpoints span
/// every index period the schema knows about; SingleBinary answers instantly on
/// a tiny store, but a distributed Loki fans the request across queriers and
/// returns `504: request timed out, decrease the duration of the request`. Tier 1
/// passed this for months before tier 2 exposed it.
///
/// Wider than the write-path window on purpose: this asks *which labels exist*,
/// not whether data is arriving — [`recent_query`] owns that — so a brief lull in
/// ingestion should not fail it.
const LABEL_WINDOW: Duration = Duration::from_secs(3600);

/// Loki answers HTTP at all.
///
/// Diagnostic. `/ready` is green on a Loki whose write path is broken.
pub async fn ready(ctx: &Ctx) -> Result<()> {
    let service = ctx
        .cluster
        .first_existing_service(&ctx.features.loki_service_candidates().read)
        .await?;
    // No tenant header: `/ready` is not a tenant-scoped endpoint.
    let target = loki_target(ctx, service)?;

    retry_until("loki /ready", ctx.deadline, ctx.interval, || async {
        let body = ctx.cluster.get(&target, "ready").await?;
        if body.trim() == "ready" {
            Ok(())
        } else {
            bail!("/ready returned {:?}", body.trim())
        }
    })
    .await
}

/// Loki's ingester has created at least one stream.
///
/// Diagnostic, and specifically **not** proof of a live write path: Loki's
/// filesystem store survives a pod restart and WAL-replayed streams count toward
/// this counter, so it stays non-zero on a stack that stopped ingesting hours
/// ago. Its value is telling "nothing was ever written" apart from "the query is
/// wrong" — Loki answers a query against an empty store with `success` and no
/// data, which those two cases share.
pub async fn streams_created(ctx: &Ctx) -> Result<()> {
    let service = ctx
        .cluster
        .first_existing_service(&ctx.features.loki_service_candidates().ingester)
        .await?;
    let target = loki_target(ctx, service)?;

    retry_until(
        "loki ingested at least one stream",
        ctx.deadline,
        ctx.interval,
        || async {
            // With replicas > 1 the API-server proxy picks one endpoint per
            // request, so this reads an arbitrary ingester. That is sound for
            // the assertion — any ingester with streams proves ingestion — and
            // retrying re-picks, so an idle replica does not pin the result.
            let body = ctx.cluster.get(&target, "metrics").await?;
            match sum_samples(&body, STREAMS_METRIC) {
                Some(n) if n > 0.0 => Ok(()),
                Some(_) => bail!("{STREAMS_METRIC} is 0: nothing has been ingested"),
                None => bail!(
                    "{STREAMS_METRIC} is absent from {}:{LOKI_PORT}/metrics \
                     — this is not an ingester",
                    target.service
                ),
            }
        },
    )
    .await
}

/// The Kubernetes-derived labels this check requires, from the gateway's final
/// `stage.labels`.
///
/// **Not the whole label set**, and deliberately not asserted as one. A stream
/// also carries `level`, `component`, `job` and `service_name`, applied by
/// earlier stages; node journal logs carry `unit` and no `namespace` at all; and
/// a per-environment namespace adds `environment_id`. Asserting an exhaustive
/// inventory here would make every future label a test failure, which is the
/// opposite of what this guards.
///
/// These three are the ones whose presence proves the *gateway* relabelled the
/// line rather than something writing to Loki directly.
const GATEWAY_LABELS: &[&str] = &["namespace", "app", "container"];

/// Attributes that must stay structured metadata rather than becoming labels.
///
/// `pod` is the one that matters and the one that was a label until it was
/// demoted; the `k8s_`-prefixed four are the compatibility aliases removed
/// alongside it. Three of those merely duplicated the unprefixed labels, and
/// `k8s_pod` carried the cardinality problem.
const STRUCTURED_METADATA_ONLY: &[&str] = &[
    "pod",
    "k8s_pod",
    "k8s_namespace",
    "k8s_app",
    "k8s_container",
];

/// The gateway's relabelling stage ran, and it produced the label set we promise.
///
/// `namespace`, `app` and `container` are applied by the Alloy gateway, not by
/// the agent, so their presence separates "the gateway forwarded and relabelled"
/// from "something wrote to Loki".
///
/// The absent half is asserted just as hard. **`pod` is structured metadata, not
/// a label**, and so are the four `k8s_`-prefixed aliases this pipeline used to
/// emit. A pod name is unbounded and changes on every restart, so promoting it
/// to a label means a new stream per pod per rollout — the cardinality mistake
/// that is cheap to make and expensive to undo, since it is already in the index
/// by the time anyone notices. Nothing fails loudly when a label reappears, which
/// is why it is checked here rather than left to review.
///
/// The two halves read different sources, and that asymmetry is the point. Which
/// labels *exist* is a fair question for the label API. Which labels are *still
/// being written* is not: that API answers over a window, so it goes on reporting
/// a label for as long as any stream in the window carried it, and a correct
/// rollout would keep failing until the last old stream aged out. The absent half
/// therefore queries the recent window for lines actually carrying each label.
pub async fn gateway_labels(ctx: &Ctx) -> Result<()> {
    let service = ctx
        .cluster
        .first_existing_service(&ctx.features.loki_service_candidates().read)
        .await?;
    let target = loki_target(ctx, service)?.with_tenant(ctx.features.loki_tenant());

    retry_until(
        "the gateway applies namespace/app/container, and pod stays metadata",
        ctx.deadline,
        ctx.interval,
        || async {
            let path = format!(
                "loki/api/v1/labels?start={}&end={}",
                unix_nanos_ago(LABEL_WINDOW)?,
                unix_nanos_now()?,
            );
            let body = ctx.cluster.get_json(&target, &path).await?;
            expect_success(&body).context("querying labels")?;
            let labels = body
                .get("data")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .unwrap_or_default();

            let missing: Vec<&str> = GATEWAY_LABELS
                .iter()
                .copied()
                .filter(|label| !labels.contains(label))
                .collect();
            if !missing.is_empty() {
                bail!(
                    "the gateway's relabelling did not run: missing {}. Loki knows only: {}",
                    missing.join(", "),
                    labels.join(", ")
                );
            }

            // Deliberately *not* asked of the label API. That reports every label
            // any stream carried anywhere in the window, so a correct rollout
            // keeps failing until the last pre-rollout stream ages out of it --
            // a false failure lasting a whole `LABEL_WINDOW`. Found exactly that
            // way: against a cluster whose deployed pipeline had already stopped
            // emitting these, the label API still listed all four.
            //
            // So ask what is being written *now* instead: select on each label
            // over the recent window and require no stream to come back.
            let mut promoted = Vec::new();
            for label in STRUCTURED_METADATA_ONLY {
                let selector = format!("{{{label}=~\".+\"}}");
                let start = unix_nanos_ago(ctx.recent_window)?;
                let path = format!(
                    "loki/api/v1/query_range?query={}&limit=1&start={start}",
                    encode(&selector)
                );
                let body = ctx.cluster.get_json(&target, &path).await?;
                expect_success(&body).with_context(|| format!("querying {selector}"))?;
                let streams = body
                    .pointer("/data/result")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or_default();
                if streams > 0 {
                    promoted.push(*label);
                }
            }
            if !promoted.is_empty() {
                bail!(
                    "these are structured metadata and must not be stream labels, but lines \
                     written in the last {}s carry them as labels: {}. Each one multiplies \
                     stream cardinality -- `pod` alone means a new stream per pod per rollout",
                    ctx.recent_window.as_secs(),
                    promoted.join(", ")
                );
            }

            Ok(())
        },
    )
    .await
}

/// A log line written inside the recent window is queryable.
///
/// **This is the load-bearing assertion of the tier.** It is bounded to a recent
/// window on purpose: an unbounded query is satisfied by chunks already on disk,
/// so it passes against a stack whose write path is broken. That was verified by
/// breaking the write path deliberately — the unbounded query and the streams
/// counter both still passed.
///
/// Keep the window meaningfully smaller than how long a broken stack would have
/// been broken. Too wide and stale-but-in-window chunks satisfy it, which is how
/// the shell version of this check first passed against a stack that had stopped
/// ingesting.
pub async fn recent_query(ctx: &Ctx) -> Result<()> {
    let service = ctx
        .cluster
        .first_existing_service(&ctx.features.loki_service_candidates().read)
        .await?;
    let target = loki_target(ctx, service)?.with_tenant(ctx.features.loki_tenant());

    // `namespace` is gateway-applied, so this selector also confirms the logs
    // came through the pipeline rather than from something writing to Loki
    // directly.
    let selector = format!("{{namespace=\"{}\"}}", ctx.cluster.namespace());
    let what = format!(
        "query_range returns a stream written in the last {}s",
        ctx.recent_window.as_secs()
    );

    retry_until(&what, ctx.deadline, ctx.interval, || async {
        // Recomputed per attempt: a `start` pinned before the retry loop drifts
        // into a wider and wider window the longer we retry, so a check that
        // eventually passes would be asserting something weaker than the one
        // that passed immediately.
        let start = unix_nanos_ago(ctx.recent_window)?;
        let path = format!(
            "loki/api/v1/query_range?query={}&limit=1&start={start}",
            encode(&selector)
        );

        let body = ctx.cluster.get_json(&target, &path).await?;
        expect_success(&body).context("querying recent logs")?;

        let results = body
            .pointer("/data/result")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if results > 0 {
            Ok(())
        } else {
            bail!(
                "no streams matching {selector} in the last {}s",
                ctx.recent_window.as_secs()
            )
        }
    })
    .await
}

/// Loki reports `status: success`.
///
/// Checked separately from the payload so that an auth failure reads as one. A
/// read without the tenant header fails with `no org id`, and the resulting
/// empty payload is otherwise indistinguishable from a stack that ingested
/// nothing.
fn expect_success(body: &Value) -> Result<()> {
    match body.get("status").and_then(Value::as_str) {
        Some("success") => Ok(()),
        Some(other) => bail!(
            "loki returned status {other:?}: {}",
            body.get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error message")
        ),
        None => bail!("loki response has no status field: {body}"),
    }
}

/// A target for Loki's HTTP port, speaking TLS when the release moved that port
/// to TLS.
///
/// Every direct Loki assertion goes through here rather than deciding for
/// itself, because getting it wrong is not a visible failure: a plaintext GET at
/// a TLS listener comes back `400 Client sent an HTTP request to an HTTPS
/// server`, which reads as a malformed query rather than as the wrong scheme.
fn loki_target(ctx: &Ctx, service: impl Into<String>) -> Result<ServiceTarget> {
    let target = ServiceTarget::new(service, LOKI_PORT);
    if ctx.features.loki_server_tls() {
        Ok(target.with_tls(ctx.client_tls()?))
    } else {
        Ok(target)
    }
}
