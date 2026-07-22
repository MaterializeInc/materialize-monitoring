// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Best-effort metric extraction from rendered PromQL.
//!
//! Ported from `ExtractedMetric.extract_from_promql` in
//! `py_mzmon_lib.registry.queries`, which walks the parsed AST and collects each
//! named vector selector's metric name plus the set of label names it matches
//! on. Both the Python original and this port sit on the GreptimeTeam PromQL
//! parser (`promql_parser` there, [`promql_parser`] here), so the AST shapes line
//! up one-to-one.
//!
//! Two behaviors are load-bearing for parity with the Python output:
//!
//! * **Anonymous selectors are skipped.** A `{__name__="x"}` selector has no
//!   `name` (the parser leaves it `None`), so it contributes no metric — matching
//!   the Python `if not name: return`. `__name__` is also filtered from the label
//!   set defensively (it never appears when the name is written as a prefix).
//! * **Range selectors are descended into.** `promql_parser::util::walk_expr`
//!   treats a [`MatrixSelector`] (e.g. `foo[5m]` inside `rate(...)`) as a leaf and
//!   does *not* visit its inner vector selector, so the visitor handles
//!   `MatrixSelector` explicitly. Without this, every metric used only inside a
//!   `rate`/`increase`/… range would be missed.

use std::collections::HashSet;
use std::convert::Infallible;

use promql_parser::label::{METRIC_NAME, Matchers};
use promql_parser::parser::{Expr, VectorSelector, parse};
use promql_parser::util::{ExprVisitor, walk_expr};

use crate::query::error::{Error, Result};
use crate::query::model::Query;
use crate::query::render::TemplateContext;

/// A metric name and the label names a query matches on it. "Best-effort": this
/// reflects only what the rendered PromQL names syntactically, not what the
/// series actually carry at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedMetric {
    /// The metric (`__name__`) the selector names.
    pub name: String,
    /// The distinct label names matched on the selector, in first-seen order.
    /// Callers that need a canonical order should sort (as `docgen` does).
    pub labels: Vec<String>,
}

impl ExtractedMetric {
    /// Extract every named metric selector from a PromQL expression.
    pub fn extract_from_promql(promql: &str) -> Result<Vec<ExtractedMetric>> {
        let ast = parse(promql).map_err(|message| Error::PromQlParse {
            expr: promql.to_string(),
            message,
        })?;
        let mut collector = Collector::default();
        // `pre_visit` is infallible, so `walk_expr` cannot error.
        let Ok(_) = walk_expr(&mut collector, &ast);
        Ok(collector.metrics)
    }
}

impl Query {
    /// Render this query for `ctx` and extract every metric it references.
    pub fn extract_metrics(&self, ctx: &TemplateContext) -> Result<Vec<ExtractedMetric>> {
        let mut out = Vec::new();
        for rendered in self.render(ctx)? {
            out.extend(ExtractedMetric::extract_from_promql(&rendered)?);
        }
        Ok(out)
    }
}

/// AST visitor accumulating one [`ExtractedMetric`] per named vector selector.
#[derive(Default)]
struct Collector {
    metrics: Vec<ExtractedMetric>,
}

impl Collector {
    /// Record a metric for `vs` if it names one (skipping anonymous selectors).
    fn record(&mut self, vs: &VectorSelector) {
        let Some(name) = vs.name.as_ref().filter(|n| !n.is_empty()) else {
            return;
        };
        self.metrics.push(ExtractedMetric {
            name: name.clone(),
            labels: matcher_labels(&vs.matchers),
        });
    }
}

impl ExprVisitor for Collector {
    type Error = Infallible;

    fn pre_visit(&mut self, expr: &Expr) -> std::result::Result<bool, Infallible> {
        match expr {
            Expr::VectorSelector(vs) => self.record(vs),
            // `walk_expr` does not descend into a matrix selector's inner vector
            // selector, so capture it here.
            Expr::MatrixSelector(ms) => self.record(&ms.vs),
            _ => {}
        }
        Ok(true)
    }
}

/// The distinct label names across a selector's matchers (both the plain
/// matchers and the `or` groups), excluding the synthetic `__name__`, in
/// first-seen order.
fn matcher_labels(matchers: &Matchers) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut labels = Vec::new();
    let groups = std::iter::once(&matchers.matchers).chain(matchers.or_matchers.iter());
    for group in groups {
        for matcher in group {
            if matcher.name == METRIC_NAME {
                continue;
            }
            if seen.insert(matcher.name.clone()) {
                labels.push(matcher.name.clone());
            }
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(promql: &str) -> Vec<ExtractedMetric> {
        let mut metrics = ExtractedMetric::extract_from_promql(promql).unwrap();
        // Sort for stable assertions (order is irrelevant downstream).
        for m in &mut metrics {
            m.labels.sort();
        }
        metrics.sort_by(|a, b| a.name.cmp(&b.name));
        metrics
    }

    #[test]
    fn simple_selector() {
        let metrics = extract(r#"up{job="x", instance="y"}"#);
        assert_eq!(
            metrics,
            vec![ExtractedMetric {
                name: "up".to_string(),
                labels: vec!["instance".to_string(), "job".to_string()],
            }]
        );
    }

    #[test]
    fn name_matcher_is_not_a_label() {
        // Explicit prefix: `__name__` never appears among the labels.
        let metrics = extract(r#"foo{a="1"}"#);
        assert_eq!(metrics[0].labels, vec!["a".to_string()]);
    }

    #[test]
    fn anonymous_selector_is_skipped() {
        // `{__name__="x"}` has no name → contributes nothing (matches Python).
        assert!(extract(r#"{__name__="baz", d="4"}"#).is_empty());
    }

    #[test]
    fn descends_into_range_selectors_and_functions() {
        // The metric lives inside a `rate(...[5m])`; it must still be found.
        let metrics =
            extract(r#"sum(rate(container_cpu_usage_seconds_total{container="c"}[5m])) by (pod)"#);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "container_cpu_usage_seconds_total");
        assert_eq!(metrics[0].labels, vec!["container".to_string()]);
    }

    #[test]
    fn binary_expression_collects_both_sides() {
        let metrics = extract(r#"foo{a="1"} / bar{b="2"}"#);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].name, "bar");
        assert_eq!(metrics[1].name, "foo");
    }

    #[test]
    fn or_matchers_are_included() {
        // Prometheus `or` label matchers contribute their names too.
        let metrics = extract(r#"foo{a="1" or b="2"}"#);
        assert_eq!(metrics[0].labels, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn vector_zero_from_or_zero_contributes_no_metric() {
        // `orZero` wraps expressions as `(...) or vector(0)`; `vector(0)` is a
        // call, not a selector, so it adds nothing.
        let metrics = extract(r#"(foo{a="1"}) or vector(0)"#);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "foo");
    }

    #[test]
    fn parse_error_is_reported() {
        let err = ExtractedMetric::extract_from_promql("sum(((").unwrap_err();
        assert!(matches!(err, Error::PromQlParse { .. }));
    }
}
