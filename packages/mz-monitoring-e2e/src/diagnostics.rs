// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! State collection on failure.
//!
//! In CI the cluster dies with the runner, so anything not captured at the
//! moment of failure is unrecoverable. A red E2E with no artifacts costs more
//! than the test saves.
//!
//! Collection delegates to `test/e2e/dump-diagnostics.sh` rather than
//! reimplementing it: the same script runs when a `helm install` fails before
//! this suite is ever reached, and one artifact layout is worth more than a
//! marginally tidier binary.

use std::path::Path;
use std::process::Command;

/// Run the diagnostics script into `out_dir`.
///
/// Never propagates a failure. This runs *because* something already failed, and
/// a broken collector must not replace the real error with its own.
pub fn dump(script: &Path, out_dir: &Path, context: Option<&str>) {
    if !script.exists() {
        eprintln!(
            "warning: diagnostics script {} not found; no artifacts collected",
            script.display()
        );
        return;
    }

    eprintln!("collecting diagnostics into {}", out_dir.display());

    let mut cmd = Command::new(script);
    cmd.arg(out_dir);
    if let Some(context) = context {
        cmd.env("KUBE_CONTEXT", context);
    }

    match cmd.status() {
        // Silent on success: the script announces the directory itself, and
        // saying it twice reads like it ran twice.
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("warning: diagnostics script exited {status}");
        }
        Err(err) => {
            eprintln!("warning: could not run {}: {err}", script.display());
        }
    }
}
