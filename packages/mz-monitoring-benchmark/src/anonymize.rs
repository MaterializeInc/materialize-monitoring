// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Turns real scrapes into fixtures.
//!
//! Two jobs, done in one pass over each sample:
//!
//! * **Generalize.** Every user entity a sample names — a persist shard, a
//!   collection, a cluster, a replica, a replica's introspection collection —
//!   becomes a numbered placeholder, so `gen-load` can stamp out as many as it
//!   is asked for. Entities are recognized by the label that names them and by
//!   the shape of the ID: user objects are `u<n>`, system objects `s<n>`.
//!   System objects exist in every environment, so they stay literal.
//! * **Anonymize.** Names a user chose (objects, schemas, databases, clusters,
//!   replicas, applications), zones, environment IDs, UUIDs, addresses, host
//!   names and long hex strings never reach the fixture. User names become
//!   attributes of the entity's placeholder, and the rest become fixed
//!   stand-ins.
//!
//! The label tables below are the reviewed contract. A label that is in none
//! of them still has its values scrubbed of the patterns above, and is listed
//! in the report so that someone looks at it; `fixture-gen --strict` fails on
//! one instead.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;

use crate::expo::{self, Sample, counter_like};
use crate::fixture::{EntityKind, Family, Fixture, Series, Template, Value};
use crate::role::Role;

/// Labels whose `u<n>` value is a cluster.
const CLUSTER_ID_LABELS: &[&str] = &[
    "instance_id",
    "cluster_id",
    "compute_cluster_id",
    "compute_instance",
    "storage_cluster_id",
];
/// Labels whose `u<n>` value is a replica.
const REPLICA_ID_LABELS: &[&str] = &["replica_id", "compute_replica_id"];
const CLUSTER_NAME_LABELS: &[&str] = &["cluster_name", "compute_cluster_name"];
const REPLICA_NAME_LABELS: &[&str] = &["replica_name", "compute_replica_name"];
/// Labels holding an object's name, or its ID where persist names a shard by
/// the collection it backs.
const OBJECT_NAME_LABELS: &[&str] = &["name", "object_name", "source_name", "sink_name"];
const SCHEMA_LABELS: &[&str] = &["schema_name"];
const DATABASE_LABELS: &[&str] = &["database_name"];
const ZONE_LABELS: &[&str] = &["availability_zone", "zone", "region"];

/// Labels whose values are enumerations, IDs or numbers fixed by Materialize's
/// code rather than chosen by a user. Their values are kept, after the
/// pattern scrub every value gets.
const REVIEWED_LABELS: &[&str] = &[
    "action",
    "build_type",
    "cache",
    "call",
    "caller",
    "case",
    "cluster",
    "cmd",
    "code",
    "col",
    "collection",
    "collection_id",
    "column",
    "command_type",
    "compressed",
    "connection_type",
    "context",
    "controller",
    "envelope",
    "envelope_type",
    "external",
    "format",
    "format_connection",
    "format_connection_networking",
    "global_id",
    "honeycomb",
    "hydrated",
    "id",
    "immediately_handled",
    "isolation_level",
    "le",
    "location",
    "log",
    "log2_rows",
    "message_kind",
    "message_type",
    "method",
    "metric",
    "mz_version",
    "networking_type",
    "notice_type",
    "object_id",
    "object_type",
    "op",
    "operation",
    "order",
    "outcome",
    "parent_source_id",
    "path",
    "percentile",
    "phase",
    "quantile",
    "reason",
    "relation_type",
    "replica_full_name",
    "respond_immediately",
    "response_type",
    "result",
    "runtime",
    "sample",
    "selection",
    "server_name",
    "session_type",
    "shard",
    "shard_id",
    "sink",
    "sink_id",
    "size",
    "size_class",
    "source",
    "source_id",
    "stage",
    "state",
    "statement_type",
    "status",
    "step",
    "strategy",
    "substrate",
    "synthesized_pod",
    "transform",
    "type",
    "version",
    "worker_id",
    "workload_class",
];

/// Application names Materialize's own clients set. Anything else a user chose.
const KNOWN_APPLICATIONS: &[&str] = &[
    "",
    "unspecified",
    "psql",
    "web_console",
    "materialize_console",
    "mz_psql",
];

/// `name` values Materialize's code chooses where no object ID is in the
/// sample: persist's system shards, its tasks and semaphores, and the metrics
/// updaters. Any other ID-less name is replaced.
const KNOWN_NAMES: &[&str] = &[
    "unknown",
    "catalog",
    "txns",
    "expressions_cache",
    "builtin_migration",
    "heartbeat_read",
    "fetch",
    "usage",
    "rusage",
    "lgalloc",
    "lgalloc_map",
];

const SYSTEM_SCHEMAS: &[&str] = &[
    "",
    "information_schema",
    "mz_catalog",
    "mz_catalog_unstable",
    "mz_internal",
    "mz_introspection",
    "mz_unsafe",
    "pg_catalog",
];

static USER_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^u\d+$").unwrap());
static SYSTEM_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^s\d+$").unwrap());
static INTROSPECTION_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^si\d+$").unwrap());
static REPLICA_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^r\d+$").unwrap());
static SIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\d+(?:cc|C)?|mz_[a-z_]+|[0-9]+-[0-9]+|xsmall|small|medium|large|xlarge|[0-9]*x?large)$").unwrap()
});
static SYNTHESIZED_POD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^cluster-(u\d+)-replica-(u\d+)-(\d+)$").unwrap());
static SHARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bs[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap()
});
static ENVIRONMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b[a-z]+(?:-[a-z0-9]+)*-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}-\d+\b",
    )
    .unwrap()
});
static UUID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")
        .unwrap()
});
static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+").unwrap());
static IPV4: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").unwrap());
static FQDN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+){2,}\b").unwrap());
static LONG_HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[0-9a-fA-F]{16,}\b").unwrap());
static RESOURCE_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bmz[0-9a-z]{10}\b").unwrap());

const YEAR_MS: f64 = 365.25 * 24.0 * 3600.0 * 1000.0;

/// One endpoint's scrapes.
pub struct Capture {
    pub path: String,
    pub first: String,
    /// A second scrape `window_secs` later, from which counters get their rates.
    pub second: Option<String>,
    pub window_secs: f64,
    /// When the first scrape was taken, in milliseconds since the epoch.
    pub at_unix_ms: f64,
}

pub struct Options {
    /// Keep at most this many shards and collections; series naming more are
    /// dropped. They add nothing `gen-load` cannot stamp out from the rest.
    pub max_exemplars: u32,
}

impl Default for Options {
    fn default() -> Self {
        Options { max_exemplars: 32 }
    }
}

#[derive(Debug, Default)]
pub struct Report {
    /// `(family, label)` pairs whose label is in no reviewed table.
    pub unreviewed: BTreeSet<(String, String)>,
    /// Series dropped for naming an entity past the exemplar cap.
    pub over_cap: usize,
    /// Series dropped because anonymizing made them identical to another.
    pub duplicates: usize,
    /// Exemplars of each kind in the role's fixtures.
    pub exemplars: BTreeMap<&'static str, u32>,
    /// Distinct values replaced by a fixed stand-in, per class. A class that
    /// grows between captures is a label that started carrying new values.
    pub standins: BTreeMap<&'static str, u32>,
}

#[derive(Default)]
struct Registry {
    ids: HashMap<(EntityKind, String), u32>,
    next: HashMap<EntityKind, u32>,
    /// A user replica's introspection collections, numbered per replica.
    introspection: HashMap<(u32, String), u32>,
    introspection_next: HashMap<u32, u32>,
    /// Fixed stand-ins for anonymized values, per class.
    standins: HashMap<(&'static str, String), String>,
    standin_next: HashMap<&'static str, u32>,
}

impl Registry {
    fn get(&self, kind: EntityKind, raw: &str) -> Option<u32> {
        self.ids.get(&(kind, raw.to_string())).copied()
    }

    fn id(&mut self, kind: EntityKind, raw: &str) -> u32 {
        if let Some(i) = self.get(kind, raw) {
            return i;
        }
        let next = self.next.entry(kind).or_default();
        let i = *next;
        *next += 1;
        self.ids.insert((kind, raw.to_string()), i);
        i
    }

    fn would_exceed(&self, kind: EntityKind, raw: &str, cap: u32) -> bool {
        self.get(kind, raw).is_none() && self.next.get(&kind).copied().unwrap_or(0) >= cap
    }

    fn introspection(&mut self, replica: u32, raw: &str) -> u32 {
        if let Some(i) = self.introspection.get(&(replica, raw.to_string())) {
            return *i;
        }
        let next = self.introspection_next.entry(replica).or_default();
        let i = *next;
        *next += 1;
        self.introspection.insert((replica, raw.to_string()), i);
        i
    }

    fn standin(&mut self, class: &'static str, raw: &str, make: impl Fn(u32) -> String) -> String {
        if let Some(s) = self.standins.get(&(class, raw.to_string())) {
            return s.clone();
        }
        let next = self.standin_next.entry(class).or_default();
        let s = make(*next);
        *next += 1;
        self.standins.insert((class, raw.to_string()), s.clone());
        s
    }

    /// Replace every identifying pattern in `raw`, returning template text
    /// (placeholders in `${…}` syntax, literal `$` doubled).
    fn scrub(&mut self, raw: &str) -> String {
        let mut s = raw.replace('$', "$$");
        s = replace_all(&SHARD, &s, |m| {
            format!("${{shard:{}}}", self.id(EntityKind::Shard, m))
        });
        s = replace_all(&ENVIRONMENT, &s, |m| {
            format!("${{environment:{}}}", self.id(EntityKind::Environment, m))
        });
        s = replace_all(&UUID, &s, |m| {
            format!("${{uuid:{}}}", self.id(EntityKind::Uuid, m))
        });
        s = replace_all(&EMAIL, &s, |m| {
            self.standin("email", m, |n| format!("user{n}@example.com"))
        });
        s = replace_all(&IPV4, &s, |m| {
            self.standin("ip", m, |n| format!("192.0.2.{}", n % 254 + 1))
        });
        s = replace_all(&FQDN, &s, |m| {
            let last = m.rsplit('.').next().unwrap_or("");
            let is_host = last.len() >= 2 && last.chars().all(|c| c.is_ascii_alphabetic());
            if is_host {
                self.standin("host", m, |n| format!("host-{n}.example.internal"))
            } else {
                m.to_string()
            }
        });
        s = replace_all(&LONG_HEX, &s, |m| {
            if m.chars().any(|c| c.is_ascii_alphabetic()) {
                self.standin("hex", m, |n| format!("{n:016x}"))
            } else {
                m.to_string()
            }
        });
        replace_all(&RESOURCE_ID, &s, |m| {
            self.standin("resource", m, |n| format!("mzbench{n:05}"))
        })
    }
}

fn replace_all(re: &Regex, s: &str, mut f: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for m in re.find_iter(s) {
        // Leave the placeholders an earlier pattern wrote alone.
        let before = &s[..m.start()];
        let inside = before
            .rfind("${")
            .is_some_and(|open| !before[open..].contains('}'));
        if inside {
            continue;
        }
        out.push_str(&s[last..m.start()]);
        out.push_str(&f(m.as_str()));
        last = m.end();
    }
    out.push_str(&s[last..]);
    out
}

/// The user entities one sample names, as raw IDs.
#[derive(Default)]
struct Named {
    clusters: Vec<String>,
    replicas: Vec<String>,
    collections: Vec<String>,
    shards: Vec<String>,
    /// Whether the sample names a system object, whose names are public.
    system: bool,
}

fn scan(sample: &Sample) -> Named {
    let mut n = Named::default();
    for (k, v) in &sample.labels {
        for m in SHARD.find_iter(v) {
            n.shards.push(m.as_str().to_string());
        }
        if let Some(c) = SYNTHESIZED_POD.captures(v) {
            n.clusters.push(c[1].to_string());
            n.replicas.push(c[2].to_string());
        } else if USER_ID.is_match(v) {
            if CLUSTER_ID_LABELS.contains(&k.as_str()) {
                n.clusters.push(v.clone());
            } else if REPLICA_ID_LABELS.contains(&k.as_str()) {
                n.replicas.push(v.clone());
            } else {
                n.collections.push(v.clone());
            }
        } else if SYSTEM_ID.is_match(v) {
            n.system = true;
        }
    }
    n
}

/// Anonymize one role's captures into one fixture per endpoint.
pub fn anonymize(
    role: Role,
    captures: &[Capture],
    materialize: Option<String>,
    opts: &Options,
) -> Result<(Vec<Fixture>, Report)> {
    let mut reg = Registry::default();
    let mut report = Report::default();
    let mut fixtures = Vec::new();

    for cap in captures {
        let first = expo::parse(&cap.first).with_context(|| format!("parsing {}", cap.path))?;
        let second: HashMap<String, f64> = match &cap.second {
            None => HashMap::new(),
            Some(body) => expo::parse(body)
                .with_context(|| format!("parsing second scrape of {}", cap.path))?
                .families
                .iter()
                .flat_map(|f| f.samples.iter())
                .filter_map(|s| Some((sample_key(s), expo::parse_f64(&s.value)?)))
                .collect(),
        };

        let mut families = Vec::with_capacity(first.families.len());
        for fam in &first.families {
            let mut seen = HashSet::new();
            let mut series = Vec::with_capacity(fam.samples.len());
            for sample in &fam.samples {
                let named = scan(sample);
                let cap_hit = named
                    .shards
                    .iter()
                    .any(|r| reg.would_exceed(EntityKind::Shard, r, opts.max_exemplars))
                    || named
                        .collections
                        .iter()
                        .any(|r| reg.would_exceed(EntityKind::Collection, r, opts.max_exemplars));
                if cap_hit {
                    report.over_cap += 1;
                    continue;
                }
                let labels = sample
                    .labels
                    .iter()
                    .map(|(k, v)| {
                        if !is_reviewed(k) {
                            report.unreviewed.insert((fam.name.clone(), k.clone()));
                        }
                        Ok((
                            k.clone(),
                            Template::parse(&rewrite(&mut reg, &named, k, v))?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;

                let mut key = sample.name.clone();
                for (k, v) in &labels {
                    key.push('\x1f');
                    key.push_str(k);
                    key.push('=');
                    key.push_str(&v.to_string());
                }
                if !seen.insert(key) {
                    report.duplicates += 1;
                    continue;
                }

                let counter = counter_like(fam.kind, &fam.name, &sample.name);
                let raw = expo::parse_f64(&sample.value)
                    .with_context(|| format!("bad value on {}", sample.name))?;
                let rate = if counter {
                    second
                        .get(&sample_key(sample))
                        .map(|v2| ((v2 - raw) / cap.window_secs).max(0.0))
                        .filter(|r| r.is_finite() && *r > 0.0)
                        .map(|r| round_sig(r, 4))
                } else {
                    None
                };
                series.push(Series {
                    name: sample.name.clone(),
                    labels,
                    value: if counter {
                        Value::Num(raw)
                    } else {
                        gauge_value(&fam.name, raw, cap.at_unix_ms)
                    },
                    rate,
                });
            }
            families.push(Family {
                name: fam.name.clone(),
                kind: fam.kind,
                help: fam.help.as_ref().map(|h| {
                    // Help text is static in Materialize's code; scrub it
                    // anyway, but to fixed text rather than placeholders.
                    Template::parse(&reg.scrub(h))
                        .map(|t| strip_placeholders(&t))
                        .unwrap_or_default()
                }),
                series,
            });
        }
        fixtures.push(Fixture {
            role: role.as_str().to_string(),
            path: cap.path.clone(),
            materialize: materialize.clone(),
            families,
        });
    }

    for kind in EntityKind::ALL {
        let n = if kind == EntityKind::Introspection {
            reg.introspection_next.values().copied().max().unwrap_or(0)
        } else {
            reg.next.get(&kind).copied().unwrap_or(0)
        };
        if n > 0 {
            report.exemplars.insert(kind.as_str(), n);
        }
    }
    report.standins = reg.standin_next.iter().map(|(k, v)| (*k, *v)).collect();
    Ok((fixtures, report))
}

fn is_reviewed(label: &str) -> bool {
    [
        CLUSTER_ID_LABELS,
        REPLICA_ID_LABELS,
        CLUSTER_NAME_LABELS,
        REPLICA_NAME_LABELS,
        OBJECT_NAME_LABELS,
        SCHEMA_LABELS,
        DATABASE_LABELS,
        ZONE_LABELS,
        REVIEWED_LABELS,
        &["application_name"],
    ]
    .iter()
    .any(|t| t.contains(&label))
}

/// The template text for one label value.
fn rewrite(reg: &mut Registry, named: &Named, label: &str, value: &str) -> String {
    let first = |kind: EntityKind, raws: &[String], reg: &mut Registry| -> Option<u32> {
        raws.first().map(|r| reg.id(kind, r))
    };
    let replica_index = named
        .replicas
        .first()
        .map(|r| reg.id(EntityKind::Replica, r));

    if let Some(c) = SYNTHESIZED_POD.captures(value) {
        let cluster = reg.id(EntityKind::Cluster, &c[1]);
        let replica = reg.id(EntityKind::Replica, &c[2]);
        return format!(
            "cluster-${{cluster:{cluster}}}-replica-${{replica:{replica}}}-{}",
            &c[3]
        );
    }
    if USER_ID.is_match(value) {
        let kind = if CLUSTER_ID_LABELS.contains(&label) {
            EntityKind::Cluster
        } else if REPLICA_ID_LABELS.contains(&label) {
            EntityKind::Replica
        } else {
            EntityKind::Collection
        };
        return format!("${{{}:{}}}", kind.as_str(), reg.id(kind, value));
    }
    if INTROSPECTION_ID.is_match(value)
        && let Some(replica) = replica_index
    {
        return format!("${{introspection:{}}}", reg.introspection(replica, value));
    }

    if CLUSTER_NAME_LABELS.contains(&label) {
        if let Some(i) = first(EntityKind::Cluster, &named.clusters, reg) {
            return format!("${{cluster:{i}.name}}");
        }
        return keep_system(reg, "cluster", value);
    }
    if REPLICA_NAME_LABELS.contains(&label) {
        if let Some(i) = replica_index {
            return format!("${{replica:{i}.name}}");
        }
        if REPLICA_NAME.is_match(value) {
            return value.to_string();
        }
        return reg.standin("replica", value, |n| format!("replica-{n}"));
    }
    if label == "replica_full_name" {
        let (cluster, replica) = value.split_once('.').unwrap_or((value, ""));
        let cluster = match first(EntityKind::Cluster, &named.clusters, reg) {
            Some(i) => format!("${{cluster:{i}.name}}"),
            None => keep_system(reg, "cluster", cluster),
        };
        let replica = match replica_index {
            Some(i) => format!("${{replica:{i}.name}}"),
            None if REPLICA_NAME.is_match(replica) => replica.to_string(),
            None => reg.standin("replica", replica, |n| format!("replica-{n}")),
        };
        return format!("{cluster}.{replica}");
    }
    if OBJECT_NAME_LABELS.contains(&label) {
        // The most specific entity the sample names owns the name:
        // `mz_object_info` names a collection, `mz_replica_info` a replica,
        // `mz_cluster_info` a cluster.
        if let Some(i) = first(EntityKind::Collection, &named.collections, reg) {
            return format!("${{collection:{i}.name}}");
        }
        if let Some(i) = replica_index {
            return format!("${{replica:{i}.name}}");
        }
        if let Some(i) = first(EntityKind::Cluster, &named.clusters, reg) {
            return format!("${{cluster:{i}.name}}");
        }
        if named.system || SYSTEM_ID.is_match(value) || KNOWN_NAMES.contains(&value) {
            return reg.scrub(value);
        }
        return keep_system(reg, "object", value);
    }
    if SCHEMA_LABELS.contains(&label) {
        if SYSTEM_SCHEMAS.contains(&value) {
            return value.to_string();
        }
        if let Some(i) = first(EntityKind::Collection, &named.collections, reg) {
            return format!("${{collection:{i}.schema}}");
        }
        return reg.standin("schema", value, |n| format!("schema_{n}"));
    }
    if DATABASE_LABELS.contains(&label) {
        if value.is_empty() {
            return String::new();
        }
        if let Some(i) = first(EntityKind::Collection, &named.collections, reg) {
            return format!("${{collection:{i}.database}}");
        }
        return reg.standin("database", value, |n| format!("database_{n}"));
    }
    if ZONE_LABELS.contains(&label) {
        if value.is_empty() {
            return String::new();
        }
        return reg.standin("zone", value, |n| {
            format!("zone-{}", char::from(b'a' + (n % 26) as u8))
        });
    }
    if label == "application_name" {
        if KNOWN_APPLICATIONS.contains(&value) || value.starts_with("mz_") {
            return value.to_string();
        }
        return reg.standin("application", value, |n| format!("application_{n}"));
    }
    if label == "size" && !SIZE.is_match(value) {
        return reg.standin("size", value, |n| format!("size-{n}"));
    }
    if label == "path"
        && let Some(rest) = value.strip_prefix("/api/webhook/")
    {
        let depth = rest.split('/').count();
        return format!("/api/webhook{}", "/redacted".repeat(depth));
    }
    reg.scrub(value)
}

/// Keep a name Materialize itself chose; replace one a user chose.
fn keep_system(reg: &mut Registry, class: &'static str, value: &str) -> String {
    if value.is_empty()
        || value.starts_with("mz_")
        || value.starts_with("pg_")
        || value.starts_with('_')
    {
        return reg.scrub(value);
    }
    reg.standin(class, value, |n| format!("{class}_{n}"))
}

fn strip_placeholders(t: &Template) -> String {
    t.parts
        .iter()
        .map(|p| match p {
            crate::fixture::Part::Lit(s) => s.clone(),
            crate::fixture::Part::Ph(ph) => format!("<{}>", ph.kind.as_str()),
        })
        .collect()
}

fn sample_key(s: &Sample) -> String {
    let mut key = s.name.clone();
    for (k, v) in &s.labels {
        key.push('\x1f');
        key.push_str(k);
        key.push('=');
        key.push_str(v);
    }
    key
}

/// A gauge's value, made relative to the clock when it is a wall-clock time.
///
/// By range and by name together: a range alone would mistake a 1.6 GB memory
/// gauge for a time in seconds, and a 1.7 TB one for a time in milliseconds.
fn gauge_value(family: &str, v: f64, at_ms: f64) -> Value {
    if family == "process_start_time_seconds" {
        return Value::StartS;
    }
    if family.contains("bytes") || !v.is_finite() {
        return Value::Num(v);
    }
    let near = |now: f64, span: f64| v > now - 5.0 * span && v < now + span / 365.0;
    if near(at_ms, YEAR_MS) && v.fract() == 0.0 {
        return Value::NowMs((at_ms - v).round());
    }
    let timeish = ["time", "timestamp", "seconds", "frontier", "since", "upper"]
        .iter()
        .any(|w| family.contains(w));
    if timeish && near(at_ms / 1000.0, YEAR_MS / 1000.0) {
        return Value::NowS(round_sig(at_ms / 1000.0 - v, 6));
    }
    Value::Num(v)
}

fn round_sig(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT_MS: f64 = 1_794_946_351_000.0;

    fn capture(path: &str, first: &str, second: Option<&str>) -> Capture {
        Capture {
            path: path.to_string(),
            first: first.to_string(),
            second: second.map(str::to_string),
            window_secs: 10.0,
            at_unix_ms: AT_MS,
        }
    }

    const FIRST: &str = r#"# HELP mz_object_info Objects.
# TYPE mz_object_info gauge
mz_object_info{database_name="acme",global_id="u1537",name="orders_by_region",object_id="u1537",schema_name="acme_prod",type="source"} 1
mz_object_info{database_name="",global_id="s441",name="mz_tables",object_id="s441",schema_name="mz_catalog",type="table"} 1
# HELP mz_persist_shard_usage_bytes Usage.
# TYPE mz_persist_shard_usage_bytes gauge
mz_persist_shard_usage_bytes{name="u1537",shard="s09b50805-d2b5-1d95-ed5b-df1a17af442d"} 1824
# HELP mz_compute_cluster_status Replicas.
# TYPE mz_compute_cluster_status gauge
mz_compute_cluster_status{availability_zone="us-east1-b",compute_cluster_id="u1",compute_cluster_name="acme_cluster",compute_replica_id="u7",compute_replica_name="r1",size="25cc"} 1
# HELP mz_replica_write_frontiers Frontiers.
# TYPE mz_replica_write_frontiers gauge
mz_replica_write_frontiers{collection_id="si144115188075856129",instance_id="u1",replica_id="u7"} 1794946350000
mz_replica_write_frontiers{collection_id="si144115188075856130",instance_id="u1",replica_id="u7"} 1794946350500
# HELP mz_queries_total Queries.
# TYPE mz_queries_total counter
mz_queries_total{application_name="acme-billing",session_type="user"} 100
# HELP mz_env_info Info.
# TYPE mz_env_info gauge
mz_env_info{environment_id="gcp-us-east1-a676b284-369d-45de-a349-ae11b6aeda8b-0",host="mzyz3lplvppc-environmentd.acme.svc.cluster.local",ip="10.4.5.6",org="a676b284-369d-45de-a349-ae11b6aeda8b",user="ops@acme.example"} 1
# HELP mz_storage_objects Storage.
# TYPE mz_storage_objects gauge
mz_storage_objects{cluster_id="u1",id="u1537",replica_id="u7",synthesized_pod="cluster-u1-replica-u7-0"} 1
# HELP mz_cluster_info Clusters.
# TYPE mz_cluster_info gauge
mz_cluster_info{cluster_id="u1",name="acme_cluster",size="acme-xl"} 1
# HELP mz_persist_shard_compact_batches_topk Batches.
# TYPE mz_persist_shard_compact_batches_topk gauge
mz_persist_shard_compact_batches_topk{name="txns",shard="s19a097e1-9733-4eba-a7e2-ed2d2305339b"} 4
mz_persist_shard_compact_batches_topk{name="acme_leftover",shard="s1f0e7de3-4212-4d31-b525-6fd8b258def3"} 4
# HELP process_start_time_seconds Start.
# TYPE process_start_time_seconds gauge
process_start_time_seconds 1794940000
"#;

    fn run() -> (Fixture, Report) {
        let second = FIRST.replace(
            "mz_queries_total{application_name=\"acme-billing\",session_type=\"user\"} 100",
            "mz_queries_total{application_name=\"acme-billing\",session_type=\"user\"} 125",
        );
        let (mut f, r) = anonymize(
            Role::Environmentd,
            &[capture("/metrics", FIRST, Some(&second))],
            Some("v26.49.1".into()),
            &Options::default(),
        )
        .unwrap();
        (f.remove(0), r)
    }

    #[test]
    fn nothing_identifying_survives() {
        let (f, _) = run();
        let text = f.to_text();
        for leak in [
            "acme",
            "orders_by_region",
            "a676b284",
            "s09b50805",
            "us-east1",
            "10.4.5.6",
            "svc.cluster.local",
            "mzyz3lplvppc",
            "ops@",
            "u1537",
            "\"u1\"",
            "\"u7\"",
        ] {
            assert!(!text.contains(leak), "{leak} leaked into:\n{text}");
        }
    }

    #[test]
    fn user_entities_become_placeholders() {
        let (f, r) = run();
        let text = f.to_text();
        for want in [
            r#"name="${collection:0.name}""#,
            r#"schema_name="${collection:0.schema}""#,
            r#"database_name="${collection:0.database}""#,
            r#"shard="${shard:0}""#,
            r#"name="${collection:0}""#,
            r#"compute_cluster_name="${cluster:0.name}""#,
            r#"compute_replica_name="${replica:0.name}""#,
            r#"collection_id="${introspection:1}""#,
            r#"synthesized_pod="cluster-${cluster:0}-replica-${replica:0}-0""#,
            r#"environment_id="${environment:0}""#,
            r#"org="${uuid:0}""#,
            r#"availability_zone="zone-a""#,
            r#"application_name="application_0""#,
            r#"mz_cluster_info{cluster_id="${cluster:0}",name="${cluster:0.name}",size="size-0"}"#,
            r#"{name="txns",shard="${shard:1}"}"#,
            r#"{name="object_0",shard="${shard:2}"}"#,
        ] {
            assert!(text.contains(want), "missing {want} in:\n{text}");
        }
        assert_eq!(r.exemplars.get("collection"), Some(&1));
        assert_eq!(r.exemplars.get("introspection"), Some(&2));
    }

    #[test]
    fn system_objects_stay_literal() {
        let (f, _) = run();
        let text = f.to_text();
        assert!(text.contains(r#"name="mz_tables""#));
        assert!(text.contains(r#"schema_name="mz_catalog""#));
        assert!(text.contains(r#"global_id="s441""#));
    }

    #[test]
    fn counters_get_rates_and_clocks_become_relative() {
        let (f, _) = run();
        let text = f.to_text();
        assert!(text.contains("} 100 2.5\n"), "{text}");
        assert!(text.contains("} now_ms-1000\n"), "{text}");
        assert!(text.contains("process_start_time_seconds start_s\n"));
        // A byte gauge in the millisecond range stays a number.
        assert!(text.contains("} 1824\n"));
    }

    #[test]
    fn unreviewed_labels_are_reported() {
        let (_, r) = run();
        let labels: Vec<_> = r.unreviewed.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels, ["environment_id", "host", "ip", "org", "user"]);
    }

    #[test]
    fn exemplar_cap_drops_extra_entities() {
        let body = (0..10)
            .map(|i| format!("mz_x{{collection_id=\"u{i}\"}} 1\n"))
            .collect::<String>();
        let (f, r) = anonymize(
            Role::Clusterd,
            &[capture(
                "/metrics",
                &format!("# TYPE mz_x gauge\n{body}"),
                None,
            )],
            None,
            &Options { max_exemplars: 4 },
        )
        .unwrap();
        assert_eq!(f[0].families[0].series.len(), 4);
        assert_eq!(r.over_cap, 6);
    }

    #[test]
    fn webhook_paths_are_redacted() {
        let mut reg = Registry::default();
        assert_eq!(
            rewrite(
                &mut reg,
                &Named::default(),
                "path",
                "/api/webhook/db/acme/orders"
            ),
            "/api/webhook/redacted/redacted/redacted"
        );
        assert_eq!(
            rewrite(&mut reg, &Named::default(), "path", "/api/login"),
            "/api/login"
        );
    }
}
