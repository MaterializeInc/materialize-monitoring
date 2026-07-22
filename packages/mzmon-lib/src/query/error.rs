// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Errors for the query registry. Mirrors `scrape::error` / `alloy::error` but
//! scoped to this subsystem so the three stay decoupled.

/// The query-engine expression that was missing / referenced when an error
/// occurred. Kept as an owned `String` rather than [`crate::query::model::QueryEngine`]
/// so error values are cheap to move and don't borrow the model.
pub type EngineName = String;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A registry file declared the same query `id` twice (or a dependency
    /// re-registered an existing id). Registration requires unique ids; use
    /// [`crate::query::registry::QueryRegistry::overload_query`] to overwrite on
    /// purpose.
    #[error("query id {0:?} is already registered")]
    DuplicateQuery(String),

    /// Ditto for a recording-rule `record` name.
    #[error("rule {0:?} is already registered")]
    DuplicateRule(String),

    /// Ditto for an alert `alert` name.
    #[error("alert {0:?} is already registered")]
    DuplicateAlert(String),

    /// A [`crate::query::model::TemplateExpr`] had neither, or both, of
    /// `template` / `queryId` set — exactly one is required.
    #[error("template expression requires exactly one of `template` or `queryId`")]
    InvalidTemplateExpr,

    /// A `%%{name}` placeholder had no value in the render context.
    #[error(
        "template parameter %%{{{name}}} has no value in this context (known: {})",
        format_list(known)
    )]
    MissingParameter { name: String, known: Vec<String> },

    /// A template referenced a function the context does not implement.
    #[error(
        "template function {name:?} is not implemented by this context (known: {})",
        format_list(known)
    )]
    UnknownFunction { name: String, known: Vec<String> },

    /// The query has no expression for the requested engine.
    #[error("query {id:?} has no {engine} expression")]
    MissingExpression { id: String, engine: EngineName },

    /// A `queryId` template reference pointed at a query with no context to
    /// resolve it against.
    #[error("template references query {0:?} but the context has no registry to resolve it")]
    NoResolver(String),

    /// A `queryId` reference resolved to a query that renders multiple
    /// expressions, which cannot be embedded as a single template reference.
    #[error(
        "query {0:?} renders multiple expressions and cannot be embedded as a single reference"
    )]
    MultipleExpressions(String),

    /// A referenced `queryId` is not present in the registry.
    #[error("template references unknown query {0:?}")]
    UnknownQuery(String),

    /// `promql-parser` failed to parse a rendered PromQL expression.
    #[error("failed to parse PromQL: {message}\n--- expression ---\n{expr}")]
    PromQlParse { expr: String, message: String },

    /// A `metricPattern` on a metric override was not a valid regex.
    #[error("invalid metricPattern {pattern:?}: {message}")]
    InvalidPattern { pattern: String, message: String },

    /// A JSONSchema violation (used by [`crate::query::validate`]).
    #[error("schema violation at `{path}`: {message}")]
    Schema { path: String, message: String },

    #[error("{}", format_multiple(.0))]
    Multiple(Vec<Error>),
}

/// Render a sorted, comma-joined list for the "known: …" hints.
fn format_list(items: &[String]) -> String {
    let mut sorted: Vec<&String> = items.iter().collect();
    sorted.sort();
    sorted
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a `Multiple` as a header plus an indented bullet per child error.
/// (Same shape as `scrape::error` / `alloy::error` so the subsystems stay
/// independent.)
fn format_multiple(errs: &[Error]) -> String {
    let mut out = format!("{} errors:", errs.len());
    for e in errs {
        for (i, line) in e.to_string().lines().enumerate() {
            out.push_str(if i == 0 { "\n  - " } else { "\n    " });
            out.push_str(line);
        }
    }
    out
}

pub type Result<T> = std::result::Result<T, Error>;
