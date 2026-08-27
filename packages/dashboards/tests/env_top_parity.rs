//! Parity ledger for the env-top port.
//!
//! Compares the Rust dashboard against the pre-rendered baseline the Python
//! emits, and reports what is still missing. While the port is in progress this
//! is the checklist; once every tab lands, `no_panel_diverges_from_the_baseline`
//! becomes the regression test.
//!
//! Deliberate divergences are enumerated in one place ([`ALLOWED`]) rather than
//! scattered through assertions, so the list of things we changed on purpose stays
//! readable and small.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mz_dashboards::grafana::env_top;
use mz_dashboards::grafana::render::canonical;
use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::variable;

/// Fields whose divergence from the baseline is intentional.
///
/// Each is justified in the module that produces it.
const ALLOWED: &[(&str, &str)] = &[
    (
        "cursorSync",
        "baseline left it Off; a correlation dashboard wants a shared crosshair",
    ),
    (
        "liveNow",
        "Grafana writes it on save, so we emit it up front",
    ),
    (
        "annotations",
        "the built-in Annotations & Alerts query, which Grafana adds on save",
    ),
    (
        "thresholds.steps[0].value",
        "an explicit null base step; the baseline omits the base entirely",
    ),
    (
        "variable order",
        "the ad-hoc filter moves last and the environment first, so the controls row \
         reads as a narrowing funnel; the baseline's order was an artifact of \
         registering datasources before variables",
    ),
    (
        "errors/load threshold spacing",
        "respaced so the last colour lands on max",
    ),
    (
        "environmentIdList -> environmentNameList",
        "the old name was wrong: it holds organization names",
    ),
];

/// Panel descriptions that deliberately differ from the baseline.
///
/// Two of these fix broken cross-references: the baseline points readers at tabs
/// that do not exist. `no_description_points_at_a_tab_that_does_not_exist` in
/// `env_top` is what stops that recurring.
fn registry() -> mzmon_lib::query::QueryRegistry {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
    mzmon_lib::query::QueryRegistry::from_directory(&dir).expect("load the query registry")
}

fn baseline() -> serde_json::Value {
    // A frozen fixture, not the checked-in artifact: the Rust generator writes that
    // file now, so reading it would compare the output to itself and invert every
    // deliberate-divergence assertion. See `mzmon-lib/tests/fixtures/README.md`.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../mzmon-lib/tests/fixtures/env-top.python-baseline.yaml");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read the frozen Python baseline at {}: {e}", path.display()));
    serde_yaml_ng::from_str(&raw).expect("parse the baseline")
}

/// Baseline panels, keyed by element name, with the tab they sit on.
fn baseline_panels() -> BTreeMap<String, (String, serde_json::Value)> {
    let baseline = baseline();
    let elements = baseline["spec"]["elements"].as_object().expect("elements");
    let mut out = BTreeMap::new();
    for tab in baseline["spec"]["layout"]["spec"]["tabs"]
        .as_array()
        .expect("tabs")
    {
        let title = tab["spec"]["title"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        for row in tab["spec"]["layout"]["spec"]["rows"]
            .as_array()
            .expect("rows")
        {
            for item in row["spec"]["layout"]["spec"]["items"]
                .as_array()
                .expect("items")
            {
                let name = item["spec"]["element"]["name"].as_str().expect("name");
                out.insert(name.to_string(), (title.clone(), elements[name].clone()));
            }
        }
    }
    out
}

fn ours() -> dashboardv2::Dashboard {
    env_top::build(mz_dashboards::grafana::Cloud::Generic, "mz_", &registry())
        .expect("build the dashboard")
        .spec
}

#[test]
fn the_ported_panels_carry_the_baseline_titles() {
    // A title is the panel's contract with the reader, so it has to match the
    // baseline exactly. Descriptions no longer do: they come from the query
    // registry now, which is deliberately better prose than the baseline carried
    // -- see `descriptions_come_from_the_registry` below.
    let baseline = baseline_panels();
    let ours = ours();

    let mut mismatches = Vec::new();
    for (name, element) in &ours.elements {
        let dashboardv2::Element::PanelKind(panel) = element else {
            continue;
        };
        let Some((_, want)) = baseline.get(name) else {
            mismatches.push(format!("{name}: not present in the baseline"));
            continue;
        };
        let want_title = want["spec"]["title"].as_str().unwrap_or_default();
        if panel.spec.title != want_title {
            mismatches.push(format!(
                "{name}: title {:?} != baseline {want_title:?}",
                panel.spec.title
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} panel(s) diverge:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

#[test]
fn the_ported_panels_use_the_baseline_plugin_and_unit() {
    let baseline = baseline_panels();
    let ours = ours();

    let mut mismatches = Vec::new();
    for (name, element) in &ours.elements {
        let dashboardv2::Element::PanelKind(panel) = element else {
            continue;
        };
        let Some((_, want)) = baseline.get(name) else {
            continue;
        };

        let want_plugin = want["spec"]["vizConfig"]["group"]
            .as_str()
            .unwrap_or_default();
        if panel.spec.viz_config.group != want_plugin {
            mismatches.push(format!(
                "{name}: plugin {} != baseline {want_plugin}",
                panel.spec.viz_config.group
            ));
        }

        let want_unit =
            want["spec"]["vizConfig"]["spec"]["fieldConfig"]["defaults"]["unit"].as_str();
        let our_unit = panel
            .spec
            .viz_config
            .spec
            .field_config
            .defaults
            .unit
            .as_deref();
        if our_unit != want_unit {
            mismatches.push(format!(
                "{name}: unit {our_unit:?} != baseline {want_unit:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} panel(s) diverge:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

/// Panels whose registry query differs from the baseline's expression, by class.
///
/// The dashboard takes its queries from the registry now, so the registry is the
/// source of truth and these are recorded rather than reconciled. Three of the four
/// classes are registry gaps with a matching `#[ignore]`d unit test naming them;
/// the fourth is cosmetic.
///
/// The list is checked in both directions: an entry that no longer diverges fails
/// too, so it cannot rot into a stale excuse.
const QUERY_DIVERGENCES: &[(&str, &str)] = &[
    // The registry's `orZero` helper parenthesizes its operand, so `(X) or
    // vector(0)` where the baseline wrote `X or vector(0)`.
    ("active-indexes", "orZero parenthesizes its operand"),
    (
        "hydration-unhydrated-count",
        "orZero parenthesizes its operand",
    ),
    ("replica-count", "orZero parenthesizes its operand"),
    ("storage-active-sinks", "orZero parenthesizes its operand"),
    ("storage-active-sources", "orZero parenthesizes its operand"),
    ("storage-active-tables", "orZero parenthesizes its operand"),
    (
        "summary-currently-hydrating",
        "orZero parenthesizes its operand",
    ),
    // Quantiles written `0.50` / `0.90` rather than `0.5` / `0.9`.
    (
        "queries-peek-latency-p50",
        "the registry writes the quantile as 0.50",
    ),
    (
        "queries-peek-latency-p90",
        "the registry writes the quantile as 0.90",
    ),
    (
        "sinks-iceberg-commit-latency",
        "the registry writes the p50/p90 quantiles as 0.50 / 0.90",
    ),
    // `max by (source_id)` already collapses across `job`, so the baseline's inner
    // `max without (job)` was redundant in that one expression.
    (
        "sources-errors",
        "the aggregation subsumes the max-without-job dedup",
    ),
    // `topk` sits inside the template, so the enrichment functions wrap it and it
    // ends up inside each branch of the name-join fallback rather than around the
    // whole thing. Equivalent, not a defect: the fallback is `... unless on (id)
    // ...`, so the branches are disjoint and their union is exactly the topk set.
    (
        "freshness-top-collections",
        "topk inside the enrichment; equivalent",
    ),
    (
        "hydration-slowest-collections",
        "topk inside the enrichment; equivalent",
    ),
];

/// The queries must be semantically the baseline's, modulo the variable rename.
///
/// Compared with whitespace collapsed: the baseline's expressions are hand-indented
/// strings and matching their exact newlines would be testing the formatter.
#[test]
fn the_ported_panels_query_the_same_thing() {
    let baseline = baseline_panels();
    let ours = ours();

    let mut mismatches = Vec::new();
    for (name, element) in &ours.elements {
        let dashboardv2::Element::PanelKind(panel) = element else {
            continue;
        };
        let Some((_, want)) = baseline.get(name) else {
            continue;
        };

        let want_exprs: Vec<String> = want["spec"]["data"]["spec"]["queries"]
            .as_array()
            .map(|qs| {
                qs.iter()
                    .map(|q| normalize(q["spec"]["query"]["spec"]["expr"].as_str().unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default();
        let our_exprs: Vec<String> = panel
            .spec
            .data
            .spec
            .queries
            .iter()
            .map(|q| {
                normalize(
                    q.spec
                        .query
                        .spec
                        .as_ref()
                        .and_then(|s| s.get("expr"))
                        .and_then(|e| e.as_str())
                        .unwrap_or(""),
                )
            })
            .collect();

        let allowed = QUERY_DIVERGENCES.iter().any(|(panel, _)| panel == name);
        if allowed {
            if our_exprs == want_exprs {
                mismatches.push(format!(
                    "{name}: listed as a query divergence but matches the baseline"
                ));
            }
            continue;
        }
        if our_exprs != want_exprs {
            for (index, (got, want)) in our_exprs.iter().zip(&want_exprs).enumerate() {
                if got != want {
                    mismatches.push(format!(
                        "{name} query {index}:\n      ours: {got}\n      base: {want}"
                    ));
                }
            }
            if our_exprs.len() != want_exprs.len() {
                mismatches.push(format!(
                    "{name}: {} queries != baseline {}",
                    our_exprs.len(),
                    want_exprs.len()
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} query divergence(s):\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

/// The variable set must match the baseline's, as a set; the order is ours.
///
/// Nothing compared variables before, which is how a reorder went in undocumented.
/// The *set* is a correctness property — a query referencing a variable no dashboard
/// defines interpolates to nothing and silently matches no series — while the order
/// is only what the controls row looks like, so they are asserted separately.
#[test]
fn the_variable_set_matches_the_baseline_modulo_the_rename() {
    let baseline = baseline();
    let ours = ours();

    let want: BTreeSet<String> = baseline["spec"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .map(|v| {
            let name = v["spec"]["name"].as_str().expect("name");
            // The one deliberate rename, allow-listed above.
            if name == "environmentIdList" {
                "environmentNameList".to_string()
            } else {
                name.to_string()
            }
        })
        .collect();
    let got: BTreeSet<String> = ours
        .variables
        .iter()
        .map(|v| variable::name_of(v).to_string())
        .collect();
    assert_eq!(got, want, "variable set differs from the baseline");

    // The order is deliberately not the baseline's: environment first, ad-hoc filter
    // last, so the controls row reads as a narrowing funnel.
    let order: Vec<&str> = ours.variables.iter().map(variable::name_of).collect();
    assert_eq!(
        order,
        vec![
            "metricsDatasource",
            "environmentNameList",
            "mzNamespaceList",
            "includeSystemClusters",
            "mzClusterList",
            "mzReplicaList",
            "metricAdhoc",
        ]
    );
    let baseline_order: Vec<&str> = baseline["spec"]["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .map(|v| v["spec"]["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        baseline_order[1], "metricAdhoc",
        "the baseline put the ad-hoc filter second; if that changed, the \
         `variable order` divergence is stale"
    );
}

/// The tab and row skeleton must match the baseline: titles, order, column caps.
///
/// The panel-level checks compare panels the baseline also has, so they say nothing
/// about the frame around them. This is the check whose absence let a row ship as
/// "Applications" instead of "SQL Control Plane Commands", and with a three-column
/// cap on a row holding one wide table.
#[test]
fn the_tab_and_row_skeleton_matches_the_baseline() {
    let baseline = baseline();
    let ours = ours();

    // Through `canonical` on both sides: the generated structs type a column count
    // as `f64` because the schema says `number`, so an uncanonicalized comparison
    // reports every `3.0` against the baseline's `3`.
    let want = skeleton(&canonical(baseline["spec"]["layout"].clone()));
    let got = skeleton(&canonical(
        serde_json::to_value(&ours.layout).expect("serialize"),
    ));

    let mut mismatches = Vec::new();
    for index in 0..want.len().max(got.len()) {
        match (got.get(index), want.get(index)) {
            (Some(g), Some(w)) if g == w => {}
            (Some(g), Some(w)) => mismatches.push(format!("ours: {g}\n      base: {w}")),
            (Some(g), None) => mismatches.push(format!("ours: {g}\n      base: <absent>")),
            (None, Some(w)) => mismatches.push(format!("ours: <absent>\n      base: {w}")),
            (None, None) => {}
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} skeleton divergence(s):\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

/// Flatten a layout to one comparable line per tab and row.
fn skeleton(layout: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    for tab in layout["spec"]["tabs"].as_array().into_iter().flatten() {
        let title = tab["spec"]["title"].as_str().unwrap_or("<untitled>");
        out.push(format!("tab {title:?}"));
        for row in tab["spec"]["layout"]["spec"]["rows"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let grid = &row["spec"]["layout"]["spec"];
            out.push(format!(
                "  row {:?} cols={} panels={}",
                row["spec"]["title"].as_str().unwrap_or("<untitled>"),
                grid["maxColumnCount"],
                grid["items"].as_array().map(|i| i.len()).unwrap_or(0),
            ));
        }
    }
    out
}

/// Transformations must match the baseline exactly, options included.
///
/// This is the check whose absence let the version panel ship without its
/// `extractFields` step: a transformation is invisible in a title/query/unit
/// comparison, but dropping one silently changes what a panel displays. Options
/// are compared whole rather than by transformation id, since an `organize` with
/// the wrong column order is as wrong as a missing one.
#[test]
fn the_ported_panels_transform_their_data_the_same_way() {
    let baseline = baseline_panels();
    let ours = ours();

    let mut mismatches = Vec::new();
    for (name, element) in &ours.elements {
        let dashboardv2::Element::PanelKind(panel) = element else {
            continue;
        };
        let Some((_, want)) = baseline.get(name) else {
            continue;
        };

        let want_transforms = want["spec"]["data"]["spec"]["transformations"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let our_transforms: Vec<serde_json::Value> = panel
            .spec
            .data
            .spec
            .transformations
            .iter()
            .map(|t| serde_json::to_value(t).expect("serialize transformation"))
            .collect();

        if our_transforms.len() != want_transforms.len() {
            mismatches.push(format!(
                "{name}: {} transformation(s) != baseline {} (ours: {:?}, base: {:?})",
                our_transforms.len(),
                want_transforms.len(),
                ids(&our_transforms),
                ids(&want_transforms),
            ));
            continue;
        }
        for (index, (got, want)) in our_transforms.iter().zip(&want_transforms).enumerate() {
            if got["group"] != want["group"] {
                mismatches.push(format!(
                    "{name} transformation {index}: {} != baseline {}",
                    got["group"], want["group"]
                ));
                continue;
            }
            // An absent options block and an empty one mean the same thing to
            // Grafana, so compare the maps rather than the enclosing values.
            let got_options = got["spec"]["options"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let want_options = want["spec"]["options"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            if got_options != want_options {
                mismatches.push(format!(
                    "{name} transformation {index} ({}) options differ:\n      ours: {}\n      base: {}",
                    got["group"],
                    serde_json::to_string(&got_options).unwrap_or_default(),
                    serde_json::to_string(&want_options).unwrap_or_default(),
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} transformation divergence(s):\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}

/// The transformation ids in order, for a readable count mismatch.
fn ids(transforms: &[serde_json::Value]) -> Vec<String> {
    transforms
        .iter()
        .map(|t| t["group"].as_str().unwrap_or("?").to_string())
        .collect()
}

/// Collapse whitespace and apply the deliberate variable rename.
fn normalize(expr: &str) -> String {
    // All whitespace goes, not just runs of it: whitespace is never significant in
    // PromQL outside string literals, and the registry's formatting is its own.
    let mut out: String = expr
        .replace("$environmentIdList", "$environmentNameList")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    // Redundant grouping parens go too. The registry's `orZero` helper wraps its
    // operand, so `(X)orvector(0)` and `Xorvector(0)` are the same query written
    // two ways -- comparing those as different would be testing the template
    // engine rather than the query.
    for _ in 0..8 {
        let collapsed = collapse_double_parens(&out);
        if collapsed == out {
            break;
        }
        out = collapsed;
    }
    out
}

/// Rewrite `((X))` as `(X)` once, for parenthesized groups containing no parens.
fn collapse_double_parens(expr: &str) -> String {
    let bytes = expr.as_bytes();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' && i + 1 < bytes.len() && bytes[i + 1] == b'(' {
            // Find the inner group's close.
            if let Some(inner_end) = expr[i + 2..].find(')').map(|p| i + 2 + p) {
                let inner = &expr[i + 2..inner_end];
                let closes_immediately = expr[inner_end + 1..].starts_with(')');
                if !inner.contains('(') && closes_immediately {
                    out.push('(');
                    out.push_str(inner);
                    out.push(')');
                    i = inner_end + 2;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// The ledger: what is ported, and what is left.
///
/// This does not fail on missing panels — the port is deliberately incremental —
/// but it does fail if coverage goes *backwards*, and it prints the remaining work
/// so the next tab is obvious.
#[test]
fn report_porting_coverage() {
    let baseline = baseline_panels();
    let ours = ours();
    let ported: BTreeSet<&String> = ours.elements.keys().collect();

    let mut by_tab: BTreeMap<&str, (usize, Vec<&str>)> = BTreeMap::new();
    for (name, (tab, _)) in &baseline {
        let entry = by_tab.entry(tab.as_str()).or_default();
        entry.0 += 1;
        if !ported.contains(name) {
            entry.1.push(name.as_str());
        }
    }

    eprintln!("\n=== env-top porting coverage ===");
    let mut done = 0usize;
    for (tab, (total, missing)) in &by_tab {
        let have = total - missing.len();
        done += have;
        eprintln!("  {tab:32} {have:2}/{total:2}");
        if !missing.is_empty() {
            eprintln!("      missing: {}", missing.join(", "));
        }
    }
    eprintln!("  {:32} {done:2}/{}", "TOTAL", baseline.len());
    eprintln!("\n=== deliberate divergences from the baseline ===");
    for (what, why) in ALLOWED {
        eprintln!("  {what}: {why}");
    }

    // Coverage may not regress. Bump as tabs land.
    const PORTED: usize = 69;
    assert_eq!(
        done, PORTED,
        "porting coverage changed; update PORTED if this was intentional"
    );

    // Nothing we ported should be absent from the baseline: a panel we invented
    // would mean the port drifted rather than progressed.
    let unknown: Vec<&&String> = ported
        .iter()
        .filter(|n| !baseline.contains_key(**n))
        .collect();
    assert!(
        unknown.is_empty(),
        "panels not in the baseline: {unknown:?}"
    );
}

#[test]
fn the_layout_matches_the_baseline_for_ported_tabs() {
    // Rows, grid settings, and placement order, for the tabs that exist.
    let baseline = baseline();
    let ours = ours();

    let want_tab = baseline["spec"]["layout"]["spec"]["tabs"]
        .as_array()
        .expect("tabs")
        .iter()
        .find(|t| t["spec"]["title"] == "Summary")
        .expect("the baseline has a Summary tab");

    let dashboardv2::DashboardLayout::TabsLayoutKind(tabs) = &ours.layout else {
        panic!("expected a tabbed layout");
    };
    let got_tab = tabs
        .spec
        .tabs
        .iter()
        .find(|t| t.spec.title.as_deref() == Some("Summary"))
        .expect("our Summary tab");

    let dashboardv2::TabsLayoutTabSpecLayout::RowsLayoutKind(got_rows) = &got_tab.spec.layout
    else {
        panic!("expected rows");
    };
    let want_rows = want_tab["spec"]["layout"]["spec"]["rows"]
        .as_array()
        .expect("rows");
    assert_eq!(got_rows.spec.rows.len(), want_rows.len());

    for (got, want) in got_rows.spec.rows.iter().zip(want_rows) {
        assert_eq!(got.spec.title.as_deref(), want["spec"]["title"].as_str());
        let dashboardv2::RowsLayoutRowSpecLayout::AutoGridLayoutKind(grid) = &got.spec.layout
        else {
            panic!("expected an auto grid");
        };
        let want_grid = &want["spec"]["layout"]["spec"];
        assert_eq!(
            grid.spec.max_column_count,
            want_grid["maxColumnCount"]
                .as_f64()
                .expect("maxColumnCount"),
            "row {:?} column count",
            got.spec.title
        );
        assert_eq!(
            grid.spec.row_height_mode.to_string(),
            want_grid["rowHeightMode"].as_str().unwrap_or_default(),
            "row {:?} height mode",
            got.spec.title
        );
        // Placement order is what a reader sees; it has to match.
        let got_names: Vec<&str> = grid
            .spec
            .items
            .iter()
            .map(|i| i.spec.element.name.as_str())
            .collect();
        let want_names: Vec<&str> = want_grid["items"]
            .as_array()
            .expect("items")
            .iter()
            .map(|i| i["spec"]["element"]["name"].as_str().expect("name"))
            .collect();
        assert_eq!(got_names, want_names, "row {:?} placement", got.spec.title);
    }
}

#[test]
fn the_shell_matches_the_baseline_where_we_did_not_deviate() {
    let baseline = baseline();
    let ours = ours();
    let want = &baseline["spec"];

    assert_eq!(Some(ours.title.as_str()), want["title"].as_str());
    assert_eq!(ours.description.as_deref(), want["description"].as_str());
    assert_eq!(ours.editable, want["editable"].as_bool().unwrap_or(true));
    assert_eq!(ours.preload, want["preload"].as_bool().unwrap_or(false));

    let want_tags: Vec<&str> = want["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .map(|t| t.as_str().expect("tag"))
        .collect();
    assert_eq!(ours.tags, want_tags);

    let want_time = &want["timeSettings"];
    assert_eq!(
        Some(ours.time_settings.from.as_str()),
        want_time["from"].as_str()
    );
    assert_eq!(
        Some(ours.time_settings.to.as_str()),
        want_time["to"].as_str()
    );
    assert_eq!(
        Some(ours.time_settings.timezone.as_str()),
        want_time["timezone"].as_str()
    );

    // And the one shell field we changed on purpose.
    assert_eq!(want["cursorSync"], "Off");
    assert_eq!(
        ours.cursor_sync,
        dashboardv2::DashboardCursorSync::Crosshair
    );
}

/// Every panel's description is its registry query's, rendered — none is retyped.
///
/// This is the invariant that replaced baseline-description parity. The point of
/// sourcing panels from the registry is that a query's semantics and the prose
/// explaining it are maintained in one place; a panel that hardcodes prose has
/// opted out of that, and will drift the first time the query changes.
#[test]
fn descriptions_come_from_the_registry() {
    let registry = registry();
    let scope = mzmon_lib::grafana::context::DashboardScope::default();
    let ctx = mzmon_lib::grafana::context::dashboard_context(
        &registry,
        mzmon_lib::query::QueryEngine::PromQl,
        &scope,
    );

    // Every description the registry can produce, by its rendered text. A panel's
    // description has to be one of these.
    let from_registry: BTreeSet<String> = registry
        .queries()
        .map(|query| mzmon_lib::grafana::query::format_description(&query.description))
        .collect();
    let _ = &ctx;

    let mut orphans = Vec::new();
    for (name, element) in &ours().elements {
        let dashboardv2::Element::PanelKind(panel) = element else {
            continue;
        };
        if panel.spec.description.is_empty() {
            orphans.push(format!("{name}: empty description"));
        } else if !from_registry.contains(&panel.spec.description) {
            orphans.push(format!("{name}: description is not any registry query's"));
        }
    }
    assert!(
        orphans.is_empty(),
        "{} panel(s) do not take their description from the registry:\n  {}",
        orphans.len(),
        orphans.join("\n  ")
    );
}
