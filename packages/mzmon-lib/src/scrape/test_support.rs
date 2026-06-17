// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Test-only helpers for the scrape transpiler.
//!
//! Goldens are compared *structurally* (parsed to `serde_json::Value`), not as
//! exact bytes: classic Prometheus YAML has no canonical key order or quoting,
//! so an exact-bytes oracle (cf. `alloy`'s `alloy fmt`) would be brittle. Where
//! `promtool` is installed, [`assert_promtool_ok`] additionally checks the
//! rendered document is something Prometheus actually accepts.

use std::io::Write;
use std::process::Command;

use serde_json::Value;

use crate::scrape::classic::config::ScrapeJob;
use crate::scrape::error::Result;

/// The real Monitor fixtures under `packages/prometheus-scrapers/`, embedded so
/// tests run hermetically. Keep in sync with that directory.
pub(crate) const FIXTURES: &[(&str, &str)] = &[
    (
        "podmonitor-environmentd",
        include_str!("../../../prometheus-scrapers/podmonitor-environmentd.yaml"),
    ),
    (
        "podmonitor-sql",
        include_str!("../../../prometheus-scrapers/podmonitor-sql.yaml"),
    ),
    (
        "podmonitor-clusterd",
        include_str!("../../../prometheus-scrapers/podmonitor-clusterd.yaml"),
    ),
    (
        "podmonitor-materialize-operator",
        include_str!("../../../prometheus-scrapers/podmonitor-materialize-operator.yaml"),
    ),
    (
        "scrapeconfig-cadvisor",
        include_str!("../../../prometheus-scrapers/scrapeconfig-cadvisor.yaml"),
    ),
];

/// Look up an embedded fixture by stem (panics if absent — test bug).
pub(crate) fn fixture(name: &str) -> &'static str {
    FIXTURES
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

/// Assert the produced jobs match `expected_yaml` structurally (order- and
/// quoting-independent map comparison via `serde_json::Value`).
pub(crate) fn assert_jobs(produced: Result<Vec<ScrapeJob>>, expected_yaml: &str) {
    let produced = produced.expect("transpile should succeed");
    let produced_val = serde_json::to_value(&produced).expect("serialize produced jobs");
    let expected_val: Value =
        serde_yaml_ng::from_str(expected_yaml).expect("parse expected jobs YAML");
    assert_eq!(
        produced_val,
        expected_val,
        "transpiled jobs mismatch\n--- produced ---\n{}",
        serde_yaml_ng::to_string(&produced).unwrap()
    );
}

/// Assert an arbitrary serializable value matches `expected_yaml` structurally
/// (order- and quoting-independent). Used for the GMP output goldens.
pub(crate) fn assert_serializes_to<T: serde::Serialize>(value: &T, expected_yaml: &str) {
    let produced_val = serde_json::to_value(value).expect("serialize value");
    let expected_val: Value = serde_yaml_ng::from_str(expected_yaml).expect("parse expected YAML");
    assert_eq!(
        produced_val,
        expected_val,
        "serialized value mismatch\n--- produced ---\n{}",
        serde_yaml_ng::to_string(value).unwrap()
    );
}

/// True if the `promtool` binary is available on PATH.
fn promtool_available() -> bool {
    Command::new("promtool").arg("--version").output().is_ok()
}

/// Assert that `doc_yaml` is accepted by `promtool check config`. Skips (does
/// not fail) when `promtool` is not installed.
#[allow(dead_code)]
pub(crate) fn assert_promtool_ok(doc_yaml: &str) {
    if !promtool_available() {
        eprintln!("skipping promtool oracle: `promtool` not found on PATH");
        return;
    }
    let mut tmp = tempfile::Builder::new()
        .suffix(".yaml")
        .tempfile()
        .expect("create temp file");
    tmp.write_all(doc_yaml.as_bytes()).expect("write temp file");
    tmp.flush().expect("flush temp file");

    let output = Command::new("promtool")
        .arg("check")
        .arg("config")
        .arg(tmp.path())
        .output()
        .expect("run promtool check config");

    assert!(
        output.status.success(),
        "promtool rejected rendered config:\n{doc_yaml}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}
