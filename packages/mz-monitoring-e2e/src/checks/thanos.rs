// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Thanos: is the store fanout healthy, and is anything actually being scraped.
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

/// Run an instant query and return its result vector.
async fn instant_query(ctx: &Ctx, target: &ServiceTarget, query: &str) -> Result<Vec<Value>> {
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
fn sample_value(series: &Value) -> Option<f64> {
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
    use super::sample_value;
    use serde_json::json;

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
