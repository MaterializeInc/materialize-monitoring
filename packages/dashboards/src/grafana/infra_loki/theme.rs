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
//! in one place. Logs keeps the cyan it carries on `env-logs` and `infra-logs`,
//! because it is the same kind of content — an operator moving between the three
//! is not told otherwise. The other four are this dashboard's own.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Is the log store working, end to end.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0], // blue
};

/// What arrives, and what becomes of it.
pub const WRITES: Theme = Theme {
    title: "Writes",
    shade: palette::THEME[2], // teal
};

/// What is asked of it, and how long that takes.
pub const READS: Theme = Theme {
    title: "Reads",
    shade: palette::THEME[3], // orange
};

/// Where it all ends up, and what tidies it away.
pub const STORAGE: Theme = Theme {
    title: "Storage",
    shade: palette::THEME[4], // yellow
};

/// What Loki itself had to say about any of it.
pub const LOGS: Theme = Theme {
    title: "Logs",
    shade: palette::THEME[1], // cyan
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 5] = [OVERVIEW, WRITES, READS, STORAGE, LOGS];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_two_tabs_share_a_shade_or_a_title() {
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

    #[test]
    fn logs_keeps_the_shade_it_carries_on_the_other_logs_dashboards() {
        // Not arbitrary: `env-logs` and `infra-logs` both shade their Logs tab
        // with THEME[1], and a reader moving between them should not have to
        // work out that the cyan tab is the same kind of tab.
        assert_eq!(LOGS.shade, palette::THEME[1]);
    }
}
