// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The label vocabulary, `packages/prometheus-scrapers/labels.yaml`.
//!
//! A Materialize metric often carries an id or a name under a label that the
//! [label vocabulary] spells differently: a cluster id as `instance_id`, a
//! cluster's name as `name` on `mz_cluster_info`. The vocabulary lists each
//! canonical label and the aliases copied into it. `gen-scrape-configs` renders
//! every alias as a fill-if-empty `metricRelabelings` rule, through
//! [`LabelVocabulary::add_to_monitor`], on every endpoint of every monitor the
//! vocabulary applies to. A rule copies only where the canonical label is
//! empty, so a value Materialize emits under the canonical name is never
//! overwritten.
//!
//! [label vocabulary]: https://materializeinc.github.io/materialize-monitoring/reference/development/design-docs/20261002-metric-label-vocabulary/

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use jsonschema::Validator;
use serde::Deserialize;
use serde_json::Value;

use crate::scrape::error::{Error, Result};
use crate::scrape::operator::common::RelabelConfig;

const SCHEMA: &str = include_str!("../../schemas/scrape/mzmon-labels.schema.yaml");

static VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let schema: Value =
        crate::yaml::from_str(SCHEMA).expect("embedded label vocabulary schema is valid YAML");
    jsonschema::options()
        .build(&schema)
        .expect("embedded label vocabulary schema compiles into a validator")
});

/// The values a canonical label copies when it sets no `values`: any non-empty
/// value.
const ANY_VALUE: &str = ".+";

/// The parsed `labels.yaml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelVocabulary {
    /// The `app.kubernetes.io/name` of every monitor the rules render into.
    pub monitors: Vec<String>,
    /// The canonical labels, in rule order.
    pub canonical: Vec<CanonicalLabel>,
}

/// One canonical label and the aliases copied into it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalLabel {
    pub name: String,
    /// A regex every copied value must match. `None` copies any non-empty value.
    #[serde(default)]
    pub values: Option<String>,
    pub aliases: Vec<AliasDefinition>,
}

/// One alias as `labels.yaml` writes it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasDefinition {
    pub label: String,
    /// The only families the alias is copied on. Empty means every family.
    #[serde(default)]
    pub families: Vec<String>,
    /// The materialize-monitoring release whose notes deprecate the alias.
    pub deprecated_in: String,
}

/// One alias, resolved against its canonical label: the unit a rule renders
/// from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelAlias<'a> {
    pub canonical: &'a str,
    pub alias: &'a str,
    pub families: &'a [String],
    pub values: &'a str,
    pub deprecated_in: &'a str,
}

impl LabelVocabulary {
    /// Parse `labels.yaml`, validate it against its schema, then check what a
    /// schema cannot: that no two rules write the same alias on the same family,
    /// that no canonical label is also an alias, and that every `values` regex
    /// compiles.
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let value: Value = crate::yaml::from_str(yaml)?;
        let errors: Vec<Error> = VALIDATOR
            .iter_errors(&value)
            .map(|err| Error::Schema {
                path: err.instance_path().to_string(),
                message: err.to_string(),
            })
            .collect();
        if !errors.is_empty() {
            return Err(Error::Multiple(errors));
        }
        let vocabulary: Self = serde_json::from_value(value)?;
        vocabulary.check()?;
        Ok(vocabulary)
    }

    fn check(&self) -> Result<()> {
        let mut errors = Vec::new();
        let canonical: BTreeSet<&str> = self.canonical.iter().map(|c| c.name.as_str()).collect();
        if canonical.len() != self.canonical.len() {
            errors.push(Error::Vocabulary(
                "a canonical label is listed twice".into(),
            ));
        }

        // Each alias label, mapped to the families it is copied on. An empty
        // family list means every family, so it collides with any other entry.
        let mut seen: BTreeMap<&str, Vec<&[String]>> = BTreeMap::new();
        for alias in self.aliases() {
            if canonical.contains(alias.alias) {
                errors.push(Error::Vocabulary(format!(
                    "`{}` is a canonical label and an alias of `{}`",
                    alias.alias, alias.canonical
                )));
            }
            let earlier = seen.entry(alias.alias).or_default();
            let overlaps = earlier.iter().any(|families| {
                families.is_empty()
                    || alias.families.is_empty()
                    || families.iter().any(|f| alias.families.contains(f))
            });
            if overlaps {
                errors.push(Error::Vocabulary(format!(
                    "`{}` is an alias twice on the same family",
                    alias.alias
                )));
            }
            earlier.push(alias.families);
        }

        for canonical in &self.canonical {
            if let Some(values) = &canonical.values
                && let Err(err) = regex::Regex::new(&format!("^(?:{values})$"))
            {
                errors.push(Error::Vocabulary(format!(
                    "`{}` values `{values}` is not a regex: {err}",
                    canonical.name
                )));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Multiple(errors))
        }
    }

    /// Every alias, in rule order.
    pub fn aliases(&self) -> impl Iterator<Item = LabelAlias<'_>> {
        self.canonical.iter().flat_map(|canonical| {
            canonical.aliases.iter().map(move |alias| LabelAlias {
                canonical: &canonical.name,
                alias: &alias.label,
                families: &alias.families,
                values: canonical.values.as_deref().unwrap_or(ANY_VALUE),
                deprecated_in: &alias.deprecated_in,
            })
        })
    }

    /// The rules every endpoint of a matching monitor carries, in order.
    pub fn metric_relabelings(&self) -> Vec<RelabelConfig> {
        self.aliases().map(|alias| alias.relabel_config()).collect()
    }

    /// Whether the vocabulary applies to a monitor, by its
    /// `app.kubernetes.io/name` label.
    pub fn applies_to(&self, monitor: &Value) -> bool {
        monitor
            .pointer("/metadata/labels/app.kubernetes.io~1name")
            .and_then(Value::as_str)
            .is_some_and(|name| self.monitors.iter().any(|m| m == name))
    }

    /// Append the rules to every endpoint's `metricRelabelings`, after any the
    /// monitor already sets. Returns whether the vocabulary applies to the
    /// monitor; one it does not apply to is left untouched.
    ///
    /// Only PodMonitors and ServiceMonitors have endpoints to add them to. A
    /// matching monitor of another kind is an error, because its series would
    /// silently lack the canonical labels.
    pub fn add_to_monitor(&self, monitor: &mut Value) -> Result<bool> {
        if !self.applies_to(monitor) {
            return Ok(false);
        }
        let endpoints = match monitor.get("kind").and_then(Value::as_str) {
            Some("PodMonitor") => "/spec/podMetricsEndpoints",
            Some("ServiceMonitor") => "/spec/endpoints",
            other => {
                return Err(Error::Unsupported(format!(
                    "the label vocabulary applies to a {other:?}, which has no endpoints to carry its rules"
                )));
            }
        };
        let rules = serde_json::to_value(self.metric_relabelings())?;
        let Some(Value::Array(endpoints)) = monitor.pointer_mut(endpoints) else {
            return Ok(true);
        };
        for endpoint in endpoints {
            let Value::Object(endpoint) = endpoint else {
                continue;
            };
            let existing = endpoint
                .entry("metricRelabelings")
                .or_insert_with(|| Value::Array(Vec::new()));
            if let (Value::Array(existing), Value::Array(rules)) = (existing, &rules) {
                existing.extend(rules.iter().cloned());
            }
        }
        Ok(true)
    }
}

impl LabelAlias<'_> {
    /// The fill-if-empty rule, in prometheus-operator form.
    ///
    /// The canonical label is the first source label, so the joined value
    /// starts with the separator only when it is empty. A family-scoped rule
    /// adds `__name__` in front.
    pub fn relabel_config(&self) -> RelabelConfig {
        let (source_labels, regex) = match self.families {
            [] => (
                vec![self.canonical, self.alias],
                format!(";({})", self.values),
            ),
            [family] => (
                vec!["__name__", self.canonical, self.alias],
                format!("{family};;({})", self.values),
            ),
            families => (
                vec!["__name__", self.canonical, self.alias],
                format!("(?:{});;({})", families.join("|"), self.values),
            ),
        };
        RelabelConfig {
            source_labels: source_labels.into_iter().map(String::from).collect(),
            separator: Some(";".into()),
            target_label: Some(self.canonical.into()),
            regex: Some(regex),
            modulus: None,
            replacement: Some("$1".into()),
            action: Some("replace".into()),
        }
    }

    /// A PromQL selector for series that carry the alias, with a value its rule
    /// copies, and lack the canonical label. On an install running the shipped
    /// monitors it matches nothing.
    ///
    /// An unscoped alias is matched on Materialize's own families only, because
    /// other exporters use some of the same label names.
    pub fn missing_canonical_selector(&self) -> String {
        let name = match self.families {
            [] => "__name__=~\"(v2_)?mz_.+\"".to_string(),
            [family] => format!("__name__=\"{family}\""),
            families => format!("__name__=~\"{}\"", families.join("|")),
        };
        format!(
            "{{{name}, {alias}=~\"{values}\", {canonical}=\"\"}}",
            alias = self.alias,
            values = self.values,
            canonical = self.canonical,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use regex::Regex;

    use super::*;
    use crate::scrape::test_support::fixture;
    use crate::scrape::transpile::Monitor;

    const SHIPPED: &str = include_str!("../../../prometheus-scrapers/labels.yaml");

    /// The monitor sources the shipped vocabulary applies to.
    const MATERIALIZE_MONITORS: &[&str] = &[
        "podmonitor-environmentd",
        "podmonitor-clusterd",
        "podmonitor-sql",
        "podmonitor-materialize-operator",
    ];

    fn shipped() -> LabelVocabulary {
        LabelVocabulary::from_yaml_str(SHIPPED).expect("labels.yaml loads")
    }

    fn with_rules(stem: &str) -> Value {
        let mut monitor: Value = crate::yaml::from_str(fixture(stem)).unwrap();
        assert!(
            shipped().add_to_monitor(&mut monitor).unwrap(),
            "{stem} is a Materialize monitor"
        );
        monitor
    }

    /// Prometheus's `replace` action: join the source labels, match the regex
    /// anchored, and write the expanded replacement to the target label. An
    /// empty result removes the target label.
    fn apply(
        rules: &[RelabelConfig],
        labels: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut labels = labels.clone();
        for rule in rules {
            assert_eq!(rule.action.as_deref(), Some("replace"), "{rule:?}");
            let separator = rule.separator.as_deref().unwrap_or(";");
            let joined = rule
                .source_labels
                .iter()
                .map(|label| labels.get(label).map_or("", String::as_str))
                .collect::<Vec<_>>()
                .join(separator);
            let regex = Regex::new(&format!(
                "^(?:{})$",
                rule.regex.as_deref().unwrap_or("(.*)")
            ))
            .unwrap();
            let Some(captures) = regex.captures(&joined) else {
                continue;
            };
            let mut value = String::new();
            captures.expand(rule.replacement.as_deref().unwrap_or("$1"), &mut value);
            let target = rule
                .target_label
                .clone()
                .expect("replace needs a target label");
            if value.is_empty() {
                labels.remove(&target);
            } else {
                labels.insert(target, value);
            }
        }
        labels
    }

    fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// A value the alias's rule copies.
    fn sample(alias: &LabelAlias) -> &'static str {
        if alias.values == ANY_VALUE {
            "sample"
        } else {
            "u1"
        }
    }

    fn family<'a>(alias: &LabelAlias<'a>) -> &'a str {
        alias
            .families
            .first()
            .map_or("mz_example_metric", String::as_str)
    }

    fn rejects(yaml: &str, needle: &str) {
        let err = LabelVocabulary::from_yaml_str(yaml)
            .unwrap_err()
            .to_string();
        assert!(err.contains(needle), "expected {needle:?} in: {err}");
    }

    #[test]
    fn shipped_vocabulary_loads() {
        let vocabulary = shipped();
        assert!(vocabulary.aliases().count() > 0);
        assert_eq!(
            vocabulary.monitors,
            ["environmentd", "clusterd", "materialize-operator"]
        );
    }

    /// The CNI, kubelet and kube-proxy monitors are part of this stack too, but
    /// not Materialize's, so the vocabulary skips them.
    #[test]
    fn only_materialize_monitors_match() {
        let vocabulary = shipped();
        for stem in ["podmonitor-cni-aws-vpc", "scrapeconfig-kube-proxy"] {
            let path = format!(
                "{}/../prometheus-scrapers/{stem}.yaml",
                env!("CARGO_MANIFEST_DIR")
            );
            let monitor: Value =
                crate::yaml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(!vocabulary.applies_to(&monitor), "{stem}");
        }
    }

    #[test]
    fn rejects_an_unknown_field() {
        rejects(
            "monitors: [m]\ncanonical:\n  - name: x\n    meaning: y\n    aliases: [{label: z, deprecatedIn: 1.0.0}]\n",
            "meaning",
        );
    }

    #[test]
    fn rejects_an_alias_twice_on_one_family() {
        rejects(
            "monitors: [m]\ncanonical:\n  - name: x\n    aliases: [{label: z, deprecatedIn: 1.0.0}]\n  - name: w\n    aliases: [{label: z, families: [mz_a], deprecatedIn: 1.0.0}]\n",
            "alias twice",
        );
        // The same label on two different families is two aliases, not one.
        LabelVocabulary::from_yaml_str(
            "monitors: [m]\ncanonical:\n  - name: x\n    aliases: [{label: z, families: [mz_a], deprecatedIn: 1.0.0}]\n  - name: w\n    aliases: [{label: z, families: [mz_b], deprecatedIn: 1.0.0}]\n",
        )
        .unwrap();
    }

    #[test]
    fn rejects_a_canonical_label_used_as_an_alias() {
        rejects(
            "monitors: [m]\ncanonical:\n  - name: x\n    aliases: [{label: w, deprecatedIn: 1.0.0}]\n  - name: w\n    aliases: [{label: z, deprecatedIn: 1.0.0}]\n",
            "canonical label and an alias",
        );
    }

    #[test]
    fn rejects_values_that_are_not_a_regex() {
        rejects(
            "monitors: [m]\ncanonical:\n  - name: x\n    values: \"[su\"\n    aliases: [{label: z, deprecatedIn: 1.0.0}]\n",
            "not a regex",
        );
    }

    #[test]
    fn rules_take_three_forms() {
        let unscoped = LabelAlias {
            canonical: "cluster_id",
            alias: "instance_id",
            families: &[],
            values: "[su][0-9]+",
            deprecated_in: "1.6.0",
        };
        let one = ["mz_cluster_info".to_string()];
        let two = ["mz_a".to_string(), "mz_b".to_string()];
        let rule = |alias: LabelAlias| {
            let rule = alias.relabel_config();
            (rule.source_labels.join(","), rule.regex.unwrap())
        };
        assert_eq!(
            rule(unscoped),
            ("cluster_id,instance_id".into(), ";([su][0-9]+)".into())
        );
        assert_eq!(
            rule(LabelAlias {
                families: &one,
                values: ".+",
                ..unscoped
            }),
            (
                "__name__,cluster_id,instance_id".into(),
                "mz_cluster_info;;(.+)".into()
            )
        );
        assert_eq!(
            rule(LabelAlias {
                families: &two,
                values: ".+",
                ..unscoped
            }),
            (
                "__name__,cluster_id,instance_id".into(),
                "(?:mz_a|mz_b);;(.+)".into()
            )
        );
    }

    #[test]
    fn every_endpoint_of_every_materialize_monitor_carries_the_rules() {
        let expected = serde_json::to_value(shipped().metric_relabelings()).unwrap();
        for stem in MATERIALIZE_MONITORS {
            let monitor = with_rules(stem);
            let endpoints = monitor["spec"]["podMetricsEndpoints"].as_array().unwrap();
            assert!(!endpoints.is_empty(), "{stem}");
            for (i, endpoint) in endpoints.iter().enumerate() {
                assert_eq!(
                    endpoint["metricRelabelings"], expected,
                    "{stem} endpoint {i}"
                );
            }
        }
    }

    #[test]
    fn other_monitors_are_left_alone() {
        let source: Value = crate::yaml::from_str(fixture("scrapeconfig-cadvisor")).unwrap();
        let mut monitor = source.clone();
        assert!(!shipped().add_to_monitor(&mut monitor).unwrap());
        assert_eq!(monitor, source);
    }

    #[test]
    fn classic_and_gmp_renders_carry_the_rules() {
        let operator = shipped().metric_relabelings();
        let classic: Vec<Value> = operator
            .iter()
            .map(|rule| serde_json::to_value(rule.to_classic_relabel_config()).unwrap())
            .collect();
        let operator = serde_json::to_value(&operator).unwrap();
        for stem in MATERIALIZE_MONITORS {
            let monitor = Monitor::from_value(with_rules(stem)).unwrap();
            for job in monitor.transpile().unwrap() {
                let rendered: Vec<Value> = job
                    .metric_relabel_configs
                    .iter()
                    .map(|rule| serde_json::to_value(rule).unwrap())
                    .collect();
                assert_eq!(rendered, classic, "{stem} classic job {}", job.job_name);
            }
            for resource in monitor.to_gmp().unwrap() {
                for endpoint in &resource.spec.endpoints {
                    assert_eq!(
                        serde_json::to_value(&endpoint.metric_relabeling).unwrap(),
                        operator,
                        "{stem} GMP resource {:?}",
                        resource.metadata.name,
                    );
                }
            }
        }
    }

    /// The four label sets from DEP-421's acceptance, against the whole rule
    /// list as Prometheus applies it: alias only, canonical only, both equal,
    /// and neither. A canonical label that disagrees with its alias is never
    /// overwritten either.
    #[test]
    fn aliases_fill_only_an_empty_canonical_label() {
        let vocabulary = shipped();
        let rules = vocabulary.metric_relabelings();
        for alias in vocabulary.aliases() {
            let name = family(&alias);
            let value = sample(&alias);
            let context = format!("{} -> {}", alias.alias, alias.canonical);

            let alias_only = labels(&[("__name__", name), (alias.alias, value)]);
            let mut filled = alias_only.clone();
            filled.insert(alias.canonical.into(), value.into());
            assert_eq!(apply(&rules, &alias_only), filled, "alias only, {context}");

            for unchanged in [
                labels(&[("__name__", name), (alias.canonical, value)]),
                labels(&[
                    ("__name__", name),
                    (alias.canonical, value),
                    (alias.alias, value),
                ]),
                labels(&[
                    ("__name__", name),
                    (alias.canonical, "u2"),
                    (alias.alias, value),
                ]),
                labels(&[("__name__", name)]),
            ] {
                assert_eq!(
                    apply(&rules, &unchanged),
                    unchanged,
                    "{unchanged:?}, {context}"
                );
            }
        }
    }

    #[test]
    fn scoped_aliases_leave_other_families_alone() {
        let vocabulary = shipped();
        let rules = vocabulary.metric_relabelings();
        for alias in vocabulary.aliases().filter(|a| !a.families.is_empty()) {
            for other in ["mz_persist_shard_blob_gets", "mz_compute_cluster_status"] {
                let series = labels(&[("__name__", other), (alias.alias, "sample")]);
                assert_eq!(apply(&rules, &series), series, "{alias:?} on {other}");
            }
        }
    }

    #[test]
    fn id_aliases_copy_only_ids() {
        let vocabulary = shipped();
        let rules = vocabulary.metric_relabelings();
        let ids: Vec<_> = vocabulary
            .aliases()
            .filter(|a| a.values != ANY_VALUE)
            .collect();
        assert!(!ids.is_empty());
        for alias in ids {
            let series = labels(&[
                ("__name__", "mz_time_to_first_row_seconds_count"),
                (alias.alias, "none"),
            ]);
            assert_eq!(apply(&rules, &series), series, "{alias:?}");
        }
    }

    #[test]
    fn missing_canonical_selectors() {
        let vocabulary = shipped();
        let instance = vocabulary
            .aliases()
            .find(|a| a.alias == "instance_id")
            .unwrap();
        assert_eq!(
            instance.missing_canonical_selector(),
            r#"{__name__=~"(v2_)?mz_.+", instance_id=~"[su][0-9]+", cluster_id=""}"#,
        );
        let name = vocabulary
            .aliases()
            .find(|a| a.canonical == "cluster_name" && a.alias == "name")
            .unwrap();
        assert_eq!(
            name.missing_canonical_selector(),
            r#"{__name__="mz_cluster_info", name=~".+", cluster_name=""}"#,
        );
    }
}
