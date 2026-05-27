// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Test-only helpers shared across the alloy render/pipeline test modules.
//!
//! The centerpiece is [`assert_canonical`], which uses the real `alloy fmt`
//! binary as an oracle: it asserts that our rendered output is already in the
//! canonical form `alloy fmt` would produce. If `alloy` isn't installed (e.g.
//! CI without it), the check is skipped rather than failed.

use std::io::Write;
use std::process::Command;

use crate::alloy::error::Result;

/// Returns true if the `alloy` binary is available on PATH.
fn alloy_available() -> bool {
    Command::new("alloy").arg("--version").output().is_ok()
}

/// Assert that `rendered` is already canonically formatted according to
/// `alloy fmt` — i.e. running `alloy fmt` over it is a no-op.
///
/// Skips (does not fail) when the `alloy` binary is not installed.
pub(crate) fn assert_canonical(rendered: &str) {
    if !alloy_available() {
        eprintln!("skipping alloy fmt oracle: `alloy` not found on PATH");
        return;
    }

    // `alloy fmt` operates on a *file*, and files end with a trailing newline.
    // `Block::render` produces a *fragment* (no trailing newline); the `Pipeline`
    // layer is what appends the final newline. Normalize our input to file form
    // so the comparison isn't tripped up by that one-byte convention.
    let as_file = if rendered.ends_with('\n') {
        rendered.to_string()
    } else {
        format!("{rendered}\n")
    };

    // alloy fmt operates on a file path, so stage the normalized output.
    let mut tmp = tempfile::Builder::new()
        .suffix(".alloy")
        .tempfile()
        .expect("create temp file");
    tmp.write_all(as_file.as_bytes()).expect("write temp file");
    tmp.flush().expect("flush temp file");

    let output = Command::new("alloy")
        .arg("fmt")
        .arg(tmp.path())
        .output()
        .expect("run alloy fmt");

    assert!(
        output.status.success(),
        "alloy fmt rejected rendered output:\n{as_file}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&output.stderr),
    );

    let formatted = String::from_utf8(output.stdout).expect("alloy fmt output is utf-8");
    assert_eq!(
        as_file, formatted,
        "renderer output is not alloy-fmt canonical (compared as file content)",
    );
}

/// Assert that a render result matches `expected` byte-for-byte AND that
/// `expected` is itself canonical `alloy fmt` output.
///
/// The exact-bytes check is fast and runs everywhere; the canonical check
/// runs wherever `alloy` is installed and catches formatting drift the
/// hand-written string can't anticipate.
pub(crate) fn assert_renders(rendered: Result<String>, expected: &str) {
    let rendered = rendered.expect("render should succeed");
    assert_eq!(rendered, expected, "exact-bytes mismatch");
    assert_canonical(&rendered);
}
