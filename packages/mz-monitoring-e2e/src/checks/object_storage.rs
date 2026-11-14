// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Object storage: every component that uses a bucket reaches it.
//!
//! Tier 2's store serves HTTPS with a certificate from a CA that no image
//! trusts, so a component reaches it only through the trust bundle. That makes
//! this the trust bundle's assertion as well as the bucket's.
//!
//! It reads the objstore client's own counters, which Thanos and Loki both
//! export, rather than waiting for data to land. Thanos Receive uploads a block
//! every two hours and a Loki ingester flushes a chunk only once it is full or
//! idle, both longer than a test run. The store gateway, the compactors and
//! Loki's index sync list their buckets at startup.
//!
//! The pass condition is a success per component, not zero failures. A
//! component that started before the store answered fails a few operations and
//! then recovers, which is fine. One that fails every operation it tries, which
//! is what an untrusted certificate does, never records a success.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde_json::Value;

use crate::cluster::ServiceTarget;
use crate::ctx::Ctx;
use crate::retry::retry_until;

use super::thanos::{QUERY_PORT, QUERY_SERVICE, instant_query, sample_value};

/// Thanos and Loki prefix the objstore client's families differently, so the
/// name is matched by its suffix.
const OPERATIONS: &str = "objstore_bucket_operations_total";
const FAILURES: &str = "objstore_bucket_operation_failures_total";

/// Each component that uses a bucket has completed at least one operation.
///
/// Requires Thanos, which is where the counters are read, and asserts on Loki
/// as well when Loki stores to a bucket rather than its filesystem.
pub async fn reachable(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let ns = ctx.cluster.namespace();
    let loki_expected = ctx.features.enabled("loki")
        && ctx
            .features
            .string("loki.loki.storage.type")
            .is_some_and(|kind| kind != "filesystem");

    retry_until(
        "every component that uses object storage reaches it",
        ctx.deadline,
        ctx.interval,
        || async {
            let operations = by_component(ctx, &target, OPERATIONS, ns).await?;
            let failures = by_component(ctx, &target, FAILURES, ns).await?;

            let mut problems = Vec::new();
            let mut attempted = Vec::new();
            for (component, tried) in &operations {
                if *tried <= 0.0 {
                    continue;
                }
                attempted.push(component.clone());
                let failed = failures.get(component).copied().unwrap_or(0.0);
                if failed >= *tried {
                    problems.push(format!(
                        "{component} failed all {tried} of its object-storage operations"
                    ));
                }
            }

            let tried = |product: &str| attempted.iter().any(|c| c.contains(product));
            if !tried("thanos") {
                problems.push("no Thanos component has tried its bucket yet".to_owned());
            }
            if loki_expected && !tried("loki") {
                problems.push("no Loki component has tried its bucket yet".to_owned());
            }

            if problems.is_empty() {
                return Ok(());
            }
            bail!(
                "{}\nComponents with operations: {}.\n\
                 A store behind a private CA needs it in the trust bundle, the \
                 `mzmon-trust-bundle` ConfigMap, before the pods start. Look for \
                 `x509: certificate signed by unknown authority` in the component's log",
                problems.join("\n"),
                if attempted.is_empty() {
                    "none".to_owned()
                } else {
                    attempted.join(", ")
                },
            )
        },
    )
    .await
}

/// One family, summed per scrape job and keyed `family{job}`, so a message
/// names both the product and the component.
async fn by_component(
    ctx: &Ctx,
    target: &ServiceTarget,
    suffix: &str,
    ns: &str,
) -> Result<BTreeMap<String, f64>> {
    let query =
        format!(r#"sum by (__name__, job) ({{__name__=~"(.+_)?{suffix}",namespace="{ns}"}})"#);
    let series = instant_query(ctx, target, &query).await?;
    Ok(series
        .iter()
        .filter_map(|s| {
            let name = s.pointer("/metric/__name__").and_then(Value::as_str)?;
            let job = s
                .pointer("/metric/job")
                .and_then(Value::as_str)
                .unwrap_or("<no job>");
            Some((format!("{name}{{job={job:?}}}"), sample_value(s)?))
        })
        .collect())
}
