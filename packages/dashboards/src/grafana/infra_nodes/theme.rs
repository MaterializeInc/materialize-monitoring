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

/// What the machine is, and whether Kubernetes is willing to use it.
pub const SUMMARY: Theme = Theme {
    title: "Summary",
    shade: palette::THEME[0],
};

/// Time on the processors.
pub const CPU: Theme = Theme {
    title: "CPU",
    shade: palette::THEME[3],
};

/// What is resident, and what has spilled.
pub const MEMORY: Theme = Theme {
    title: "Memory & Swap",
    shade: palette::THEME[2],
};

/// What is scheduled here, and what it reserved.
pub const PODS: Theme = Theme {
    title: "Pods",
    shade: palette::THEME[5],
};

/// Bytes and packets across the wire.
pub const NETWORK: Theme = Theme {
    title: "Network",
    shade: palette::THEME[4],
};

/// Disks and the filesystems on them.
pub const STORAGE: Theme = Theme {
    title: "Storage",
    shade: palette::THEME[6],
};

/// What the node itself said, and what Kubernetes said about it.
///
/// Takes the shade `env-logs` and `infra-logs` give their Logs tab, since it is
/// the same kind of content reached from a different direction.
pub const LOGS: Theme = Theme {
    title: "Logs & Events",
    shade: palette::THEME[1],
};

/// Every themed tab, in the order they appear.
///
/// Seven tabs against a seven-colour palette: this dashboard uses the whole of
/// it, and an eighth tab would have to repeat a shade.
pub const THEMED: [Theme; 7] = [SUMMARY, CPU, MEMORY, NETWORK, STORAGE, PODS, LOGS];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tabs_are_told_apart_by_shade_and_title() {
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
