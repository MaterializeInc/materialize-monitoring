// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Test-only helpers and fixtures for the query registry.
//!
//! The real query files under `packages/queries/` are embedded so tests run
//! hermetically (cf. `scrape::test_support`). The golden `metrics.yaml.snap` is
//! the Python `query_cli docgen` output for those same files; the parity test
//! compares the Rust `extract-metrics` output against it **structurally** (parsed
//! to a value), so YAML formatting differences between the two serializers don't
//! matter.
//!
//! Keep both in sync with `packages/queries/`: when a query file changes,
//! regenerate the golden with
//! `python3 -m py_mzmon_lib.registry.query_cli docgen --source-dir packages/queries --out-dir packages/mzmon-lib/src/query/testdata`
//! then rename `metrics.yaml` to `metrics.yaml.snap`.

use serde_json::Value;

use crate::query::def::RegistryDoc;
use crate::query::registry::QueryRegistry;

/// The real registry files under `packages/queries/`, embedded. Keep in sync
/// with that directory.
pub(crate) const FIXTURES: &[(&str, &str)] = &[
    (
        "materialize-clusters",
        include_str!("../../../queries/materialize-clusters.yaml"),
    ),
    (
        "materialize-compute",
        include_str!("../../../queries/materialize-compute.yaml"),
    ),
    (
        "materialize-connections",
        include_str!("../../../queries/materialize-connections.yaml"),
    ),
    (
        "materialize-health",
        include_str!("../../../queries/materialize-health.yaml"),
    ),
    (
        "materialize-kubernetes",
        include_str!("../../../queries/materialize-kubernetes.yaml"),
    ),
    (
        "materialize-perf",
        include_str!("../../../queries/materialize-perf.yaml"),
    ),
    (
        "materialize-storage",
        include_str!("../../../queries/materialize-storage.yaml"),
    ),
];

/// The Python `docgen` golden for the fixtures above.
pub(crate) const GOLDEN_METRICS: &str = include_str!("testdata/metrics.yaml.snap");

/// Build a registry from all embedded fixtures, in the same sorted order
/// `from_directory` would use.
pub(crate) fn corpus_registry() -> QueryRegistry {
    let mut registry = QueryRegistry::new();
    let mut fixtures: Vec<_> = FIXTURES.to_vec();
    fixtures.sort_by_key(|(name, _)| *name);
    for (name, yaml) in fixtures {
        let doc = RegistryDoc::from_yaml_str(yaml)
            .unwrap_or_else(|err| panic!("fixture {name} failed to parse: {err}"));
        registry
            .load(doc)
            .unwrap_or_else(|err| panic!("fixture {name} failed to load: {err}"));
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::docgen::extract_metric_docs;
    use crate::query::model::QueryEngine;
    use crate::query::render::doc_context;

    #[test]
    fn corpus_loads_all_queries() {
        let registry = corpus_registry();
        // 71 queries across the seven files, matching the Python loader.
        assert_eq!(registry.len(), 71);
        // A representative spread of engines / shapes is present.
        assert!(registry.get("materialize.clusters.count").is_some());
        assert_eq!(registry.iter_metric_queries().count(), 71);
        assert_eq!(registry.iter_log_queries(false).count(), 0);
    }

    #[test]
    fn corpus_renders_without_errors_under_doc_context() {
        let registry = corpus_registry();
        let ctx = doc_context(&registry, QueryEngine::PromQl);
        for query in registry.iter_metric_queries() {
            query
                .render(&ctx)
                .unwrap_or_else(|err| panic!("query {} failed to render: {err}", query.id));
        }
    }

    /// The headline parity test: the Rust `extract-metrics` output equals the
    /// Python `docgen` golden, compared as structured data.
    #[test]
    fn extract_metrics_matches_python_golden() {
        let registry = corpus_registry();
        let ctx = doc_context(&registry, QueryEngine::PromQl);
        let outcome = extract_metric_docs(&registry, &ctx);
        assert!(
            outcome.errors.is_empty(),
            "unexpected extraction errors: {:?}",
            outcome.errors
        );

        let produced: Value =
            serde_json::to_value(&outcome.metrics).expect("serialize produced metrics");
        let expected: Value =
            serde_yaml_ng::from_str(GOLDEN_METRICS).expect("parse golden metrics.yaml.snap");

        assert_eq!(
            produced,
            expected,
            "extract-metrics output diverged from the Python golden\n\
             --- produced ({} metrics) ---\n{}",
            outcome.metrics.len(),
            outcome.to_yaml().unwrap()
        );
    }
}
