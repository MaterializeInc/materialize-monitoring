// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Storage tab: the filesystems, and the disks under them.
//!
//! Filesystems first, because a full one is the failure an operator can act on
//! and the one that evicts pods. The disk rows below explain slowness rather than
//! fullness — a node whose disks are saturated makes every write on it slow,
//! which reaches Materialize as latency with no obvious cause.
//!
//! **Inodes and read-only mounts are the two that surprise people.** A
//! filesystem can be nowhere near full and still refuse writes because it has run
//! out of inodes, and a disk that hit an I/O error is silently remounted
//! read-only rather than failing loudly.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::theme;
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![filesystems(q), disks(q), descriptors(q)]
}

fn filesystems(q: &Queries) -> Row {
    Row::new("Filesystems").grid(
        AutoGrid::new(3)
            .panel("fs-available", available(q))
            .panel("fs-inodes", inodes(q))
            .panel("fs-readonly", readonly(q)),
    )
}

fn disks(q: &Queries) -> Row {
    Row::new("Disks").grid(
        AutoGrid::new(2)
            .panel("disk-iops", iops(q))
            .panel("disk-throughput", throughput(q))
            .panel("disk-latency", latency(q))
            .panel("disk-utilization", utilization(q))
            .panel("disk-queue-depth", queue_depth(q)),
    )
}

fn descriptors(q: &Queries) -> Row {
    Row::new("File Descriptors").grid(AutoGrid::new(1).panel("filefd", filefd(q)))
}

fn available(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Filesystem Available")
        .query(
            q.get("node.filesystem.available.ratio")
                .legend("{{mountpoint}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.1, 0.2).build())
        .build(0)
}

fn inodes(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Inodes Available")
        .query(
            q.get("node.debug.filesystem.inodes.available.ratio")
                .legend("{{mountpoint}}"),
        )
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.1, 0.2).build())
        .build(0)
}

fn readonly(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Read-only Mounts")
        .query(q.get("node.filesystem.readonly").legend("{{mountpoint}}"))
        .min(0.0)
        .max(1.0)
        .decimals(0.0)
        .no_value(NoValue::Custom(
            "No read-only mounts — every filesystem is still writable.".to_string(),
        ))
        .build(0)
}

fn iops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk IOPS")
        .query(q.legended(
            "node.debug.disk.iops",
            &["read {{device}}", "write {{device}}"],
        ))
        .unit("cps")
        .min(0.0)
        .build(0)
}

fn throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Throughput")
        .query(q.legended(
            "node.debug.disk.throughput",
            &["read {{device}}", "written {{device}}"],
        ))
        .unit("Bps")
        .min(0.0)
        .build(0)
}

fn latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Latency")
        .query(q.legended(
            "node.debug.disk.latency",
            &["read {{device}}", "write {{device}}"],
        ))
        .unit("s")
        .min(0.0)
        .build(0)
}

fn utilization(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Utilization")
        .query(q.get("node.disk.io_utilization").legend("{{device}}"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .build(0)
}

fn queue_depth(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Queue Depth")
        .query(q.get("node.debug.disk.queue_depth").legend("{{device}}"))
        .unit("short")
        .min(0.0)
        .build(0)
}

fn filefd(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("File Descriptors In Use")
        .query(q.get("node.filefd.utilization").legend("allocated / max"))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::utilization(0.8, 1.0, 0.05).build())
        .shade(theme::STORAGE.shade)
        .build(0)
}
