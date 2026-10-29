// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! A rendered Datadog expression, split into the shape a widget request takes.
//!
//! The registry writes each `datadogQuery` as one expression, such as
//! `default_zero(count_not_null(avg:m{f} by {t}))`, which is how Datadog's query
//! API and monitors read it. A dashboard widget does not. Its request holds bare
//! metric queries, each named, and formulas that apply every function and every
//! operator to those names. A widget whose query object carries a function is
//! drawn as "Invalid Formula in Query Object", although the query API evaluates
//! the same expression without complaint.
//!
//! So each metric query is lifted out of the expression and replaced by its
//! name, and what remains is the formula: the expression above becomes
//! `query1 = avg:m{f} by {t}` and `default_zero(count_not_null(query1))`.

use std::sync::LazyLock;

use regex::Regex;

/// One metric query: a space aggregator, a metric, its filter, an optional
/// group-by, and the modifiers Datadog keeps on the query object, such as
/// `.as_rate()`, `.rollup(sum, 60)` and `.fill(zero)`.
///
/// The registry's templates put the modifiers after the group-by, and use no
/// other space aggregator than these.
static METRIC_QUERY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:avg|sum|min|max|count|p\d+(?:\.\d+)?):[A-Za-z_][A-Za-z0-9_.]*\{[^{}]*\}(?:\s*by\s*\{[^{}]*\})?(?:\.[a-z_]+\([^()]*\))*",
    )
    .unwrap()
});

/// The name a widget gives its `index`th metric query, which its formulas use.
///
/// `query1`, `query2`, … is what the Datadog editor names them, so a widget edited
/// in the UI after import keeps reading the same way.
pub fn query_name(index: usize) -> String {
    format!("query{}", index + 1)
}

/// A rendered expression as a widget request carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    /// The bare metric queries, in order of first appearance. The `i`th is named
    /// [`query_name`]`(i)`.
    pub queries: Vec<String>,
    /// The expression with each metric query replaced by its name.
    pub formula: String,
}

impl Expression {
    /// Split a one-line rendered expression.
    ///
    /// A metric query that appears twice gets one name. An expression with no
    /// metric query in it is an error, because a widget has nothing to draw.
    pub fn parse(expr: &str) -> Result<Expression, String> {
        let mut queries: Vec<String> = Vec::new();
        let mut formula = String::with_capacity(expr.len());
        let mut last = 0;
        for found in METRIC_QUERY.find_iter(expr) {
            formula.push_str(&expr[last..found.start()]);
            let query = found.as_str().to_string();
            let index = match queries.iter().position(|q| *q == query) {
                Some(index) => index,
                None => {
                    queries.push(query);
                    queries.len() - 1
                }
            };
            formula.push_str(&query_name(index));
            last = found.end();
        }
        formula.push_str(&expr[last..]);
        if queries.is_empty() {
            return Err(format!("no metric query in `{expr}`"));
        }
        Ok(Expression {
            queries,
            formula: tidy(&formula),
        })
    }
}

/// A formula's reference to one of its request's queries.
static QUERY_REFERENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bquery(\d+)\b").unwrap());

/// Several expressions as one widget request: their metric queries, named across
/// all of them, and one formula per expression, in order.
///
/// Each expression names its queries from `query1`, so the second one's names are
/// renumbered past the first's. Each expression comes with a key that its queries
/// carry, such as how a scalar widget reduces them, and a metric query two
/// expressions share under the same key gets one name.
pub fn combine<K: Clone + PartialEq>(
    parts: &[(&Expression, K)],
) -> (Vec<(String, K)>, Vec<String>) {
    let mut queries: Vec<(String, K)> = Vec::new();
    let mut formulas = Vec::with_capacity(parts.len());
    for (expr, key) in parts {
        let names: Vec<String> = expr
            .queries
            .iter()
            .map(|query| {
                let known = queries.iter().position(|(q, k)| q == query && k == key);
                let index = match known {
                    Some(index) => index,
                    None => {
                        queries.push((query.clone(), key.clone()));
                        queries.len() - 1
                    }
                };
                query_name(index)
            })
            .collect();
        let formula = QUERY_REFERENCE.replace_all(&expr.formula, |found: &regex::Captures| {
            let own: usize = found[1].parse().expect("the pattern matched digits");
            names[own - 1].clone()
        });
        formulas.push(formula.into_owned());
    }
    (queries, formulas)
}

/// A group-by function's tag set, as in `max(query1, {pod})`.
static GROUP_BY_TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r",\s*\{").unwrap());

/// Whether `formula` aggregates in two layers, through a group-by function such
/// as `max(query1, {pod})` or `count(query1, {size})`.
///
/// A scalar widget can't reduce such a formula with its queries' aggregators.
/// Datadog refuses an aggregator under a group-by function, so the formula
/// reduces over the window itself, with `reduce(…, 'last')`.
pub fn aggregates_in_layers(formula: &str) -> bool {
    GROUP_BY_TAGS.is_match(formula)
}

/// Whether `query` is exactly one bare metric query, with nothing around it.
pub fn is_bare_metric_query(query: &str) -> bool {
    METRIC_QUERY
        .find(query)
        .is_some_and(|found| found.start() == 0 && found.end() == query.len())
}

/// Drop the padding the registry's wrapped templates leave inside parentheses.
fn tidy(formula: &str) -> String {
    formula.trim().replace("( ", "(").replace(" )", ")")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_query_is_its_own_formula() {
        let expr = Expression::parse("max:mz_views_count{$environment}").unwrap();
        assert_eq!(expr.queries, vec!["max:mz_views_count{$environment}"]);
        assert_eq!(expr.formula, "query1");
    }

    #[test]
    fn functions_move_to_the_formula() {
        let expr = Expression::parse(
            "default_zero( count_not_null( avg:m{$environment AND NOT r:r1} by {c,r} ) )",
        )
        .unwrap();
        assert_eq!(
            expr.queries,
            vec!["avg:m{$environment AND NOT r:r1} by {c,r}"]
        );
        assert_eq!(expr.formula, "default_zero(count_not_null(query1))");
    }

    #[test]
    fn modifiers_stay_on_the_query() {
        let expr =
            Expression::parse("per_second(sum:m{a:b} by {pod}.as_rate().rollup(sum, 60))").unwrap();
        assert_eq!(
            expr.queries,
            vec!["sum:m{a:b} by {pod}.as_rate().rollup(sum, 60)"]
        );
        assert_eq!(expr.formula, "per_second(query1)");
    }

    #[test]
    fn each_operand_gets_a_name_and_a_repeat_reuses_it() {
        let expr =
            Expression::parse("sum:a{x:1} by {n} / sum:b{x:1} by {n} * 100 + sum:a{x:1} by {n}")
                .unwrap();
        assert_eq!(expr.queries, vec!["sum:a{x:1} by {n}", "sum:b{x:1} by {n}"]);
        assert_eq!(expr.formula, "query1 / query2 * 100 + query1");
    }

    #[test]
    fn group_by_functions_and_numbers_stay_in_the_formula() {
        let expr = Expression::parse(
            "max(cutoff_max(max:lag{q:1} by {pod,collection_id}, 1000000000), {pod})",
        )
        .unwrap();
        assert_eq!(expr.queries, vec!["max:lag{q:1} by {pod,collection_id}"]);
        assert_eq!(expr.formula, "max(cutoff_max(query1, 1000000000), {pod})");
    }

    #[test]
    fn percentile_aggregators_are_metric_queries() {
        let expr = Expression::parse("p99:latency{op:cas} by {op}").unwrap();
        assert_eq!(expr.formula, "query1");
    }

    #[test]
    fn an_expression_without_a_metric_query_is_an_error() {
        assert!(Expression::parse("count_not_null(1)").is_err());
    }

    #[test]
    fn combined_expressions_are_named_across_the_request() {
        let rx = Expression::parse("sum:rx{a:b} by {pod}").unwrap();
        let ratio = Expression::parse("sum:tx{a:b} by {pod} / sum:rx{a:b} by {pod}").unwrap();
        let (queries, formulas) = combine(&[(&rx, ()), (&ratio, ())]);
        let queries: Vec<&str> = queries.iter().map(|(q, ())| q.as_str()).collect();
        assert_eq!(
            queries,
            vec!["sum:rx{a:b} by {pod}", "sum:tx{a:b} by {pod}"]
        );
        assert_eq!(formulas, vec!["query1", "query2 / query1"]);
    }

    #[test]
    fn one_query_under_two_keys_gets_two_names() {
        let records = Expression::parse("max:records{a:b} by {id}").unwrap();
        let (queries, formulas) = combine(&[(&records, "min"), (&records, "max")]);
        assert_eq!(queries.len(), 2);
        assert_eq!(formulas, vec!["query1", "query2"]);
    }

    #[test]
    fn a_tag_set_argument_is_a_second_layer() {
        assert!(aggregates_in_layers("count(query1, {size})"));
        assert!(aggregates_in_layers(
            "top(max(cutoff_max(query1, 1e9), {a,b}), 15, 'max', 'desc')"
        ));
        assert!(!aggregates_in_layers(
            "default_zero(count_not_null(query1))"
        ));
        assert!(!aggregates_in_layers("top(query1, 15, 'max', 'desc')"));
    }

    #[test]
    fn bare_means_nothing_around_the_query() {
        assert!(is_bare_metric_query("avg:m{a:b} by {c}.rollup(avg, 60)"));
        assert!(!is_bare_metric_query("count_not_null(avg:m{a:b} by {c})"));
        assert!(!is_bare_metric_query("avg:m{a:b} / 2"));
    }
}
