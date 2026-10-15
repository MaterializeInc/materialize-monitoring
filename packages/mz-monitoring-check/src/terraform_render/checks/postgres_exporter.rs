// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `postgres_exporter`: the module's exporter targets reach the scrape, their
//! passwords reach the exporter's Secret and nothing else, and the declared
//! metadata databases reach the recording rules.
//!
//! Each coupling fails quietly. A target missing from the scrape is a database
//! nobody watches. A password that misses the Secret is a probe the exporter
//! answers with HTTP 400. A declaration that misses `externalDependencies` is a
//! recording rule matching `a^`, which records nothing and errors nowhere.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;

use super::grafana;
use crate::terraform_render::ctx::{Ctx, parse_yaml, str_at};

/// The name the chart gives its ServiceMonitor for the exporter.
const MONITOR: &str = "mzmon-postgres-exporter";

/// The Secret the chart mounts by default, which the module creates.
const SECRET: &str = "postgres-exporter";

pub fn enabled(ctx: &Ctx) -> bool {
    ctx.declared("enable_postgres_exporter")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The target names the example declared, plus `grafana` where the module adds
/// Grafana's own database.
fn expected_targets(ctx: &Ctx) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = ctx
        .declared("postgres_exporter_targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|target| str_at(target, "name"))
        .map(str::to_owned)
        .collect();
    if grafana::database_enabled(ctx) && grafana::password_secret_expected(ctx) {
        names.insert("grafana".to_owned());
    }
    names
}

/// Every declared target, and Grafana's, is one probe endpoint of the scrape,
/// and the exporter mounts the Secret Terraform plans.
pub fn targets_reach_the_scrape(ctx: &Ctx) -> Result<()> {
    let monitor = ctx
        .rendered
        .find("ServiceMonitor", MONITOR)
        .with_context(|| format!("no {MONITOR} ServiceMonitor rendered"))?;
    let scraped: BTreeSet<String> = monitor
        .get("spec.endpoints")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|endpoint| endpoint.pointer("/params/auth_module/0"))
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let expected = expected_targets(ctx);
    if scraped != expected {
        bail!(
            "the {MONITOR} ServiceMonitor probes {scraped:?}, but the example declares {expected:?}"
        );
    }

    let planned = ctx
        .planned("kubernetes_secret", "postgres_exporter")
        .and_then(|secret| secret.pointer("/metadata/0/name"))
        .and_then(Value::as_str)
        .context("the module plans no postgres_exporter Secret")?;
    let deployment = ctx
        .rendered
        .find("Deployment", "postgres-exporter")
        .context("no postgres-exporter Deployment rendered")?;
    let mounted = deployment
        .pod_spec()
        .and_then(|spec| spec.get("volumes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|volume| volume.pointer("/secret/secretName"))
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if planned != SECRET || !mounted.contains(&planned) {
        bail!(
            "the exporter mounts {mounted:?}, and the module plans the Secret {planned:?}: they \
             have to be the same {SECRET:?}, or the pod waits on a volume that never appears"
        );
    }
    Ok(())
}

/// Every declared password is in the planned Secret's config file, under its
/// target's auth module, and none reached the Helm release.
pub fn passwords_stay_in_the_secret(ctx: &Ctx) -> Result<()> {
    let config = ctx
        .planned("kubernetes_secret", "postgres_exporter")
        .and_then(|secret| secret.pointer("/data/postgres_exporter.yml"))
        .and_then(Value::as_str)
        .context("the planned postgres_exporter Secret has no postgres_exporter.yml")?;
    let config = parse_yaml(config).context("postgres_exporter.yml does not parse")?;

    let mut declared: Vec<(String, &str)> = ctx
        .declared("postgres_exporter_passwords")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, password)| Some((name.clone(), password.as_str()?)))
        .collect();
    if expected_targets(ctx).contains("grafana")
        && let Some(password) = ctx.declared_str("grafana_database_password")
    {
        declared.push(("grafana".to_owned(), password));
    }

    let mut problems = Vec::new();
    for (name, password) in &declared {
        let module = config.pointer(&format!("/auth_modules/{name}/userpass/password"));
        if module.and_then(Value::as_str) != Some(password) {
            problems.push(format!("auth_modules.{name} does not carry its password"));
        }
        if ctx.rendered.text().contains(password) {
            problems.push(format!(
                "the password for {name} reached the Helm release, where 'helm get values' reads it"
            ));
        }
    }
    if !problems.is_empty() {
        bail!("{}", problems.join("\n"));
    }
    Ok(())
}

/// Whether the example declares any metadata database, by provider or by target.
pub fn declares_metadata(ctx: &Ctx) -> bool {
    ctx.declares("metadata_databases") || !metadata_targets(ctx).is_empty()
}

fn metadata_targets(ctx: &Ctx) -> Vec<&str> {
    if !enabled(ctx) {
        return Vec::new();
    }
    ctx.declared("postgres_exporter_targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|target| target.get("metadata_database").and_then(Value::as_bool) == Some(true))
        .filter_map(|target| str_at(target, "name"))
        .collect()
}

/// Every declared metadata database is an `externalDependencies.consensus`
/// entry, and each exporter target among them reaches the `postgres` adapter's
/// rules.
pub fn metadata_databases_reach_the_rules(ctx: &Ctx) -> Result<()> {
    let composed: Vec<(String, String)> = ctx
        .composed("externalDependencies.consensus")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some((
                str_at(entry, "flavor")?.to_owned(),
                str_at(entry, "resourceId")?.to_owned(),
            ))
        })
        .collect();

    let mut expected: Vec<(String, String)> = ctx
        .declared("metadata_databases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some((
                str_at(entry, "flavor")?.to_owned(),
                str_at(entry, "resource_id")?.to_owned(),
            ))
        })
        .collect();
    expected.extend(
        metadata_targets(ctx)
            .into_iter()
            .map(|name| ("postgres".to_owned(), name.to_owned())),
    );

    let missing: Vec<String> = expected
        .iter()
        .filter(|entry| !composed.contains(entry))
        .map(|(flavor, id)| format!("{flavor}/{id}"))
        .collect();
    if !missing.is_empty() {
        bail!(
            "externalDependencies.consensus is missing: {}",
            missing.join(", ")
        );
    }

    let rules = ctx
        .rendered
        .find("PrometheusRule", "mzmon-ext-consensus")
        .context("no mzmon-ext-consensus PrometheusRule rendered")?;
    let exprs: Vec<&str> = rules
        .get("spec.groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|group| str_at(group, "name") == Some("ext_consensus_postgres"))
        .filter_map(|group| group.get("rules")?.as_array())
        .flatten()
        .filter_map(|rule| str_at(rule, "expr"))
        .collect();
    let recorded = recorded_targets(&exprs);
    let unrecorded: Vec<&str> = metadata_targets(ctx)
        .into_iter()
        .filter(|name| !recorded.contains(&name.to_lowercase()))
        .collect();
    if !unrecorded.is_empty() {
        bail!(
            "the ext_consensus_postgres rules do not read {}: the declaration did not reach the \
             recording rules",
            unrecorded.join(", ")
        );
    }
    Ok(())
}

/// The target names the `ext_consensus_postgres` rules read, lowercased, from
/// every `target=~"(?i)…"` matcher in them.
///
/// The chart joins every declared name into one alternation, `(?i)a|b`, so a
/// name is read whole out of the matcher rather than searched for: only the
/// first alternative follows `(?i)`. Lowercased because the rules match
/// ignoring case.
fn recorded_targets(exprs: &[&str]) -> BTreeSet<String> {
    static MATCHER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"target=~"\(\?i\)([^"]*)""#).expect("the target matcher pattern compiles")
    });
    exprs
        .iter()
        .flat_map(|expr| MATCHER.captures_iter(expr))
        .flat_map(|captures| {
            captures[1]
                .split('|')
                .map(|name| name.trim_matches(|c| c == '(' || c == ')').to_lowercase())
                .collect::<Vec<_>>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alternative_of_the_matcher_is_recorded() {
        // The shape the chart renders for two declared metadata targets.
        let expr = r#"max by (resource) (label_replace(pg_up{job="postgres-exporter", target=~"(?i)consensus-a|Consensus-B"}, "resource", "$1", "target", "(.+)"))"#;
        assert_eq!(
            recorded_targets(&[expr]),
            BTreeSet::from(["consensus-a".to_owned(), "consensus-b".to_owned()])
        );
    }

    #[test]
    fn an_undeclared_matcher_records_nothing() {
        // `a^` is what the chart renders when nothing is declared.
        let expr = r#"pg_up{job="postgres-exporter", target=~"(?i)a^"}"#;
        assert_eq!(recorded_targets(&[expr]), BTreeSet::from(["a^".to_owned()]));
        assert!(!recorded_targets(&[expr]).contains("consensus"));
    }
}
