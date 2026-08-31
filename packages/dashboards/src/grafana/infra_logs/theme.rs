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
//! Same scheme as the `env-*` dashboards: one qualitative colour per tab,
//! assigned in one place. Logs and Events keep the shades they carry on
//! `env-logs`, since they are the same kind of content read by a different
//! audience; Nodes is the one this dashboard adds.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// What the platform's own workloads said.
pub const LOGS: Theme = Theme {
    title: "Logs",
    shade: palette::THEME[1], // cyan
};

/// What systemd said, on the nodes underneath them.
pub const NODES: Theme = Theme {
    title: "Nodes",
    shade: palette::THEME[2], // teal
};

/// What Kubernetes said about any of it.
pub const EVENTS: Theme = Theme {
    title: "Events",
    shade: palette::THEME[5], // magenta
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 3] = [LOGS, NODES, EVENTS];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_two_tabs_share_a_shade_or_a_title() {
        // Distinctness is only achievable while the palette can afford it: a
        // dashboard with more tabs than THEME has colours has to repeat, and that
        // is the palette's limit rather than this module's mistake.
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
