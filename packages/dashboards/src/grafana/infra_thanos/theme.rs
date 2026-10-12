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
//! Same scheme as everywhere else here: one qualitative colour per tab, assigned
//! in one place. Every tab `infra-loki` also has keeps the colour it carries
//! there, so a reader moving between the two meta-monitoring dashboards sees
//! the write path, the read path and storage in the same colours.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Is the metrics store working, and is anything about it alerting.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0], // blue
};

/// Remote writes into the receivers, and what the receivers do with them.
pub const WRITES: Theme = Theme {
    title: "Writes",
    shade: palette::THEME[2], // teal
};

/// Queries through Thanos Query, and the stores that answer them.
pub const READS: Theme = Theme {
    title: "Reads",
    shade: palette::THEME[3], // orange
};

/// Rule evaluation, notifications, and the results written back.
pub const RULER: Theme = Theme {
    title: "Ruler",
    shade: palette::THEME[5], // magenta
};

/// The compactor and object storage.
pub const STORAGE: Theme = Theme {
    title: "Storage",
    shade: palette::THEME[4], // yellow
};

/// Whether the processes have room to do the work.
pub const RESOURCES: Theme = Theme {
    title: "Resources",
    shade: palette::THEME[6], // gray
};

/// What Thanos itself had to say about any of it.
pub const LOGS: Theme = Theme {
    title: "Logs",
    shade: palette::THEME[1], // cyan
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 7] = [OVERVIEW, WRITES, READS, RULER, STORAGE, RESOURCES, LOGS];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::infra_loki::theme as loki;
    use std::collections::HashSet;

    #[test]
    fn every_tab_has_its_own_shade_and_title() {
        let shades: HashSet<&str> = THEMED.iter().map(|t| t.shade).collect();
        assert_eq!(shades.len(), THEMED.len(), "two tabs share a shade");
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

    #[test]
    fn the_tabs_infra_loki_shares_keep_its_shades() {
        assert_eq!(OVERVIEW.shade, loki::OVERVIEW.shade);
        assert_eq!(WRITES.shade, loki::WRITES.shade);
        assert_eq!(READS.shade, loki::READS.shade);
        assert_eq!(STORAGE.shade, loki::STORAGE.shade);
        assert_eq!(LOGS.shade, loki::LOGS.shade);
    }
}
