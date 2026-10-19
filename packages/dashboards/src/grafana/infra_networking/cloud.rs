// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Cloud Networking tab: what the provider put in front of the cluster, and
//! the way out of it.
//!
//! # Two halves, from two sides
//!
//! Kubernetes knows that a load balancer was *asked for* and what address came
//! back, which is the first row and works everywhere. What the load balancer is
//! *doing* — whether it has a healthy backend, what it carries, who resets the
//! connections — is not in the cluster at all. It is in CloudWatch, Cloud
//! Monitoring and Azure Monitor, and the gateway pulls it for the load
//! balancers and NAT gateways named in `pipeline.metrics.provider.*`.
//!
//! The NAT gateway is the other half. Every pod without a public address leaves
//! the network through one, which is where a Materialize source's upstream
//! usually is, and its failure — running out of source ports — presents inside
//! the cluster as a source that stalls and reconnects with no cause of its own.
//! Nothing on the other tabs can confirm it.
//!
//! # Rows render on the pull, not the provider
//!
//! Each networking pull stamps its series with `provider_pull`, and
//! [`variable::cloud_network_pulls`] reads that back off `up`, so a row renders
//! where its pull runs and nowhere else. `infra-cloud` renders on the provider,
//! which is enough there because every provider row is about the same two
//! resources; here an install that pulls its database from CloudWatch has not
//! thereby named a load balancer. Each half has its own fallback, since each
//! pull is configured on its own.
//!
//! # One provider's rows at a time
//!
//! Each provider publishes different facts, so there is no cross-provider row
//! as `infra-cloud` has. CloudWatch publishes target health, Cloud Monitoring
//! only traffic, and Azure Monitor a probe of the data path beside the
//! backends' health probes. Each provider's rows draw what that provider has.
//!
//! [`variable::cloud_network_pulls`]: mzmon_lib::grafana::variable::cloud_network_pulls

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;
use mzmon_lib::grafana::variable::extra;

use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// The variable every provider row is rendered on.
const PULLS: &str = extra::CLOUD_NETWORK_PULL_LIST;

/// The CloudWatch Network Load Balancer and target group pulls.
pub(crate) const AWS_LB: &str = "cloudwatch_nlb";
/// The Cloud Monitoring forwarding rule pull.
pub(crate) const GCP_LB: &str = "gcp_load_balancing";
/// The Azure Monitor load balancer pull.
pub(crate) const AZURE_LB: &str = "azure_load_balancer";
/// The CloudWatch NAT gateway pull.
pub(crate) const AWS_NAT: &str = "cloudwatch_nat_gateway";
/// The Cloud Monitoring Cloud NAT pull.
pub(crate) const GCP_NAT: &str = "gcp_cloud_nat";
/// The Azure Monitor NAT gateway pull.
pub(crate) const AZURE_NAT: &str = "azure_nat_gateway";

/// Every load balancer pull, as one alternation for the fallback to negate.
fn load_balancer_pulls() -> String {
    [AWS_LB, GCP_LB, AZURE_LB].join("|")
}

/// Every NAT gateway pull, as one alternation for the fallback to negate.
fn nat_pulls() -> String {
    [AWS_NAT, GCP_NAT, AZURE_NAT].join("|")
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        load_balancers(q),
        aws_load_balancers(q),
        gcp_load_balancers(q),
        azure_load_balancers(q),
        no_load_balancer_pull(),
        aws_nat(q),
        gcp_nat(q),
        azure_nat(q),
        no_nat_pull(),
    ]
}

/// What a provider panel shows when its pull sent nothing.
///
/// Reachable only on a row that rendered, so a pull exists: it is either still
/// waiting for its first data or being refused.
fn not_pulled() -> NoValue {
    NoValue::Custom(
        "No data from the provider. A new pull takes a few minutes to report; \
         Infrastructure Cloud Provider -> Collection shows whether it is being refused."
            .to_string(),
    )
}

// --- Kubernetes ----------------------------------------------------------

/// The half that every cluster has.
///
/// Standard height, not tall: a cluster has a handful of LoadBalancer Services
/// at most, and a load balancer Terraform made has none, so a tall table opened
/// the tab on a screen of empty space with the provider rows below the fold.
fn load_balancers(q: &Queries) -> Row {
    Row::new("Load Balancers").grid(AutoGrid::new(1).panel("cloud-load-balancers", lb_table(q)))
}

fn lb_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Load Balancer Services")
        // `table_format`, or each Service arrives as its own frame and the panel
        // renders a frame picker rather than a table.
        .query(q.get("infra.net.cloud.load_balancers").table_format())
        // The AWS wrappers make the Network Load Balancer in Terraform and attach
        // it with TargetGroupBindings, so no Service there has type LoadBalancer.
        .no_value(NoValue::Custom(
            "No LoadBalancer Service. A load balancer made outside Kubernetes, such as the \
             AWS wrappers' Network Load Balancer, has no row here; its provider rows are below."
                .to_string(),
        ))
        // `hostname` beside `ip`: a load balancer is reached by address on GCP
        // and Azure and by DNS name on AWS, and Kubernetes reports whichever
        // applies. Ordering by `ip` alone dropped the AWS case entirely, which
        // would have read as a load balancer with no address.
        .transformations(vec![transform::organize(
            &["Time", "Value", "__name__"],
            &["namespace", "service", "ip", "hostname"],
        )])
        .build(0)
}

// --- Load balancers, per provider ------------------------------------------

const NLB: &str = "{{load_balancer}}";
const TARGET_GROUP: &str = "{{target_group}}";

fn aws_load_balancers(q: &Queries) -> Row {
    Row::new("Network Load Balancer: Targets and Connections")
        .only_when_variable(PULLS, AWS_LB)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("cloud-nlb-healthy", nlb_healthy(q))
                .panel("cloud-nlb-unhealthy", nlb_unhealthy(q))
                .panel("cloud-nlb-resets", nlb_resets(q))
                .panel("cloud-nlb-open", nlb_open(q))
                .panel("cloud-nlb-new", nlb_new(q))
                .panel("cloud-nlb-throughput", nlb_throughput(q)),
        )
}

fn nlb_healthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Healthy Targets")
        .query(
            q.get("infra.net.cloud.nlb.healthy_targets")
                .legend(TARGET_GROUP),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::health(1.0, 1.0).build())
        .no_value(NoValue::Custom(
            "No target groups are pulled. CloudWatch publishes health only per target group, \
             so list them beside their load balancer in values."
                .to_string(),
        ))
        .build(0)
}

fn nlb_unhealthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Unhealthy Targets")
        .query(
            q.get("infra.net.cloud.nlb.unhealthy_targets")
                .legend(TARGET_GROUP),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::errors(1.0, 10.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn nlb_resets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connection Resets (per 5 min)")
        .query(q.legended(
            "infra.net.cloud.nlb.resets",
            &[
                "client {{load_balancer}}",
                "target {{load_balancer}}",
                "load balancer {{load_balancer}}",
            ],
        ))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn nlb_open(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections")
        .query(q.get("infra.net.cloud.nlb.open_connections").legend(NLB))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn nlb_new(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("New Connections")
        .query(q.get("infra.net.cloud.nlb.new_connections").legend(NLB))
        .unit("cps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn nlb_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput")
        .query(q.get("infra.net.cloud.nlb.throughput").legend(NLB))
        .unit("Bps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcp_load_balancers(q: &Queries) -> Row {
    Row::new("Cloud Load Balancing: Traffic")
        .only_when_variable(PULLS, GCP_LB)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("cloud-gcp-lb-packets", gcp_lb_packets(q))
                .panel("cloud-gcp-lb-throughput", gcp_lb_throughput(q)),
        )
}

/// What a forwarding rule panel shows with nothing to draw.
///
/// Cloud Monitoring writes no point for a minute without traffic, so an idle
/// rule and a refused pull look alike here; the text names both.
fn no_forwarding_rule_traffic() -> NoValue {
    NoValue::Custom(
        "No traffic reported. Cloud Monitoring writes nothing for a minute a rule carried none; \
         Infrastructure Cloud Provider -> Collection shows whether the pull is being refused."
            .to_string(),
    )
}

fn gcp_lb_packets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Packets")
        .query(q.legended(
            "infra.net.cloud.gcp_lb.packets",
            &[
                "in {{forwarding_rule_name}}",
                "out {{forwarding_rule_name}}",
            ],
        ))
        .unit("pps")
        .min(0.0)
        .no_value(no_forwarding_rule_traffic())
        .build(0)
}

fn gcp_lb_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput")
        .query(q.legended(
            "infra.net.cloud.gcp_lb.throughput",
            &[
                "in {{forwarding_rule_name}}",
                "out {{forwarding_rule_name}}",
            ],
        ))
        .unit("Bps")
        .min(0.0)
        .no_value(no_forwarding_rule_traffic())
        .build(0)
}

/// A frontend address and its load balancer's resource group, which is the AKS
/// cluster's node resource group: two clusters' internal frontends can share
/// an address.
const FRONTEND: &str = "{{frontend}} ({{resourceGroup}})";

fn azure_load_balancers(q: &Queries) -> Row {
    Row::new("Azure Load Balancer: Health and Traffic")
        .only_when_variable(PULLS, AZURE_LB)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("cloud-azure-lb-healthy", azure_lb_healthy(q))
                .panel("cloud-azure-lb-availability", azure_lb_availability(q))
                .panel("cloud-azure-lb-new", azure_lb_new(q))
                .panel("cloud-azure-lb-throughput", azure_lb_throughput(q)),
        )
}

fn azure_lb_healthy(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Healthy Backends")
        .query(
            q.get("infra.net.cloud.azure_lb.healthy_backends")
                .legend(FRONTEND),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::health(1.0, 1.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn azure_lb_availability(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Data Path Availability")
        .query(
            q.get("infra.net.cloud.azure_lb.availability")
                .legend(FRONTEND),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.99, 1.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn azure_lb_new(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("New Connections")
        .query(
            q.get("infra.net.cloud.azure_lb.new_connections")
                .legend(FRONTEND),
        )
        .unit("cps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn azure_lb_throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput")
        .query(q.legended(
            "infra.net.cloud.azure_lb.throughput",
            &[
                "in {{frontend}} ({{resourceGroup}})",
                "out {{frontend}} ({{resourceGroup}})",
            ],
        ))
        .unit("Bps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

/// Why there are no provider load balancer rows.
const NO_LOAD_BALANCER: &str = "**No load balancer metrics are being collected.**\n\n\
     The table above is what Kubernetes knows: that a load balancer exists, and its address. \
     What it is doing — whether it has a healthy backend, what it carries, who resets its \
     connections — is published by the cloud provider, and the gateway pulls it only for load \
     balancers named in values. That is opt-in, because it needs a cloud identity with read \
     access and every pull is billed by the provider.\n\n\
     - **AWS**: list each Network Load Balancer's ARN, with its target groups' ARNs, under \
       `pipeline.metrics.provider.cloudwatch.nlb.loadBalancers`.\n\
     - **Google Cloud**: list the cluster's VPC network under \
       `pipeline.metrics.provider.gcp.loadBalancing.networks`, or each forwarding rule under \
       `.forwardingRules`.\n\
     - **Azure**: list the AKS cluster under `pipeline.metrics.provider.azure.loadBalancer.clusters`.\n\n\
     A pull that is configured but failing still renders its rows, empty, so this note never \
     hides one. The Cloud Provider Metrics page of the documentation has the details.\n\n\
     Without it, _Kubernetes -> Endpoints by Namespace_ is the nearest in-cluster substitute: a \
     Service with no ready endpoints has nothing for its load balancer to send to.";

fn no_load_balancer_pull() -> Row {
    Row::new("No Load Balancer Metrics")
        .only_unless_variable(PULLS, load_balancer_pulls())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "cloud-no-lb-pull",
                Panel::text("No Load Balancer Metrics", NO_LOAD_BALANCER)
                    .description("Why this tab has no provider load balancer rows on it.")
                    .build(0),
            ),
        )
}

// --- NAT gateways, per provider --------------------------------------------

const NAT_ID: &str = "{{dimension_NatGatewayId}}";

fn aws_nat(q: &Queries) -> Row {
    Row::new("NAT Gateway: Ports and Connections")
        .only_when_variable(PULLS, AWS_NAT)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("cloud-nat-exhaustion", aws_nat_exhaustion(q))
                .panel("cloud-nat-established", aws_nat_established(q))
                .panel("cloud-nat-connections", aws_nat_connections(q))
                .panel("cloud-nat-idle", aws_nat_idle(q)),
        )
}

fn aws_nat_exhaustion(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Port Exhaustion (per 5 min)")
        .query(q.legended(
            "infra.net.cloud.aws_nat.exhaustion",
            &[
                "port allocation errors {{dimension_NatGatewayId}}",
                "dropped packets {{dimension_NatGatewayId}}",
            ],
        ))
        .unit("short")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 100.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn aws_nat_established(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections Established")
        .query(q.get("infra.net.cloud.aws_nat.established").legend(NAT_ID))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.5, 0.9).build())
        .no_value(not_pulled())
        .build(0)
}

fn aws_nat_connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections")
        .query(q.get("infra.net.cloud.aws_nat.connections").legend(NAT_ID))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn aws_nat_idle(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Idle Timeouts (per 5 min)")
        .query(
            q.get("infra.net.cloud.aws_nat.idle_timeouts")
                .legend(NAT_ID),
        )
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

const CLOUD_NAT: &str = "{{gateway_name}}";

fn gcp_nat(q: &Queries) -> Row {
    Row::new("Cloud NAT: Ports and Connections")
        .only_when_variable(PULLS, GCP_NAT)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("cloud-cloud-nat-failed", gcp_nat_failed(q))
                .panel("cloud-cloud-nat-dropped", gcp_nat_dropped(q))
                .panel("cloud-cloud-nat-ports", gcp_nat_ports(q))
                .panel("cloud-cloud-nat-connections", gcp_nat_connections(q)),
        )
}

fn gcp_nat_failed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Allocation Failures")
        .query(
            q.get("infra.net.cloud.cloud_nat.allocation_failed")
                .legend(CLOUD_NAT),
        )
        .unit("bool_yes_no")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::errors(1.0, 1.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn gcp_nat_dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Dropped Packets")
        .query(
            q.get("infra.net.cloud.cloud_nat.dropped")
                .legend("{{reason}} {{gateway_name}}"),
        )
        .unit("pps")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcp_nat_ports(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Port Usage")
        .query(
            q.get("infra.net.cloud.cloud_nat.port_usage")
                .legend(CLOUD_NAT),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcp_nat_connections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Open Connections")
        .query(
            q.get("infra.net.cloud.cloud_nat.open_connections")
                .legend(CLOUD_NAT),
        )
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

/// NAT gateway names are unique only within a resource group, and the pull
/// collects every gateway of a listed name.
const AZURE_NAT_NAME: &str = "{{resourceName}} ({{resourceGroup}})";

fn azure_nat(q: &Queries) -> Row {
    Row::new("Azure NAT Gateway: Ports and Connections")
        .only_when_variable(PULLS, AZURE_NAT)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("cloud-azure-nat-snat", azure_nat_snat(q))
                .panel("cloud-azure-nat-dropped", azure_nat_dropped(q))
                .panel("cloud-azure-nat-availability", azure_nat_availability(q)),
        )
}

fn azure_nat_snat(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("SNAT Connections (per 5 min)")
        .query(q.legended(
            "infra.net.cloud.azure_nat.snat_connections",
            &[
                "attempted {{resourceName}} ({{resourceGroup}})",
                "failed {{resourceName}} ({{resourceGroup}})",
            ],
        ))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn azure_nat_dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Dropped Packets (per 5 min)")
        .query(
            q.get("infra.net.cloud.azure_nat.dropped")
                .legend(AZURE_NAT_NAME),
        )
        .unit("short")
        .min(0.0)
        .thresholds(threshold::errors(1.0, 100.0).build())
        .no_value(not_pulled())
        .build(0)
}

fn azure_nat_availability(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Data Path Availability")
        .query(
            q.get("infra.net.cloud.azure_nat.availability")
                .legend(AZURE_NAT_NAME),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.99, 1.0).build())
        .no_value(not_pulled())
        .build(0)
}

/// Why there are no NAT gateway rows.
const NO_NAT: &str = "**No NAT gateway metrics are being collected.**\n\n\
     Pods without a public address leave the network through a NAT gateway, and when it runs out \
     of source ports for one destination, a Materialize source stalls and reconnects with no \
     cause anywhere in the cluster. Only the provider can say so, and the gateway pulls it only \
     for NAT gateways named in values:\n\n\
     - **AWS**: `pipeline.metrics.provider.cloudwatch.natGateway.ids`.\n\
     - **Google Cloud**: `pipeline.metrics.provider.gcp.cloudNat.gateways`.\n\
     - **Azure**: `pipeline.metrics.provider.azure.natGateway.names`.\n\n\
     A cluster whose nodes have public addresses of their own has no NAT gateway, and nothing \
     here applies to it.\n\n\
     Without it, _Node Networking -> Conntrack Table_ is the nearest in-cluster signal: a node \
     whose connection table is full fails in much the same way and for a similar reason.";

fn no_nat_pull() -> Row {
    Row::new("No NAT Gateway Metrics")
        .only_unless_variable(PULLS, nat_pulls())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "cloud-no-nat-pull",
                Panel::text("No NAT Gateway Metrics", NO_NAT)
                    .description("Why this tab has no NAT gateway rows on it.")
                    .build(0),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PULL_TOKENS: [&str; 6] = [AWS_LB, GCP_LB, AZURE_LB, AWS_NAT, GCP_NAT, AZURE_NAT];

    #[test]
    fn every_pull_token_is_one_the_pipeline_stamps() {
        // The pipeline writes these as `provider_pull`, the variable reads them
        // back, and the rows match them. A token here that no pull stamps is a
        // row that never renders.
        let pipeline = include_str!("../../../../alloy-pipelines/gateway-provider.yaml");
        for token in PULL_TOKENS {
            assert!(
                pipeline.contains(&format!("provider_pull: {token}\n")),
                "no pull stamps {token}"
            );
        }
    }

    #[test]
    fn no_pull_token_matches_another() {
        // Row conditions are unanchored regexes, so a token inside another would
        // render one provider's rows on another's pull.
        for a in PULL_TOKENS {
            for b in PULL_TOKENS {
                if a != b {
                    assert!(!b.contains(a), "{a} is inside {b}");
                }
            }
        }
    }

    #[test]
    fn no_pull_token_matches_an_unset_variable() {
        // An unset variable reaches a row condition as one of these strings.
        for token in PULL_TOKENS {
            for unset in ["", "undefined", "[object Object]", "$__all"] {
                assert!(!unset.contains(token), "{unset} matches {token}");
            }
        }
    }

    #[test]
    fn each_fallback_explains_and_names_its_values() {
        for key in [
            "cloudwatch.nlb.loadBalancers",
            "gcp.loadBalancing.networks",
            "azure.loadBalancer.clusters",
        ] {
            assert!(NO_LOAD_BALANCER.contains(key), "{key}");
        }
        for key in [
            "cloudwatch.natGateway.ids",
            "gcp.cloudNat.gateways",
            "azure.natGateway.names",
        ] {
            assert!(NO_NAT.contains(key), "{key}");
        }
        // And the nearest thing that works without them.
        assert!(NO_LOAD_BALANCER.contains("Endpoints by Namespace"));
        assert!(NO_NAT.contains("Conntrack Table"));
    }
}
