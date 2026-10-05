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
//!
//! The pod-label checks cover the other half of the exporter's identity: which
//! pod labels `kube_pod_labels` carries. The allowlist decides that, and both of
//! its failure modes are quiet. Too narrow, and a join to a Materialize replica
//! matches nothing. Too broad, and the cost arrives later, as index size and
//! churn in the store.

use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::cluster::{ServiceTarget, encode};
use crate::ctx::Ctx;
use crate::features::Features;
use crate::retry::retry_until;

/// The pod label orchestratord puts on every replica pod, naming its cluster.
pub const CLUSTER_ID_POD_LABEL: &str = "cluster.environmentd.materialize.cloud/cluster-id";

/// The same pods' replica id.
const REPLICA_ID_POD_LABEL: &str = "cluster.environmentd.materialize.cloud/replica-id";

/// Requests per Materialize replica, through `kube_pod_labels`.
///
/// The shape a cost or right-sizing panel takes. Requests rather than
/// `kube_pod_info` because a join is only worth asserting on a family someone
/// would aggregate, and orchestratord sets requests on every replica pod.
///
/// The `group by` on the right drops `instance`, so the join holds when more
/// than one kube-state-metrics replica reports the same pod. Without it every
/// pod matches once per replica and the join fails as many-to-many.
const REPLICA_JOIN_QUERY: &str = r#"group by (namespace, cluster_id, replica_id) (
  kube_pod_container_resource_requests{resource="memory"}
  * on (namespace, pod) group_left (cluster_id, replica_id)
  group by (namespace, pod, cluster_id, replica_id) (kube_pod_labels{cluster_id!=""})
)"#;

const QUERY_SERVICE: &str = "thanos-query";
const QUERY_PORT: u16 = 9090;

/// The `kube_*` families the query registry depends on.
///
/// Kept in step with the registry by `registry_kube_metrics_are_covered` below,
/// which fails if a query starts using a family this list does not name. Written
/// out rather than loaded at runtime so the assertion binary needs no source tree
/// beside it.
pub const REGISTRY_KUBE_METRICS: &[&str] = &[
    "kube_daemonset_status_desired_number_scheduled",
    "kube_deployment_status_condition",
    "kube_deployment_status_replicas_ready",
    "kube_deployment_status_replicas_unavailable",
    "kube_endpointslice_endpoints",
    "kube_horizontalpodautoscaler_spec_max_replicas",
    "kube_horizontalpodautoscaler_spec_min_replicas",
    "kube_horizontalpodautoscaler_status_condition",
    "kube_horizontalpodautoscaler_status_current_replicas",
    "kube_horizontalpodautoscaler_status_desired_replicas",
    "kube_networkpolicy_spec_egress_rules",
    "kube_networkpolicy_spec_ingress_rules",
    "kube_node_created",
    "kube_node_info",
    "kube_node_labels",
    "kube_node_spec_taint",
    "kube_node_spec_unschedulable",
    "kube_node_status_allocatable",
    "kube_node_status_capacity",
    "kube_node_status_condition",
    "kube_pod_container_resource_limits",
    "kube_pod_container_resource_requests",
    "kube_pod_container_status_last_terminated_exitcode",
    "kube_pod_container_status_last_terminated_reason",
    "kube_pod_container_status_restarts_total",
    "kube_pod_container_status_waiting",
    "kube_pod_created",
    "kube_pod_info",
    "kube_pod_start_time",
    "kube_pod_status_phase",
    "kube_pod_status_ready",
    "kube_pod_status_scheduled",
    "kube_service_spec_type",
    "kube_service_status_load_balancer_ingress",
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

/// The pod labels the release's kube-state-metrics allowlist names.
///
/// `None` when the allowlist has no `pods` entry, in which case
/// kube-state-metrics does not publish `kube_pod_labels` at all and there is
/// nothing to assert.
pub fn allowlisted_pod_labels(features: &Features) -> Option<Vec<String>> {
    let entries = features
        .get("kube-state-metrics.metricLabelsAllowlist")?
        .as_array()?;
    let flag = entries
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(",");
    parse_allowlist(&flag).remove("pods")
}

/// Split a `--metric-labels-allowlist` value into its resources.
///
/// The syntax is `<resource>=[<key>,...]`, comma-separated. A label key cannot
/// contain `]`, so the first one after an opening bracket closes it.
fn parse_allowlist(flag: &str) -> BTreeMap<String, Vec<String>> {
    let mut resources: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut rest = flag;
    while let Some(open) = rest.find("=[") {
        let Some(len) = rest[open..].find(']') else {
            break;
        };
        let resource = rest[..open].trim_start_matches(',').trim();
        let keys = rest[open + 2..open + len]
            .split(',')
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(String::from);
        resources
            .entry(resource.to_owned())
            .or_default()
            .extend(keys);
        rest = &rest[open + len + 1..];
    }
    resources
}

/// The label name kube-state-metrics publishes a pod label under.
fn published_name(key: &str) -> String {
    let sanitized: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("label_{sanitized}")
}

/// `kube_pod_labels` stays one series per pod and carries only what the
/// allowlist names.
///
/// The cardinality bound on the family. Neither failure below breaks anything
/// when it starts:
///
/// - A pod with more than one current series from one scrape target is being
///   scraped twice, by two jobs or two collectors. Counted per `instance`,
///   because each kube-state-metrics replica of an HA deployment legitimately
///   reports every pod, and by `uid` rather than by name, so a StatefulSet pod
///   recreated under the same name is not mistaken for it. A label changed on a
///   running pod does not show here, because the old series goes stale at the
///   next scrape.
/// - A `label_*` name the allowlist does not account for means something
///   broader than the list is copying labels: a wildcard, or a second flag.
///   Names the monitor renames to a canonical form simply do not appear.
///
/// Runs on every tier. Once the allowlist names a pod label, kube-state-metrics
/// publishes the family for every pod in the cluster, the stack's own included,
/// so it is a self-monitoring series.
pub async fn pod_labels_are_bounded(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);
    let allowed = allowlisted_pod_labels(&ctx.features).unwrap_or_default();

    // Not retried: no amount of waiting makes a wildcard narrower.
    if allowed.iter().any(|k| k.contains('*')) {
        bail!(
            "the kube-state-metrics allowlist copies every pod label (`pods=[{}]`), so \
             kube_pod_labels has no bound. Name the labels individually",
            allowed.join(",")
        );
    }
    let allowed_names: BTreeSet<String> = allowed.iter().map(|k| published_name(k)).collect();

    retry_until(
        "kube_pod_labels stays one series per pod, within its allowlist",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, "kube_pod_labels").await?;
            if series.is_empty() {
                bail!("kube_pod_labels has not reported yet");
            }

            let repeated = instant_query(
                ctx,
                &target,
                "count by (uid, instance) (kube_pod_labels) > 1",
            )
            .await?;
            if !repeated.is_empty() {
                bail!(
                    "{} pod(s) carry more than one current kube_pod_labels series from the \
                     same kube-state-metrics target, so it is being scraped twice. Pod uids: \
                     {:?}",
                    repeated.len(),
                    repeated
                        .iter()
                        .filter_map(|s| s.pointer("/metric/uid").and_then(Value::as_str))
                        .take(5)
                        .collect::<Vec<_>>()
                );
            }

            let unexpected: BTreeSet<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric").and_then(Value::as_object))
                .flat_map(|labels| labels.keys())
                .map(String::as_str)
                .filter(|name| name.starts_with("label_") && !allowed_names.contains(*name))
                .collect();
            if !unexpected.is_empty() {
                bail!(
                    "kube_pod_labels carries {} label(s) the allowlist does not name, so \
                     something broader than `kube-state-metrics.metricLabelsAllowlist` is \
                     copying pod labels: {}",
                    unexpected.len(),
                    unexpected.into_iter().collect::<Vec<_>>().join(", ")
                );
            }

            Ok(())
        },
    )
    .await
}

/// A join through `kube_pod_labels` yields a series per Materialize replica.
///
/// The reason the allowlist names the Materialize labels: any `kube_pod_*`
/// family, grouped by cluster and replica rather than by namespace. It needs two
/// halves to agree, in two subchart keys: the allowlist copies the labels, and
/// the monitor's `metricRelabelings` renames them to `cluster_id` and
/// `replica_id`. Either half missing leaves the join matching nothing.
///
/// Compared against the pods themselves, read from the API server, so a replica
/// that is missing reports by name. Only runs where a Materialize replica
/// exists; the kind tiers run none.
pub async fn pod_labels_join_to_replicas(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "a join through kube_pod_labels yields a series per Materialize replica",
        ctx.deadline,
        ctx.interval,
        || async {
            let pods = ctx.cluster.pods_with_label(CLUSTER_ID_POD_LABEL).await?;
            let want: BTreeSet<(String, String, String)> = pods
                .iter()
                .filter_map(|pod| {
                    let labels = pod.metadata.labels.as_ref()?;
                    Some((
                        pod.metadata.namespace.clone()?,
                        labels.get(CLUSTER_ID_POD_LABEL)?.clone(),
                        labels.get(REPLICA_ID_POD_LABEL)?.clone(),
                    ))
                })
                .collect();
            if want.is_empty() {
                bail!("no pod carries {CLUSTER_ID_POD_LABEL} any more");
            }

            let series = instant_query(ctx, &target, REPLICA_JOIN_QUERY).await?;
            let got: BTreeSet<(String, String, String)> = series
                .iter()
                .filter_map(|s| {
                    let label = |name: &str| {
                        s.pointer(&format!("/metric/{name}"))
                            .and_then(Value::as_str)
                            .map(String::from)
                    };
                    Some((
                        label("namespace")?,
                        label("cluster_id")?,
                        label("replica_id")?,
                    ))
                })
                .collect();

            if got == want {
                return Ok(());
            }
            let missing: Vec<_> = want.difference(&got).collect();
            let extra: Vec<_> = got.difference(&want).collect();
            bail!(
                "requests joined through kube_pod_labels name {} replica(s), and the API \
                 server has {}. Missing (namespace, cluster_id, replica_id): {missing:?}; not \
                 on any pod: {extra:?}. With every replica missing, check that \
                 `kube-state-metrics.metricLabelsAllowlist` names {CLUSTER_ID_POD_LABEL} and \
                 that `kube-state-metrics.prometheus.monitor.http.metricRelabelings` renames \
                 it. With only some missing, check those pods set a memory request",
                got.len(),
                want.len()
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

    /// The allowlist reads the same whether its resources are one list item or
    /// several, which is the difference between the chart's shape and a
    /// hand-written override.
    #[test]
    fn allowlist_parses_per_resource() {
        let split = parse_allowlist("nodes=[a.io/x,b],pods=[c.io/y]");
        assert_eq!(split["nodes"], ["a.io/x", "b"]);
        assert_eq!(split["pods"], ["c.io/y"]);

        let features = Features::from_values(serde_json::json!({
            "kube-state-metrics": {
                "metricLabelsAllowlist": ["nodes=[a]", "pods=[c.io/y,d]", "pods=[e]"],
            }
        }));
        assert_eq!(
            allowlisted_pod_labels(&features).unwrap(),
            ["c.io/y", "d", "e"]
        );

        let no_pods = Features::from_values(serde_json::json!({
            "kube-state-metrics": { "metricLabelsAllowlist": ["nodes=[a]"] }
        }));
        assert!(allowlisted_pod_labels(&no_pods).is_none());
    }

    /// The chart's own allowlist names the label the join assertion is gated on.
    /// If it stops doing so, that assertion goes quietly ignored on every tier.
    #[test]
    fn chart_allowlists_the_cluster_id() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../charts/materialize-monitoring/values.yaml");
        let text = std::fs::read_to_string(&path).expect("read the chart's values.yaml");
        let entry = text
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix("- pods=["))
            .expect("the chart's allowlist has a pods entry");
        let flag = format!("pods=[{entry}");
        assert!(
            parse_allowlist(&flag)["pods"]
                .iter()
                .any(|k| k == CLUSTER_ID_POD_LABEL),
            "{flag} does not name {CLUSTER_ID_POD_LABEL}"
        );
    }

    #[test]
    fn published_names_match_kube_state_metrics() {
        assert_eq!(
            published_name("cluster.environmentd.materialize.cloud/cluster-id"),
            "label_cluster_environmentd_materialize_cloud_cluster_id"
        );
        assert_eq!(published_name("team"), "label_team");
    }
}
