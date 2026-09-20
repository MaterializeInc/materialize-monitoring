// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Kubernetes tab: pods, Services, and the routing between them.
//!
//! Everything here comes from cAdvisor or kube-state-metrics, so unlike the CNI
//! tab it is the same on every cluster.
//!
//! The rows run from traffic to intent to mechanism: what the pods are sending,
//! what Services were declared and what is behind them, and — where kube-proxy
//! exists — how long it takes for a declaration to become a rule on the node.
//! That last row is the answer to a question the first two cannot ask, which is
//! why a Service can be failing while every pod behind it is healthy.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![traffic(q), services(q), proxy(q)]
}

fn traffic(q: &Queries) -> Row {
    Row::new("Pod Traffic").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("k8s-throughput-by-namespace", throughput(q))
            .panel("k8s-errors-by-namespace", errors(q)),
    )
}

fn services(q: &Queries) -> Row {
    Row::new("Services and Endpoints").grid(
        AutoGrid::new(2)
            .panel("k8s-services-by-type", by_type(q))
            .panel("k8s-endpoints-by-namespace", endpoints(q)),
    )
}

/// Collected by default — the gateway scrapes kube-proxy where it runs — and
/// still collapsed, because it is empty on any cluster that runs none: GKE
/// Dataplane V2, or Cilium in kube-proxy-replacement mode. A row blank on a
/// third of installs should not be the first thing on screen, and the title
/// says it exists, which is what an operator needs in order to go looking.
fn proxy(q: &Queries) -> Row {
    Row::new("kube-proxy").collapsed().grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("k8s-proxy-sync", sync_latency(q))
            .panel("k8s-proxy-programming", programming_latency(q)),
    )
}

fn throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Throughput by Namespace")
        .query(q.legended(
            "infra.net.k8s.throughput.by_namespace",
            &["rx {{namespace}}", "tx {{namespace}}"],
        ))
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Errors and Drops by Namespace")
        .query(
            q.get("infra.net.k8s.errors.by_namespace")
                .legend("{{namespace}}"),
        )
        .unit("cps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No pod lost a packet in this window.".to_string(),
        ))
        .build(0)
}

fn by_type(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Services by Type")
        .query(q.get("infra.net.k8s.services.by_type").legend("{{type}}"))
        .unit("short")
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn endpoints(q: &Queries) -> dashboardv2::PanelKind {
    Panel::barchart("Endpoints by Namespace")
        .query(
            q.get("infra.net.k8s.endpoints.by_namespace")
                .legend("{{namespace}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

/// Shown when kube-proxy is not running, which is a legitimate cluster shape
/// rather than a gap. Both panels say the same thing, since either could be the
/// one an operator opened the row for.
fn no_kube_proxy() -> NoValue {
    NoValue::Custom(
        "No kube-proxy on this cluster — Dataplane V2 and Cilium's \
         kube-proxy replacement both program Services themselves."
            .to_string(),
    )
}

fn sync_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rule Sync Duration (p99)")
        .query(q.get("infra.net.k8s.proxy.sync_latency").legend("p99"))
        .unit("s")
        .min(0.0)
        .no_value(no_kube_proxy())
        .build(0)
}

fn programming_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Network Programming Latency (p99)")
        .query(
            q.get("infra.net.k8s.proxy.programming_latency")
                .legend("p99"),
        )
        .unit("s")
        .min(0.0)
        .no_value(no_kube_proxy())
        .build(0)
}
