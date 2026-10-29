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
//! registry id, and its `datadogQuery` and description come from the registry.
//!
//! Parameters are mapped as panels need them. A template that uses one this
//! context does not set fails the render, rather than reaching Datadog as a
//! literal `%%{…}`, so porting a panel is also the moment its parameters get a
//! Datadog meaning.

use std::cell::RefCell;

use mzmon_lib::grafana::query::format_description;
use mzmon_lib::query::{QueryEngine, QueryRegistry, TemplateContext};

use super::formula::Expression;
use super::{Options, variable};

/// A registry query rendered for one widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelQuery {
    /// The registry id, kept for error messages.
    pub id: String,
    /// One expression per template the registry entry defines, split into the
    /// metric queries and the formula a widget request carries.
    pub expressions: Vec<Expression>,
    /// The registry description, as Markdown.
    pub description: String,
}

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
            context: dashboard_context(registry, options),
            failures: RefCell::new(Vec::new()),
        }
    }

    /// Render a registry query for a widget, recording a failure rather than
    /// propagating it so widget builders stay infallible.
    pub fn get(&self, id: &str) -> PanelQuery {
        let Some(query) = self.registry.get(id) else {
            return self.fail(id, "no such query in the registry".to_string());
        };
        let rendered = match query.render(&self.context) {
            Ok(rendered) => rendered,
            Err(e) => return self.fail(id, e.to_string()),
        };
        let expressions = match rendered
            .iter()
            .map(|e| Expression::parse(&one_line(e)))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(expressions) => expressions,
            Err(e) => return self.fail(id, e),
        };
        PanelQuery {
            id: id.to_string(),
            expressions,
            description: format_description(&query.description),
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

    fn fail(&self, id: &str, reason: String) -> PanelQuery {
        self.failures.borrow_mut().push(format!("{id}: {reason}"));
        PanelQuery {
            id: id.to_string(),
            expressions: Vec::new(),
            description: String::new(),
        }
    }
}

/// Collapse a rendered template onto one line.
///
/// The registry wraps long templates for reading, and a Datadog widget's query
/// field is a single line. Whitespace in a metric query is never significant, so
/// every run of it becomes one space.
fn one_line(expr: &str) -> String {
    expr.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The parameters a Datadog dashboard gives the registry's templates.
fn dashboard_context<'a>(registry: &'a QueryRegistry, options: &Options) -> TemplateContext<'a> {
    let mut context = TemplateContext::new(QueryEngine::Datadog).with_registry(registry);
    for (name, value) in [
        ("mzSqlPrefix", options.sql_metric_prefix.clone()),
        // `$environment` expands to the environment tag and the picked value, or
        // to nothing when the picker is `*`.
        ("mzEnvironmentFilter", format!("${}", variable::ENVIRONMENT)),
        // Every cluster and replica: the templates write `compute_cluster_id:%%{…}`,
        // and `*` matches any value.
        // TODO: map to cluster and replica pickers when env-top's are ported (DEP-115).
        ("mzClusterList", "*".to_string()),
        ("mzReplicaList", "*".to_string()),
    ] {
        context.parameters.insert(name.to_string(), value);
    }
    context
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
        // `cpu.capacity` reads `%%{cAdvisorFilter}`, which this context does not
        // map yet.
        let queries = Queries::new(test_registry(), &Options::default());
        let query = queries.get("materialize.kubernetes.cpu.capacity");
        assert!(query.expressions.is_empty());
        let failures = queries.failures();
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("materialize.kubernetes.cpu.capacity"));
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
    fn a_wrapped_template_renders_on_one_line() {
        assert_eq!(
            one_line("count_not_null(\n  avg:m{a} by {b}\n)\n"),
            "count_not_null( avg:m{a} by {b} )"
        );
    }

    #[test]
    fn an_unknown_id_is_a_failure() {
        let queries = Queries::new(test_registry(), &Options::default());
        queries.get("materialize.no.such.query");
        assert_eq!(queries.failures().len(), 1);
    }
}
