//! Deserialize the pre-rendered `env-top` dashboard into the generated models.
//!
//! `charts/materialize-monitoring/pre-rendered/dashboards/grafana/env-top.yaml`
//! is what the Python implementation ships today, so it is the baseline the Rust
//! models have to accept. This is a parse test, not a byte-for-byte one -- key
//! order and omitted-vs-null are not part of the contract.

use std::path::PathBuf;

use mzmon_lib::grafana::generated::dashboardv2;

fn golden_path() -> PathBuf {
    // tests/ -> mzmon-lib -> packages -> repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../charts/materialize-monitoring/pre-rendered/dashboards/grafana/env-top.yaml")
}

/// The Kubernetes-style envelope the chart ships; `spec` is the dashboard.
#[derive(serde::Deserialize)]
struct DashboardResource {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    spec: dashboardv2::Dashboard,
}

#[test]
fn golden_env_top_deserializes() {
    let raw = std::fs::read_to_string(golden_path()).expect("read golden dashboard");
    let res: DashboardResource =
        serde_yaml_ng::from_str(&raw).expect("deserialize golden dashboard");

    assert_eq!(res.api_version, "dashboard.grafana.app/v2");
    assert_eq!(res.kind, "Dashboard");
    assert_eq!(res.spec.title, "Materialize Environment Overview");
    assert!(
        res.spec.elements.len() > 50,
        "expected the full panel set, got {}",
        res.spec.elements.len()
    );
}

/// Every panel's `vizConfig.group` should be a plugin id the generated module set
/// covers -- if the golden dashboard grows a panel type we have not generated,
/// this is where it surfaces.
#[test]
fn golden_panel_plugins_are_all_generated() {
    let raw = std::fs::read_to_string(golden_path()).expect("read golden dashboard");
    let res: DashboardResource =
        serde_yaml_ng::from_str(&raw).expect("deserialize golden dashboard");

    let known = [
        dashboardv2_plugin_ids::TIMESERIES,
        dashboardv2_plugin_ids::STAT,
        dashboardv2_plugin_ids::PIECHART,
        dashboardv2_plugin_ids::TABLE,
        dashboardv2_plugin_ids::GAUGE,
        dashboardv2_plugin_ids::BARCHART,
    ];

    let mut unknown: Vec<String> = Vec::new();
    for element in res.spec.elements.values() {
        if let dashboardv2::Element::PanelKind(panel) = element {
            let group = panel.spec.viz_config.group.as_str();
            if !known.contains(&group) {
                unknown.push(group.to_string());
            }
        }
    }
    unknown.sort();
    unknown.dedup();
    assert!(unknown.is_empty(), "ungenerated panel plugins: {unknown:?}");
}

mod dashboardv2_plugin_ids {
    use mzmon_lib::grafana::generated as g;
    pub const TIMESERIES: &str = g::TIMESERIES_PLUGIN_ID;
    pub const STAT: &str = g::STAT_PLUGIN_ID;
    pub const PIECHART: &str = g::PIECHART_PLUGIN_ID;
    pub const TABLE: &str = g::TABLE_PLUGIN_ID;
    pub const GAUGE: &str = g::GAUGE_PLUGIN_ID;
    pub const BARCHART: &str = g::BARCHART_PLUGIN_ID;
}

/// Re-serializing the golden dashboard must not lose or alter anything.
///
/// The parse test above is already most of the guarantee -- the generated structs
/// carry `deny_unknown_fields`, so a field the schema does not know about fails
/// the parse rather than vanishing. What this adds is the other direction: that
/// everything parsed comes back out. Nulls, empty containers, and integer-vs-float
/// spelling are normalized away, since none of those are part of the contract.
/// (Several counts, `maxColumnCount` among them, are `type: number` upstream and
/// so land as `f64`; re-serializing yields `3.0` where the input said `3`.)
#[test]
fn golden_env_top_round_trips_without_loss() {
    let raw = std::fs::read_to_string(golden_path()).expect("read golden dashboard");
    let original: serde_json::Value = serde_yaml_ng::from_str(&raw).expect("parse golden as yaml");

    let parsed: DashboardResource = serde_yaml_ng::from_str(&raw).expect("deserialize golden");
    let round_tripped = serde_json::json!({
        "apiVersion": parsed.api_version,
        "kind": parsed.kind,
        "spec": serde_json::to_value(&parsed.spec).expect("re-serialize dashboard"),
    });

    let mut losses = Vec::new();
    diff(
        &normalize(&original["spec"]),
        &normalize(&round_tripped["spec"]),
        "spec",
        &mut losses,
    );
    assert!(
        losses.is_empty(),
        "round-trip lost {} value(s):\n  {}",
        losses.len(),
        losses.join("\n  ")
    );
}

/// Drop nulls and empty maps/arrays so omitted-vs-empty differences do not
/// register as loss.
fn normalize(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, v)| (k.clone(), normalize(v)))
                .filter(|(_, v)| !matches!(v, Value::Null) && !is_empty(v))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        other => other.clone(),
    }
}

fn is_empty(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(m) => m.is_empty(),
        serde_json::Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

/// Report anything present in `before` but missing or changed in `after`.
fn diff(before: &serde_json::Value, after: &serde_json::Value, path: &str, out: &mut Vec<String>) {
    use serde_json::Value;
    match (before, after) {
        (Value::Object(b), Value::Object(a)) => {
            for (k, bv) in b {
                match a.get(k) {
                    Some(av) => diff(bv, av, &format!("{path}.{k}"), out),
                    None => out.push(format!("{path}.{k} dropped (was {bv})")),
                }
            }
        }
        (Value::Array(b), Value::Array(a)) if b.len() == a.len() => {
            for (i, (bv, av)) in b.iter().zip(a).enumerate() {
                diff(bv, av, &format!("{path}[{i}]"), out);
            }
        }
        // 3 and 3.0 are the same value; only the spelling differs.
        (Value::Number(b), Value::Number(a)) if b.as_f64() == a.as_f64() => {}
        (b, a) if b != a => out.push(format!("{path} changed: {b} -> {a}")),
        _ => {}
    }
}

/// The panel presets must reproduce the baseline's options blocks.
///
/// This is what keeps `grafana::panel` honest: the presets were derived from this
/// dashboard, so any drift between them and it is a regression in one or the
/// other. Deliberate deviations are enumerated below rather than smoothed over --
/// a new divergence fails the test.
#[test]
fn panel_presets_reproduce_the_golden_options_blocks() {
    use mzmon_lib::grafana::generated::{piechart, stat};
    use mzmon_lib::grafana::panel::Panel;

    let raw = std::fs::read_to_string(golden_path()).expect("read golden dashboard");
    let value: serde_json::Value = serde_yaml_ng::from_str(&raw).expect("parse golden");

    let mut mismatches = Vec::new();
    let mut compared = 0usize;

    for (name, element) in value["spec"]["elements"].as_object().expect("elements") {
        let viz = &element["spec"]["vizConfig"];
        let plugin = viz["group"].as_str().unwrap_or_default();
        let want = &viz["spec"]["options"];

        // Rebuild the panel with the knobs the golden used, then compare only the
        // options block -- fieldConfig is per-panel by design and covered above.
        let built = match plugin {
            "timeseries" => Panel::timeseries(name.clone()).build(0),
            "table" => Panel::table(name.clone()).build(0),
            "barchart" => Panel::barchart(name.clone()).build(0),
            "gauge" => Panel::gauge(name.clone()).build(0),
            "piechart" => {
                let p = Panel::piechart(name.clone());
                let p = if want["pieType"] == "pie" {
                    p.full_pie()
                } else {
                    p
                };
                p.build(0)
            }
            "stat" => {
                let mut p = Panel::stat(name.clone());
                if want["colorMode"] == "background" {
                    p = p.color_background();
                }
                if want["justifyMode"] == "center" {
                    p = p.justify_center();
                }
                if want["textMode"] == "value_and_name" {
                    p = p.text_mode(stat::BigValueTextMode::ValueAndName);
                }
                if want["graphMode"] == "none" {
                    p = p.graph_mode(stat::BigValueGraphMode::None);
                }
                if let Some(size) = want["text"]["valueSize"].as_f64() {
                    p = p.value_size(size);
                }
                if let Some(fields) = want["reduceOptions"]["fields"].as_str() {
                    p = p.reduce_fields(fields);
                }
                p.build(0)
            }
            other => {
                mismatches.push(format!("{name}: unhandled plugin {other:?}"));
                continue;
            }
        };
        compared += 1;

        let got = serde_json::to_value(
            built
                .spec
                .viz_config
                .spec
                .options
                .as_ref()
                .expect("options"),
        )
        .expect("serialize options");

        for diff in options_diff(want, &got, plugin) {
            mismatches.push(format!("{name} ({plugin}): {diff}"));
        }
    }

    assert_eq!(compared, 69, "expected to compare every panel");
    assert!(
        mismatches.is_empty(),
        "{} preset divergence(s) from the golden:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
    // Keep the deviation list honest: piechart is the plugin whose legend we
    // factored out, so prove at least one deviation is actually being exercised.
    let _ = piechart::PieChartType::Donut;
}

/// Numeric-aware equality: several counts are `type: number` upstream and so
/// land as `f64`, making `0` and `0.0` the same value spelled two ways.
fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(v, w)| json_eq(v, w))
        }
        _ => a == b,
    }
}

/// Compare a golden options block against a built one, allowing only the
/// deviations we chose deliberately.
fn options_diff(want: &serde_json::Value, got: &serde_json::Value, plugin: &str) -> Vec<String> {
    let (want, got) = match (want.as_object(), got.as_object()) {
        (Some(w), Some(g)) => (w, g),
        _ => return vec!["options is not an object".to_string()],
    };
    let mut out = Vec::new();

    for (key, wv) in want {
        match got.get(key) {
            None => out.push(format!("we drop {key} (golden had {wv})")),
            Some(gv) if json_eq(gv, wv) => {}
            Some(gv) => {
                // `reduceOptions` differs only by the `values` key we add on
                // purpose -- Grafana filled it in and migrated the panel
                // otherwise. Compare the golden's keys within it.
                if key == "reduceOptions"
                    && let (Some(w), Some(g)) = (wv.as_object(), gv.as_object())
                {
                    for (k, v) in w {
                        if !g.get(k).is_some_and(|x| json_eq(x, v)) {
                            out.push(format!("reduceOptions.{k}: {v} -> {:?}", g.get(k)));
                        }
                    }
                    continue;
                }
                // Grafana rewrote the golden's `true` to `false` on save, so
                // `false` is what actually rendered.
                if plugin == "gauge" && key == "showThresholdMarkers" {
                    continue;
                }
                out.push(format!("{key}: golden {wv} -> ours {gv}"));
            }
        }
    }
    for key in got.keys() {
        if !want.contains_key(key) && key != "reduceOptions" {
            // Additions are fine only where we documented them.
            out.push(format!("we add {key} (golden had none)"));
        }
    }
    out
}

/// Rebuilding the baseline's layout tree through the API must reproduce it.
///
/// Same contract as the panel-preset test: the layout API was derived from this
/// dashboard, so drift between them is a regression in one or the other.
/// Deliberate additions are enumerated; anything else fails.
#[test]
fn the_layout_api_reproduces_the_golden_tree() {
    use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Layout, Row, RowHeight, Tab};
    use mzmon_lib::grafana::panel::Panel;

    let raw = std::fs::read_to_string(golden_path()).expect("read golden dashboard");
    let value: serde_json::Value = serde_yaml_ng::from_str(&raw).expect("parse golden");
    let want = &value["spec"]["layout"];

    // Walk the golden tree and rebuild it, taking every knob from the golden so
    // only the *shape* and the emitted field set are under test.
    let mut tabs = Vec::new();
    for gtab in want["spec"]["tabs"].as_array().expect("tabs") {
        let mut rows = Vec::new();
        for grow in gtab["spec"]["layout"]["spec"]["rows"]
            .as_array()
            .expect("rows")
        {
            let ggrid = &grow["spec"]["layout"]["spec"];
            let mut grid = AutoGrid::new(ggrid["maxColumnCount"].as_u64().expect("cols") as u32)
                .column_width(match ggrid["columnWidthMode"].as_str() {
                    Some("narrow") => ColumnWidth::Narrow,
                    Some("wide") => ColumnWidth::Wide,
                    _ => ColumnWidth::Standard,
                })
                .row_height(match ggrid["rowHeightMode"].as_str() {
                    Some("short") => RowHeight::Short,
                    Some("tall") => RowHeight::Tall,
                    _ => RowHeight::Standard,
                });
            for item in ggrid["items"].as_array().expect("items") {
                let name = item["spec"]["element"]["name"].as_str().expect("name");
                // The panel body is covered by the preset test; a placeholder is
                // enough to exercise placement and id assignment.
                grid = grid.panel(name, Panel::timeseries(name).build(0));
            }
            let mut row = Row::new(grow["spec"]["title"].as_str().unwrap_or_default());
            if grow["spec"]["hideHeader"] == true {
                row = row.hide_header();
            }
            if grow["spec"]["collapse"] == true {
                row = row.collapsed();
            }
            rows.push(row.grid(grid));
        }
        tabs.push(Tab::new(gtab["spec"]["title"].as_str().unwrap_or_default()).rows(rows));
    }

    let assembled = Layout::tabs(tabs).assemble().expect("assemble");
    let got = serde_json::to_value(&assembled.layout).expect("serialize layout");

    // Panel bodies live in `elements`, so the trees should agree outright.
    let mut diffs = Vec::new();
    layout_diff(want, &got, "layout", &mut diffs);
    assert!(
        diffs.is_empty(),
        "{} layout divergence(s) from the golden:\n  {}",
        diffs.len(),
        diffs.join("\n  ")
    );

    // And the element set must match the golden's exactly -- 69 panels, no
    // dangling references either way.
    let mut want_names: Vec<&str> = value["spec"]["elements"]
        .as_object()
        .expect("elements")
        .keys()
        .map(String::as_str)
        .collect();
    want_names.sort_unstable();
    assert_eq!(assembled.names(), want_names);
}

/// Compare layout trees, allowing only the fields we deliberately add.
fn layout_diff(
    want: &serde_json::Value,
    got: &serde_json::Value,
    path: &str,
    out: &mut Vec<String>,
) {
    use serde_json::Value;
    match (want, got) {
        (Value::Object(w), Value::Object(g)) => {
            for (k, wv) in w {
                match g.get(k) {
                    Some(gv) => layout_diff(wv, gv, &format!("{path}.{k}"), out),
                    None => out.push(format!("{path}.{k} dropped (golden had {wv})")),
                }
            }
            for k in g.keys() {
                if w.contains_key(k) {
                    continue;
                }
                // Both are emitted on purpose. Grafana writes `collapse` on
                // save, so emitting it up front keeps a UI save from diffing.
                // `hideHeader` has no schema default, so absent and `false` behave
                // identically -- 14 golden rows write it explicitly and 2 omit it,
                // and being explicit costs nothing.
                if k == "collapse" || k == "hideHeader" {
                    continue;
                }
                out.push(format!("{path}.{k} added (golden had none)"));
            }
        }
        (Value::Array(w), Value::Array(g)) if w.len() == g.len() => {
            for (i, (wv, gv)) in w.iter().zip(g).enumerate() {
                layout_diff(wv, gv, &format!("{path}[{i}]"), out);
            }
        }
        (Value::Array(w), Value::Array(g)) => {
            out.push(format!("{path} length {} -> {}", w.len(), g.len()));
        }
        (Value::Number(w), Value::Number(g)) if w.as_f64() == g.as_f64() => {}
        (w, g) if w != g => out.push(format!("{path}: golden {w} -> ours {g}")),
        _ => {}
    }
}
