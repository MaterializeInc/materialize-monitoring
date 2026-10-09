// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `scheduling`: the fan-out reached every workload it should, and no others.
//!
//! A `nodeSelector` written to a path no subchart reads renders perfectly well
//! and is silently ignored, so rendering alone proves nothing. Counting is not
//! enough either: the per-node DaemonSets are supposed to *lack* the selector,
//! so these assert the exact set in both directions.
//!
//! The subcharts disagree about where scheduling goes — `thanos.global`,
//! `loki.defaults`, `alloy-*.controller`, top level for the rest — and three
//! Loki components render from their own templates rather than `_pod.tpl`, so
//! they fall out of `defaults` and must be named individually. That map lives in
//! two places (`terraform/modules/materialize-monitoring/scheduling.tf` for
//! Terraform, `charts/materialize-monitoring/profiles/scheduling.values.yaml`
//! for Helm), and this is what keeps them honest against the pinned subchart
//! versions.

use anyhow::{Context, Result};
use serde_json::{Map, Value};

use super::report;
use crate::terraform_render::ctx::Ctx;

const HINT: &str = "See scheduling.tf and profiles/scheduling.values.yaml.";

/// Workloads that must NOT carry a node selector, by name prefix.
///
/// Both are per-node collectors. A node selector *narrows* where a pod may run,
/// so on a DaemonSet whose job is to observe every node it is a silent blind
/// spot — the pods simply stop landing on the excluded nodes and no dashboard
/// shows a hole. Tolerations are the opposite (they widen placement), which is
/// why `alloy-agent` still takes those and only `node-exporter` is excluded from
/// both.
const NO_NODE_SELECTOR: &[&str] = &["alloy-agent", "node-exporter"];
const NO_TOLERATIONS: &[&str] = &["node-exporter"];

/// `minDomains` is only valid alongside `whenUnsatisfiable: DoNotSchedule`, and
/// the API server rejects the pod outright otherwise.
///
/// Worth catching at render time because the rejection surfaces as an admission
/// error on a controller nobody is watching, not as a failed `helm upgrade`.
/// `min_zones` in the module patches `minDomains` onto the hard zone rule only,
/// for exactly this reason; sweeping up the soft host rule alongside it would
/// render valid YAML that the API server refuses.
pub fn min_domains_need_do_not_schedule(ctx: &Ctx) -> Result<()> {
    let mut offenders = Vec::new();
    for (name, spec) in workloads(ctx) {
        let constraints = spec
            .get("topologySpreadConstraints")
            .and_then(Value::as_array)
            .into_iter()
            .flatten();
        for constraint in constraints {
            if constraint.get("minDomains").is_some()
                && constraint.get("whenUnsatisfiable").and_then(Value::as_str)
                    != Some("DoNotSchedule")
            {
                let key = constraint.get("topologyKey").and_then(Value::as_str);
                offenders.push(format!("{name} ({})", key.unwrap_or("?")));
            }
        }
    }
    report(
        listed(
            "carry minDomains on a non-DoNotSchedule constraint",
            offenders,
        ),
        "",
    )
}

/// Every workload but the per-node collectors carries the declared selector.
pub fn node_selector(ctx: &Ctx) -> Result<()> {
    let want = ctx
        .declared("node_selector")
        .and_then(Value::as_object)
        .context("node_selector is not a map")?;
    let (missing, unexpected) = audit(ctx, NO_NODE_SELECTOR, |spec| has_selector(spec, want));
    let mut problems = listed("missing the node selector", missing);
    problems.extend(listed("carry a node selector they must not", unexpected));
    report(problems, HINT)
}

/// Every workload but node-exporter tolerates every declared taint.
pub fn tolerations(ctx: &Ctx) -> Result<()> {
    let want = ctx
        .declared("tolerations")
        .and_then(Value::as_array)
        .context("tolerations is not a list")?;
    let (missing, unexpected) = audit(ctx, NO_TOLERATIONS, |spec| has_tolerations(spec, want));
    let mut problems = listed("missing the tolerations", missing);
    problems.extend(listed("carry tolerations they must not", unexpected));
    report(problems, HINT)
}

/// A caller's tolerations must not cost the agent the one the chart ships.
///
/// That one is keyless `Exists` on `NoSchedule`, i.e. every NoSchedule taint.
/// Helm overwrites lists, so `alloy-agent.controller.tolerations` is replaced
/// wholesale by whatever the module writes there; `daemonset_tolerations` in
/// scheduling.tf exists to append instead. The failure is invisible from the
/// values alone — the rendered pod carries a toleration list that looks entirely
/// correct while covering only the taints the caller happened to name.
pub fn agent_keeps_blanket_toleration(ctx: &Ctx) -> Result<()> {
    let offenders = workloads(ctx)
        .filter(|(name, _)| name.starts_with("alloy-agent"))
        .filter(|(_, spec)| !tolerations_of(spec).any(is_blanket_no_schedule))
        .map(|(name, _)| name.to_owned())
        .collect();
    report(
        listed("lost the chart's blanket NoSchedule toleration", offenders),
        HINT,
    )
}

fn workloads(ctx: &Ctx) -> impl Iterator<Item = (&str, &Value)> {
    ctx.rendered
        .workloads()
        .filter_map(|doc| Some((doc.name(), doc.pod_spec()?)))
}

/// Split workloads into (wrongly missing, wrongly present).
fn audit(
    ctx: &Ctx,
    excluded_prefixes: &[&str],
    carries: impl Fn(&Value) -> bool,
) -> (Vec<String>, Vec<String>) {
    let (mut missing, mut unexpected) = (Vec::new(), Vec::new());
    for (name, spec) in workloads(ctx) {
        let excluded = excluded_prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix));
        match (carries(spec), excluded) {
            (true, true) => unexpected.push(name.to_owned()),
            (false, false) => missing.push(name.to_owned()),
            _ => {}
        }
    }
    (missing, unexpected)
}

/// Whether a pod spec carries every expected node-selector label.
///
/// Subset rather than equality: node-exporter ships its own
/// `kubernetes.io/os: linux`, and maps merge rather than replace.
fn has_selector(spec: &Value, want: &Map<String, Value>) -> bool {
    let got = spec.get("nodeSelector");
    want.iter()
        .all(|(key, value)| got.and_then(|got| got.get(key)) == Some(value))
}

/// Whether a pod spec tolerates every expected taint key.
fn has_tolerations(spec: &Value, want: &[Value]) -> bool {
    let keys: Vec<Option<&Value>> = tolerations_of(spec).map(|t| t.get("key")).collect();
    want.iter().all(|t| keys.contains(&t.get("key")))
}

fn tolerations_of(spec: &Value) -> impl Iterator<Item = &Value> {
    spec.get("tolerations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn is_blanket_no_schedule(toleration: &Value) -> bool {
    toleration.get("key").is_none_or(Value::is_null)
        && toleration.get("operator").and_then(Value::as_str) == Some("Exists")
        && toleration.get("effect").and_then(Value::as_str) == Some("NoSchedule")
}

/// One line naming every offender, or nothing when there are none.
fn listed(label: &str, mut names: Vec<String>) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }
    names.sort();
    vec![format!(
        "{} workload(s) {label}: {}",
        names.len(),
        names.join(", ")
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selector_is_a_subset_match() {
        let want = json!({ "workload": "generic" });
        let want = want.as_object().unwrap();
        let spec =
            json!({ "nodeSelector": { "workload": "generic", "kubernetes.io/os": "linux" } });
        assert!(has_selector(&spec, want));
        assert!(!has_selector(&json!({}), want));
    }

    /// The chart's own toleration has no key at all, and the agent losing it is
    /// the failure; one that merely names a key is not a replacement for it.
    #[test]
    fn blanket_toleration_is_keyless() {
        assert!(is_blanket_no_schedule(
            &json!({ "operator": "Exists", "effect": "NoSchedule" })
        ));
        assert!(!is_blanket_no_schedule(
            &json!({ "key": "dedicated", "operator": "Exists", "effect": "NoSchedule" })
        ));
    }
}
