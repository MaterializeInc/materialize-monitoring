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
//! in one place so no two collide.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Can users reach the environment, and how is the front door holding up.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0],
};

/// balancerd, which every SQL and HTTPS connection passes through.
pub const BALANCER: Theme = Theme {
    title: "Balancer",
    shade: palette::THEME[1],
};

/// The console's nginx, which serves the web app and forwards its API calls.
pub const CONSOLE: Theme = Theme {
    title: "Console",
    shade: palette::THEME[2],
};

/// environmentd's SQL and HTTP APIs, which both of the above forward to.
pub const API: Theme = Theme {
    title: "SQL and HTTP API",
    shade: palette::THEME[3],
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 4] = [OVERVIEW, BALANCER, CONSOLE, API];

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
