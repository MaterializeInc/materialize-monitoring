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
//! Both scenarios run on one period, five minutes unless `period=` says
//! otherwise, and every count in them is per period. Shapes repeat once per
//! period, aligned to the wall clock, so a metric pulse and a log pulse with
//! the same numbers line up. A count below 1 means "not every period":
//! `panic=0.25` panics once every four periods.
//!
//! The shapes borrow from Grafana's TestData datasource. They exist to shape
//! load over time, not to draw convincing graphs.

use std::f64::consts::TAU;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};

/// Seconds per period unless a scenario sets `period=`.
pub const DEFAULT_PERIOD: f64 = 300.0;

/// How a rate or value moves across each period. Every shape averages to 1
/// over the period, so a shaped rate keeps its configured mean.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Flat,
    /// A Gaussian bump centred in the period.
    Bell,
    Sine,
    /// A ramp from 0 at the start of the period to 2 at its end.
    Saw,
    /// On for this many seconds at the start of each period, off for the rest.
    Pulse(f64),
    /// A trapezoid: ramp up, hold for this many seconds, ramp down.
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

impl Shape {
    /// The multiplier at phase `x` in `[0, 1)` of a period `period` seconds long.
    pub fn factor(self, x: f64, period: f64) -> f64 {
        match self {
            Shape::Flat => 1.0,
            Shape::Bell => bell_raw(x) / *BELL_MEAN,
            Shape::Sine => 1.0 + (TAU * x).sin(),
            Shape::Saw => 2.0 * x,
            Shape::Pulse(on) => {
                let on = on / period;
                if x < on { 1.0 / on } else { 0.0 }
            }
            Shape::Plateau(hold) => {
                let hold = hold / period;
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

    fn parse(s: &str) -> Result<Shape> {
        let at = s
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(s.len());
        let (kind, n) = (&s[..at], &s[at..]);
        let seconds = || -> Result<f64> {
            parse_duration(n).with_context(|| format!("{s:?} wants a duration, such as {kind}30s"))
        };
        Ok(match kind {
            "flat" | "average" if n.is_empty() => Shape::Flat,
            "bell" if n.is_empty() => Shape::Bell,
            "sine" if n.is_empty() => Shape::Sine,
            "saw" if n.is_empty() => Shape::Saw,
            "pulse" => Shape::Pulse(seconds()?),
            "plateau" => Shape::Plateau(seconds()?),
            _ => bail!(
                "unknown shape {s:?}: want average, bell, sine, saw, pulse<duration> or plateau<duration>"
            ),
        })
    }

    /// A pulse or plateau has to fit in the period it repeats in.
    fn check(self, period: f64) -> Result<()> {
        if let Shape::Pulse(n) | Shape::Plateau(n) = self
            && !(n > 0.0 && n < period)
        {
            bail!("{self:?} needs a duration inside the {period}s period");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricScenario {
    pub shape: Shape,
    /// How far the shape moves gauges and counter rates, from 0 (flat) to 1
    /// (the full shape).
    pub amp: f64,
    pub period: f64,
    /// Collections and shards added per period.
    pub collection_grow: f64,
    pub shard_grow: f64,
    /// Collections and shards replaced per period: the oldest go, new IDs come.
    pub collection_churn: f64,
    pub shard_churn: f64,
}

impl Default for MetricScenario {
    fn default() -> Self {
        MetricScenario {
            shape: Shape::Flat,
            amp: 0.5,
            period: DEFAULT_PERIOD,
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
            match k {
                "dist" => out.shape = Shape::parse(v)?,
                "amp" => {
                    out.amp = non_negative(k, v)?;
                    if out.amp > 1.0 {
                        bail!("amp must be at most 1, so counters never run backwards");
                    }
                }
                "period" => out.period = period(v)?,
                "collection-grow" => out.collection_grow = non_negative(k, v)?,
                "shard-grow" => out.shard_grow = non_negative(k, v)?,
                "collection-churn" => out.collection_churn = non_negative(k, v)?,
                "shard-churn" => out.shard_churn = non_negative(k, v)?,
                _ => bail!(
                    "unknown metric-scenario key {k:?}: want dist, amp, period, collection-grow, \
                     collection-churn, shard-grow or shard-churn"
                ),
            }
        }
        out.shape.check(out.period)?;
        Ok(out)
    }

    /// The multiplier on gauges and counter rates at phase `x` of the period.
    pub fn multiplier(&self, x: f64) -> f64 {
        1.0 + self.amp * (self.shape.factor(x, self.period) - 1.0)
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

#[derive(Clone, Debug, PartialEq)]
pub struct LogScenario {
    /// Mean lines per period at each level, indexed by `Level as usize`.
    pub rates: [f64; 5],
    pub shapes: [Shape; 5],
    pub period: f64,
    /// Whether every pod's period starts on the same wall-clock boundary, so
    /// bursts land together, rather than at a seeded offset.
    pub synced: bool,
    /// Lines per period that are not in the role's log format.
    pub non_json: f64,
    /// Secret-filter canary lines per period.
    pub eicar: f64,
    /// Panics per period.
    pub panic: f64,
    /// Exit after a panic, the way a real one ends the process.
    pub panic_exit: bool,
}

impl Default for LogScenario {
    fn default() -> Self {
        LogScenario {
            rates: [0.0, 0.0, 1500.0, 150.0, 15.0],
            shapes: [Shape::Flat; 5],
            period: DEFAULT_PERIOD,
            synced: true,
            non_json: 0.0,
            eicar: 0.0,
            panic: 0.0,
            panic_exit: false,
        }
    }
}

impl LogScenario {
    pub fn parse(s: &str) -> Result<LogScenario> {
        let mut out = LogScenario::default();
        let mut explicit_rate = false;
        let mut default_shape = None;
        let mut level_shapes: [Option<Shape>; 5] = [None; 5];
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
                level_shapes[*level as usize] = Some(Shape::parse(v)?);
                continue;
            }
            match k {
                "dist" => default_shape = Some(Shape::parse(v)?),
                "period" => out.period = period(v)?,
                "phase" => {
                    out.synced = match v {
                        "sync" => true,
                        "seeded" => false,
                        _ => bail!("phase is sync or seeded, not {v:?}"),
                    }
                }
                "non-json" => out.non_json = non_negative(k, v)?,
                "eicar" => out.eicar = non_negative(k, v)?,
                "panic" => out.panic = non_negative(k, v)?,
                "panic-exit" => {
                    out.panic_exit = v
                        .parse()
                        .with_context(|| format!("panic-exit is true or false, not {v:?}"))?
                }
                _ => bail!(
                    "unknown log-scenario key {k:?}: want trace, debug, info, warn, error, \
                     <level>-dist, dist, period, phase, non-json, eicar, panic or panic-exit"
                ),
            }
        }
        for level in Level::ALL {
            let shape = level_shapes[level as usize]
                .or(default_shape)
                .unwrap_or(Shape::Flat);
            shape.check(out.period)?;
            out.shapes[level as usize] = shape;
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

fn non_negative(k: &str, v: &str) -> Result<f64> {
    let n: f64 = v
        .parse()
        .with_context(|| format!("{k} wants a number, not {v:?}"))?;
    if !(n >= 0.0 && n.is_finite()) {
        bail!("{k} must be a non-negative number, not {v:?}");
    }
    Ok(n)
}

/// At least a second: a shorter period is below the resolution of a scrape,
/// and of the tick the log writer runs on.
fn period(v: &str) -> Result<f64> {
    let p = parse_duration(v)?;
    if p < 1.0 {
        bail!("period must be at least 1s, not {v:?}");
    }
    Ok(p)
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

    fn mean(shape: Shape, period: f64) -> f64 {
        let n = 300_000;
        (0..n)
            .map(|i| shape.factor((i as f64 + 0.5) / n as f64, period))
            .sum::<f64>()
            / n as f64
    }

    #[test]
    fn every_shape_averages_to_one() {
        for s in [
            "average",
            "bell",
            "sine",
            "saw",
            "pulse15",
            "pulse180",
            "pulse4m",
            "plateau15",
            "plateau150",
            "plateau2m",
        ] {
            let m = mean(Shape::parse(s).unwrap(), DEFAULT_PERIOD);
            assert!((m - 1.0).abs() < 1e-3, "{s}: {m}");
        }
    }

    #[test]
    fn parses_the_example_scenarios() {
        let m = MetricScenario::parse("dist=pulse180,collection-grow=200").unwrap();
        assert_eq!(m.shape, Shape::Pulse(180.0));
        assert_eq!(m.period, 300.0);
        assert_eq!(m.collection_grow, 200.0);

        let l = LogScenario::parse(
            "info=25000,warn=5000,error=2500,error-dist=bell,non-json=10,eicar=0.25,panic=0.25",
        )
        .unwrap();
        assert_eq!(l.rates, [0.0, 0.0, 25000.0, 5000.0, 2500.0]);
        assert_eq!(l.shapes[Level::Error as usize], Shape::Bell);
        assert_eq!(l.shapes[Level::Info as usize], Shape::Flat);
        assert_eq!((l.non_json, l.eicar, l.panic), (10.0, 0.25, 0.25));
    }

    #[test]
    fn pulse_and_plateau_must_fit_the_period() {
        assert!(MetricScenario::parse("dist=pulse300").is_err());
        assert!(MetricScenario::parse("dist=pulse90,period=60s").is_err());
        assert!(MetricScenario::parse("period=60s,dist=pulse30").is_ok());
        assert!(LogScenario::parse("info-dist=plateau6m").is_err());
    }

    /// A period shorter than a second would ask the integrator, and the log
    /// writer's tick, for a resolution neither has.
    #[test]
    fn rejects_sub_second_periods() {
        assert!(MetricScenario::parse("period=0.000001").is_err());
        assert!(LogScenario::parse("period=500ms").is_err());
    }

    #[test]
    fn rejects_unknown_keys_and_shapes() {
        assert!(MetricScenario::parse("dist=zigzag").is_err());
        assert!(MetricScenario::parse("dist=bell30").is_err());
        assert!(MetricScenario::parse("bogus=1").is_err());
        assert!(LogScenario::parse("fatal=3").is_err());
        assert!(LogScenario::parse("panic-period=1200s").is_err());
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("1200s").unwrap(), 1200.0);
        assert_eq!(parse_duration("20m").unwrap(), 1200.0);
        assert_eq!(parse_duration("1500ms").unwrap(), 1.5);
        assert_eq!(parse_duration("90").unwrap(), 90.0);
    }

    #[test]
    fn metric_pulse_is_high_then_low() {
        let m = MetricScenario::parse("dist=pulse180").unwrap();
        assert!(m.multiplier(0.1) > 1.0);
        assert!(m.multiplier(0.9) < 1.0);
        assert!(m.multiplier(0.9) >= 0.0);
    }
}
