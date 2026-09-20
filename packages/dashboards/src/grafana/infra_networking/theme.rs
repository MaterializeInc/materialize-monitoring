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
//! Same scheme as the other dashboards: one qualitative colour per tab, assigned
//! in one place so no two collide. See `env_top::theme` for why the palette is
//! deliberately free of red.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Is the cluster's networking healthy, and what is it built on.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0],
};

/// Pods, Services and the routing between them.
pub const KUBERNETES: Theme = Theme {
    title: "Kubernetes",
    shade: palette::THEME[3],
};

/// The dataplane underneath, whichever one this cluster runs.
pub const CNI: Theme = Theme {
    title: "CNI",
    shade: palette::THEME[2],
};

/// The machines' own interfaces and the kernel's networking.
///
/// Takes the shade `infra-nodes` gives its Network tab: it is the same family of
/// measurements seen across the fleet rather than one node, and an operator
/// moving between the two should not be told otherwise.
pub const NODES: Theme = Theme {
    title: "Node Networking",
    shade: palette::THEME[4],
};

/// What the cloud provider put in front of the cluster.
pub const CLOUD: Theme = Theme {
    title: "Cloud Networking",
    shade: palette::THEME[6],
};

/// Which traffic is allowed, and what was stopped.
pub const SECURITY: Theme = Theme {
    title: "Security",
    shade: palette::THEME[5],
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 6] = [OVERVIEW, KUBERNETES, CNI, NODES, CLOUD, SECURITY];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tabs_are_told_apart_by_shade_and_title() {
        let shades: HashSet<&str> = THEMED.iter().map(|t| t.shade).collect();
        assert_eq!(
            shades.len(),
            THEMED.len().min(palette::THEME.len()),
            "two tabs share a shade while the palette still has a spare"
        );
        let titles: HashSet<&str> = THEMED.iter().map(|t| t.title).collect();
        assert_eq!(titles.len(), THEMED.len(), "two tabs share a title");
    }

    #[test]
    fn every_shade_comes_from_the_qualitative_palette() {
        for theme in THEMED {
            assert!(palette::THEME.contains(&theme.shade), "{}", theme.title);
            assert!(
                !palette::INCANDESCENT.contains(&theme.shade),
                "{} uses a health colour",
                theme.title
            );
        }
    }
}
