// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Network tab: bytes moved, and everything that stopped them.
//!
//! Throughput first because it is what people look for, then the three ways a
//! packet dies — the interface dropping it, the kernel dropping it, and the
//! protocol giving up — because that is the order in which the cause gets harder
//! to see.
//!
//! **Errors and drops are what matter, not throughput.** A node saturating its
//! link is a capacity question with an obvious answer. Drops on an unsaturated
//! link mean something is wrong with the machine or the network under it, and
//! that is the finding worth handing over.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::theme;
use crate::grafana::queries::Queries;

/// Shown when a counter has recorded nothing, which for an error counter is the
/// healthy reading rather than a gap.
fn none_recorded(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} recorded."))
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![throughput(q), losses(q), protocols(q), kernel(q)]
}

fn throughput(q: &Queries) -> Row {
    Row::new("Throughput").grid(
        AutoGrid::new(2)
            .panel("network-throughput", bytes(q))
            .panel("network-saturation", saturation(q)),
    )
}

fn losses(q: &Queries) -> Row {
    Row::new("Errors and Drops").grid(
        AutoGrid::new(3)
            .panel("network-errors", errors(q))
            .panel("network-drops", drops(q))
            .panel("network-operstate", operstate(q)),
    )
}

fn protocols(q: &Queries) -> Row {
    Row::new("TCP and UDP").grid(
        AutoGrid::new(3)
            .panel("tcp-sockets", tcp_sockets(q))
            .panel("tcp-retransmits", tcp_retransmits(q))
            .panel("tcp-errors", tcp_errors(q))
            .panel("udp-errors", udp_errors(q))
            .panel("udp-queues", udp_queues(q))
            .panel("socket-memory", socket_memory(q)),
    )
}

fn kernel(q: &Queries) -> Row {
    Row::new("Kernel Networking").collapsed().grid(
        AutoGrid::new(3)
            .panel("softnet-processed", softnet_processed(q))
            .panel("softnet-dropped", softnet_dropped(q))
            .panel("softnet-squeezed", softnet_squeezed(q))
            .panel("conntrack", conntrack(q))
            .panel("arp-entries", arp(q)),
    )
}

fn bytes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Network Throughput")
        .query(q.legended(
            "node.debug.network.throughput",
            &["rx {{device}}", "tx {{device}}"],
        ))
        .unit("Bps")
        .min(0.0)
        .build(0)
}

fn saturation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Link Saturation")
        .query(q.legended(
            "node.debug.network.saturation",
            &["rx {{device}}", "tx {{device}}"],
        ))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .build(0)
}

fn errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Interface Errors")
        .query(q.legended("node.network.errors", &["rx {{device}}", "tx {{device}}"]))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("interface errors"))
        .build(0)
}

fn drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Interface Drops")
        .query(q.legended("node.network.drops", &["rx {{device}}", "tx {{device}}"]))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("interface drops"))
        .build(0)
}

fn operstate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Interface State")
        .query(q.legended(
            "node.debug.network.operstate",
            &["up {{device}}", "carrier {{device}}"],
        ))
        .min(0.0)
        .max(1.0)
        .decimals(0.0)
        .build(0)
}

fn tcp_sockets(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("TCP Sockets")
        .query(q.legended(
            "node.debug.sockets.tcp",
            &["in use", "allocated", "orphaned", "time-wait"],
        ))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn tcp_retransmits(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("TCP Retransmits")
        .query(q.legended(
            "node.debug.tcp.retransmits",
            &["retransmitted", "SYN retransmits", "segments sent"],
        ))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn tcp_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("TCP Errors")
        .query(q.legended(
            "node.debug.tcp.errors",
            &[
                "listen overflows",
                "listen drops",
                "receive queue drops",
                "timeouts",
            ],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("TCP errors"))
        .build(0)
}

fn udp_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("UDP Errors")
        .query(q.legended(
            "node.debug.udp.errors",
            &["in errors", "receive buffer errors", "no port"],
        ))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("UDP errors"))
        .build(0)
}

fn udp_queues(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("UDP Queues")
        .query(q.legended("node.debug.udp.queues", &["receive", "transmit"]))
        .unit("bytes")
        .min(0.0)
        .build(0)
}

fn socket_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Socket Memory")
        .query(q.legended("node.debug.sockets.memory", &["TCP", "UDP"]))
        .unit("bytes")
        .min(0.0)
        .build(0)
}

fn softnet_processed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Softnet Processed")
        .query(q.get("node.debug.softnet.processed").legend("core {{cpu}}"))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn softnet_dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Softnet Dropped")
        .query(q.get("node.debug.softnet.dropped").legend("core {{cpu}}"))
        .unit("cps")
        .min(0.0)
        .no_value(none_recorded("softnet drops"))
        .build(0)
}

fn softnet_squeezed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Softnet Squeezed")
        .query(q.get("node.debug.softnet.squeezed").legend("core {{cpu}}"))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn conntrack(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Conntrack Table")
        .query(q.get("node.conntrack.utilization").legend("in use"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .shade(theme::NETWORK.shade)
        .build(0)
}

fn arp(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("ARP Entries")
        .query(q.get("node.debug.arp.entries").legend("{{device}}"))
        .unit("short")
        .min(0.0)
        .build(0)
}
