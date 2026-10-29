// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Registry queries, rendered for a Datadog dashboard.
//!
//! The Datadog counterpart of [`crate::grafana::queries`]: a panel names a
//! registry id, and [`mzmon_lib::datadog::query`] renders its `datadogQuery` in
//! [`mzmon_lib::datadog::context`]. This handle records the lookups that fail,
//! so a dashboard reports every bad id at once.

use std::cell::RefCell;

use mzmon_lib::datadog::context::dashboard_context;
use mzmon_lib::datadog::query::{PanelQuery, panel_query};
use mzmon_lib::query::{QueryRegistry, TemplateContext};

use super::Options;

/// A registry plus the Datadog rendering context a dashboard uses.
pub struct Queries<'a> {
    registry: &'a QueryRegistry,
    context: TemplateContext<'a>,
    failures: RefCell<Vec<String>>,
}

impl<'a> Queries<'a> {
    /// Bind a registry to the Datadog dashboard context.
    pub fn new(registry: &'a QueryRegistry, options: &Options) -> Self {
        Queries {
            registry,
            context: dashboard_context(registry, &options.sql_metric_prefix),
            failures: RefCell::new(Vec::new()),
        }
    }

    /// Render a registry query for a widget, recording a failure rather than
    /// propagating it so widget builders stay infallible.
    pub fn get(&self, id: &str) -> PanelQuery {
        match panel_query(self.registry, id, &self.context) {
            Ok(query) => query,
            Err(e) => {
                self.failures.borrow_mut().push(format!("{id}: {e}"));
                PanelQuery {
                    id: id.to_string(),
                    expressions: Vec::new(),
                    description: String::new(),
                }
            }
        }
    }

    /// [`Queries::get`] for a widget that shows one number, such as a query value.
    ///
    /// A registry entry with several templates would leave all but the first
    /// undrawn, so that is recorded as a failure too.
    pub fn single(&self, id: &str) -> PanelQuery {
        let query = self.get(id);
        if query.expressions.len() > 1 {
            self.failures.borrow_mut().push(format!(
                "{id}: has {} Datadog templates, and this widget draws one",
                query.expressions.len()
            ));
        }
        query
    }

    /// [`Queries::get`] for a widget per template, each labeled in order.
    ///
    /// The labels are positional against the registry entry's `datadogQuery`
    /// list, as [`crate::grafana::queries::Queries::legended`]'s are against its
    /// `promQL` list, so a count mismatch means the two have drifted and is
    /// recorded as a failure.
    pub fn labeled(&self, id: &str, labels: &[&str]) -> PanelQuery {
        let query = self.get(id);
        if !query.expressions.is_empty() && query.expressions.len() != labels.len() {
            self.failures.borrow_mut().push(format!(
                "{id}: {} labels for {} Datadog templates",
                labels.len(),
                query.expressions.len()
            ));
        }
        query
    }

    /// Every lookup that failed, in the order they were attempted.
    pub fn failures(&self) -> Vec<String> {
        self.failures.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_registry;

    #[test]
    fn a_registry_query_renders_with_the_environment_variable() {
        let queries = Queries::new(test_registry(), &Options::default());
        let query = queries.get("materialize.compute.views.count");
        assert_eq!(queries.failures(), Vec::<String>::new());
        assert_eq!(query.expressions.len(), 1);
        assert_eq!(
            query.expressions[0].queries,
            vec!["max:mz_views_count{$environment}"]
        );
        assert_eq!(query.expressions[0].formula, "query1");
        assert!(!query.description.is_empty(), "the description comes along");
    }

    #[test]
    fn a_wrapped_template_splits_into_a_query_and_a_formula() {
        // The registry writes `count_not_null(avg:… by {…})` over two lines; the
        // widget gets the bare query and the function as its formula.
        let queries = Queries::new(test_registry(), &Options::default());
        let query = queries.get("materialize.clusters.count");
        assert_eq!(queries.failures(), Vec::<String>::new());
        for expression in &query.expressions {
            assert_eq!(expression.formula, "count_not_null(query1)");
            assert!(expression.queries[0].starts_with("avg:mz_compute_cluster_status{"));
        }
    }

    #[test]
    fn an_unmapped_parameter_is_a_failure_not_a_literal() {
        // `generations.version` reads `%%{range}`, which has no meaning on a
        // Datadog dashboard: a widget reduces over its own window.
        let queries = Queries::new(test_registry(), &Options::default());
        let query = queries.get("materialize.generations.version");
        assert!(query.expressions.is_empty());
        let failures = queries.failures();
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("materialize.generations.version"));
    }

    #[test]
    fn the_pickers_fill_tag_values() {
        let queries = Queries::new(test_registry(), &Options::default());
        let query = queries.get("materialize.kubernetes.pods.network_rx");
        assert_eq!(queries.failures(), Vec::<String>::new());
        assert_eq!(
            query.expressions[0].queries,
            vec![
                "sum:container_network_receive_bytes_total{namespace:$namespace.value AND \
                 pod:*-cluster-$cluster.value-replica-$replica.value-*} by {namespace,pod}\
                 .as_rate().rollup(sum, 60)"
            ]
        );
    }

    #[test]
    fn labels_have_to_match_the_templates() {
        let queries = Queries::new(test_registry(), &Options::default());
        let query = queries.labeled("materialize.clusters.count", &["Total", "System"]);
        assert_eq!(query.expressions.len(), 2);
        assert_eq!(queries.failures(), Vec::<String>::new());

        queries.labeled("materialize.clusters.count", &["Total"]);
        assert_eq!(queries.failures().len(), 1, "one label for two templates");
    }

    #[test]
    fn an_unknown_id_is_a_failure() {
        let queries = Queries::new(test_registry(), &Options::default());
        queries.get("materialize.no.such.query");
        assert_eq!(queries.failures().len(), 1);
    }
}
