//! Generated Grafana models -- DO NOT EDIT.
//!
//! Regenerate with `./bin/gen-grafana-models.sh`. One module per upstream schema document;
//! see that script for why a flat namespace is not possible.

#![allow(dead_code, clippy::derivable_impls, clippy::large_enum_variant)]

pub mod dashboardv2;

/// Grafana plugin id `timeseries` (panelcfg).
pub const TIMESERIES_PLUGIN_ID: &str = "timeseries";
pub mod timeseries;

/// Grafana plugin id `stat` (panelcfg).
pub const STAT_PLUGIN_ID: &str = "stat";
pub mod stat;

/// Grafana plugin id `piechart` (panelcfg).
pub const PIECHART_PLUGIN_ID: &str = "piechart";
pub mod piechart;

/// Grafana plugin id `table` (panelcfg).
pub const TABLE_PLUGIN_ID: &str = "table";
pub mod table;

/// Grafana plugin id `gauge` (panelcfg).
pub const GAUGE_PLUGIN_ID: &str = "gauge";
pub mod gauge;

/// Grafana plugin id `barchart` (panelcfg).
pub const BARCHART_PLUGIN_ID: &str = "barchart";
pub mod barchart;

/// Grafana plugin id `logs` (panelcfg).
pub const LOGS_PLUGIN_ID: &str = "logs";
pub mod logs;

/// Grafana plugin id `text` (panelcfg).
pub const TEXT_PLUGIN_ID: &str = "text";
pub mod text;

/// Grafana plugin id `prometheus` (dataquery).
pub const PROMETHEUS_PLUGIN_ID: &str = "prometheus";
pub mod prometheus;

/// Grafana plugin id `loki` (dataquery).
pub const LOKI_PLUGIN_ID: &str = "loki";
pub mod loki;
