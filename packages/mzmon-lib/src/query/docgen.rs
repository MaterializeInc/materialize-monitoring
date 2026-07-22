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
//! Descended from `query_cli.docgen`, but the per-metric column is now
//! **importance**, not stability: each metric query is rendered and its metrics
//! extracted; occurrences of the same metric across queries are merged (labels
//! and `usage` unioned, importance rolled up **greatest-wins** — if any query is
//! `essential`, the metric is `essential`). `metricOverrides` then set a metric's
//! importance outright. The result is sorted most-important-then-most-used-then-
//! name and serialized to YAML.
//!
//! Importance is the axis Python's `docgen` does not have, so this output
//! intentionally diverges from it — the Rust tool is authoritative. Extraction
//! (metric names, labels, usage) is unchanged and still matches Python.
//!
//! The output is a pure function of the registry content: labels and usage are
//! sorted, importance is a commutative max plus deterministic overrides, and the
//! final order is total — so it does not depend on query registration order.

use std::collections::BTreeSet;
use std::str::FromStr;

use indexmap::IndexMap;
use serde::Serialize;

use crate::query::error::{Error, Result};
use crate::query::importance::Importance;
use crate::query::registry::QueryRegistry;
use crate::query::render::TemplateContext;

/// Documentation for a single metric. Field order is alphabetical (`importance`,
/// `labels`, `name`, `usage`) so the serialized YAML keys stay sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetricDoc {
    /// The metric's importance: the greatest-wins roll-up across the queries that
    /// reference it, or an override's value if one matches.
    pub importance: String,
    /// The distinct label names any query matches on this metric, sorted.
    pub labels: Vec<String>,
    /// The metric name.
    pub name: String,
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

/// Per-metric aggregation accumulator: label/usage sets plus the running
/// greatest-wins importance across the queries that reference the metric.
struct Aggregate {
    labels: BTreeSet<String>,
    usage: BTreeSet<String>,
    importance: Importance,
}

/// Extract and aggregate the documented metrics for every metric query in
/// `registry`, rendered through `ctx`.
pub fn extract_metric_docs(registry: &QueryRegistry, ctx: &TemplateContext) -> DocgenOutcome {
    let mut aggregates: IndexMap<String, Aggregate> = IndexMap::new();
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
            let entry = aggregates.entry(metric.name).or_insert_with(|| Aggregate {
                labels: BTreeSet::new(),
                usage: BTreeSet::new(),
                importance: query.importance,
            });
            entry.labels.extend(metric.labels);
            entry.usage.insert(query.id.clone());
            // Greatest-wins: the metric is as important as its most important
            // referencing query.
            entry.importance = entry.importance.max(query.importance);
        }
    }

    let mut docs: Vec<MetricDoc> = aggregates
        .into_iter()
        .map(|(name, aggregate)| {
            // A matching override replaces the rolled-up importance outright.
            let importance = registry
                .override_importance(&name)
                .unwrap_or(aggregate.importance);
            MetricDoc {
                importance: importance.to_string(),
                labels: aggregate.labels.into_iter().collect(),
                name,
                usage: aggregate.usage.into_iter().collect(),
            }
        })
        .collect();

    docs.sort_by(|a, b| {
        let rank = |doc: &MetricDoc| {
            Importance::from_str(&doc.importance)
                .expect("importance strings are written by us")
                .rank()
        };
        // Most important first, then most used, then name ascending.
        rank(b)
            .cmp(&rank(a))
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

    /// Load a registry from one or more full registry-file YAML documents.
    fn load_docs(docs: &[&str]) -> QueryRegistry {
        let mut registry = QueryRegistry::new();
        for doc in docs {
            let parsed: crate::query::def::RegistryDoc = serde_yaml_ng::from_str(doc).unwrap();
            registry.load(parsed).unwrap();
        }
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

    fn by_name<'a>(docs: &'a [MetricDoc], name: &str) -> &'a MetricDoc {
        docs.iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("no metric named {name}"))
    }

    #[test]
    fn merges_labels_usage_and_rolls_up_importance_greatest_wins() {
        // `shared` is referenced by a diagnostic-hinted file and an
        // essential-hinted file; the more important wins.
        let registry = load_docs(&[
            r#"
description: low
metricImportanceHint: diagnostic
queries:
  - id: q.diagnostic
    stability: best-effort
    description: {summary: s}
    promQL: 'shared{a="1"}'
"#,
            r#"
description: high
metricImportanceHint: essential
queries:
  - id: q.essential
    stability: best-effort
    description: {summary: s}
    promQL: 'shared{b="2"}'
"#,
        ]);
        let docs = run(&registry);
        assert_eq!(docs.len(), 1);
        let doc = &docs[0];
        assert_eq!(doc.name, "shared");
        assert_eq!(doc.labels, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            doc.usage,
            vec!["q.diagnostic".to_string(), "q.essential".to_string()]
        );
        assert_eq!(doc.importance, "essential");
    }

    #[test]
    fn overrides_set_importance_absolutely_both_directions() {
        // One override raises an `_info` metric; another lowers a `noisy_` metric
        // even though its query is recommended. A metric matching nothing keeps
        // the rolled-up hint.
        let registry = load_docs(&[r#"
description: test
metricImportanceHint: recommended
queries:
  - id: q
    stability: best-effort
    description: {summary: s}
    promQL:
      - 'mz_object_info{a="1"}'
      - 'noisy_metric{b="2"}'
      - 'normal_metric{c="3"}'
metricOverrides:
  - metricPattern: ".*_info"
    importance: essential
  - metricPattern: "noisy_.*"
    importance: diagnostic
"#]);
        let docs = run(&registry);
        assert_eq!(by_name(&docs, "mz_object_info").importance, "essential");
        assert_eq!(by_name(&docs, "noisy_metric").importance, "diagnostic");
        assert_eq!(by_name(&docs, "normal_metric").importance, "recommended");
    }

    #[test]
    fn override_priority_breaks_overlaps() {
        // Two overrides match `mz_object_info`; the higher priority wins.
        let registry = load_docs(&[r#"
description: test
metricImportanceHint: recommended
queries:
  - id: q
    stability: best-effort
    description: {summary: s}
    promQL: 'mz_object_info{a="1"}'
metricOverrides:
  - metricPattern: "mz_.*"
    importance: extended
    priority: 1
  - metricPattern: ".*_info"
    importance: essential
    priority: 5
"#]);
        let docs = run(&registry);
        assert_eq!(by_name(&docs, "mz_object_info").importance, "essential");
    }

    #[test]
    fn anchored_pattern_does_not_match_substrings() {
        // `mz_foo` is anchored, so it must match the whole name.
        let registry = load_docs(&[r#"
description: test
metricImportanceHint: recommended
queries:
  - id: q
    stability: best-effort
    description: {summary: s}
    promQL: 'mz_foo_extra{a="1"}'
metricOverrides:
  - metricPattern: "mz_foo"
    importance: essential
"#]);
        // The override pattern `mz_foo` must not match `mz_foo_extra`.
        assert_eq!(
            by_name(&run(&registry), "mz_foo_extra").importance,
            "recommended"
        );
    }

    #[test]
    fn sorts_by_importance_then_usage_then_name() {
        let registry = load_docs(&[
            r#"
description: recommended file
metricImportanceHint: recommended
queries:
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
"#,
            r#"
description: diagnostic file
metricImportanceHint: diagnostic
queries:
  - id: q3
    stability: best-effort
    description: {summary: s}
    promQL: 'diagnostic_metric{c="3"}'
"#,
        ]);
        let docs = run(&registry);
        let names: Vec<&str> = docs.iter().map(|d| d.name.as_str()).collect();
        // recommended (more important) before diagnostic; within recommended,
        // used_twice (2 uses) first, then the single-use pair alphabetically.
        assert_eq!(
            names,
            vec![
                "used_twice",
                "used_once_a",
                "used_once_b",
                "diagnostic_metric",
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
