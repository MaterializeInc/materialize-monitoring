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
