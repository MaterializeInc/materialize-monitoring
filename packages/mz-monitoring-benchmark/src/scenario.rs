// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `--metric-scenario` and `--log-scenario`: comma-separated `key=value` lists.
//!
//! The shapes borrow from Grafana's TestData datasource. They exist to shape
//! load over time, not to draw convincing graphs.

use std::f64::consts::TAU;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};

use crate::seed::Seed;

/// How gauges and counter rates move over time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricShape {
    /// The captured values, with jitter only.
    Flat,
    /// A square wave with this period in seconds: high for the first half.
    Pulse(f64),
    Sine(f64),
    Saw(f64),
    /// A smooth random walk through a new seeded point every period.
    Walk(f64),
}

impl MetricShape {
    /// Where the wave is at unix time `t`, in `[-1, 1]`.
    ///
    /// Keyed to the wall clock rather than to the pod's start, so every pod in
    /// a fleet moves in step and the shape survives aggregation.
    pub fn wave(self, seed: Seed, t: f64) -> f64 {
        match self {
            MetricShape::Flat => 0.0,
            MetricShape::Pulse(p) => {
                if (t / p).rem_euclid(1.0) < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            MetricShape::Sine(p) => (TAU * t / p).sin(),
            MetricShape::Saw(p) => 2.0 * (t / p).rem_euclid(1.0) - 1.0,
            MetricShape::Walk(p) => {
                let k = (t / p).floor();
                let f = (t / p) - k;
                let a = 2.0 * seed.unit(k as u64) - 1.0;
                let b = 2.0 * seed.unit(k as u64 + 1) - 1.0;
                let smooth = f * f * (3.0 - 2.0 * f);
                a + (b - a) * smooth
            }
        }
    }

    fn parse(s: &str) -> Result<MetricShape> {
        let (kind, period) = split_shape(s);
        let period = match period {
            "" => 60.0,
            p => parse_duration(p)?,
        };
        if period <= 0.0 {
            bail!("shape period must be positive: {s:?}");
        }
        Ok(match kind {
            "flat" | "average" | "const" => MetricShape::Flat,
            "pulse" => MetricShape::Pulse(period),
            "sine" => MetricShape::Sine(period),
            "saw" => MetricShape::Saw(period),
            "walk" => MetricShape::Walk(period),
            _ => bail!(
                "unknown metric shape {s:?}: want flat, pulse<period>, sine<period>, saw<period> or walk<period>"
            ),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricScenario {
    pub shape: MetricShape,
    /// How far the shape swings gauges and counter rates, as a fraction.
    pub amp: f64,
    /// Collections and shards added per minute.
    pub collection_grow: f64,
    pub shard_grow: f64,
    /// Collections and shards replaced per minute: the oldest go, new IDs come.
    pub collection_churn: f64,
    pub shard_churn: f64,
}

impl Default for MetricScenario {
    fn default() -> Self {
        MetricScenario {
            shape: MetricShape::Flat,
            amp: 0.5,
            collection_grow: 0.0,
            shard_grow: 0.0,
            collection_churn: 0.0,
            shard_churn: 0.0,
        }
    }
}

impl MetricScenario {
    pub fn parse(s: &str) -> Result<MetricScenario> {
        let mut out = MetricScenario::default();
        for (k, v) in pairs(s)? {
            let rate = || -> Result<f64> { non_negative(k, v) };
            match k {
                "dist" => out.shape = MetricShape::parse(v)?,
                "amp" => {
                    out.amp = non_negative(k, v)?;
                    if out.amp > 1.0 {
                        bail!("amp must be at most 1, so counters never run backwards");
                    }
                }
                "collection-grow" => out.collection_grow = rate()?,
                "shard-grow" => out.shard_grow = rate()?,
                "collection-churn" => out.collection_churn = rate()?,
                "shard-churn" => out.shard_churn = rate()?,
                _ => bail!(
                    "unknown metric-scenario key {k:?}: want dist, amp, collection-grow, \
                     collection-churn, shard-grow or shard-churn"
                ),
            }
        }
        Ok(out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub const ALL: [Level; 5] = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }

    pub fn upper(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// How a log rate is spread across each cycle. Every shape averages to 1, so
/// the configured rate is the mean whatever the shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogShape {
    Flat,
    /// A Gaussian bump centred in the cycle.
    Bell,
    Sine,
    /// On for this fraction of the cycle, at the start, and off for the rest.
    Pulse(f64),
    /// A trapezoid: ramp up, hold for this fraction of the cycle, ramp down.
    Plateau(f64),
}

static BELL_MEAN: LazyLock<f64> = LazyLock::new(|| {
    let n = 100_000;
    (0..n)
        .map(|i| bell_raw((i as f64 + 0.5) / n as f64))
        .sum::<f64>()
        / n as f64
});

fn bell_raw(x: f64) -> f64 {
    let sigma = 1.0 / 6.0;
    (-((x - 0.5) / sigma).powi(2) / 2.0).exp()
}

impl LogShape {
    /// The rate multiplier at phase `x` in `[0, 1)` of the cycle.
    pub fn factor(self, x: f64) -> f64 {
        match self {
            LogShape::Flat => 1.0,
            LogShape::Bell => bell_raw(x) / *BELL_MEAN,
            LogShape::Sine => 1.0 + (TAU * x).sin(),
            LogShape::Pulse(on) => {
                if x < on {
                    1.0 / on
                } else {
                    0.0
                }
            }
            LogShape::Plateau(hold) => {
                let ramp = (1.0 - hold) / 2.0;
                let peak = 2.0 / (1.0 + hold);
                if x < ramp {
                    peak * x / ramp
                } else if x < ramp + hold {
                    peak
                } else {
                    peak * ((1.0 - x) / ramp).max(0.0)
                }
            }
        }
    }

    /// `pulse15` and `plateau15` count seconds of a minute, so they mean a
    /// quarter of the cycle whatever its length: 15 seconds of the default
    /// one-minute cycle, or 15 minutes of an hour.
    fn parse(s: &str) -> Result<LogShape> {
        let (kind, n) = split_shape(s);
        let fraction = || -> Result<f64> {
            let n: f64 = n
                .parse()
                .with_context(|| format!("{s:?} needs seconds of a minute, such as {kind}15"))?;
            if !(n > 0.0 && n < 60.0) {
                bail!("{s:?}: the number is seconds of a minute, between 0 and 60");
            }
            Ok(n / 60.0)
        };
        Ok(match kind {
            "flat" | "average" => LogShape::Flat,
            "bell" => LogShape::Bell,
            "sine" => LogShape::Sine,
            "pulse" => LogShape::Pulse(fraction()?),
            "plateau" => LogShape::Plateau(fraction()?),
            _ => bail!("unknown log shape {s:?}: want average, bell, sine, pulse<N> or plateau<N>"),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogScenario {
    /// Mean lines per minute at each level, indexed by `Level as usize`.
    pub rates: [f64; 5],
    pub shapes: [LogShape; 5],
    /// Seconds per cycle.
    pub cycle: f64,
    /// Whether every pod's cycle starts on the same wall-clock boundary, so
    /// bursts land together, rather than at a seeded offset.
    pub synced: bool,
    /// Lines per minute that are not in the role's log format.
    pub non_json: f64,
    /// Secret-filter canary lines per minute.
    pub eicar: f64,
    /// Seconds between panics.
    pub panic_period: Option<f64>,
    /// Exit after a panic, the way a real one ends the process.
    pub panic_exit: bool,
}

impl Default for LogScenario {
    fn default() -> Self {
        LogScenario {
            rates: [0.0, 0.0, 300.0, 30.0, 3.0],
            shapes: [LogShape::Flat; 5],
            cycle: 60.0,
            synced: true,
            non_json: 0.0,
            eicar: 0.0,
            panic_period: None,
            panic_exit: false,
        }
    }
}

impl LogScenario {
    pub fn parse(s: &str) -> Result<LogScenario> {
        let mut out = LogScenario::default();
        let mut explicit_rate = false;
        let mut default_shape = None;
        let mut level_shapes: [Option<LogShape>; 5] = [None; 5];
        for (k, v) in pairs(s)? {
            if let Some(level) = Level::ALL.iter().find(|l| l.key() == k) {
                if !explicit_rate {
                    // Naming any level replaces the default mix rather than
                    // adding to it.
                    out.rates = [0.0; 5];
                    explicit_rate = true;
                }
                out.rates[*level as usize] = non_negative(k, v)?;
                continue;
            }
            if let Some(level) = k
                .strip_suffix("-dist")
                .and_then(|l| Level::ALL.iter().find(|x| x.key() == l))
            {
                level_shapes[*level as usize] = Some(LogShape::parse(v)?);
                continue;
            }
            match k {
                "dist" => default_shape = Some(LogShape::parse(v)?),
                "cycle" => {
                    out.cycle = parse_duration(v)?;
                    if out.cycle <= 0.0 {
                        bail!("cycle must be positive");
                    }
                }
                "phase" => {
                    out.synced = match v {
                        "sync" => true,
                        "seeded" => false,
                        _ => bail!("phase is sync or seeded, not {v:?}"),
                    }
                }
                "non-json" => out.non_json = non_negative(k, v)?,
                "eicar" => out.eicar = non_negative(k, v)?,
                "panic-period" => {
                    let p = parse_duration(v)?;
                    if p <= 0.0 {
                        bail!("panic-period must be positive");
                    }
                    out.panic_period = Some(p);
                }
                "panic-exit" => {
                    out.panic_exit = v
                        .parse()
                        .with_context(|| format!("panic-exit is true or false, not {v:?}"))?
                }
                _ => bail!(
                    "unknown log-scenario key {k:?}: want trace, debug, info, warn, error, \
                     <level>-dist, dist, cycle, phase, non-json, eicar, panic-period or panic-exit"
                ),
            }
        }
        for level in Level::ALL {
            out.shapes[level as usize] = level_shapes[level as usize]
                .or(default_shape)
                .unwrap_or(LogShape::Flat);
        }
        Ok(out)
    }
}

fn pairs(s: &str) -> Result<Vec<(&str, &str)>> {
    s.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .with_context(|| format!("expected key=value, got {p:?}"))
        })
        .collect()
}

/// `pulse180` is `("pulse", "180")`.
fn split_shape(s: &str) -> (&str, &str) {
    let at = s
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(s.len());
    (&s[..at], &s[at..])
}

fn non_negative(k: &str, v: &str) -> Result<f64> {
    let n: f64 = v
        .parse()
        .with_context(|| format!("{k} wants a number, not {v:?}"))?;
    if !(n >= 0.0 && n.is_finite()) {
        bail!("{k} must be a non-negative number, not {v:?}");
    }
    Ok(n)
}

/// `90`, `90s`, `1500ms`, `20m` or `1h`, in seconds.
pub fn parse_duration(s: &str) -> Result<f64> {
    let at = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let n: f64 = s[..at]
        .parse()
        .with_context(|| format!("bad duration {s:?}"))?;
    let scale = match &s[at..] {
        "" | "s" => 1.0,
        "ms" => 0.001,
        "m" => 60.0,
        "h" => 3600.0,
        u => bail!("bad duration unit {u:?} in {s:?}: want ms, s, m or h"),
    };
    Ok(n * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mean(shape: LogShape) -> f64 {
        let n = 60_000;
        (0..n)
            .map(|i| shape.factor((i as f64 + 0.5) / n as f64))
            .sum::<f64>()
            / n as f64
    }

    #[test]
    fn every_log_shape_averages_to_one() {
        for s in [
            "average",
            "bell",
            "sine",
            "pulse15",
            "pulse30",
            "pulse45",
            "plateau15",
            "plateau30",
            "plateau45",
        ] {
            let m = mean(LogShape::parse(s).unwrap());
            assert!((m - 1.0).abs() < 1e-3, "{s}: {m}");
        }
    }

    #[test]
    fn parses_the_example_scenarios() {
        let m = MetricScenario::parse("dist=pulse180,collection-grow=200").unwrap();
        assert_eq!(m.shape, MetricShape::Pulse(180.0));
        assert_eq!(m.collection_grow, 200.0);

        let l = LogScenario::parse(
            "info=5000,warn=1000,error=500,error-dist=bell,non-json=2,eicar=3,panic-period=1200s",
        )
        .unwrap();
        assert_eq!(l.rates, [0.0, 0.0, 5000.0, 1000.0, 500.0]);
        assert_eq!(l.shapes[Level::Error as usize], LogShape::Bell);
        assert_eq!(l.shapes[Level::Info as usize], LogShape::Flat);
        assert_eq!(l.panic_period, Some(1200.0));
        assert_eq!((l.non_json, l.eicar), (2.0, 3.0));
    }

    #[test]
    fn rejects_unknown_keys_and_shapes() {
        assert!(MetricScenario::parse("dist=zigzag").is_err());
        assert!(MetricScenario::parse("bogus=1").is_err());
        assert!(LogScenario::parse("info-dist=pulse90").is_err());
        assert!(LogScenario::parse("fatal=3").is_err());
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("1200s").unwrap(), 1200.0);
        assert_eq!(parse_duration("20m").unwrap(), 1200.0);
        assert_eq!(parse_duration("1500ms").unwrap(), 1.5);
        assert_eq!(parse_duration("90").unwrap(), 90.0);
    }

    #[test]
    fn pulse_wave_is_square() {
        let p = MetricShape::Pulse(180.0);
        let s = Seed::new("x");
        assert_eq!(p.wave(s, 0.0), 1.0);
        assert_eq!(p.wave(s, 89.0), 1.0);
        assert_eq!(p.wave(s, 91.0), -1.0);
        assert_eq!(p.wave(s, 181.0), 1.0);
    }
}
