// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Node Networking tab: the machines' own interfaces, across the fleet.
//!
//! **Every query here is one `infra-nodes` already uses.** `node-health.yaml`
//! and `node-debug.yaml` scope themselves with `instance=~"$nodeList"`, and the
//! only difference between the two dashboards is what that variable holds — one
//! address there, every address here. So this tab costs no new registry entries
//! and inherits the vetting those families already carry.
//!
//! What does change is the legends. On a single-node dashboard `{{device}}` is
//! unambiguous; across a fleet it draws one line per interface per machine with
//! no way to tell which is which, so every legend here leads with `{{instance}}`.
//!
//! The rows are ordered by how far down the stack the cause is: the interface,
//! then the connection table in front of it, then the kernel's own receive path.
//! An operator reading top to bottom stops at the first row that is not flat.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use crate::grafana::queries::Queries;

/// Shown when a counter has recorded nothing, which for an error counter is the
/// healthy reading rather than a gap.
fn none_recorded(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} recorded."))
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![throughput(q), losses(q), connections(q), kernel(q)]
}

fn throughput(q: &Queries) -> Row {
    Row::new("Throughput").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("nodes-rx", rx(q))
            .panel("nodes-tx", tx(q)),
    )
}

fn losses(q: &Queries) -> Row {
    Row::new("Errors and Drops").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("nodes-errors", errors(q))
            .panel("nodes-drops", drops(q)),
    )
}

/// Conntrack and sockets: the two tables that fill up and start refusing
/// connections on a machine that otherwise looks healthy.
fn connections(q: &Queries) -> Row {
    Row::new("Connection Tracking").grid(
        AutoGrid::new(3)
            .panel("nodes-conntrack", conntrack(q))
            .panel("nodes-tcp-sockets", tcp_sockets(q))
            .panel("nodes-tcp-retransmits", retransmits(q)),
    )
}

fn kernel(q: &Queries) -> Row {
    Row::new("Kernel Receive Path").collapsed().grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("nodes-softnet-dropped", softnet_dropped(q))
            .panel("nodes-softnet-squeezed", softnet_squeezed(q)),
    )
}

fn rx(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Received")
        .query(q.get("node.network.rx.total").legend("{{instance}}"))
        .unit("Bps")
        .min(0.0)
        .build(0)
}

fn tx(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Transmitted")
        .query(q.get("node.network.tx.total").legend("{{instance}}"))
        .unit("Bps")
        .min(0.0)
        .build(0)
}

fn errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Interface Errors")
        .query(q.legended(
            "node.network.errors",
            &["rx {{instance}} {{device}}", "tx {{instance}} {{device}}"],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("interface errors"))
        .build(0)
}

fn drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Interface Drops")
        .query(q.legended(
            "node.network.drops",
            &["rx {{instance}} {{device}}", "tx {{instance}} {{device}}"],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("interface drops"))
        .build(0)
}

fn conntrack(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Conntrack Table")
        .query(q.get("node.conntrack.utilization").legend("{{instance}}"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .build(0)
}

fn tcp_sockets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("TCP Sockets")
        .query(q.legended(
            "node.debug.sockets.tcp",
            &[
                "in use {{instance}}",
                "allocated {{instance}}",
                "orphaned {{instance}}",
                "time-wait {{instance}}",
            ],
        ))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn retransmits(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("TCP Retransmits")
        .query(q.legended(
            "node.debug.tcp.retransmits",
            &[
                "retransmitted {{instance}}",
                "SYN retransmits {{instance}}",
                "segments sent {{instance}}",
            ],
        ))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn softnet_dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Softnet Dropped")
        .query(
            q.get("node.debug.softnet.dropped")
                .legend("{{instance}} core {{cpu}}"),
        )
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("softnet drops"))
        .build(0)
}

fn softnet_squeezed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Softnet Squeezed")
        .query(
            q.get("node.debug.softnet.squeezed")
                .legend("{{instance}} core {{cpu}}"),
        )
        .unit("cps")
        .min(0.0)
        .build(0)
}
