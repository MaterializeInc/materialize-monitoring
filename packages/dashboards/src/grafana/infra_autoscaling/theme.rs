// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Tab identities and their theme colours, for the whole dashboard.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Whether the cluster is keeping up with its pods.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0],
};

/// The nodes, by the pool that owns them.
pub const NODE_POOLS: Theme = Theme {
    title: "Node Pools",
    shade: palette::THEME[1],
};

/// The pods waiting for a node, and why.
pub const PENDING_PODS: Theme = Theme {
    title: "Pending Pods",
    shade: palette::THEME[2],
};

/// HorizontalPodAutoscalers: workloads that scale by adding pods.
pub const WORKLOADS: Theme = Theme {
    title: "Workload Autoscaling",
    shade: palette::THEME[3],
};

/// Whether the cloud can supply the nodes asked for.
pub const CLOUD_CAPACITY: Theme = Theme {
    title: "Cloud Capacity",
    shade: palette::THEME[4],
};

/// The autoscalers' and node lifecycle's events. Magenta, as every Events tab
/// in this repository is.
pub const EVENTS: Theme = Theme {
    title: "Events",
    shade: palette::THEME[5],
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 6] = [
    OVERVIEW,
    NODE_POOLS,
    PENDING_PODS,
    WORKLOADS,
    CLOUD_CAPACITY,
    EVENTS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tabs_are_told_apart_by_shade_and_title() {
        let shades: HashSet<&str> = THEMED.iter().map(|t| t.shade).collect();
        assert_eq!(shades.len(), THEMED.len(), "two tabs share a shade");
        let titles: HashSet<&str> = THEMED.iter().map(|t| t.title).collect();
        assert_eq!(titles.len(), THEMED.len(), "two tabs share a title");
    }

    #[test]
    fn every_shade_comes_from_the_qualitative_palette() {
        for theme in THEMED {
            assert!(palette::THEME.contains(&theme.shade), "{}", theme.title);
        }
    }
}
