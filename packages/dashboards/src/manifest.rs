// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Panel manifests: what each backend's dashboard shows, in a form both share.
//!
//! The parity ledger in `tests/datadog_parity.rs` compares a Datadog dashboard's
//! manifest with its Grafana counterpart's, panel by panel. A manifest records
//! what a reader would call the same panel: its slug, title, registry queries,
//! broad kind, and unit. Layout, colors, legends, and transformations belong to
//! each format, and comparing them would mostly report noise.
//!
//! The Datadog builders record their manifest as they build each widget. The
//! Grafana one is read back from the render in [`crate::grafana::manifest`],
//! because a Grafana panel's builder never sees the slug it is placed under.

/// One panel, as both backends describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// The stable id the panel is placed under, such as `freshness-lag-by-cluster`.
    pub slug: String,
    /// The title the reader sees.
    pub title: String,
    /// The registry ids the panel reads, in order.
    pub queries: Vec<String>,
    pub kind: Kind,
    pub unit: Unit,
}

/// A broad class of panel, coarse enough that the two formats' widgets map onto
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Values over time: a Grafana time series, a Datadog timeseries.
    TimeSeries,
    /// The latest value, or one per group: a Grafana stat or gauge, a Datadog
    /// query value or toplist.
    Value,
    /// Rows and columns.
    Table,
    /// Parts of a whole: a Grafana pie chart, a Datadog sunburst.
    Pie,
    /// The top or bottom few, as bars: a Grafana bar chart, a Datadog toplist.
    Ranking,
    /// Anything else, such as a Grafana logs panel, by its plugin id.
    Other(String),
}

/// A Grafana panel that its Datadog counterpart draws differently, or not at all,
/// and why.
///
/// The ledger fails on any difference that isn't listed, and on any listing that
/// no longer matches a difference, so the list can't rot into a stale excuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Difference {
    pub slug: &'static str,
    pub gap: Gap,
    pub reason: &'static str,
}

/// What differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gap {
    /// No Datadog counterpart.
    Missing,
    /// A different [`Kind`].
    Kind,
    /// A different [`Unit`].
    Unit,
}

/// What a panel's numbers measure, coarse enough that Grafana's unit ids and
/// Datadog's canonical units map onto it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unit {
    /// A count, a ratio, or nothing a unit names.
    None,
    Seconds,
    Bytes,
    BytesPerSecond,
    /// A percentage, whether stored as 0–100 or as a fraction of 1.
    Percent,
    Cores,
    /// Events per second, whatever the event.
    PerSecond,
    /// A unit with no counterpart here, by its own id.
    Other(String),
}
