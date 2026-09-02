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
mod tls;

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

    // Loaded once, and only when there is something to load: on a plaintext
    // stack there is no Secret to read and nothing that would use it. Printed
    // because the identity a run borrows decides what the servers log, and
    // tracing a refused handshake back to the material should not need a guess.
    //
    // A discovery failure is a warning rather than a fatal error, and the
    // difference matters: the usual cause is certificates that have not become
    // Ready, and `tls::certificates_ready` is the assertion that says so. Exiting
    // here would replace that diagnosis with a setup error from a step the reader
    // did not ask for. The checks that need the material fail on their own, with
    // a message naming it.
    //
    // An explicit --client-cert-secret is fatal, because that is the caller
    // naming a Secret rather than the suite guessing, and silently proceeding
    // without it would run a weaker suite than they asked for.
    let tls = match (
        features.certificates_enabled(),
        args.client_cert_secret.as_deref(),
    ) {
        (_, Some(name)) => {
            let loaded = tls::ClientTls::load(&cluster, Some(name)).await?;
            eprintln!("client tls: trusting and presenting {}", loaded.source());
            Some(loaded)
        }
        (true, None) => match tls::ClientTls::load(&cluster, None).await {
            Ok(loaded) => {
                eprintln!("client tls: trusting and presenting {}", loaded.source());
                Some(loaded)
            }
            Err(err) => {
                eprintln!("warning: no client TLS material ({err:#})");
                None
            }
        },
        (false, None) => None,
    };

    Ok(Ctx {
        cluster,
        features,
        deadline: args.deadline(),
        interval: args.retry_interval(),
        recent_window: args.recent_window(),
        allow_unhealthy: args.allow_unhealthy.clone(),
        allow_disruptive: args.allow_disruptive,
        tls,
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
    //
    // Not gated on whether Loki serves TLS: these dial it over the suite's own
    // TLS client when it does, so a stack with certificates on gets the same
    // coverage as one without. It briefly was gated, and the resulting run was
    // green with the entire log read path unasserted.
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

    // Gated on both: the assertion reads kube-state-metrics series *through*
    // Thanos, so either being absent makes it unanswerable rather than failing.
    let kube_state = thanos && ctx.features.enabled("kube-state-metrics");
    trials.push(trial(
        runtime,
        ctx,
        "kube_state::labels_are_honored",
        kube_state,
        checks::kube_state::labels_are_honored,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "kube_state::pods_are_distinguishable",
        kube_state,
        checks::kube_state::pods_are_distinguishable,
    ));

    // The node-detail dashboard's join spans both exporters, so it needs both
    // features present before the assertion means anything.
    let node_metrics = kube_state && ctx.features.enabled("node-exporter");
    trials.push(trial(
        runtime,
        ctx,
        "node::identity_joins_across_exporters",
        node_metrics,
        checks::node::identity_joins_across_exporters,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "node::capacity_is_reported_per_node",
        kube_state,
        checks::node::capacity_is_reported_per_node,
    ));

    trials.push(trial(
        runtime,
        ctx,
        "grafana::thanos_datasource_query",
        datasources && thanos,
        checks::grafana::thanos_datasource_query,
    ));

    // TLS. Gated on the release having asked for certificates, so a stack
    // without them yields ignored trials rather than absent ones.
    //
    // `alloy_components_healthy` runs whenever Alloy does, certificates or not:
    // the failure it catches — a component that fails to build while the pod
    // stays Ready — is not specific to TLS, and it is the only assertion in the
    // suite that would notice a listener silently never binding.
    let certificates = ctx.features.certificates_enabled();
    let alloy = ctx.features.enabled("alloy-gateway") || ctx.features.enabled("alloy-agent");

    trials.push(trial(
        runtime,
        ctx,
        "tls::alloy_components_healthy",
        alloy,
        checks::tls::alloy_components_healthy,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "tls::certificates_ready",
        certificates,
        checks::tls::certificates_ready,
    ));
    // Needs a TLS hop to point at, so it is gated on Loki actually serving TLS
    // rather than on certificates merely being issued.
    // The authentication assertion, gated on the hop actually being at phase 3:
    // at phase 2 an anonymous client is *supposed* to be served, so running it
    // there would fail a correctly-configured stack.
    trials.push(trial(
        runtime,
        ctx,
        "tls::gateway_requires_client_certificate",
        certificates
            && ctx.features.enabled("alloy-gateway")
            && ctx.features.gateway_logs_requires_client_cert(),
        checks::tls::gateway_requires_client_certificate,
    ));
    trials.push(trial(
        runtime,
        ctx,
        "tls::loki_refuses_plaintext",
        certificates && ctx.features.enabled("loki") && ctx.features.loki_server_tls(),
        checks::tls::loki_refuses_plaintext,
    ));
    // Destructive: forces a reissue by deleting a Secret. Opt-in, because this
    // binary is pointed at real clusters too.
    //
    // Gated on the hop actually serving TLS, not merely on certificates being
    // issued. Issuance and use are separate switches, and on a release with the
    // first and not the second this deleted a Secret, waited for cert-manager to
    // reissue, then passed a **plaintext** Loki query — reporting that delivery
    // survived a certificate renewal on a path where no process had opened the
    // certificate at all.
    trials.push(trial(
        runtime,
        ctx,
        "tls::survives_renewal",
        certificates && logging && ctx.features.loki_server_tls() && ctx.allow_disruptive,
        checks::tls::survives_renewal,
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
