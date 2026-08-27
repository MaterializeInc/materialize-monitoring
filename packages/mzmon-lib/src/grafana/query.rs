// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Bridge from the query registry to Grafana dashboard query groups.
//!
//! A registry [`Query`] carries one template per query engine plus a structured
//! [`Description`]. A dashboard panel wants a [`dashboardv2::QueryGroupKind`] and
//! a markdown description string. [`panel_query`] is the whole trip: look an id
//! up in the registry, render it for the engine the [`TemplateContext`] names,
//! and hand back both halves.
//!
//! ```no_run
//! use mzmon_lib::grafana::query::panel_query;
//! use mzmon_lib::query::{QueryEngine, QueryRegistry, TemplateContext};
//!
//! # fn main() -> mzmon_lib::query::Result<()> {
//! # let registry = QueryRegistry::new();
//! let ctx = TemplateContext::new(QueryEngine::PromQl).with_registry(&registry);
//! let panel = panel_query(
//!     &registry,
//!     "materialize.health.environment.availability.percentage",
//!     &ctx,
//! )?;
//! // panel.query_group goes into PanelSpec::data; panel.description into
//! // PanelSpec::description.
//! # Ok(())
//! # }
//! ```
//!
//! The engine in the context is what picks the datasource: [`QueryEngine::PromQl`]
//! builds Prometheus dataqueries, [`QueryEngine::LogQl`] builds Loki ones. Datadog
//! and Honeycomb have no Grafana datasource here and are rejected rather than
//! silently rendered into a Prometheus query.

use crate::grafana::generated::{
    LOKI_PLUGIN_ID, PROMETHEUS_PLUGIN_ID, dashboardv2, loki, prometheus,
};
use crate::query::{
    Description, Error, Query, QueryEngine, QueryRegistry, Result, TemplateContext,
};

/// Dashboard variable holding the metrics (Prometheus/Thanos) datasource uid.
pub const METRICS_DATASOURCE_VAR: &str = "metricsDatasource";

/// Dashboard variable holding the logs (Loki) datasource uid.
pub const LOGS_DATASOURCE_VAR: &str = "logsDatasource";

/// `DataQueryKind::version` for datasource dataqueries. Not carried by
/// `packages.json` -- that manifest records plugin identity, not the per-kind
/// API version -- and the pre-rendered dashboards all use `v0`.
const DATAQUERY_VERSION: &str = "v0";

/// `DataQueryKind::kind` discriminant.
const DATA_QUERY_KIND: &str = "DataQuery";
/// `PanelQueryKind::kind` discriminant.
const PANEL_QUERY_KIND: &str = "PanelQuery";
/// `QueryGroupKind::kind` discriminant.
const QUERY_GROUP_KIND: &str = "QueryGroup";

/// A registry query rendered into the two things a panel needs.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelQuery {
    /// Goes into `PanelSpec::data`.
    pub query_group: dashboardv2::QueryGroupKind,
    /// Markdown, goes into `PanelSpec::description`.
    pub description: String,
}

/// Look `id` up in `registry`, render it for the context's engine, and build the
/// query group and description for a panel.
///
/// A query with several templates for the engine (a list-valued `promQL`) becomes
/// several dataqueries in one group, which is how a panel shows several series.
pub fn panel_query(
    registry: &QueryRegistry,
    id: &str,
    ctx: &TemplateContext,
) -> Result<PanelQuery> {
    let query = registry
        .get(id)
        .ok_or_else(|| Error::NoSuchQuery(id.to_string()))?;
    panel_query_for(query, ctx)
}

/// [`panel_query`] against a [`Query`] already in hand.
pub fn panel_query_for(query: &Query, ctx: &TemplateContext) -> Result<PanelQuery> {
    let exprs = query.render(ctx)?;
    let data_queries = exprs
        .into_iter()
        .map(|expr| data_query(&expr, ctx.engine, query.instant))
        .collect::<Result<Vec<_>>>()?;

    Ok(PanelQuery {
        query_group: query_group(data_queries),
        description: format_description(&query.description),
    })
}

/// Wrap dataqueries in a query group, assigning each a `refId`.
///
/// The `query-{i}` naming matches what the Python implementation emits, so a
/// dashboard regenerated in Rust does not churn every refId. Grafana only
/// requires refIds be unique within the group.
pub fn query_group(data_queries: Vec<dashboardv2::DataQueryKind>) -> dashboardv2::QueryGroupKind {
    let queries = data_queries
        .into_iter()
        .enumerate()
        .map(|(i, query)| dashboardv2::PanelQueryKind {
            kind: PANEL_QUERY_KIND.to_string(),
            spec: dashboardv2::PanelQuerySpec {
                hidden: false,
                query,
                ref_id: format!("query-{i}"),
            },
        })
        .collect();

    dashboardv2::QueryGroupKind {
        kind: QUERY_GROUP_KIND.to_string(),
        spec: dashboardv2::QueryGroupSpec {
            queries,
            query_options: dashboardv2::QueryOptionsSpec::default(),
            transformations: Vec::new(),
        },
    }
}

/// Build the dataquery for one rendered expression, picking the datasource from
/// the engine it was rendered for.
pub fn data_query(
    expr: &str,
    engine: QueryEngine,
    instant: Option<bool>,
) -> Result<dashboardv2::DataQueryKind> {
    match engine {
        QueryEngine::PromQl => Ok(promql_data_query(expr, METRICS_DATASOURCE_VAR, instant)),
        QueryEngine::LogQl => Ok(logql_data_query(expr, LOGS_DATASOURCE_VAR, instant)),
        // Rendering succeeded, but there is no Grafana datasource for these --
        // emitting the expression as a Prometheus query would produce a dashboard
        // that looks fine and returns nothing.
        QueryEngine::Datadog | QueryEngine::Honeycomb => Err(Error::MissingExpression {
            id: String::new(),
            engine: engine.to_string(),
        }),
    }
}

/// A Prometheus dataquery against `${<datasource_var>}`.
pub fn promql_data_query(
    expr: &str,
    datasource_var: &str,
    instant: Option<bool>,
) -> dashboardv2::DataQueryKind {
    let dataquery = prometheus::Dataquery {
        expr: expr.to_string(),
        instant,
        // Everything below is left to Grafana's own defaults. Spelled out rather
        // than `..Default::default()` because the generated type has no Default
        // (`expr` is required with no schema default), and spelling it out is
        // what makes an upstream field addition a compile error here instead of
        // a silent behavior change.
        adhoc_filters: Vec::new(),
        datasource: None,
        editor_mode: None,
        exemplar: None,
        format: None,
        group_by_keys: Vec::new(),
        hide: None,
        interval: None,
        interval_factor: None,
        interval_ms: None,
        legend_format: None,
        max_data_points: None,
        query_type: None,
        range: None,
        ref_id: None,
        result_assertions: None,
        scopes: Vec::new(),
        time_range: None,
    };

    let mut spec = to_spec(&dataquery);
    add_prometheus_compat_fields(&mut spec, dataquery.format.as_ref());

    data_query_kind(PROMETHEUS_PLUGIN_ID, datasource_var, spec)
}

/// A Loki dataquery against `${<datasource_var>}`.
pub fn logql_data_query(
    expr: &str,
    datasource_var: &str,
    instant: Option<bool>,
) -> dashboardv2::DataQueryKind {
    let dataquery = loki::Dataquery {
        expr: expr.to_string(),
        instant,
        // See the note in `promql_data_query` on spelling these out.
        datasource: None,
        direction: None,
        editor_mode: None,
        hide: None,
        legend_format: None,
        max_lines: None,
        query_type: None,
        range: None,
        ref_id: None,
        resolution: None,
        step: None,
    };

    data_query_kind(LOKI_PLUGIN_ID, datasource_var, to_spec(&dataquery))
}

/// Grafana's Prometheus datasource reads `query` and `qryType`, while the schema
/// cog generates calls them `expr` and `queryType`. Sending only the schema
/// spelling loses data on push, so the Python implementation emits both and we
/// match it -- see `py_mzmon_lib.query_v2.CompatPrometheusDataQuery`.
///
/// `qryType` encodes the *format*: 1 time series, 2 table, 3 heatmap. (The Python
/// maps from `queryType` rather than `format` while comparing against `format`
/// values, which is almost certainly a slip; both are unset in every dashboard we
/// render, so the emitted value is 1 either way.)
fn add_prometheus_compat_fields(
    spec: &mut serde_json::Map<String, serde_json::Value>,
    format: Option<&prometheus::PromQueryFormat>,
) {
    if let Some(expr) = spec.get("expr").cloned() {
        spec.insert("query".to_string(), expr);
    }
    let qry_type = match format {
        None | Some(prometheus::PromQueryFormat::TimeSeries) => 1,
        Some(prometheus::PromQueryFormat::Table) => 2,
        Some(prometheus::PromQueryFormat::Heatmap) => 3,
    };
    spec.insert("qryType".to_string(), serde_json::json!(qry_type));
}

/// Wrap a datasource-specific spec in the dashboard's `DataQueryKind` envelope.
fn data_query_kind(
    plugin_id: &str,
    datasource_var: &str,
    spec: serde_json::Map<String, serde_json::Value>,
) -> dashboardv2::DataQueryKind {
    dashboardv2::DataQueryKind {
        datasource: Some(dashboardv2::DataQueryKindDatasource {
            name: Some(format!("${{{datasource_var}}}")),
        }),
        group: plugin_id.to_string(),
        kind: DATA_QUERY_KIND.to_string(),
        labels: Default::default(),
        spec: Some(spec),
        version: DATAQUERY_VERSION.to_string(),
    }
}

/// Erase a typed dataquery into the free-form map the dashboard schema wants.
///
/// The dashboard document types `DataQueryKind::spec` as an open object, so this
/// erasure is forced by the schema rather than chosen -- the same bargain as
/// panel options in `VizConfigSpec`.
fn to_spec<T: serde::Serialize>(value: &T) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map,
        // The dataquery types are plain structs; they cannot serialize to
        // anything but an object.
        _ => unreachable!("dataquery did not serialize to a JSON object"),
    }
}

/// Render a registry [`Description`] as panel-description markdown.
///
/// The pre-rendered dashboards open a description with its summary in bold and
/// continue in prose, so that is the shape here: bold summary, then the
/// behavioral fields as labeled paragraphs, then notes unlabeled.
///
/// Registry prose is hard-wrapped in YAML, so each field is unwrapped to a single
/// paragraph. Markdown would collapse those newlines anyway; doing it here keeps
/// the emitted YAML readable.
pub fn format_description(description: &Description) -> String {
    let mut blocks = Vec::new();

    let summary = unwrap_prose(&description.summary);
    if !summary.is_empty() {
        blocks.push(format!("**{summary}**"));
    }
    for (label, value) in [
        ("Nominal", &description.nominal),
        ("Degraded", &description.degraded),
        ("Unhealthy", &description.unhealthy),
    ] {
        if let Some(text) = value.as_deref().map(unwrap_prose).filter(|t| !t.is_empty()) {
            blocks.push(format!("**{label}:** {text}"));
        }
    }
    if let Some(notes) = description
        .notes
        .as_deref()
        .map(unwrap_prose)
        .filter(|t| !t.is_empty())
    {
        blocks.push(notes);
    }

    blocks.join("\n\n")
}

/// Collapse a hard-wrapped YAML block scalar into one paragraph, preserving
/// deliberate blank-line paragraph breaks.
fn unwrap_prose(text: &str) -> String {
    text.split("\n\n")
        .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::model::TemplateExpr;
    use crate::query::{Importance, Stability};

    fn query(id: &str, promql: &[&str]) -> Query {
        Query {
            id: id.to_string(),
            description: Description {
                summary: "A summary\nwrapped over lines.\n".to_string(),
                nominal: Some("All good.\n".to_string()),
                degraded: None,
                unhealthy: Some("Very bad.\n".to_string()),
                notes: Some("A note.\n".to_string()),
            },
            stability: Stability::BestEffort,
            importance: Importance::default(),
            dependencies: Vec::new(),
            promql: promql.iter().map(|p| TemplateExpr::template(*p)).collect(),
            datadog_query: Vec::new(),
            honeycomb_sql: Vec::new(),
            logql: Vec::new(),
            instant: Some(true),
        }
    }

    #[test]
    fn renders_a_single_expression_into_one_dataquery() {
        let ctx = TemplateContext::new(QueryEngine::PromQl);
        let panel = panel_query_for(&query("q", &["up"]), &ctx).expect("render");

        assert_eq!(panel.query_group.kind, "QueryGroup");
        assert_eq!(panel.query_group.spec.queries.len(), 1);

        let panel_query = &panel.query_group.spec.queries[0];
        assert_eq!(panel_query.kind, "PanelQuery");
        assert_eq!(panel_query.spec.ref_id, "query-0");
        assert!(!panel_query.spec.hidden);

        let data = &panel_query.spec.query;
        assert_eq!(data.kind, "DataQuery");
        assert_eq!(data.group, "prometheus");
        assert_eq!(data.version, "v0");
        assert_eq!(
            data.datasource.as_ref().and_then(|d| d.name.as_deref()),
            Some("${metricsDatasource}")
        );

        let spec = data.spec.as_ref().expect("spec");
        assert_eq!(spec["expr"], "up");
        // Compat fields the Grafana Prometheus datasource actually reads.
        assert_eq!(spec["query"], "up");
        assert_eq!(spec["qryType"], 1);
        assert_eq!(spec["instant"], true);
    }

    #[test]
    fn a_multi_template_query_becomes_several_dataqueries() {
        let ctx = TemplateContext::new(QueryEngine::PromQl);
        let panel = panel_query_for(&query("q", &["up", "down"]), &ctx).expect("render");

        let ref_ids: Vec<&str> = panel
            .query_group
            .spec
            .queries
            .iter()
            .map(|q| q.spec.ref_id.as_str())
            .collect();
        assert_eq!(ref_ids, vec!["query-0", "query-1"]);
    }

    #[test]
    fn logql_renders_against_the_logs_datasource() {
        let mut q = query("q", &[]);
        q.logql = vec![TemplateExpr::template("{app=\"mz\"}")];
        let ctx = TemplateContext::new(QueryEngine::LogQl);
        let panel = panel_query_for(&q, &ctx).expect("render");

        let data = &panel.query_group.spec.queries[0].spec.query;
        assert_eq!(data.group, "loki");
        assert_eq!(
            data.datasource.as_ref().and_then(|d| d.name.as_deref()),
            Some("${logsDatasource}")
        );
        let spec = data.spec.as_ref().expect("spec");
        assert_eq!(spec["expr"], "{app=\"mz\"}");
        // The Prometheus compat fields are Prometheus-specific; Loki reads `expr`.
        assert!(!spec.contains_key("qryType"));
        assert!(!spec.contains_key("query"));
    }

    #[test]
    fn engines_without_a_grafana_datasource_are_rejected() {
        let mut q = query("q", &[]);
        q.datadog_query = vec![TemplateExpr::template("avg:up{*}")];
        let ctx = TemplateContext::new(QueryEngine::Datadog);
        assert!(panel_query_for(&q, &ctx).is_err());
    }

    #[test]
    fn an_unknown_id_names_itself() {
        let registry = QueryRegistry::new();
        let ctx = TemplateContext::new(QueryEngine::PromQl);
        let err = panel_query(&registry, "nope.not.here", &ctx).expect_err("should fail");
        assert!(
            err.to_string().contains("nope.not.here"),
            "unhelpful error: {err}"
        );
    }

    #[test]
    fn description_unwraps_and_labels() {
        let formatted = format_description(&query("q", &[]).description);
        assert_eq!(
            formatted,
            "**A summary wrapped over lines.**\n\n\
             **Nominal:** All good.\n\n\
             **Unhealthy:** Very bad.\n\n\
             A note."
        );
    }

    #[test]
    fn description_keeps_deliberate_paragraph_breaks() {
        let description = Description {
            summary: "First para\nwrapped.\n\nSecond para.\n".to_string(),
            ..Default::default()
        };
        assert_eq!(
            format_description(&description),
            "**First para wrapped.\n\nSecond para.**"
        );
    }

    #[test]
    fn an_empty_description_is_empty() {
        assert_eq!(format_description(&Description::default()), "");
    }
}
