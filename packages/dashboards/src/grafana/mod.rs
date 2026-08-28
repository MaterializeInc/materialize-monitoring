// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Grafana dashboards, built on [`mzmon_lib::grafana`].
//!
//! One module per dashboard. Each owns its own `theme` so the colour assignments
//! for its tabs live in one file rather than being spread across them.
//!
//! [`ALL`] is the registry the renderer walks, so adding a dashboard is a module
//! plus one entry — nothing in the CLI needs to know their names.

pub mod env_top;
pub mod env_upgrade;
pub mod queries;
pub mod render;
pub mod transform;

use mzmon_lib::grafana::dashboard::Resource;
use mzmon_lib::query::QueryRegistry;

/// Everything a dashboard needs from its deployment.
#[derive(Debug, Clone)]
pub struct Options {
    /// `mz_` on self-managed, `v2_mz_` on Materialize Cloud. Reaches the
    /// cluster-discovery variable, which reads a SQL-derived metric.
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
    /// Artifact stem: the chart and the docsite both key off it, so it is a stable
    /// identifier rather than a title.
    pub name: &'static str,
    /// One-line summary, for `--list`.
    pub summary: &'static str,
    pub render: fn(&Options, &QueryRegistry) -> render::Result<Resource>,
}

/// Every dashboard, in artifact order.
pub const ALL: &[Renderable] = &[
    Renderable {
        name: env_top::NAME_STEM,
        summary: "High-level overview of one Materialize environment",
        render: env_top::render,
    },
    Renderable {
        name: env_upgrade::NAME_STEM,
        summary: "What happened during a Materialize upgrade",
        render: env_upgrade::render,
    },
];

/// Look one up by artifact stem.
pub fn find(name: &str) -> Option<&'static Renderable> {
    ALL.iter().find(|d| d.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dashboard_is_findable_by_its_stem() {
        for dashboard in ALL {
            assert!(find(dashboard.name).is_some(), "{}", dashboard.name);
        }
        assert!(find("nope").is_none());
    }

    #[test]
    fn stems_are_unique_and_filename_safe() {
        // The stem becomes a filename and a Helm `Files.Glob` key.
        let mut seen = std::collections::HashSet::new();
        for dashboard in ALL {
            assert!(
                seen.insert(dashboard.name),
                "duplicate stem {}",
                dashboard.name
            );
            assert!(
                dashboard
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{} is not a safe filename stem",
                dashboard.name
            );
        }
    }
}
