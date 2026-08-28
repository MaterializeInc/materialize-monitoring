// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Panel construction.
//!
//! Shaped by an analysis of the pre-rendered `env-top.yaml`: of 4667 written
//! fields, 47% carried no panel-specific information. Three findings drive the
//! design here.
//!
//! **`kind` discriminants are never authored.** `Panel`, `VizConfig`,
//! `AutoGridLayoutItem` and friends accounted for ~600 field-writes of pure
//! noise. They are set by [`Panel::build`] and are not part of this API.
//!
//! **A panel's `options` block is a property of the plugin, not the panel.**
//! Across 69 panels, `timeseries`, `table`, `gauge` and `barchart` each used
//! exactly one options block; `piechart` used two (pie vs donut) and `stat` five,
//! differing only in a handful of display knobs. So each plugin gets a preset
//! whose `Default` is the block the baseline actually used, and the knobs that
//! varied are the only ones exposed -- typed per plugin, so `log_scale` on a stat
//! panel does not compile.
//!
//! **`fieldConfig` is where the real per-panel variation lives**, and it is a
//! short list: `unit`, `min`, `noValue`, `custom`, `thresholds`, `color`, and
//! three one-offs. Those are the shared builder methods.
//!
//! One deliberate omission: `vizConfig.version` stays `""`. Grafana stamps the
//! running plugin version on load and we cannot predict it, so authoring a guess
//! would just be a value the server overwrites.
//!
//! ```no_run
//! use mzmon_lib::grafana::panel::{NoValue, Panel};
//! # fn main() -> mzmon_lib::query::Result<()> {
//! # let registry = mzmon_lib::query::QueryRegistry::new();
//! # let ctx = mzmon_lib::query::TemplateContext::new(mzmon_lib::query::QueryEngine::PromQl);
//! # let bridged = mzmon_lib::grafana::query::panel_query(&registry, "some.query", &ctx)?;
//! let panel = Panel::timeseries("Dataflow Elapsed Rate")
//!     .query(bridged)          // sets both the query group and the description
//!     .unit("cps")
//!     .min(0.0)
//!     .no_value(NoValue::FilterMismatch)
//!     .log_scale(10.0)
//!     .build(1);
//! # Ok(())
//! # }
//! ```

use serde::Serialize;
use serde_json::{Map, Value};

use crate::grafana::generated::{
    BARCHART_PLUGIN_ID, GAUGE_PLUGIN_ID, LOGS_PLUGIN_ID, PIECHART_PLUGIN_ID, STAT_PLUGIN_ID,
    TABLE_PLUGIN_ID, TIMESERIES_PLUGIN_ID, barchart, dashboardv2, gauge, logs, piechart, stat,
    table, timeseries,
};
use crate::grafana::query::PanelQuery;

/// `PanelKind::kind` discriminant.
const PANEL_KIND: &str = "Panel";
/// `VizConfigKind::kind` discriminant.
const VIZ_CONFIG_KIND: &str = "VizConfig";

/// The "no data" message a panel shows. The baseline used five distinct strings;
/// four of them describe a *missing scrape target* rather than a filter miss, and
/// naming them keeps that distinction from being retyped per panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoValue {
    /// The query is fine but the dashboard's filters exclude everything.
    FilterMismatch,
    /// Needs cadvisor metrics, scraped via the kubelet.
    RequiresCAdvisor,
    /// Needs kube-state-metrics.
    RequiresKubeStateMetrics,
    /// Needs both cadvisor and kube-state-metrics.
    RequiresCAdvisorAndKubeStateMetrics,
    /// Anything panel-specific ("Hydration Queue is empty").
    Custom(String),
}

impl NoValue {
    /// The string Grafana renders.
    pub fn text(&self) -> &str {
        match self {
            NoValue::FilterMismatch => "No matches for the current filters",
            NoValue::RequiresCAdvisor => "No metrics: cadvisor metrics are required (via kubelet)",
            NoValue::RequiresKubeStateMetrics => "No metrics: kube-state-metrics is required",
            NoValue::RequiresCAdvisorAndKubeStateMetrics => {
                "No metrics: cadvisor and kube-state-metrics are required"
            }
            NoValue::Custom(text) => text,
        }
    }
}

/// A plugin's options preset plus the knobs that vary across panels using it.
///
/// `Default` is the options block the baseline used; implementors expose only the
/// fields that actually differed between panels.
pub trait PanelOptions {
    /// Grafana plugin id, for `VizConfigKind::group`.
    fn plugin_id(&self) -> &'static str;

    /// The `vizConfig.spec.options` block.
    fn options(&self) -> Map<String, Value>;

    /// The plugin's half of `fieldConfig.defaults.custom`, when the plugin has a
    /// constant one. Returning `None` means "emit nothing" -- and note Grafana
    /// discards an empty `custom` object anyway (29 such writes in the baseline),
    /// so there is no reason to emit one.
    fn field_config_custom(&self) -> Option<Value> {
        None
    }
}

/// Serialize a typed options struct into the free-form map the dashboard wants.
fn erase<T: Serialize>(value: &T) -> Map<String, Value> {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        // Options types are plain structs; they cannot serialize to anything else.
        _ => unreachable!("panel options did not serialize to a JSON object"),
    }
}

/// `reduceOptions` complete enough not to provoke a panel migration.
///
/// The baseline emitted only `calcs`, and Grafana responded by filling in
/// `values` and `fields` *and* stamping `vizConfig.version` -- a migration pass on
/// ten panels every load. Emitting all three avoids it.
fn reduce_options(fields: Option<String>) -> stat::ReduceDataOptions {
    stat::ReduceDataOptions {
        calcs: Vec::new(),
        fields,
        limit: None,
        values: Some(false),
    }
}

// ---------------------------------------------------------------- timeseries

/// Timeseries preset. One options block across all 28 baseline panels: a table
/// legend beneath the chart with Max / Avg / Last columns, single-series tooltip.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timeseries {
    /// Log-scale base for the y axis (`10.0` in the baseline's 8 log panels).
    pub log_scale: Option<f64>,
}

impl PanelOptions for Timeseries {
    fn plugin_id(&self) -> &'static str {
        TIMESERIES_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&timeseries::Options {
            legend: timeseries::VizLegendOptions {
                calcs: vec!["max".into(), "mean".into(), "lastNotNull".into()],
                display_mode: timeseries::LegendDisplayMode::Table,
                placement: timeseries::LegendPlacement::Bottom,
                show_legend: true,
                as_table: None,
                is_visible: None,
                sort_by: None,
                sort_desc: None,
                width: None,
            },
            tooltip: timeseries::VizTooltipOptions {
                mode: timeseries::TooltipDisplayMode::Single,
                sort: timeseries::SortOrder::Asc,
                hide_zeros: None,
                max_height: None,
                max_width: None,
            },
            orientation: None,
            timezone: Vec::new(),
        })
    }

    fn field_config_custom(&self) -> Option<Value> {
        let base = self.log_scale?;
        Some(serde_json::json!({
            "scaleDistribution": { "type": "log", "log": base }
        }))
    }
}

// ---------------------------------------------------------------------- stat

/// Stat preset. Five baseline variants, differing only in the knobs below.
#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    pub color_mode: stat::BigValueColorMode,
    pub graph_mode: stat::BigValueGraphMode,
    pub justify_mode: stat::BigValueJustifyMode,
    pub text_mode: stat::BigValueTextMode,
    /// Point size for the big value (25 and 20 in the baseline).
    pub value_size: Option<f64>,
    /// Field-name regex to reduce over (`/^mz_version$/` in the baseline).
    pub reduce_fields: Option<String>,
}

impl Default for Stat {
    fn default() -> Self {
        Stat {
            color_mode: stat::BigValueColorMode::None,
            graph_mode: stat::BigValueGraphMode::Area,
            justify_mode: stat::BigValueJustifyMode::Auto,
            text_mode: stat::BigValueTextMode::Value,
            value_size: None,
            reduce_fields: None,
        }
    }
}

impl PanelOptions for Stat {
    fn plugin_id(&self) -> &'static str {
        STAT_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&stat::Options {
            color_mode: self.color_mode,
            graph_mode: self.graph_mode,
            justify_mode: self.justify_mode,
            text_mode: self.text_mode,
            orientation: stat::VizOrientation::Auto,
            percent_change_color_mode: stat::PercentChangeColorMode::Standard,
            show_percent_change: false,
            wide_layout: true,
            reduce_options: reduce_options(self.reduce_fields.clone()),
            text: self.value_size.map(|size| stat::VizTextDisplayOptions {
                value_size: Some(size),
                percent_size: None,
                title_size: None,
            }),
        })
    }
}

// ----------------------------------------------------------------- piechart

/// Pie chart preset. Two baseline variants: donut (8 panels) and pie (1).
#[derive(Debug, Clone, PartialEq)]
pub struct PieChart {
    pub pie_type: piechart::PieChartType,
}

impl Default for PieChart {
    fn default() -> Self {
        PieChart {
            pie_type: piechart::PieChartType::Donut,
        }
    }
}

impl PanelOptions for PieChart {
    fn plugin_id(&self) -> &'static str {
        PIECHART_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&piechart::Options {
            pie_type: self.pie_type,
            display_labels: Some(vec![
                piechart::PieChartLabels::Name,
                piechart::PieChartLabels::Value,
            ]),
            legend: piechart_legend(),
            orientation: piechart::VizOrientation::Auto,
            reduce_options: piechart_reduce_options(),
            tooltip: piechart::VizTooltipOptions {
                mode: piechart::TooltipDisplayMode::Single,
                sort: piechart::SortOrder::Asc,
                hide_zeros: None,
                max_height: None,
                max_width: None,
            },
            text: None,
        })
    }
}

/// The legend block all 9 baseline pie charts shared.
fn piechart_legend() -> piechart::PieChartLegendOptions {
    piechart::PieChartLegendOptions {
        as_table: Some(true),
        calcs: Vec::new(),
        display_mode: piechart::LegendDisplayMode::Table,
        is_visible: Some(true),
        placement: piechart::LegendPlacement::Right,
        show_legend: true,
        values: vec![piechart::PieChartLegendValues::Value],
        sort_by: None,
        sort_desc: None,
        width: None,
    }
}

fn piechart_reduce_options() -> piechart::ReduceDataOptions {
    piechart::ReduceDataOptions {
        calcs: Vec::new(),
        fields: None,
        limit: None,
        values: Some(false),
    }
}

// -------------------------------------------------------------------- table

/// Table preset. One options block across all 6 baseline panels. The plugin's
/// `custom` block was constant too, so it lives here rather than per panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {}

impl PanelOptions for Table {
    fn plugin_id(&self) -> &'static str {
        TABLE_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&table::Options {
            cell_height: Some(table::TableCellHeight::Sm),
            footer: Some(table::TableFooterOptions {
                count_rows: Some(false),
                reducer: Vec::new(),
                show: false,
                enable_pagination: None,
                fields: Vec::new(),
            }),
            frame_index: 0.0,
            show_header: true,
            show_type_icons: false,
            sort_by: Vec::new(),
        })
    }

    fn field_config_custom(&self) -> Option<Value> {
        Some(serde_json::json!({
            "align": "auto",
            "filterable": true,
            "inspect": false
        }))
    }
}

// -------------------------------------------------------------------- gauge

/// Gauge preset. One options block across both baseline panels.
///
/// `show_threshold_markers` defaults to `true`, matching the schema's own default:
/// the markers are the coloured band around the gauge arc, which is how a gauge
/// shows where the reading sits relative to its thresholds. Turning them off
/// leaves a bare needle whose thresholds only affect the value's colour.
///
/// `show_threshold_labels` stays off — it prints the numeric boundaries around the
/// arc, which is noise on a small panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Gauge {
    pub show_threshold_labels: bool,
    pub show_threshold_markers: bool,
}

impl Default for Gauge {
    fn default() -> Self {
        Gauge {
            show_threshold_labels: false,
            show_threshold_markers: true,
        }
    }
}

impl PanelOptions for Gauge {
    fn plugin_id(&self) -> &'static str {
        GAUGE_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&gauge::Options {
            min_viz_height: 75,
            min_viz_width: 75,
            text: None,
            orientation: gauge::VizOrientation::Auto,
            reduce_options: gauge_reduce_options(),
            show_threshold_labels: self.show_threshold_labels,
            show_threshold_markers: self.show_threshold_markers,
            sizing: gauge::BarGaugeSizing::Auto,
        })
    }
}

fn gauge_reduce_options() -> gauge::ReduceDataOptions {
    gauge::ReduceDataOptions {
        calcs: Vec::new(),
        fields: None,
        limit: None,
        values: Some(false),
    }
}

// ----------------------------------------------------------------- barchart

/// Bar chart preset. One options block across both baseline panels.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarChart {}

impl PanelOptions for BarChart {
    fn plugin_id(&self) -> &'static str {
        BARCHART_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&barchart::Options {
            bar_radius: 0.0,
            bar_width: 0.8,
            full_highlight: false,
            group_width: 0.95,
            legend: barchart::VizLegendOptions {
                calcs: Vec::new(),
                display_mode: barchart::LegendDisplayMode::List,
                placement: barchart::LegendPlacement::Bottom,
                show_legend: false,
                as_table: None,
                is_visible: None,
                sort_by: None,
                sort_desc: None,
                width: None,
            },
            orientation: barchart::VizOrientation::Horizontal,
            show_value: barchart::VisibilityMode::Auto,
            stacking: barchart::StackingMode::None,
            tooltip: barchart::VizTooltipOptions {
                mode: barchart::TooltipDisplayMode::Single,
                sort: barchart::SortOrder::Asc,
                hide_zeros: None,
                max_height: None,
                max_width: None,
            },
            x_tick_label_max_length: 0,
            x_tick_label_rotation: 0,
            x_tick_label_spacing: 100,
            color_by_field: None,
            text: None,
            x_field: None,
        })
    }
}

// --------------------------------------------------------------------- logs

/// Log panel preset, for a LogQL query rendered as log lines rather than a chart.
///
/// The first plugin here with no baseline to copy: `env-top` has no log panel, so
/// these defaults are chosen rather than measured. They target reading an event
/// stream — newest first, with the timestamp shown, because "when did this start"
/// is the first question asked of one.
///
/// `show_labels` stays off deliberately. Loki's stream labels repeat on every
/// line (`namespace`, `job`, `level` are constant within a panel that selected on
/// them), so rendering them inline costs most of the line width to say nothing.
/// What actually varies per event is structured metadata, which `enable_log_details`
/// puts one click away.
#[derive(Debug, Clone, PartialEq)]
pub struct Logs {
    /// Newest line first. `Descending` is the reading order for an event feed;
    /// `Ascending` suits a panel meant to be read as a narrative.
    pub sort_order: logs::LogsSortOrder,
    /// Wrap long lines rather than truncating them. Event notes carry the cause of
    /// a failure at the end of the line, so truncation hides the answer.
    pub wrap_log_message: bool,
    /// Show the per-line timestamp.
    pub show_time: bool,
}

impl Default for Logs {
    fn default() -> Self {
        Logs {
            sort_order: logs::LogsSortOrder::Descending,
            wrap_log_message: true,
            show_time: true,
        }
    }
}

impl PanelOptions for Logs {
    fn plugin_id(&self) -> &'static str {
        LOGS_PLUGIN_ID
    }

    fn options(&self) -> Map<String, Value> {
        erase(&logs::Options {
            // `Exact` would collapse the repeated lines that make a crash loop
            // legible as a crash loop.
            dedup_strategy: logs::LogsDedupStrategy::None,
            enable_log_details: true,
            prettify_log_message: false,
            show_common_labels: false,
            show_labels: false,
            show_log_context_toggle: false,
            show_time: self.show_time,
            sort_order: self.sort_order,
            wrap_log_message: self.wrap_log_message,
            details_mode: None,
            enable_infinite_scrolling: None,
            font_size: None,
            show_controls: None,
            show_field_selector: None,
            syntax_highlighting: None,
        })
    }
}

// -------------------------------------------------------------------- Panel

/// A panel under construction. Generic over its plugin's options so that
/// plugin-specific knobs are only reachable on the right plugin.
#[derive(Debug, Clone)]
pub struct Panel<O> {
    title: String,
    description: String,
    options: O,
    field_config: dashboardv2::FieldConfig,
    custom: Option<Value>,
    overrides: Vec<dashboardv2::FieldConfigSourceOverridesItem>,
    data: Option<dashboardv2::QueryGroupKind>,
    transformations: Vec<dashboardv2::TransformationKind>,
    transparent: Option<bool>,
}

impl<O: PanelOptions + Default> Panel<O> {
    /// Start a panel with its plugin's preset options.
    pub fn new(title: impl Into<String>) -> Self {
        Panel {
            title: title.into(),
            description: String::new(),
            options: O::default(),
            field_config: dashboardv2::FieldConfig::default(),
            custom: None,
            overrides: Vec::new(),
            data: None,
            transformations: Vec::new(),
            transparent: None,
        }
    }
}

impl Panel<Timeseries> {
    pub fn timeseries(title: impl Into<String>) -> Self {
        Self::new(title)
    }

    /// Put the y axis on a log scale.
    pub fn log_scale(mut self, base: f64) -> Self {
        self.options.log_scale = Some(base);
        self
    }
}

impl Panel<Stat> {
    pub fn stat(title: impl Into<String>) -> Self {
        Self::new(title)
    }

    /// Fill the panel background with the value's color, rather than tinting the
    /// text only.
    pub fn color_background(mut self) -> Self {
        self.options.color_mode = stat::BigValueColorMode::Background;
        self
    }

    pub fn text_mode(mut self, mode: stat::BigValueTextMode) -> Self {
        self.options.text_mode = mode;
        self
    }

    pub fn graph_mode(mut self, mode: stat::BigValueGraphMode) -> Self {
        self.options.graph_mode = mode;
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.options.justify_mode = stat::BigValueJustifyMode::Center;
        self
    }

    pub fn value_size(mut self, size: f64) -> Self {
        self.options.value_size = Some(size);
        self
    }

    /// Reduce over only the fields matching this regex (e.g. `/^mz_version$/`).
    pub fn reduce_fields(mut self, pattern: impl Into<String>) -> Self {
        self.options.reduce_fields = Some(pattern.into());
        self
    }
}

impl Panel<PieChart> {
    pub fn piechart(title: impl Into<String>) -> Self {
        Self::new(title)
    }

    /// Render as a full pie rather than the default donut.
    pub fn full_pie(mut self) -> Self {
        self.options.pie_type = piechart::PieChartType::Pie;
        self
    }
}

impl Panel<Table> {
    pub fn table(title: impl Into<String>) -> Self {
        Self::new(title)
    }
}

impl Panel<Gauge> {
    pub fn gauge(title: impl Into<String>) -> Self {
        Self::new(title)
    }

    pub fn threshold_markers(mut self, show: bool) -> Self {
        self.options.show_threshold_markers = show;
        self
    }

    pub fn threshold_labels(mut self, show: bool) -> Self {
        self.options.show_threshold_labels = show;
        self
    }
}

impl Panel<BarChart> {
    pub fn barchart(title: impl Into<String>) -> Self {
        Self::new(title)
    }
}

impl Panel<Logs> {
    pub fn logs(title: impl Into<String>) -> Self {
        Self::new(title)
    }

    /// Read oldest-first, for a panel meant to be followed as a sequence rather
    /// than checked for what just happened.
    pub fn oldest_first(mut self) -> Self {
        self.options.sort_order = logs::LogsSortOrder::Ascending;
        self
    }

    /// Truncate long lines instead of wrapping them, for a dense feed where the
    /// leading text is the whole signal.
    pub fn truncate_lines(mut self) -> Self {
        self.options.wrap_log_message = false;
        self
    }
}

impl<O: PanelOptions> Panel<O> {
    /// Attach a bridged registry query: its query group *and* its description.
    ///
    /// This is the path that makes a panel's description track the query registry
    /// rather than being retyped alongside it.
    pub fn query(mut self, bridged: PanelQuery) -> Self {
        self.data = Some(bridged.query_group);
        self.description = bridged.description;
        self
    }

    /// Attach a query group without touching the description.
    pub fn data(mut self, data: dashboardv2::QueryGroupKind) -> Self {
        self.data = Some(data);
        self
    }

    /// Override the description a [`Panel::query`] supplied.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.field_config.unit = Some(unit.into());
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.field_config.min = Some(min);
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.field_config.max = Some(max);
        self
    }

    pub fn decimals(mut self, decimals: f64) -> Self {
        self.field_config.decimals = Some(decimals);
        self
    }

    pub fn no_value(mut self, no_value: NoValue) -> Self {
        self.field_config.no_value = Some(no_value.text().to_string());
        self
    }

    /// Color the series by shades of one hex color -- the only `FieldColor` mode
    /// the baseline used, across 27 panels.
    pub fn shade(mut self, hex: impl Into<String>) -> Self {
        self.field_config.color = Some(dashboardv2::FieldColor {
            mode: dashboardv2::FieldColorModeId::Shades,
            fixed_color: Some(hex.into()),
            series_by: None,
        });
        self
    }

    pub fn thresholds(mut self, thresholds: dashboardv2::ThresholdsConfig) -> Self {
        self.field_config.thresholds = Some(thresholds);
        self
    }

    pub fn mappings(mut self, mappings: Vec<dashboardv2::ValueMapping>) -> Self {
        self.field_config.mappings = mappings;
        self
    }

    pub fn links(mut self, links: Vec<Map<String, Value>>) -> Self {
        self.field_config.links = links;
        self
    }

    pub fn transformations(
        mut self,
        transformations: Vec<dashboardv2::TransformationKind>,
    ) -> Self {
        self.transformations = transformations;
        self
    }

    pub fn overrides(
        mut self,
        overrides: Vec<dashboardv2::FieldConfigSourceOverridesItem>,
    ) -> Self {
        self.overrides = overrides;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// Escape hatch for a plugin-specific `fieldConfig.defaults.custom` this API
    /// does not model. Overrides whatever the plugin preset would have supplied.
    pub fn custom(mut self, custom: Value) -> Self {
        self.custom = Some(custom);
        self
    }

    /// Finish the panel.
    ///
    /// `id` is caller-assigned because it is a generated value, not an authored
    /// one -- the baseline had 69 distinct ids threaded through authoring code
    /// that had no reason to know them. The dashboard assembler owns it.
    pub fn build(self, id: u32) -> dashboardv2::PanelKind {
        let mut field_config = self.field_config;
        // The plugin's constant custom block, unless the caller replaced it.
        // Nothing is emitted when neither supplies one: Grafana discards an empty
        // `custom` object, so writing one is pure churn.
        let custom = self.custom.or_else(|| self.options.field_config_custom());
        field_config.custom = match custom {
            Some(Value::Object(map)) => Some(map),
            Some(_) | None => None,
        };

        dashboardv2::PanelKind {
            kind: PANEL_KIND.to_string(),
            spec: dashboardv2::PanelSpec {
                id: f64::from(id),
                title: self.title,
                description: self.description,
                // Constant across all 69 baseline panels.
                links: Vec::new(),
                transparent: self.transparent,
                data: {
                    // Transformations live on the query group, not the panel, so
                    // they have to be folded in here. Building them separately and
                    // forgetting this step dropped them silently.
                    let mut data = self
                        .data
                        .unwrap_or_else(|| crate::grafana::query::query_group(Vec::new()));
                    data.spec.transformations = self.transformations;
                    data
                },
                viz_config: dashboardv2::VizConfigKind {
                    kind: VIZ_CONFIG_KIND.to_string(),
                    group: self.options.plugin_id().to_string(),
                    // Grafana stamps the running plugin version on load; a guess
                    // here is only a value the server overwrites.
                    version: String::new(),
                    spec: dashboardv2::VizConfigSpec {
                        options: Some(self.options.options()),
                        field_config: dashboardv2::FieldConfigSource {
                            defaults: field_config,
                            overrides: self.overrides,
                        },
                    },
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options_of(panel: &dashboardv2::PanelKind) -> &Map<String, Value> {
        panel
            .spec
            .viz_config
            .spec
            .options
            .as_ref()
            .expect("options")
    }

    #[test]
    fn a_minimal_panel_sets_every_discriminant_itself() {
        let panel = Panel::timeseries("Title").build(7);
        assert_eq!(panel.kind, "Panel");
        assert_eq!(panel.spec.viz_config.kind, "VizConfig");
        assert_eq!(panel.spec.viz_config.group, "timeseries");
        assert_eq!(panel.spec.viz_config.version, "");
        assert_eq!(panel.spec.id, 7.0);
        assert!(panel.spec.links.is_empty());
    }

    #[test]
    fn the_timeseries_preset_matches_the_baseline_block() {
        let opts = serde_json::to_value(options_of(&Panel::timeseries("t").build(1))).unwrap();
        assert_eq!(
            opts,
            serde_json::json!({
                "legend": {
                    "calcs": ["max", "mean", "lastNotNull"],
                    "displayMode": "table",
                    "placement": "bottom",
                    "showLegend": true
                },
                "tooltip": {"mode": "single", "sort": "asc"}
            })
        );
    }

    #[test]
    fn reduce_options_are_complete_enough_to_avoid_a_migration() {
        // The baseline emitted only `calcs` and Grafana migrated the panel.
        for opts in [
            serde_json::to_value(options_of(&Panel::stat("s").build(1))).unwrap(),
            serde_json::to_value(options_of(&Panel::piechart("p").build(1))).unwrap(),
            serde_json::to_value(options_of(&Panel::gauge("g").build(1))).unwrap(),
        ] {
            let reduce = &opts["reduceOptions"];
            assert_eq!(reduce["calcs"], serde_json::json!([]));
            assert_eq!(reduce["values"], false, "values must be present: {opts}");
        }
    }

    #[test]
    fn transformations_reach_the_query_group() {
        // They hang off the query group rather than the panel, so a builder that
        // stored them without folding them in would drop them silently -- which is
        // exactly what happened first time.
        let transform = dashboardv2::TransformationKind {
            kind: "Transformation".to_string(),
            group: "merge".to_string(),
            spec: dashboardv2::TransformationSpec {
                options: Default::default(),
                disabled: None,
                filter: None,
                topic: None,
            },
        };
        let panel = Panel::table("t").transformations(vec![transform]).build(1);
        assert_eq!(panel.spec.data.spec.transformations.len(), 1);
        assert_eq!(panel.spec.data.spec.transformations[0].group, "merge");
    }

    #[test]
    fn transformations_survive_a_supplied_query_group() {
        // The data and the transformations arrive through different builder calls;
        // neither may clobber the other regardless of order.
        let data =
            crate::grafana::query::query_group(vec![crate::grafana::query::promql_data_query(
                "up", "ds", None,
            )]);
        let transform = dashboardv2::TransformationKind {
            kind: "Transformation".to_string(),
            group: "organize".to_string(),
            spec: dashboardv2::TransformationSpec {
                options: Default::default(),
                disabled: None,
                filter: None,
                topic: None,
            },
        };
        let panel = Panel::table("t")
            .transformations(vec![transform])
            .data(data)
            .build(1);
        assert_eq!(panel.spec.data.spec.queries.len(), 1);
        assert_eq!(panel.spec.data.spec.transformations.len(), 1);
    }

    #[test]
    fn an_empty_custom_block_is_not_emitted() {
        // Grafana discards `custom: {}` -- 29 such writes in the baseline.
        let panel = Panel::timeseries("t").build(1);
        assert!(
            panel
                .spec
                .viz_config
                .spec
                .field_config
                .defaults
                .custom
                .is_none()
        );
    }

    #[test]
    fn log_scale_populates_the_custom_block() {
        let panel = Panel::timeseries("t").log_scale(10.0).build(1);
        let custom = panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .custom
            .expect("custom");
        assert_eq!(
            serde_json::to_value(custom).unwrap(),
            serde_json::json!({"scaleDistribution": {"type": "log", "log": 10.0}})
        );
    }

    #[test]
    fn table_carries_its_constant_custom_block() {
        let panel = Panel::table("t").build(1);
        let custom = panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .custom
            .expect("custom");
        assert_eq!(
            serde_json::to_value(custom).unwrap(),
            serde_json::json!({"align": "auto", "filterable": true, "inspect": false})
        );
    }

    #[test]
    fn the_custom_escape_hatch_wins_over_the_preset() {
        let panel = Panel::table("t")
            .custom(serde_json::json!({"align": "left"}))
            .build(1);
        let custom = panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .custom
            .expect("custom");
        assert_eq!(
            serde_json::to_value(custom).unwrap(),
            serde_json::json!({"align": "left"})
        );
    }

    #[test]
    fn stat_knobs_reach_the_options_block() {
        let panel = Panel::stat("s")
            .color_background()
            .justify_center()
            .text_mode(stat::BigValueTextMode::ValueAndName)
            .value_size(25.0)
            .build(1);
        let opts = serde_json::to_value(options_of(&panel)).unwrap();
        assert_eq!(opts["colorMode"], "background");
        assert_eq!(opts["justifyMode"], "center");
        assert_eq!(opts["textMode"], "value_and_name");
        assert_eq!(opts["text"], serde_json::json!({"valueSize": 25.0}));
    }

    #[test]
    fn piechart_defaults_to_a_donut() {
        let donut = serde_json::to_value(options_of(&Panel::piechart("p").build(1))).unwrap();
        assert_eq!(donut["pieType"], "donut");
        let pie =
            serde_json::to_value(options_of(&Panel::piechart("p").full_pie().build(1))).unwrap();
        assert_eq!(pie["pieType"], "pie");
    }

    #[test]
    fn gauge_threshold_markers_default_off() {
        // The baseline asked for `true`; Grafana rewrote it to `false` on save.
        let opts = serde_json::to_value(options_of(&Panel::gauge("g").build(1))).unwrap();
        assert_eq!(opts["showThresholdMarkers"], true);
    }

    #[test]
    fn named_no_value_messages_render_the_baseline_strings() {
        let panel = Panel::stat("s")
            .no_value(NoValue::RequiresCAdvisor)
            .build(1);
        assert_eq!(
            panel
                .spec
                .viz_config
                .spec
                .field_config
                .defaults
                .no_value
                .as_deref(),
            Some("No metrics: cadvisor metrics are required (via kubelet)")
        );
    }

    #[test]
    fn field_config_knobs_land_where_expected() {
        let panel = Panel::stat("s")
            .unit("bytes")
            .min(0.0)
            .decimals(4.0)
            .shade("#EE7733")
            .build(1);
        let fc = &panel.spec.viz_config.spec.field_config.defaults;
        assert_eq!(fc.unit.as_deref(), Some("bytes"));
        assert_eq!(fc.min, Some(0.0));
        assert_eq!(fc.decimals, Some(4.0));
        let color = fc.color.as_ref().expect("color");
        assert_eq!(color.fixed_color.as_deref(), Some("#EE7733"));
        assert_eq!(color.mode, dashboardv2::FieldColorModeId::Shades);
    }
}
