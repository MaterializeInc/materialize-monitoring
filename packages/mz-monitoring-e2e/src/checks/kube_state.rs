// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! kube-state-metrics: are its labels the object's, or the exporter's own.
//!
//! This exists because the failure it catches produced no error anywhere.
//!
//! Every `kube_*` series carries `namespace`, `pod` and `container` labels
//! describing *the object it reports on* — that is what the exporter is for.
//! Those names collide with the target labels of the scrape, which describe the
//! kube-state-metrics pod. Without `honorLabels`, Prometheus resolves the
//! collision by renaming the exporter's labels to `exported_namespace` /
//! `exported_pod` and writing the *target's* identity into `namespace` / `pod`.
//!
//! Nothing is missing afterwards. Every series arrives, `up` is 1,
//! `scrape_samples_scraped` is healthy, and the existing Thanos assertions all
//! pass. But every series reads `namespace="<the monitoring namespace>"`, so
//! `kube_pod_info` collapses from one series per pod to a single identity, and
//! every dashboard query written as `kube_*{namespace=…}` — which is all of ours
//! — silently matches nothing. It shipped that way and was found by eye.
//!
//! So these assert the two things that are unambiguous signatures of the
//! collision rather than trying to check each panel: that no `exported_*` label
//! exists on the families our queries depend on, and that `kube_pod_info` still
//! distinguishes pods from one another.

use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;

use crate::cluster::{ServiceTarget, encode};
use crate::ctx::Ctx;
use crate::retry::retry_until;

const QUERY_SERVICE: &str = "thanos-query";
const QUERY_PORT: u16 = 9090;

/// The `kube_*` families the query registry depends on.
///
/// Kept in step with the registry by `registry_kube_metrics_are_covered` below,
/// which fails if a query starts using a family this list does not name. Written
/// out rather than loaded at runtime so the assertion binary needs no source tree
/// beside it.
pub const REGISTRY_KUBE_METRICS: &[&str] = &[
    "kube_deployment_status_condition",
    "kube_deployment_status_replicas_ready",
    "kube_deployment_status_replicas_unavailable",
    "kube_horizontalpodautoscaler_spec_max_replicas",
    "kube_horizontalpodautoscaler_status_current_replicas",
    "kube_node_spec_taint",
    "kube_node_status_condition",
    "kube_pod_container_resource_limits",
    "kube_pod_container_resource_requests",
    "kube_pod_container_status_last_terminated_exitcode",
    "kube_pod_container_status_last_terminated_reason",
    "kube_pod_container_status_restarts_total",
    "kube_pod_container_status_waiting",
    "kube_pod_created",
    "kube_pod_start_time",
    "kube_pod_status_phase",
    "kube_statefulset_status_replicas_ready",
];

/// No `kube_*` family our queries use carries an `exported_*` label.
///
/// The direct signature of the collision. `exported_namespace` exists only
/// because something already took `namespace`, so its presence is proof the
/// scrape is overwriting the exporter rather than honoring it — and its absence
/// is proof it is not.
pub async fn labels_are_honored(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "kube-state-metrics series keep their own namespace and pod labels",
        ctx.deadline,
        ctx.interval,
        || async {
            let mut clobbered = Vec::new();
            let mut seen = 0usize;

            for metric in REGISTRY_KUBE_METRICS {
                let series = instant_query(ctx, &target, metric).await?;
                if series.is_empty() {
                    continue;
                }
                seen += 1;
                let overwritten: BTreeSet<&str> = series
                    .iter()
                    .filter_map(|s| s.pointer("/metric").and_then(Value::as_object))
                    .flat_map(|labels| labels.keys())
                    .filter(|label| label.starts_with("exported_"))
                    .map(String::as_str)
                    .collect();
                if !overwritten.is_empty() {
                    clobbered.push(format!(
                        "{metric} carries {}",
                        overwritten.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
            }

            // Nothing to judge yet. A freshly-installed kube-state-metrics can
            // take a scrape or two to appear, and reporting "no collision" from
            // an empty result would pass this assertion for the wrong reason.
            if seen == 0 {
                bail!(
                    "none of the {} kube_* families the registry uses has reported yet",
                    REGISTRY_KUBE_METRICS.len()
                );
            }

            if clobbered.is_empty() {
                Ok(())
            } else {
                bail!(
                    "kube-state-metrics labels are being overwritten by the scrape's target \
                     labels, so every one of these reads the exporter's own namespace and pod: \
                     {}. Set `kube-state-metrics.prometheus.monitor.http.honorLabels: true`",
                    clobbered.join("; ")
                )
            }
        },
    )
    .await
}

/// `kube_pod_info` still tells pods apart.
///
/// The consequence of the collision rather than its signature, and the one that
/// makes it obvious what was lost: with the exporter's `pod` label overwritten,
/// every pod in the cluster reports under the kube-state-metrics pod's name and
/// the family collapses to a single series identity.
///
/// One distinct pod is the failure. A cluster running this stack has many.
pub async fn pods_are_distinguishable(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "kube_pod_info distinguishes more than one pod",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, "kube_pod_info").await?;
            if series.is_empty() {
                bail!("kube_pod_info has not reported yet");
            }
            let pods: BTreeSet<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric/pod").and_then(Value::as_str))
                .collect();
            let namespaces: BTreeSet<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric/namespace").and_then(Value::as_str))
                .collect();

            if pods.len() > 1 && namespaces.len() > 1 {
                return Ok(());
            }
            bail!(
                "kube_pod_info reports {} series but only {} distinct pod(s) and {} distinct \
                 namespace(s) ({:?} / {:?}) — the exporter's labels have been overwritten by \
                 the scrape target's",
                series.len(),
                pods.len(),
                namespaces.len(),
                pods,
                namespaces
            )
        },
    )
    .await
}

/// Run an instant query and return its result vector.
async fn instant_query(ctx: &Ctx, target: &ServiceTarget, query: &str) -> Result<Vec<Value>> {
    let path = format!("api/v1/query?query={}", encode(query));
    let body = ctx.cluster.get_json(target, &path).await?;
    match body.get("status").and_then(Value::as_str) {
        Some("success") => {}
        other => bail!("querying {query}: thanos returned status {other:?}"),
    }
    Ok(body
        .pointer("/data/result")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzmon_lib::query::{QueryEngine, QueryRegistry, render::doc_context};
    use std::collections::BTreeSet;

    /// Every `kube_*` family the registry actually uses is in the list above.
    ///
    /// The link that makes the runtime assertion mean something: without it the
    /// list is a snapshot that quietly stops covering the queries it exists for.
    /// A query adopting a new family fails here rather than going unchecked.
    #[test]
    fn registry_kube_metrics_are_covered() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
        let registry = QueryRegistry::from_directory(&dir).expect("load the query registry");
        let ctx = doc_context(&registry, QueryEngine::PromQl, "mz_");

        let mut used = BTreeSet::new();
        for query in registry.iter_metric_queries() {
            if query.promql.is_empty() {
                continue;
            }
            let Ok(metrics) = query.extract_metrics(&ctx) else {
                continue;
            };
            for metric in metrics {
                if metric.name.starts_with("kube_") {
                    used.insert(metric.name);
                }
            }
        }

        let covered: BTreeSet<String> = REGISTRY_KUBE_METRICS
            .iter()
            .map(|m| m.to_string())
            .collect();

        let uncovered: Vec<&String> = used.difference(&covered).collect();
        assert!(
            uncovered.is_empty(),
            "the registry uses kube_* families the e2e check does not assert on: {uncovered:?}"
        );

        let stale: Vec<&String> = covered.difference(&used).collect();
        assert!(
            stale.is_empty(),
            "the e2e check names kube_* families no query uses any more: {stale:?}"
        );
    }
}
