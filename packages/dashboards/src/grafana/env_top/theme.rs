// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Tab identities and their theme colours, for the whole dashboard.
//!
//! Every tab gets one colour from the qualitative palette, and its non-health
//! stat panels shade themselves with it. Keeping the assignment here rather than
//! in each tab module is what makes the scheme reviewable: you can see at a
//! glance that no two tabs collide, and reordering or recolouring a tab is one
//! edit instead of a search across six files.
//!
//! The colours come from [`palette::THEME`], which is *qualitative* — no ordering,
//! and deliberately free of red, because red reads as a health colour and a tab's
//! identity is not a health signal. Health colouring is thresholds' job
//! ([`mzmon_lib::grafana::threshold`]); a tab shade only says "you are here".
//!
//! # Summary borrows
//!
//! The Summary tab has no colour of its own. Its panels are pointers into the
//! other tabs, so each takes the shade of the tab it points at — the total-CPU
//! and total-memory panels are Kubernetes-blue because that is where you go next,
//! and the hydration count is Compute-orange for the same reason. That is the
//! coordination this module exists for: `summary.rs` writes
//! `theme::KUBERNETES.shade` and stays correct if Kubernetes is ever recoloured.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it
    /// (as `_Kubernetes Workloads_`).
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Pointers to the other tabs, and the environment's health at a glance.
///
/// Deliberately shadeless — see the module docs.
pub const SUMMARY_TITLE: &str = "Summary";

/// Pods, containers, and the resources they are allowed.
pub const KUBERNETES: Theme = Theme {
    title: "Kubernetes Workloads",
    shade: palette::THEME[0], // blue
};

/// Sessions, queries, and the SQL control plane.
pub const CONNECTIONS: Theme = Theme {
    title: "Connections / Activity",
    shade: palette::THEME[1], // cyan
};

/// Clusters and their replicas.
pub const CLUSTERS: Theme = Theme {
    title: "Cluster Objects / Replicas",
    shade: palette::THEME[2], // teal
};

/// Dataflows, arrangements, and freshness.
pub const COMPUTE: Theme = Theme {
    title: "Compute Objects",
    shade: palette::THEME[3], // orange
};

/// Sources, sinks, and the storage layer.
pub const SOURCES_SINKS: Theme = Theme {
    title: "Sources and Sinks",
    shade: palette::THEME[4], // yellow
};

/// Every themed tab, in the order they appear.
///
/// Summary is absent because it has no shade of its own.
pub const THEMED: [Theme; 5] = [KUBERNETES, CONNECTIONS, CLUSTERS, COMPUTE, SOURCES_SINKS];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_two_tabs_share_a_shade() {
        // The whole point of a per-tab colour is telling tabs apart.
        let shades: HashSet<&str> = THEMED.iter().map(|t| t.shade).collect();
        assert_eq!(shades.len(), THEMED.len(), "two tabs share a shade");
    }

    #[test]
    fn no_two_tabs_share_a_title() {
        let titles: HashSet<&str> = THEMED.iter().map(|t| t.title).collect();
        assert_eq!(titles.len(), THEMED.len());
        assert!(!titles.contains(SUMMARY_TITLE));
    }

    #[test]
    fn every_shade_comes_from_the_qualitative_palette() {
        // Not the sequential one: a health colour would make a tab's identity read
        // as a health verdict.
        for theme in THEMED {
            assert!(
                palette::THEME.contains(&theme.shade),
                "{} uses {} which is not a THEME colour",
                theme.title,
                theme.shade
            );
            assert!(
                !palette::INCANDESCENT.contains(&theme.shade),
                "{} uses a health colour",
                theme.title
            );
        }
    }

    #[test]
    fn the_shades_match_the_baseline_assignment() {
        // Pinned so a palette reorder is a visible change rather than a silent
        // recolouring of every tab.
        assert_eq!(KUBERNETES.shade, "#0077BB");
        assert_eq!(CONNECTIONS.shade, "#33BBEE");
        assert_eq!(CLUSTERS.shade, "#009988");
        assert_eq!(COMPUTE.shade, "#EE7733");
        assert_eq!(SOURCES_SINKS.shade, "#CCBB44");
    }
}
