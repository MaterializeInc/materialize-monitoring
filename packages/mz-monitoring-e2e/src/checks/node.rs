// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Node metrics: do the two exporters agree on what a node is called.
//!
//! `infra-nodes` is built on a join that spans two exporters, and neither of them
//! knows it. kube-state-metrics names a node `node="<kubernetes name>"`;
//! node-exporter names the same machine `instance="<ip>:9100"` and carries the
//! Kubernetes name only as `nodename` on `node_uname_info`. That metric is the
//! only bridge between them, and the dashboard's node picker is that bridge
//! expressed as a variable: an operator selects a name from kube-state-metrics,
//! and the hidden `$nodeList` resolves it to an address through
//! `node_uname_info`.
//!
//! **Nothing fails loudly if the bridge breaks.** A relabel that rewrites
//! `nodename`, a node-exporter deployed with a different hostname source, or a
//! cluster where the kubelet's node name is not the machine's hostname all leave
//! both exporters healthy and every panel empty — the variable resolves to
//! nothing and each query matches no series. It is the same shape of failure as
//! the kube-state-metrics label collision next door: all the data present, all
//! the queries silent.
//!
//! So this asserts the join itself, which no unit test can reach: that the two
//! exporters report the same set of node names, and that a name resolves to
//! exactly one address.

use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::checks::thanos::instant_query;
use crate::cluster::ServiceTarget;
use crate::ctx::Ctx;
use crate::retry::retry_until;

const QUERY_SERVICE: &str = "thanos-query";
const QUERY_PORT: u16 = 9090;

/// The `node_*` families `infra-nodes` reads through the `$nodeList` join.
///
/// Only the two the join itself depends on. The rest of the node-exporter
/// surface is asserted by the queries that use it, not enumerated here.
const JOIN_METRICS: &[&str] = &["node_uname_info", "kube_node_info"];

/// The two exporters name the same machines.
///
/// The check that makes `$node` -> `$nodeList` meaningful. Compares the *sets*
/// rather than the counts, so a partial overlap — which is what a hostname-source
/// change looks like — reports which names are missing from which side.
pub async fn identity_joins_across_exporters(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "kube-state-metrics and node-exporter agree on node names",
        ctx.deadline,
        ctx.interval,
        || async {
            let ksm = instant_query(ctx, &target, "kube_node_info").await?;
            let exporter = instant_query(ctx, &target, "node_uname_info").await?;
            for (metric, series) in JOIN_METRICS.iter().zip([&ksm, &exporter]) {
                if series.is_empty() {
                    bail!("{metric} has not reported yet");
                }
            }

            let by_ksm: BTreeSet<&str> = ksm
                .iter()
                .filter_map(|s| s.pointer("/metric/node").and_then(Value::as_str))
                .collect();
            let by_exporter: BTreeSet<&str> = exporter
                .iter()
                .filter_map(|s| s.pointer("/metric/nodename").and_then(Value::as_str))
                .collect();

            if by_ksm != by_exporter {
                let missing_exporter: Vec<&&str> = by_ksm.difference(&by_exporter).collect();
                let missing_ksm: Vec<&&str> = by_exporter.difference(&by_ksm).collect();
                bail!(
                    "the node picker's join is broken: kube-state-metrics names {} node(s) and \
                     node-exporter names {}. Missing from node_uname_info: {missing_exporter:?}; \
                     missing from kube_node_info: {missing_ksm:?}",
                    by_ksm.len(),
                    by_exporter.len()
                );
            }

            // One address per name, or `$nodeList` resolves to several and every
            // panel silently aggregates machines the operator did not select.
            let mut instances: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
            for series in &exporter {
                let (Some(name), Some(instance)) = (
                    series.pointer("/metric/nodename").and_then(Value::as_str),
                    series.pointer("/metric/instance").and_then(Value::as_str),
                ) else {
                    continue;
                };
                instances.entry(name).or_default().insert(instance);
            }
            let ambiguous: Vec<(&&str, &BTreeSet<&str>)> =
                instances.iter().filter(|(_, v)| v.len() > 1).collect();
            if !ambiguous.is_empty() {
                bail!(
                    "a node name resolves to more than one node-exporter address, so the \
                     dashboard would blend machines: {ambiguous:?}"
                );
            }

            Ok(())
        },
    )
    .await
}

/// The node families report per node rather than collapsing to one identity.
///
/// The node-side counterpart of `kube_state::pods_are_distinguishable`, and the
/// same failure: with the exporter's labels overwritten, every
/// `kube_node_status_capacity` series reads as one node and every per-node panel
/// shows the wrong machine's numbers while looking perfectly healthy.
pub async fn capacity_is_reported_per_node(ctx: &Ctx) -> Result<()> {
    let target = ServiceTarget::new(QUERY_SERVICE, QUERY_PORT);

    retry_until(
        "kube_node_status_capacity distinguishes nodes and resources",
        ctx.deadline,
        ctx.interval,
        || async {
            let series = instant_query(ctx, &target, "kube_node_status_capacity").await?;
            if series.is_empty() {
                bail!("kube_node_status_capacity has not reported yet");
            }

            let nodes: BTreeSet<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric/node").and_then(Value::as_str))
                .collect();
            let resources: BTreeSet<&str> = series
                .iter()
                .filter_map(|s| s.pointer("/metric/resource").and_then(Value::as_str))
                .collect();

            // `cpu`, `memory` and `pods` are the three the Summary tab reads; a
            // node reporting none of them is one the dashboard cannot describe.
            for required in ["cpu", "memory", "pods"] {
                if !resources.contains(required) {
                    bail!(
                        "kube_node_status_capacity reports no {required} resource — \
                         resources present: {resources:?}"
                    );
                }
            }
            if nodes.is_empty() {
                bail!("kube_node_status_capacity carries no node label: {resources:?}");
            }
            Ok(())
        },
    )
    .await
}
