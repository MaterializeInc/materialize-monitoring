// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The label aliases the shipped Materialize monitors fill (DEP-421).
//!
//! A Materialize metric often carries an id or a name under a label that the
//! [label vocabulary] spells differently: a cluster id as `instance_id`, a
//! cluster's name as `name` on `mz_cluster_info`. Each monitor's
//! `metricRelabelings` copy those aliases into their canonical names, and only
//! where the canonical label is empty, so a value Materialize emits under the
//! canonical name is never overwritten.
//!
//! The rules are written by hand into the monitor sources under
//! `packages/prometheus-scrapers/`. [`LABEL_ALIASES`] is what they are checked
//! against: the tests below assert that every endpoint carries exactly its
//! monitor's rules, in every flavor, and that the rules behave as fill-if-empty
//! copies. The E2E suite checks a live install for series that carry an alias
//! without its canonical name. `labels.yaml` (DEP-349) replaces both the table
//! and the hand-written rules.
//!
//! [label vocabulary]: https://materializeinc.github.io/materialize-monitoring/reference/development/design-docs/20261002-metric-label-vocabulary/

use crate::scrape::operator::common::RelabelConfig;

/// The values every rule copies: any non-empty value.
const ANY_VALUE: &str = ".+";

/// A Materialize cluster or replica id, as `ClusterId` and `ReplicaId` print.
/// `instance_id` is `"none"` on `mz_time_to_first_row_seconds`, which is not a
/// cluster.
const MZ_ID: &str = "[su][0-9]+";

/// The monitor sources, by file stem.
pub const ENVIRONMENTD: &str = "podmonitor-environmentd";
pub const CLUSTERD: &str = "podmonitor-clusterd";
pub const SQL_EXPORTER: &str = "podmonitor-sql";

/// One emitted label, copied into its canonical name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelAlias {
    /// The canonical label the rule fills.
    pub canonical: &'static str,
    /// The emitted label it copies.
    pub alias: &'static str,
    /// The one family the rule copies on, for an alias that means something
    /// else on other families. `name` is a persist shard's name on
    /// `mz_persist_shard_*`, for example.
    pub family: Option<&'static str>,
    /// The values the rule copies, as a regex. Relabel regexes are anchored.
    pub values: &'static str,
    /// The monitor sources whose endpoints carry the rule: those whose process
    /// emits the alias.
    pub monitors: &'static [&'static str],
}

/// Every alias, in the order each monitor's rules list them.
pub const LABEL_ALIASES: &[LabelAlias] = &[
    alias(
        "cluster_id",
        "instance_id",
        MZ_ID,
        &[ENVIRONMENTD, SQL_EXPORTER],
    ),
    alias("cluster_id", "compute_instance", MZ_ID, &[ENVIRONMENTD]),
    alias("cluster_id", "compute_cluster_id", MZ_ID, &[SQL_EXPORTER]),
    alias(
        "cluster_id",
        "cluster_environmentd_materialize_cloud_cluster_id",
        MZ_ID,
        &[CLUSTERD],
    ),
    alias(
        "replica_id",
        "cluster_environmentd_materialize_cloud_replica_id",
        MZ_ID,
        &[CLUSTERD],
    ),
    alias(
        "cluster_name",
        "compute_cluster_name",
        ANY_VALUE,
        &[SQL_EXPORTER],
    ),
    alias("replica_id", "compute_replica_id", MZ_ID, &[SQL_EXPORTER]),
    alias("collection_id", "global_id", ANY_VALUE, &[ENVIRONMENTD]),
    alias("shard_id", "shard", ANY_VALUE, &[ENVIRONMENTD, CLUSTERD]),
    scoped("cluster_name", "name", "mz_cluster_info"),
    scoped("cluster_size", "size", "mz_cluster_info"),
    scoped("replica_name", "name", "mz_replica_info"),
    scoped("replica_size", "size", "mz_replica_info"),
    scoped("object_name", "name", "mz_object_info"),
    scoped("object_type", "type", "mz_object_info"),
];

const fn alias(
    canonical: &'static str,
    alias: &'static str,
    values: &'static str,
    monitors: &'static [&'static str],
) -> LabelAlias {
    LabelAlias {
        canonical,
        alias,
        family: None,
        values,
        monitors,
    }
}

/// An alias on one of the `_info` families, which only environmentd emits.
const fn scoped(canonical: &'static str, alias: &'static str, family: &'static str) -> LabelAlias {
    LabelAlias {
        canonical,
        alias,
        family: Some(family),
        values: ANY_VALUE,
        monitors: &[ENVIRONMENTD],
    }
}

impl LabelAlias {
    /// The aliases a monitor source's endpoints carry, in rule order.
    pub fn for_monitor(monitor: &str) -> impl Iterator<Item = &'static LabelAlias> {
        LABEL_ALIASES
            .iter()
            .filter(move |alias| alias.monitors.contains(&monitor))
    }

    /// The fill-if-empty rule, in prometheus-operator form.
    ///
    /// The canonical label is the first source label, so the joined value
    /// starts with the separator only when it is empty. A family-scoped rule
    /// adds `__name__` in front.
    pub fn relabel_config(&self) -> RelabelConfig {
        let (source_labels, regex) = match self.family {
            None => (
                vec![self.canonical, self.alias],
                format!(";({})", self.values),
            ),
            Some(family) => (
                vec!["__name__", self.canonical, self.alias],
                format!("{family};;({})", self.values),
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

    /// A PromQL selector for series that carry the alias, with a value the rule
    /// copies, and lack the canonical label. On an install running the shipped
    /// monitors it matches nothing.
    ///
    /// An unscoped alias is matched on Materialize's own families only, because
    /// other exporters use some of the same label names.
    pub fn missing_canonical_selector(&self) -> String {
        let name = match self.family {
            Some(family) => format!("__name__=\"{family}\""),
            None => "__name__=~\"(v2_)?mz_.+\"".to_string(),
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
    use serde_json::Value;

    use super::*;
    use crate::scrape::operator::pod_monitor::PodMonitor;
    use crate::scrape::test_support::fixture;
    use crate::scrape::transpile::Monitor;

    /// Every Materialize monitor source, including the ones that carry no alias.
    const MONITORS: &[&str] = &[
        ENVIRONMENTD,
        CLUSTERD,
        SQL_EXPORTER,
        "podmonitor-materialize-operator",
    ];

    fn pod_monitor(stem: &str) -> PodMonitor {
        match Monitor::from_yaml_str(fixture(stem)).unwrap() {
            Monitor::PodMonitor(pm) => pm,
            other => panic!("{stem} is not a PodMonitor: {other:?}"),
        }
    }

    fn expected_rules(stem: &str) -> Value {
        let rules: Vec<RelabelConfig> = LabelAlias::for_monitor(stem)
            .map(LabelAlias::relabel_config)
            .collect();
        serde_json::to_value(rules).unwrap()
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
        if alias.values == MZ_ID {
            "u1"
        } else {
            "sample"
        }
    }

    fn family(alias: &LabelAlias) -> &'static str {
        alias.family.unwrap_or("mz_example_metric")
    }

    #[test]
    fn every_endpoint_carries_exactly_its_monitors_aliases() {
        for stem in MONITORS {
            let expected = expected_rules(stem);
            for (i, endpoint) in pod_monitor(stem)
                .spec
                .pod_metrics_endpoints
                .iter()
                .enumerate()
            {
                assert_eq!(
                    serde_json::to_value(&endpoint.metric_relabelings).unwrap(),
                    expected,
                    "{stem} endpoint {i}: metricRelabelings differ from LABEL_ALIASES",
                );
            }
        }
    }

    #[test]
    fn every_alias_is_carried_somewhere() {
        for alias in LABEL_ALIASES {
            assert!(!alias.monitors.is_empty(), "{alias:?} names no monitor");
            for monitor in alias.monitors {
                assert!(
                    MONITORS.contains(monitor),
                    "{alias:?} names unknown {monitor}"
                );
            }
        }
    }

    #[test]
    fn classic_and_gmp_renders_carry_the_rules() {
        for stem in MONITORS {
            let expected = expected_rules(stem);
            let monitor = Monitor::from_yaml_str(fixture(stem)).unwrap();
            for job in monitor.transpile().unwrap() {
                let classic: Vec<Value> = job
                    .metric_relabel_configs
                    .iter()
                    .map(|rule| serde_json::to_value(rule).unwrap())
                    .collect();
                let operator: Vec<Value> = expected
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|rule| {
                        let rule: RelabelConfig = serde_json::from_value(rule.clone()).unwrap();
                        serde_json::to_value(rule.to_classic_relabel_config()).unwrap()
                    })
                    .collect();
                assert_eq!(classic, operator, "{stem} classic job {}", job.job_name);
            }
            for resource in monitor.to_gmp().unwrap() {
                for endpoint in &resource.spec.endpoints {
                    assert_eq!(
                        serde_json::to_value(&endpoint.metric_relabeling).unwrap(),
                        expected,
                        "{stem} GMP resource {:?}",
                        resource.metadata.name,
                    );
                }
            }
        }
    }

    /// The four label sets from DEP-421's acceptance, against each monitor's
    /// whole rule list, as Prometheus applies it: alias only, canonical only,
    /// both equal, and neither. A canonical label that disagrees with its alias
    /// is never overwritten either.
    #[test]
    fn aliases_fill_only_an_empty_canonical_label() {
        for alias in LABEL_ALIASES {
            for stem in alias.monitors {
                let rules = &pod_monitor(stem).spec.pod_metrics_endpoints[0].metric_relabelings;
                let name = family(alias);
                let value = sample(alias);
                let context = format!("{stem}: {} -> {}", alias.alias, alias.canonical);

                let alias_only = labels(&[("__name__", name), (alias.alias, value)]);
                let mut filled = alias_only.clone();
                filled.insert(alias.canonical.into(), value.into());
                assert_eq!(apply(rules, &alias_only), filled, "alias only, {context}");

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
                        apply(rules, &unchanged),
                        unchanged,
                        "{unchanged:?}, {context}"
                    );
                }
            }
        }
    }

    #[test]
    fn scoped_aliases_leave_other_families_alone() {
        for alias in LABEL_ALIASES.iter().filter(|a| a.family.is_some()) {
            let rules = &pod_monitor(ENVIRONMENTD).spec.pod_metrics_endpoints[0].metric_relabelings;
            for other in ["mz_persist_shard_blob_gets", "mz_compute_cluster_status"] {
                let series = labels(&[("__name__", other), (alias.alias, "sample")]);
                assert_eq!(apply(rules, &series), series, "{alias:?} on {other}");
            }
        }
    }

    #[test]
    fn id_aliases_copy_only_ids() {
        for alias in LABEL_ALIASES.iter().filter(|a| a.values == MZ_ID) {
            for stem in alias.monitors {
                let rules = &pod_monitor(stem).spec.pod_metrics_endpoints[0].metric_relabelings;
                let series = labels(&[
                    ("__name__", "mz_time_to_first_row_seconds_count"),
                    (alias.alias, "none"),
                ]);
                assert_eq!(apply(rules, &series), series, "{alias:?} on {stem}");
            }
        }
    }

    #[test]
    fn missing_canonical_selectors() {
        let instance = LABEL_ALIASES
            .iter()
            .find(|a| a.alias == "instance_id")
            .unwrap();
        assert_eq!(
            instance.missing_canonical_selector(),
            r#"{__name__=~"(v2_)?mz_.+", instance_id=~"[su][0-9]+", cluster_id=""}"#,
        );
        let name = LABEL_ALIASES
            .iter()
            .find(|a| a.family == Some("mz_cluster_info") && a.alias == "name")
            .unwrap();
        assert_eq!(
            name.missing_canonical_selector(),
            r#"{__name__="mz_cluster_info", name=~".+", cluster_name=""}"#,
        );
    }
}
