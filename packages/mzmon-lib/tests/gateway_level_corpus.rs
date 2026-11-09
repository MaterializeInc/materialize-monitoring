//! Runs the gateway's level detection through the Alloy binary.
//!
//! The UNKNOWN detectors in `gateway.yaml` are LogQL line filters and regexes
//! rendered through three layers of escaping, and a mistake in any of them
//! still validates: the detector just never matches. So this test renders the
//! real `loki.process "inputProcessor"`, runs it on `alloy run` behind a
//! `loki.source.api` and in front of a `loki.echo`, pushes every line in
//! `levels.corpus.yaml`, and checks the level and the `levelheuristic` each one
//! leaves with.
//!
//! It skips when `alloy` is not on `PATH`, unless `MZMON_REQUIRE_ALLOY` is set,
//! as it is in the CI job that pins the binary. The Alloy helpers are shared
//! with `secretfilter_corpus.rs`, in `common/alloy.rs`.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat};
use common::alloy::{Alloy, alloy_available, http, label_set, logfmt_fields};
use mzmon_lib::alloy::ast::{AttributeValue, Expression, ToBlock};
use mzmon_lib::alloy::pipeline::Pipeline;
use serde::Deserialize;

fn pipelines_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines")
}

// --- the corpus --------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    id: String,
    labels: BTreeMap<String, String>,
    line: String,
    level: String,
    #[serde(default)]
    heuristic: Option<String>,
}

fn corpus() -> Vec<Fixture> {
    let yaml = std::fs::read_to_string(pipelines_dir().join("levels.corpus.yaml"))
        .expect("read the corpus");
    let fixtures: Vec<Fixture> = serde_saphyr::from_str(&yaml).expect("parse the corpus");
    let mut ids = BTreeSet::new();
    for f in &fixtures {
        assert!(ids.insert(f.id.clone()), "fixture {} appears twice", f.id);
    }
    fixtures
}

/// `inputProcessor` from `gateway.yaml`, forwarding to `loki.echo.out` alone.
fn input_processor() -> String {
    let yaml = std::fs::read_to_string(pipelines_dir().join("gateway.yaml")).expect("read gateway");
    let pipeline = Pipeline::from_yaml_str(&yaml).expect("parse gateway");
    let mut block = pipeline
        .blocks
        .iter()
        .map(|b| b.to_block().expect("to_block"))
        .find(|b| b.component == "loki.process" && b.label.as_deref() == Some("inputProcessor"))
        .expect("gateway.yaml defines loki.process \"inputProcessor\"");
    block.attributes.insert(
        "forward_to".to_string(),
        AttributeValue::Array(vec![AttributeValue::Expression(Expression::name_to_ref(
            "loki.echo.out.receiver",
        ))]),
    );
    block.render().expect("render inputProcessor")
}

// --- reading what Alloy printed ----------------------------------------------

/// What `loki.echo` printed for one fixture.
struct Echoed {
    labels: BTreeMap<String, String>,
    metadata: BTreeMap<String, String>,
}

/// Every fixture `loki.echo` printed, by id, once all `n` have arrived.
fn echoed(alloy: &Alloy, n: usize) -> BTreeMap<String, Echoed> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let mut out = BTreeMap::new();
        for line in alloy.log_text().lines() {
            let fields = logfmt_fields(line);
            if fields.get("component_id").map(String::as_str) != Some("loki.echo.out") {
                continue;
            }
            let labels = label_set(fields.get("labels").map_or("", String::as_str));
            let metadata = fields
                .get("structured_metadata")
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();
            if let Some(id) = labels.get("fixture").cloned() {
                out.insert(id, Echoed { labels, metadata });
            }
        }
        if out.len() >= n {
            return out;
        }
        assert!(
            Instant::now() < deadline,
            "{} of {n} fixtures came out of inputProcessor:\n{}",
            out.len(),
            alloy.log_text()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[test]
fn every_corpus_line_leaves_with_its_level_and_detector() {
    if !alloy_available() {
        return;
    }
    let fixtures = corpus();
    let config = format!(
        r#"loki.source.api "in" {{
    http {{
        listen_address = "127.0.0.1"
        listen_port = LOKI_PORT
    }}
    forward_to = [loki.process.inputProcessor.receiver]
}}

loki.echo "out" {{}}

{}"#,
        input_processor()
    );
    let (alloy, loki_port) = Alloy::start(&config);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Fills `<<NOW>>`, because a timestamp a parser adopts has to be inside
    // the two hours `inputProcessor` keeps.
    let stamp =
        DateTime::from_timestamp_nanos(now as i64).to_rfc3339_opts(SecondsFormat::Nanos, true);
    let streams: Vec<_> = fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut labels = f.labels.clone();
            labels.insert("fixture".to_string(), f.id.clone());
            serde_json::json!({
                "stream": labels,
                "values": [[(now + i as u128).to_string(), f.line.replace("<<NOW>>", &stamp)]],
            })
        })
        .collect();
    let body = serde_json::json!({ "streams": streams }).to_string();
    let (status, response) =
        http(loki_port, "POST", "/loki/api/v1/push", &body).expect("push the corpus");
    assert_eq!(status, 204, "push refused: {response}");

    let out = echoed(&alloy, fixtures.len());
    let mut failures = Vec::new();
    for f in &fixtures {
        let got = &out[&f.id];
        let level = got.labels.get("level").map_or("<none>", String::as_str);
        let heuristic = got.metadata.get("levelheuristic").map(String::as_str);
        if level != f.level || heuristic != f.heuristic.as_deref() {
            failures.push(format!(
                "{}: level {level}, levelheuristic {heuristic:?}; want {}, {:?}\n    {}",
                f.id, f.level, f.heuristic, f.line
            ));
        }
        if got.labels.contains_key("journal_priority") {
            failures.push(format!("{}: journal_priority left as a stream label", f.id));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
