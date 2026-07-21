// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Aggregating extracted metrics into the `metrics.yaml` documentation model.
//!
//! Ported from `query_cli.docgen`. Each metric query is rendered and its metrics
//! extracted; occurrences of the same metric across queries are merged (labels
//! and `usage` unioned, stability promoted to the most mature). The result is
//! sorted most-mature-then-most-used-then-name and serialized to YAML.
//!
//! The output is a pure function of the registry content: labels and usage are
//! sorted, stability is a commutative max, and the final order is total — so it
//! does not depend on query registration/iteration order. That is what lets the
//! Rust `extract-metrics` reproduce the Python `docgen` output structurally.

use std::collections::BTreeSet;
use std::str::FromStr;

use indexmap::IndexMap;
use serde::Serialize;

use crate::query::error::{Error, Result};
use crate::query::registry::QueryRegistry;
use crate::query::render::TemplateContext;
use crate::query::stability::Stability;

/// Documentation for a single metric. Field order matches the Python
/// `yaml.safe_dump` output (keys sorted: `labels`, `name`, `stability`,
/// `usage`), so the serialized YAML lines up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetricDoc {
    /// The distinct label names any query matches on this metric, sorted.
    pub labels: Vec<String>,
    /// The metric name.
    pub name: String,
    /// The most mature stability across the queries that use this metric.
    pub stability: String,
    /// The ids of the queries that reference this metric, sorted.
    pub usage: Vec<String>,
}

/// The outcome of a docgen run: the aggregated metrics plus any per-query errors
/// that were skipped (a bad query does not abort the whole run, mirroring the
/// Python `docgen` try/except).
#[derive(Debug, Default)]
pub struct DocgenOutcome {
    pub metrics: Vec<MetricDoc>,
    pub errors: Vec<(String, Error)>,
}

impl DocgenOutcome {
    /// Serialize the metrics to YAML (the `metrics.yaml` artifact body).
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml_ng::to_string(&self.metrics)?)
    }
}

/// Prometheus infra metrics documented as best-effort regardless of the using
/// query's stability (they are upstream contracts, not ours to grade).
fn infra_best_effort(name: &str) -> bool {
    name.starts_with("kube_") || name.starts_with("container_")
}

/// Extract and aggregate the documented metrics for every metric query in
/// `registry`, rendered through `ctx`.
pub fn extract_metric_docs(registry: &QueryRegistry, ctx: &TemplateContext) -> DocgenOutcome {
    let mut metrics: IndexMap<String, MetricDoc> = IndexMap::new();
    let mut errors = Vec::new();

    for query in registry.iter_metric_queries() {
        let extracted = match query.extract_metrics(ctx) {
            Ok(extracted) => extracted,
            Err(err) => {
                errors.push((query.id.clone(), err));
                continue;
            }
        };

        for metric in extracted {
            let stability = if infra_best_effort(&metric.name) {
                Stability::BestEffort
            } else {
                query.stability
            };

            match metrics.get_mut(&metric.name) {
                None => {
                    let mut labels: Vec<String> = metric.labels;
                    labels.sort();
                    labels.dedup();
                    metrics.insert(
                        metric.name.clone(),
                        MetricDoc {
                            labels,
                            name: metric.name,
                            stability: stability.to_string(),
                            usage: vec![query.id.clone()],
                        },
                    );
                }
                Some(doc) => {
                    let labels: BTreeSet<String> =
                        doc.labels.drain(..).chain(metric.labels).collect();
                    doc.labels = labels.into_iter().collect();

                    let mut usage: BTreeSet<String> = doc.usage.drain(..).collect();
                    usage.insert(query.id.clone());
                    doc.usage = usage.into_iter().collect();

                    // Keep the more mature stability.
                    let existing = Stability::from_str(&doc.stability)
                        .expect("stability strings are written by us");
                    if stability > existing {
                        doc.stability = stability.to_string();
                    }
                }
            }
        }
    }

    let mut docs: Vec<MetricDoc> = metrics.into_values().collect();
    docs.sort_by(|a, b| {
        let maturity = |doc: &MetricDoc| {
            Stability::from_str(&doc.stability)
                .expect("stability strings are written by us")
                .maturity()
        };
        // Most mature first, then most used, then name ascending.
        maturity(b)
            .cmp(&maturity(a))
            .then_with(|| b.usage.len().cmp(&a.usage.len()))
            .then_with(|| a.name.cmp(&b.name))
    });

    DocgenOutcome {
        metrics: docs,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::model::QueryEngine;
    use crate::query::render::doc_context;

    /// Build a registry from a single PromQL query definition (YAML fragment).
    fn registry_with(queries_yaml: &str) -> QueryRegistry {
        let doc = format!("description: test\nqueries:\n{queries_yaml}");
        let parsed: crate::query::def::RegistryDoc = serde_yaml_ng::from_str(&doc).unwrap();
        let mut registry = QueryRegistry::new();
        registry.load(parsed).unwrap();
        registry
    }

    fn run(registry: &QueryRegistry) -> Vec<MetricDoc> {
        let ctx = doc_context(registry, QueryEngine::PromQl);
        let outcome = extract_metric_docs(registry, &ctx);
        assert!(
            outcome.errors.is_empty(),
            "unexpected errors: {:?}",
            outcome.errors
        );
        outcome.metrics
    }

    #[test]
    fn merges_labels_usage_and_promotes_stability() {
        let registry = registry_with(
            r#"
  - id: q.experimental
    stability: experimental
    description: {summary: s}
    promQL: 'shared{a="1"}'
  - id: q.canonical
    stability: canonical
    description: {summary: s}
    promQL: 'shared{b="2"}'
"#,
        );
        let docs = run(&registry);
        assert_eq!(docs.len(), 1);
        let doc = &docs[0];
        assert_eq!(doc.name, "shared");
        assert_eq!(doc.labels, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            doc.usage,
            vec!["q.canonical".to_string(), "q.experimental".to_string()]
        );
        // Canonical is more mature than experimental.
        assert_eq!(doc.stability, "canonical");
    }

    #[test]
    fn kube_and_container_metrics_are_forced_best_effort() {
        let registry = registry_with(
            r#"
  - id: q.playground
    stability: playground
    description: {summary: s}
    promQL:
      - 'kube_pod_status_phase{a="1"}'
      - 'container_cpu_usage_seconds_total{b="2"}'
      - 'mz_regular_metric{c="3"}'
"#,
        );
        let docs = run(&registry);
        let by_name = |n: &str| docs.iter().find(|d| d.name == n).unwrap();
        assert_eq!(by_name("kube_pod_status_phase").stability, "best-effort");
        assert_eq!(
            by_name("container_cpu_usage_seconds_total").stability,
            "best-effort"
        );
        // A non-infra metric keeps the query's stability.
        assert_eq!(by_name("mz_regular_metric").stability, "playground");
    }

    #[test]
    fn sorts_by_maturity_then_usage_then_name() {
        let registry = registry_with(
            r#"
  - id: q1
    stability: best-effort
    description: {summary: s}
    promQL:
      - 'used_twice{a="1"}'
      - 'used_once_a{a="1"}'
  - id: q2
    stability: best-effort
    description: {summary: s}
    promQL:
      - 'used_twice{b="2"}'
      - 'used_once_b{b="2"}'
  - id: q3
    stability: experimental
    description: {summary: s}
    promQL: 'experimental_metric{c="3"}'
"#,
        );
        let docs = run(&registry);
        let names: Vec<&str> = docs.iter().map(|d| d.name.as_str()).collect();
        // best-effort (more mature) before experimental; within best-effort,
        // used_twice (2 uses) first, then the single-use pair alphabetically.
        assert_eq!(
            names,
            vec![
                "used_twice",
                "used_once_a",
                "used_once_b",
                "experimental_metric",
            ]
        );
    }

    #[test]
    fn empty_registry_yields_no_metrics() {
        let registry = QueryRegistry::new();
        let ctx = doc_context(&registry, QueryEngine::PromQl);
        assert!(extract_metric_docs(&registry, &ctx).metrics.is_empty());
    }
}
