// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Metadata Database tab: each instance, as its provider reports it.
//!
//! # Headroom first, and for every provider at once
//!
//! CPU, connections and transaction-ID consumption are the three quantities all
//! three providers publish in a comparable form, so the Headroom row draws each
//! as one panel with one query per provider. On any install two of the three
//! return nothing, which costs nothing and keeps a single-cloud reader from
//! having to know which row is theirs. The row is still rendered on the provider
//! variable, so that a cluster pulling from nothing shows the fallback instead
//! of three empty graphs.
//!
//! Storage is the one headroom quantity that is not comparable, and so the one
//! that is not on that row: CloudWatch publishes RDS free space in bytes and not
//! the allocated size, while Cloud SQL and Azure publish only a fraction. It
//! opens each provider's own rows instead, directly under Headroom — which,
//! since only one provider's rows render, is where a reader looks next anyway.
//!
//! # Why these signals
//!
//! The incidents this project has seen against a metadata database are a full
//! disk, a saturated CPU, a neighbouring workload consuming a shared instance,
//! and running out of connection slots. The first two are the Headroom and
//! storage rows. The third is what _Connections by Database_ is for — Cloud SQL
//! is the one provider that publishes connections per database, which is the
//! attribution an instance-level metric cannot give. The fourth is the
//! Connections panel, drawn beside the number Materialize itself holds.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::Panel;
use mzmon_lib::grafana::threshold;

use super::{
    AWS, AZURE, DB_LEGENDS, GCP, PROVIDERS, known_providers, no_provider_row, not_pulled,
    ratio_ladder,
};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        headroom(q),
        rds_storage(q),
        rds_io(q),
        cloudsql_storage(q),
        cloudsql_connections(q),
        flexible_storage(q),
        flexible_io(q),
        no_provider_row("db-none-detected"),
    ]
}

// --- Every provider ------------------------------------------------------

fn headroom(q: &Queries) -> Row {
    Row::new("Headroom")
        .only_when_variable(PROVIDERS, known_providers())
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("db-cpu", cpu(q))
                .panel("db-connections", connections(q))
                .panel("db-transaction-ids", transaction_ids(q)),
        )
}

fn cpu(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Utilization")
        .query(q.legended("infra.cloud.db.cpu", &DB_LEGENDS))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        // 60% is the sizing guidance's steady-state ceiling, not a failure: an
        // instance above it has too little room for an upgrade.
        .thresholds(ratio_ladder(0.6, 0.9))
        .no_value(not_pulled())
        .build(0)
}

fn connections(q: &Queries) -> dashboardv2::PanelKind {
    let [aws, gcp, azure] = DB_LEGENDS;
    Panel::timeseries("Connections: Database and Materialize")
        .query(q.legended(
            "infra.cloud.db.connections",
            &[aws, gcp, azure, "held by Materialize"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn transaction_ids(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Transaction IDs Used")
        .query(q.legended("infra.cloud.db.transaction_ids", &DB_LEGENDS))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.25, 0.5))
        .no_value(not_pulled())
        .build(0)
}

// --- RDS -----------------------------------------------------------------

const RDS: &str = "{{dimension_DBInstanceIdentifier}}";

fn rds_storage(q: &Queries) -> Row {
    Row::new("RDS: Storage and Memory")
        .only_when_variable(PROVIDERS, AWS)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("rds-free-storage", rds_free_storage(q))
                .panel("rds-freeable-memory", rds_freeable_memory(q))
                .panel("rds-cpu-credits", rds_cpu_credits(q)),
        )
}

fn rds_io(q: &Queries) -> Row {
    Row::new("RDS: Disk I/O")
        .only_when_variable(PROVIDERS, AWS)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("rds-io-latency", rds_io_latency(q))
                .panel("rds-queue-depth", rds_queue_depth(q))
                .panel("rds-balances", rds_balances(q)),
        )
}

fn rds_free_storage(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Free Storage")
        .query(q.get("infra.cloud.rds.free_storage").legend(RDS))
        .unit("bytes")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn rds_freeable_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Freeable Memory")
        .query(q.get("infra.cloud.rds.freeable_memory").legend(RDS))
        .unit("bytes")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn rds_cpu_credits(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Credit Balance")
        .query(q.get("infra.cloud.rds.cpu_credits").legend(RDS))
        .unit("short")
        .min(0.0)
        .no_value(mzmon_lib::grafana::panel::NoValue::Custom(
            "No burstable (db.t*) instances.".to_string(),
        ))
        .build(0)
}

fn rds_io_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Latency")
        .query(q.legended(
            "infra.cloud.rds.io_latency",
            &[
                "read {{dimension_DBInstanceIdentifier}}",
                "write {{dimension_DBInstanceIdentifier}}",
            ],
        ))
        .unit("s")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn rds_queue_depth(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Queue Depth")
        .query(q.get("infra.cloud.rds.queue_depth").legend(RDS))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn rds_balances(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Burst and I/O Balances")
        .query(q.legended(
            "infra.cloud.rds.io_balances",
            &[
                "burst {{dimension_DBInstanceIdentifier}}",
                "EBS I/O {{dimension_DBInstanceIdentifier}}",
                "EBS throughput {{dimension_DBInstanceIdentifier}}",
            ],
        ))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(threshold::health(0.1, 0.5).build())
        .no_value(not_pulled())
        .build(0)
}

// --- Cloud SQL -----------------------------------------------------------

const CLOUDSQL: &str = "{{database_id}}";

fn cloudsql_storage(q: &Queries) -> Row {
    Row::new("Cloud SQL: Storage and Memory")
        .only_when_variable(PROVIDERS, GCP)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("cloudsql-disk", cloudsql_disk(q))
                .panel("cloudsql-memory", cloudsql_memory(q))
                .panel("cloudsql-up", cloudsql_up(q)),
        )
}

fn cloudsql_connections(q: &Queries) -> Row {
    Row::new("Cloud SQL: Connections")
        .only_when_variable(PROVIDERS, GCP)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("cloudsql-by-database", cloudsql_by_database(q))
                .panel("cloudsql-by-state", cloudsql_by_state(q)),
        )
}

fn cloudsql_disk(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Used")
        .query(q.get("infra.cloud.cloudsql.disk").legend(CLOUDSQL))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.8, 0.9))
        .no_value(not_pulled())
        .build(0)
}

fn cloudsql_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Used")
        .query(q.get("infra.cloud.cloudsql.memory").legend(CLOUDSQL))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_pulled())
        .build(0)
}

fn cloudsql_up(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Instance Serving")
        .query(q.get("infra.cloud.cloudsql.up").legend(CLOUDSQL))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(not_pulled())
        .build(0)
}

fn cloudsql_by_database(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections by Database")
        .query(
            q.get("infra.cloud.cloudsql.backends_by_database")
                .legend("{{database_id}} {{database}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn cloudsql_by_state(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections by State")
        .query(
            q.get("infra.cloud.cloudsql.backends_by_state")
                .legend("{{database_id}} {{state}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

// --- Azure Flexible Server -----------------------------------------------

const FLEXIBLE: &str = "{{resourceName}}";

fn flexible_storage(q: &Queries) -> Row {
    Row::new("Flexible Server: Storage and Memory")
        .only_when_variable(PROVIDERS, AZURE)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("flexible-storage", flexible_storage_used(q))
                .panel("flexible-memory", flexible_memory(q))
                .panel("flexible-cpu-credits", flexible_cpu_credits(q))
                .panel("flexible-alive", flexible_alive(q)),
        )
}

fn flexible_io(q: &Queries) -> Row {
    Row::new("Flexible Server: Disk I/O and Connections")
        .only_when_variable(PROVIDERS, AZURE)
        .grid(
            AutoGrid::new(3)
                .column_width(ColumnWidth::Wide)
                .panel("flexible-io-consumed", flexible_io_consumed(q))
                .panel("flexible-queue-depth", flexible_queue_depth(q))
                .panel(
                    "flexible-connections-failed",
                    flexible_connections_failed(q),
                ),
        )
}

fn flexible_storage_used(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Storage Used")
        .query(q.get("infra.cloud.flexible.storage").legend(FLEXIBLE))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.8, 0.9))
        .no_value(not_pulled())
        .build(0)
}

fn flexible_memory(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Memory Used")
        .query(q.get("infra.cloud.flexible.memory").legend(FLEXIBLE))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_pulled())
        .build(0)
}

fn flexible_cpu_credits(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("CPU Credits Remaining")
        .query(q.get("infra.cloud.flexible.cpu_credits").legend(FLEXIBLE))
        .unit("short")
        .min(0.0)
        .no_value(mzmon_lib::grafana::panel::NoValue::Custom(
            "No Burstable-tier servers.".to_string(),
        ))
        .build(0)
}

fn flexible_alive(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Server Reachable")
        .query(q.get("infra.cloud.flexible.alive").legend(FLEXIBLE))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(not_pulled())
        .build(0)
}

fn flexible_io_consumed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk IOPS and Throughput Used")
        .query(q.legended(
            "infra.cloud.flexible.io_consumed",
            &["IOPS {{resourceName}}", "throughput {{resourceName}}"],
        ))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .thresholds(ratio_ladder(0.8, 1.0))
        .no_value(not_pulled())
        .build(0)
}

fn flexible_queue_depth(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Disk Queue Depth")
        .query(q.get("infra.cloud.flexible.queue_depth").legend(FLEXIBLE))
        .unit("short")
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn flexible_connections_failed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Refused Connections")
        .query(
            q.get("infra.cloud.flexible.connections_failed")
                .legend(FLEXIBLE),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}
