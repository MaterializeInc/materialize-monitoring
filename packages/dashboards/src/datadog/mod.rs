// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Datadog dashboards, built on the models in [`datadog_api_client`].
//!
//! The layout mirrors [`crate::grafana`]: one module per dashboard, and [`ALL`]
//! as the registry the renderer walks. Only the crate's models are used. Nothing
//! here calls Datadog's API, because a Datadog dashboard reaches Datadog through
//! an operator importing the JSON the docsite offers, never through the cluster.
//! That is also why there is no YAML form: no chart carries these.
//!
//! A dashboard shares its artifact stem with its Grafana counterpart, which is
//! what the docsite's download table keys on.

pub mod env_top;
pub mod formula;
pub mod queries;
pub mod render;
pub mod variable;
pub mod widget;

use datadog_api_client::datadogV1::model::Dashboard;
use mzmon_lib::query::QueryRegistry;

/// Everything a dashboard needs from its deployment.
#[derive(Debug, Clone)]
pub struct Options {
    /// `mz_` on self-managed, `v2_mz_` on Materialize Cloud. Reaches every query
    /// that reads a SQL-derived metric.
    pub sql_metric_prefix: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            sql_metric_prefix: "mz_".to_string(),
        }
    }
}

/// A dashboard the renderer can emit.
pub struct Renderable {
    /// Artifact stem, shared with the Grafana dashboard this one ports.
    pub name: &'static str,
    /// One-line summary, for `--list`.
    pub summary: &'static str,
    pub render: fn(&Options, &QueryRegistry) -> render::Result<Dashboard>,
    /// The panels, for the parity ledger in `tests/datadog_parity.rs`.
    pub manifest: fn(&Options, &QueryRegistry) -> render::Result<Vec<crate::manifest::Panel>>,
    /// Grafana panels this dashboard draws differently, or not at all.
    pub differences: &'static [crate::manifest::Difference],
}

/// Every dashboard, in artifact order.
pub const ALL: &[Renderable] = &[Renderable {
    name: env_top::NAME_STEM,
    summary: "Environment overview: health, workloads, connections, clusters, compute, storage",
    render: env_top::render,
    manifest: env_top::manifest,
    differences: env_top::DIFFERENCES,
}];

/// Look one up by artifact stem.
pub fn find(name: &str) -> Option<&'static Renderable> {
    ALL.iter().find(|d| d.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stem_names_a_grafana_dashboard() {
        // The docsite's download table looks a dashboard up by stem in each
        // format, so a Datadog stem with no Grafana twin would have no row.
        for dashboard in ALL {
            assert!(
                crate::grafana::find(dashboard.name).is_some(),
                "{} has no Grafana counterpart",
                dashboard.name
            );
        }
    }

    #[test]
    fn stems_are_unique() {
        let mut stems: Vec<&str> = ALL.iter().map(|d| d.name).collect();
        stems.sort_unstable();
        stems.dedup();
        assert_eq!(stems.len(), ALL.len(), "duplicate artifact stem");
    }
}
