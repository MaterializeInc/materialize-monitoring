// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `terraform-render`: plan each Terraform example, render the chart against the
//! Helm values the module composed, and assert that each value landed.
//!
//! This is the tier-0 test from the Terraform module design doc, and it is the
//! cheapest place to catch the whole class of bug the module can actually cause:
//! a value written to a path the chart does not read, or a backend key left at a
//! default that only happens to be right on one cloud. Rendering is what proves a
//! value landed — `terraform validate` cannot, because every wrong path is still
//! valid HCL, and `helm template` alone cannot, because it never sees what the
//! module composed.
//!
//! No cluster required. The examples plan against a kubeconfig that does not
//! exist: every resource is a create, nothing refreshes, so the providers are
//! never asked to connect. That is deliberate — a check that needs a cluster does
//! not run on a pull request.
//!
//! The harness is the libtest-mimic one `mz-monitoring-e2e` runs on, with the
//! same conventions: one named trial per assertion per example, filterable by
//! substring, run serially so the first failure in run order is the one to read,
//! and an assertion the example gives nothing to check reported as **ignored**
//! rather than quietly passing.
//!
//! Where a module input stands behind an assertion, the gate is that input as
//! the example *declared* it, read from the plan's configuration — never what the
//! module composed or the chart rendered. Gating on the output is circular: a
//! module that stopped composing a value would switch its own check off and pass.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Result;
use libtest_mimic::{Completion, Failed, Trial};
use serde_json::Value;

mod checks;
mod ctx;
mod manifests;
mod setup;

use ctx::{Ctx, truthy};
use setup::{Example, Outcome};

/// Arguments for the `terraform-render` command.
#[derive(clap::Args)]
pub struct TerraformRenderArgs {
    /// Directory holding one Terraform example per subdirectory.
    #[arg(
        long,
        default_value = "terraform/modules/materialize-monitoring/examples",
        value_name = "DIR"
    )]
    examples_dir: PathBuf,

    /// Plan only this example. Repeatable; unset plans every example.
    ///
    /// Planning is nearly all of the runtime, so this is the way to iterate on
    /// one. A trial filter alone still plans everything.
    #[arg(long = "example", value_name = "NAME")]
    examples: Vec<String>,

    /// The chart the module installs.
    #[arg(
        long,
        default_value = "charts/materialize-monitoring",
        value_name = "DIR"
    )]
    chart: PathBuf,

    /// Terraform binary.
    #[arg(
        long,
        env = "TERRAFORM",
        default_value = "terraform",
        value_name = "PATH"
    )]
    terraform: String,

    /// Helm binary.
    #[arg(long, env = "HELM", default_value = "helm", value_name = "PATH")]
    helm: String,

    #[command(flatten)]
    harness: libtest_mimic::Arguments,
}

/// Entry point for `mz-monitoring-check terraform-render`.
pub fn terraform_render(args: TerraformRenderArgs) -> ExitCode {
    let examples = match setup::plan_and_render(&args) {
        Ok(examples) => examples,
        // A tool that cannot start is not a failed assertion, and reporting it as
        // one would point the reader at the module instead of their PATH.
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::FAILURE;
        }
    };

    // Serial by default, as in the E2E suite: output in run order, so `plan` and
    // `render` come before the assertions that depend on them.
    let mut harness = args.harness.clone();
    harness.test_threads.get_or_insert(1);

    let trials = examples
        .into_iter()
        .flat_map(|example| build_trials(example, &harness))
        .collect();

    libtest_mimic::run(&harness, trials).exit_code()
}

/// One assertion, run once per example.
struct Check {
    name: &'static str,
    /// Whether the example gives this assertion anything to check. False marks
    /// the trial ignored rather than leaving it out: a list that silently
    /// shrinks reads the same as one that passed.
    applies: fn(&Ctx) -> bool,
    run: fn(&Ctx) -> Result<()>,
}

/// Every assertion, in run order.
const CHECKS: &[Check] = &[
    Check {
        name: "storage_class::reaches_every_claim",
        applies: |ctx| ctx.declares("storage_class"),
        run: checks::storage_class::reaches_every_claim,
    },
    // An invariant of the chart rather than of an input, so it holds for every
    // example whether or not it sets any scheduling.
    Check {
        name: "scheduling::min_domains_need_do_not_schedule",
        applies: |_| true,
        run: checks::scheduling::min_domains_need_do_not_schedule,
    },
    Check {
        name: "scheduling::node_selector",
        applies: |ctx| ctx.declares("node_selector"),
        run: checks::scheduling::node_selector,
    },
    Check {
        name: "scheduling::tolerations",
        applies: |ctx| ctx.declares("tolerations"),
        run: checks::scheduling::tolerations,
    },
    Check {
        name: "scheduling::agent_keeps_blanket_toleration",
        applies: |ctx| ctx.declares("tolerations"),
        run: checks::scheduling::agent_keeps_blanket_toleration,
    },
    // Gated on the render, because the invariant is between two rendered things:
    // a pod running as a ServiceAccount that names an Azure identity has to carry
    // the label the webhook selects on.
    Check {
        name: "workload_identity::pods_labelled",
        applies: |ctx| !checks::workload_identity::bound_service_accounts(ctx).is_empty(),
        run: checks::workload_identity::pods_labelled,
    },
    // `install_node_exporter` defaults on, so every example has a circuit
    // breaker to check.
    Check {
        name: "node_exporter::circuit_breaker",
        applies: |_| true,
        run: checks::node_exporter::circuit_breaker,
    },
    Check {
        name: "cluster_name::reaches_every_stamp",
        applies: |ctx| ctx.declares("cluster_name"),
        run: checks::cluster_name::reaches_every_stamp,
    },
    Check {
        name: "kube_state_metrics::pod_labels_extend_allowlist",
        applies: |ctx| ctx.declares("kube_state_metrics_pod_labels"),
        run: checks::kube_state_metrics::pod_labels_extend_allowlist,
    },
    Check {
        name: "object_storage::static_credentials_land",
        applies: |ctx| ctx.declares("object_storage_access_key_id"),
        run: checks::object_storage::static_credentials_land,
    },
    Check {
        name: "object_storage::secret_key_stays_in_secrets",
        applies: |ctx| ctx.declares("object_storage_secret_access_key"),
        run: checks::object_storage::secret_key_stays_in_secrets,
    },
    Check {
        name: "object_storage::http_endpoint_bare_and_insecure",
        applies: |ctx| {
            ctx.declared_at("object_storage", "endpoint")
                .and_then(Value::as_str)
                .is_some_and(|endpoint| endpoint.starts_with("http://"))
        },
        run: checks::object_storage::http_endpoint_bare_and_insecure,
    },
    // `aws` is the only dialect with an endpoint of the caller's choosing. Also
    // gated on the policy rendering at all: with network policies off nothing
    // restricts the egress, and that is a different question.
    Check {
        name: "object_storage::loki_egress_covers_endpoint",
        applies: |ctx| {
            checks::object_storage::on_s3(ctx)
                && checks::object_storage::loki_egress_policy(ctx).is_some()
        },
        run: checks::object_storage::loki_egress_covers_endpoint,
    },
    Check {
        name: "object_storage::loki_s3_endpoint",
        applies: checks::object_storage::on_s3,
        run: checks::object_storage::loki_s3_endpoint,
    },
    Check {
        name: "certificates::issuer",
        applies: |ctx| ctx.declares("certificates_enabled"),
        run: checks::certificates::issuer,
    },
    Check {
        name: "certificates::issuer_group",
        applies: |ctx| ctx.declares("certificates_enabled"),
        run: checks::certificates::issuer_group,
    },
    Check {
        name: "certificates::loki_san_ladder",
        applies: |ctx| ctx.declares("certificates_enabled"),
        run: checks::certificates::loki_san_ladder,
    },
    Check {
        name: "certificates::external",
        applies: |ctx| {
            ctx.declares("certificates_enabled") && ctx.declares("grafana_external_dns_names")
        },
        run: checks::certificates::external,
    },
    Check {
        name: "internal_tls::loki_serves_tls",
        applies: checks::internal_tls::enabled,
        run: checks::internal_tls::loki_serves_tls,
    },
    Check {
        name: "internal_tls::thanos_receive_serves_tls",
        applies: checks::internal_tls::enabled,
        run: checks::internal_tls::thanos_receive_serves_tls,
    },
    Check {
        name: "internal_tls::replication_factor_survives",
        applies: checks::internal_tls::enabled,
        run: checks::internal_tls::replication_factor_survives,
    },
    Check {
        name: "internal_tls::client_auth_matches_stage",
        applies: checks::internal_tls::enabled,
        run: checks::internal_tls::client_auth_matches_stage,
    },
    // `encrypt` presents no client certificates anywhere, so there is no CA
    // policy to have composed yet.
    Check {
        name: "internal_tls::loki_verifies_client_certs",
        applies: |ctx| matches!(checks::internal_tls::stage(ctx), "present" | "authenticate"),
        run: checks::internal_tls::loki_verifies_client_certs,
    },
    Check {
        name: "internal_tls::alertmanager_serves_tls",
        applies: |ctx| {
            checks::internal_tls::enabled(ctx) && checks::alerting::config_secret(ctx).is_some()
        },
        run: checks::internal_tls::alertmanager_serves_tls,
    },
    Check {
        name: "internal_tls::rulers_remote_write_over_tls",
        applies: checks::internal_tls::enabled,
        run: checks::internal_tls::rulers_remote_write_over_tls,
    },
    Check {
        name: "destinations::remote_write",
        applies: |ctx| ctx.declares("prometheus_remote_write"),
        run: checks::destinations::remote_write,
    },
    Check {
        name: "destinations::google_cloud",
        applies: |ctx| ctx.declares("google_cloud_metrics"),
        run: checks::destinations::google_cloud,
    },
    Check {
        name: "destinations::datadog",
        applies: |ctx| ctx.declares("datadog_metrics"),
        run: checks::destinations::datadog,
    },
    Check {
        name: "destinations::otlp",
        applies: |ctx| ctx.declares("otlp_metrics"),
        run: checks::destinations::otlp,
    },
    // Read from the plan, since the Secret is a Terraform resource and never
    // appears in `helm template` output.
    Check {
        name: "gateway_credentials::read_by_pipeline",
        applies: |ctx| !checks::gateway_credentials::planned(ctx).is_empty(),
        run: checks::gateway_credentials::read_by_pipeline,
    },
    Check {
        name: "gateway_credentials::absent_from_release",
        applies: |ctx| !checks::gateway_credentials::planned(ctx).is_empty(),
        run: checks::gateway_credentials::absent_from_release,
    },
    // `grafana_database_enabled` is inferred from the host when null, so either
    // one declares a database.
    Check {
        name: "grafana::database",
        applies: |ctx| {
            ctx.declares("grafana_database_host") || ctx.declares("grafana_database_enabled")
        },
        run: checks::grafana::database,
    },
    Check {
        name: "alerting::receivers",
        applies: |ctx| ctx.declared_at("alerting", "receivers").is_some_and(truthy),
        run: checks::alerting::receivers,
    },
    Check {
        name: "alerting::preset",
        applies: |ctx| ctx.declared_at("alerting", "preset").is_some_and(truthy),
        run: checks::alerting::preset,
    },
    Check {
        name: "alerting::extra_routes",
        applies: |ctx| {
            ctx.declared_at("alerting", "routes.extra")
                .is_some_and(truthy)
        },
        run: checks::alerting::extra_routes,
    },
    // The module creates the Secret only when it holds the receivers'
    // credentials; an example that leaves it to another owner has none to check.
    Check {
        name: "alerting::receiver_secret",
        applies: |ctx| checks::alerting::planned_receiver_secret(ctx).is_some(),
        run: checks::alerting::receiver_secret,
    },
    Check {
        name: "alert_rules::disabled",
        applies: |ctx| ctx.declared_at("alert_rules", "enabled") == Some(&Value::Bool(false)),
        run: checks::alerting::rules_disabled,
    },
    Check {
        name: "alert_rules::selection",
        applies: |ctx| {
            checks::alerting::rules_on(ctx)
                && ["selected", "disabled"]
                    .iter()
                    .any(|key| ctx.declared_at("alert_rules", key).is_some_and(truthy))
        },
        run: checks::alerting::rule_selection,
    },
    Check {
        name: "alert_rules::overrides",
        applies: |ctx| {
            checks::alerting::rules_on(ctx)
                && ctx
                    .declared_at("alert_rules", "overrides")
                    .is_some_and(truthy)
        },
        run: checks::alerting::rule_overrides,
    },
    Check {
        name: "alert_rules::scopes",
        applies: |ctx| {
            checks::alerting::rules_on(ctx)
                && [
                    "excluded_namespaces",
                    "environment_namespaces",
                    "infra_workloads",
                ]
                .iter()
                .any(|key| ctx.declared_at("alert_rules", key).is_some_and(truthy))
        },
        run: checks::alerting::rule_scopes,
    },
];

/// The trials for one example: its plan, its render, and every assertion.
fn build_trials(example: Example, harness: &libtest_mimic::Arguments) -> Vec<Trial> {
    let name = example.name;
    let (plan, render, ctx) = match example.outcome {
        Outcome::Rendered(ctx) => (Ok(()), Ok(()), Some(ctx)),
        Outcome::PlanFailed(err) => (
            Err(err),
            Err(format!("not attempted: {name}::plan failed")),
            None,
        ),
        Outcome::RenderFailed(err) => (Ok(()), Err(err), None),
    };
    let failed_stage = match (&plan, &render) {
        (Err(_), _) => "plan",
        _ => "render",
    };

    let mut trials = vec![
        Trial::test(format!("{name}::plan"), move || plan.map_err(Failed::from)),
        Trial::test(format!("{name}::render"), move || {
            render.map_err(Failed::from)
        }),
    ];

    // With nothing rendered the assertions cannot run. They are ignored, with
    // the reason, when the failed stage's own trial is in this run to carry the
    // failure — and fail otherwise, so a filter that selected only assertions
    // cannot report success on an example that never rendered.
    let stage_trial = if failed_stage == "plan" {
        &trials[0]
    } else {
        &trials[1]
    };
    let stage_reported = !harness.is_filtered_out(stage_trial);

    for check in CHECKS {
        let trial_name = format!("{name}::{}", check.name);
        let trial = match &ctx {
            Some(ctx) => {
                let ctx = Arc::clone(ctx);
                let applies = (check.applies)(&ctx);
                let run = check.run;
                Trial::test(trial_name, move || {
                    // `{:#}` prints the whole context chain, whose outer layer is
                    // usually the least specific part of it.
                    run(&ctx).map_err(|err| Failed::from(format!("{err:#}")))
                })
                .with_ignored_flag(!applies)
            }
            None => {
                let reason = format!("{name}::{failed_stage} failed");
                Trial::ignorable_test(trial_name, move || {
                    if stage_reported {
                        Ok(Completion::ignored_with(reason))
                    } else {
                        Err(Failed::from(format!("not checked: {reason}")))
                    }
                })
            }
        };
        trials.push(trial);
    }

    trials
}
