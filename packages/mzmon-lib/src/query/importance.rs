// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Metric importance: how much a metric matters, orthogonal to query
//! [`stability`](crate::query::stability).
//!
//! Where `stability` grades a *query's* API (how safe it is to change),
//! `importance` grades a *metric* (whether it is worth collecting when trimming
//! the metric set). It drives cardinality/volume reduction: pick a threshold and
//! collect everything at least that important.
//!
//! Authored as a per-file `metricImportanceHint` that is stamped onto every query
//! at load time, then rolled up to each metric **greatest-wins** — if any query
//! that references a metric is `essential`, the metric is `essential`, because
//! you cannot drop a metric an alert depends on. `metricOverrides` can then set a
//! metric's importance outright (see [`crate::query::registry`]).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::query::error::Error;

/// How important a metric is. Ordered most-important → least: `essential`
/// (powers alerts) > `recommended` (powers dashboards) > `extended` (optional /
/// experimental) > `diagnostic` (debugging and deep analysis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Importance {
    Essential,
    Recommended,
    Extended,
    Diagnostic,
}

/// Importance levels from least to most important. [`Ord`] compares by position
/// here, so `a.max(b)` is the greatest-wins roll-up and sorting descending puts
/// `essential` first.
pub const IMPORTANCE_ORDER: [Importance; 4] = [
    Importance::Diagnostic,
    Importance::Extended,
    Importance::Recommended,
    Importance::Essential,
];

impl Importance {
    /// This level's position in [`IMPORTANCE_ORDER`] (0 = least important,
    /// 3 = `essential`).
    pub fn rank(self) -> usize {
        IMPORTANCE_ORDER
            .iter()
            .position(|&i| i == self)
            .expect("every Importance variant is in IMPORTANCE_ORDER")
    }

    /// The kebab-case wire value (matches the schema enum).
    pub fn as_str(self) -> &'static str {
        match self {
            Importance::Essential => "essential",
            Importance::Recommended => "recommended",
            Importance::Extended => "extended",
            Importance::Diagnostic => "diagnostic",
        }
    }
}

/// The default hint for a registry file that omits `metricImportanceHint`. Only
/// reached on schema-invalid input (the schema requires the hint); `recommended`
/// is the neutral middle.
impl Default for Importance {
    fn default() -> Self {
        Importance::Recommended
    }
}

impl fmt::Display for Importance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Importance {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "essential" => Ok(Importance::Essential),
            "recommended" => Ok(Importance::Recommended),
            "extended" => Ok(Importance::Extended),
            "diagnostic" => Ok(Importance::Diagnostic),
            other => Err(Error::Schema {
                path: "importance".to_string(),
                message: format!("unknown importance level {other:?}"),
            }),
        }
    }
}

// Ordering by importance, so `max` performs the greatest-wins roll-up. Consistent
// with `Eq`: distinct variants have distinct ranks.
impl Ord for Importance {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl PartialOrd for Importance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn essential_is_the_most_important() {
        assert!(Importance::Essential > Importance::Recommended);
        assert!(Importance::Recommended > Importance::Extended);
        assert!(Importance::Extended > Importance::Diagnostic);
    }

    #[test]
    fn greatest_wins_picks_the_more_important() {
        assert_eq!(
            Importance::Diagnostic.max(Importance::Essential),
            Importance::Essential
        );
        assert_eq!(
            Importance::Extended.max(Importance::Recommended),
            Importance::Recommended
        );
    }

    #[test]
    fn default_is_recommended() {
        assert_eq!(Importance::default(), Importance::Recommended);
    }

    #[test]
    fn string_round_trips_and_serde_uses_kebab() {
        for i in IMPORTANCE_ORDER {
            assert_eq!(Importance::from_str(i.as_str()).unwrap(), i);
        }
        assert_eq!(
            serde_json::to_string(&Importance::Essential).unwrap(),
            "\"essential\""
        );
        assert!(Importance::from_str("critical").is_err());
    }
}
