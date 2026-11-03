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
//! so a dashboard names a title, a registry query, and a [`Look`].
//!
//! Widgets carry no `layout`: the dashboards use the ordered layout with
//! automatic reflow, which places widgets in the order they are listed. Only a
//! dashboard's top-level groups carry an `id`, because its tabs name them by id.
//!
//! # Scalar widgets
//!
//! A query value, toplist, sunburst, or table reads each metric query reduced
//! over the dashboard's time window, through Datadog's scalar query API. Each
//! query's `aggregator` does that reduction, except under a group-by function
//! such as `max(query1, {tag})`, which aggregates in two layers. Datadog refuses
//! an aggregator there, so the formula reduces itself, as
//! `reduce(max(query1, {tag}), 'last')`, and its queries carry none.

use datadog_api_client::datadogV1::model::{
    FormulaAndFunctionMetricAggregation, FormulaAndFunctionMetricDataSource,
    FormulaAndFunctionMetricQueryDefinition, FormulaAndFunctionQueryDefinition,
    FormulaAndFunctionResponseFormat, GroupWidgetDefinition, GroupWidgetDefinitionType,
    NoteWidgetDefinition, NoteWidgetDefinitionType, NumberFormatUnit, NumberFormatUnitCanonical,
    NumberFormatUnitScaleType, QuerySortOrder, QueryValueWidgetDefinition,
    QueryValueWidgetDefinitionType, QueryValueWidgetRequest, SunburstWidgetDefinition,
    SunburstWidgetDefinitionType, SunburstWidgetLegend, SunburstWidgetLegendTable,
    SunburstWidgetLegendTableType, SunburstWidgetRequest, TableWidgetDefinition,
    TableWidgetDefinitionType, TableWidgetRequest, TimeseriesWidgetDefinition,
    TimeseriesWidgetDefinitionType, TimeseriesWidgetLegendLayout, TimeseriesWidgetRequest,
    ToplistWidgetDefinition, ToplistWidgetDefinitionType, ToplistWidgetRequest, Widget, WidgetAxis,
    WidgetComparator, WidgetConditionalFormat, WidgetDefinition, WidgetDisplayType, WidgetFormula,
    WidgetFormulaLimit, WidgetLayoutType, WidgetLegacyLiveSpan, WidgetLiveSpan, WidgetNumberFormat,
    WidgetPalette, WidgetTime,
};

use super::formula::{Expression, aggregates_in_layers, combine, query_name};
use super::query::PanelQuery;

/// How a scalar widget reduces each metric query over the time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reduce {
    Last,
    Avg,
    Max,
    Min,
    /// The total over the window, for a count such as `.as_count()`.
    Sum,
}

impl Reduce {
    fn aggregation(self) -> FormulaAndFunctionMetricAggregation {
        match self {
            Reduce::Last => FormulaAndFunctionMetricAggregation::LAST,
            Reduce::Avg => FormulaAndFunctionMetricAggregation::AVG,
            Reduce::Max => FormulaAndFunctionMetricAggregation::MAX,
            Reduce::Min => FormulaAndFunctionMetricAggregation::MIN,
            Reduce::Sum => FormulaAndFunctionMetricAggregation::SUM,
        }
    }

    /// The reducer's name in a formula's `reduce(…, 'last')`.
    fn name(self) -> &'static str {
        match self {
            Reduce::Last => "last",
            Reduce::Avg => "avg",
            Reduce::Max => "max",
            Reduce::Min => "min",
            Reduce::Sum => "sum",
        }
    }
}

/// What a widget's numbers measure, and how Datadog labels them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A count or ratio, unlabeled.
    Plain,
    Seconds,
    Bytes,
    BytesPerSecond,
    /// Already 0–100.
    Percent,
    /// A fraction of 1, drawn as a percent by scaling the formula by 100.
    Fraction,
    Cores,
    /// Events per second, named by a Datadog unit such as `query`.
    PerSecond(&'static str),
}

impl Format {
    /// Datadog's canonical unit and per-unit names.
    fn canonical(self) -> Option<(&'static str, Option<&'static str>)> {
        match self {
            Format::Plain => None,
            Format::Seconds => Some(("second", None)),
            Format::Bytes => Some(("byte", None)),
            Format::BytesPerSecond => Some(("byte", Some("second"))),
            Format::Percent | Format::Fraction => Some(("percent", None)),
            Format::Cores => Some(("core", None)),
            Format::PerSecond(unit) => Some((unit, Some("second"))),
        }
    }

    fn number_format(self) -> Option<WidgetNumberFormat> {
        let (unit, per) = self.canonical()?;
        let mut canonical = NumberFormatUnitCanonical::new()
            .type_(NumberFormatUnitScaleType::CANONICAL_UNIT)
            .unit_name(unit.to_string());
        if let Some(per) = per {
            canonical = canonical.per_unit_name(per.to_string());
        }
        Some(
            WidgetNumberFormat::new().unit(NumberFormatUnit::NumberFormatUnitCanonical(Box::new(
                canonical,
            ))),
        )
    }

    fn formula(self, formula: &str) -> String {
        match self {
            Format::Fraction => format!("({formula}) * 100"),
            _ => formula.to_string(),
        }
    }
}

/// The presentation a builder leaves to the panel.
#[derive(Debug, Clone)]
pub struct Look {
    reduce: Reduce,
    format: Format,
    precision: Option<i64>,
    log_scale: bool,
    min_zero: bool,
    instant: bool,
    colors: Vec<(WidgetComparator, f64, WidgetPalette)>,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            reduce: Reduce::Last,
            format: Format::Plain,
            precision: None,
            log_scale: false,
            min_zero: false,
            instant: false,
            colors: Vec::new(),
        }
    }
}

impl Look {
    pub fn new() -> Self {
        Look::default()
    }

    /// How a scalar widget reduces over the window. Last, unless set.
    pub fn reduce(mut self, reduce: Reduce) -> Self {
        self.reduce = reduce;
        self
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// What the widget's numbers measure, as [`Look::format`] set it.
    pub fn formatting(&self) -> Format {
        self.format
    }

    /// Decimal places on a query value. Whole numbers, unless set.
    pub fn precision(mut self, precision: i64) -> Self {
        self.precision = Some(precision);
        self
    }

    /// A log y-axis, for values that span orders of magnitude.
    pub fn log_scale(mut self) -> Self {
        self.log_scale = true;
        self
    }

    /// A y-axis that starts at 0.
    pub fn min_zero(mut self) -> Self {
        self.min_zero = true;
        self
    }

    /// Read the last five minutes rather than the dashboard's time window, for a
    /// registry query marked `instant`.
    ///
    /// An instant PromQL query reads each series' latest sample within a
    /// five-minute lookback, so a series that stopped reporting drops out. A
    /// scalar widget's `last` over the dashboard's window keeps a stopped
    /// series' final value instead. The cost is that the widget always shows the
    /// present: it doesn't follow the time picker into the past.
    pub fn instant(mut self) -> Self {
        self.instant = true;
        self
    }

    /// Green from `healthy`, yellow from `degraded`, red below it.
    ///
    /// The rules are listed most demanding first, because a value takes the
    /// first one it meets.
    pub fn healthy_from(mut self, degraded: f64, healthy: f64) -> Self {
        self.colors = vec![
            (
                WidgetComparator::GREATER_THAN_OR_EQUAL_TO,
                healthy,
                WidgetPalette::WHITE_ON_GREEN,
            ),
            (
                WidgetComparator::GREATER_THAN_OR_EQUAL_TO,
                degraded,
                WidgetPalette::WHITE_ON_YELLOW,
            ),
            (
                WidgetComparator::LESS_THAN,
                degraded,
                WidgetPalette::WHITE_ON_RED,
            ),
        ];
        self
    }

    /// Red from `bad`, yellow from `warning`, green below it.
    pub fn unhealthy_from(mut self, warning: f64, bad: f64) -> Self {
        self.colors = vec![
            (
                WidgetComparator::GREATER_THAN_OR_EQUAL_TO,
                bad,
                WidgetPalette::WHITE_ON_RED,
            ),
            (
                WidgetComparator::GREATER_THAN_OR_EQUAL_TO,
                warning,
                WidgetPalette::WHITE_ON_YELLOW,
            ),
            (
                WidgetComparator::LESS_THAN,
                warning,
                WidgetPalette::WHITE_ON_GREEN,
            ),
        ];
        self
    }

    fn formula(&self, formula: &str, alias: Option<&str>) -> WidgetFormula {
        let mut built = WidgetFormula::new(self.format.formula(formula));
        if let Some(alias) = alias {
            built = built.alias(alias.to_string());
        }
        if let Some(number_format) = self.format.number_format() {
            built = built.number_format(number_format);
        }
        built
    }

    fn time(&self) -> Option<WidgetTime> {
        self.instant.then(|| {
            WidgetTime::WidgetLegacyLiveSpan(Box::new(
                WidgetLegacyLiveSpan::new().live_span(WidgetLiveSpan::PAST_FIVE_MINUTES),
            ))
        })
    }

    fn yaxis(&self) -> Option<WidgetAxis> {
        match (self.log_scale, self.min_zero) {
            (false, false) => None,
            (true, _) => Some(WidgetAxis::new().scale("log".to_string())),
            (false, true) => Some(WidgetAxis::new().min("0".to_string())),
        }
    }
}

/// One number: a registry query's first template, reduced over the window.
pub fn query_value(title: &str, query: &PanelQuery, look: &Look) -> Widget {
    value_widget(title, query.expressions.first(), &query.description, look)
}

/// One number per template, each in its own widget and titled by its label, in
/// order.
///
/// A Grafana stat panel draws several values side by side, and a Datadog query
/// value draws one, so a two-template registry entry becomes two widgets.
pub fn query_values(labels: &[&str], query: &PanelQuery, look: &Look) -> Vec<Widget> {
    labels
        .iter()
        .zip(&query.expressions)
        .map(|(label, expr)| value_widget(label, Some(expr), &query.description, look))
        .collect()
}

fn value_widget(title: &str, expr: Option<&Expression>, description: &str, look: &Look) -> Widget {
    let (queries, formulas) = scalar_parts(expr.into_iter().map(|e| (e, look.reduce, None)), look);
    let request = QueryValueWidgetRequest::new()
        .queries(queries)
        .formulas(formulas)
        .response_format(FormulaAndFunctionResponseFormat::SCALAR);
    let mut definition =
        QueryValueWidgetDefinition::new(vec![request], QueryValueWidgetDefinitionType::QUERY_VALUE)
            .title(title.to_string())
            .autoscale(true)
            .precision(look.precision.unwrap_or(0));
    if !description.is_empty() {
        definition = definition.description(description.to_string());
    }
    if let Some(time) = look.time() {
        definition = definition.time(time);
    }
    Widget::new(WidgetDefinition::QueryValueWidgetDefinition(Box::new(
        definition,
    )))
}

/// The top `count` groups of a registry query's first template, as bars.
///
/// It draws a Grafana stat or gauge that shows a value per series, such as
/// sessions by type, as well as a Grafana bar chart.
pub fn toplist(title: &str, query: &PanelQuery, look: &Look, count: i64) -> Widget {
    let (queries, formulas) = scalar_parts(
        query.expressions.first().map(|e| (e, look.reduce, None)),
        look,
    );
    let formulas = formulas
        .into_iter()
        .map(|f| {
            f.limit(
                WidgetFormulaLimit::new()
                    .count(count)
                    .order(QuerySortOrder::DESC),
            )
        })
        .collect();
    let request = ToplistWidgetRequest::new()
        .queries(queries)
        .formulas(formulas)
        .response_format(FormulaAndFunctionResponseFormat::SCALAR);
    let mut definition =
        ToplistWidgetDefinition::new(vec![request], ToplistWidgetDefinitionType::TOPLIST)
            .title(title.to_string());
    if !query.description.is_empty() {
        definition = definition.description(query.description.clone());
    }
    if let Some(time) = look.time() {
        definition = definition.time(time);
    }
    Widget::new(WidgetDefinition::ToplistWidgetDefinition(Box::new(
        definition,
    )))
}

/// Parts of a whole, a Datadog pie chart: one slice per group of a registry
/// query's first template, with a table of the groups and their values beside
/// it.
///
/// The automatic legend shows nothing on most pies at a dashboard's widget size,
/// so a reader couldn't tell the slices apart.
pub fn sunburst(title: &str, query: &PanelQuery, look: &Look) -> Widget {
    let (queries, formulas) = scalar_parts(
        query.expressions.first().map(|e| (e, look.reduce, None)),
        look,
    );
    let request = SunburstWidgetRequest::new()
        .queries(queries)
        .formulas(formulas)
        .response_format(FormulaAndFunctionResponseFormat::SCALAR);
    let mut definition =
        SunburstWidgetDefinition::new(vec![request], SunburstWidgetDefinitionType::SUNBURST)
            .title(title.to_string())
            .legend(SunburstWidgetLegend::SunburstWidgetLegendTable(Box::new(
                SunburstWidgetLegendTable::new(SunburstWidgetLegendTableType::TABLE),
            )));
    if !query.description.is_empty() {
        definition = definition.description(query.description.clone());
    }
    if let Some(time) = look.time() {
        definition = definition.time(time);
    }
    Widget::new(WidgetDefinition::SunburstWidgetDefinition(Box::new(
        definition,
    )))
}

/// Every template over time, as lines, each aliased by its label when there are
/// labels.
///
/// The templates share one request, so their queries are named across all of
/// them.
pub fn timeseries(title: &str, query: &PanelQuery, labels: &[&str], look: &Look) -> Widget {
    let parts: Vec<(&Expression, ())> = query.expressions.iter().map(|e| (e, ())).collect();
    let (queries, formulas) = combine(&parts);
    let queries = queries
        .iter()
        .enumerate()
        .map(|(index, (q, ()))| metric_query(index, q, None))
        .collect();
    let formulas = formulas
        .iter()
        .enumerate()
        .map(|(index, f)| look.formula(f, labels.get(index).copied()))
        .collect();
    let request = TimeseriesWidgetRequest::new()
        .queries(queries)
        .formulas(formulas)
        .response_format(FormulaAndFunctionResponseFormat::TIMESERIES)
        .display_type(WidgetDisplayType::LINE);
    let mut definition =
        TimeseriesWidgetDefinition::new(vec![request], TimeseriesWidgetDefinitionType::TIMESERIES)
            .title(title.to_string())
            .show_legend(true)
            .legend_layout(TimeseriesWidgetLegendLayout::AUTO);
    if let Some(yaxis) = look.yaxis() {
        definition = definition.yaxis(yaxis);
    }
    if !query.description.is_empty() {
        definition = definition.description(query.description.clone());
    }
    Widget::new(WidgetDefinition::TimeseriesWidgetDefinition(Box::new(
        definition,
    )))
}

/// A table column: one of the query's templates, reduced over the window.
#[derive(Debug, Clone, Copy)]
pub struct Column {
    pub label: &'static str,
    /// The template's position in the registry entry's `datadogQuery` list.
    pub template: usize,
    pub reduce: Reduce,
}

impl Column {
    /// The first template, reduced as `reduce`.
    pub const fn of(label: &'static str, reduce: Reduce) -> Self {
        Column {
            label,
            template: 0,
            reduce,
        }
    }

    /// Template `template`, reduced as `reduce`.
    pub const fn nth(label: &'static str, template: usize, reduce: Reduce) -> Self {
        Column {
            label,
            template,
            reduce,
        }
    }
}

/// One row per group, with a column per [`Column`].
///
/// Every column has to group by the same tags, or Datadog can't line their rows
/// up.
pub fn table(title: &str, query: &PanelQuery, columns: &[Column], look: &Look) -> Widget {
    let (queries, formulas) = scalar_parts(
        columns.iter().filter_map(|column| {
            query
                .expressions
                .get(column.template)
                .map(|e| (e, column.reduce, Some(column.label)))
        }),
        look,
    );
    // Every row, not the default ten: these list what exists.
    let formulas = formulas
        .into_iter()
        .enumerate()
        .map(|(index, f)| match index {
            0 => f.limit(
                WidgetFormulaLimit::new()
                    .count(500)
                    .order(QuerySortOrder::DESC),
            ),
            _ => f,
        })
        .collect();
    let request = TableWidgetRequest::new()
        .queries(queries)
        .formulas(formulas)
        .response_format(FormulaAndFunctionResponseFormat::SCALAR);
    let mut definition =
        TableWidgetDefinition::new(vec![request], TableWidgetDefinitionType::QUERY_TABLE)
            .title(title.to_string());
    if !query.description.is_empty() {
        definition = definition.description(query.description.clone());
    }
    if let Some(time) = look.time() {
        definition = definition.time(time);
    }
    Widget::new(WidgetDefinition::TableWidgetDefinition(Box::new(
        definition,
    )))
}

/// The metric queries and formulas of a scalar request, each expression reduced
/// as given and aliased when it has a label.
///
/// An expression that aggregates in two layers reduces in its formula rather
/// than in its queries' aggregators; see the module docs.
fn scalar_parts<'a>(
    parts: impl IntoIterator<Item = (&'a Expression, Reduce, Option<&'a str>)>,
    look: &Look,
) -> (Vec<FormulaAndFunctionQueryDefinition>, Vec<WidgetFormula>) {
    let parts: Vec<_> = parts.into_iter().collect();
    // The key is the queries' aggregator, which a layered formula's queries lack.
    let keyed: Vec<(&Expression, Option<Reduce>)> = parts
        .iter()
        .map(|(e, r, _)| (*e, (!aggregates_in_layers(&e.formula)).then_some(*r)))
        .collect();
    let (queries, formulas) = combine(&keyed);
    let queries = queries
        .iter()
        .enumerate()
        .map(|(index, (q, reduce))| metric_query(index, q, reduce.map(Reduce::aggregation)))
        .collect();
    let formulas = formulas
        .iter()
        .zip(&parts)
        .map(|(f, (expr, reduce, label))| {
            let f = match aggregates_in_layers(&expr.formula) {
                true => format!("reduce({f}, '{}')", reduce.name()),
                false => f.clone(),
            };
            let mut formula = look.formula(&f, *label);
            if !look.colors.is_empty() {
                formula = formula.conditional_formats(
                    look.colors
                        .iter()
                        .map(|(comparator, value, palette)| {
                            WidgetConditionalFormat::new(
                                comparator.clone(),
                                palette.clone(),
                                *value,
                            )
                        })
                        .collect(),
                );
            }
            formula
        })
        .collect();
    (queries, formulas)
}

/// A group, the Datadog counterpart of a Grafana row, in one of Datadog's group
/// colors, such as `vivid_blue`.
///
/// `show_title` is false for a row whose header Grafana hides, such as a tab's
/// leading summary.
pub fn group(title: &str, color: &str, show_title: bool, widgets: Vec<Widget>) -> Widget {
    let definition = GroupWidgetDefinition::new(
        WidgetLayoutType::ORDERED,
        GroupWidgetDefinitionType::GROUP,
        widgets,
    )
    .title(title.to_string())
    .show_title(show_title)
    .background_color(color.to_string());
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

/// A metric query, named for the widget's formulas. A scalar widget reduces it
/// over the window by `aggregator`; a timeseries takes none.
fn metric_query(
    index: usize,
    expr: &str,
    aggregator: Option<FormulaAndFunctionMetricAggregation>,
) -> FormulaAndFunctionQueryDefinition {
    let mut query = FormulaAndFunctionMetricQueryDefinition::new(
        FormulaAndFunctionMetricDataSource::METRICS,
        query_name(index),
        expr.to_string(),
    );
    if let Some(aggregator) = aggregator {
        query = query.aggregator(aggregator);
    }
    FormulaAndFunctionQueryDefinition::FormulaAndFunctionMetricQueryDefinition(Box::new(query))
}
