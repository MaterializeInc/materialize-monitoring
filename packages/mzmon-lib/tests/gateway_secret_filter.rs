//! Holds the gateway's log path to the secret filter's placement.
//!
//! `loki.secretfilter` scans only the line, so it protects what is stored only
//! if no log entry can reach a sink without passing through it. These tests
//! read the pipeline as a graph of `forward_to` edges, across `gateway.yaml`
//! and the source and destination stubs it is validated with, and walk it.
//!
//! The chart renders the real `loki.source.api` from a helper that mirrors the
//! source stub; `charts/materialize-monitoring/tests/secret_filter_test.yaml`
//! holds the helper to the same edge.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mzmon_lib::alloy::ast::{AttributeValue, Expressable, ToBlock};
use mzmon_lib::alloy::components::loki::StageBlock;
use mzmon_lib::alloy::pipeline::{ComponentBlock, Pipeline};

const PIPELINES: &[&str] = &[
    "gateway.yaml",
    "gateway-sources-stub.yaml",
    "gateway-dest-stub.yaml",
];

fn load(name: &str) -> Pipeline {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../alloy-pipelines")
        .join(name);
    let yaml = std::fs::read_to_string(&path).expect("read pipeline");
    Pipeline::from_yaml_str(&yaml).expect("parse pipeline")
}

/// Every log component, keyed `<component>.<label>`, with the components its
/// `forward_to` names.
fn log_graph() -> BTreeMap<String, Vec<String>> {
    let mut graph = BTreeMap::new();
    for block in PIPELINES.iter().flat_map(|name| load(name).blocks) {
        let block = block.to_block().expect("to_block");
        let is_log =
            block.component.starts_with("loki.") || block.component == "otelcol.exporter.loki";
        if !is_log {
            continue;
        }
        let id = format!(
            "{}.{}",
            block.component,
            block.label.expect("every log component is labelled")
        );
        let targets = match block.attributes.get("forward_to") {
            Some(AttributeValue::Array(items)) => items
                .iter()
                .map(|item| match item {
                    AttributeValue::Expression(e) => e
                        .ref_name
                        .as_deref()
                        .and_then(|r| r.strip_suffix(".receiver"))
                        .expect("forward_to names a receiver")
                        .to_string(),
                    other => panic!("{id}: forward_to holds {other:?}"),
                })
                .collect(),
            _ => Vec::new(),
        };
        graph.insert(id, targets);
    }
    graph
}

fn is_source(id: &str) -> bool {
    id.starts_with("loki.source.") || id.starts_with("otelcol.exporter.loki.")
}

fn is_sink(id: &str) -> bool {
    id.starts_with("loki.write.") || id.starts_with("loki.echo.")
}

fn is_filter(id: &str) -> bool {
    id.starts_with("loki.secretfilter.")
}

/// A path from `from` to a sink that passes no secret filter, if there is one.
fn unfiltered_path(graph: &BTreeMap<String, Vec<String>>, from: &str) -> Option<Vec<String>> {
    fn walk(
        graph: &BTreeMap<String, Vec<String>>,
        node: &str,
        path: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
    ) -> bool {
        if is_filter(node) || !seen.insert(node.to_string()) {
            return false;
        }
        path.push(node.to_string());
        if is_sink(node) {
            return true;
        }
        let next = graph
            .get(node)
            .unwrap_or_else(|| panic!("{node} is forwarded to but not defined"));
        if next.iter().any(|n| walk(graph, n, path, seen)) {
            return true;
        }
        path.pop();
        false
    }
    let mut path = Vec::new();
    walk(graph, from, &mut path, &mut BTreeSet::new()).then_some(path)
}

#[test]
fn every_log_source_reaches_a_sink_only_through_a_secret_filter() {
    let graph = log_graph();
    let sources: Vec<&String> = graph.keys().filter(|id| is_source(id)).collect();
    // The walk is vacuous if the sources it starts from go missing.
    for expected in [
        "loki.source.api.gateway",
        "loki.source.kubernetes.agent_logs",
        "loki.source.kubernetes_events.k8sEvents",
        "otelcol.exporter.loki.bridge",
    ] {
        assert!(
            sources.iter().any(|s| *s == expected),
            "{expected} is missing"
        );
    }
    for source in sources {
        if let Some(path) = unfiltered_path(&graph, source) {
            panic!("reaches a sink unfiltered: {}", path.join(" -> "));
        }
    }
}

#[test]
fn every_path_into_input_processor_comes_from_a_secret_filter() {
    // inputProcessor copies text out of the line, so a sink reached through a
    // filter is not enough: the filter has to run before inputProcessor.
    let graph = log_graph();
    let feeders: Vec<&String> = graph
        .iter()
        .filter(|(_, targets)| targets.iter().any(|t| t == "loki.process.inputProcessor"))
        .map(|(id, _)| id)
        .collect();
    assert!(!feeders.is_empty());
    for feeder in feeders {
        assert!(is_filter(feeder), "{feeder} forwards to inputProcessor");
    }
}

/// The prefilter's level sniff, read from `gateway.yaml` so the test checks
/// the expression that ships.
fn sniff() -> regex::Regex {
    let expression = load("gateway.yaml")
        .blocks
        .into_iter()
        .find_map(|block| match block {
            ComponentBlock::LokiProcess(p) if p.label.as_deref() == Some("inputPrefilter") => {
                p.blocks.into_iter().find_map(|stage| match stage {
                    StageBlock::Regex(r) if r.expression.contains("sniffed_level") => {
                        Some(r.expression)
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("inputPrefilter sniffs the level");
    regex::Regex::new(&expression).expect("the sniff compiles")
}

#[test]
fn the_level_sniff_puts_only_debug_and_trace_in_the_bulk_tier() {
    let sniff = sniff();
    let bulk = |line: &str| sniff.captures(line).map(|c| c["sniffed_level"].to_string());
    for (line, tier) in [
        (
            r#"{"timestamp":"2026-11-08T16:49:29.676746Z","level":"DEBUG","fields":{"message":"m"},"target":"mz_adapter"}"#,
            Some("DEBUG"),
        ),
        (
            r#"{"timestamp":"2026-11-08T16:49:29.676746Z","level":"TRACE","fields":{"message":"m"},"target":"mz_adapter"}"#,
            Some("TRACE"),
        ),
        (
            "2026-11-08T16:49:20.826176Z DEBUG HTTP request{request.uri=/metrics}: mz_orchestratord: started",
            Some("DEBUG"),
        ),
        (
            "2026-11-08T16:49:20.826176Z TRACE reconcile{}: mz_orchestratord::controller: tick",
            Some("TRACE"),
        ),
        (
            r#"ts=2026-11-08T16:49:20Z level=debug msg="scrape done""#,
            Some("debug"),
        ),
        (
            r#"time=2026-11-08T16:49:20Z level=DEBUG msg="slog""#,
            Some("DEBUG"),
        ),
        // Everything below goes to the full tier. The bulk tier is rate
        // limited, so a line wrongly sniffed into it can be dropped.
        (
            r#"{"timestamp":"2026-11-08T16:49:29.676746Z","level":"INFO","fields":{"message":"m"},"target":"mz_adapter"}"#,
            None,
        ),
        (
            r#"{"timestamp":"2026-11-08T16:49:29.676746Z","level":"ERROR","fields":{"inner":{"level":"DEBUG"}},"target":"t"}"#,
            None,
        ),
        (
            "2026-11-08T16:49:20.826176Z  INFO mz_orchestratord: reconciled",
            None,
        ),
        (
            "2026-11-08T16:49:20.826176Z  thread 'timely:work-0' panicked at src/string.rs:1333:5:",
            None,
        ),
        (
            r#"time=2026-11-08T16:49:20Z level=INFO msg="saw level=debug in config""#,
            None,
        ),
        ("Starting debug server on :6060", None),
        ("DEBUG without a timestamp", None),
    ] {
        assert_eq!(bulk(line).as_deref(), tier, "{line}");
    }
}

#[test]
fn every_secret_filter_reads_a_shipped_rule_file() {
    // The chart ships each file in packages/alloy-pipelines/secretfilter/ as
    // /etc/alloy/secretfilter-<name>, and secretfilter_corpus.rs holds each one
    // to the corpus and to being flat. Any other file is neither tested nor
    // safe: two instances that load an `[extend] path` chain in one process
    // race at startup, and one can come up with no rules.
    const MOUNT: &str = "/etc/alloy/secretfilter-";
    let shipped = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines/secretfilter");
    let filters: Vec<_> = load("gateway.yaml")
        .blocks
        .into_iter()
        .filter_map(|block| match block {
            ComponentBlock::LokiSecretfilter(f) => Some(f),
            _ => None,
        })
        .collect();
    assert!(!filters.is_empty());
    for f in filters {
        let label = f.label.as_deref().unwrap_or_default();
        let path = match &f.gitleaks_config {
            Some(Expressable::Literal(path)) => path,
            other => panic!("{label}: gitleaks_config is {other:?}, not a literal path"),
        };
        let file = path
            .strip_prefix(MOUNT)
            .unwrap_or_else(|| panic!("{label}: {path} is outside {MOUNT}"));
        assert!(
            shipped.join(file).is_file(),
            "{label}: {file} is not in packages/alloy-pipelines/secretfilter/"
        );
    }
}
