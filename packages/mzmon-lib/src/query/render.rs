// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Rendering a query for one query engine.
//!
//! Ported from the template-engine half of `py_mzmon_lib.registry.queries`:
//! [`TemplateContext`], `_substitute_params`, `_render_template_string`, and
//! `Query.render`. A [`TemplateContext`] carries everything needed to render for
//! one engine — the concrete `%%{param}` values, the template-engine functions
//! (`orZero`, …), and the registry to resolve `queryId` references against — so
//! one registry entry renders differently per target (a Grafana dashboard, a
//! static Google Cloud Monitoring dashboard, the docs).

use std::collections::HashMap;

use crate::query::error::{Error, Result};
use crate::query::model::{Query, QueryEngine, TemplateExpr};
use crate::query::registry::QueryRegistry;

/// A template-engine transform: `fn(base, rendered_args) -> String`. Supplied
/// per engine by a [`TemplateContext`]; **not** a query-engine function.
pub type TemplateFn = Box<dyn Fn(&str, &[String]) -> String>;

/// Everything needed to render a query for one query engine.
///
/// `parameters` maps `knownParameter` names to their rendered values (e.g.
/// `interval` -> `[$__rate_interval]` for Grafana, `[5m]` for a static
/// dashboard); `functions` supplies the template-engine transforms for this
/// engine; and `registry` resolves `queryId` references. The lifetime ties the
/// context to the registry it resolves against.
pub struct TemplateContext<'a> {
    pub engine: QueryEngine,
    pub parameters: HashMap<String, String>,
    pub functions: HashMap<String, TemplateFn>,
    pub registry: Option<&'a QueryRegistry>,
}

impl<'a> TemplateContext<'a> {
    /// Build a context with no parameters or functions for `engine`.
    pub fn new(engine: QueryEngine) -> Self {
        TemplateContext {
            engine,
            parameters: HashMap::new(),
            functions: HashMap::new(),
            registry: None,
        }
    }

    /// Attach a registry so `queryId` references resolve.
    pub fn with_registry(mut self, registry: &'a QueryRegistry) -> Self {
        self.registry = Some(registry);
        self
    }
}

impl Query {
    /// Render this query for the context's query engine.
    ///
    /// Returns one expression per template the query defines for that engine (a
    /// list-valued `promQL` renders to several expressions). Errors if the query
    /// has no expression for the engine, if a `%%{param}` is unset, or if a
    /// referenced function / query is missing.
    pub fn render(&self, ctx: &TemplateContext) -> Result<Vec<String>> {
        let value = self.value_for_engine(ctx.engine);
        if value.is_empty() {
            return Err(Error::MissingExpression {
                id: self.id.clone(),
                engine: ctx.engine.to_string(),
            });
        }
        value.iter().map(|ts| render_expr(ts, ctx)).collect()
    }
}

/// Render a single [`TemplateExpr`] to a concrete expression.
fn render_expr(ts: &TemplateExpr, ctx: &TemplateContext) -> Result<String> {
    let mut base = if let Some(template) = &ts.template {
        substitute_params(template, ctx)?
    } else {
        // A `queryId` reference. Resolve it against the registry and embed its
        // single rendered expression.
        //
        // NOTE: the Python original has a latent bug here — it rejects *every*
        // reference because `Query.render` always returns a list — so this path
        // was never actually exercised. We implement the clearly-intended
        // behavior (embed a single-expression query; error only when the
        // referenced query renders several). No registry query uses `queryId`
        // today, so this does not affect extraction parity.
        let query_id = ts.query_id.as_ref().ok_or(Error::InvalidTemplateExpr)?;
        let registry = ctx
            .registry
            .ok_or_else(|| Error::NoResolver(query_id.clone()))?;
        let referenced = registry
            .get(query_id)
            .ok_or_else(|| Error::UnknownQuery(query_id.clone()))?;
        let mut rendered = referenced.render(ctx)?;
        if rendered.len() != 1 {
            return Err(Error::MultipleExpressions(query_id.clone()));
        }
        rendered.remove(0)
    };

    for func in &ts.functions {
        let implementation =
            ctx.functions
                .get(&func.name)
                .ok_or_else(|| Error::UnknownFunction {
                    name: func.name.clone(),
                    known: ctx.functions.keys().cloned().collect(),
                })?;
        let rendered_args = func
            .args
            .iter()
            .map(|arg| render_expr(arg, ctx))
            .collect::<Result<Vec<_>>>()?;
        base = implementation(&base, &rendered_args);
    }
    Ok(base)
}

/// Replace every `%%{name}` placeholder in `template` with its context value.
///
/// A hand-rolled scan equivalent to the Python `re.sub(r"%%\{([A-Za-z0-9_]+)\}",
/// …)`: a `%%{` not followed by `[A-Za-z0-9_]+}` is left verbatim (not a
/// placeholder). Errors if a matched name has no value, mirroring the Python
/// `KeyError`.
fn substitute_params(template: &str, ctx: &TemplateContext) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(pos) = rest.find("%%{") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 3..];
        // A name is a run of `[A-Za-z0-9_]`; those are all ASCII, so counting
        // bytes is a valid char boundary for the slice below.
        let name_len = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        let name = &after[..name_len];
        let tail = &after[name_len..];
        if !name.is_empty() && tail.starts_with('}') {
            match ctx.parameters.get(name) {
                Some(value) => out.push_str(value),
                None => {
                    return Err(Error::MissingParameter {
                        name: name.to_string(),
                        known: ctx.parameters.keys().cloned().collect(),
                    });
                }
            }
            rest = &tail[1..];
        } else {
            // Not a valid placeholder: emit the literal `%%{` and continue past
            // it. No new `%%{` can begin inside those three bytes, so this stays
            // equivalent to the regex's one-char advance.
            out.push_str("%%{");
            rest = after;
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Harden `base` to yield `0` rather than an empty result. Port of
/// `_promql_or_zero`.
pub fn promql_or_zero(base: &str, _args: &[String]) -> String {
    format!("({base}) or vector(0)")
}

/// Placeholder rendered for `%%{mzSqlPrefix}` by [`tier_context`]. It is a valid
/// PromQL name prefix (so extraction parses cleanly) that never collides with a
/// real metric; [`crate::query::tiers`] rewrites it to [`SQL_PREFIX_REGEX`] in
/// the emitted allowlist so the SQL-exporter metrics match under either prefix.
pub const SQL_PREFIX_SENTINEL: &str = "__mz_sql_prefix__";

/// The anchored-regex fragment the [`SQL_PREFIX_SENTINEL`] rewrites to: matches
/// both self-managed's `mz_` prefix and Cloud's `v2_mz_` one.
///
/// `v2_` is **not** a version: it is the prefix on one part of the Cloud
/// platform's SQL-based metric endpoints, and it does not exist on self-managed.
/// The two prefixes are two deployments, not two generations of one metric.
pub const SQL_PREFIX_REGEX: &str = "(?:v2_)?mz_";

/// Shared builder for the extraction contexts. Only `mz_sql_prefix` varies: it
/// is the sole parameter that affects extracted *metric names* (every other
/// parameter fills label matchers / ranges, which extraction ignores).
fn extraction_context<'a>(
    registry: &'a QueryRegistry,
    engine: QueryEngine,
    mz_sql_prefix: &str,
) -> TemplateContext<'a> {
    let parameters = [
        ("interval", "[51m]"),
        ("range", "[42m]"),
        ("mzSqlPrefix", mz_sql_prefix),
        (
            "mzEnvironmentFilter",
            r#"materialize_cloud_organization_name=~"your-env-name""#,
        ),
        (
            "mzEnvironmentNamespaceFilter",
            r#"namespace=~"materialize-environment""#,
        ),
        ("mzOperatorNamespaceFilter", r#"namespace=~"materialize""#),
        ("mzClusterList", ".+"),
        ("mzReplicaList", ".+"),
        ("mzNamespaceList", "materialize-environment"),
        (
            "cAdvisorFilter",
            r#"container!="POD", container!="", namespace=~"materialize-environment""#,
        ),
        // Empty for extraction: an environment-exclusion fragment would only add
        // spurious label matchers to the parsed selectors. A real deployment
        // fills this in (e.g. mz_context_org_type!="e2e_test").
        ("excludeEnvironmentFilter", ""),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    let mut functions: HashMap<String, TemplateFn> = HashMap::new();
    functions.insert("orZero".to_string(), Box::new(promql_or_zero));
    // Identity: extraction sees the raw metric, not the enrichment join. The
    // real per-engine implementations (id->name joins, per-environment grouping)
    // live in the rendering context that targets that engine.
    for name in ["mzClusterName", "mzObjectName", "mzEnvironmentName"] {
        functions.insert(
            name.to_string(),
            Box::new(|base: &str, _args: &[String]| base.to_string()),
        );
    }

    TemplateContext {
        engine,
        parameters,
        functions,
        registry: Some(registry),
    }
}

/// The documentation [`TemplateContext`], matching `query_cli.doc_context`.
///
/// Parameters use deliberately recognizable sentinel values (`interval` ->
/// `[51m]`, `mzSqlPrefix` -> `v2_mz_`, …) so rendered docs are obviously
/// examples. Crucially for extraction, the id->name enrichment functions
/// (`mzClusterName` / `mzObjectName`) are the identity here — the docs show the
/// raw metric, not the `mz_object_info` / `mz_cluster_info` left joins — so the
/// extracted metric set is exactly the metrics the queries name directly.
pub fn doc_context<'a>(
    registry: &'a QueryRegistry,
    engine: QueryEngine,
    sql_metric_prefix: &str,
) -> TemplateContext<'a> {
    extraction_context(registry, engine, sql_metric_prefix)
}

/// The metric-tiers [`TemplateContext`], used by [`crate::query::tiers`] to build
/// per-destination allowlists.
///
/// Identical to [`doc_context`] except `%%{mzSqlPrefix}` renders to
/// [`SQL_PREFIX_SENTINEL`] rather than a concrete prefix. The sentinel is a valid
/// PromQL name (extraction parses it) that the tiers projection later rewrites to
/// [`SQL_PREFIX_REGEX`] — so a SQL-exporter metric lands in the allowlist as a
/// fragment matching both `mz_…` (self-managed) and `v2_mz_…` (Cloud), and works
/// regardless of which endpoint a deployment scrapes.
pub fn tier_context<'a>(registry: &'a QueryRegistry, engine: QueryEngine) -> TemplateContext<'a> {
    extraction_context(registry, engine, SQL_PREFIX_SENTINEL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::importance::Importance;
    use crate::query::model::{Description, TemplateFunction};
    use crate::query::stability::Stability;

    fn ctx_with_params(pairs: &[(&str, &str)]) -> TemplateContext<'static> {
        let mut ctx = TemplateContext::new(QueryEngine::PromQl);
        for (k, v) in pairs {
            ctx.parameters.insert(k.to_string(), v.to_string());
        }
        ctx
    }

    #[test]
    fn substitutes_placeholders() {
        let ctx = ctx_with_params(&[("mzSqlPrefix", "v2_mz_"), ("mzClusterList", ".+")]);
        let out = substitute_params(
            r#"%%{mzSqlPrefix}compute_cluster_status{compute_cluster_id=~"%%{mzClusterList}"}"#,
            &ctx,
        )
        .unwrap();
        assert_eq!(
            out,
            r#"v2_mz_compute_cluster_status{compute_cluster_id=~".+"}"#
        );
    }

    #[test]
    fn missing_parameter_errors() {
        let ctx = ctx_with_params(&[]);
        let err = substitute_params("%%{nope}", &ctx).unwrap_err();
        assert!(matches!(err, Error::MissingParameter { .. }));
    }

    #[test]
    fn malformed_placeholder_is_left_verbatim() {
        let ctx = ctx_with_params(&[("foo", "X")]);
        // No closing brace, and a bare `%%{` with an invalid name char, stay put.
        assert_eq!(substitute_params("a %%{ b", &ctx).unwrap(), "a %%{ b");
        assert_eq!(substitute_params("%%{}", &ctx).unwrap(), "%%{}");
        // Overlapping: the inner valid placeholder still resolves.
        assert_eq!(substitute_params("%%{%%{foo}", &ctx).unwrap(), "%%{X");
    }

    #[test]
    fn applies_functions_in_order() {
        let query = Query {
            id: "q".to_string(),
            description: Description::default(),
            stability: Stability::BestEffort,
            importance: Importance::Recommended,
            dependencies: vec![],
            promql: vec![TemplateExpr {
                template: Some("m{}".to_string()),
                query_id: None,
                functions: vec![TemplateFunction {
                    name: "orZero".to_string(),
                    args: vec![],
                }],
            }],
            datadog_query: vec![],
            honeycomb_sql: vec![],
            logql: vec![],
            instant: None,
        };
        let registry = QueryRegistry::new();
        let ctx = doc_context(&registry, QueryEngine::PromQl, "v2_mz_");
        assert_eq!(query.render(&ctx).unwrap(), vec!["(m{}) or vector(0)"]);
    }

    #[test]
    fn missing_expression_for_engine_errors() {
        let query = Query {
            id: "q".to_string(),
            description: Description::default(),
            stability: Stability::BestEffort,
            importance: Importance::Recommended,
            dependencies: vec![],
            promql: vec![TemplateExpr::template("m{}")],
            datadog_query: vec![],
            honeycomb_sql: vec![],
            logql: vec![],
            instant: None,
        };
        let ctx = TemplateContext::new(QueryEngine::LogQl);
        assert!(matches!(
            query.render(&ctx).unwrap_err(),
            Error::MissingExpression { .. }
        ));
    }

    #[test]
    fn query_id_reference_embeds_single_expression() {
        let mut registry = QueryRegistry::new();
        registry.overload_query(Query {
            id: "base".to_string(),
            description: Description::default(),
            stability: Stability::BestEffort,
            importance: Importance::Recommended,
            dependencies: vec![],
            promql: vec![TemplateExpr::template("inner{}")],
            datadog_query: vec![],
            honeycomb_sql: vec![],
            logql: vec![],
            instant: None,
        });
        registry.overload_query(Query {
            id: "wrapper".to_string(),
            description: Description::default(),
            stability: Stability::BestEffort,
            importance: Importance::Recommended,
            dependencies: vec![],
            promql: vec![TemplateExpr {
                template: None,
                query_id: Some("base".to_string()),
                functions: vec![TemplateFunction {
                    name: "orZero".to_string(),
                    args: vec![],
                }],
            }],
            datadog_query: vec![],
            honeycomb_sql: vec![],
            logql: vec![],
            instant: None,
        });
        let ctx = doc_context(&registry, QueryEngine::PromQl, "v2_mz_");
        let wrapper = registry.get("wrapper").unwrap();
        assert_eq!(
            wrapper.render(&ctx).unwrap(),
            vec!["(inner{}) or vector(0)"]
        );
    }
}
