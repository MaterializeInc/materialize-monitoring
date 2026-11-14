// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `trust_bundle_ca_pem`: the CAs reach the ConfigMap the chart mounts, and
//! every pod that mounts it rolls when they change.
//!
//! Both halves fail silently. The chart mounts `mzmon-trust-bundle` optionally,
//! so a ConfigMap under any other name, or a key nothing reads, leaves the pods
//! running with no extra roots. And a Go process reads its roots once, so a pod
//! whose template carries no hash of the bundle keeps distrusting a rotated CA
//! until something else restarts it. A hash written to a path a subchart doesn't
//! merge into its pods reaches no pod at all.

use anyhow::{Result, bail};
use serde_json::Value;

use super::report;
use crate::terraform_render::ctx::Ctx;

/// The name the chart's volumes mount, and the module's ConfigMap.
const CONFIG_MAP: &str = "mzmon-trust-bundle";

/// The pod annotation the module rolls the consumers with.
const HASH_ANNOTATION: &str = "mzmon.materialize.cloud/trust-bundle-hash";

pub fn enabled(ctx: &Ctx) -> bool {
    ctx.declares("trust_bundle_ca_pem")
}

/// The module plans `mzmon-trust-bundle` with the declared PEM under `ca.crt`.
pub fn config_map_holds_the_pem(ctx: &Ctx) -> Result<()> {
    let Some(planned) = ctx.planned("kubernetes_config_map", "trust_bundle") else {
        bail!(
            "the example declares trust_bundle_ca_pem but the module plans no trust_bundle \
             ConfigMap, so every pod starts with only its image's roots"
        );
    };
    let mut problems = Vec::new();

    let name = planned.pointer("/metadata/0/name").and_then(Value::as_str);
    if name != Some(CONFIG_MAP) {
        problems.push(format!(
            "the ConfigMap is named {name:?}, but the chart mounts {CONFIG_MAP:?}"
        ));
    }

    let held = planned.pointer("/data/ca.crt").and_then(Value::as_str);
    if held != ctx.declared_str("trust_bundle_ca_pem") {
        problems.push("its `ca.crt` is not the declared trust_bundle_ca_pem".to_owned());
    }

    report(problems, "")
}

/// Every workload that mounts the bundle carries its hash, the same one.
///
/// Read from the rendered pods rather than the composed values, so a hash
/// written to a key the subchart never merges into its pod template fails here.
pub fn consumers_roll_on_change(ctx: &Ctx) -> Result<()> {
    let mut consumers = 0usize;
    let mut hashes = std::collections::BTreeSet::new();
    let mut problems = Vec::new();

    for workload in ctx.rendered.workloads() {
        if !mounts_the_bundle(workload.pod_spec()) {
            continue;
        }
        consumers += 1;
        let hash = workload
            .get("spec.template.metadata.annotations")
            .and_then(|annotations| annotations.get(HASH_ANNOTATION))
            .and_then(Value::as_str);
        match hash {
            Some(hash) => {
                hashes.insert(hash.to_owned());
            }
            None => problems.push(format!(
                "{} mounts {CONFIG_MAP} but its pod template has no {HASH_ANNOTATION}, so it \
                 keeps the CAs it started with",
                workload.id()
            )),
        }
    }

    if consumers == 0 {
        bail!("no rendered workload mounts the {CONFIG_MAP} ConfigMap");
    }
    if hashes.len() > 1 {
        problems.push(format!(
            "the consumers carry {} different {HASH_ANNOTATION} values: {hashes:?}",
            hashes.len()
        ));
    }
    report(
        problems,
        "The hash is written in trust_bundle.tf, one podAnnotations map per subchart.",
    )
}

/// Whether a pod spec has a volume from the `mzmon-trust-bundle` ConfigMap.
fn mounts_the_bundle(pod_spec: Option<&Value>) -> bool {
    pod_spec
        .and_then(|spec| spec.get("volumes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|volume| volume.pointer("/configMap/name").and_then(Value::as_str) == Some(CONFIG_MAP))
}
