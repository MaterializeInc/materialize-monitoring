//! Holds secret-filter shadow mode to leaving the main stream unchanged.
//!
//! A shadow copy of a matched line reaches `inputProcessor` beside its
//! original. If copies counted against `inputProcessor`'s rate limits, a rule
//! that matched most of a namespace's DEBUG lines would spend the budget their
//! originals need, and originals would be dropped. Shadow mode exists to find
//! rules like that, so the copies have a budget of their own in
//! `secretfilterShadow`.
//!
//! The functional test runs the shadow instance, `secretfilterShadow`, and
//! `inputProcessor` from `gateway.yaml` on `alloy run`. It skips when `alloy`
//! is not on `PATH`, unless `MZMON_REQUIRE_ALLOY` is set, as it is in the CI
//! job that pins the binary.

mod common;

use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::alloy::{Alloy, alloy_available, http};
use mzmon_lib::alloy::ast::{AttributeValue, Expression, ToBlock};
use mzmon_lib::alloy::pipeline::Pipeline;
use regex::Regex;
use serde_json::Value;

/// Matching DEBUG lines pushed from one namespace. They fit the DEBUG limit's
/// burst of 5,000 alone, and not with a copy of each.
const LINES: usize = 4000;

/// `secretfilterShadow`'s limit, which bounds the copies that come out.
const SHADOW_RATE: f64 = 100.0;
const SHADOW_BURST: f64 = 1000.0;

/// The one exclusion every `inputProcessor` rate limit's selector carries.
const SKIP_SHADOW: &str = r#"secretfilter!="shadow""#;

fn pipelines_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines")
}

fn gateway_yaml() -> String {
    std::fs::read_to_string(pipelines_dir().join("gateway.yaml")).expect("read gateway")
}

// --- inputProcessor's rate limits ---------------------------------------------

/// The selectors of the `stage.match` blocks enclosing every `stage.limit`
/// under `stages`, outermost first; an empty list is a limit no match encloses.
fn limits(stages: &Value, enclosing: &[String], out: &mut Vec<Vec<String>>) {
    for stage in stages.as_array().into_iter().flatten() {
        if stage.get("stage.limit").is_some() {
            out.push(enclosing.to_vec());
        }
        if let Some(m) = stage.get("stage.match") {
            let mut inner = enclosing.to_vec();
            inner.push(m["selector"].as_str().unwrap_or_default().to_string());
            limits(&m["blocks"], &inner, out);
        }
    }
}

#[test]
fn every_input_processor_rate_limit_skips_shadow_copies() {
    let yaml = gateway_yaml();
    let gateway: Value = mzmon_lib::yaml::from_str(&yaml).expect("parse gateway");
    let processor = gateway["blocks"]
        .as_array()
        .expect("gateway.yaml has blocks")
        .iter()
        .find_map(|b| {
            let p = b.get("loki.process")?;
            (p["label"] == "inputProcessor").then_some(p)
        })
        .expect("gateway.yaml defines loki.process \"inputProcessor\"");

    let mut found = Vec::new();
    limits(&processor["blocks"], &[], &mut found);
    assert!(
        !found.is_empty(),
        "inputProcessor has no rate limits to check"
    );
    for selectors in &found {
        assert!(
            selectors.iter().any(|s| s.contains(SKIP_SHADOW)),
            "an inputProcessor stage.limit sits under {selectors:?}, none of which has \
             {SKIP_SHADOW}, so shadow copies spend its budget"
        );
    }
}

// --- the same, under load ------------------------------------------------------

/// The gateway block `component "label"`, rendered with `attributes` replaced.
fn block(component: &str, label: &str, attributes: Vec<(&str, AttributeValue)>) -> String {
    let pipeline = Pipeline::from_yaml_str(&gateway_yaml()).expect("parse gateway");
    let mut block = pipeline
        .blocks
        .iter()
        .map(|b| b.to_block().expect("to_block"))
        .find(|b| b.component == component && b.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("gateway.yaml defines {component} \"{label}\""));
    for (name, value) in attributes {
        block.attributes.insert(name.to_string(), value);
    }
    block.render().expect("render")
}

fn refs(ids: &[&str]) -> AttributeValue {
    AttributeValue::Array(
        ids.iter()
            .map(|id| AttributeValue::Expression(Expression::name_to_ref(*id)))
            .collect(),
    )
}

/// The originals and the shadow copies `loki.echo` has printed.
///
/// Reads thousands of lines on every poll, so it matches the two fields it
/// needs rather than parsing each line in full.
fn echoed(alloy: &Alloy) -> (usize, usize) {
    static LABELS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"labels=("(?:[^"\\]|\\.)*")"#).unwrap());
    let (mut originals, mut copies) = (0, 0);
    for line in alloy
        .log_text()
        .lines()
        .filter(|l| l.contains("component_id=loki.echo.out "))
    {
        let labels: String = LABELS
            .captures(line)
            .and_then(|c| serde_json::from_str(&c[1]).ok())
            .unwrap_or_default();
        if labels.contains(r#"secretfilter="shadow""#) {
            copies += 1;
        } else {
            originals += 1;
        }
    }
    (originals, copies)
}

/// Lines the shadow instance has scanned.
fn scanned(alloy: &Alloy) -> f64 {
    let (_, body) = http(alloy.server_port, "GET", "/metrics", "").expect("scrape alloy");
    body.lines()
        .filter(|l| l.starts_with("loki_secretfilter_processing_duration_seconds_count{"))
        .filter(|l| l.contains(r#"component_id="loki.secretfilter.shadow""#))
        .filter_map(|l| l.rsplit(' ').next()?.parse::<f64>().ok())
        .sum()
}

#[test]
fn shadow_copies_do_not_cost_their_originals_a_rate_limit() {
    if !alloy_available() {
        return;
    }
    let rules = pipelines_dir().join("secretfilter/ingest.toml");
    // The originals go straight to inputProcessor: in shadow mode the ingest
    // filters forward every line unscanned.
    let config = format!(
        r#"loki.source.api "in" {{
    http {{
        listen_address = "127.0.0.1"
        listen_port = LOKI_PORT
    }}
    forward_to = [loki.process.inputProcessor.receiver, loki.secretfilter.shadow.receiver]
}}

loki.echo "out" {{}}

{}
{}
{}"#,
        block(
            "loki.secretfilter",
            "shadow",
            vec![(
                "gitleaks_config",
                AttributeValue::String(rules.display().to_string()),
            )],
        ),
        block("loki.process", "secretfilterShadow", vec![]),
        block(
            "loki.process",
            "inputProcessor",
            vec![("forward_to", refs(&["loki.echo.out.receiver"]))],
        ),
    );
    let (alloy, loki_port) = Alloy::start(&config);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let line = "level=debug caller=db.go:88 msg=connecting \
                url=postgres://materialize:Hunter2-not-real@mzmon-metadata-db:5432/materialize";
    let values: Vec<_> = (0..LINES)
        .map(|i| serde_json::json!([(now + i as u128).to_string(), line]))
        .collect();
    let body = serde_json::json!({ "streams": [{
        "stream": {"namespace": "chatty", "app": "loki", "container": "querier", "job": "chatty/querier"},
        "values": values,
    }]})
    .to_string();
    let pushed = Instant::now();
    let (status, response) =
        http(loki_port, "POST", "/loki/api/v1/push", &body).expect("push the lines");
    assert_eq!(status, 204, "push refused: {response}");

    // Done once the shadow instance has scanned every line and the echo has
    // stopped printing.
    let deadline = pushed + Duration::from_secs(60);
    let mut last = (usize::MAX, usize::MAX);
    let (originals, copies) = loop {
        std::thread::sleep(Duration::from_millis(500));
        let now = echoed(&alloy);
        if scanned(&alloy) >= LINES as f64 && now == last {
            break now;
        }
        assert!(
            Instant::now() < deadline,
            "the shadow instance scanned {} of {LINES} lines, and {now:?} came out:\n{}",
            scanned(&alloy),
            alloy.log_text()
        );
        last = now;
    };
    let elapsed = pushed.elapsed().as_secs_f64();

    assert_eq!(
        originals,
        LINES,
        "{} of {LINES} originals were dropped beside {copies} shadow copies",
        LINES - originals
    );
    assert!(copies > 0, "no shadow copy came out, so nothing was tested");
    let ceiling = SHADOW_BURST + SHADOW_RATE * elapsed.ceil();
    assert!(
        copies as f64 <= ceiling,
        "{copies} shadow copies came out in {elapsed:.1}s, past secretfilterShadow's limit of {ceiling}"
    );
}
