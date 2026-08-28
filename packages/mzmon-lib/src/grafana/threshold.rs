// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Threshold ladders and value mappings. Port of `dashboards.threshold`.
//!
//! # The base step
//!
//! Grafana's first threshold step **is** the base: it colours everything below
//! the second step, and its own value is ignored. The schema says so directly —
//! `Threshold.value` is nullable, documented as *"Value null means -Infinity"* —
//! and a load-and-save round trip confirms it, Grafana rewriting whatever first
//! value it is given to `0`.
//!
//! The Python does not model that. Only `health_thresholds` supplies a base (as
//! `-2147483647`, a hack for a nullable field that does not need one); the other
//! four generators emit their first real threshold as step zero, so Grafana
//! silently promotes it to the base and its colour bleeds down over everything
//! beneath it. On the baseline's error column, authored as `1 -> light orange` to
//! make "non-zero jump out visually", that means **zero errors is coloured as
//! errors**.
//!
//! So every ladder here emits an explicit base with `value: None`. Where the base
//! colour repeats the first band, rendering is unchanged and the step merely
//! becomes honest; where it does not — [`errors`] and [`utilization`] — the region
//! below the first threshold changes colour, which is the bug being fixed.
//! [`Ladder::base`] overrides it.
//!
//! Grafana normalises `None` to `0` on save, so a UI round trip shows one changed
//! field per ladder. That is cosmetic: `0` is a real boundary for a metric that
//! can go negative, whereas `None` is unambiguous, so authoring `None` is worth
//! the diff.

use crate::grafana::generated::dashboardv2;
use crate::grafana::palette;

/// Thresholds under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct Ladder {
    mode: dashboardv2::ThresholdsMode,
    base: String,
    steps: Vec<(f64, String)>,
}

impl Ladder {
    /// A ladder with an explicit base colour and no steps.
    pub fn new(base: impl Into<String>) -> Self {
        Ladder {
            mode: dashboardv2::ThresholdsMode::Absolute,
            base: base.into(),
            steps: Vec::new(),
        }
    }

    /// Interpret step values as percentages of the field's min..max rather than
    /// absolute values.
    pub fn percentage(mut self) -> Self {
        self.mode = dashboardv2::ThresholdsMode::Percentage;
        self
    }

    /// Replace the base colour — what everything below the first step shows.
    pub fn base(mut self, colour: impl Into<String>) -> Self {
        self.base = colour.into();
        self
    }

    /// Add a step: `value` and above take `colour`, until the next step.
    pub fn step(mut self, value: f64, colour: impl Into<String>) -> Self {
        self.steps.push((value, colour.into()));
        self
    }

    /// Finish the ladder.
    pub fn build(self) -> dashboardv2::ThresholdsConfig {
        let mut steps = Vec::with_capacity(self.steps.len() + 1);
        // The base, explicitly. `None` is -Infinity per the schema.
        steps.push(dashboardv2::Threshold {
            value: None,
            color: self.base,
        });
        for (value, color) in self.steps {
            steps.push(dashboardv2::Threshold {
                value: Some(value),
                color,
            });
        }
        dashboardv2::ThresholdsConfig {
            mode: self.mode,
            steps,
        }
    }
}

/// Tri-state health: unhealthy below `min_degraded`, degraded up to
/// `min_healthy`, healthy above.
///
/// Setting `min_degraded == min_healthy` collapses this to healthy/unhealthy.
pub fn health(min_degraded: f64, min_healthy: f64) -> Ladder {
    Ladder::new(palette::tri_health::UNHEALTHY)
        .step(min_degraded, palette::tri_health::DEGRADED)
        .step(min_healthy, palette::tri_health::HEALTHY)
}

/// The default health ladder: unhealthy below 80, healthy at 100.
pub fn health_default() -> Ladder {
    health(80.0, 100.0)
}

/// Utilisation, where high is bad: the incandescent palette mapped across
/// `0..max_value`, with steps every `step` from `min_value`.
///
/// The colour for a step is picked by its position in the *full* range, not
/// within `min_value..max_value`, so a ladder starting at 80% of the range opens
/// on that palette entry rather than on the palette's first colour.
///
/// Unlike the Python, everything below `min_value` takes the palette's low colour
/// rather than inheriting the `min_value` band.
///
/// # Panics
///
/// If `step` is not a positive finite number. The loop below advances by `step`,
/// so a zero or negative one never reaches `max_value` and spins forever. A
/// dashboard is built at CI time from literal arguments, so failing loudly at the
/// call site beats returning a `Result` every caller would unwrap.
pub fn utilization(min_value: f64, max_value: f64, step: f64) -> Ladder {
    assert!(
        step.is_finite() && step > 0.0,
        "utilization step must be a positive finite number, got {step}"
    );
    assert!(
        min_value.is_finite() && max_value.is_finite(),
        "utilization bounds must be finite, got {min_value}..{max_value}"
    );
    let total = palette::INCANDESCENT.len();
    let mut ladder = Ladder::new(palette::INCANDESCENT[0]);
    let mut value = min_value;
    while value < max_value {
        let index = ((total as f64) * value / max_value) as usize;
        ladder = ladder.step(value, palette::INCANDESCENT[index.min(total - 1)]);
        value += step;
    }
    ladder.step(max_value, palette::INCANDESCENT[total - 1])
}

/// The default utilisation ladder: 80..100 in steps of 10, as percentages.
pub fn utilization_default() -> Ladder {
    utilization(80.0, 100.0, 10.0).percentage()
}

/// Error counts, warming from `min_errors` to `max_errors`.
///
/// The five sunset-error colours are spread inclusively, so the first band opens
/// at `min_errors` and the last at `max_errors` — which is what the Python's own
/// docstring promises ("how many errors for the highest color") and what its
/// arithmetic did not deliver. It divided the range by the colour *count* rather
/// than by the number of gaps, so the top band opened at `min + 4/5 * (max - min)`
/// and `max_errors` was never reached: `errors(1, 100)` topped out at 80.2.
///
/// The base is the healthy colour, not the first error colour. See the module
/// docs: in the Python, a count below `min_errors` — zero errors — renders in the
/// first error colour.
pub fn errors(min_errors: f64, max_errors: f64) -> Ladder {
    let mut ladder = Ladder::new(palette::tri_health::HEALTHY);
    for (value, colour) in spread(min_errors, max_errors, &palette::SUNSET_ERROR) {
        ladder = ladder.step(value, colour);
    }
    ladder
}

/// The default error ladder: 1 through 100.
pub fn errors_default() -> Ladder {
    errors(1.0, 100.0)
}

/// Load average, cool to warm from `min_load` to `max_load`.
///
/// Spread inclusively, like [`errors`], so `load(0.0, 1.0)` steps in clean tenths
/// (`0.0, 0.1, … 1.0`) rather than the Python's elevenths that stopped at
/// `0.909`.
pub fn load(min_load: f64, max_load: f64) -> Ladder {
    let mut ladder = Ladder::new(palette::INCANDESCENT[0]);
    for (value, colour) in spread(min_load, max_load, &palette::INCANDESCENT) {
        ladder = ladder.step(value, colour);
    }
    ladder
}

/// Spread `colours` evenly and inclusively over `min..=max`.
///
/// Dividing by the number of *gaps* rather than by the colour count is what puts
/// the last colour at `max`. With a single colour there are no gaps, so it sits at
/// `min`.
fn spread(min: f64, max: f64, colours: &[&'static str]) -> Vec<(f64, &'static str)> {
    let gaps = colours.len().saturating_sub(1);
    let step = if gaps == 0 {
        0.0
    } else {
        (max - min) / gaps as f64
    };
    colours
        .iter()
        .enumerate()
        .map(|(index, colour)| (min + step * index as f64, *colour))
        .collect()
}

/// The default load ladder: 0.0 through 1.0.
pub fn load_default() -> Ladder {
    load(0.0, 1.0)
}

/// Increasing "how long has this been stable" time, on a geometric ladder.
///
/// `stable_seconds` is the duration at which the ladder tops out. Because
/// stability improves exponentially rather than linearly, the steps are the
/// geometric sequence `f, f^2, … f^n` where `f = stable^(1/n)` and `n` is the
/// palette length — a linear ladder would spend every colour on the last few
/// percent of the range.
///
/// `high_is_bad` reverses the palette. The default (`false`) is for durations
/// where *longer is better*, so short durations get the alarming end.
pub fn stability(stable_seconds: f64, high_is_bad: bool) -> Ladder {
    let mut colours: Vec<&str> = palette::INCANDESCENT.to_vec();
    if !high_is_bad {
        colours.reverse();
    }
    let steps = colours.len();
    let factor = stable_seconds.powf(1.0 / steps as f64);

    // Base repeats the first band: a duration below the first step is at least as
    // bad as the first step, so extending that colour down is what was already
    // being rendered -- now stated rather than inferred by Grafana.
    let mut ladder = Ladder::new(colours[0]);
    let mut value = factor;
    for colour in colours {
        // Truncated to whole seconds, matching the Python's `int(value)`.
        ladder = ladder.step(value.trunc(), colour);
        value *= factor;
    }
    ladder
}

/// [`stability`] from a number of days.
pub fn stability_days(days: f64, high_is_bad: bool) -> Ladder {
    stability(days * 24.0 * 3600.0, high_is_bad)
}

/// Tri-state health as *value mappings* rather than thresholds.
///
/// Thresholds colour a number; mappings replace it with a word. Use both on a
/// stat panel that should read "Healthy" rather than "100".
///
/// Unlike thresholds, `RangeMap` bounds are not nullable, so the outer bounds use
/// a large finite sentinel — the one place the Python's `_ALMOST_INFINITY` hack is
/// actually necessary.
pub fn health_mapping(min_degraded: f64, min_healthy: f64) -> Vec<dashboardv2::ValueMapping> {
    /// `0x7FFFFFFF`. JSON has no infinity and `RangeMap.from`/`to` are not
    /// nullable, so this stands in for one.
    const ALMOST_INFINITY: f64 = 2_147_483_647.0;

    fn range(
        from: f64,
        to: f64,
        text: &str,
        colour: &str,
        index: i64,
    ) -> dashboardv2::ValueMapping {
        dashboardv2::ValueMapping::RangeMap(dashboardv2::RangeMap {
            type_: dashboardv2::MappingType::Range,
            options: dashboardv2::RangeMapOptions {
                from: Some(from),
                to: Some(to),
                result: dashboardv2::ValueMappingResult {
                    text: Some(text.to_string()),
                    color: Some(colour.to_string()),
                    index: Some(index),
                    icon: None,
                },
            },
        })
    }

    vec![
        range(
            min_healthy,
            ALMOST_INFINITY,
            "Healthy",
            palette::tri_health::HEALTHY,
            1,
        ),
        range(
            min_degraded,
            min_healthy,
            "Degraded",
            palette::tri_health::DEGRADED,
            2,
        ),
        range(
            -ALMOST_INFINITY,
            min_degraded,
            "Unhealthy",
            palette::tri_health::UNHEALTHY,
            3,
        ),
        dashboardv2::ValueMapping::SpecialValueMap(dashboardv2::SpecialValueMap {
            type_: dashboardv2::MappingType::Special,
            options: dashboardv2::SpecialValueMapOptions {
                match_: dashboardv2::SpecialValueMatch::NullNan,
                result: dashboardv2::ValueMappingResult {
                    text: Some("Missing Data".to_string()),
                    color: Some(palette::tri_health::INVALID.to_string()),
                    index: Some(4),
                    icon: None,
                },
            },
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(value, colour)` per step, for comparing against captured Python output.
    fn steps(ladder: Ladder) -> Vec<(Option<f64>, String)> {
        ladder
            .build()
            .steps
            .into_iter()
            .map(|s| (s.value, s.color))
            .collect()
    }

    fn values(ladder: Ladder) -> Vec<Option<f64>> {
        steps(ladder).into_iter().map(|(v, _)| v).collect()
    }

    fn colours(ladder: Ladder) -> Vec<String> {
        steps(ladder).into_iter().map(|(_, c)| c).collect()
    }

    #[test]
    fn every_ladder_opens_with_an_explicit_base() {
        // The whole point of the module: step zero is the base, and it says so.
        for ladder in [
            health_default(),
            utilization_default(),
            errors_default(),
            load_default(),
            stability_days(2.0, false),
        ] {
            let built = ladder.build();
            assert_eq!(
                built.steps[0].value, None,
                "first step must be the base (null = -Infinity)"
            );
            assert!(built.steps.len() > 1, "a base alone is not a ladder");
        }
    }

    #[test]
    fn health_matches_the_python_below_the_base() {
        // Python: [(-2147483647, UNHEALTHY), (80, DEGRADED), (100, HEALTHY)].
        // Ours replaces the sentinel with a real base; the rest is identical.
        assert_eq!(
            steps(health_default()),
            vec![
                (None, palette::tri_health::UNHEALTHY.to_string()),
                (Some(80.0), palette::tri_health::DEGRADED.to_string()),
                (Some(100.0), palette::tri_health::HEALTHY.to_string()),
            ]
        );
    }

    #[test]
    fn collapsing_the_health_bounds_drops_the_degraded_band() {
        let built = health(100.0, 100.0).build();
        // Two steps at the same value: Grafana takes the later, so degraded is
        // unreachable -- healthy/unhealthy only, as the Python documents.
        assert_eq!(built.steps.len(), 3);
        assert_eq!(built.steps[1].value, built.steps[2].value);
    }

    #[test]
    #[should_panic(expected = "positive finite")]
    fn a_zero_utilization_step_panics_rather_than_spinning() {
        // `while value < max { value += step }` never terminates on a zero step.
        utilization(80.0, 100.0, 0.0);
    }

    #[test]
    #[should_panic(expected = "positive finite")]
    fn a_negative_utilization_step_panics() {
        utilization(80.0, 100.0, -10.0);
    }

    #[test]
    fn utilization_matches_the_python_step_values() {
        // Python THRESHOLD_80_10: [(80, #F94902), (90, #E40515), (100, #AB0003)]
        // in percentage mode.
        let built = utilization_default().build();
        assert_eq!(built.mode, dashboardv2::ThresholdsMode::Percentage);
        assert_eq!(
            values(utilization_default()),
            vec![None, Some(80.0), Some(90.0), Some(100.0)]
        );
        assert_eq!(
            colours(utilization_default()),
            vec![
                palette::INCANDESCENT[0].to_string(), // the added base
                "#F94902".to_string(),
                "#E40515".to_string(),
                "#AB0003".to_string(),
            ]
        );
    }

    #[test]
    fn errors_spans_min_to_max_inclusively() {
        // The Python divided by the colour count rather than the gap count, so it
        // produced [1, 20.8, 40.6, 60.4, 80.2] and never reached max_errors -- in
        // contradiction of its own docstring, "how many errors for the highest
        // color". Spread over the four gaps instead.
        assert_eq!(
            values(errors_default())[1..]
                .iter()
                .map(|v| v.unwrap())
                .collect::<Vec<_>>(),
            vec![1.0, 25.75, 50.5, 75.25, 100.0]
        );
        // And the base fix: zero errors is no longer coloured as errors.
        assert_eq!(colours(errors_default())[0], palette::tri_health::HEALTHY);
        assert_ne!(colours(errors_default())[0], "#FEDA8B");
    }

    #[test]
    fn the_worst_error_colour_opens_exactly_at_max_errors() {
        let steps = steps(errors(1.0, 10.0));
        let (value, colour) = steps.last().unwrap().clone();
        assert_eq!(value, Some(10.0));
        assert_eq!(colour, *palette::SUNSET_ERROR.last().unwrap());
    }

    #[test]
    fn a_degenerate_error_range_puts_every_step_at_one_value() {
        // `errors(1, 1)` has no range to spread over. The baseline's
        // `sources-errors` panel relies on this: every colour lands on 1, so only
        // the last is reachable and any error shows the worst colour.
        let numbers: Vec<f64> = values(errors(1.0, 1.0))[1..]
            .iter()
            .map(|v| v.unwrap())
            .collect();
        assert_eq!(numbers, vec![1.0; palette::SUNSET_ERROR.len()]);
    }

    #[test]
    fn load_steps_in_clean_tenths() {
        // Eleven colours over ten gaps. The Python divided by eleven, giving
        // elevenths that stopped at 0.909 -- for a 0..1 load average, tenths are
        // both correct and legible.
        let numbers: Vec<f64> = values(load_default())[1..]
            .iter()
            .map(|v| v.unwrap())
            .collect();
        assert_eq!(numbers.len(), palette::INCANDESCENT.len());
        for (index, value) in numbers.iter().enumerate() {
            let want = index as f64 / 10.0;
            assert!((value - want).abs() < 1e-9, "{value} != {want}");
        }
        assert_eq!(*numbers.last().unwrap(), 1.0);
        // First real step is 0.0 with the palette's low colour, so the added base
        // repeats it and rendering below 0 is unchanged.
        assert_eq!(colours(load_default())[0], palette::INCANDESCENT[0]);
        assert_eq!(colours(load_default())[1], palette::INCANDESCENT[0]);
    }

    #[test]
    fn stability_matches_the_python_step_values() {
        // Python time_stable_thresholds(days=2):
        // [2, 8, 26, 80, 240, 719, 2152, 6443, 19286, 57730, 172800]
        assert_eq!(
            values(stability_days(2.0, false))[1..]
                .iter()
                .map(|v| v.unwrap())
                .collect::<Vec<_>>(),
            vec![
                2.0, 8.0, 26.0, 80.0, 240.0, 719.0, 2152.0, 6443.0, 19286.0, 57730.0, 172800.0
            ]
        );
    }

    #[test]
    fn stability_reverses_the_palette_when_low_is_bad() {
        // Default: longer is better, so the short end is alarming.
        let low_bad = colours(stability_days(2.0, false));
        assert_eq!(low_bad[1], palette::INCANDESCENT[10]); // dark red at 2s
        assert_eq!(low_bad.last().unwrap(), palette::INCANDESCENT[0]);

        let high_bad = colours(stability_days(2.0, true));
        assert_eq!(high_bad[1], palette::INCANDESCENT[0]);
        assert_eq!(high_bad.last().unwrap(), palette::INCANDESCENT[10]);

        // Either way the base repeats the first band, so this ladder renders
        // exactly as the Python's did.
        assert_eq!(low_bad[0], low_bad[1]);
        assert_eq!(high_bad[0], high_bad[1]);
    }

    #[test]
    fn a_hand_built_ladder_works() {
        let built = Ladder::new("#000000")
            .step(10.0, "#111111")
            .step(20.0, "#222222")
            .percentage()
            .build();
        assert_eq!(built.mode, dashboardv2::ThresholdsMode::Percentage);
        assert_eq!(built.steps.len(), 3);
        assert_eq!(built.steps[0].value, None);
    }

    #[test]
    fn the_base_can_be_overridden() {
        let built = errors_default().base("#123456").build();
        assert_eq!(built.steps[0].color, "#123456");
    }

    #[test]
    fn health_mapping_covers_every_band_and_missing_data() {
        let mappings = health_mapping(80.0, 100.0);
        assert_eq!(mappings.len(), 4);
        let json = serde_json::to_value(&mappings).expect("serialize");
        let text: Vec<&str> = json
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["options"]["result"]["text"].as_str().unwrap())
            .collect();
        assert_eq!(
            text,
            vec!["Healthy", "Degraded", "Unhealthy", "Missing Data"]
        );
        // Ranges must be contiguous: healthy starts where degraded ends.
        assert_eq!(json[0]["options"]["from"], 100.0);
        assert_eq!(json[1]["options"]["to"], 100.0);
        assert_eq!(json[1]["options"]["from"], 80.0);
        assert_eq!(json[2]["options"]["to"], 80.0);
    }
}
