// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Alertmanager: one notifier or two, and whether every ruler reaches both.
//!
//! Each of these guards a failure that leaves every pod Ready.
//!
//! * Two replicas that cannot gossip are two independent notifiers, so every
//!   notification arrives twice. Nothing errors; the mesh port is simply closed,
//!   or open on TCP and not UDP.
//! * Gossip replicates silences and the notification log, **not alerts**. A ruler
//!   that notifies the load-balanced Service leaves a replica that never received
//!   the alerts its peer holds, and that replica has nothing to send when the
//!   peer is lost. The ruler's own discovery result is the only place that shows.
//! * An unscraped Alertmanager is the component that cannot report its own
//!   delivery failures — the meta-alerts that will read these series have
//!   nothing to read.
//!
//! The Loki ruler is not asserted on here. It builds a tenant's notifier only
//! once that tenant has rule groups, and a stack with no rules installed has
//! none, so its discovery result is absent for a correct configuration.

use anyhow::{Result, bail};
use serde_json::Value;

use crate::checks::thanos::{instant_query, sample_value};
use crate::cluster::ServiceTarget;
use crate::ctx::Ctx;
use crate::retry::retry_until;

const API_PORT: u16 = 9093;
const QUERY_SERVICE: &str = "thanos-query";
const QUERY_PORT: u16 = 9090;

/// The ClusterIP Service in front of Alertmanager.
///
/// The chart pins it to `alertmanager` through `alertmanager.fullnameOverride`,
/// which `helm get values --all` reports. The fallback mirrors the subchart's
/// own `alertmanager.fullname` for a release that cleared the override, or one
/// installed before the name was pinned, including the rule that collapses
/// `<release>-alertmanager` to `<release>` when the release name already
/// contains `alertmanager`.
pub fn service_name(ctx: &Ctx) -> String {
    if let Some(name) = ctx
        .features
        .string("alertmanager.fullnameOverride")
        .filter(|s| !s.is_empty())
    {
        return truncate(name);
    }
    let chart = ctx
        .features
        .string("alertmanager.nameOverride")
        .filter(|s| !s.is_empty())
        .unwrap_or("alertmanager");
    if ctx.release.contains(chart) {
        truncate(&ctx.release)
    } else {
        truncate(&format!("{}-{chart}", ctx.release))
    }
}

fn truncate(name: &str) -> String {
    name.chars()
        .take(63)
        .collect::<String>()
        .trim_end_matches('-')
        .to_owned()
}

/// The replica count the release asked for.
fn replicas(ctx: &Ctx) -> u64 {
    ctx.features
        .get("alertmanager.replicaCount")
        .and_then(Value::as_u64)
        .unwrap_or(1)
}

/// Every replica is a member of one gossip cluster, and it has settled.
///
/// Read from a single replica's own view of the mesh. A replica that cannot
/// reach its peer lists only itself, so a peer count below the replica count is
/// exactly the partition that doubles every notification.
pub async fn mesh_converged(ctx: &Ctx) -> Result<()> {
    let want = replicas(ctx);
    let target = ServiceTarget::new(service_name(ctx), API_PORT);

    retry_until(
        "alertmanager replicas form one settled gossip cluster",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx.cluster.get_json(&target, "api/v2/status").await?;
            let status = body
                .pointer("/cluster/status")
                .and_then(Value::as_str)
                .unwrap_or("<absent>");
            let peers: Vec<&str> = body
                .pointer("/cluster/peers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|p| p.get("address").and_then(Value::as_str))
                .collect();

            if status == "ready" && peers.len() as u64 == want {
                return Ok(());
            }
            bail!(
                "alertmanager cluster status is {status:?} with {} of {want} replicas as peers \
                 ({peers:?}). Replicas that cannot gossip notify independently, so every \
                 notification goes out once per replica — check that \
                 networkPolicies.alertmanager.ingress opens 9094 on both TCP and UDP",
                peers.len()
            )
        },
    )
    .await
}

/// Every Thanos ruler replica has discovered every Alertmanager replica.
///
/// `thanos_rule_alertmanagers_dns_provider_results` is the number of addresses a
/// ruler's Alertmanager lookup resolved to. Pointed at the headless Service it
/// is one per ready replica; pointed at the load-balanced Service it is one,
/// which is the misconfiguration this exists to catch.
///
/// Read through Thanos rather than from a ruler's own `/metrics`, for two
/// reasons: it covers every ruler replica rather than whichever one a
/// port-forward lands on, and the subchart's `thanos-ruler` Service selects on
/// `component: ruler` alone, so it also selects the Loki ruler's pods.
pub async fn thanos_ruler_reaches_every_replica(ctx: &Ctx) -> Result<()> {
    let want = replicas(ctx);
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let query = format!(
        "thanos_rule_alertmanagers_dns_provider_results{{namespace=\"{}\"}}",
        ctx.cluster.namespace()
    );

    retry_until(
        "every thanos ruler resolves one alertmanager address per replica",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, &query).await?;
            if series.is_empty() {
                bail!(
                    "no thanos_rule_alertmanagers_dns_provider_results in Thanos yet; either the \
                     ruler is not scraped or it has no Alertmanager configured at all"
                );
            }
            let short: Vec<String> = series
                .iter()
                .filter(|s| sample_value(s).map(|v| v as u64) != Some(want))
                .map(|s| {
                    format!(
                        "{} resolves {}",
                        s.pointer("/metric/pod")
                            .and_then(Value::as_str)
                            .unwrap_or("<pod>"),
                        s.pointer("/value/1").and_then(Value::as_str).unwrap_or("?")
                    )
                })
                .collect();
            if short.is_empty() {
                return Ok(());
            }
            bail!(
                "{} for {want} alertmanager replicas. Gossip does not replicate alerts, so each \
                 replica has to receive every alert directly: thanos.ruler.alertmanagers should \
                 address the headless Service with a dns+ lookup",
                short.join("; ")
            )
        },
    )
    .await
}

/// Alertmanager's own metrics reach Thanos, once per replica.
///
/// Both Services the subchart renders carry the same labels, so its
/// ServiceMonitor selects the headless one as well; the chart drops those
/// targets. Twice as many `up` series as replicas means the drop stopped
/// working, and none means nothing is watching the component that reports
/// delivery failures.
pub async fn scraped_once_per_replica(ctx: &Ctx) -> Result<()> {
    let want = replicas(ctx);
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let query = format!(
        "up{{app=\"alertmanager\", namespace=\"{}\"}}",
        ctx.cluster.namespace()
    );

    retry_until(
        "alertmanager is scraped once per replica",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, &query).await?;
            let headless: Vec<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric/service").and_then(Value::as_str))
                .filter(|svc| svc.ends_with("-headless"))
                .collect();
            if !headless.is_empty() {
                bail!(
                    "alertmanager is scraped through its headless Service too ({headless:?}), so \
                     every replica is scraped twice; the ServiceMonitor's headless drop is gone"
                );
            }
            let up = series
                .iter()
                .filter(|s| sample_value(s) == Some(1.0))
                .count() as u64;
            if up == want && series.len() as u64 == want {
                return Ok(());
            }
            bail!(
                "{up} of {} alertmanager scrape target(s) are up, for {want} replicas",
                series.len()
            )
        },
    )
    .await
}
