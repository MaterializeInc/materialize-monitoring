// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Query stability levels and their two orderings.
//!
//! Ported from `py_mzmon_lib.registry.queries.Stability`. The Python type is a
//! `StrEnum` whose comparison operators are overridden to compare *maturity*
//! (via [`MATURITY_ORDER`]) rather than the string value; we reproduce that by
//! implementing [`Ord`]/[`PartialOrd`] over the maturity index. Equality stays
//! by-variant (consistent with the derived [`Eq`], since every variant has a
//! distinct maturity index).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::query::error::Error;

/// The stability of a query, controlling how it may be changed and whether it
/// carries user-facing documentation. See `mzmon-query.schema.yaml` for the
/// prose contract behind each level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stability {
    /// Not used anywhere; using an `unused` query is an error. No docs.
    Unused,
    /// Development / experimentation only. No docs.
    Playground,
    /// Experimental; may change or be removed without notice. Has docs.
    Experimental,
    /// Expected to be fully supported; incompatible changes are breaking.
    BestEffort,
    /// Fully supported and stable; incompatible changes are breaking. Requires
    /// test coverage.
    Canonical,
    /// Deprecated; still available but not recommended. Using it warns. Has docs.
    Deprecated,
    /// Unsupported; still available but not recommended. Using it warns. No docs.
    Unsupported,
}

/// Ordering of stability levels from least to most mature — the normal flow of
/// query development and promotion.
///
/// `>= BestEffort` deliberately includes `Deprecated` (placed just above
/// `BestEffort`) so a dashboard's minimum-maturity filter keeps deprecated
/// queries rather than needing separate `BestEffortDeprecated` /
/// `CanonicalDeprecated` levels.
pub const MATURITY_ORDER: [Stability; 7] = [
    Stability::Unused,
    Stability::Playground,
    Stability::Unsupported,
    Stability::Experimental,
    Stability::BestEffort,
    Stability::Deprecated,
    Stability::Canonical,
];

/// Ordering of stability levels when being slated for removal.
///
/// Best-effort and canonical queries must be deprecated before removal;
/// experimental queries can be removed without deprecation. After deprecation a
/// query can be removed, or marked unsupported if it lingers.
pub const DEPRECATION_ORDER: [Stability; 5] = [
    Stability::Canonical,
    Stability::BestEffort,
    Stability::Deprecated,
    Stability::Unsupported,
    Stability::Unused,
];

impl Stability {
    /// This level's index in [`MATURITY_ORDER`] (0 = least mature).
    pub fn maturity(self) -> usize {
        MATURITY_ORDER
            .iter()
            .position(|&s| s == self)
            .expect("every Stability variant is in MATURITY_ORDER")
    }

    /// This level's index in [`DEPRECATION_ORDER`], if it participates in the
    /// deprecation flow (`Playground` and `Experimental` do not).
    pub fn deprecation(self) -> Option<usize> {
        DEPRECATION_ORDER.iter().position(|&s| s == self)
    }

    /// The kebab-case wire value (matches the schema enum and the Python
    /// `StrEnum` value).
    pub fn as_str(self) -> &'static str {
        match self {
            Stability::Unused => "unused",
            Stability::Playground => "playground",
            Stability::Experimental => "experimental",
            Stability::BestEffort => "best-effort",
            Stability::Canonical => "canonical",
            Stability::Deprecated => "deprecated",
            Stability::Unsupported => "unsupported",
        }
    }
}

impl fmt::Display for Stability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Stability {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unused" => Ok(Stability::Unused),
            "playground" => Ok(Stability::Playground),
            "experimental" => Ok(Stability::Experimental),
            "best-effort" => Ok(Stability::BestEffort),
            "canonical" => Ok(Stability::Canonical),
            "deprecated" => Ok(Stability::Deprecated),
            "unsupported" => Ok(Stability::Unsupported),
            other => Err(Error::Schema {
                path: "stability".to_string(),
                message: format!("unknown stability level {other:?}"),
            }),
        }
    }
}

// Maturity comparison, matching the Python operator overrides. Consistent with
// `Eq`: distinct variants have distinct maturity indices, so `cmp == Equal`
// exactly when the variants are equal.
impl Ord for Stability {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.maturity().cmp(&other.maturity())
    }
}

impl PartialOrd for Stability {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maturity_order_is_total_and_covers_all_variants() {
        // Every variant appears exactly once.
        for s in MATURITY_ORDER {
            assert_eq!(MATURITY_ORDER.iter().filter(|&&x| x == s).count(), 1);
        }
        assert_eq!(MATURITY_ORDER.len(), 7);
    }

    #[test]
    fn maturity_comparisons_match_python_semantics() {
        // Canonical is the most mature.
        assert!(Stability::Canonical > Stability::Deprecated);
        assert!(Stability::Canonical > Stability::BestEffort);
        // Deprecated sits just above BestEffort so `>= BestEffort` keeps it.
        assert!(Stability::Deprecated > Stability::BestEffort);
        assert!(Stability::BestEffort > Stability::Experimental);
        assert!(Stability::Experimental > Stability::Unsupported);
        assert!(Stability::Unsupported > Stability::Playground);
        assert!(Stability::Playground > Stability::Unused);
    }

    #[test]
    fn max_by_maturity_picks_the_more_mature_level() {
        // The docgen stability merge keeps the more mature of two levels.
        assert_eq!(
            Stability::Experimental.max(Stability::BestEffort),
            Stability::BestEffort
        );
        assert_eq!(
            Stability::Canonical.max(Stability::Deprecated),
            Stability::Canonical
        );
    }

    #[test]
    fn deprecation_order_excludes_dev_levels() {
        assert!(Stability::Playground.deprecation().is_none());
        assert!(Stability::Experimental.deprecation().is_none());
        assert_eq!(Stability::Canonical.deprecation(), Some(0));
    }

    #[test]
    fn string_round_trips() {
        for s in MATURITY_ORDER {
            assert_eq!(Stability::from_str(s.as_str()).unwrap(), s);
        }
        assert!(Stability::from_str("bogus").is_err());
    }

    #[test]
    fn serde_uses_kebab_values() {
        let json = serde_json::to_string(&Stability::BestEffort).unwrap();
        assert_eq!(json, "\"best-effort\"");
        let parsed: Stability = serde_json::from_str("\"best-effort\"").unwrap();
        assert_eq!(parsed, Stability::BestEffort);
    }
}
