// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `kube_state_metrics`: extra pod labels extend the chart's allowlist rather
//! than replacing it.
//!
//! Helm overwrites lists, so the wrong composition renders a valid flag that has
//! lost the chart's Materialize labels or its `nodes` entry, and the only symptom
//! is an empty panel. Read from the rendered Deployment, since that is the flag
//! kube-state-metrics parses, and against the chart's own values, so the check
//! follows the list rather than restating it.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;

use crate::terraform_render::ctx::Ctx;

const FLAG: &str = "--metric-labels-allowlist=";

pub fn pod_labels_extend_allowlist(ctx: &Ctx) -> Result<()> {
    let extra: Vec<&str> = ctx
        .declared("kube_state_metrics_pod_labels")
        .and_then(Value::as_array)
        .context("kube_state_metrics_pod_labels is not a list")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let chart: Vec<&str> = ctx
        .chart_default("kube-state-metrics.metricLabelsAllowlist")
        .and_then(Value::as_array)
        .context("the chart's values.yaml has no kube-state-metrics.metricLabelsAllowlist list")?
        .iter()
        .filter_map(Value::as_str)
        .collect();

    let mut want = parse(&chart);
    let pods = want.entry("pods".to_owned()).or_default();
    for label in extra {
        pods.push(label.to_owned());
    }
    dedup(pods);

    let flags: Vec<&str> = ctx
        .rendered
        .of_kind("Deployment")
        .flat_map(|deployment| deployment.container_args())
        .filter_map(|arg| arg.strip_prefix(FLAG))
        .collect();
    let [flag] = flags.as_slice() else {
        bail!("{} {FLAG} flags rendered, expected 1", flags.len());
    };
    let got = parse(&[flag]);
    if got != want {
        bail!(
            "kube_state_metrics_pod_labels did not compose: rendered {got:?}, expected {want:?}\n\
             See terraform/modules/materialize-monitoring/kube_state_metrics.tf."
        );
    }
    Ok(())
}

/// `resource=[label,label]` entries, keyed by resource.
fn parse(entries: &[&str]) -> BTreeMap<String, Vec<String>> {
    static ENTRY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\w+)=\[([^\]]*)\]").expect("valid regex"));
    ENTRY
        .captures_iter(&entries.join(","))
        .map(|entry| {
            let labels = entry[2].split(',').map(str::to_owned).collect();
            (entry[1].to_owned(), labels)
        })
        .collect()
}

/// Drop repeats, keeping the first of each.
fn dedup(labels: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    labels.retain(|label| seen.insert(label.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_flag_shape() {
        let parsed = parse(&["pods=[a,b]", "nodes=[c]"]);
        assert_eq!(parsed["pods"], vec!["a", "b"]);
        assert_eq!(parsed["nodes"], vec!["c"]);
        // The rendered flag is one string with the entries comma-joined.
        assert_eq!(parse(&["pods=[a,b],nodes=[c]"]), parsed);
    }
}
