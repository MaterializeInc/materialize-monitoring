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

/// Whether Karpenter is keeping up, and its capacity by NodePool.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0],
};

/// Adding nodes: launches, their errors, and how long they take.
pub const PROVISIONING: Theme = Theme {
    title: "Provisioning",
    shade: palette::THEME[1],
};

/// Removing and replacing nodes, what blocks it, and AWS's interruptions.
pub const DISRUPTION: Theme = Theme {
    title: "Disruption",
    shade: palette::THEME[2],
};

/// The controller itself: errors, queues and API calls.
pub const CONTROLLER: Theme = Theme {
    title: "Controller",
    shade: palette::THEME[6],
};

/// Karpenter's events and logs. Magenta, as every Events tab in this
/// repository is.
pub const EVENTS: Theme = Theme {
    title: "Events and Logs",
    shade: palette::THEME[5],
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 5] = [OVERVIEW, PROVISIONING, DISRUPTION, CONTROLLER, EVENTS];

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
