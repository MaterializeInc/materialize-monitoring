// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! E2E assertions against a running `materialize-monitoring` stack.
//!
//! One binary across all tiers: tier 1 (chart on kind), tier 2 (chart on kind
//! over the generic-cloud substrate), and tier 3 (a real cloud, run by hand
//! outside CI). It takes a kubeconfig, a context, a namespace and a release
//! name, and nothing else about the target is assumed — which assertions apply
//! is read from the release's own Helm values.
//!
//! What it deliberately does not do is install anything. Lifecycle belongs to
//! `make`, Terraform, or a human; this asserts against whatever is already
//! running, which is what lets the same binary cover a kind job and a live
//! cluster.

use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use libtest_mimic::{Failed, Trial};
use tokio::runtime::Runtime;

mod checks;
mod cli;
mod cluster;
mod ctx;
mod diagnostics;
mod features;
mod promtext;
mod retry;

use cli::Args;
use cluster::Cluster;
use ctx::Ctx;
use features::Features;

fn main() -> ExitCode {
    let args = Args::parse();

    // Named explicitly because it cannot be inferred: both `aws-lc-rs` and
    // `ring` reach the dependency graph through other crates, so rustls declines
    // to choose and panics on the first handshake. Ignoring the error is
    // deliberate — it only means a provider is already installed.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let runtime = match Runtime::new() {
        Ok(runtime) => Arc::new(runtime),
        Err(err) => {
            eprintln!("error: could not start the async runtime: {err}");
            return ExitCode::FAILURE;
        }
    };

    let ctx = match runtime.block_on(connect(&args)) {
        Ok(ctx) => Arc::new(ctx),
        // A setup failure is not a test failure, and reporting it as one would
        // point whoever reads the log at the stack instead of at their
        // invocation. Bail before the harness starts.
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::FAILURE;
        }
    };

    let trials = build_trials(&runtime, &ctx);

    // Serial by default. These assertions are layered — `/ready` failing makes
    // every later Loki failure noise — so the first error in run order is the
    // one worth reading, and interleaved output from parallel trials buries it.
    // Total runtime is dominated by whichever check has to wait for ingestion
    // anyway, so there is little to win by overlapping them.
    let mut harness = args.harness.clone();
    harness.test_threads.get_or_insert(1);

    let conclusion = libtest_mimic::run(&harness, trials);

    if conclusion.has_failed()
        && let Some(dir) = &args.diagnostics_dir
    {
        diagnostics::dump(&args.diagnostics_script, dir, args.context.as_deref());
    }

    conclusion.exit_code()
}

/// Connect to the cluster and read what the release turned on.
async fn connect(args: &Args) -> Result<Ctx> {
    let cluster = Cluster::connect(
        args.kubeconfig.as_ref(),
        args.context.as_deref(),
        &args.namespace,
    )
    .await?;
    cluster.preflight().await?;

    let features = Features::load(&args.namespace, &args.release, args.context.as_deref())?;

    eprintln!(
        "target: context {}, namespace {}, release {}",
        cluster.context(),
        args.namespace,
        args.release,
    );

    Ok(Ctx {
        cluster,
        features,
        deadline: args.deadline(),
        interval: args.retry_interval(),
        recent_window: args.recent_window(),
    })
}

/// Assemble the assertions that apply to this release.
///
/// A component the values disable yields an ignored trial rather than no trial:
/// a suite whose test list silently shrinks looks identical to one that passed,
/// and "0 tests ran, 0 failed" exits zero.
fn build_trials(runtime: &Arc<Runtime>, ctx: &Arc<Ctx>) -> Vec<Trial> {
    let mut trials = Vec::new();

    // Loki reads through the gateway's relabelling, so the round trip needs both
    // Alloy roles as well as Loki itself. Missing any one of them makes the
    // assertion untestable rather than failing.
    let logging = ctx.features.enabled("loki")
        && ctx.features.enabled("alloy-agent")
        && ctx.features.enabled("alloy-gateway");

    trials.push(trial(
        runtime,
        ctx,
        "loki::ready",
        logging,
        checks::loki::ready,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "loki::streams_created",
        logging,
        checks::loki::streams_created,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "loki::gateway_labels",
        logging,
        checks::loki::gateway_labels,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "loki::recent_query",
        logging,
        checks::loki::recent_query,
    ));

    trials
}

/// Wrap an async assertion as a trial.
///
/// `enabled` false marks it ignored, which libtest-mimic reports separately from
/// a pass — so a run against a stack with Loki turned off says so, instead of
/// quietly reporting a smaller green suite.
fn trial<F>(runtime: &Arc<Runtime>, ctx: &Arc<Ctx>, name: &str, enabled: bool, check: F) -> Trial
where
    F: AsyncFnOnce(&Ctx) -> Result<()> + Send + 'static,
{
    let runtime = Arc::clone(runtime);
    let ctx = Arc::clone(ctx);

    Trial::test(name, move || {
        runtime
            .block_on(check(&ctx))
            // `{:#}` rather than `{}`: anyhow's alternate form prints the whole
            // context chain, and the outer layer here is usually the least
            // specific part of it.
            .map_err(|err| Failed::from(format!("{err:#}")))
    })
    .with_ignored_flag(!enabled)
}
