// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Load Balancer tab: the cloud load balancer in front of balancerd and the
//! console, the hop before every other tab.
//!
//! Kubernetes knows the address it was given, which is the first row and works
//! wherever the load balancer is a Service. What the load balancer is doing is
//! the provider's account, which the gateway pulls only where values name it,
//! so the provider rows render on `infra-net`'s pull detection and share its
//! constants. Each provider publishes different facts: CloudWatch has target
//! health and resets, Azure Monitor its health probes, Cloud Monitoring only
//! traffic.
//!
//! # Scoped to the environment, three ways
//!
//! The pulled series carry no environment, and `infra-net` draws every load
//! balancer the pull names, Grafana's included. Here each provider is tied back
//! to the environment by what it does carry: the AWS wrappers' target group
//! names, GKE's forwarding rule names derived from the Service's UID, and
//! Azure's frontend IP. The registry header has the details.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;
use mzmon_lib::grafana::variable::extra;

use super::LOG_BASE;
use crate::grafana::infra_networking::cloud::{AWS_LB, AZURE_LB, GCP_LB};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// The variable every provider row is rendered on.
const PULLS: &str = extra::CLOUD_NETWORK_PULL_LIST;

/// Every load balancer pull, as one alternation for the fallback to negate.
pub(crate) fn load_balancer_pulls() -> String {
    [AWS_LB, GCP_LB, AZURE_LB].join("|")
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![addresses(q), aws(q), gcp(q), azure(q), no_pull()]
}

/// What a provider panel shows when its pull sent nothing for this environment.
///
/// Reachable only on a row that rendered, so a pull exists: it is waiting for
/// its first data, being refused, or names a load balancer that is not this
/// environment's.
fn not_pulled() -> NoValue {
    NoValue::Custom(
        "No data for this environment's load balancer. Check that the pull names it, and on \
         Infrastructure Networking -> Cloud Networking that the pull is reporting at all."
            .to_string(),
    )
}

fn addresses(q: &Queries) -> Row {
    Row::new("Load Balancer Services").grid(
        AutoGrid::new(1).panel(
            "lb-addresses",
            Panel::table("Load Balancer Addresses")
                // `table_format`, or each Service arrives as its own frame and
                // the panel renders a frame picker rather than a table.
                .query(q.get("materialize.ingress.lb.addresses").table_format())
                .no_value(NoValue::Custom(
                    "No LoadBalancer Service in this environment. The AWS wrappers make their \
                     Network Load Balancer outside Kubernetes, so it has no row here; its \
                     provider row is below."
                        .to_string(),
                ))
                .transformations(vec![transform::organize(
                    &["Time", "Value", "__name__"],
                    &["service", "ip", "hostname"],
                )])
                .build(0),
        ),
    )
}

// --- AWS -------------------------------------------------------------------

fn aws(q: &Queries) -> Row {
    Row::new("AWS Network Load Balancer")
        .only_when_variable(PULLS, AWS_LB)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("lb-aws-healthy", aws_healthy(q))
                .panel("lb-aws-unhealthy", aws_unhealthy(q))
                .panel("lb-aws-resets", aws_resets(q))
                .panel("lb-aws-open", aws_open(q))
                .panel("lb-aws-new", aws_new(q))
                .panel("lb-aws-throughput", aws_throughput(q)),
        )
}

fn aws_healthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Healthy Targets")
        .query(
            q.get("materialize.ingress.lb.aws.healthy_targets")
                .legend("{{target_group}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::health(1.0, 1.0).build())
        .no_value(NoValue::Custom(
            "No target group of this environment is pulled. CloudWatch publishes health only per \
             target group, so list the pgwire, http and console target groups beside the load \
             balancer in values."
                .to_string(),
        ))
        .build(0)
}

fn aws_unhealthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Unhealthy Targets")
        .query(
            q.get("materialize.ingress.lb.aws.unhealthy_targets")
                .legend("{{target_group}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::errors(1.0, 10.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn aws_resets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Resets (per 5 min)")
        .query(q.legended(
            "materialize.ingress.lb.aws.resets",
            &[
                "client {{load_balancer}}",
                "target {{load_balancer}}",
                "load balancer {{load_balancer}}",
            ],
        ))
        .unit("short")
        .decimals(0.0)
        .log_scale(LOG_BASE)
        .no_value(not_pulled())
        .build(0)
}

fn aws_open(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections")
        .query(
            q.get("materialize.ingress.lb.aws.open_connections")
                .legend("{{load_balancer}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn aws_new(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("New Connections")
        .query(
            q.get("materialize.ingress.lb.aws.new_connections")
                .legend("{{load_balancer}}"),
        )
        .unit("cps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn aws_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput")
        .query(
            q.get("materialize.ingress.lb.aws.throughput")
                .legend("{{load_balancer}}"),
        )
        .unit("Bps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

// --- Google Cloud ----------------------------------------------------------

fn gcp(q: &Queries) -> Row {
    Row::new("Google Cloud Load Balancer")
        .only_when_variable(PULLS, GCP_LB)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("lb-gcp-packets", gcp_packets(q))
                .panel("lb-gcp-throughput", gcp_throughput(q)),
        )
}

fn gcp_packets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Packets by Service")
        .query(q.legended(
            "materialize.ingress.lb.gcp.packets",
            &["{{service}} in", "{{service}} out"],
        ))
        .unit("pps")
        .log_scale(LOG_BASE)
        .no_value(not_pulled())
        .build(0)
}

fn gcp_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput by Service")
        .query(q.legended(
            "materialize.ingress.lb.gcp.throughput",
            &["{{service}} in", "{{service}} out"],
        ))
        .unit("Bps")
        .log_scale(LOG_BASE)
        .no_value(not_pulled())
        .build(0)
}

// --- Azure -----------------------------------------------------------------

fn azure(q: &Queries) -> Row {
    Row::new("Azure Load Balancer")
        .only_when_variable(PULLS, AZURE_LB)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("lb-azure-backends", azure_backends(q))
                .panel("lb-azure-new", azure_new(q))
                .panel("lb-azure-throughput", azure_throughput(q)),
        )
}

fn azure_backends(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes Passing the Health Probe")
        .query(q.legended(
            "materialize.ingress.lb.azure.backends",
            &["{{service}} healthy", "{{service}} total"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn azure_new(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("New Connections by Service")
        .query(
            q.get("materialize.ingress.lb.azure.new_connections")
                .legend("{{service}}"),
        )
        .unit("cps")
        .log_scale(LOG_BASE)
        .no_value(not_pulled())
        .build(0)
}

fn azure_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput by Service")
        .query(q.legended(
            "materialize.ingress.lb.azure.throughput",
            &["{{service}} in", "{{service}} out"],
        ))
        .unit("Bps")
        .log_scale(LOG_BASE)
        .no_value(not_pulled())
        .build(0)
}

// --- Fallback --------------------------------------------------------------

/// Why the tab has no provider rows, and how to get them.
pub(crate) const NO_LOAD_BALANCER: &str = "**No load balancer metrics are being collected.**\n\n\
     The table above is what Kubernetes knows about the load balancer in front of balancerd and \
     the console: that it exists, and its address. What it is doing — whether its targets are \
     healthy, what it carries, who resets its connections — is published by the cloud provider, \
     and the gateway pulls it only for load balancers named in values.\n\n\
     - **AWS**: list the Network Load Balancer's ARN, with its pgwire, http and console target \
       groups' ARNs, under `pipeline.metrics.provider.cloudwatch.nlb.loadBalancers`. From the \
       Terraform wrappers, that is `provider_metrics.network_load_balancers`.\n\
     - **Google Cloud**: list the balancerd and console forwarding rules under \
       `pipeline.metrics.provider.gcp.loadBalancing.forwardingRules`, or the cluster's VPC \
       network under `.networks`. From the Terraform wrappers, that is \
       `provider_metrics.forwarding_rules`.\n\
     - **Azure**: list the AKS cluster under `pipeline.metrics.provider.azure.loadBalancer.clusters`. \
       From the Terraform wrappers, that is `provider_metrics.load_balancer_clusters`.\n\n\
     The Cloud Provider Metrics page of the documentation has the details.";

fn no_pull() -> Row {
    Row::new("No Load Balancer Metrics")
        .only_unless_variable(PULLS, load_balancer_pulls())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "lb-no-pull",
                Panel::text("No Load Balancer Metrics", NO_LOAD_BALANCER)
                    .description("Why this tab has no provider load balancer rows on it.")
                    .build(0),
            ),
        )
}
