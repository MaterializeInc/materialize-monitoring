// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Widgets placed under a Grafana panel's slug, with their manifest entry.
//!
//! Each function draws a widget through [`mzmon_lib::datadog::widget`] and
//! records what the parity ledger in `tests/datadog_parity.rs` compares with the
//! Grafana panel of the same slug: its title, registry query, kind, and unit.

use datadog_api_client::datadogV1::model::Widget;
use mzmon_lib::datadog::query::PanelQuery;
use mzmon_lib::datadog::widget::{self, Column, Format, Look};

use crate::manifest::{self, Kind, Unit};

/// What a dashboard drew for one Grafana panel's slug.
#[derive(Debug, Clone)]
pub struct Panel {
    /// The entry the parity ledger compares.
    pub manifest: manifest::Panel,
    /// Usually one widget. A Grafana stat with several values draws one query
    /// value per value, because a Datadog query value draws one number.
    pub widgets: Vec<Widget>,
}

fn panel(
    slug: &str,
    title: &str,
    query: &PanelQuery,
    kind: Kind,
    look: &Look,
    widgets: Vec<Widget>,
) -> Panel {
    Panel {
        manifest: manifest::Panel {
            slug: slug.to_string(),
            title: title.to_string(),
            queries: vec![query.id.clone()],
            kind,
            unit: unit(look.formatting()),
        },
        widgets,
    }
}

/// The manifest's unit for what a widget's numbers measure.
fn unit(format: Format) -> Unit {
    match format {
        Format::Plain => Unit::None,
        Format::Seconds => Unit::Seconds,
        Format::Bytes => Unit::Bytes,
        Format::BytesPerSecond => Unit::BytesPerSecond,
        Format::Percent | Format::Fraction => Unit::Percent,
        Format::Cores => Unit::Cores,
        Format::PerSecond(_) => Unit::PerSecond,
    }
}

/// One number, from [`super::queries::Queries::single`].
pub fn query_value(slug: &str, title: &str, query: &PanelQuery, look: &Look) -> Panel {
    let widget = widget::query_value(title, query, look);
    panel(slug, title, query, Kind::Value, look, vec![widget])
}

/// One number per template, each titled by its label, from
/// [`super::queries::Queries::labeled`].
pub fn query_values(
    slug: &str,
    title: &str,
    labels: &[&str],
    query: &PanelQuery,
    look: &Look,
) -> Panel {
    let widgets = widget::query_values(labels, query, look);
    panel(slug, title, query, Kind::Value, look, widgets)
}

/// One bar per group, for a Grafana stat or gauge that shows a value per series,
/// such as sessions by type.
pub fn grouped_values(slug: &str, title: &str, query: &PanelQuery, look: &Look) -> Panel {
    let widget = widget::toplist(title, query, look, 10);
    panel(slug, title, query, Kind::Value, look, vec![widget])
}

/// The top `count` groups as bars, for a Grafana bar chart.
pub fn ranking(slug: &str, title: &str, query: &PanelQuery, look: &Look, count: i64) -> Panel {
    let widget = widget::toplist(title, query, look, count);
    panel(slug, title, query, Kind::Ranking, look, vec![widget])
}

/// A pie, for a Grafana pie chart.
pub fn pie(slug: &str, title: &str, query: &PanelQuery, look: &Look) -> Panel {
    let widget = widget::sunburst(title, query, look);
    panel(slug, title, query, Kind::Pie, look, vec![widget])
}

/// Every template over time, each aliased by its label when there are labels.
pub fn timeseries(
    slug: &str,
    title: &str,
    query: &PanelQuery,
    labels: &[&str],
    look: &Look,
) -> Panel {
    let widget = widget::timeseries(title, query, labels, look);
    panel(slug, title, query, Kind::TimeSeries, look, vec![widget])
}

/// One row per group, with a column per [`Column`].
pub fn table(
    slug: &str,
    title: &str,
    query: &PanelQuery,
    columns: &[Column],
    look: &Look,
) -> Panel {
    let widget = widget::table(title, query, columns, look);
    panel(slug, title, query, Kind::Table, look, vec![widget])
}

/// A group of panels: one Grafana row.
pub fn group(title: &str, color: &str, show_title: bool, panels: &[Panel]) -> Widget {
    let widgets = panels
        .iter()
        .flat_map(|p| p.widgets.iter().cloned())
        .collect();
    widget::group(title, color, show_title, widgets)
}
