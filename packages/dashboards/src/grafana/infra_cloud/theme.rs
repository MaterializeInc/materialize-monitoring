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

/// The metadata database instances, as their provider reports them.
pub const DATABASE: Theme = Theme {
    title: "Metadata Database",
    shade: palette::THEME[3],
};

/// The buckets, as their provider reports them.
pub const OBJECT_STORAGE: Theme = Theme {
    title: "Object Storage",
    shade: palette::THEME[4],
};

/// Whether the pulls themselves are working.
pub const COLLECTION: Theme = Theme {
    title: "Collection",
    shade: palette::THEME[6],
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 3] = [DATABASE, OBJECT_STORAGE, COLLECTION];

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
