// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The CNI tab: the dataplane underneath, whichever one this cluster runs.
//!
//! # The tab that builds itself
//!
//! Every other tab in this repository draws the same panels everywhere. This one
//! cannot: the metrics that describe a dataplane are per-vendor and share
//! nothing — `awscni_ip_max` and `cilium_bpf_map_pressure` are not two spellings
//! of one idea, they are different facts about differently-shaped software.
//!
//! The alternative to adapting would be a dashboard per cloud, which this repo
//! tried once for GCP and retired: two artifacts that differed only in which
//! panels were blank. So the rows here are conditioned on
//! [`variable::network_components`], and a cluster is shown its own vendor's row
//! and none of the others.
//!
//! # Why the fallback row is not optional
//!
//! A tab whose every condition failed is indistinguishable from a broken one.
//! [`missing`] carries the complement of every pattern its siblings match, so
//! exactly one thing is always on screen — and because the most common reason to
//! land there is not a fault, it says which reason applies rather than
//! announcing an absence.
//!
//! **GKE Dataplane V2 is that case and is worth stating in the product.** It
//! runs Cilium, labels its daemon `k8s-app: cilium`, and names the container
//! `cilium-agent` — and disables the agent's Prometheus endpoint, routing the
//! metrics to Cloud Monitoring through a sidecar instead. Measured against GKE
//! 1.34 / cilium v1.18.7-gke1.35: `:9962` serves nothing, and Hubble's port
//! serves its own health counters and no flow metrics. An operator who reads
//! "no dataplane metrics" there should not go looking for a scrape to fix.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;
use mzmon_lib::grafana::variable::extra;

use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::CNI.shade;

/// The variable every row on this tab is rendered on.
const DATAPLANE: &str = extra::NETWORK_COMPONENT_LIST;

/// The AWS VPC CNI, as the scrape config labels it.
pub(super) const AWS: &str = "aws-vpc-cni";
/// Cilium, in any of its three deployments.
pub(super) const CILIUM: &str = "cilium";

/// Every vendor this tab can draw, as one alternation.
///
/// [`missing`] renders on the *negation* of this, so a vendor added above
/// without being added here would leave the fallback row on screen beside it —
/// which is why the two are derived from one place.
fn known_vendors() -> String {
    format!("{AWS}|{CILIUM}")
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        aws_addresses(q),
        aws_api(q),
        cilium_dataplane(q),
        cilium_pressure(q),
        hubble(q),
        missing(),
    ]
}

// --- AWS VPC CNI ---------------------------------------------------------

/// Address exhaustion, which is the AWS-specific failure worth knowing about.
fn aws_addresses(q: &Queries) -> Row {
    Row::new("VPC CNI: Addresses and Interfaces")
        .only_when_variable(DATAPLANE, AWS)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("cni-aws-ip-utilization", aws_ip_utilization(q))
                .panel("cni-aws-exhausted", aws_exhausted(q))
                .panel("cni-aws-addresses", aws_address_counts(q))
                .panel("cni-aws-enis", aws_enis(q)),
        )
}

/// How the CNI is getting on with EC2, which is what makes pods slow to start.
fn aws_api(q: &Queries) -> Row {
    Row::new("VPC CNI: AWS API")
        .only_when_variable(DATAPLANE, AWS)
        .collapsed()
        .grid(AutoGrid::new(1).panel("cni-aws-api-latency", aws_api_latency(q)))
}

fn aws_ip_utilization(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Address Utilization")
        .query(q.get("infra.net.cni.aws.ip_utilization").legend("{{node}}"))
        .unit("percentunit")
        // A bounded fraction whose ceiling is the failure: pinned so a healthy
        // node stays visibly flat instead of autoscaling to its own noise.
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .build(0)
}

fn aws_exhausted(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Nodes Out of Addresses")
        .query(q.get("infra.net.cni.aws.exhaustion").legend("nodes"))
        .unit("short")
        .min(0.0)
        .shade(SHADE)
        .no_value(NoValue::Custom(
            "Every node still has addresses to give out.".to_string(),
        ))
        .build(0)
}

fn aws_address_counts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Addresses per Node")
        .query(q.legended(
            "infra.net.cni.aws.addresses",
            &["assigned {{node}}", "pool {{node}}", "max {{node}}"],
        ))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn aws_enis(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Network Interfaces per Node")
        .query(q.legended(
            "infra.net.cni.aws.enis",
            &["attached {{node}}", "max {{node}}"],
        ))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn aws_api_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("EC2 API Latency (mean)")
        .query(q.get("infra.net.cni.aws.api_latency").legend("{{api}}"))
        .unit("ms")
        .min(0.0)
        // A mean is a rate over a rate, so a window in which the CNI made no
        // EC2 calls divides zero by zero and Prometheus drops the series
        // outright rather than reporting it as 0. Observed on a live EKS
        // cluster whose nodes had been stable for an hour, which is the
        // ordinary case: the CNI calls EC2 when pods are placed, not
        // continuously. An empty panel here therefore means nothing happened.
        .no_value(NoValue::Custom(
            "No EC2 API calls in this window — the CNI calls EC2 when pods \
             are placed, not continuously."
                .to_string(),
        ))
        .build(0)
}

// --- Cilium --------------------------------------------------------------

/// What the datapath did: endpoints programmed, packets dropped, policy applied.
///
/// Regeneration time and unreachable nodes are here on the strength of the
/// internal cloud networking dashboards, which both lead with the former: it
/// degrades well before anything starts dropping, so a row built only from drop
/// counters reports the problem late.
fn cilium_dataplane(q: &Queries) -> Row {
    Row::new("Cilium: Datapath")
        .only_when_variable(DATAPLANE, CILIUM)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("cni-cilium-endpoints", cilium_endpoints(q))
                .panel("cni-cilium-regeneration", cilium_regeneration(q))
                .panel("cni-cilium-unreachable", cilium_unreachable(q))
                .panel("cni-cilium-drops", cilium_drops(q))
                .panel("cni-cilium-verdicts", cilium_verdicts(q)),
        )
}

/// The capacity limit that fails closed, on its own row because it is a
/// different kind of problem from the ones above.
fn cilium_pressure(q: &Queries) -> Row {
    Row::new("Cilium: BPF Maps")
        .only_when_variable(DATAPLANE, CILIUM)
        .collapsed()
        .grid(AutoGrid::new(1).panel("cni-cilium-bpf-pressure", cilium_bpf(q)))
}

fn hubble(q: &Queries) -> Row {
    Row::new("Hubble: Flows")
        .only_when_variable(DATAPLANE, CILIUM)
        .collapsed()
        .grid(AutoGrid::new(1).panel("cni-hubble-flows", hubble_flows(q)))
}

fn cilium_endpoints(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Endpoints by State")
        .query(
            q.get("infra.net.cni.cilium.endpoints")
                .legend("{{endpoint_state}}"),
        )
        .unit("short")
        .min(0.0)
        .build(0)
}

fn cilium_regeneration(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Endpoint Regeneration (p99)")
        .query(
            q.get("infra.net.cni.cilium.endpoint_regeneration")
                .legend("{{scope}}"),
        )
        .unit("s")
        .min(0.0)
        .build(0)
}

fn cilium_unreachable(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Unreachable Nodes and Endpoints")
        .query(q.legended(
            "infra.net.cni.cilium.unreachable",
            &["nodes", "health endpoints"],
        ))
        .unit("short")
        .min(0.0)
        .decimals(0.0)
        .no_value(NoValue::Custom("Cilium can reach every node.".to_string()))
        .build(0)
}

fn cilium_drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Datapath Drops by Reason")
        .query(q.get("infra.net.cni.cilium.drops").legend("{{reason}}"))
        .unit("pps")
        .min(0.0)
        .no_value(NoValue::Custom("No packets dropped.".to_string()))
        .build(0)
}

fn cilium_verdicts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Policy Verdicts")
        .query(
            q.get("infra.net.cni.cilium.policy_verdicts")
                .legend("{{action}}"),
        )
        .unit("pps")
        .min(0.0)
        .build(0)
}

fn cilium_bpf(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("BPF Map Pressure")
        .query(
            q.get("infra.net.cni.cilium.bpf_map_pressure")
                .legend("{{node}} {{map_name}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .build(0)
}

fn hubble_flows(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Flows by Verdict")
        .query(
            q.get("infra.net.cni.cilium.hubble_flows")
                .legend("{{verdict}}"),
        )
        .unit("cps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Hubble is not exporting flow metrics.".to_string(),
        ))
        .build(0)
}

// --- Nothing detected ----------------------------------------------------

/// What the note says when no vendor's row applies.
///
/// Its job is the *reason*, not the absence — the reader needs to know whether
/// they are looking at a scrape to fix or at a platform that cannot be scraped,
/// and those look identical from an empty tab.
const NO_DATAPLANE: &str = "**No dataplane metrics are being collected on this cluster.**\n\n\
     This is not necessarily a fault. Every cluster has a CNI; not every CNI can be scraped.\n\n\
     - **GKE Dataplane V2** runs Cilium but disables the agent's Prometheus endpoint, sending \
       those metrics to Cloud Monitoring instead. Nothing here can change that, and Hubble on \
       GKE serves only its own health counters rather than flow data.\n\
     - **A CNI this stack has no monitor for.** Monitors ship for the AWS VPC CNI and for \
       Cilium; Calico, Azure NPM and anything bring-your-own are not covered yet.\n\
     - **Metrics disabled at the CNI.** Upstream Cilium exposes nothing until it is installed \
       with its Prometheus endpoint enabled, and the monitor addresses that port *by name*, so \
       an install without it produces no target rather than a failing one.\n\n\
     Everything not specific to a dataplane still works: _Kubernetes_ for pods and Services, \
     _Node Networking_ for interfaces and the kernel, and _Security_ for the NetworkPolicy \
     objects themselves — though what a policy actually *dropped* can only come from the CNI, \
     so that half is empty too.";

fn missing() -> Row {
    Row::new("No Dataplane Metrics")
        .only_unless_variable(DATAPLANE, known_vendors())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "cni-none-detected",
                Panel::text("No Dataplane Metrics", NO_DATAPLANE)
                    .description("Why this tab has no vendor rows on it.")
                    .build(0),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fallback_covers_every_vendor_row() {
        // The fallback renders on the negation of this pattern, so a vendor with
        // a row but no entry here would show its panels *and* a note saying
        // nothing was detected.
        let pattern = known_vendors();
        for vendor in [AWS, CILIUM] {
            assert!(
                pattern.contains(vendor),
                "{vendor} has rows but is not in the fallback's alternation"
            );
        }
    }

    #[test]
    fn the_note_explains_rather_than_announces() {
        // An empty tab already says "nothing here". What the reader cannot work
        // out alone is whether this is a scrape to fix or a platform that cannot
        // be scraped, so the note has to name the cases.
        assert!(NO_DATAPLANE.contains("Dataplane V2"), "{NO_DATAPLANE}");
        assert!(NO_DATAPLANE.contains("not necessarily a fault"));
        // And where to go instead, rather than leaving the reader stuck.
        assert!(NO_DATAPLANE.contains("_Kubernetes_"));
        assert!(NO_DATAPLANE.contains("_Node Networking_"));
    }
}
