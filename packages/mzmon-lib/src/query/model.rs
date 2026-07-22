// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The query-registry data model, ported from
//! `py_mzmon_lib.registry.queries`.
//!
//! These are the *resolved* types the registry holds after loading — the raw
//! YAML shapes live in [`crate::query::def`], and behavior (rendering,
//! extraction) is attached in [`crate::query::render`] /
//! [`crate::query::extract`] so this module stays pure data.

use std::fmt;

use indexmap::IndexMap;

use crate::query::importance::Importance;
use crate::query::stability::Stability;

/// A stable identifier for a query, e.g. `materialize.compute.peek_latency.p99`.
pub type QueryId = String;

/// The name of a Prometheus recording-rule group. Groups are defined
/// externally (in the alerting/recording-rule files), so this is just a label.
pub type RuleGroup = String;

/// A backend query language a query can be rendered for.
///
/// This is the *query engine* (Prometheus, Datadog, …), distinct from the
/// *template engine* (this library) that renders a query for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryEngine {
    PromQl,
    Datadog,
    Honeycomb,
    LogQl,
}

impl QueryEngine {
    /// The lowercase wire value (matches the Python `StrEnum` value).
    pub fn as_str(self) -> &'static str {
        match self {
            QueryEngine::PromQl => "promql",
            QueryEngine::Datadog => "datadog",
            QueryEngine::Honeycomb => "honeycomb",
            QueryEngine::LogQl => "logql",
        }
    }
}

impl fmt::Display for QueryEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A template-engine transform applied to a rendered template string.
///
/// Names (`orZero`, `mzClusterName`, …) are resolved to implementations by the
/// [`crate::query::render::TemplateContext`], so the same registry entry renders
/// differently per engine. `args` are themselves template expressions, rendered
/// before the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFunction {
    pub name: String,
    pub args: Vec<TemplateExpr>,
}

/// The object form of a template string.
///
/// Either an inline `template` (with `%%{param}` placeholders) or a reference to
/// another query by `query_id`, optionally wrapped by template-engine
/// `functions` (applied in order). Exactly one of `template` / `query_id` is
/// set — enforced when built from the raw YAML in [`crate::query::def`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateExpr {
    pub template: Option<String>,
    pub query_id: Option<QueryId>,
    pub functions: Vec<TemplateFunction>,
}

impl TemplateExpr {
    /// Build a bare inline-template expression (no functions, no reference).
    pub fn template(s: impl Into<String>) -> Self {
        TemplateExpr {
            template: Some(s.into()),
            query_id: None,
            functions: Vec::new(),
        }
    }

    /// Build a bare query-reference expression.
    pub fn reference(id: impl Into<QueryId>) -> Self {
        TemplateExpr {
            template: None,
            query_id: Some(id.into()),
            functions: Vec::new(),
        }
    }
}

/// Structured, human-readable description for queries and rules.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Description {
    /// A brief summary of the query.
    pub summary: String,
    /// The nominal or expected behavior of the query.
    pub nominal: Option<String>,
    /// The degraded behavior of the query and actions to take.
    pub degraded: Option<String>,
    /// The unhealthy behavior of the query and actions to take.
    pub unhealthy: Option<String>,
    /// Additional notes about the query.
    pub notes: Option<String>,
}

/// A concrete query definition in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Stable identifier for this query.
    pub id: QueryId,
    /// Human-readable description of this query.
    pub description: Description,
    /// Stability level of this query.
    pub stability: Stability,
    /// Importance stamped from the source file's `metricImportanceHint` at load
    /// time. Rolls up (greatest-wins) to each metric this query references.
    pub importance: Importance,
    /// Query ids this query depends on (dependencies are promoted to top-level
    /// registry entries at load time; this records the edge).
    pub dependencies: Vec<QueryId>,

    /// PromQL template(s) for this query (one, or several distinct series).
    pub promql: Vec<TemplateExpr>,
    /// Datadog SQL template(s) for this query.
    pub datadog_sql: Vec<TemplateExpr>,
    /// Honeycomb SQL template(s) for this query.
    pub honeycomb_sql: Vec<TemplateExpr>,
    /// LogQL template(s) for this query.
    pub logql: Vec<TemplateExpr>,
    /// Whether this query is an instant (rather than range) query.
    pub instant: Option<bool>,
}

impl Query {
    /// True if this query has any metric (PromQL / Datadog / Honeycomb)
    /// definition.
    pub fn is_metric_query(&self) -> bool {
        !self.promql.is_empty() || !self.datadog_sql.is_empty() || !self.honeycomb_sql.is_empty()
    }

    /// True if this query has a LogQL definition.
    pub fn is_log_query(&self) -> bool {
        !self.logql.is_empty()
    }

    /// The (unrendered) template value for `engine`, if any.
    pub fn value_for_engine(&self, engine: QueryEngine) -> &[TemplateExpr] {
        match engine {
            QueryEngine::PromQl => &self.promql,
            QueryEngine::Datadog => &self.datadog_sql,
            QueryEngine::Honeycomb => &self.honeycomb_sql,
            QueryEngine::LogQl => &self.logql,
        }
    }
}

/// A concrete recording rule in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub record: String,
    pub description: Description,
    pub group: RuleGroup,
    pub stability: Stability,
    pub query_id: QueryId,
    pub labels: IndexMap<String, String>,
}

/// A concrete alerting rule in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub alert: String,
    pub description: Description,
    pub group: RuleGroup,
    pub stability: Stability,
    pub query_id: QueryId,
    /// The `for` duration (Rust keyword, so the field is `for_`).
    pub for_: String,
    pub keep_firing_for: Option<String>,
    pub labels: IndexMap<String, String>,
    pub annotations: IndexMap<String, String>,
}
