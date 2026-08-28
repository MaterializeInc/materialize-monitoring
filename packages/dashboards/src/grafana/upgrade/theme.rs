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
//! Same scheme as `env_top::theme`: one qualitative colour per tab, assigned in
//! one place so no two collide and a recolour is a single edit. See that module
//! for why the palette is deliberately free of red.
//!
//! The colours do *not* have to differ from `env-top`'s. A shade says "you are
//! here" within one dashboard's tab strip, and the two dashboards are never on
//! screen together, so what has to hold is that no two tabs *here* collide.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// What the cluster and the operator reported while the upgrade ran.
pub const EVENTS: Theme = Theme {
    title: "Events",
    shade: palette::THEME[5], // magenta
};

/// The two sides of a blue/green rollout, and whether the new one has caught up.
pub const GENERATIONS: Theme = Theme {
    title: "Generations",
    shade: palette::THEME[3], // orange
};

/// The operator's reconciliation loop, as counters and histograms.
pub const RECONCILIATION: Theme = Theme {
    title: "Reconciliation",
    shade: palette::THEME[2], // teal
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 3] = [EVENTS, GENERATIONS, RECONCILIATION];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_two_tabs_share_a_shade_or_a_title() {
        let shades: HashSet<&str> = THEMED.iter().map(|t| t.shade).collect();
        assert_eq!(shades.len(), THEMED.len(), "two tabs share a shade");
        let titles: HashSet<&str> = THEMED.iter().map(|t| t.title).collect();
        assert_eq!(titles.len(), THEMED.len(), "two tabs share a title");
    }

    #[test]
    fn every_shade_comes_from_the_qualitative_palette() {
        for theme in THEMED {
            assert!(
                palette::THEME.contains(&theme.shade),
                "{} uses {} which is not a THEME colour",
                theme.title,
                theme.shade
            );
            assert!(
                !palette::INCANDESCENT.contains(&theme.shade),
                "{} uses a health colour",
                theme.title
            );
        }
    }

    #[test]
    fn the_shades_are_pinned() {
        // So a palette reorder is a visible change rather than a silent recolour.
        assert_eq!(EVENTS.shade, "#EE3377");
        assert_eq!(GENERATIONS.shade, "#EE7733");
        assert_eq!(RECONCILIATION.shade, "#009988");
    }
}
