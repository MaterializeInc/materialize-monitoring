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

/// Is object storage working for this environment right now.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0],
};

/// What the environment asks of object storage, and how long it takes.
pub const OPERATIONS: Theme = Theme {
    title: "Operations",
    shade: palette::THEME[1],
};

/// The background work that keeps reads fast and storage small.
pub const COMPACTION: Theme = Theme {
    title: "Compaction",
    shade: palette::THEME[2],
};

/// What the environment's data in object storage is for.
pub const STORAGE: Theme = Theme {
    title: "Storage",
    shade: palette::THEME[4],
};

/// What an on-premise object store reports about itself.
pub const OBJECT_STORE: Theme = Theme {
    title: "Object Store Internals",
    shade: palette::THEME[5],
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 5] = [OVERVIEW, OPERATIONS, COMPACTION, STORAGE, OBJECT_STORE];

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
