// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Widget builders over the models in [`datadog_api_client`].
//!
//! The crate's models are `#[non_exhaustive]`, built with `new` and a setter per
//! optional field. These builders fix the choices every widget of a kind shares,
//! so a dashboard module names a title and a registry query and nothing else.
//!
//! Widgets carry no `id` and no `layout`. Datadog assigns ids on import, and the
//! dashboards use the ordered layout with automatic reflow, which places widgets
//! in the order they are listed.

use datadog_api_client::datadogV1::model::{
    FormulaAndFunctionMetricAggregation, FormulaAndFunctionMetricDataSource,
    FormulaAndFunctionMetricQueryDefinition, FormulaAndFunctionQueryDefinition,
    FormulaAndFunctionResponseFormat, GroupWidgetDefinition, GroupWidgetDefinitionType,
    NoteWidgetDefinition, NoteWidgetDefinitionType, QueryValueWidgetDefinition,
    QueryValueWidgetDefinitionType, QueryValueWidgetRequest, Widget, WidgetDefinition,
    WidgetFormula, WidgetLayoutType,
};

use super::formula::{Expression, query_name};
use super::queries::PanelQuery;

/// A widget showing one number: the latest value of a registry query, from
/// [`super::queries::Queries::single`].
pub fn query_value(title: &str, query: &PanelQuery) -> Widget {
    value_widget(title, query.expressions.first(), &query.description)
}

/// One number per template, each in its own widget and titled by its label, from
/// [`super::queries::Queries::labeled`].
///
/// A Grafana stat panel draws several values side by side, and a Datadog query
/// value draws one, so a two-template registry entry becomes two widgets.
pub fn query_values(labels: &[&str], query: &PanelQuery) -> Vec<Widget> {
    labels
        .iter()
        .zip(&query.expressions)
        .map(|(label, expr)| value_widget(label, Some(expr), &query.description))
        .collect()
}

/// A query value over one expression: its metric queries, each reduced to its
/// latest value, and its formula over them.
fn value_widget(title: &str, expr: Option<&Expression>, description: &str) -> Widget {
    let (queries, formulas) = match expr {
        Some(expr) => (
            expr.queries
                .iter()
                .enumerate()
                .map(|(index, query)| {
                    metric_query(index, query, FormulaAndFunctionMetricAggregation::LAST)
                })
                .collect(),
            vec![WidgetFormula::new(expr.formula.clone())],
        ),
        None => (Vec::new(), Vec::new()),
    };
    let request = QueryValueWidgetRequest::new()
        .queries(queries)
        .formulas(formulas)
        .response_format(FormulaAndFunctionResponseFormat::SCALAR);
    let mut definition =
        QueryValueWidgetDefinition::new(vec![request], QueryValueWidgetDefinitionType::QUERY_VALUE)
            .title(title.to_string())
            .autoscale(true)
            .precision(0);
    if !description.is_empty() {
        definition = definition.description(description.to_string());
    }
    Widget::new(WidgetDefinition::QueryValueWidgetDefinition(Box::new(
        definition,
    )))
}

/// A titled group, the Datadog counterpart of a Grafana row.
pub fn group(title: &str, widgets: Vec<Widget>) -> Widget {
    let definition = GroupWidgetDefinition::new(
        WidgetLayoutType::ORDERED,
        GroupWidgetDefinitionType::GROUP,
        widgets,
    )
    .title(title.to_string());
    Widget::new(WidgetDefinition::GroupWidgetDefinition(Box::new(
        definition,
    )))
}

/// A Markdown note, for text a reader needs before the panels.
pub fn note(markdown: &str) -> Widget {
    let definition =
        NoteWidgetDefinition::new(markdown.to_string(), NoteWidgetDefinitionType::NOTE)
            .background_color("yellow".to_string());
    Widget::new(WidgetDefinition::NoteWidgetDefinition(Box::new(definition)))
}

/// A metric query, named for the widget's formulas.
fn metric_query(
    index: usize,
    expr: &str,
    aggregator: FormulaAndFunctionMetricAggregation,
) -> FormulaAndFunctionQueryDefinition {
    FormulaAndFunctionQueryDefinition::FormulaAndFunctionMetricQueryDefinition(Box::new(
        FormulaAndFunctionMetricQueryDefinition::new(
            FormulaAndFunctionMetricDataSource::METRICS,
            query_name(index),
            expr.to_string(),
        )
        .aggregator(aggregator),
    ))
}
