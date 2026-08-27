//! Build a whole dashboard through every layer and check the result is valid v2.
//!
//! Registry -> render context -> query bridge -> panel presets -> thresholds ->
//! layout -> shell. Each layer has its own tests; this one exists to catch the
//! seams between them, and to prove the output is something Grafana would accept
//! rather than merely something that compiles.

use std::path::PathBuf;

use mzmon_lib::grafana::context::{DashboardScope, dashboard_context};
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Layout, Row, RowHeight, Tab};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::query::panel_query;
use mzmon_lib::grafana::{palette, threshold, variable};
use mzmon_lib::query::{QueryEngine, QueryRegistry};

fn registry() -> QueryRegistry {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/queries");
    QueryRegistry::from_directory(&dir).expect("load query registry")
}

/// A small but complete dashboard, exercising every layer.
fn build() -> Resource {
    let registry = registry();
    let scope = DashboardScope::self_managed();
    let ctx = dashboard_context(&registry, QueryEngine::PromQl, &scope);

    let availability = panel_query(
        &registry,
        "materialize.health.environment.availability.percentage",
        &ctx,
    )
    .expect("bridge availability");

    let version = panel_query(&registry, "materialize.info.version", &ctx).expect("bridge version");

    Dashboard::new("mz-mon-e2e", "End To End")
        .description("A dashboard built through every layer.")
        .tags(["materialize", "monitoring"])
        .cursor_sync(CursorSync::Crosshair)
        .variables(variable::environment_scoped(
            &scope.sql_metric_prefix,
            false,
        ))
        .metadata_annotation("monitoring.materialize.cloud/sql-metric-prefix", "mz_")
        .layout(Layout::tabs([
            Tab::new("Summary").rows([
                Row::new("Environment Health").grid(
                    AutoGrid::new(3)
                        .column_width(ColumnWidth::Wide)
                        .panel(
                            "availability-percent",
                            Panel::stat("Environment Availability")
                                .query(availability)
                                .color_background()
                                .unit("percent")
                                .decimals(4.0)
                                .no_value(NoValue::FilterMismatch)
                                .thresholds(threshold::health(95.0, 99.0).percentage().build())
                                .mappings(threshold::health_mapping(95.0, 99.0))
                                .build(0),
                        )
                        .panel(
                            "materialize-version",
                            Panel::stat("Materialize Version")
                                .query(version)
                                .reduce_fields("/^mz_version$/")
                                .no_value(NoValue::FilterMismatch)
                                .build(0),
                        ),
                ),
                Row::new("Resources").hide_header().grid(
                    AutoGrid::new(5)
                        .row_height(RowHeight::Short)
                        .panel(
                            "cpu-usage",
                            Panel::gauge("CPU Usage")
                                .unit("percentunit")
                                .no_value(NoValue::RequiresCAdvisor)
                                .thresholds(threshold::load_default().build())
                                .build(0),
                        )
                        .panel(
                            "sink-errors",
                            Panel::table("Sink Errors")
                                .no_value(NoValue::FilterMismatch)
                                .thresholds(threshold::errors(1.0, 10.0).build())
                                .build(0),
                        ),
                ),
            ]),
            Tab::new("Detail").row(
                Row::new("Series").collapsed().grid(
                    AutoGrid::new(2).panel(
                        "arrangement-rate",
                        Panel::timeseries("Arrangement Rate")
                            .unit("cps")
                            .min(0.0)
                            .log_scale(10.0)
                            .shade(palette::THEME[3])
                            .build(0),
                    ),
                ),
            ),
        ]))
        .build()
        .expect("build the dashboard")
}

#[test]
fn the_whole_stack_produces_a_deserializable_v2_dashboard() {
    let resource = build();
    let json = serde_json::to_string(&resource).expect("serialize");

    // The real check: it must come back through the generated models, which carry
    // `deny_unknown_fields` -- so anything we invented fails here.
    let back: Resource = serde_json::from_str(&json).expect("deserialize as a v2 dashboard");

    assert_eq!(back.api_version, "dashboard.grafana.app/v2");
    assert_eq!(back.kind, "Dashboard");
    assert_eq!(back.metadata.name, "mz-mon-e2e");
    assert_eq!(back.spec.elements.len(), 5);
    assert_eq!(back.spec.variables.len(), 7);
}

#[test]
fn it_also_deserializes_as_a_bare_dashboard_spec() {
    // The chart ships the enveloped resource, but the Grafana HTTP API takes the
    // spec, so both shapes have to hold.
    let resource = build();
    let json = serde_json::to_string(&resource.spec).expect("serialize spec");
    let back: dashboardv2::Dashboard = serde_json::from_str(&json).expect("deserialize spec");
    assert_eq!(back.title, "End To End");
}

#[test]
fn every_panel_is_referenced_exactly_once_by_the_layout() {
    let resource = build();
    let layout = serde_json::to_value(&resource.spec.layout).expect("serialize layout");
    let mut refs = Vec::new();
    collect_element_refs(&layout, &mut refs);
    refs.sort();

    let mut names: Vec<String> = resource.spec.elements.keys().cloned().collect();
    names.sort();

    assert_eq!(refs, names, "elements and references must agree exactly");
    assert_eq!(refs.len(), 5);
}

#[test]
fn panel_ids_are_unique_and_sequential() {
    let resource = build();
    let mut ids: Vec<u32> = resource
        .spec
        .elements
        .values()
        .map(|e| match e {
            dashboardv2::Element::PanelKind(p) => p.spec.id as u32,
            other => panic!("unexpected element {other:?}"),
        })
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![1000, 1001, 1002, 1003, 1004]);
}

#[test]
fn the_bridged_queries_are_rendered_and_scoped() {
    let resource = build();
    let panel = match &resource.spec.elements["availability-percent"] {
        dashboardv2::Element::PanelKind(p) => p,
        other => panic!("unexpected {other:?}"),
    };
    let spec = panel.spec.data.spec.queries[0]
        .spec
        .query
        .spec
        .as_ref()
        .expect("dataquery spec");
    let expr = spec["expr"].as_str().expect("expr");

    // Rendered through the dashboard context, so: no template parameters left,
    // scoped to the environment variable, and using the Grafana range built-in.
    assert!(!expr.contains("%%{"), "unsubstituted parameter:\n{expr}");
    assert!(expr.contains("$environmentNameList"), "{expr}");
    assert!(expr.contains("$__range"), "{expr}");
    // And the description came from the registry, not the panel author.
    assert!(
        panel.spec.description.starts_with("**"),
        "description should open with the bold summary: {}",
        panel.spec.description
    );
}

#[test]
fn thresholds_carry_an_explicit_base() {
    let resource = build();
    for name in ["availability-percent", "cpu-usage", "sink-errors"] {
        let panel = match &resource.spec.elements[name] {
            dashboardv2::Element::PanelKind(p) => p,
            other => panic!("unexpected {other:?}"),
        };
        let thresholds = panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .thresholds
            .as_ref()
            .unwrap_or_else(|| panic!("{name} should carry thresholds"));
        assert_eq!(
            thresholds.steps[0].value, None,
            "{name}: first step must be the base"
        );
    }
}

#[test]
fn the_shell_emits_what_grafana_would_add_on_save() {
    // The three round-trip findings, checked on a real assembled dashboard.
    let resource = build();
    assert_eq!(resource.spec.live_now, Some(false));
    assert_eq!(resource.spec.annotations.len(), 1);
    assert!(resource.spec.annotations[0].spec.built_in);
    assert_eq!(
        resource.spec.cursor_sync,
        dashboardv2::DashboardCursorSync::Crosshair
    );
}

#[test]
fn a_dashboard_missing_its_variables_is_rejected() {
    // The end-to-end version of the silent-empty-panel guard: the bridged queries
    // reference $environmentNameList, so omitting the variable set must fail.
    let registry = registry();
    let scope = DashboardScope::self_managed();
    let ctx = dashboard_context(&registry, QueryEngine::PromQl, &scope);
    let bridged = panel_query(
        &registry,
        "materialize.health.environment.availability.percentage",
        &ctx,
    )
    .expect("bridge");

    let result = Dashboard::new("mz-mon-broken", "Broken")
        .layout(Layout::rows([Row::new("R").grid(
            AutoGrid::new(1).panel("a", Panel::stat("A").query(bridged).build(0)),
        )]))
        .build();
    assert!(
        result.is_err(),
        "should reject undefined variable references"
    );
}

/// Collect `ElementReference` names from a serialized layout.
fn collect_element_refs(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("kind").and_then(|k| k.as_str()) == Some("ElementReference")
                && let Some(name) = map.get("name").and_then(|n| n.as_str())
            {
                out.push(name.to_string());
            }
            for v in map.values() {
                collect_element_refs(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_element_refs(v, out);
            }
        }
        _ => {}
    }
}
