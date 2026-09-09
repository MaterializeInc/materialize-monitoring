// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The tags shipped dashboards carry.
//!
//! Tags are the flat index over the dashboard list — the folder a dashboard lives
//! in ([`super::folder`]) puts it in one place, tags let it be found from several.
//! They are user-visible in Grafana's dashboard search and are what an operator
//! filters a mixed Grafana by, so they are declared here once rather than retyped
//! per dashboard.

/// Everything this repository ships, and nothing else in the Grafana.
///
/// The one tag common to every dashboard, so it is the filter that answers "what
/// came from `materialize-monitoring`" in a Grafana that also holds dashboards
/// from elsewhere. It replaced `monitoring`, which was too generic to answer that.
pub const MZMON: &str = "mzmon";

/// Dashboards scoped to Materialize itself.
pub const MATERIALIZE: &str = "materialize";

/// Dashboards scoped to the platform Materialize runs on.
pub const INFRA: &str = "infrastructure";

/// What a dashboard is about, as opposed to what it is scoped to.
///
/// A dashboard carries one scope tag above and as many of these as apply.
pub mod content {
    /// Reads from Loki rather than from a metrics datasource.
    pub const LOGS: &str = "logs";

    /// Covers a Materialize upgrade.
    pub const UPGRADE: &str = "upgrade";

    /// Covers the nodes of the cluster.
    pub const NODES: &str = "nodes";
}
