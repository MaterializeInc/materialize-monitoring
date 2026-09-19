// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Cloud Networking tab: what the provider put in front of the cluster.
//!
//! # Half built, deliberately
//!
//! Kubernetes knows that a load balancer was *asked for* and what address came
//! back. It does not know anything about what that load balancer is doing —
//! connection counts, rejected requests, how many targets it believes are
//! healthy — because none of that is in the cluster. It is in CloudWatch, Cloud
//! Monitoring and Azure Monitor, and this stack does not read them yet
//! ([DEP-233](https://linear.app/materializeinc/issue/DEP-233)).
//!
//! So the first row is real and the rest are stubs. The split is worth being
//! explicit about rather than shipping a tab that looks broken: the Kubernetes
//! half genuinely answers *was the load balancer created and did it get an
//! address*, which is the question behind most "the endpoint is unreachable"
//! reports, and it is worth having before the provider-side half lands.
//!
//! # Why stubs at all
//!
//! A named empty row is a commitment and a piece of documentation; an absent one
//! is indistinguishable from never having thought about it. Each stub says which
//! provider metrics would fill it, so whoever picks up the collection work
//! inherits the target rather than re-deriving it.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::queries::Queries;
use crate::grafana::transform;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![load_balancers(q), provider_metrics(), egress()]
}

/// The half that works today.
fn load_balancers(q: &Queries) -> Row {
    Row::new("Load Balancers").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("cloud-load-balancers", lb_table(q)),
    )
}

fn lb_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Load Balancer Services")
        // `table_format`, or each Service arrives as its own frame and the panel
        // renders a frame picker rather than a table.
        .query(q.get("infra.net.cloud.load_balancers").table_format())
        .no_value(NoValue::Custom(
            "No Service in this cluster has a cloud load balancer.".to_string(),
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

/// What a provider-side scrape would add.
const PROVIDER_NOTE: &str = "**Load balancer metrics are not collected yet.**\n\n\
     Kubernetes can say that a load balancer exists and what address it was given — that is the \
     table above. Everything about what it is *doing* lives at the cloud provider:\n\n\
     - **AWS** — `AWS/ApplicationELB` and `AWS/NetworkELB` in CloudWatch: request and connection \
       counts, target response time, `HealthyHostCount`, `UnHealthyHostCount`, and the 4xx/5xx \
       families that separate a backend fault from a load balancer one.\n\
     - **GCP** — the `loadbalancing.googleapis.com` metrics in Cloud Monitoring.\n\
     - **Azure** — the Load Balancer and Application Gateway metric namespaces in Azure Monitor.\n\n\
     Collecting these needs a cloud-provider scrape path, which this stack does not have. Tracked \
     as DEP-233 🔒.\n\n\
     Until then: an unreachable endpoint whose Service *does* appear above is usually a target \
     health problem, and _Kubernetes -> Endpoints by Namespace_ is the closest in-cluster \
     substitute — a Service with no endpoints has nothing for the load balancer to send to.";

fn provider_metrics() -> Row {
    Row::new("Load Balancer Traffic and Health").grid(
        AutoGrid::new(1).row_height(RowHeight::Tall).panel(
            "cloud-provider-stub",
            Panel::text("Load Balancer Traffic and Health", PROVIDER_NOTE)
                .description("What a cloud-provider scrape would add here, and why it is absent.")
                .build(0),
        ),
    )
}

/// What a VPC-side scrape would add.
const EGRESS_NOTE: &str = "**VPC and egress metrics are not collected yet.**\n\n\
     The network the cluster sits in is invisible from inside it. What a provider-side scrape \
     would add:\n\n\
     - **NAT gateway throughput, packet drops and port allocation errors.** Port exhaustion on a \
       NAT gateway presents inside the cluster as intermittent connection failures to external \
       sources — a Materialize source that stalls and recovers with no in-cluster cause is the \
       classic symptom, and nothing on this dashboard can currently confirm it.\n\
     - **Cross-zone traffic volume**, which is both a latency and a billing question for a \
       deployment spread across availability zones.\n\
     - **VPC flow logs**, for the connections that never arrived at all. These are logs rather \
       than metrics and would belong with _Logs and Events_ rather than here.\n\n\
     Tracked as DEP-233 🔒.\n\n\
     Until then, _Node Networking -> Conntrack Table_ is the nearest in-cluster signal: a node \
     whose connection table is full fails in much the same way and for a similar reason.";

fn egress() -> Row {
    Row::new("VPC and Egress").collapsed().grid(
        AutoGrid::new(1).row_height(RowHeight::Tall).panel(
            "cloud-egress-stub",
            Panel::text("VPC and Egress", EGRESS_NOTE)
                .description("What a VPC-side scrape would add here, and why it is absent.")
                .build(0),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stub_names_what_would_fill_it() {
        // A stub that only says "not yet" is a worse version of an empty row.
        // These have to hand the next reader the actual target.
        for note in [PROVIDER_NOTE, EGRESS_NOTE] {
            assert!(note.contains("DEP-233"), "{note}");
            assert!(note.contains("not collected yet"), "{note}");
        }
        assert!(PROVIDER_NOTE.contains("CloudWatch"));
        assert!(EGRESS_NOTE.contains("NAT gateway"));
    }

    #[test]
    fn every_stub_offers_the_nearest_substitute() {
        // The reader came with a question. Being told the answer is unavailable
        // is only useful alongside the closest thing that is.
        for note in [PROVIDER_NOTE, EGRESS_NOTE] {
            assert!(note.contains("Until then"), "{note}");
        }
    }
}
