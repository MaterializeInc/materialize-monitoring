// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Thanos: is the store fanout healthy, is anything actually being scraped, and
//! does it arrive under the names it was scraped with.
//!
//! These do not run at tier 1 — Thanos needs object storage in every deployment
//! shape it supports, so a hermetic kind run cannot include it, and every
//! assertion here reports as ignored there. They were developed and verified
//! against a real cloud cluster.
//!
//! The distinction the last assertion draws is the one worth keeping: `up == 1`
//! means a target answered, `scrape_samples_scraped > 0` means it answered with
//! *data*. A target that is reachable and exporting nothing satisfies the first
//! and fails the second, and that is a real failure mode of a misconfigured
//! relabel rule.

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::cluster::{ServiceTarget, encode};
use crate::ctx::Ctx;
use crate::retry::retry_until;

const QUERY_SERVICE: &str = "thanos-query";
const QUERY_PORT: u16 = 9090;

/// Thanos Query answers.
///
/// Diagnostic. Ready says nothing about whether any store is reachable behind
/// it, which is the next assertion's job.
pub async fn ready(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until("thanos /-/ready", ctx.deadline, ctx.interval, || async {
        let body = ctx.cluster.get(&target, "-/ready").await?;
        if body.trim().eq_ignore_ascii_case("ok") {
            Ok(())
        } else {
            bail!("/-/ready returned {:?}", body.trim())
        }
    })
    .await
}

/// Every store endpoint Query fans out to is reachable.
///
/// `/api/v1/stores` groups endpoints by type (`receive`, `store`, `sidecar`, …).
/// A store that is registered but erroring still appears in the list, so the
/// assertion is on `lastError` rather than on the list being non-empty — an
/// unreachable store gateway leaves historical queries silently short of data
/// while Query itself stays ready.
pub async fn stores(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "every thanos store endpoint is healthy",
        ctx.deadline,
        ctx.interval,
        || async {
            let body = ctx.cluster.get_json(&target, "api/v1/stores").await?;
            expect_success(&body).context("listing stores")?;

            let groups = body
                .get("data")
                .and_then(Value::as_object)
                .map(|o| o.iter().collect::<Vec<_>>())
                .unwrap_or_default();

            let mut total = 0usize;
            let mut failing = Vec::new();
            for (kind, endpoints) in groups {
                for endpoint in endpoints.as_array().into_iter().flatten() {
                    total += 1;
                    // `lastError` is null on a healthy endpoint; anything else
                    // is the store's own description of what is wrong.
                    match endpoint.get("lastError") {
                        None | Some(Value::Null) => {}
                        Some(err) => failing.push(format!(
                            "{kind} {}: {err}",
                            endpoint
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("<unnamed>")
                        )),
                    }
                }
            }

            // Query with no stores answers every query successfully and empty,
            // which is the failure this exists to catch rather than tolerate.
            if total == 0 {
                bail!("thanos query has no store endpoints; every query will return empty");
            }
            if failing.is_empty() {
                Ok(())
            } else {
                bail!(
                    "{}/{total} thanos store endpoints are failing: {}",
                    failing.len(),
                    failing.join("; ")
                )
            }
        },
    )
    .await
}

/// Targets are up.
///
/// Non-empty is legitimate to demand: the stack scrapes itself, so `up` exists
/// with no Materialize instance anywhere in the cluster.
pub async fn targets_up(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "thanos has at least one target reporting up",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, "up").await?;
            let healthy = series
                .iter()
                .filter(|s| sample_value(s) == Some(1.0))
                .count();
            if healthy > 0 {
                Ok(())
            } else {
                bail!("no target reports up==1 ({} series returned)", series.len())
            }
        },
    )
    .await
}

/// Targets are being scraped, and returning data.
///
/// The assertion `up` cannot make. A target that is reachable but exports
/// nothing — a relabel rule that dropped every metric, a port that answers 200
/// with an empty body — reports `up == 1` and `scrape_samples_scraped == 0`.
pub async fn samples_scraped(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "thanos has at least one target returning samples",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, "scrape_samples_scraped").await?;
            let scraping = series
                .iter()
                .filter(|s| sample_value(s).is_some_and(|v| v > 0.0))
                .count();
            if scraping > 0 {
                Ok(())
            } else {
                bail!(
                    "every target scraped zero samples ({} series returned) — \
                     reachable but exporting nothing",
                    series.len()
                )
            }
        },
    )
    .await
}

/// One series of each Prometheus type, and the names it must reach Thanos under.
///
/// All from Alloy's own `/metrics`, which the gateway scrapes in every install
/// that has Thanos, so none of them depends on Materialize running.
const TYPED_NAMES: &[(&str, &str)] = &[
    ("counter", "process_cpu_seconds_total"),
    ("gauge", "go_goroutines"),
    (
        "histogram buckets",
        "alloy_component_evaluation_seconds_bucket",
    ),
    (
        "histogram count",
        "alloy_component_evaluation_seconds_count",
    ),
    ("histogram sum", "alloy_component_evaluation_seconds_sum"),
    ("summary quantiles", "go_gc_duration_seconds"),
    ("summary count", "go_gc_duration_seconds_count"),
    ("summary sum", "go_gc_duration_seconds_sum"),
];

/// What those series would be called had the round trip renamed them: the
/// counter without `_total`, the histogram under its bare family name.
const RENAMED_NAMES: &[&str] = &["process_cpu_seconds", "alloy_component_evaluation_seconds"];

/// Typed series reach Thanos under the names they were scraped with.
///
/// The gateway's scrapes honor metadata, so the series it converts to OTLP are
/// typed: a counter becomes a cumulative sum, and a histogram's `_bucket`,
/// `_count` and `_sum` become one histogram named for its family.
/// `otelcol.exporter.prometheus "outputBridge"` turns them back into Prometheus
/// series on the way to Thanos. Were that to come back renamed — a counter
/// without its `_total`, a histogram under its family name — every dashboard and
/// alert reading the old name would go quiet, and nothing else here would fail.
pub async fn typed_names_round_trip(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let names: Vec<&str> = TYPED_NAMES
        .iter()
        .map(|(_, name)| *name)
        .chain(RENAMED_NAMES.iter().copied())
        .collect();
    let query = format!(
        "count by (__name__) ({{__name__=~\"{}\"}})",
        names.join("|")
    );

    retry_until(
        "typed series reach thanos under their scraped names",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, &query).await?;
            let present: Vec<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric/__name__").and_then(Value::as_str))
                .collect();
            check_typed_names(&present)
        },
    )
    .await
}

/// The half of [`typed_names_round_trip`] that needs no cluster.
fn check_typed_names(present: &[&str]) -> Result<()> {
    let missing: Vec<String> = TYPED_NAMES
        .iter()
        .filter(|(_, name)| !present.contains(name))
        .map(|(kind, name)| format!("{name} ({kind})"))
        .collect();
    let renamed: Vec<&str> = RENAMED_NAMES
        .iter()
        .copied()
        .filter(|name| present.contains(name))
        .collect();

    if missing.is_empty() && renamed.is_empty() {
        return Ok(());
    }
    let mut problems = Vec::new();
    if !missing.is_empty() {
        problems.push(format!("missing from thanos: {}", missing.join(", ")));
    }
    if !renamed.is_empty() {
        problems.push(format!(
            "present under a renamed form: {}",
            renamed.join(", ")
        ));
    }
    bail!(
        "{}. The gateway's OTLP round trip (otelcol.receiver.prometheus \"inputBridge\" -> \
         otelcol.exporter.prometheus \"outputBridge\") no longer gives typed series back their \
         scraped names; check outputBridge's add_metric_suffixes and the scrapes' honor_metadata",
        problems.join("; ")
    )
}

/// Both rulers' remote-write to the gateway is keeping up.
///
/// A ruler that cannot write still evaluates and still notifies, so nothing
/// else in the suite fails: its WAL fills, each batch is retried forever, and
/// the gateway logs a TLS handshake error for every attempt. That is what a
/// ruler left on the wrong scheme, or without a CA or client certificate, looks
/// like once the mTLS profiles secure the gateway's metrics listener.
///
/// Measured as the newest sample queued minus the newest sample sent, per
/// queue, which is the lag upstream Prometheus alerts on at two minutes. An idle
/// queue reads zero, so a Loki ruler with no rules passes. `min by` keeps a
/// replaced pod's last series, which lingers for the lookback window under a
/// different `instance`, from failing a pod that has since caught up.
pub async fn rulers_remote_write_current(ctx: &Ctx, thanos_ruler: bool) -> Result<()> {
    const MAX_LAG_SECONDS: f64 = 120.0;
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let ns = ctx.cluster.namespace();
    let queries = [
        (
            "Thanos ruler",
            format!(
                "min by (pod, remote_name) (prometheus_remote_storage_queue_highest_timestamp_seconds{{job=\"thanos-ruler\", namespace=\"{ns}\"}} \
                 - prometheus_remote_storage_queue_highest_sent_timestamp_seconds{{job=\"thanos-ruler\", namespace=\"{ns}\"}})"
            ),
        ),
        (
            "Loki ruler",
            format!(
                "min by (pod, remote_name, tenant) (loki_ruler_wal_prometheus_remote_storage_queue_highest_timestamp_seconds{{namespace=\"{ns}\"}} \
                 - loki_ruler_wal_prometheus_remote_storage_queue_highest_sent_timestamp_seconds{{namespace=\"{ns}\"}})"
            ),
        ),
    ];

    retry_until(
        "both rulers' remote-write to the gateway is keeping up",
        ctx.deadline,
        ctx.interval,
        || async {
            let mut behind = Vec::new();
            for (who, query) in &queries {
                let series = instant_query(ctx, &target, query).await?;
                if *who == "Thanos ruler" && thanos_ruler && series.is_empty() {
                    bail!(
                        "no remote-write queue metrics from the Thanos ruler in Thanos. It runs \
                         stateless, so it always has a queue; either it is not scraped or it is \
                         no longer remote-writing (check thanos.ruler.remoteWrite)"
                    );
                }
                for s in &series {
                    let lag = sample_value(s).unwrap_or(0.0);
                    if lag > MAX_LAG_SECONDS {
                        let pod = s
                            .pointer("/metric/pod")
                            .and_then(Value::as_str)
                            .unwrap_or("<pod>");
                        let queue = s
                            .pointer("/metric/remote_name")
                            .and_then(Value::as_str)
                            .unwrap_or("<queue>");
                        // A queue that has never sent reads its newest
                        // timestamp as the lag, which is decades rather than a
                        // backlog, so say what it means.
                        behind.push(if lag > 1.0e9 {
                            format!("{who} {pod} ({queue}) has sent nothing since it started")
                        } else {
                            format!("{who} {pod} ({queue}) is {lag:.0}s behind")
                        });
                    }
                }
            }
            if behind.is_empty() {
                return Ok(());
            }
            bail!(
                "{}. Rule results and the ALERTS series are not reaching the gateway. If \
                 pipeline.metrics.gateway.server.tls is on, check each ruler's remote-write \
                 scheme, CA and client certificate, and the gateway's log for TLS handshake errors",
                behind.join("; ")
            )
        },
    )
    .await
}

/// Each Loki rule group is loaded by one ruler.
///
/// Without `enable_sharding` every ruler replica loads every group, so each
/// alert goes out once per replica and each recording-rule sample is written
/// once per replica into the same series. Every pod stays healthy, and nothing
/// else in the suite notices.
///
/// Read from the rule manager's per-group gauge, which each ruler exports only
/// for the groups it loaded. A ruler with no rules passes.
///
/// Only series scraped in the last [`FRESH_SECONDS`] count. A replaced pod's
/// series can stay in the five-minute lookback window under its old `instance`
/// (see [`rulers_remote_write_current`]), which is longer than an assertion's
/// default deadline, so without the filter a correct rollout reads as two
/// owners. While the ring changes, a group can also be loaded by its old and
/// new owner at once until each ruler's next sync, so a brief overlap is
/// retried rather than failed.
pub async fn loki_rule_groups_evaluated_once(ctx: &Ctx) -> Result<()> {
    // Six scrapes at the Loki subchart's default 15s ServiceMonitor interval.
    const FRESH_SECONDS: u32 = 90;
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let ns = ctx.cluster.namespace();
    let series = format!("loki_prometheus_rule_group_rules{{namespace=\"{ns}\"}}");
    let query = format!(
        "count by (rule_group) ({series} and (time() - timestamp({series})) < {FRESH_SECONDS}) > 1"
    );

    retry_until(
        "each Loki rule group is loaded by one ruler",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, &query).await?;
            if series.is_empty() {
                return Ok(());
            }
            // The label is `<rule file>;<group>`; the group name is what a
            // reader recognizes.
            let groups: Vec<String> = series
                .iter()
                .map(|s| {
                    let group = s
                        .pointer("/metric/rule_group")
                        .and_then(Value::as_str)
                        .unwrap_or("<group>");
                    let name = group.rsplit(';').next().unwrap_or(group);
                    format!("{name} ({} rulers)", sample_value(s).unwrap_or(0.0))
                })
                .collect();
            bail!(
                "{} loaded by more than one Loki ruler, so each is evaluated once per ruler. \
                 Check loki.loki.rulerConfig.enable_sharding, and that every ruler is in the \
                 memberlist cluster (/ruler/ring on a ruler lists the members it sees)",
                groups.join(", ")
            )
        },
    )
    .await
}

/// Run an instant query and return its result vector.
/// Run an instant query against a Prometheus-compatible endpoint.
///
/// `pub(crate)` so `node` can reuse it rather than start a third copy — there is
/// already one more in `kube_state`, which is worth collapsing into a shared
/// helper separately from this change.
pub(crate) async fn instant_query(
    ctx: &Ctx,
    target: &ServiceTarget,
    query: &str,
) -> Result<Vec<Value>> {
    let path = format!("api/v1/query?query={}", encode(query));
    let body = ctx.cluster.get_json(target, &path).await?;
    expect_success(&body).with_context(|| format!("querying {query}"))?;

    Ok(body
        .pointer("/data/result")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// The sample value out of an instant-query series.
///
/// Prometheus encodes it as `[<timestamp>, "<value>"]` — a *string*, so that
/// `NaN` and `Inf` survive JSON. Reading it as a number silently yields nothing.
pub(crate) fn sample_value(series: &Value) -> Option<f64> {
    series
        .pointer("/value/1")
        .and_then(Value::as_str)
        .and_then(|v| v.parse().ok())
}

fn expect_success(body: &Value) -> Result<()> {
    match body.get("status").and_then(Value::as_str) {
        Some("success") => Ok(()),
        Some(other) => bail!(
            "thanos returned status {other:?}: {}",
            body.get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error message")
        ),
        None => bail!("thanos response has no status field: {body}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{RENAMED_NAMES, TYPED_NAMES, check_typed_names, sample_value};
    use serde_json::json;

    fn all_typed() -> Vec<&'static str> {
        TYPED_NAMES.iter().map(|(_, name)| *name).collect()
    }

    #[test]
    fn every_typed_name_present_passes() {
        assert!(check_typed_names(&all_typed()).is_ok());
    }

    /// A counter that lost `_total` shows up twice: missing under its own name,
    /// present under the renamed one.
    #[test]
    fn a_counter_without_total_fails_on_both_counts() {
        let mut present = all_typed();
        present.retain(|name| *name != "process_cpu_seconds_total");
        present.push("process_cpu_seconds");
        let err = check_typed_names(&present).unwrap_err().to_string();
        assert!(
            err.contains("missing from thanos: process_cpu_seconds_total (counter)"),
            "{err}"
        );
        assert!(
            err.contains("present under a renamed form: process_cpu_seconds"),
            "{err}"
        );
    }

    /// Old series linger in the lookback window after a bad rollout, so the
    /// renamed form fails the check even with every expected name present.
    #[test]
    fn a_renamed_form_alongside_the_right_one_still_fails() {
        let mut present = all_typed();
        present.push(RENAMED_NAMES[1]);
        assert!(check_typed_names(&present).is_err());
    }

    /// Pins the string encoding. `as_f64` on this returns `None`, which would
    /// make every "is it scraping" assertion read as zero and fail on a healthy
    /// cluster.
    #[test]
    fn a_sample_value_is_a_string_not_a_number() {
        let series = json!({"metric": {}, "value": [1786832656.848, "363"]});
        assert_eq!(sample_value(&series), Some(363.0));
    }

    #[test]
    fn a_series_without_a_value_is_none() {
        assert_eq!(sample_value(&json!({"metric": {}})), None);
    }
}
