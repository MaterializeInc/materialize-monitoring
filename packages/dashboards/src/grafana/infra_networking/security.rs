// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Security tab: which traffic is allowed, and what was stopped.
//!
//! # Two halves, and only one of them always works
//!
//! **Policy as declared** comes from kube-state-metrics and is present
//! everywhere: which namespaces have policies, how many rules each carries, and
//! — the panel worth the tab — which namespaces have none at all.
//!
//! **Policy as enforced** can only come from the CNI, because nothing else sees
//! a packet. Those rows are conditioned on the detected dataplane exactly as the
//! CNI tab's are, and on a cluster with no dataplane metrics that half is
//! simply absent.
//!
//! The gap between the halves is itself worth knowing about: Kubernetes will
//! accept a NetworkPolicy on a cluster whose CNI does not implement policy at
//! all and report nothing wrong. The object exists, the rules read correctly,
//! and no packet is ever filtered. Legacy-datapath GKE does this. So a cluster
//! showing a healthy policy inventory and no enforcement metrics has not
//! demonstrated that any of it works.
//!
//! # Coverage is the finding
//!
//! An operator opening this tab is usually asking one of two questions — "is
//! something being blocked that should not be" or "is anything unprotected". The
//! second is answered by an *absence*, which no metric reports, so
//! `infra.net.security.policies.uncovered` subtracts the namespaces that have
//! policies from those that have pods. It is the one panel here that states a
//! negative directly, and it goes first.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::variable::extra;

use crate::grafana::queries::Queries;
use crate::grafana::transform;

/// The variable the enforcement rows are rendered on, as on the CNI tab.
const DATAPLANE: &str = extra::NETWORK_COMPONENT_LIST;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        coverage(q),
        inventory(q),
        aws_enforcement(q),
        cilium_enforcement(q),
    ]
}

/// What is protected and what is not.
fn coverage(q: &Queries) -> Row {
    Row::new("Policy Coverage").grid(
        AutoGrid::new(2)
            .panel("security-uncovered", uncovered(q))
            .panel("security-policies-by-namespace", by_namespace(q)),
    )
}

/// What each policy actually says.
fn inventory(q: &Queries) -> Row {
    Row::new("NetworkPolicy Inventory").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("security-policy-rules", rules(q)),
    )
}

fn aws_enforcement(q: &Queries) -> Row {
    Row::new("Enforcement: AWS VPC CNI")
        .only_when_variable(DATAPLANE, super::cni::AWS)
        .grid(
            AutoGrid::new(1)
                .column_width(ColumnWidth::Wide)
                .panel("security-drops-aws", aws_drops(q)),
        )
}

fn cilium_enforcement(q: &Queries) -> Row {
    Row::new("Enforcement: Cilium")
        .only_when_variable(DATAPLANE, super::cni::CILIUM)
        .grid(
            AutoGrid::new(1)
                .column_width(ColumnWidth::Wide)
                .panel("security-drops-cilium", cilium_drops(q)),
        )
}

/// The negative that no metric reports.
///
/// A table rather than a stat: the count alone would prompt the question this
/// answers, which is *which* namespaces.
fn uncovered(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Namespaces Without Policy")
        .query(
            q.get("infra.net.security.policies.uncovered")
                .table_format(),
        )
        .no_value(NoValue::Custom(
            "Every namespace running pods has at least one NetworkPolicy.".to_string(),
        ))
        .transformations(vec![transform::organize(
            &["Time", "Value", "__name__"],
            &["namespace"],
        )])
        .build(0)
}

fn by_namespace(q: &Queries) -> dashboardv2::PanelKind {
    Panel::barchart("Policies by Namespace")
        .query(
            q.get("infra.net.security.policies.by_namespace")
                .legend("{{namespace}}"),
        )
        .unit("short")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No NetworkPolicy objects exist in this cluster.".to_string(),
        ))
        .build(0)
}

fn rules(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Ingress and Egress Rules per Policy")
        .query(q.get("infra.net.security.policies.rules").table_format())
        // No `noValue`: two queries joined into one table, so Grafana would put
        // the message in every empty *cell* rather than standing in for an empty
        // panel. A policy with only ingress rules legitimately has a blank
        // egress column.
        .transformations(vec![
            transform::merge(),
            // `organize_full`, not `organize_renamed`: the latter takes an
            // *order* rather than an exclusion, so naming Time there pinned it
            // to column zero instead of hiding it. A merged instant query still
            // carries a Time field, and it is the same value on every row.
            transform::organize_full(
                &["Time", "__name__"],
                &["namespace", "networkpolicy"],
                &[
                    ("Value #query-0", "Ingress Rules"),
                    ("Value #query-1", "Egress Rules"),
                ],
            ),
            // One namespace at a time. A cluster's whole policy set is a long
            // flat list and nobody reads it that way -- the question is always
            // about one namespace, and the row count per namespace is itself
            // worth seeing at a glance.
            transform::group_to_nested_table("namespace"),
        ])
        .build(0)
}

fn aws_drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Packets Dropped by Policy")
        .query(q.get("infra.net.security.drops.aws").legend("{{node}}"))
        .unit("pps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No packets have been dropped by a NetworkPolicy.".to_string(),
        ))
        .build(0)
}

fn cilium_drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Packets Dropped by Policy")
        .query(q.get("infra.net.security.drops.cilium").legend("{{node}}"))
        .unit("pps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No packets have been dropped by a NetworkPolicy.".to_string(),
        ))
        .build(0)
}
