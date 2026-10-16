//! Holds the gateway's `inputProcessor` to the limits Loki enforces.
//!
//! Loki refuses an entry over its limits rather than shortening it, so the
//! pipeline's last stages cut each entry to fit. The structured metadata limit
//! is a total across an entry's keys, so no one `stage.truncate` rule can be
//! checked alone: a key added anywhere in `inputProcessor` spends the budget.
//! This test walks the pipeline in order, tracks the bound on every value that
//! can reach Loki, and redoes the sum.
//!
//! The walk is conservative. A key promoted inside a `stage.match` counts as
//! present, and only an unconditional `stage.truncate` rule with a literal
//! limit bounds anything.
//!
//! Structured metadata a Loki push client sends itself cannot be walked: it
//! arrives with the entry, under names this pipeline does not know, and no
//! Alloy stage drops keys by pattern or bounds a name or the number of keys.
//! The budget holds back a fixed reserve for it instead.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mzmon_lib::alloy::ast::Expressable;
use mzmon_lib::alloy::components::loki::{LokiProcessBlock, StageBlock, TruncateSubBlock};
use mzmon_lib::alloy::pipeline::{ComponentBlock, Pipeline};

/// Loki 3.7.8's default limits, measured against a running Loki: the largest
/// line, label value, and structured metadata total (key names included) it
/// accepts, and the most structured metadata entries.
const LOKI_MAX_LINE: u64 = 262_144;
const LOKI_MAX_LABEL_VALUE: u64 = 2_048;
const LOKI_MAX_STRUCTURED_METADATA: u64 = 65_536;
const LOKI_MAX_STRUCTURED_METADATA_ENTRIES: usize = 128;

/// Structured metadata, key names included, left for what a push client sends
/// itself. `loki.source.api` keeps it, and the gateway cannot bound it.
const PUSH_CLIENT_RESERVE: u64 = 16_384;

/// The limit on every structured metadata value the gateway sets, except `msg`.
const SHORT_VALUE: u64 = 512;

/// Bytes in a literal `stage.truncate` limit, read as Alloy reads it: base 2,
/// whether spelled `KiB` or `KB`.
fn bytes(limit: &str) -> u64 {
    let split = limit
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or_else(|| panic!("limit {limit:?} has no unit"));
    let (number, unit) = limit.split_at(split);
    let scale = match unit {
        "B" => 1,
        "KiB" | "KB" => 1 << 10,
        "MiB" | "MB" => 1 << 20,
        "GiB" | "GB" => 1 << 30,
        other => panic!("limit {limit:?} has a unit this test does not read: {other:?}"),
    };
    number.parse::<u64>().expect("integer limit") * scale
}

/// What can reach Loki after a `loki.process`, and the bound on each part.
/// `None` is unbounded.
#[derive(Debug, Default)]
struct Bounds {
    line: Option<u64>,
    labels: Option<u64>,
    structured_metadata: BTreeMap<String, Option<u64>>,
}

impl Bounds {
    fn of(process: &LokiProcessBlock) -> Self {
        let mut bounds = Bounds::default();
        for stage in &process.blocks {
            bounds.apply(stage, true);
        }
        bounds
    }

    /// Applies one stage. `unconditional` is false inside a `stage.match`,
    /// where a stage may or may not run, so it can add values but never bound
    /// or remove them.
    fn apply(&mut self, stage: &StageBlock, unconditional: bool) {
        match stage {
            StageBlock::Match(m) => {
                for nested in &m.blocks {
                    self.apply(nested, false);
                }
            }
            StageBlock::StructuredMetadata(sm) => {
                for key in sm.values.keys() {
                    self.structured_metadata.insert(key.clone(), None);
                }
            }
            StageBlock::StructuredMetadataDrop(drop) if unconditional => {
                for key in &drop.values {
                    self.structured_metadata.remove(key);
                }
            }
            StageBlock::Labels(_) | StageBlock::StaticLabels(_) => self.labels = None,
            StageBlock::Regex(r) if r.labels_from_groups == Some(true) => self.labels = None,
            StageBlock::Replace(r) if r.source.is_none() => self.line = None,
            StageBlock::Multiline(_) | StageBlock::Cri(_) => self.line = None,
            StageBlock::Truncate(t) if unconditional => {
                for rule in &t.blocks {
                    let TruncateSubBlock::Rule(rule) = rule else {
                        panic!("a raw truncate rule cannot be accounted for");
                    };
                    let Expressable::Literal(limit) = &rule.limit else {
                        // An environment-driven limit bounds nothing the test can check.
                        continue;
                    };
                    let limit = Some(bytes(limit));
                    match rule.source_type.as_deref().unwrap_or("line") {
                        "line" => self.line = min(self.line, limit),
                        "label" if rule.sources.is_empty() => {
                            self.labels = min(self.labels, limit);
                        }
                        "structured_metadata" => {
                            for (key, bound) in &mut self.structured_metadata {
                                if rule.sources.is_empty() || rule.sources.contains(key) {
                                    *bound = min(*bound, limit);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            StageBlock::Raw(block) => panic!("a raw {} cannot be accounted for", block.component),
            _ => {}
        }
    }

    /// Every way the entry can exceed Loki's limits.
    fn violations(&self) -> Vec<String> {
        let mut out = Vec::new();
        match self.line {
            Some(b) if b <= LOKI_MAX_LINE => {}
            other => out.push(format!("line bound {other:?} exceeds {LOKI_MAX_LINE}")),
        }
        match self.labels {
            Some(b) if b <= LOKI_MAX_LABEL_VALUE => {}
            other => out.push(format!(
                "label bound {other:?} exceeds {LOKI_MAX_LABEL_VALUE}"
            )),
        }
        let unbounded: Vec<_> = self
            .structured_metadata
            .iter()
            .filter(|(_, bound)| bound.is_none())
            .map(|(key, _)| key.as_str())
            .collect();
        if !unbounded.is_empty() {
            out.push(format!("structured metadata with no limit: {unbounded:?}"));
        }
        let total: u64 = self
            .structured_metadata
            .iter()
            .map(|(key, bound)| key.len() as u64 + bound.unwrap_or(0))
            .sum();
        if total + PUSH_CLIENT_RESERVE > LOKI_MAX_STRUCTURED_METADATA {
            out.push(format!(
                "structured metadata can total {total} bytes over {} keys, which with the \
                 {PUSH_CLIENT_RESERVE}-byte push-client reserve is above {LOKI_MAX_STRUCTURED_METADATA}",
                self.structured_metadata.len(),
            ));
        }
        if self.structured_metadata.len() > LOKI_MAX_STRUCTURED_METADATA_ENTRIES {
            out.push(format!(
                "{} structured metadata keys, above {LOKI_MAX_STRUCTURED_METADATA_ENTRIES}",
                self.structured_metadata.len(),
            ));
        }
        out
    }
}

/// The literal limits of every unconditional structured metadata rule that can
/// cut `key`, wherever the key came from, including a push client.
fn limits_reaching(process: &LokiProcessBlock, key: &str) -> Vec<u64> {
    let mut out = Vec::new();
    for stage in &process.blocks {
        let StageBlock::Truncate(t) = stage else {
            continue;
        };
        for rule in &t.blocks {
            let TruncateSubBlock::Rule(rule) = rule else {
                continue;
            };
            let Expressable::Literal(limit) = &rule.limit else {
                continue;
            };
            if rule.source_type.as_deref() == Some("structured_metadata")
                && (rule.sources.is_empty() || rule.sources.iter().any(|s| s == key))
            {
                out.push(bytes(limit));
            }
        }
    }
    out
}

fn min(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

fn input_processor(yaml: &str) -> LokiProcessBlock {
    let pipeline = Pipeline::from_yaml_str(yaml).expect("pipeline");
    pipeline
        .blocks
        .into_iter()
        .find_map(|block| match block {
            ComponentBlock::LokiProcess(p) if p.label.as_deref() == Some("inputProcessor") => {
                Some(p)
            }
            _ => None,
        })
        .expect("a loki.process labelled inputProcessor")
}

#[test]
fn gateway_entries_fit_lokis_limits() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines/gateway.yaml");
    let yaml = std::fs::read_to_string(&path).expect("read gateway.yaml");
    let process = input_processor(&yaml);
    let bounds = Bounds::of(&process);
    assert!(
        bounds.structured_metadata.contains_key("msg"),
        "the walk found no structured metadata; it is not reading the pipeline"
    );
    let violations = bounds.violations();
    assert!(violations.is_empty(), "{violations:#?}");

    // The 512B rule names its keys, so a key added to the pipeline and not to
    // that list falls through to the larger catch-all.
    let loose: Vec<_> = bounds
        .structured_metadata
        .iter()
        .filter(|(key, bound)| *key != "msg" && bound.is_some_and(|b| b > SHORT_VALUE))
        .map(|(key, _)| key.as_str())
        .collect();
    assert!(
        loose.is_empty(),
        "add {loose:?} to the `sources` of the {SHORT_VALUE}B rule in gateway.yaml"
    );
}

#[test]
fn an_incoming_msg_keeps_its_own_limit() {
    // A `msg` that arrives as structured metadata, from a chained gateway or
    // another push client, is promoted again only when this gateway extracts
    // one from the line. When it does not, the first rule to reach it is the
    // last word, so no rule may cut it below the limit a re-extracted `msg`
    // gets.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines/gateway.yaml");
    let yaml = std::fs::read_to_string(&path).expect("read gateway.yaml");
    let process = input_processor(&yaml);
    let limit = Bounds::of(&process).structured_metadata["msg"].expect("msg is bounded");
    let reaching = limits_reaching(&process, "msg");
    assert!(
        reaching.iter().all(|&l| l >= limit),
        "a rule cuts an incoming msg to {reaching:?}, below its {limit}-byte limit"
    );
}

#[test]
fn a_key_promoted_after_the_limits_is_caught() {
    // The negative control: the same shape as the gateway's tail, with one key
    // promoted after the rule that should have bounded it.
    let bounds = Bounds::of(&input_processor(
        r#"
        blocks:
          - loki.process:
              label: inputProcessor
              forward_to: ["loki.write.out.receiver"]
              blocks:
                - stage.structured_metadata:
                    values: {pod: ""}
                - stage.truncate:
                    blocks:
                      - rule: {limit: "250KiB"}
                      - rule: {limit: "2KiB", source_type: label}
                      - rule: {limit: "512B", source_type: structured_metadata}
                - stage.match:
                    selector: '{app="x"}'
                    blocks:
                      - stage.structured_metadata:
                          values: {error: ""}
        "#,
    ));
    assert_eq!(
        bounds.violations(),
        vec![r#"structured metadata with no limit: ["error"]"#.to_string()],
    );
}

#[test]
fn a_budget_over_the_total_is_caught() {
    let bounds = Bounds::of(&input_processor(
        r#"
        blocks:
          - loki.process:
              label: inputProcessor
              forward_to: ["loki.write.out.receiver"]
              blocks:
                - stage.structured_metadata:
                    values: {msg: "", error: ""}
                - stage.truncate:
                    blocks:
                      - rule: {limit: "250KiB"}
                      - rule: {limit: "2KiB", source_type: label}
                      - rule: {limit: "32KiB", source_type: structured_metadata}
        "#,
    ));
    assert_eq!(bounds.violations().len(), 1, "{:?}", bounds.violations());
    assert!(bounds.violations()[0].contains("65544 bytes"));
}

#[test]
fn the_reserve_counts_against_the_total() {
    // 49,155 bytes on its own fits; with the push-client reserve it is three
    // bytes over.
    let bounds = Bounds::of(&input_processor(
        r#"
        blocks:
          - loki.process:
              label: inputProcessor
              forward_to: ["loki.write.out.receiver"]
              blocks:
                - stage.structured_metadata:
                    values: {a: "", b: "", c: ""}
                - stage.truncate:
                    blocks:
                      - rule: {limit: "250KiB"}
                      - rule: {limit: "2KiB", source_type: label}
                      - rule: {limit: "16KiB", source_type: structured_metadata}
        "#,
    ));
    assert_eq!(bounds.violations().len(), 1, "{:?}", bounds.violations());
    assert!(bounds.violations()[0].contains("49155 bytes"));
}
