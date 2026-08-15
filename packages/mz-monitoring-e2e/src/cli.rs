// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Command line.
//!
//! The libtest-mimic arguments are flattened in, so `--list`, `--exact`,
//! `--ignored` and a filter substring all work the way they do under `cargo
//! test` — this is a test runner and should not invent its own vocabulary for
//! that.

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "mz-monitoring-e2e",
    about = "Assert a running materialize-monitoring stack behaves",
    long_about = "Assert a running materialize-monitoring stack behaves.

Reads the release's coalesced Helm values to decide which assertions apply,
then talks to the stack through the API server's Service proxy. Which
assertions exist depends on what the release enabled; run with --list to see
the set for a given cluster."
)]
pub struct Args {
    /// Path to the kubeconfig. Defaults to the usual resolution order.
    #[arg(long, value_name = "PATH", env = "KUBECONFIG")]
    pub kubeconfig: Option<PathBuf>,

    /// Kubeconfig context to target.
    ///
    /// Named explicitly rather than inherited, so a stale current-context does
    /// not silently point this at another cluster. Unset means the configured
    /// current context, which is what the CI job wants — there is only one
    /// cluster there.
    #[arg(long, value_name = "NAME", env = "KUBE_CONTEXT")]
    pub context: Option<String>,

    /// Namespace the release is installed in.
    #[arg(long, short = 'n', default_value = "monitoring", env = "NAMESPACE")]
    pub namespace: String,

    /// Helm release name.
    #[arg(long, default_value = "mzmon", env = "RELEASE")]
    pub release: String,

    /// Seconds any single assertion may retry before it fails.
    #[arg(long, default_value_t = 180, value_name = "SECONDS")]
    pub deadline: u64,

    /// Seconds between retries.
    #[arg(long, default_value_t = 5, value_name = "SECONDS")]
    pub retry_interval: u64,

    /// How recent a log line must be to prove the write path is live.
    ///
    /// Wide enough to absorb the gateway's batch interval, narrow enough that
    /// chunks left by a previous run cannot satisfy it.
    #[arg(long, default_value_t = 120, value_name = "SECONDS")]
    pub recent_window: u64,

    /// Directory to collect cluster diagnostics into when anything fails.
    ///
    /// Unset collects nothing. In CI this should always be set: the cluster is
    /// deleted with the runner, so whatever is not captured here is gone.
    #[arg(long, value_name = "DIR")]
    pub diagnostics_dir: Option<PathBuf>,

    /// The collector to run for --diagnostics-dir.
    #[arg(
        long,
        default_value = "test/e2e/dump-diagnostics.sh",
        value_name = "PATH"
    )]
    pub diagnostics_script: PathBuf,

    #[command(flatten)]
    pub harness: libtest_mimic::Arguments,
}

impl Args {
    pub fn deadline(&self) -> Duration {
        Duration::from_secs(self.deadline)
    }

    pub fn retry_interval(&self) -> Duration {
        Duration::from_secs(self.retry_interval)
    }

    pub fn recent_window(&self) -> Duration {
        Duration::from_secs(self.recent_window)
    }
}
