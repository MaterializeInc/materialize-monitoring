// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Expands a role's fixtures into scrapes.
//!
//! # Expansion
//!
//! A series whose labels hold placeholders is *driven* by the most specific
//! entity among them, in the order shard, collection, replica, cluster. It is
//! emitted once per generated entity of that kind whose exemplar it belongs to:
//! generated entity `j` copies exemplar `j mod E`, where `E` is how many
//! exemplars of that kind the fixtures hold. The other placeholders are
//! derived from the driver: collection `j` lives on cluster `j mod C`, shard
//! `j` backs collection `j mod N`, replica `r` belongs to cluster `r / R`.
//!
//! A shard- or collection-driven series that also names a replica fans out
//! across that cluster's replicas, so `--replicas-per-cluster` multiplies the
//! per-replica series the way a real second replica does.
//!
//! # Values
//!
//! Nothing is stored per series. Every value is a pure function of the seed,
//! the series' identity and the clock:
//!
//! * a counter is `v0 + rate × s × F(t)`, where `s` is a seeded factor within
//!   the jitter and `F` is the integral of the scenario's shape since start, so
//!   it never decreases and its `rate()` follows the shape;
//! * a gauge is `v0 × (1 + amp × wave(t))`, times seeded noise within the
//!   jitter that changes each second; zeros and ones stay put, since they are
//!   usually flags;
//! * a wall-clock gauge stays its captured distance behind the clock.
//!
//! All series of one histogram or summary share `s` and the noise, so buckets
//! stay cumulative and quantiles stay ordered.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use anyhow::{Result, bail};

use crate::expo::{self, counter_like};
use crate::fixture::{Attr, EntityKind, Fixture, Part, Placeholder, Template, Value};
use crate::role::Role;
use crate::scenario::MetricScenario;
use crate::seed::{Seed, fnv1a, jitter_count};

const KINDS: usize = EntityKind::ALL.len();
const NONE: u32 = u32::MAX;
const DRIVERS: [EntityKind; 4] = [
    EntityKind::Shard,
    EntityKind::Collection,
    EntityKind::Replica,
    EntityKind::Cluster,
];

/// How many of each entity to generate. `None` means as many as the fixtures
/// hold exemplars of.
#[derive(Clone, Debug, Default)]
pub struct Sizing {
    pub shards: Option<u64>,
    pub collections: Option<u64>,
    pub clusters: Option<u64>,
    pub replicas_per_cluster: Option<u64>,
}

struct CSeries {
    name: String,
    labels: Vec<(String, Template)>,
    /// The lowest exemplar index of each kind among the placeholders, or `NONE`.
    anchors: [u32; KINDS],
    /// Which of its cluster's replicas the named replica is, for series that
    /// fan out across replicas.
    replica_ord: Option<u32>,
    value: Value,
    rate: f64,
    counter: bool,
    integral: bool,
    group: u64,
}

struct CFamily {
    header: String,
    series: Vec<CSeries>,
    statics: Vec<usize>,
    /// For each driving kind, the series of each exemplar.
    driven: Vec<(EntityKind, Vec<Vec<usize>>)>,
    fans_out: bool,
}

struct CPath {
    path: String,
    families: Vec<CFamily>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Ctx {
    shard: Option<u64>,
    collection: Option<u64>,
    cluster: Option<u64>,
    replica: Option<u64>,
}

/// What one render needs to know about the moment it renders.
struct Env {
    now: f64,
    tick: u64,
    integral: f64,
    multiplier: f64,
    shard_off: u64,
    shards: u64,
    coll_off: u64,
    colls: u64,
    clusters: u64,
    rpc: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub shards: u64,
    pub collections: u64,
    pub clusters: u64,
    pub replicas_per_cluster: u64,
}

pub struct Model {
    seed: Seed,
    jitter: f64,
    scenario: MetricScenario,
    paths: Vec<CPath>,
    exemplars: [u32; KINDS],
    replica_ords: u32,
    counts: Counts,
    start: f64,
    /// `∫(multiplier − 1)` from the start of a period to each of
    /// `DEVIATION_STEPS` evenly spaced points in it, in periods.
    deviation: Vec<f64>,
    /// The largest `F` returned so far, as `f64` bits.
    high_water: AtomicU64,
    last_size: AtomicUsize,
}

impl Model {
    pub fn new(
        role: Role,
        fixtures: &[Fixture],
        sizing: &Sizing,
        scenario: MetricScenario,
        seed: Seed,
        jitter: f64,
        start: f64,
    ) -> Result<Model> {
        let mut exemplars = [0u32; KINDS];
        let mut replica_cluster: Vec<(u32, u32)> = Vec::new();
        for f in fixtures {
            for fam in &f.families {
                for s in &fam.series {
                    let mut cluster = None;
                    let mut replica = None;
                    for ph in s.labels.iter().flat_map(|(_, t)| t.placeholders()) {
                        let e = &mut exemplars[ph.kind as usize];
                        *e = (*e).max(ph.index + 1);
                        match ph.kind {
                            EntityKind::Cluster => cluster = cluster.or(Some(ph.index)),
                            EntityKind::Replica => replica = replica.or(Some(ph.index)),
                            _ => {}
                        }
                    }
                    if let (Some(c), Some(r)) = (cluster, replica)
                        && !replica_cluster.iter().any(|(x, _)| *x == r)
                    {
                        replica_cluster.push((r, c));
                    }
                }
            }
        }
        // A replica's ordinal among its cluster's replicas. Replicas never seen
        // beside a cluster are counted against the first cluster.
        let replica_ord = |r: u32| -> u32 {
            let c = replica_cluster
                .iter()
                .find(|(x, _)| *x == r)
                .map_or(0, |(_, c)| *c);
            (0..r)
                .filter(|x| {
                    replica_cluster
                        .iter()
                        .find(|(y, _)| y == x)
                        .map_or(0, |(_, c)| *c)
                        == c
                })
                .count() as u32
        };
        let replica_ords = (0..exemplars[EntityKind::Replica as usize])
            .map(|r| replica_ord(r) + 1)
            .max()
            .unwrap_or(1);

        let want = |flag: &str, n: Option<u64>, kind: EntityKind, noun: &str| -> Result<()> {
            if n.is_some() && exemplars[kind as usize] == 0 {
                bail!(
                    "{flag} has no effect for role {role}: its fixtures name no {noun}. \
                     It applies to {}",
                    roles_naming(kind)
                );
            }
            Ok(())
        };
        want(
            "--persist-shards",
            sizing.shards,
            EntityKind::Shard,
            "persist shard",
        )?;
        want(
            "--collection-count",
            sizing.collections,
            EntityKind::Collection,
            "user collection",
        )?;
        want(
            "--cluster-count",
            sizing.clusters,
            EntityKind::Cluster,
            "user cluster",
        )?;
        if sizing.replicas_per_cluster.is_some_and(|n| n != 1) {
            want(
                "--replicas-per-cluster",
                sizing.replicas_per_cluster,
                EntityKind::Replica,
                "replica",
            )?;
        }
        let grows = |a: f64, b: f64| (a > 0.0 || b > 0.0).then_some(1u64);
        want(
            "collection-grow/collection-churn",
            grows(scenario.collection_grow, scenario.collection_churn),
            EntityKind::Collection,
            "user collection",
        )?;
        want(
            "shard-grow/shard-churn",
            grows(scenario.shard_grow, scenario.shard_churn),
            EntityKind::Shard,
            "persist shard",
        )?;

        let count = |tag: &str, n: Option<u64>, kind: EntityKind| {
            jitter_count(
                seed,
                tag,
                n.unwrap_or(u64::from(exemplars[kind as usize])),
                jitter,
            )
        };
        let counts = Counts {
            shards: count("shards", sizing.shards, EntityKind::Shard),
            collections: count("collections", sizing.collections, EntityKind::Collection),
            clusters: count("clusters", sizing.clusters, EntityKind::Cluster),
            replicas_per_cluster: sizing
                .replicas_per_cluster
                .unwrap_or(u64::from(replica_ords))
                .max(1),
        };

        let paths = fixtures
            .iter()
            .map(|f| CPath {
                path: f.path.clone(),
                families: f
                    .families
                    .iter()
                    .map(|fam| compile_family(fam, &exemplars, &replica_ord))
                    .collect(),
            })
            .collect();

        let deviation = deviation_table(&scenario);
        Ok(Model {
            seed,
            jitter,
            scenario,
            paths,
            exemplars,
            replica_ords,
            counts,
            start,
            deviation,
            high_water: AtomicU64::new(0),
            last_size: AtomicUsize::new(64 * 1024),
        })
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.paths.iter().map(|p| p.path.as_str())
    }

    pub fn counts(&self) -> Counts {
        self.counts
    }

    pub fn exemplars(&self, kind: EntityKind) -> u32 {
        self.exemplars[kind as usize]
    }

    /// Where unix time `t` falls in the scenario's period, in `[0, 1)`. Keyed
    /// to the wall clock, so every pod in a fleet moves in step.
    fn phase(&self, t: f64) -> f64 {
        (t / self.scenario.period).rem_euclid(1.0)
    }

    /// `F(now)`: the integral of the multiplier since start, in constant time.
    ///
    /// Every shape averages to 1 over a period, so the multiplier's integral is
    /// the elapsed time plus a deviation that repeats every period. The table
    /// holds that deviation across one period; whole periods add nothing to it.
    ///
    /// The multiplier is never negative, so `F` never decreases in exact
    /// arithmetic. Where it is flat, as in a pulse's off phase at `amp=1`,
    /// rounding can still move it down by a few ulps, and Prometheus reads any
    /// decrease in a counter as a reset. So `F` is also clamped to the largest
    /// value any render has returned.
    fn integral(&self, now: f64) -> f64 {
        let elapsed = (now - self.start).max(0.0);
        let deviation = |t: f64| {
            let x = self.phase(t) * (self.deviation.len() - 1) as f64;
            let i = x.floor() as usize;
            let next = self.deviation[(i + 1).min(self.deviation.len() - 1)];
            self.deviation[i] + (next - self.deviation[i]) * (x - i as f64)
        };
        let f = (elapsed
            + self.scenario.period * (deviation(self.start + elapsed) - deviation(self.start)))
        .max(0.0);
        // Non-negative floats order the same as their bit patterns.
        let prev = self.high_water.fetch_max(f.to_bits(), Ordering::Relaxed);
        f.max(f64::from_bits(prev))
    }

    fn env(&self, now: f64) -> Env {
        let periods = ((now - self.start) / self.scenario.period).max(0.0);
        let s = &self.scenario;
        Env {
            now,
            tick: now.floor() as u64,
            integral: self.integral(now),
            multiplier: s.multiplier(self.phase(now)),
            shard_off: (s.shard_churn * periods).floor() as u64,
            shards: self.counts.shards + (s.shard_grow * periods).floor() as u64,
            coll_off: (s.collection_churn * periods).floor() as u64,
            colls: self.counts.collections + (s.collection_grow * periods).floor() as u64,
            clusters: self.counts.clusters.max(1),
            rpc: self.counts.replicas_per_cluster,
        }
    }

    /// The scrape body for `path` at unix time `now`, or `None` for a path the
    /// role does not serve.
    pub fn render(&self, path: &str, now: f64) -> Option<String> {
        let p = self.paths.iter().find(|p| p.path == path)?;
        let env = self.env(now);
        let mut out = String::with_capacity(self.last_size.load(Ordering::Relaxed) + 4096);
        for fam in &p.families {
            out.push_str(&fam.header);
            for &i in &fam.statics {
                self.emit(&mut out, &fam.series[i], Ctx::default(), &env);
            }
            for (kind, table) in &fam.driven {
                let e = self.exemplars[*kind as usize].max(1) as u64;
                let (lo, n) = match kind {
                    EntityKind::Shard => (env.shard_off, env.shards),
                    EntityKind::Collection => (env.coll_off, env.colls),
                    EntityKind::Replica => (0, env.clusters * env.rpc),
                    _ => (0, env.clusters),
                };
                for j in lo..lo + n {
                    let members = &table[(j % e) as usize];
                    if members.is_empty() {
                        continue;
                    }
                    let ctx = derive(*kind, j, &env);
                    let fan = if fam.fans_out
                        && matches!(kind, EntityKind::Shard | EntityKind::Collection)
                    {
                        env.rpc
                    } else {
                        1
                    };
                    for o in 0..fan {
                        for &i in members {
                            let s = &fam.series[i];
                            match s.replica_ord {
                                // Replica `o` of the cluster copies the exemplar
                                // replica in the same position.
                                Some(ro) => {
                                    if o % u64::from(self.replica_ords) == u64::from(ro) {
                                        let mut c = ctx;
                                        c.replica = c.cluster.map(|cl| cl * env.rpc + o);
                                        self.emit(&mut out, s, c, &env);
                                    }
                                }
                                None if o == 0 => self.emit(&mut out, s, ctx, &env),
                                None => {}
                            }
                        }
                    }
                }
            }
        }
        self.last_size.store(out.len(), Ordering::Relaxed);
        Some(out)
    }

    fn emit(&self, out: &mut String, s: &CSeries, ctx: Ctx, env: &Env) {
        out.push_str(&s.name);
        if !s.labels.is_empty() {
            out.push('{');
            for (i, (k, t)) in s.labels.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(k);
                out.push_str("=\"");
                for part in &t.parts {
                    match part {
                        Part::Lit(l) => expo::escape_label_value(out, l),
                        Part::Ph(ph) => self.write_placeholder(out, s, ph, ctx, env),
                    }
                }
                out.push('"');
            }
            out.push('}');
        }
        out.push(' ');
        expo::write_f64(out, self.value(s, ctx, env));
        out.push('\n');
    }

    fn value(&self, s: &CSeries, ctx: Ctx, env: &Env) -> f64 {
        let key = self.seed.mix(&[
            s.group,
            ctx.shard.map_or(0, |x| x + 1),
            ctx.collection.map_or(0, |x| x + 1),
            ctx.cluster.map_or(0, |x| x + 1),
            ctx.replica.map_or(0, |x| x + 1),
        ]);
        let spread = 1.0 + self.jitter * (2.0 * crate::seed::to_unit(key) - 1.0);
        match s.value {
            Value::Num(v0) if s.counter => {
                let v = v0 + s.rate * spread * env.integral;
                if s.integral { v.floor() } else { v }
            }
            Value::Num(v0) => {
                if v0 == 0.0 || v0 == 1.0 || !v0.is_finite() {
                    return v0;
                }
                let noise = 1.0
                    + self.jitter
                        * (2.0 * crate::seed::to_unit(self.seed.mix(&[key, env.tick])) - 1.0);
                let v = v0 * env.multiplier * noise;
                if s.integral { v.round() } else { v }
            }
            Value::NowMs(lag) => (env.now * 1000.0 - lag * spread).round(),
            Value::NowS(lag) => env.now - lag * spread,
            Value::StartS => (self.start * 1000.0).round() / 1000.0,
        }
    }

    fn write_placeholder(
        &self,
        out: &mut String,
        s: &CSeries,
        ph: &Placeholder,
        ctx: Ctx,
        env: &Env,
    ) {
        use std::fmt::Write as _;
        let offset = u64::from(ph.index - s.anchors[ph.kind as usize].min(ph.index));
        let wrap = |base: Option<u64>, lo: u64, n: u64| -> u64 {
            let b = base.unwrap_or(lo);
            if n == 0 {
                b
            } else {
                lo + (b - lo + offset) % n
            }
        };
        match (ph.kind, ph.attr) {
            (EntityKind::Shard, _) => {
                let j = wrap(ctx.shard, env.shard_off, env.shards);
                out.push('s');
                out.push_str(&self.seed.derive("shard").uuid(j));
            }
            (EntityKind::Collection, attr) => {
                let j = wrap(ctx.collection, env.coll_off, env.colls);
                let _ = match attr {
                    Attr::Id => write!(out, "u{}", 1000 + j),
                    Attr::Name => write!(out, "object_{j}"),
                    Attr::Schema => write!(out, "schema_{}", j % 16),
                    Attr::Database => write!(out, "materialize"),
                };
            }
            (EntityKind::Cluster, attr) => {
                let c = wrap(ctx.cluster, 0, env.clusters);
                let _ = match attr {
                    Attr::Name => write!(out, "cluster_{c}"),
                    _ => write!(out, "u{}", c + 1),
                };
            }
            (EntityKind::Replica, attr) => {
                let r = wrap(ctx.replica, 0, env.clusters * env.rpc);
                let _ = match attr {
                    Attr::Name => write!(out, "r{}", r % env.rpc + 1),
                    _ => write!(out, "u{}", r + 1),
                };
            }
            (EntityKind::Introspection, _) => {
                let r = ctx.replica.unwrap_or(0);
                let _ = write!(out, "si{}", (1u64 << 57) + r * 4096 + u64::from(ph.index));
            }
            (EntityKind::Environment, _) => {
                let _ = write!(
                    out,
                    "bench-{}-{}",
                    self.seed.derive("environment").uuid(u64::from(ph.index)),
                    0
                );
            }
            (EntityKind::Uuid, _) => {
                out.push_str(&self.seed.derive("uuid").uuid(u64::from(ph.index)))
            }
        }
    }
}

const DEVIATION_STEPS: usize = 3000;

/// The table [`Model::integral`] interpolates. Each cell integrates the
/// multiplier at a finer grain, so a pulse's edge lands within one cell.
fn deviation_table(scenario: &MetricScenario) -> Vec<f64> {
    const FINE: usize = 16;
    let n = DEVIATION_STEPS * FINE;
    let cell = |i: usize| {
        (0..FINE)
            .map(|k| scenario.multiplier(((i * FINE + k) as f64 + 0.5) / n as f64) - 1.0)
            .sum::<f64>()
            / n as f64
    };
    let mut table = Vec::with_capacity(DEVIATION_STEPS + 1);
    let mut acc = 0.0;
    table.push(0.0);
    for i in 0..DEVIATION_STEPS {
        acc += cell(i);
        table.push(acc);
    }
    // Rounding leaves a sliver at the end of the period; spread it back so a
    // whole period integrates to exactly its length.
    let residual = acc;
    for (i, v) in table.iter_mut().enumerate() {
        *v -= residual * i as f64 / DEVIATION_STEPS as f64;
    }
    table
}

fn derive(kind: EntityKind, j: u64, env: &Env) -> Ctx {
    match kind {
        EntityKind::Shard => {
            let collection =
                (env.colls > 0).then(|| env.coll_off + (j - env.shard_off) % env.colls);
            let cluster = collection.map(|c| c % env.clusters);
            Ctx {
                shard: Some(j),
                collection,
                cluster,
                replica: cluster.map(|c| c * env.rpc),
            }
        }
        EntityKind::Collection => {
            let c = j % env.clusters;
            Ctx {
                shard: None,
                collection: Some(j),
                cluster: Some(c),
                replica: Some(c * env.rpc),
            }
        }
        EntityKind::Replica => Ctx {
            replica: Some(j),
            cluster: Some(j / env.rpc),
            ..Ctx::default()
        },
        _ => Ctx {
            cluster: Some(j),
            replica: Some(j * env.rpc),
            ..Ctx::default()
        },
    }
}

fn compile_family(
    fam: &crate::fixture::Family,
    exemplars: &[u32; KINDS],
    replica_ord: &dyn Fn(u32) -> u32,
) -> CFamily {
    let mut header = String::new();
    if let Some(help) = &fam.help {
        header.push_str("# HELP ");
        header.push_str(&fam.name);
        header.push(' ');
        header.push_str(help);
        header.push('\n');
    }
    header.push_str("# TYPE ");
    header.push_str(&fam.name);
    header.push(' ');
    header.push_str(fam.kind.as_str());
    header.push('\n');

    let mut series = Vec::with_capacity(fam.series.len());
    let mut statics = Vec::new();
    let mut driven: Vec<(EntityKind, Vec<Vec<usize>>)> = Vec::new();
    let mut fans_out = false;
    for (i, s) in fam.series.iter().enumerate() {
        let mut anchors = [NONE; KINDS];
        for ph in s.labels.iter().flat_map(|(_, t)| t.placeholders()) {
            let a = &mut anchors[ph.kind as usize];
            *a = (*a).min(ph.index);
        }
        let driver = DRIVERS.into_iter().find(|k| anchors[*k as usize] != NONE);
        let replica = anchors[EntityKind::Replica as usize];
        let replica_ord = (replica != NONE
            && matches!(driver, Some(EntityKind::Shard | EntityKind::Collection)))
        .then(|| replica_ord(replica));
        fans_out |= replica_ord.is_some();

        match driver {
            None => statics.push(i),
            Some(kind) => {
                let slot = match driven.iter().position(|(k, _)| *k == kind) {
                    Some(p) => p,
                    None => {
                        driven.push((
                            kind,
                            vec![Vec::new(); exemplars[kind as usize].max(1) as usize],
                        ));
                        driven.len() - 1
                    }
                };
                driven[slot].1[anchors[kind as usize] as usize].push(i);
            }
        }

        let counter = counter_like(fam.kind, &fam.name, &s.name);
        let integral = match s.value {
            Value::Num(v) => v.fract() == 0.0,
            _ => true,
        };
        series.push(CSeries {
            name: s.name.clone(),
            labels: s.labels.clone(),
            anchors,
            replica_ord,
            value: s.value,
            rate: s.rate.unwrap_or(0.0),
            counter,
            integral,
            group: group_of(&fam.name, s),
        });
    }
    // Drive in a fixed order, so output order never depends on which kind a
    // family happened to name first.
    driven.sort_by_key(|(k, _)| DRIVERS.iter().position(|d| d == k));
    CFamily {
        header,
        series,
        statics,
        driven,
        fans_out,
    }
}

/// One histogram's or summary's series share a group: the family and every
/// label but `le` and `quantile`.
fn group_of(family: &str, s: &crate::fixture::Series) -> u64 {
    let mut key = String::from(family);
    for (k, v) in &s.labels {
        if k == "le" || k == "quantile" {
            continue;
        }
        key.push('\x1f');
        key.push_str(k);
        key.push('=');
        v.write(&mut key);
    }
    fnv1a(key.as_bytes())
}

fn roles_naming(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Shard | EntityKind::Collection => "environmentd and clusterd",
        _ => "environmentd",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expo;
    use crate::scenario::DEFAULT_PERIOD;

    const FIX: &str = r#"# role: environmentd
# path: /metrics
# TYPE mz_static gauge
mz_static{x="y"} 5
# TYPE mz_coll counter
mz_coll{collection_id="${collection:0}",instance_id="${cluster:0}",replica_id="${replica:0}"} 10 2
mz_coll{collection_id="${collection:1}",instance_id="${cluster:0}",replica_id="${replica:0}"} 10 2
# TYPE mz_shard gauge
mz_shard{shard="${shard:0}",name="${collection:0}"} 100
# TYPE mz_lat histogram
mz_lat_bucket{instance_id="${cluster:0}",le="1"} 1 1
mz_lat_bucket{instance_id="${cluster:0}",le="+Inf"} 3 2
mz_lat_sum{instance_id="${cluster:0}"} 1.5 0.5
mz_lat_count{instance_id="${cluster:0}"} 3 2
# TYPE mz_frontier gauge
mz_frontier{collection_id="${introspection:0}",replica_id="${replica:0}",instance_id="${cluster:0}"} now_ms-1000
"#;

    fn model(sizing: Sizing, scenario: MetricScenario) -> Model {
        let f = Fixture::parse(FIX).unwrap();
        Model::new(
            Role::Environmentd,
            &[f],
            &sizing,
            scenario,
            Seed::new("t"),
            0.0,
            1_000_000.0,
        )
        .unwrap()
    }

    fn series(body: &str) -> Vec<expo::Sample> {
        expo::parse(body)
            .unwrap()
            .families
            .into_iter()
            .flat_map(|f| f.samples)
            .collect()
    }

    #[test]
    fn expands_to_the_requested_counts() {
        let m = model(
            Sizing {
                shards: Some(50),
                collections: Some(100),
                clusters: Some(4),
                replicas_per_cluster: Some(2),
            },
            MetricScenario::default(),
        );
        let s = series(&m.render("/metrics", 1_000_100.0).unwrap());
        let count = |name: &str| s.iter().filter(|x| x.name == name).count();
        assert_eq!(count("mz_static"), 1);
        // 100 collections, each on both replicas of its cluster.
        assert_eq!(count("mz_coll"), 200);
        assert_eq!(count("mz_shard"), 50);
        assert_eq!(count("mz_lat_bucket"), 8);
        // One introspection collection per replica.
        assert_eq!(count("mz_frontier"), 8);
    }

    #[test]
    fn no_duplicate_series() {
        let m = model(
            Sizing {
                shards: Some(30),
                collections: Some(77),
                clusters: Some(5),
                replicas_per_cluster: Some(3),
            },
            MetricScenario::default(),
        );
        let s = series(&m.render("/metrics", 1_000_100.0).unwrap());
        let mut keys: Vec<_> = s
            .iter()
            .map(|x| format!("{}{:?}", x.name, x.labels))
            .collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n);
    }

    #[test]
    fn derived_placeholders_are_consistent() {
        let m = model(
            Sizing {
                collections: Some(10),
                clusters: Some(3),
                ..Sizing::default()
            },
            MetricScenario::default(),
        );
        for s in series(&m.render("/metrics", 1_000_100.0).unwrap())
            .iter()
            .filter(|x| x.name == "mz_coll")
        {
            let coll: u64 = s.label("collection_id").unwrap()[1..].parse().unwrap();
            let cluster: u64 = s.label("instance_id").unwrap()[1..].parse().unwrap();
            assert_eq!((coll - 1000) % 3 + 1, cluster);
        }
    }

    #[test]
    fn counters_never_decrease_and_buckets_stay_cumulative() {
        let scenario = MetricScenario::parse("dist=pulse60,amp=1", DEFAULT_PERIOD).unwrap();
        let f = Fixture::parse(FIX).unwrap();
        let m = Model::new(
            Role::Environmentd,
            &[f],
            &Sizing::default(),
            scenario,
            Seed::new("t"),
            0.1,
            1_000_000.0,
        )
        .unwrap();
        let mut last: Option<Vec<expo::Sample>> = None;
        for t in (0..600).step_by(7) {
            let s = series(&m.render("/metrics", 1_000_000.0 + t as f64).unwrap());
            let v = |name: &str, le: Option<&str>| -> f64 {
                s.iter()
                    .find(|x| x.name == name && x.label("le") == le)
                    .map(|x| x.value.parse().unwrap())
                    .unwrap()
            };
            assert!(v("mz_lat_bucket", Some("1")) <= v("mz_lat_bucket", Some("+Inf")));
            assert_eq!(v("mz_lat_bucket", Some("+Inf")), v("mz_lat_count", None));
            if let Some(prev) = &last {
                for (a, b) in prev.iter().zip(&s) {
                    if a.name == "mz_coll" || a.name.starts_with("mz_lat") {
                        let (a, b): (f64, f64) =
                            (a.value.parse().unwrap(), b.value.parse().unwrap());
                        assert!(b >= a, "{a} -> {b}");
                    }
                }
            }
            last = Some(s);
        }
    }

    /// The integral is a table lookup, so a render far from start costs what
    /// one near it does, and it never decreases, across period boundaries
    /// included. A stepwise integrator once looped forever on a period too
    /// short for its step to advance the clock.
    #[test]
    fn integral_is_monotonic_and_constant_time() {
        for (dist, period) in [
            ("dist=pulse30,amp=1", 60.0),
            ("dist=plateau500ms,amp=1", 1.0),
            ("dist=bell,amp=0.7", DEFAULT_PERIOD),
        ] {
            let m = model(
                Sizing::default(),
                MetricScenario::parse(dist, period).unwrap(),
            );
            let mut last = 0.0;
            for i in 0..20_000 {
                let elapsed = i as f64 * 0.037;
                let f = m.integral(1_000_000.0 + elapsed);
                assert!(f >= last, "{dist}: F fell from {last} to {f} at step {i}");
                // Within one period's swing of the elapsed time.
                assert!(
                    (f - elapsed).abs() <= m.scenario.period + 1e-6,
                    "{dist}: {f} vs {elapsed}"
                );
                last = f;
            }
            let far = m.integral(1_000_000.0 + 1e9);
            assert!((far - 1e9).abs() < 1e-3 * 1e9, "{dist}: {far}");
        }
    }

    #[test]
    fn shard_ids_are_seeded_and_stable() {
        let a = model(
            Sizing {
                shards: Some(3),
                ..Sizing::default()
            },
            MetricScenario::default(),
        );
        let b = model(
            Sizing {
                shards: Some(3),
                ..Sizing::default()
            },
            MetricScenario::default(),
        );
        let shards = |m: &Model| -> Vec<String> {
            series(&m.render("/metrics", 1_000_100.0).unwrap())
                .iter()
                .filter_map(|x| x.label("shard").map(str::to_string))
                .collect()
        };
        assert_eq!(shards(&a), shards(&b));
        assert_eq!(shards(&a).len(), 3);
        assert!(shards(&a)[0].starts_with('s'));
    }

    #[test]
    fn churn_replaces_and_grow_adds() {
        let m = model(
            Sizing {
                collections: Some(10),
                ..Sizing::default()
            },
            MetricScenario::parse("collection-grow=60,collection-churn=30", DEFAULT_PERIOD)
                .unwrap(),
        );
        let ids = |t: f64| -> Vec<String> {
            series(&m.render("/metrics", t).unwrap())
                .iter()
                .filter(|x| x.name == "mz_coll")
                .map(|x| x.label("collection_id").unwrap().to_string())
                .collect()
        };
        let before = ids(1_000_000.0);
        // Two periods: 120 added, 60 replaced.
        let after = ids(1_000_600.0);
        assert_eq!(before.len(), 10);
        assert_eq!(after.len(), 130);
        assert!(!after.contains(&"u1000".to_string()));
        assert!(after.contains(&"u1060".to_string()));
    }

    #[test]
    fn rejects_sizing_the_fixture_cannot_express() {
        let f =
            Fixture::parse("# role: balancerd\n# path: /metrics\n# TYPE x gauge\nx 1\n").unwrap();
        let err = Model::new(
            Role::Balancerd,
            &[f],
            &Sizing {
                collections: Some(5),
                ..Sizing::default()
            },
            MetricScenario::default(),
            Seed::new("t"),
            0.1,
            0.0,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(
            err.contains("--collection-count has no effect for role balancerd"),
            "{err}"
        );
    }

    #[test]
    fn clocks_stay_behind_now() {
        let m = model(Sizing::default(), MetricScenario::default());
        let s = series(&m.render("/metrics", 2_000_000.0).unwrap());
        let f = s.iter().find(|x| x.name == "mz_frontier").unwrap();
        assert_eq!(f.value, "1999999000");
    }
}
