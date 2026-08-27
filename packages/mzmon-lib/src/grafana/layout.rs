// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Dashboard layout, and the element/layout pair it assembles.
//!
//! A v2 dashboard keeps panels in a flat `elements` map and describes their
//! arrangement in a separate `layout` tree that references them by name. The two
//! have to agree exactly — a reference with no element silently drops a panel, an
//! element nobody references is invisible — so this module owns both halves and
//! [`Layout::assemble`] is what checks they line up.
//!
//! Nesting follows the pre-rendered baseline: tabs of rows of auto-grids, or rows
//! of auto-grids when a dashboard needs no tabs. The schema also allows
//! `GridLayout` anywhere and arbitrary re-nesting of rows and tabs; none of that
//! is modelled here, since nothing uses it and the generated types remain
//! available for a dashboard that does.
//!
//! **Auto-grid means no partition arithmetic.** A `GridLayout` needs an x/y/w/h
//! per panel; `AutoGridLayout` takes a column cap and flows panels into it, which
//! is what every row of the baseline uses.
//!
//! ```no_run
//! use mzmon_lib::grafana::layout::{AutoGrid, Layout, Row, Tab};
//! use mzmon_lib::grafana::panel::Panel;
//!
//! let layout = Layout::tabs([
//!     Tab::new("Summary").rows([
//!         Row::new("Environment Health").grid(
//!             AutoGrid::new(3)
//!                 .panel("availability-percent", Panel::stat("Environment Availability").build(0))
//!                 .panel("is-healthy", Panel::stat("Healthy").build(0)),
//!         ),
//!     ]),
//! ]);
//! let assembled = layout.assemble().expect("layout is consistent");
//! // assembled.elements -> Dashboard::elements, assembled.layout -> Dashboard::layout
//! ```

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use crate::grafana::generated::dashboardv2;

/// `AutoGridLayoutItemKind::kind` discriminant.
const AUTO_GRID_ITEM_KIND: &str = "AutoGridLayoutItem";
/// `AutoGridLayoutKind::kind` discriminant.
const AUTO_GRID_KIND: &str = "AutoGridLayout";
/// `ElementReference::kind` discriminant.
const ELEMENT_REFERENCE_KIND: &str = "ElementReference";
/// `RowsLayoutKind::kind` discriminant.
const ROWS_KIND: &str = "RowsLayout";
/// `RowsLayoutRowKind::kind` discriminant.
const ROW_KIND: &str = "RowsLayoutRow";
/// `TabsLayoutKind::kind` discriminant.
const TABS_KIND: &str = "TabsLayout";
/// `TabsLayoutTabKind::kind` discriminant.
const TAB_KIND: &str = "TabsLayoutTab";

/// First panel id [`Layout::assemble`] hands out.
///
/// Ids are generated, not authored: the baseline's are sequential `1000..1068`,
/// and Grafana preserves whatever it is given. Starting above zero keeps them
/// visually distinct from an unset field in a hand-inspected diff.
pub const FIRST_PANEL_ID: u32 = 1000;

/// What went wrong assembling a layout.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Two panels claimed the same element name. The later one would have
    /// silently replaced the earlier in the elements map.
    #[error("duplicate element name {0:?}: two panels cannot share one name")]
    DuplicateElement(String),

    /// A layout with no panels renders as an empty dashboard, which is almost
    /// always a mistake rather than an intent.
    #[error("layout contains no panels")]
    Empty,
}

/// Result of assembling a layout.
pub type Result<T> = std::result::Result<T, Error>;

/// Column width, as the auto-grid's discrete modes plus an explicit width.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ColumnWidth {
    Narrow,
    #[default]
    Standard,
    Wide,
    /// Explicit width; sets the mode to `custom`.
    Custom(f64),
}

/// Row height, as the auto-grid's discrete modes plus an explicit height.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RowHeight {
    Short,
    #[default]
    Standard,
    Tall,
    /// Explicit height; sets the mode to `custom`.
    Custom(f64),
}

impl ColumnWidth {
    fn mode(self) -> dashboardv2::AutoGridLayoutSpecColumnWidthMode {
        use dashboardv2::AutoGridLayoutSpecColumnWidthMode as M;
        match self {
            ColumnWidth::Narrow => M::Narrow,
            ColumnWidth::Standard => M::Standard,
            ColumnWidth::Wide => M::Wide,
            ColumnWidth::Custom(_) => M::Custom,
        }
    }

    fn width(self) -> Option<f64> {
        match self {
            ColumnWidth::Custom(w) => Some(w),
            _ => None,
        }
    }
}

impl RowHeight {
    fn mode(self) -> dashboardv2::AutoGridLayoutSpecRowHeightMode {
        use dashboardv2::AutoGridLayoutSpecRowHeightMode as M;
        match self {
            RowHeight::Short => M::Short,
            RowHeight::Standard => M::Standard,
            RowHeight::Tall => M::Tall,
            RowHeight::Custom(_) => M::Custom,
        }
    }

    fn height(self) -> Option<f64> {
        match self {
            RowHeight::Custom(h) => Some(h),
            _ => None,
        }
    }
}

/// Panels flowed into a column-capped grid.
#[derive(Debug, Clone)]
pub struct AutoGrid {
    max_columns: u32,
    column_width: ColumnWidth,
    row_height: RowHeight,
    /// `(element name, panel)`, in placement order.
    ///
    /// Element names are authored, not derived: only 14 of the baseline's 69 are
    /// slugs of their titles, and the rest are deliberately short stable ids
    /// (`availability-percent` for "Environment Availability (Select Time
    /// Range)"). Deriving them from titles would couple a permalink-visible
    /// identifier to display-text churn.
    panels: Vec<(String, dashboardv2::PanelKind)>,
}

impl AutoGrid {
    /// A grid capped at `max_columns`, standard width and height.
    pub fn new(max_columns: u32) -> Self {
        AutoGrid {
            max_columns,
            column_width: ColumnWidth::default(),
            row_height: RowHeight::default(),
            panels: Vec::new(),
        }
    }

    pub fn column_width(mut self, width: ColumnWidth) -> Self {
        self.column_width = width;
        self
    }

    pub fn row_height(mut self, height: RowHeight) -> Self {
        self.row_height = height;
        self
    }

    /// Place a panel under the element name that will reference it.
    pub fn panel(mut self, name: impl Into<String>, panel: dashboardv2::PanelKind) -> Self {
        self.panels.push((name.into(), panel));
        self
    }

    /// Place several panels at once.
    pub fn panels<N, I>(mut self, panels: I) -> Self
    where
        N: Into<String>,
        I: IntoIterator<Item = (N, dashboardv2::PanelKind)>,
    {
        for (name, panel) in panels {
            self.panels.push((name.into(), panel));
        }
        self
    }

    fn build(self, sink: &mut Sink) -> Result<dashboardv2::AutoGridLayoutKind> {
        let mut items = Vec::with_capacity(self.panels.len());
        for (name, panel) in self.panels {
            sink.register(&name, panel)?;
            items.push(dashboardv2::AutoGridLayoutItemKind {
                kind: AUTO_GRID_ITEM_KIND.to_string(),
                spec: dashboardv2::AutoGridLayoutItemSpec {
                    element: dashboardv2::ElementReference {
                        kind: ELEMENT_REFERENCE_KIND.to_string(),
                        name,
                    },
                    conditional_rendering: None,
                    repeat: None,
                },
            });
        }

        Ok(dashboardv2::AutoGridLayoutKind {
            kind: AUTO_GRID_KIND.to_string(),
            spec: dashboardv2::AutoGridLayoutSpec {
                items,
                max_column_count: f64::from(self.max_columns),
                column_width_mode: self.column_width.mode(),
                column_width: self.column_width.width(),
                row_height_mode: self.row_height.mode(),
                row_height: self.row_height.height(),
                // Constant `false` across every row of the baseline. Grafana
                // discards it on save (it matches the schema default), so this is
                // one field a UI round-trip will always show as removed; the
                // generated type has no way to omit a non-optional bool.
                fill_screen: false,
            },
        })
    }
}

/// A titled row wrapping one grid.
#[derive(Debug, Clone)]
pub struct Row {
    title: String,
    hide_header: bool,
    collapsed: bool,
    grid: AutoGrid,
}

impl Row {
    /// A row with a visible header.
    pub fn new(title: impl Into<String>) -> Self {
        Row {
            title: title.into(),
            hide_header: false,
            collapsed: false,
            grid: AutoGrid::new(3),
        }
    }

    /// Start the row collapsed.
    ///
    /// The baseline collapses its two heaviest sink sections ("Iceberg Sinks",
    /// "Kafka Sinks") so the tab opens without paying for them.
    pub fn collapsed(mut self) -> Self {
        self.collapsed = true;
        self
    }

    /// Keep the row's grouping but hide its header, as the baseline does for
    /// summary rows whose panels are self-describing.
    pub fn hide_header(mut self) -> Self {
        self.hide_header = true;
        self
    }

    pub fn grid(mut self, grid: AutoGrid) -> Self {
        self.grid = grid;
        self
    }

    fn build(self, sink: &mut Sink) -> Result<dashboardv2::RowsLayoutRowKind> {
        Ok(dashboardv2::RowsLayoutRowKind {
            kind: ROW_KIND.to_string(),
            spec: dashboardv2::RowsLayoutRowSpec {
                title: Some(self.title),
                // Neither field has a schema default, so absent means "unset" and
                // Grafana's runtime decides. `hideHeader` is emitted either way:
                // 14 of the baseline's rows write `false` explicitly and 2 omit it,
                // and being explicit costs nothing since absent and false behave
                // identically.
                hide_header: Some(self.hide_header),
                // Grafana adds `collapse` to every row on save, so emitting it
                // keeps a UI save from diffing on it.
                collapse: Some(self.collapsed),
                layout: dashboardv2::RowsLayoutRowSpecLayout::AutoGridLayoutKind(
                    self.grid.build(sink)?,
                ),
                fill_screen: None,
                conditional_rendering: None,
                repeat: None,
                variables: Vec::new(),
            },
        })
    }
}

/// A titled tab wrapping rows.
#[derive(Debug, Clone)]
pub struct Tab {
    title: String,
    rows: Vec<Row>,
}

impl Tab {
    pub fn new(title: impl Into<String>) -> Self {
        Tab {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    pub fn row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows<I: IntoIterator<Item = Row>>(mut self, rows: I) -> Self {
        self.rows.extend(rows);
        self
    }

    fn build(self, sink: &mut Sink) -> Result<dashboardv2::TabsLayoutTabKind> {
        let rows = self
            .rows
            .into_iter()
            .map(|row| row.build(sink))
            .collect::<Result<Vec<_>>>()?;

        Ok(dashboardv2::TabsLayoutTabKind {
            kind: TAB_KIND.to_string(),
            spec: dashboardv2::TabsLayoutTabSpec {
                title: Some(self.title),
                layout: dashboardv2::TabsLayoutTabSpecLayout::RowsLayoutKind(
                    dashboardv2::RowsLayoutKind {
                        kind: ROWS_KIND.to_string(),
                        spec: dashboardv2::RowsLayoutSpec { rows },
                    },
                ),
                conditional_rendering: None,
                repeat: None,
                variables: Vec::new(),
            },
        })
    }
}

/// A whole dashboard's arrangement.
#[derive(Debug, Clone)]
pub enum Layout {
    /// Tabs of rows of grids — the baseline's shape.
    Tabs(Vec<Tab>),
    /// Rows of grids, for a dashboard that needs no tabs.
    Rows(Vec<Row>),
}

impl Layout {
    pub fn tabs<I: IntoIterator<Item = Tab>>(tabs: I) -> Self {
        Layout::Tabs(tabs.into_iter().collect())
    }

    pub fn rows<I: IntoIterator<Item = Row>>(rows: I) -> Self {
        Layout::Rows(rows.into_iter().collect())
    }

    /// Build the `elements` map and `layout` tree together, assigning panel ids.
    ///
    /// Doing both at once is the point: it is the only place that can guarantee
    /// every reference resolves and every element is referenced exactly once.
    pub fn assemble(self) -> Result<Assembled> {
        let mut sink = Sink::default();
        let layout = match self {
            Layout::Tabs(tabs) => {
                let tabs = tabs
                    .into_iter()
                    .map(|tab| tab.build(&mut sink))
                    .collect::<Result<Vec<_>>>()?;
                dashboardv2::DashboardLayout::TabsLayoutKind(dashboardv2::TabsLayoutKind {
                    kind: TABS_KIND.to_string(),
                    spec: dashboardv2::TabsLayoutSpec { tabs },
                })
            }
            Layout::Rows(rows) => {
                let rows = rows
                    .into_iter()
                    .map(|row| row.build(&mut sink))
                    .collect::<Result<Vec<_>>>()?;
                dashboardv2::DashboardLayout::RowsLayoutKind(dashboardv2::RowsLayoutKind {
                    kind: ROWS_KIND.to_string(),
                    spec: dashboardv2::RowsLayoutSpec { rows },
                })
            }
        };

        if sink.elements.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Assembled {
            elements: sink.elements,
            layout,
        })
    }
}

/// The two halves of a dashboard's panel arrangement.
#[derive(Debug, Clone)]
pub struct Assembled {
    /// Goes into `Dashboard::elements`.
    pub elements: BTreeMap<String, dashboardv2::Element>,
    /// Goes into `Dashboard::layout`.
    pub layout: dashboardv2::DashboardLayout,
}

impl Assembled {
    /// Element names, sorted.
    ///
    /// `elements` is a `BTreeMap`, so this is already its iteration order — which
    /// is the point: a checked-in dashboard has to serialize its keys the same way
    /// on every build, and a `HashMap`'s randomly-seeded order would churn the
    /// diff between two identical runs.
    pub fn names(&self) -> Vec<&str> {
        self.elements.keys().map(String::as_str).collect()
    }
}

/// Collects panels as the layout tree is built, assigning ids and rejecting
/// duplicate names.
#[derive(Default)]
struct Sink {
    elements: BTreeMap<String, dashboardv2::Element>,
    next_id: u32,
}

impl Sink {
    fn register(&mut self, name: &str, mut panel: dashboardv2::PanelKind) -> Result<()> {
        let id = FIRST_PANEL_ID + self.next_id;
        // Overwrites whatever `Panel::build` was given: the id is generated, and
        // only the assembly knows the full set.
        panel.spec.id = f64::from(id);

        match self.elements.entry(name.to_string()) {
            Entry::Occupied(_) => return Err(Error::DuplicateElement(name.to_string())),
            Entry::Vacant(slot) => slot.insert(dashboardv2::Element::PanelKind(panel)),
        };
        self.next_id += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::panel::Panel;

    fn panel(title: &str) -> dashboardv2::PanelKind {
        Panel::timeseries(title).build(0)
    }

    fn grid_with(names: &[&str]) -> AutoGrid {
        let mut grid = AutoGrid::new(3);
        for name in names {
            grid = grid.panel(*name, panel(name));
        }
        grid
    }

    #[test]
    fn a_tabbed_layout_matches_the_baseline_nesting() {
        let assembled = Layout::tabs([
            Tab::new("Summary").row(Row::new("Health").grid(grid_with(&["a", "b"]))),
            Tab::new("Detail").row(Row::new("More").grid(grid_with(&["c"]))),
        ])
        .assemble()
        .expect("assemble");

        // Tabs -> tab -> Rows -> row -> AutoGrid -> item -> ElementReference.
        let tabs = match &assembled.layout {
            dashboardv2::DashboardLayout::TabsLayoutKind(t) => t,
            other => panic!("expected tabs, got {other:?}"),
        };
        assert_eq!(tabs.kind, "TabsLayout");
        assert_eq!(tabs.spec.tabs.len(), 2);

        let tab = &tabs.spec.tabs[0];
        assert_eq!(tab.kind, "TabsLayoutTab");
        assert_eq!(tab.spec.title.as_deref(), Some("Summary"));

        let rows = match &tab.spec.layout {
            dashboardv2::TabsLayoutTabSpecLayout::RowsLayoutKind(r) => r,
            other => panic!("expected rows, got {other:?}"),
        };
        assert_eq!(rows.spec.rows.len(), 1);

        let row = &rows.spec.rows[0];
        assert_eq!(row.kind, "RowsLayoutRow");
        assert_eq!(row.spec.title.as_deref(), Some("Health"));

        let grid = match &row.spec.layout {
            dashboardv2::RowsLayoutRowSpecLayout::AutoGridLayoutKind(g) => g,
            other => panic!("expected auto grid, got {other:?}"),
        };
        assert_eq!(grid.spec.items.len(), 2);
        assert_eq!(grid.spec.items[0].spec.element.name, "a");
        assert_eq!(grid.spec.items[0].spec.element.kind, "ElementReference");
    }

    #[test]
    fn a_row_only_layout_skips_the_tab_level() {
        let assembled = Layout::rows([Row::new("Only").grid(grid_with(&["a"]))])
            .assemble()
            .expect("assemble");
        assert!(matches!(
            assembled.layout,
            dashboardv2::DashboardLayout::RowsLayoutKind(_)
        ));
    }

    #[test]
    fn every_reference_resolves_and_every_element_is_referenced() {
        // The invariant the baseline holds exactly: 69 elements, 69 distinct refs,
        // nothing dangling in either direction.
        let assembled = Layout::tabs([Tab::new("T").rows([
            Row::new("R1").grid(grid_with(&["a", "b"])),
            Row::new("R2").grid(grid_with(&["c"])),
        ])])
        .assemble()
        .expect("assemble");

        let mut refs = Vec::new();
        collect_refs(
            &serde_json::to_value(&assembled.layout).expect("serialize"),
            &mut refs,
        );
        refs.sort();
        assert_eq!(refs, vec!["a", "b", "c"]);
        assert_eq!(assembled.names(), vec!["a", "b", "c"]);
    }

    fn collect_refs(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|k| k.as_str()) == Some("ElementReference")
                    && let Some(name) = map.get("name").and_then(|n| n.as_str())
                {
                    out.push(name.to_string());
                }
                for v in map.values() {
                    collect_refs(v, out);
                }
            }
            serde_json::Value::Array(items) => {
                for v in items {
                    collect_refs(v, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn panel_ids_are_assigned_sequentially_from_the_base() {
        let assembled = Layout::rows([Row::new("R").grid(grid_with(&["a", "b", "c"]))])
            .assemble()
            .expect("assemble");

        let mut ids: Vec<u32> = assembled
            .elements
            .values()
            .map(|element| match element {
                dashboardv2::Element::PanelKind(p) => p.spec.id as u32,
                other => panic!("unexpected element {other:?}"),
            })
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1000, 1001, 1002]);
    }

    #[test]
    fn the_assembly_overwrites_whatever_id_the_panel_carried() {
        // `Panel::build` takes an id for direct use; the layout owns it otherwise.
        let assembled = Layout::rows([
            Row::new("R").grid(AutoGrid::new(1).panel("a", Panel::stat("s").build(4242)))
        ])
        .assemble()
        .expect("assemble");
        match &assembled.elements["a"] {
            dashboardv2::Element::PanelKind(p) => assert_eq!(p.spec.id, 1000.0),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn duplicate_element_names_are_rejected() {
        // Silently replacing the earlier panel would drop it from the dashboard
        // while leaving a reference that resolves -- invisible in review.
        let err = Layout::rows([
            Row::new("R1").grid(grid_with(&["dup"])),
            Row::new("R2").grid(grid_with(&["dup"])),
        ])
        .assemble()
        .expect_err("should reject");
        assert_eq!(err, Error::DuplicateElement("dup".to_string()));
    }

    #[test]
    fn an_empty_layout_is_rejected() {
        assert_eq!(Layout::tabs([]).assemble().unwrap_err(), Error::Empty);
        assert_eq!(
            Layout::rows([Row::new("R")]).assemble().unwrap_err(),
            Error::Empty
        );
    }

    #[test]
    fn grid_modes_reach_the_spec() {
        let assembled = Layout::rows([Row::new("R").grid(
            AutoGrid::new(5)
                .column_width(ColumnWidth::Wide)
                .row_height(RowHeight::Short)
                .panel("a", panel("a")),
        )])
        .assemble()
        .expect("assemble");

        let grid = grid_of(&assembled);
        assert_eq!(grid.spec.max_column_count, 5.0);
        assert_eq!(
            grid.spec.column_width_mode,
            dashboardv2::AutoGridLayoutSpecColumnWidthMode::Wide
        );
        assert_eq!(
            grid.spec.row_height_mode,
            dashboardv2::AutoGridLayoutSpecRowHeightMode::Short
        );
        // Discrete modes leave the explicit dimensions unset.
        assert_eq!(grid.spec.column_width, None);
        assert_eq!(grid.spec.row_height, None);
    }

    #[test]
    fn custom_dimensions_set_both_the_mode_and_the_value() {
        let assembled = Layout::rows([Row::new("R").grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Custom(240.0))
                .row_height(RowHeight::Custom(120.0))
                .panel("a", panel("a")),
        )])
        .assemble()
        .expect("assemble");

        let grid = grid_of(&assembled);
        assert_eq!(
            grid.spec.column_width_mode,
            dashboardv2::AutoGridLayoutSpecColumnWidthMode::Custom
        );
        assert_eq!(grid.spec.column_width, Some(240.0));
        assert_eq!(
            grid.spec.row_height_mode,
            dashboardv2::AutoGridLayoutSpecRowHeightMode::Custom
        );
        assert_eq!(grid.spec.row_height, Some(120.0));
    }

    #[test]
    fn rows_emit_collapse_so_a_ui_save_does_not_diff() {
        // Grafana adds `collapse: false` on save; emitting it up front means a
        // save-from-UI produces no diff on that field.
        let assembled = Layout::rows([Row::new("R").grid(grid_with(&["a"]))])
            .assemble()
            .expect("assemble");
        let row = row_of(&assembled);
        assert_eq!(row.spec.collapse, Some(false));
        assert_eq!(row.spec.hide_header, Some(false));
    }

    #[test]
    fn a_row_can_start_collapsed() {
        let assembled = Layout::rows([Row::new("R").collapsed().grid(grid_with(&["a"]))])
            .assemble()
            .expect("assemble");
        assert_eq!(row_of(&assembled).spec.collapse, Some(true));
    }

    #[test]
    fn hide_header_is_opt_in() {
        let assembled = Layout::rows([Row::new("R").hide_header().grid(grid_with(&["a"]))])
            .assemble()
            .expect("assemble");
        assert_eq!(row_of(&assembled).spec.hide_header, Some(true));
    }

    fn row_of(assembled: &Assembled) -> &dashboardv2::RowsLayoutRowKind {
        match &assembled.layout {
            dashboardv2::DashboardLayout::RowsLayoutKind(r) => &r.spec.rows[0],
            other => panic!("unexpected {other:?}"),
        }
    }

    fn grid_of(assembled: &Assembled) -> &dashboardv2::AutoGridLayoutKind {
        match &row_of(assembled).spec.layout {
            dashboardv2::RowsLayoutRowSpecLayout::AutoGridLayoutKind(g) => g,
            other => panic!("unexpected {other:?}"),
        }
    }
}
