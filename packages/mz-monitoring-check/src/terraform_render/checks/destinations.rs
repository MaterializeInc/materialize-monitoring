// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `destinations`: every extra metric destination reaches the gateway pipeline.
//!
//! Each fans out *in addition to* Thanos, so a destination that missed its path
//! leaves an install that keeps working, keeps writing to Thanos, and never
//! mentions what went missing.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::{Map, Value};

use super::report;
use crate::terraform_render::ctx::{Ctx, str_at};

/// The annotations that bind a pod to a cloud identity. A `sigv4` destination
/// has no other source of credentials, so one of these has to be on the
/// gateway's ServiceAccount for it to authenticate at all.
const IDENTITY_ANNOTATIONS: &[&str] = &[
    "eks.amazonaws.com/role-arn",
    "iam.gke.io/gcp-service-account",
    "azure.workload.identity/client-id",
];

/// Every declared Prometheus remote-write destination lands, with its own tier
/// filter.
///
/// `pipeline.metrics.gateway.destination.prometheusRemoteWrite` is a **map**,
/// and Helm deep-merges maps: a module writing to a mistyped path adds a key
/// beside the chart's `thanos` rather than replacing anything. So each
/// destination is checked in three places, because each fails differently and
/// only the first is visible in the config at all:
///
///   * the `prometheus.remote_write "<name>"` component — absent, nothing writes
///     there and no error is raised anywhere;
///   * its URL variable in the gateway env ConfigMap — absent, `sys.env`
///     resolves to the empty string, which Alloy accepts at load and every write
///     then fails at run time;
///   * its tier allowlist variable — absent, the filter falls back to `.*` and a
///     destination asked for `essential` silently receives the full firehose. On
///     a backend that bills per sample that is the expensive failure, and it is
///     invisible from the cluster's side.
///
/// A `sigv4` destination is checked one step further, because it is the one
/// auth type with *nothing* in the gateway Secret: the ServiceAccount annotation
/// is its entire credential. Missing, the AWS SDK finds no web-identity token and
/// every write comes back 403 naming neither the role nor the values key.
pub fn remote_write(ctx: &Ctx) -> Result<()> {
    let declared = ctx
        .declared("prometheus_remote_write")
        .and_then(Value::as_object)
        .context("prometheus_remote_write is not a map")?;
    let config = ctx.rendered.gateway_config()?;
    let env = ctx.rendered.gateway_env()?;
    // Read back from the composed values rather than taken from the
    // declaration, so the module's own document is what gets checked — that is
    // the half most likely to be wrong.
    let empty = Map::new();
    let composed = ctx
        .composed("pipeline.metrics.gateway.destination.prometheusRemoteWrite")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let annotations = ctx
        .rendered
        .find("ServiceAccount", "alloy-gateway")
        .and_then(|sa| sa.get("metadata.annotations"))
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    let mut problems = Vec::new();
    for (name, destination) in declared {
        let component = format!("prometheus.remote_write \"{name}\"");
        if destination.get("enabled") == Some(&Value::Bool(false)) {
            if config.contains(&component) {
                problems.push(format!(
                    "{name}: declared enabled=false but a component still rendered"
                ));
            }
            continue;
        }
        let Some(composed) = composed.get(name) else {
            problems.push(format!(
                "{name}: the module composed no \
                 pipeline.metrics.gateway.destination.prometheusRemoteWrite.{name} document"
            ));
            continue;
        };

        pipeline_problems(name, &config, &mut problems);
        env_problems(name, destination, composed, &env, &mut problems);

        if str_at(destination, "auth_type") == Some("sigv4")
            && !IDENTITY_ANNOTATIONS
                .iter()
                .any(|annotation| annotations.contains_key(*annotation))
        {
            problems.push(format!(
                "{name}: auth_type is sigv4, but the alloy-gateway ServiceAccount carries none of \
                 {} — there is nothing for the AWS SDK to sign as, so every write is refused with \
                 a 403",
                IDENTITY_ANNOTATIONS.join(", ")
            ));
        }
    }
    report(
        problems,
        "See destinations.tf and the prometheusRemoteWrite map in the chart.",
    )
}

/// The Google Cloud destination reaches the gateway pipeline.
///
/// The module writes it as the `pipeline.destinations` entry `googleCloud`, so
/// the proof is in three places: its tier allowlist variable, its edge off the
/// metric egress seam, and the OTLP exporter aimed at the Telemetry API, which
/// is what writes.
pub fn google_cloud(ctx: &Ctx) -> Result<()> {
    let mut problems = otel_destination_problems(ctx, "googleCloud", "Google Cloud")?;
    if !ctx
        .rendered
        .gateway_config()?
        .contains(r#"otelcol.exporter.otlphttp "dest_googleCloud_gcp""#)
    {
        problems.push(
            "google_cloud_metrics is set but no OTLP exporter to the Telemetry API rendered"
                .to_owned(),
        );
    }
    report(problems, "")
}

/// The Datadog destination reaches the gateway pipeline, reading the API key
/// the module's Secret supplies.
///
/// The Secret and the values name the variable independently, so a rename on
/// either side leaves an exporter that loads and authenticates with an empty
/// key.
pub fn datadog(ctx: &Ctx) -> Result<()> {
    let mut problems = otel_destination_problems(ctx, "datadog", "Datadog")?;
    let config = ctx.rendered.gateway_config()?;
    if !config.contains(r#"otelcol.exporter.datadog "dest_datadog""#) {
        problems.push("datadog_metrics is set but no Datadog exporter rendered".to_owned());
    } else if !config.contains(r#"api_key = sys.env("GATEWAY_OTEL_DEST_DATADOG_API_KEY")"#) {
        problems.push(
            "the Datadog exporter does not read GATEWAY_OTEL_DEST_DATADOG_API_KEY, the variable \
             gateway_credentials.tf puts the API key in"
                .to_owned(),
        );
    }
    report(problems, "")
}

/// The OTLP destination reaches the gateway pipeline, with its endpoint.
pub fn otlp(ctx: &Ctx) -> Result<()> {
    let mut problems = otel_destination_problems(ctx, "otlp", "OTLP")?;
    if !ctx
        .rendered
        .gateway_env()?
        .contains_key("GATEWAY_OTLP_DEST")
    {
        problems.push(
            "otlp_metrics is set but GATEWAY_OTLP_DEST, the endpoint the exporter reads, did not \
             render"
                .to_owned(),
        );
    }
    report(problems, "")
}

/// What every OTLP-family `pipeline.destinations` entry needs: its tier
/// allowlist in the env ConfigMap, and its tier filter on the metric egress
/// seam. Absent, the allowlist falls back to `.*`, and a destination asked for
/// `essential` silently receives everything; absent, the edge leaves an orphan
/// filter that never receives a sample.
fn otel_destination_problems(ctx: &Ctx, name: &str, destination: &str) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let allowlist = format!("GATEWAY_{}_DEST_METRICS_ALLOWLIST", slug(name));
    if !ctx.rendered.gateway_env()?.contains_key(&allowlist) {
        problems.push(format!(
            "the {destination} destination is set but no {allowlist} metric filter rendered"
        ));
    }
    let edge = format!("otelcol.processor.transform.dest_{name}.input");
    if !ctx.rendered.gateway_config()?.contains(&edge) {
        problems.push(format!(
            "the {destination} destination is set but the metric egress seam does not forward \
             to {edge}"
        ));
    }
    Ok(problems)
}

/// The components a destination needs in the rendered pipeline.
fn pipeline_problems(name: &str, config: &str, problems: &mut Vec<String>) {
    if !config.contains(&format!("prometheus.remote_write \"{name}\"")) {
        problems.push(format!(
            "{name}: no prometheus.remote_write \"{name}\" component rendered"
        ));
        return;
    }
    // The tier filter is a separate component upstream of the writer. Without it
    // the destination still writes, so nothing looks broken — it just ignores its
    // own min_importance.
    if !config.contains(&format!("prometheus.relabel \"{name}\"")) {
        problems.push(format!(
            "{name}: has a remote_write component but no prometheus.relabel tier filter, so \
             min_importance would be ignored"
        ));
    }
    // An orphan component is valid Alloy that never receives a sample.
    if !config.contains(&format!("prometheus.relabel.{name}.receiver")) {
        problems.push(format!(
            "{name}: nothing forwards to it — the prometheus.relabel.egress fan-out does not list \
             prometheus.relabel.{name}.receiver"
        ));
    }
}

/// The URL and tier variables the pipeline reads for this destination.
fn env_problems(
    name: &str,
    declared: &Value,
    composed: &Value,
    env: &BTreeMap<String, String>,
    problems: &mut Vec<String>,
) {
    let url_env = format!("GATEWAY_PROM_DEST_{}", slug(name));
    match (env.get(&url_env), str_at(composed, "url")) {
        (None, _) => problems.push(format!(
            "{name}: the gateway env ConfigMap sets no {url_env}"
        )),
        (Some(got), Some(want)) if got != want => {
            problems.push(format!("{name}: {url_env} is {got:?}, expected {want:?}"))
        }
        _ => {}
    }

    let tier_env = format!("GATEWAY_UNFILTERED_PROM_METRICS_{}", slug(name));
    let want_tier = str_at(declared, "min_importance").unwrap_or("all");
    match env.get(&tier_env).map(String::as_str) {
        None => problems.push(format!(
            "{name}: the gateway env ConfigMap sets no {tier_env}"
        )),
        Some(got) if want_tier == "all" && got != ".*" => problems.push(format!(
            "{name}: min_importance is 'all' but {tier_env} is not '.*'"
        )),
        Some(".*") if want_tier != "all" => problems.push(format!(
            "{name}: min_importance is {want_tier:?} but {tier_env} is '.*', so the tier filter \
             passes everything"
        )),
        Some(_) => {}
    }
}

/// The env-var fragment for a destination name.
///
/// Mirrors the chart's `regexReplaceAll "[^A-Za-z0-9]" "_" | upper` and the
/// module's `upper(replace(name, "/[^A-Za-z0-9]/", "_"))`. A third copy is not
/// ideal, but the alternative is asserting the derivation against itself.
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slug_matches_the_chart() {
        assert_eq!(slug("amp"), "AMP");
        assert_eq!(slug("us-east.amp"), "US_EAST_AMP");
    }
}
