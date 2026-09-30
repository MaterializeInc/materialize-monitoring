// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Cloud Capacity tab: whether the cloud can supply the nodes asked for.
//!
//! A node that cannot be had looks the same from inside the cluster whatever
//! the reason: pods wait and the autoscaler reports a failed launch. This tab
//! is the provider's side, from the gateway's pull, and each cloud offers a
//! different part of the answer:
//!
//! | | EKS | GKE | AKS |
//! |---|---|---|---|
//! | Host under a node failing | EC2 status checks | — | VM availability |
//! | Node group short of its size | managed node groups | — | the autoscaler's own gauges |
//! | Quota | vCPUs in use, account-wide | CPU and local SSD per family, with refusals | — |
//!
//! No provider publishes capacity refusals; those are Karpenter's and the
//! autoscalers' to report, on the Karpenter dashboard and the Pending Pods tab.
//!
//! Rows render on `$cloudProviderList`, as on `infra-cloud`, which is discovered
//! from the pull's `up` and cannot say *what* a provider is pulled for: its
//! `instance` is a resource name. A provider pulled only for its database and
//! buckets therefore renders its row here with every panel empty. Each empty
//! state says so rather than implying nothing is pulled at all, and names the
//! value that fills it.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use crate::grafana::infra_cloud::{AWS, AZURE, GCP, PROVIDERS, known_providers, ratio_ladder};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![ec2(q), gce(q), aks(q), fallback()]
}

/// What an EC2 panel shows when the pull lists no EKS cluster.
///
/// The row rendering means CloudWatch is pulled, so the likely cause is a pull
/// that covers the database and buckets only.
fn no_eks() -> NoValue {
    NoValue::Custom(
        "CloudWatch is pulled, but not for this cluster's nodes. Add the cluster under \
         pipeline.metrics.provider.cloudwatch.eks.clusters."
            .to_string(),
    )
}

/// What the node group panel shows when no managed node group reports.
///
/// Unlike the other EC2 panels this can be empty with the cluster listed: a
/// cluster whose nodes all come from Karpenter has no managed node group.
fn no_node_groups() -> NoValue {
    NoValue::Custom(
        "No managed node groups reported. Karpenter's nodes have none; if the cluster has \
         some, add it under pipeline.metrics.provider.cloudwatch.eks.clusters."
            .to_string(),
    )
}

/// What a Compute Engine panel shows when the pull lists no region.
fn no_gce() -> NoValue {
    NoValue::Custom(
        "Cloud Monitoring is pulled, but not for Compute Engine quota. Add the region under \
         pipeline.metrics.provider.gcp.compute.regions."
            .to_string(),
    )
}

/// What an AKS panel shows when the pull lists no cluster.
fn no_aks() -> NoValue {
    NoValue::Custom(
        "Azure Monitor is pulled, but not for this cluster. Add it under \
         pipeline.metrics.provider.azure.aks.clusters."
            .to_string(),
    )
}

// --- EC2 ---------------------------------------------------------------------

fn ec2(q: &Queries) -> Row {
    Row::new("EC2").only_when_variable(PROVIDERS, AWS).grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("ec2-status-checks", ec2_status_checks(q))
            .panel("ec2-node-groups", ec2_node_groups(q))
            .panel("ec2-vcpus", ec2_vcpus(q)),
    )
}

fn ec2_status_checks(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Nodes Failing a Status Check")
        .query(q.legended(
            "infra.autoscaling.cloud.ec2.status_checks",
            &["System (host)", "Instance", "Attached EBS"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::errors(1.0, 2.0).build())
        .no_value(no_eks())
        .build(0)
}

fn ec2_node_groups(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Managed Node Groups")
        .query(q.legended(
            "infra.autoscaling.cloud.ec2.node_groups",
            &[
                "{{tag_eks_nodegroup_name}} desired",
                "{{tag_eks_nodegroup_name}} in service",
                "{{tag_eks_nodegroup_name}} maximum",
            ],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_node_groups())
        .build(0)
}

fn ec2_vcpus(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("On-Demand vCPUs in the Account")
        .query(
            q.get("infra.autoscaling.cloud.ec2.vcpus")
                .legend("vCPUs in use"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_eks())
        .build(0)
}

// --- Compute Engine ----------------------------------------------------------

fn gce(q: &Queries) -> Row {
    Row::new("Compute Engine Quota")
        .only_when_variable(PROVIDERS, GCP)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("gce-cpu-quota", gce_cpu_quota(q))
                .panel("gce-ssd-quota", gce_ssd_quota(q))
                .panel("gce-refusals", gce_refusals(q)),
        )
}

const FAMILY: &str = "{{vm_family}} {{location}}";

fn gce_cpu_quota(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Quota Used by Family")
        .query(
            q.get("infra.autoscaling.cloud.gce.cpu_quota")
                .legend(FAMILY),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.8, 0.95))
        .no_value(no_gce())
        .build(0)
}

fn gce_ssd_quota(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Local SSD Quota Used by Family")
        .query(
            q.get("infra.autoscaling.cloud.gce.ssd_quota")
                .legend(FAMILY),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.8, 0.95))
        .no_value(no_gce())
        .build(0)
}

fn gce_refusals(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Quota Refusals per Hour")
        .query(q.get("infra.autoscaling.cloud.gce.refusals").legend(FAMILY))
        .unit("short")
        .min(0.0)
        // Written only when a request is refused, so empty is the healthy
        // reading, not a missing pull.
        .no_value(NoValue::Custom(
            "No refusals. Compute Engine records one only when a quota turns a request away."
                .to_string(),
        ))
        .build(0)
}

// --- AKS ---------------------------------------------------------------------

fn aks(q: &Queries) -> Row {
    Row::new("AKS").only_when_variable(PROVIDERS, AZURE).grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("aks-unschedulable", aks_unschedulable(q))
            .panel("aks-state", aks_state(q))
            .panel("aks-vms-down", aks_vms_down(q)),
    )
}

fn aks_unschedulable(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pods the Autoscaler Cannot Place")
        .query(
            q.get("infra.autoscaling.cloud.aks.unschedulable")
                .legend("{{resourceName}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_aks())
        .build(0)
}

fn aks_state(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Cluster Autoscaler State")
        .query(q.legended(
            "infra.autoscaling.cloud.aks.state",
            &[
                "{{resourceName}} safe to scale",
                "{{resourceName}} scale-down cooling down",
                "{{resourceName}} unneeded nodes",
            ],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(no_aks())
        .build(0)
}

fn aks_vms_down(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Node VMs Down")
        .query(
            q.get("infra.autoscaling.cloud.aks.vms_down")
                .legend("{{resourceName}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .thresholds(threshold::errors(1.0, 2.0).build())
        .no_value(no_aks())
        .build(0)
}

// --- Fallback ----------------------------------------------------------------

/// Why this tab has no provider rows on it.
pub(crate) const NO_PROVIDER: &str = "**No cloud provider metrics are being collected.**\n\n\
     This is the default, not a fault. Pulling from CloudWatch, Cloud Monitoring or Azure Monitor \
     is opt-in, because it needs a cloud identity with read access and every pull is billed.\n\n\
     - **EKS:** list the cluster under `pipeline.metrics.provider.cloudwatch.eks.clusters` for \
       EC2 status checks, managed node group sizes and On-Demand vCPU usage. The gateway's role \
       needs `cloudwatch:GetMetricData`, `cloudwatch:ListMetrics`, `tag:GetResources` and \
       `autoscaling:DescribeAutoScalingGroups`.\n\
     - **GKE:** list the region under `pipeline.metrics.provider.gcp.compute.regions` for CPU \
       and local SSD quota per machine family. `roles/monitoring.viewer` covers it.\n\
     - **AKS:** list the cluster under `pipeline.metrics.provider.azure.aks.clusters` for the \
       managed cluster autoscaler's own gauges, which nothing else can read, and node VM \
       availability. The identity needs Monitoring Reader on the cluster and its node \
       resource group.\n\n\
     Without them, the other tabs still answer whether pods are waiting and what the \
     autoscaler did; this one says whether the cloud was the reason.";

fn fallback() -> Row {
    Row::new("No Provider Metrics")
        .only_unless_variable(PROVIDERS, known_providers())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "capacity-none-detected",
                Panel::text("No Provider Metrics", NO_PROVIDER)
                    .description("Why this tab has no provider rows on it.")
                    .build(0),
            ),
        )
}
