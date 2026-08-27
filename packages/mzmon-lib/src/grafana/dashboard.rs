// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The dashboard shell, and the Kubernetes envelope it ships in.
//!
//! This is the last piece: [`Dashboard`] takes a title, a [`Layout`], and a
//! variable set, and produces either a bare `dashboardv2::Dashboard` or the
//! `GrafanaManifest`-style resource the chart deploys.
//!
//! ```no_run
//! use mzmon_lib::grafana::dashboard::Dashboard;
//! use mzmon_lib::grafana::layout::{AutoGrid, Layout, Row};
//! use mzmon_lib::grafana::panel::Panel;
//! use mzmon_lib::grafana::variable;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let resource = Dashboard::new("mz-mon-env-top", "Materialize Environment Overview")
//!     .description("Overview of a Materialize Environment.")
//!     .tags(["materialize", "monitoring"])
//!     .variables(variable::environment_scoped("mz_"))
//!     .layout(Layout::rows([
//!         Row::new("Health").grid(AutoGrid::new(3).panel("up", Panel::stat("Up").build(0))),
//!     ]))
//!     .build()?;
//! # let _ = resource;
//! # Ok(())
//! # }
//! ```
//!
//! # What the shell emits that the baseline did not
//!
//! Three fields come from the load-and-save round trip — Grafana writes them on
//! every save, so emitting them up front is what keeps a UI save from diffing
//! against the generated file:
//!
//! * `liveNow: false`.
//! * The built-in `Annotations & Alerts` query. Grafana adds it to any dashboard
//!   that has none; authoring it means the annotation list is what we said it was.
//! * `cursorSync`. The baseline left this `Off`, which is Grafana's default and
//!   almost certainly not what anyone wanted on a dashboard of 28 timeseries
//!   panels — see [`CursorSync`].

use crate::grafana::generated::dashboardv2;
use crate::grafana::layout::{self, Assembled, Layout};
use crate::grafana::variable;

/// `apiVersion` of the dashboard resource.
pub const API_VERSION: &str = "dashboard.grafana.app/v2";
/// `kind` of the dashboard resource.
pub const RESOURCE_KIND: &str = "Dashboard";

/// `AnnotationQueryKind::kind` discriminant.
const ANNOTATION_QUERY_KIND: &str = "AnnotationQuery";
/// `DataQueryKind::kind` discriminant.
const DATA_QUERY_KIND: &str = "DataQuery";

/// Shared-cursor behaviour across a dashboard's panels.
///
/// The baseline left this `Off`. On a dashboard whose whole purpose is
/// correlating across panels — "did memory climb when the lag did?" — that means
/// every comparison is done by eye against two independently-hovered charts.
/// [`CursorSync::Crosshair`] is the default here for that reason.
///
/// `Tooltip` is the same crosshair plus every panel's tooltip at once, which is a
/// lot of overlay on a 28-panel tab; it earns its place on a small focused
/// dashboard rather than a broad one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorSync {
    /// No sharing. Grafana's own default.
    Off,
    /// Share a crosshair, so hovering one panel marks the same instant in all.
    #[default]
    Crosshair,
    /// Share the crosshair *and* every panel's tooltip.
    Tooltip,
}

impl From<CursorSync> for dashboardv2::DashboardCursorSync {
    fn from(sync: CursorSync) -> Self {
        match sync {
            CursorSync::Off => dashboardv2::DashboardCursorSync::Off,
            CursorSync::Crosshair => dashboardv2::DashboardCursorSync::Crosshair,
            CursorSync::Tooltip => dashboardv2::DashboardCursorSync::Tooltip,
        }
    }
}

/// What went wrong building a dashboard.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No layout was supplied, so there is nothing to render.
    #[error("dashboard {0:?} has no layout")]
    NoLayout(String),

    /// One or more panels named a registry query that could not be bridged.
    ///
    /// Carries every failure rather than the first: a rename in the registry
    /// typically breaks several panels at once, and fixing them one render at a
    /// time is needless.
    #[error("dashboard {dashboard:?} has {} unresolved registry query/queries:\n  {}", failures.len(), failures.join("\n  "))]
    Registry {
        dashboard: &'static str,
        failures: Vec<String>,
    },

    /// The layout could not be assembled.
    #[error("dashboard {name:?}: {source}")]
    Layout {
        name: String,
        #[source]
        source: layout::Error,
    },

    /// A rendered query references a variable the dashboard does not define.
    ///
    /// An undefined Grafana variable interpolates to nothing, so the selector
    /// matches no series and the panel renders empty but healthy-looking.
    #[error(
        "dashboard {name:?} references ${variable} but does not define it \
         (defined: {defined})"
    )]
    UndefinedVariable {
        name: String,
        variable: String,
        defined: String,
    },
}

/// Result of building a dashboard.
pub type Result<T> = std::result::Result<T, Error>;

/// Time settings: the window, refresh options, and timezone.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSettings {
    /// Start of the default window, as a Grafana time expression.
    pub from: String,
    /// End of the default window.
    pub to: String,
    /// Auto-refresh interval; empty disables it.
    pub auto_refresh: String,
    /// Intervals offered in the refresh picker.
    pub auto_refresh_intervals: Vec<String>,
    /// `browser` renders in the viewer's timezone, which is what an operator
    /// comparing a dashboard against their own logs wants.
    pub timezone: String,
    pub hide_timepicker: bool,
}

impl Default for TimeSettings {
    fn default() -> Self {
        TimeSettings {
            from: "now-6h".to_string(),
            to: "now".to_string(),
            auto_refresh: String::new(),
            auto_refresh_intervals: [
                "5s", "10s", "30s", "1m", "5m", "15m", "30m", "1h", "2h", "1d",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            timezone: "browser".to_string(),
            hide_timepicker: false,
        }
    }
}

impl TimeSettings {
    fn build(self) -> dashboardv2::TimeSettingsSpec {
        dashboardv2::TimeSettingsSpec {
            from: self.from,
            to: self.to,
            auto_refresh: self.auto_refresh,
            auto_refresh_intervals: self.auto_refresh_intervals,
            timezone: self.timezone,
            hide_timepicker: self.hide_timepicker,
            fiscal_year_start_month: 0,
            now_delay: None,
            quick_ranges: Vec::new(),
            week_start: None,
        }
    }
}

/// A dashboard under construction.
#[derive(Debug, Clone)]
pub struct Dashboard {
    name: String,
    title: String,
    description: Option<String>,
    tags: Vec<String>,
    cursor_sync: CursorSync,
    editable: bool,
    preload: bool,
    time_settings: TimeSettings,
    variables: Vec<dashboardv2::VariableKind>,
    layout: Option<Layout>,
    annotations: Vec<dashboardv2::AnnotationQueryKind>,
    /// Kubernetes metadata annotations. Read by the docsite shortcode; note that
    /// Grafana drops them on a UI save, so they cannot gate anything without a
    /// reconcile step.
    metadata_annotations: Vec<(String, String)>,
}

impl Dashboard {
    /// Start a dashboard.
    ///
    /// `name` is the resource name (`mz-mon-env-top`) and is what a permalink and
    /// the chart's manifest key are built from, so it should be stable
    /// independently of `title`.
    pub fn new(name: impl Into<String>, title: impl Into<String>) -> Self {
        Dashboard {
            name: name.into(),
            title: title.into(),
            description: None,
            tags: Vec::new(),
            cursor_sync: CursorSync::default(),
            editable: true,
            preload: false,
            time_settings: TimeSettings::default(),
            variables: Vec::new(),
            layout: None,
            annotations: Vec::new(),
            metadata_annotations: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn tags<S: Into<String>, I: IntoIterator<Item = S>>(mut self, tags: I) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Override the shared-cursor behaviour. See [`CursorSync`] for why the
    /// default is `Crosshair` rather than Grafana's `Off`.
    pub fn cursor_sync(mut self, sync: CursorSync) -> Self {
        self.cursor_sync = sync;
        self
    }

    /// Lock the dashboard against edits in the UI.
    pub fn read_only(mut self) -> Self {
        self.editable = false;
        self
    }

    /// Load every panel on open rather than as it scrolls into view.
    ///
    /// Off by default: a broad dashboard would fire every query at once.
    pub fn preload(mut self) -> Self {
        self.preload = true;
        self
    }

    pub fn time_settings(mut self, settings: TimeSettings) -> Self {
        self.time_settings = settings;
        self
    }

    pub fn variables<I: IntoIterator<Item = dashboardv2::VariableKind>>(
        mut self,
        variables: I,
    ) -> Self {
        self.variables = variables.into_iter().collect();
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Add an annotation query beyond the built-in one.
    pub fn annotation(mut self, annotation: dashboardv2::AnnotationQueryKind) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Add a Kubernetes metadata annotation.
    pub fn metadata_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata_annotations.push((key.into(), value.into()));
        self
    }

    /// Build the dashboard spec, checking that every variable a panel references
    /// is defined.
    pub fn build_spec(self) -> Result<dashboardv2::Dashboard> {
        let name = self.name.clone();
        let layout = self.layout.ok_or_else(|| Error::NoLayout(name.clone()))?;
        let Assembled { elements, layout } = layout.assemble().map_err(|source| Error::Layout {
            name: name.clone(),
            source,
        })?;

        let mut annotations = vec![builtin_annotations()];
        annotations.extend(self.annotations);

        let dashboard = dashboardv2::Dashboard {
            title: self.title,
            description: self.description,
            tags: self.tags,
            cursor_sync: self.cursor_sync.into(),
            editable: self.editable,
            preload: self.preload,
            // Grafana writes this on save; emitting it keeps a UI save clean.
            live_now: Some(false),
            time_settings: self.time_settings.build(),
            variables: self.variables,
            elements,
            layout,
            annotations,
            links: Vec::new(),
            preferences: None,
            revision: None,
        };

        check_variable_references(&name, &dashboard)?;
        Ok(dashboard)
    }

    /// Build the full Kubernetes-style resource the chart deploys.
    pub fn build(self) -> Result<Resource> {
        let name = self.name.clone();
        let metadata_annotations = self.metadata_annotations.clone();
        let spec = self.build_spec()?;
        Ok(Resource {
            api_version: API_VERSION.to_string(),
            kind: RESOURCE_KIND.to_string(),
            metadata: Metadata {
                name,
                annotations: metadata_annotations.into_iter().collect(),
            },
            spec,
        })
    }
}

/// The Kubernetes-style envelope a dashboard ships in.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Resource {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: dashboardv2::Dashboard,
}

/// Resource metadata.
///
/// Only the fields we author. Grafana owns `uid`, `resourceVersion`,
/// `generation`, `creationTimestamp` and its own `grafana.app/*` annotations, and
/// replaces ours on a UI save.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub annotations: std::collections::BTreeMap<String, String>,
}

/// Grafana's built-in annotation query.
///
/// Grafana adds this to any dashboard that has none, so authoring it is what
/// makes the annotation list match between the generated file and a UI save.
pub fn builtin_annotations() -> dashboardv2::AnnotationQueryKind {
    dashboardv2::AnnotationQueryKind {
        kind: ANNOTATION_QUERY_KIND.to_string(),
        spec: dashboardv2::AnnotationQuerySpec {
            name: "Annotations & Alerts".to_string(),
            built_in: true,
            enable: true,
            // Hidden: the built-in query has no toggle worth showing in the
            // dashboard's annotation controls.
            hide: true,
            icon_color: "rgba(0, 211, 255, 1)".to_string(),
            query: dashboardv2::DataQueryKind {
                kind: DATA_QUERY_KIND.to_string(),
                group: "grafana".to_string(),
                version: "v0".to_string(),
                datasource: Some(dashboardv2::DataQueryKindDatasource {
                    name: Some("-- Grafana --".to_string()),
                }),
                spec: Some(Default::default()),
                labels: Default::default(),
            },
            filter: None,
            legacy_options: Default::default(),
            mappings: Default::default(),
            placement: None,
        },
    }
}

/// Fail if a rendered query references a variable the dashboard does not define.
///
/// This is the check the whole variable-naming discussion was about: an undefined
/// Grafana variable interpolates to nothing, the selector matches no series, and
/// the panel renders empty while looking perfectly healthy.
fn check_variable_references(name: &str, dashboard: &dashboardv2::Dashboard) -> Result<()> {
    /// Grafana's global interpolations, which no dashboard defines.
    ///
    /// Two families. The time and interval built-ins come from the panel's own
    /// query context; the rest are *data link* interpolations, resolved from the
    /// row under the cursor when a link is followed —
    /// `${__data.fields.commit}` in a version panel, say. Both look exactly like
    /// a variable reference and neither is one.
    const BUILTINS: &[&str] = &[
        // Query context.
        "__rate_interval",
        "__interval",
        "__interval_ms",
        "__range",
        "__range_s",
        "__range_ms",
        "__auto",
        "__auto_interval",
        "__all",
        "__from",
        "__to",
        "__timeFilter",
        // Data links and panel context.
        "__data",
        "__field",
        "__series",
        "__value",
        "__cell",
        "__url_time_range",
        "__all_variables",
        "__dashboard",
        "__org",
        "__user",
        "__timezone",
    ];

    let defined: Vec<&str> = dashboard.variables.iter().map(variable::name_of).collect();

    // Panels only: a variable's *own* query legitimately references its
    // predecessors, and `variable::environment_scoped` already checks that chain.
    let panels = serde_json::to_value(&dashboard.elements).unwrap_or(serde_json::Value::Null);
    let mut referenced = Vec::new();
    collect_references(&panels, &mut referenced);

    for reference in referenced {
        if BUILTINS.contains(&reference.as_str()) || defined.contains(&reference.as_str()) {
            continue;
        }
        return Err(Error::UndefinedVariable {
            name: name.to_string(),
            variable: reference,
            defined: defined.join(", "),
        });
    }
    Ok(())
}

/// Collect `$name` references from every string in a JSON tree.
fn collect_references(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => {
            let mut rest = text.as_str();
            while let Some(pos) = rest.find('$') {
                let after = &rest[pos + 1..];
                // `${name}` and `$name` are both valid Grafana syntax.
                let after = after.strip_prefix('{').unwrap_or(after);
                let len = after
                    .bytes()
                    .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    .count();
                // `$1` and friends are `label_replace` capture groups, not
                // variables.
                if len > 0 && !after[..len].bytes().all(|b| b.is_ascii_digit()) {
                    out.push(after[..len].to_string());
                }
                rest = &after[len..];
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_references(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_references(v, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::layout::{AutoGrid, Row};
    use crate::grafana::panel::Panel;

    fn minimal() -> Dashboard {
        Dashboard::new("mz-mon-test", "Test").layout(Layout::rows([
            Row::new("R").grid(AutoGrid::new(3).panel("a", Panel::timeseries("A").build(0)))
        ]))
    }

    #[test]
    fn the_shell_carries_the_resource_envelope() {
        let resource = minimal().build().expect("build");
        assert_eq!(resource.api_version, "dashboard.grafana.app/v2");
        assert_eq!(resource.kind, "Dashboard");
        assert_eq!(resource.metadata.name, "mz-mon-test");
        assert_eq!(resource.spec.title, "Test");
    }

    #[test]
    fn cursor_sync_defaults_to_crosshair_not_off() {
        // The baseline left this Off, which is Grafana's default and not what a
        // correlation dashboard wants.
        assert_eq!(
            minimal().build_spec().expect("build").cursor_sync,
            dashboardv2::DashboardCursorSync::Crosshair
        );
        assert_eq!(
            minimal()
                .cursor_sync(CursorSync::Off)
                .build_spec()
                .expect("build")
                .cursor_sync,
            dashboardv2::DashboardCursorSync::Off
        );
    }

    #[test]
    fn the_builtin_annotation_query_is_emitted() {
        // Grafana adds it on save, so emitting it keeps a UI save from diffing.
        let spec = minimal().build_spec().expect("build");
        assert_eq!(spec.annotations.len(), 1);
        let annotation = &spec.annotations[0];
        assert_eq!(annotation.kind, "AnnotationQuery");
        assert!(annotation.spec.built_in);
        assert_eq!(annotation.spec.name, "Annotations & Alerts");
        let query = &annotation.spec.query;
        assert_eq!(query.group, "grafana");
        assert_eq!(
            query.datasource.as_ref().and_then(|d| d.name.as_deref()),
            Some("-- Grafana --")
        );
    }

    #[test]
    fn live_now_is_emitted() {
        assert_eq!(minimal().build_spec().expect("build").live_now, Some(false));
    }

    #[test]
    fn extra_annotations_come_after_the_builtin() {
        let spec = minimal()
            .annotation(builtin_annotations())
            .build_spec()
            .expect("build");
        assert_eq!(spec.annotations.len(), 2);
    }

    #[test]
    fn a_dashboard_without_a_layout_is_rejected() {
        let err = Dashboard::new("n", "T")
            .build_spec()
            .expect_err("should fail");
        assert_eq!(err, Error::NoLayout("n".to_string()));
    }

    #[test]
    fn layout_errors_name_the_dashboard() {
        let err = Dashboard::new("n", "T")
            .layout(Layout::rows([Row::new("R")]))
            .build_spec()
            .expect_err("should fail");
        assert!(matches!(err, Error::Layout { .. }), "{err:?}");
        assert!(err.to_string().contains("\"n\""), "{err}");
    }

    #[test]
    fn a_panel_referencing_an_undefined_variable_is_rejected() {
        // The silent-empty-panel guard: this is the whole reason the check exists.
        let panel = Panel::timeseries("A")
            .no_value(crate::grafana::panel::NoValue::FilterMismatch)
            .unit("$nonesuch")
            .build(0);
        let err = Dashboard::new("n", "T")
            .layout(Layout::rows([
                Row::new("R").grid(AutoGrid::new(1).panel("a", panel))
            ]))
            .build_spec()
            .expect_err("should fail");
        match err {
            Error::UndefinedVariable { variable, .. } => assert_eq!(variable, "nonesuch"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_standard_variable_set_satisfies_a_panel_using_it() {
        // A panel whose query references the whole chain must build once the
        // standard set is attached, and fail without it.
        let expr = r#"up{materialize_cloud_organization_name=~"$environmentNameList", instance_id=~"$mzClusterList"}"#;
        let build =
            |variables: Vec<dashboardv2::VariableKind>| {
                let data = crate::grafana::query::query_group(vec![
                    crate::grafana::query::promql_data_query(expr, "metricsDatasource", None),
                ]);
                Dashboard::new("n", "T")
                    .variables(variables)
                    .layout(Layout::rows([Row::new("R").grid(
                        AutoGrid::new(1).panel("a", Panel::timeseries("A").data(data).build(0)),
                    )]))
                    .build_spec()
            };
        assert!(build(variable::environment_scoped("mz_")).is_ok());
        assert!(build(Vec::new()).is_err());
    }

    #[test]
    fn the_datasource_reference_counts_as_a_variable_reference() {
        // Every dataquery names `${metricsDatasource}`, so a dashboard that omits
        // the datasource variable has to fail too.
        let data =
            crate::grafana::query::query_group(vec![crate::grafana::query::promql_data_query(
                "up",
                "metricsDatasource",
                None,
            )]);
        let err = Dashboard::new("n", "T")
            .layout(Layout::rows([Row::new("R").grid(
                AutoGrid::new(1).panel("a", Panel::timeseries("A").data(data).build(0)),
            )]))
            .build_spec()
            .expect_err("should fail");
        match err {
            Error::UndefinedVariable { variable, .. } => assert_eq!(variable, "metricsDatasource"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn grafana_builtins_do_not_need_defining() {
        let data =
            crate::grafana::query::query_group(vec![crate::grafana::query::promql_data_query(
                "rate(up[$__rate_interval])",
                "ds",
                None,
            )]);
        let spec = Dashboard::new("n", "T")
            .variables(vec![variable::metrics_datasource()])
            .layout(Layout::rows([Row::new("R").grid(
                AutoGrid::new(1).panel("a", Panel::timeseries("A").data(data).build(0)),
            )]))
            .build_spec();
        // `ds` is undefined, but `$__rate_interval` must not be what fails.
        match spec {
            Err(Error::UndefinedVariable { variable, .. }) => assert_eq!(variable, "ds"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn time_settings_default_to_a_six_hour_browser_window() {
        let spec = minimal().build_spec().expect("build");
        assert_eq!(spec.time_settings.from, "now-6h");
        assert_eq!(spec.time_settings.to, "now");
        assert_eq!(spec.time_settings.timezone, "browser");
        assert!(spec.time_settings.auto_refresh.is_empty());
    }

    #[test]
    fn metadata_annotations_survive_into_the_resource() {
        let resource = minimal()
            .metadata_annotation("monitoring.materialize.cloud/min-mz-version", "v26.24.0")
            .build()
            .expect("build");
        assert_eq!(
            resource
                .metadata
                .annotations
                .get("monitoring.materialize.cloud/min-mz-version")
                .map(String::as_str),
            Some("v26.24.0")
        );
    }

    #[test]
    fn the_resource_round_trips_through_json() {
        let resource = minimal()
            .description("d")
            .tags(["materialize"])
            .variables(variable::environment_scoped("mz_"))
            .build()
            .expect("build");
        let json = serde_json::to_string(&resource).expect("serialize");
        let back: Resource = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            serde_json::to_string(&back).expect("re-serialize"),
            json,
            "resource should round-trip"
        );
    }
}
