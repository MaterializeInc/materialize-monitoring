// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `alerting` and `alert_rules`: the module's alerting inputs land.
//!
//! The expectation is read from what the example *declared* — Terraform's own
//! attribute names, `for_duration` and `excluded_namespaces` — and checked
//! against the render, so the module's translation into chart keys is what gets
//! tested. Every one of these fails quietly when it is wrong:
//!
//!   * a receiver written to a path the chart does not read renders an
//!     Alertmanager with no receiver, which routes every alert to nobody;
//!   * a `preset` that does not land leaves the chart's default, so `critical`
//!     reaches a chat channel where a pager was asked for;
//!   * a receiver Secret in the wrong namespace, or under keys the receivers do
//!     not read, is an optional mount with no file behind it, and Alertmanager
//!     fails the notification at the moment it is needed;
//!   * an override written to the wrong key leaves the rule's own `for`, so a
//!     cluster that hydrates for hours raises `cluster-hydration-stuck` every
//!     time.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{Map, Value};

use super::{report, shown, shown_json};
use crate::terraform_render::ctx::{Ctx, parse_yaml, str_at};
use crate::terraform_render::manifests::Manifest;

const HINT: &str = "See terraform/modules/materialize-monitoring/alerting.tf.";

/// Where the chart mounts the `alertmanager-receivers` Secret.
const RECEIVER_MOUNT: &str = "/etc/alertmanager/secrets/alertmanager-receivers";

/// The Secret holding Alertmanager's rendered configuration, when Alertmanager
/// renders.
pub fn config_secret(ctx: &Ctx) -> Option<&Manifest> {
    ctx.rendered.find("Secret", "alertmanager-config")
}

/// The planned `alertmanager-receivers` Secret, when the module creates one.
pub fn planned_receiver_secret(ctx: &Ctx) -> Option<&Value> {
    ctx.planned("kubernetes_secret", "alertmanager_receivers")
}

/// Whether the bundled rules are on, which they are unless declared off.
pub fn rules_on(ctx: &Ctx) -> bool {
    ctx.declared_at("alert_rules", "enabled") != Some(&Value::Bool(false))
}

/// Every declared receiver rendered, with every integration it declared.
pub fn receivers(ctx: &Ctx) -> Result<()> {
    let config = alertmanager_config(ctx)?;
    let rendered: BTreeMap<&str, &Value> = array(&config, "receivers")
        .filter_map(|receiver| Some((str_at(receiver, "name")?, receiver)))
        .collect();

    let mut problems = Vec::new();
    for (name, receiver) in declared_receivers(ctx) {
        let Some(got) = rendered.get(name.as_str()) else {
            problems.push(format!(
                "receiver {name:?} is not in the rendered Alertmanager configuration"
            ));
            continue;
        };
        let integrations = receiver
            .get("config")
            .and_then(Value::as_object)
            .into_iter()
            .flatten();
        for (integration, _) in integrations {
            if got.get(integration).is_none() {
                problems.push(format!(
                    "receiver {name:?} rendered without its {integration}"
                ));
            }
        }
    }
    report(problems, HINT)
}

/// The preset, checked by its effect: `critical` reaches whoever serves its
/// class.
///
/// That is the only thing about a preset that matters. Left at the chart's
/// default, the class differs, and so, in any deployment with more than one
/// receiver, does who gets woken.
pub fn preset(ctx: &Ctx) -> Result<()> {
    let preset = ctx
        .declared_at("alerting", "preset")
        .and_then(Value::as_str)
        .context("alerting.preset is not a string")?;

    let mut presets = ctx
        .chart_default("alerting.presets")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(declared) = ctx
        .declared_at("alerting", "presets")
        .and_then(Value::as_object)
    {
        presets.extend(declared.clone());
    }
    let target = presets.get(preset).and_then(|p| str_at(p, "critical"));
    let Some(target) = target.filter(|target| *target != "suppressed") else {
        return Ok(());
    };

    let receivers = declared_receivers(ctx);
    let serving: BTreeSet<&str> = receivers
        .iter()
        .filter(|(_, receiver)| classes(receiver).contains(target))
        .map(|(name, _)| name.as_str())
        .collect();
    // No declared receiver serves the class, so there is no one to follow the
    // alert to; the chart's own validator is what speaks to that.
    if serving.is_empty() {
        return Ok(());
    }

    let config = alertmanager_config(ctx)?;
    let reached = receivers_for_severity(config.get("route").unwrap_or(&Value::Null), "critical");
    if reached.iter().all(|r| !serving.contains(r.as_str())) {
        return report(
            vec![format!(
                "preset {preset} routes critical to class {target:?}, served by {serving:?}, but \
                 critical alerts reach {reached:?}"
            )],
            HINT,
        );
    }
    Ok(())
}

/// Every declared extra route sits at the top of the route tree.
pub fn extra_routes(ctx: &Ctx) -> Result<()> {
    let config = alertmanager_config(ctx)?;
    let top: Vec<&Value> = config
        .get("route")
        .map(|route| array(route, "routes").collect())
        .unwrap_or_default();

    let mut problems = Vec::new();
    let extra = ctx
        .declared_at("alerting", "routes.extra")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for route in extra {
        let found = top
            .iter()
            .any(|r| r.get("receiver") == route.get("receiver") && matchers(r) == matchers(route));
        if !found {
            problems.push(format!(
                "extra route to {} on {} is not in the route tree",
                route.get("receiver").unwrap_or(&Value::Null),
                route.get("matchers").unwrap_or(&Value::Null),
            ));
        }
    }
    report(problems, HINT)
}

/// The receiver Secret is read, lives beside Alertmanager, and never reached
/// the release.
pub fn receiver_secret(ctx: &Ctx) -> Result<()> {
    let secret =
        planned_receiver_secret(ctx).context("no planned alertmanager_receivers Secret")?;
    let namespace = secret
        .pointer("/metadata/0/namespace")
        .and_then(Value::as_str);
    let empty = Map::new();
    let data = secret
        .get("data")
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    let config = alertmanager_config(ctx)?.to_string();
    let am_namespace = ctx
        .rendered
        .find("StatefulSet", "alertmanager")
        .and_then(Manifest::namespace);

    let mut problems = Vec::new();
    let unread: Vec<&str> = data
        .keys()
        .filter(|key| !config.contains(&format!("{RECEIVER_MOUNT}/{key}")))
        .map(String::as_str)
        .collect();
    if !unread.is_empty() {
        problems.push(format!(
            "the receiver Secret sets {unread:?}, which no receiver reads"
        ));
    }
    if let Some(am_namespace) = am_namespace
        && namespace != Some(am_namespace)
    {
        problems.push(format!(
            "the receiver Secret is in {} but Alertmanager runs in {am_namespace:?}; \
             the optional mount finds nothing (set alertmanager_namespace)",
            shown(namespace)
        ));
    }
    // The reason these are not values at all: anything in `values` is readable
    // with `helm get values`. A credential in the render got there through them.
    let leaked: Vec<&str> = data
        .iter()
        .filter(|(_, value)| {
            value
                .as_str()
                .is_some_and(|value| !value.is_empty() && ctx.rendered.text().contains(value))
        })
        .map(|(key, _)| key.as_str())
        .collect();
    if !leaked.is_empty() {
        problems.push(format!(
            "receiver credentials reached the Helm release: {leaked:?}"
        ));
    }
    report(problems, HINT)
}

/// With the rules declared off, none rendered.
pub fn rules_disabled(ctx: &Ctx) -> Result<()> {
    let rules = rules_by_alert(ctx);
    if !rules.is_empty() {
        return report(
            vec![format!(
                "alert_rules.enabled is false but {} rule(s) rendered",
                rules.len()
            )],
            HINT,
        );
    }
    Ok(())
}

/// Selected alerts installed and disabled ones did not.
///
/// `selected` takes alert names (kebab-case), group names (snake_case) and
/// `*`; only alert names name one rule to look for. A selected alert that needs
/// a missing capability is the chart's to warn about, and the examples select
/// only rules that apply.
pub fn rule_selection(ctx: &Ctx) -> Result<()> {
    let rules = rules_by_alert(ctx);
    let mut problems = Vec::new();
    for name in declared_strings(ctx, "selected") {
        if name.contains('-') && !rules.contains_key(name) {
            problems.push(format!("selected alert {name:?} did not install"));
        }
    }
    for name in declared_strings(ctx, "disabled") {
        if rules.contains_key(name) {
            problems.push(format!("disabled alert {name:?} still installed"));
        }
    }
    report(problems, HINT)
}

/// Each override's `for_duration` and labels reached its rule.
pub fn rule_overrides(ctx: &Ctx) -> Result<()> {
    let rules = rules_by_alert(ctx);
    let overrides = ctx
        .declared_at("alert_rules", "overrides")
        .and_then(Value::as_object)
        .into_iter()
        .flatten();

    let mut problems = Vec::new();
    for (name, wanted) in overrides {
        // Not installed: the chart warns about that itself.
        let Some(rule) = rules.get(name.as_str()) else {
            continue;
        };
        if let Some(duration) = str_at(wanted, "for_duration")
            && str_at(rule, "for") != Some(duration)
        {
            problems.push(format!(
                "{name}: for is {}, overridden to {duration:?}",
                shown(str_at(rule, "for"))
            ));
        }
        let labels = wanted
            .get("labels")
            .and_then(Value::as_object)
            .into_iter()
            .flatten();
        for (label, value) in labels {
            let got = rule.get("labels").and_then(|labels| labels.get(label));
            if got != Some(value) {
                problems.push(format!(
                    "{name}: label {label} is {}, overridden to {value}",
                    shown_json(got)
                ));
            }
        }
    }
    report(problems, HINT)
}

/// Namespaces and workload tiers reached the rule expressions.
///
/// A tier only lands where an installed rule reads it, so an example replacing a
/// tier also selects a rule that reads that tier.
pub fn rule_scopes(ctx: &Ctx) -> Result<()> {
    let exprs = rules_by_alert(ctx)
        .values()
        .filter_map(|rule| str_at(rule, "expr"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut problems = Vec::new();
    for (namespace, operator, what) in declared_strings(ctx, "excluded_namespaces")
        .map(|ns| (ns, "!~", "exclusion"))
        .chain(declared_strings(ctx, "environment_namespaces").map(|ns| (ns, "=~", "scope")))
    {
        let pattern = format!(
            r#"namespace{operator}"[^"]*\b{}\b"#,
            regex::escape(namespace)
        );
        let matcher = Regex::new(&pattern).context("building a namespace matcher")?;
        if !matcher.is_match(&exprs) {
            let kind = if operator == "!~" {
                "excluded"
            } else {
                "environment"
            };
            problems.push(format!(
                "{kind} namespace {namespace:?} reaches no rule's {what}"
            ));
        }
    }
    let tiers = ctx
        .declared_at("alert_rules", "infra_workloads")
        .and_then(Value::as_object)
        .into_iter()
        .flatten();
    for (tier, names) in tiers {
        for entry in names
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !exprs.contains(entry) {
                problems.push(format!(
                    "infra_workloads.{tier} entry {entry:?} reaches no rule"
                ));
            }
        }
    }
    report(problems, HINT)
}

/// The rendered `alertmanager.yml`, or an empty configuration when none
/// rendered, which every receiver and route check then reports against.
fn alertmanager_config(ctx: &Ctx) -> Result<Value> {
    match config_secret(ctx).and_then(|secret| secret.entry("alertmanager.yml")) {
        Some(text) => parse_yaml(&text).context("parsing the rendered alertmanager.yml"),
        None => Ok(Value::Object(Map::new())),
    }
}

/// Every rendered alerting rule, keyed by alert name.
fn rules_by_alert(ctx: &Ctx) -> BTreeMap<&str, &Value> {
    ctx.rendered
        .of_kind("PrometheusRule")
        .filter_map(|doc| doc.get("spec.groups")?.as_array())
        .flatten()
        .flat_map(|group| array(group, "rules"))
        .filter_map(|rule| Some((str_at(rule, "alert")?, rule)))
        .collect()
}

fn declared_receivers(ctx: &Ctx) -> Vec<(String, &Value)> {
    ctx.declared_at("alerting", "receivers")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(name, receiver)| (name.clone(), receiver))
        .collect()
}

fn declared_strings<'a>(ctx: &'a Ctx, key: &str) -> impl Iterator<Item = &'a str> {
    ctx.declared_at("alert_rules", key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// The classes a declared receiver serves, which Terraform takes as a string or
/// a list.
fn classes(receiver: &Value) -> BTreeSet<&str> {
    match receiver.get("class") {
        Some(Value::String(class)) => BTreeSet::from([class.as_str()]),
        Some(Value::Array(classes)) => classes.iter().filter_map(Value::as_str).collect(),
        _ => BTreeSet::new(),
    }
}

/// The receivers an alert of `severity` reaches through the severity routes.
fn receivers_for_severity(root: &Value, severity: &str) -> BTreeSet<String> {
    let want = format!("severity=\"{severity}\"");
    let mut out = BTreeSet::new();
    walk_routes(root, &mut Vec::new(), &mut |matchers, route| {
        if matchers.contains(&want.as_str())
            && let Some(receiver) = str_at(route, "receiver").filter(|r| !r.is_empty())
        {
            out.insert(receiver.to_owned());
        }
    });
    out
}

/// Visit every route in the tree, with the matchers of every route above it.
fn walk_routes<'a>(
    route: &'a Value,
    path: &mut Vec<&'a str>,
    visit: &mut impl FnMut(&[&'a str], &'a Value),
) {
    let depth = path.len();
    path.extend(array(route, "matchers").filter_map(Value::as_str));
    visit(path, route);
    for child in array(route, "routes") {
        walk_routes(child, path, visit);
    }
    path.truncate(depth);
}

fn matchers(route: &Value) -> Vec<&Value> {
    array(route, "matchers").collect()
}

fn array<'a>(value: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A severity matcher on a parent route reaches the receivers below it.
    #[test]
    fn severity_routes_inherit_their_parents_matchers() {
        let route = json!({
            "receiver": "default",
            "routes": [
                { "matchers": ["severity=\"critical\""], "routes": [
                    { "matchers": ["audience=\"platform\""], "receiver": "oncall" },
                ]},
                { "matchers": ["severity=\"warning\""], "receiver": "chat" },
            ],
        });
        assert_eq!(
            receivers_for_severity(&route, "critical"),
            BTreeSet::from(["oncall".to_owned()])
        );
    }

    #[test]
    fn a_class_is_a_string_or_a_list() {
        assert_eq!(
            classes(&json!({ "class": "page" })),
            BTreeSet::from(["page"])
        );
        assert_eq!(
            classes(&json!({ "class": ["high", "low"] })),
            BTreeSet::from(["high", "low"])
        );
    }
}
