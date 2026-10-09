// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Database Internals tab: the metadata database, as it reports itself.
//!
//! Every other tab is Materialize's measurement, which says *that* the database
//! is slow or refusing connections. This one is the database's own, which says
//! why — and for a database running in the cluster, there is no cloud provider
//! to ask instead. Its rows render per flavor, on a discovered variable, the
//! same way `infra-cloud`'s provider rows do; CloudNativePG is the first.
//!
//! # Scoped by a picker, not by the environment
//!
//! CNPG's series name an instance pod and nothing about Materialize, so which
//! cluster is this environment's database is the reader's to say, on
//! `$cnpgClusterList`. On the usual single cluster, "All" is already right.
//!
//! The picker is a section variable on the one CloudNativePG row that holds the
//! rest, so it renders under that header and hides with it. A dashboard-level
//! picker would sit in the controls of every environment, CNPG or not.
//!
//! # Why these signals
//!
//! The Status row answers the four questions the reference installs' incidents
//! asked of a metadata database: is it up, is it out of connection slots, is
//! vacuum held back, and what version is it. The per-database panels exist for
//! the fifth, from the field: a neighbour sharing the instance and filling it,
//! which no instance-level number can attribute.

use mzmon_lib::grafana::generated::{dashboardv2, stat::BigValueGraphMode};
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::threshold;

use super::{latency_ladder, theme, zero_is_healthy};
use crate::grafana::dependency::ratio_ladder;
use crate::grafana::queries::Queries;
use crate::grafana::transform;
use mzmon_lib::grafana::variable::{self, extra};

const SHADE: &str = theme::DATABASE.shade;

/// The variable every CNPG row renders on.
pub(crate) const CNPG_DETECTED: &str = extra::CNPG_DETECTED;

/// What `$cnpgDetected` holds when a CNPG instance is scraped: the name of the
/// metric it was discovered from, matched as a substring.
///
/// Not `.+`: a variable whose query returned nothing can reach the row
/// condition as `undefined`, which `.+` matches.
pub(crate) const CNPG: &str = "cnpg_collector_up";

/// Every flavor this tab has rows for, as one alternation.
///
/// The fallback renders on the negation of this, so a flavor added without
/// being added here would leave the fallback on screen beside its rows.
pub(crate) fn known_flavors() -> String {
    CNPG.to_string()
}

/// The title of the section holding every CNPG row.
pub(crate) const CNPG_SECTION: &str = "CloudNativePG";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![cnpg_section(q), no_flavor_row()]
}

/// Every CNPG row, in one section that renders only where CNPG is scraped and
/// carries the cluster picker its rows read.
fn cnpg_section(q: &Queries) -> Row {
    Row::section(
        CNPG_SECTION,
        vec![
            status(q),
            connections(q),
            activity(q),
            storage(q),
            replication(q),
        ],
    )
    .only_when_variable(CNPG_DETECTED, CNPG)
    .variables([variable::cnpg_clusters()])
}

/// What a CNPG panel shows when its row rendered and the series is absent.
fn not_reported() -> NoValue {
    NoValue::Custom(
        "The selected CNPG cluster has not reported this. Check the CNPG Cluster picker."
            .to_string(),
    )
}

/// Why the tab has no flavor rows on it.
pub(crate) const NO_FLAVOR: &str = "**The metadata database is not reporting its own metrics.**\n\n\
     This tab shows what a database running in the cluster says about itself: connections \
     against its ceiling, transactions, the size of each database, vacuum and replication. It \
     renders for **CloudNativePG (CNPG)**, which publishes these from every instance with nothing \
     extra to deploy.\n\n\
     - **On CNPG**, create a `PodMonitor` for the cluster, selecting `cnpg.io/cluster: <name>` \
       and the `metrics` port. CNPG's documentation has the manifest; its \
       `spec.monitoring.enablePodMonitor` is deprecated. If one exists and this note remains, \
       check `up{container=\"postgres\"}` — a scrape that fails outright, such as one blocked by a \
       NetworkPolicy, reports nothing else.\n\
     - **On a managed database** — RDS, Cloud SQL, Azure Database for PostgreSQL — the provider's \
       view is on the **Infrastructure Cloud Provider** dashboard instead.\n\n\
     The other tabs here need none of this: they are Materialize's own measurement of the \
     database, and work on every flavor.";

fn no_flavor_row() -> Row {
    Row::new("No Database Metrics")
        .only_unless_variable(CNPG_DETECTED, known_flavors())
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Tall).panel(
                "database-none-detected",
                Panel::text("No Database Metrics", NO_FLAVOR)
                    .description("Why this tab has no database rows on it.")
                    .build(0),
            ),
        )
}

// --- CNPG ----------------------------------------------------------------

fn status(q: &Queries) -> Row {
    Row::new("Status").hide_header().grid(
        AutoGrid::new(7)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("cnpg-not-serving", not_serving(q))
            .panel("cnpg-primary", primary(q))
            .panel("cnpg-connections-used", connections_used(q))
            .panel("cnpg-oldest-transaction", oldest_transaction(q))
            .panel("cnpg-xid-used", xid_used(q))
            .panel("cnpg-backup-age", backup_age(q))
            .panel("cnpg-version", version(q)),
    )
}

fn connections(q: &Queries) -> Row {
    Row::new("Connections").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("cnpg-connections-by-database", connections_by_database(q))
            .panel("cnpg-connections-by-state", connections_by_state(q))
            .panel("cnpg-transactions-held", transactions_held(q)),
    )
}

fn activity(q: &Queries) -> Row {
    Row::new("Activity").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("cnpg-transactions", transactions(q))
            .panel("cnpg-rows", rows_written(q))
            .panel("cnpg-cache-hit", cache_hit(q)),
    )
}

fn storage(q: &Queries) -> Row {
    Row::new("Storage and Vacuum").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Wide)
            .panel("cnpg-database-size", database_size(q))
            .panel("cnpg-wal", wal(q))
            .panel("cnpg-xid-age", xid_age(q)),
    )
}

fn replication(q: &Queries) -> Row {
    Row::new("Replication and Archiving").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("cnpg-replication-lag", replication_lag(q))
            .panel("cnpg-archive-failures", archive_failures(q)),
    )
}

fn not_serving(q: &Queries) -> dashboardv2::PanelKind {
    zero_is_healthy("Instances Not Serving")
        .query(q.get("infra.cnpg.health.not_serving").legend("instances"))
        .thresholds(threshold::errors(1.0, 1.0).build())
        .unit("short")
        .decimals(0.0)
        .no_value(not_reported())
        .build(0)
}

/// A name rather than a number, so the stat shows the pod label.
fn primary(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Primary")
        .query(q.get("infra.cnpg.health.primary").legend("{{pod}}"))
        .graph_mode(BigValueGraphMode::None)
        .value_size(20.0)
        .reduce_fields("/^pod$/")
        .transformations(vec![transform::labels_to_fields(&[])])
        .shade(SHADE)
        .no_value(NoValue::Custom(
            "No instance reports itself as primary.".to_string(),
        ))
        .build(0)
}

fn connections_used(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Connections Used")
        .query(q.get("infra.cnpg.health.connections_used").legend("used"))
        .color_background()
        .thresholds(ratio_ladder(0.8, 0.95))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_reported())
        .build(0)
}

fn oldest_transaction(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Oldest Open Transaction")
        .query(
            q.get("infra.cnpg.health.oldest_transaction")
                .legend("oldest"),
        )
        .color_background()
        .thresholds(latency_ladder(60.0, 900.0))
        .unit("s")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn xid_used(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Transaction IDs Used")
        .query(q.get("infra.cnpg.health.xid_used").legend("used"))
        .color_background()
        .thresholds(ratio_ladder(0.25, 0.5))
        .unit("percentunit")
        .min(0.0)
        .max(1.0)
        .no_value(not_reported())
        .build(0)
}

fn backup_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Last Backup")
        .query(q.get("infra.cnpg.health.backup_age").legend("age"))
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No completed backup. CNPG reports none when backups are not configured.".to_string(),
        ))
        .build(0)
}

fn version(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("PostgreSQL Version")
        .query(q.get("infra.cnpg.health.version").legend("{{full}}"))
        .graph_mode(BigValueGraphMode::None)
        .value_size(20.0)
        .reduce_fields("/^full$/")
        .transformations(vec![transform::labels_to_fields(&[])])
        .shade(SHADE)
        .no_value(not_reported())
        .build(0)
}

fn connections_by_database(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections by Database and Role")
        .query(q.legended(
            "infra.cnpg.connections.by_database",
            &["{{datname}} ({{usename}})", "ceiling"],
        ))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn connections_by_state(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Connections by State")
        .query(q.get("infra.cnpg.connections.by_state").legend("{{state}}"))
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn transactions_held(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Oldest Transaction and Lock Waits")
        .query(q.legended(
            "infra.cnpg.connections.oldest_transaction",
            &["oldest {{datname}} ({{state}})", "waiting on a lock"],
        ))
        .unit("s")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn transactions(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Transactions by Database")
        .query(q.legended(
            "infra.cnpg.activity.transactions",
            &["{{datname}} committed", "{{datname}} rolled back"],
        ))
        .unit("ops")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn rows_written(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rows Written by Database")
        .query(q.legended(
            "infra.cnpg.activity.rows",
            &[
                "{{datname}} inserted",
                "{{datname}} updated",
                "{{datname}} deleted",
            ],
        ))
        .unit("rowsps")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn cache_hit(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Buffer Cache Hit Ratio")
        .query(q.get("infra.cnpg.activity.cache_hit").legend("{{datname}}"))
        .unit("percentunit")
        .max(1.0)
        .no_value(not_reported())
        .build(0)
}

fn database_size(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Size by Database")
        .query(
            q.get("infra.cnpg.storage.database_size")
                .legend("{{datname}}"),
        )
        .unit("bytes")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn wal(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Write-Ahead Log")
        .query(q.legended(
            "infra.cnpg.storage.wal",
            &["{{pod}} on disk", "{{pod}} waiting to archive"],
        ))
        .unit("bytes")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn xid_age(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Transaction IDs Used by Database")
        .query(q.get("infra.cnpg.storage.xid_age").legend("{{datname}}"))
        .unit("percentunit")
        .min(0.0)
        .thresholds(ratio_ladder(0.25, 0.5))
        .no_value(not_reported())
        .build(0)
}

fn replication_lag(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Replication Lag")
        .query(q.get("infra.cnpg.replication.lag").legend("{{pod}}"))
        .unit("s")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}

fn archive_failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("WAL Archiving Failures")
        .query(
            q.get("infra.cnpg.replication.archive_failures")
                .legend("{{pod}}"),
        )
        .unit("ops")
        .min(0.0)
        .no_value(not_reported())
        .build(0)
}
