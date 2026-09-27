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
//! in one place. Logs keeps the cyan it carries on every other logs tab, Events
//! the magenta it carries on `env-logs`, and Overview the blue `infra-loki` gives
//! its own, so the dashboards read alike.
//!
//! Eight tabs against a seven-colour palette means one pair shares. Components
//! and Resources are that pair: both describe the collectors' internals, and
//! neither shades a single panel, so the shared colour is never on screen.

use mzmon_lib::grafana::palette;

/// A tab: its title, and the colour its panels shade themselves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The tab title, which is also how panel descriptions cross-reference it.
    pub title: &'static str,
    /// Hex colour from [`palette::THEME`].
    pub shade: &'static str,
}

/// Is collection working, and is this dashboard even hearing from it.
pub const OVERVIEW: Theme = Theme {
    title: "Overview",
    shade: palette::THEME[0], // blue
};

/// A log line's path: node, agent, gateway, store.
pub const LOG_PIPELINE: Theme = Theme {
    title: "Log Pipeline",
    shade: palette::THEME[2], // teal
};

/// A sample's path: target, gateway, store.
pub const METRIC_PIPELINE: Theme = Theme {
    title: "Metric Pipeline",
    shade: palette::THEME[3], // orange
};

/// What pushes into the gateway: log push, remote write, and OTLP.
pub const INGEST: Theme = Theme {
    title: "Ingest",
    shade: palette::THEME[6], // gray
};

/// The controller running each pipeline, and the cluster the gateways form.
pub const COMPONENTS: Theme = Theme {
    title: "Components",
    shade: palette::THEME[4], // yellow
};

/// Whether the collectors have room to do the work.
///
/// Shares Components' shade; see the module docs.
pub const RESOURCES: Theme = Theme {
    title: "Resources",
    shade: COMPONENTS.shade,
};

/// What Kubernetes reported about the collectors.
pub const EVENTS: Theme = Theme {
    title: "Events",
    shade: palette::THEME[5], // magenta
};

/// What the collectors themselves had to say about any of it.
pub const LOGS: Theme = Theme {
    title: "Logs",
    shade: palette::THEME[1], // cyan
};

/// Every themed tab, in the order they appear.
pub const THEMED: [Theme; 8] = [
    OVERVIEW,
    LOG_PIPELINE,
    METRIC_PIPELINE,
    INGEST,
    COMPONENTS,
    RESOURCES,
    EVENTS,
    LOGS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn only_the_declared_pair_shares_a_shade() {
        // Every colour in the palette is used, and the one repeat is the pair
        // the module docs name.
        let shades: HashSet<&str> = THEMED.iter().map(|t| t.shade).collect();
        assert_eq!(
            shades.len(),
            palette::THEME.len(),
            "a palette colour is unused"
        );
        for (i, a) in THEMED.iter().enumerate() {
            for b in &THEMED[i + 1..] {
                if a.shade == b.shade {
                    assert_eq!((a.title, b.title), (COMPONENTS.title, RESOURCES.title));
                }
            }
        }
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
    fn the_shared_tabs_keep_the_shades_they_carry_elsewhere() {
        // A reader moving between the two meta-monitoring dashboards, or from
        // any logs dashboard, should not have to work out that the cyan tab is
        // the same kind of tab.
        assert_eq!(LOGS.shade, palette::THEME[1]);
        assert_eq!(EVENTS.shade, crate::grafana::env_logs::theme::EVENTS.shade);
        assert_eq!(
            OVERVIEW.shade,
            crate::grafana::infra_loki::theme::OVERVIEW.shade
        );
    }
}
