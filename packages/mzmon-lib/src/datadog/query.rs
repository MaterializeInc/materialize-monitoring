// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! A registry query, rendered for a Datadog widget.
//!
//! The Datadog counterpart of [`crate::grafana::query`]: a widget names a
//! registry id, and its `datadogQuery` templates and description come from the
//! registry, rendered in a [`super::context`].

use crate::grafana::query::format_description;
use crate::query::{QueryRegistry, TemplateContext};

use super::formula::Expression;

/// A registry query rendered for one widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelQuery {
    /// The registry id, kept for error messages and the parity manifest.
    pub id: String,
    /// One expression per template the registry entry defines, split into the
    /// metric queries and the formula a widget request carries.
    pub expressions: Vec<Expression>,
    /// The registry description, as Markdown.
    pub description: String,
}

/// Why a registry query didn't render for a widget.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no such query in the registry")]
    NoSuchQuery,

    #[error(transparent)]
    Render(#[from] crate::query::Error),

    /// A template with no metric query in it, which a widget can't draw.
    #[error("{0}")]
    Split(String),
}

/// Render registry query `id` in `ctx`, each template split for a widget.
pub fn panel_query(
    registry: &QueryRegistry,
    id: &str,
    ctx: &TemplateContext,
) -> Result<PanelQuery, Error> {
    let query = registry.get(id).ok_or(Error::NoSuchQuery)?;
    let expressions = query
        .render(ctx)?
        .iter()
        .map(|e| Expression::parse(&one_line(e)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Error::Split)?;
    Ok(PanelQuery {
        id: id.to_string(),
        expressions,
        description: format_description(&query.description),
    })
}

/// Collapse a rendered template onto one line.
///
/// The registry wraps long templates for reading, and a Datadog widget's query
/// field is a single line. Whitespace in a metric query is never significant, so
/// every run of it becomes one space.
pub fn one_line(expr: &str) -> String {
    expr.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrapped_template_renders_on_one_line() {
        assert_eq!(
            one_line("count_not_null(\n  avg:m{a} by {b}\n)\n"),
            "count_not_null( avg:m{a} by {b} )"
        );
    }
}
