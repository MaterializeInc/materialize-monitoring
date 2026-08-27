// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Colour palettes. Port of `dashboards.palette`.
//!
//! Chosen to be colour-blind-friendly and to survive black-and-white printing,
//! mostly from the work of Paul Tol (<https://sronpersonalpages.nl/~pault/>).
//!
//! Two kinds, and mixing them up is the usual mistake:
//!
//! * **Sequential** ([`INCANDESCENT`], [`SUNSET_DIVERGING`]) runs good -> bad and
//!   is safe to interpolate. Use it where the value has a direction: thresholds,
//!   health meters, utilisation.
//! * **Qualitative** ([`THEME`]) has no ordering. Use it to tell series apart
//!   where health is not the point, which is what [`crate::grafana::panel::Panel::shade`]
//!   takes.

/// Sequential palette from "good" to "bad", 11 steps.
///
/// <https://sronpersonalpages.nl/~pault/#fig:scheme_incandescent>
pub const INCANDESCENT: [&str; 11] = [
    "#CEFFFF", // pale cyan
    "#C6F7D6", // light grayish cyan-lime
    "#A2F49B", // soft lime
    "#BBE453", // soft green
    "#D5CE04", // strong yellow
    "#E7B503", // golden yellow
    "#F19903", // deep warm orange
    "#F6790B", // vivid orange
    "#F94902", // blood orange
    "#E40515", // vivid red
    "#AB0003", // dark red
];

/// Gray, for data that is missing rather than bad.
pub const INCANDESCENT_INVALID: &str = "#888888";

/// Diverging palette, blue -> yellow -> red, 11 steps.
///
/// <https://sronpersonalpages.nl/~pault/#fig:scheme_sunset>
pub const SUNSET_DIVERGING: [&str; 11] = [
    "#364B9A", // dark blue
    "#4A7BB7", //
    "#6EA6CD", //
    "#98CAE1", //
    "#C2E4EF", //
    "#EAECCC", // light yellow
    "#FEDA8B", // light orange
    "#FDB366", // orange
    "#F67E4B", // red-orange
    "#DD3D2D", // red
    "#A50026", // dark red
];

/// White, for missing data against the sunset palette.
pub const SUNSET_INVALID: &str = "#FFFFFF";

/// The neutral midpoint of [`SUNSET_DIVERGING`].
pub const SUNSET_NOMINAL: &str = SUNSET_DIVERGING[5];

/// The warm half of [`SUNSET_DIVERGING`], for error counts.
pub const SUNSET_ERROR: [&str; 5] = [
    SUNSET_DIVERGING[6],
    SUNSET_DIVERGING[7],
    SUNSET_DIVERGING[8],
    SUNSET_DIVERGING[9],
    SUNSET_DIVERGING[10],
];

/// Two-step palette, for a value that is simply good or bad.
pub mod binary {
    /// Cool teal.
    pub const LOW: &str = "#009E73";
    /// Alias of [`LOW`].
    pub const GOOD: &str = LOW;
    /// Warm orange.
    pub const HIGH: &str = "#D55E00";
    /// Alias of [`HIGH`].
    pub const BAD: &str = HIGH;
}

/// Three-step health palette, drawn from [`INCANDESCENT`] at indices 2, 6, 10.
pub mod tri_health {
    use super::{INCANDESCENT, INCANDESCENT_INVALID};

    pub const HEALTHY: &str = INCANDESCENT[2];
    pub const DEGRADED: &str = INCANDESCENT[6];
    pub const UNHEALTHY: &str = INCANDESCENT[10];
    /// Missing data, which is neither.
    pub const INVALID: &str = INCANDESCENT_INVALID;
}

/// Evenly-spaced 3-colour slice of [`INCANDESCENT`] (indices 2, 6, 10), the
/// source of [`tri_health`].
///
/// The Python also defines `INCANDESC_SEQUENTIAL_4` and `_6`. Both are unused
/// there, and both are misnamed — the slices are 3 and 5 colours long — so they
/// are not ported rather than carrying the confusion across.
pub const INCANDESCENT_3: [&str; 3] = [INCANDESCENT[2], INCANDESCENT[6], INCANDESCENT[10]];

/// Qualitative palette for telling series apart where health is not the point.
///
/// <https://sronpersonalpages.nl/~pault/#fig:scheme_bright>
///
/// Red is deliberately absent: it reads as a health colour.
pub const THEME: [&str; 7] = [
    "#0077BB", // blue
    "#33BBEE", // cyan
    "#009988", // teal
    "#EE7733", // orange
    "#CCBB44", // yellow
    "#EE3377", // magenta
    "#BBBBBB", // gray
];

/// Lighter qualitative palette.
///
/// <https://sronpersonalpages.nl/~pault/#fig:scheme_light>
pub const THEME_LIGHT: [&str; 7] = [
    "#77AADD", // light blue
    "#BBCC33", // pear
    "#AAAA00", // olive
    "#EEDD88", // light yellow
    "#EE8866", // orange
    "#FFAABB", // pink
    "#DDDDDD", // light gray
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_derived_slices_match_their_source() {
        assert_eq!(
            INCANDESCENT_3,
            [INCANDESCENT[2], INCANDESCENT[6], INCANDESCENT[10]]
        );
        assert_eq!(tri_health::HEALTHY, INCANDESCENT_3[0]);
        assert_eq!(tri_health::DEGRADED, INCANDESCENT_3[1]);
        assert_eq!(tri_health::UNHEALTHY, INCANDESCENT_3[2]);
        assert_eq!(SUNSET_NOMINAL, "#EAECCC");
        assert_eq!(SUNSET_ERROR[0], "#FEDA8B");
        assert_eq!(SUNSET_ERROR[4], "#A50026");
    }

    #[test]
    fn tri_health_values_match_the_python() {
        // Captured from `dashboards.palette.TriHealth`.
        assert_eq!(tri_health::HEALTHY, "#A2F49B");
        assert_eq!(tri_health::DEGRADED, "#F19903");
        assert_eq!(tri_health::UNHEALTHY, "#AB0003");
        assert_eq!(tri_health::INVALID, "#888888");
    }

    #[test]
    fn every_colour_is_a_hex_triplet() {
        let all = INCANDESCENT
            .iter()
            .chain(SUNSET_DIVERGING.iter())
            .chain(THEME.iter())
            .chain(THEME_LIGHT.iter())
            .chain([
                &INCANDESCENT_INVALID,
                &SUNSET_INVALID,
                &binary::LOW,
                &binary::HIGH,
            ]);
        for colour in all {
            assert!(
                colour.len() == 7
                    && colour.starts_with('#')
                    && colour[1..].bytes().all(|b| b.is_ascii_hexdigit()),
                "not a hex triplet: {colour}"
            );
        }
    }

    #[test]
    fn the_theme_palette_avoids_health_red() {
        // A qualitative series colour that reads as "bad" defeats the point.
        assert!(!THEME.contains(&"#CC3311"));
        for colour in THEME {
            assert!(
                !INCANDESCENT.contains(&colour),
                "{colour} is a health colour"
            );
        }
    }
}
