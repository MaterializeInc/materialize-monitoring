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
mod forward;
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
        args.transport,
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
        allow_unhealthy: args.allow_unhealthy.clone(),
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

    // Only the bundled mode is assertable: it is the one where the admin
    // credentials are a Secret in this namespace and the server is a Service we
    // can proxy to. `external` and `operator` are skips, not failures.
    let grafana = ctx.features.enabled("grafana") && ctx.features.grafana_mode() == "bundled";
    // Provisioning is the operator's job, so without it there is nothing to
    // assert about how dashboards and datasources got there.
    let provisioned = grafana && ctx.features.enabled("grafana-operator");
    let datasources = provisioned && ctx.features.datasources_enabled();

    trials.push(trial(
        runtime,
        ctx,
        "grafana::health",
        grafana,
        checks::grafana::health,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "grafana::dashboards_provisioned",
        provisioned,
        checks::grafana::dashboards_provisioned,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "grafana::datasources_provisioned",
        datasources,
        checks::grafana::datasources_provisioned,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "grafana::loki_datasource_query",
        datasources && ctx.features.enabled("loki"),
        checks::grafana::loki_datasource_query,
    ));
    // Enabled by default and needing no flag: the support bundle is the single
    // richest artifact the stack exposes, and each role has its own.
    trials.push(trial(
        runtime,
        ctx,
        "alloy::gateway_support_bundle",
        ctx.features.enabled("alloy-gateway"),
        checks::alloy::gateway_support_bundle,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "alloy::agent_support_bundle",
        ctx.features.enabled("alloy-agent"),
        checks::alloy::agent_support_bundle,
    ));

    let thanos = ctx.features.enabled("thanos");
    trials.push(trial(
        runtime,
        ctx,
        "thanos::ready",
        thanos,
        checks::thanos::ready,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "thanos::stores",
        thanos,
        checks::thanos::stores,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "thanos::targets_up",
        thanos,
        checks::thanos::targets_up,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "thanos::samples_scraped",
        thanos,
        checks::thanos::samples_scraped,
    ));

    trials.push(trial(
        runtime,
        ctx,
        "grafana::thanos_datasource_query",
        datasources && thanos,
        checks::grafana::thanos_datasource_query,
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
