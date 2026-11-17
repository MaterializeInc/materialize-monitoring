// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser};

use mz_monitoring_benchmark::anonymize::{self, Capture, Report};
use mz_monitoring_benchmark::fixture::Fixture;
use mz_monitoring_benchmark::logs::{self, Ids, LogGen};
use mz_monitoring_benchmark::model::{Model, Sizing};
use mz_monitoring_benchmark::role::{self, LogFormat, Role};
use mz_monitoring_benchmark::scenario::{LogScenario, MetricScenario, parse_duration};
use mz_monitoring_benchmark::seed::Seed;
use mz_monitoring_benchmark::serve;

/// Synthetic Materialize metrics and logs for load-testing the monitoring stack.
#[derive(Parser)]
#[command(name = "mz-monitoring-benchmark", version)]
enum Cli {
    /// Impersonate one Materialize process: serve its metrics, write its logs.
    GenLoad(GenLoad),
    /// Capture fixtures from a live install, anonymized.
    FixtureGen(FixtureGen),
    /// Print what `gen-load` would serve, or how much of it.
    Preview(Preview),
}

#[derive(Args)]
struct Common {
    /// The process to impersonate.
    #[arg(long, value_enum)]
    role: Role,
    /// Seeds every identity the pod generates. The same seed serves the same
    /// shard IDs after a restart. Defaults to `$HOSTNAME`, which is stable for
    /// a StatefulSet pod.
    #[arg(long, env = "MZ_BENCH_SEED")]
    seed: Option<String>,
    /// Persist shards to generate. Default: as many as the fixtures name.
    #[arg(long)]
    persist_shards: Option<u64>,
    /// User collections to generate.
    #[arg(long)]
    collection_count: Option<u64>,
    /// User clusters to generate.
    #[arg(long)]
    cluster_count: Option<u64>,
    /// Replicas in each user cluster.
    #[arg(long)]
    replicas_per_cluster: Option<u64>,
    /// How metrics move, per period (5m unless `period=` says otherwise):
    /// `dist=average|bell|sine|saw|pulse<dur>|plateau<dur>,amp=0.5,period=5m,
    /// collection-grow=<n>,collection-churn=<n>,shard-grow=<n>,shard-churn=<n>`.
    #[arg(long, default_value = "")]
    metric_scenario: String,
    /// Relative spread applied to every count, rate and value.
    #[arg(long, default_value_t = 0.1)]
    jitter: f64,
    /// Read fixtures from this directory instead of the embedded copies.
    #[arg(long)]
    fixtures: Option<PathBuf>,
}

impl Common {
    fn seed(&self) -> Seed {
        let s = self
            .seed
            .clone()
            .or_else(|| std::env::var("HOSTNAME").ok())
            .unwrap_or_else(|| "mz-monitoring-benchmark".to_string());
        Seed::new(&s)
    }

    fn model(&self, start: f64) -> Result<Model> {
        if !(0.0..1.0).contains(&self.jitter) {
            bail!("--jitter is a fraction in [0, 1)");
        }
        let fixtures = role::load_fixtures(self.role, self.fixtures.as_deref())?;
        Model::new(
            self.role,
            &fixtures,
            &Sizing {
                shards: self.persist_shards,
                collections: self.collection_count,
                clusters: self.cluster_count,
                replicas_per_cluster: self.replicas_per_cluster,
            },
            MetricScenario::parse(&self.metric_scenario).context("--metric-scenario")?,
            self.seed(),
            self.jitter,
            start,
        )
    }
}

#[derive(Args)]
struct GenLoad {
    #[command(flatten)]
    common: Common,
    #[arg(long, default_value = "0.0.0.0:6878")]
    listen: SocketAddr,
    /// Log lines per period (5m unless `period=` says otherwise):
    /// `info=1500,warn=150,error=15`, `<level>-dist=average|bell|sine|saw|
    /// pulse<dur>|plateau<dur>`, `dist=…`, `period=5m`, `phase=sync|seeded`,
    /// `non-json=<n>`, `eicar=<n>`, `panic=<n>`, `panic-exit=true|false`.
    /// A count below 1 fires once every few periods: `panic=0.25` is every 20m.
    #[arg(long, default_value = "")]
    log_scenario: String,
    /// Serve metrics only.
    #[arg(long)]
    no_logs: bool,
}

#[derive(Args)]
struct FixtureGen {
    /// Kube context to capture from. Default: the current context.
    #[arg(long)]
    context: Option<String>,
    /// The Materialize environment's namespace, when the cluster runs several.
    #[arg(long)]
    namespace: Option<String>,
    /// `namespace/name` of the Secret with the SQL exporter endpoints' basic auth.
    #[arg(long, default_value = "monitoring/materialize-sql-monitor")]
    sql_auth_secret: String,
    /// Time between the two scrapes that give counters their rates.
    #[arg(long, default_value = "15s")]
    window: String,
    /// Roles to capture. Default: all.
    #[arg(long, value_enum)]
    role: Vec<Role>,
    /// Where fixtures are written, one directory per role.
    #[arg(long, default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures"))]
    out: PathBuf,
    /// Keep at most this many shards and collections.
    #[arg(long, default_value_t = 32)]
    max_exemplars: u32,
    /// Fail when a label is in no reviewed table.
    #[arg(long)]
    strict: bool,
    /// Compare the capture's families and label names with the fixtures in
    /// `--out`, write nothing, and fail on any difference.
    #[arg(long)]
    check: bool,
    /// Read raw scrapes from `<dir>/<role>/<file>.prom` (and `<file>.2.prom`
    /// for rates) instead of capturing live.
    #[arg(long)]
    from_dir: Option<PathBuf>,
}

#[derive(Args)]
struct Preview {
    #[command(flatten)]
    common: Common,
    /// Seconds after start to render at, which matters for growth and churn.
    #[arg(long, default_value_t = 0.0)]
    at: f64,
    /// Print the body of this path instead of a summary.
    #[arg(long)]
    body: Option<String>,
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::GenLoad(args) => gen_load(args),
        Cli::FixtureGen(args) => fixture_gen(args),
        Cli::Preview(args) => preview(args),
    }
}

fn gen_load(args: GenLoad) -> Result<()> {
    let start = logs::unix_now();
    let model = Arc::new(args.common.model(start)?);
    let scenario = LogScenario::parse(&args.log_scenario).context("--log-scenario")?;
    let role = args.common.role;
    if scenario.non_json > 0.0 && role.log_format() != LogFormat::TracingJson {
        bail!(
            "non-json applies to roles that log JSON (environmentd, clusterd); every {role} line is already plain text"
        );
    }
    if scenario.panic > 0.0 && !role.is_rust() {
        bail!("panic applies to Rust processes; {role} is nginx");
    }

    let counts = model.counts();
    println!(
        "mz-monitoring-benchmark {role}: shards={} collections={} clusters={} replicas_per_cluster={} paths={}",
        counts.shards,
        counts.collections,
        counts.clusters,
        counts.replicas_per_cluster,
        model.paths().collect::<Vec<_>>().join(",")
    );

    if !args.no_logs {
        let generator = LogGen::new(
            role,
            scenario,
            args.common.seed(),
            args.common.jitter,
            Ids {
                collections: counts.collections,
                shards: counts.shards,
                clusters: counts.clusters,
            },
        );
        std::thread::Builder::new()
            .name("logs".into())
            .spawn(move || generator.run())?;
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        tokio::select! {
            r = serve::serve(args.listen, model) => r,
            _ = shutdown() => Ok(()),
        }
    })
}

async fn shutdown() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

fn preview(args: Preview) -> Result<()> {
    let start = 1_800_000_000.0;
    let model = args.common.model(start)?;
    if let Some(path) = &args.body {
        let body = model
            .render(path, start + args.at)
            .with_context(|| format!("{} serves no {path}", args.common.role))?;
        print!("{body}");
        return Ok(());
    }
    let c = model.counts();
    println!(
        "{}: shards={} collections={} clusters={} replicas_per_cluster={}",
        args.common.role, c.shards, c.collections, c.clusters, c.replicas_per_cluster
    );
    let paths: Vec<String> = model.paths().map(str::to_string).collect();
    let (mut series, mut bytes) = (0, 0);
    for path in paths {
        let body = model.render(&path, start + args.at).expect("listed path");
        let n = body.lines().filter(|l| !l.starts_with('#')).count();
        println!("  {path}: {n} series, {} KiB", body.len() / 1024);
        series += n;
        bytes += body.len();
    }
    println!("  total: {series} series, {} KiB per scrape", bytes / 1024);
    Ok(())
}

fn fixture_gen(args: FixtureGen) -> Result<()> {
    let roles = if args.role.is_empty() {
        Role::ALL.to_vec()
    } else {
        args.role.clone()
    };
    let opts = anonymize::Options {
        max_exemplars: args.max_exemplars,
    };

    let captured: Vec<Captured> = match &args.from_dir {
        Some(dir) => roles
            .iter()
            .map(|r| {
                Ok(Captured {
                    role: *r,
                    source: dir.display().to_string(),
                    materialize: None,
                    captures: read_raw(dir, *r)?,
                })
            })
            .collect::<Result<_>>()?,
        None => live(&args, &roles)?,
    };

    let mut drift = false;
    let mut unreviewed = false;
    for Captured {
        role,
        source,
        materialize,
        captures,
    } in captured
    {
        let (fixtures, report) = anonymize::anonymize(role, &captures, materialize, &opts)?;
        print_report(role, &source, &fixtures, &report);
        let leaks: Vec<String> = fixtures.iter().flat_map(anonymize::leaks).collect();
        if !leaks.is_empty() {
            bail!(
                "{role}: anonymized fixtures still look identifying; nothing was written:\n  {}",
                leaks.join("\n  ")
            );
        }
        unreviewed |= !report.unreviewed.is_empty();
        for f in &fixtures {
            let file = args
                .out
                .join(role.as_str())
                .join(Fixture::file_name(&f.path));
            if args.check {
                drift |= check(&file, f)?;
            } else {
                std::fs::create_dir_all(file.parent().expect("has a parent"))?;
                std::fs::write(&file, f.to_text())
                    .with_context(|| format!("writing {}", file.display()))?;
            }
        }
    }
    if args.strict && unreviewed {
        bail!("labels outside the reviewed tables; classify them in anonymize.rs");
    }
    if drift {
        bail!("the capture's shape differs from the committed fixtures");
    }
    Ok(())
}

#[cfg(feature = "capture")]
fn live(args: &FixtureGen, roles: &[Role]) -> Result<Vec<Captured>> {
    use mz_monitoring_benchmark::capture;
    let window = std::time::Duration::from_secs_f64(parse_duration(&args.window)?);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let captured = runtime.block_on(capture::capture(&capture::Options {
        context: args.context.clone(),
        namespace: args.namespace.clone(),
        sql_auth_secret: args.sql_auth_secret.clone(),
        window,
        roles: roles.to_vec(),
    }))?;
    Ok(captured
        .into_iter()
        .map(|c| Captured {
            role: c.role,
            source: format!("{}/{}", c.namespace, c.pod),
            materialize: c.materialize,
            captures: c.captures,
        })
        .collect())
}

#[cfg(not(feature = "capture"))]
fn live(_: &FixtureGen, _: &[Role]) -> Result<Vec<Captured>> {
    let _ = parse_duration;
    bail!("built without the `capture` feature; use --from-dir")
}

/// One role's raw scrapes, and where they came from.
struct Captured {
    role: Role,
    source: String,
    materialize: Option<String>,
    captures: Vec<Capture>,
}

fn read_raw(dir: &Path, role: Role) -> Result<Vec<Capture>> {
    role.paths()
        .iter()
        .map(|path| {
            let stem = Fixture::file_name(path)
                .trim_end_matches(".tmpl")
                .to_string();
            let first = dir.join(role.as_str()).join(format!("{stem}.prom"));
            let second = dir.join(role.as_str()).join(format!("{stem}.2.prom"));
            let modified = std::fs::metadata(&first)
                .and_then(|m| m.modified())
                .with_context(|| format!("reading {}", first.display()))?;
            let at = modified
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_secs_f64());
            Ok(Capture {
                path: (*path).to_string(),
                first: std::fs::read_to_string(&first)?,
                second: std::fs::read_to_string(&second).ok(),
                window_secs: 15.0,
                at_unix_ms: at * 1000.0,
            })
        })
        .collect()
}

fn print_report(role: Role, source: &str, fixtures: &[Fixture], report: &Report) {
    println!("{role} (from {source})");
    for f in fixtures {
        let series: usize = f.families.iter().map(|x| x.series.len()).sum();
        println!(
            "  {}: {} families, {series} series",
            f.path,
            f.families.len()
        );
    }
    let exemplars: Vec<String> = report
        .exemplars
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    println!(
        "  exemplars: {}",
        if exemplars.is_empty() {
            "none".into()
        } else {
            exemplars.join(" ")
        }
    );
    if !report.standins.is_empty() {
        let s: Vec<String> = report
            .standins
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        println!("  stand-ins: {}", s.join(" "));
    }
    if report.over_cap > 0 {
        println!("  dropped {} series past the exemplar cap", report.over_cap);
    }
    if report.duplicates > 0 {
        println!(
            "  dropped {} series that anonymized to a duplicate",
            report.duplicates
        );
    }
    let mut by_label: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (family, label) in &report.unreviewed {
        by_label.entry(label).or_default().push(family);
    }
    for (label, families) in by_label {
        println!(
            "  unreviewed label {label:?} (values pattern-scrubbed only) on {}",
            families.join(", ")
        );
    }
}

/// The shape of a fixture: each family's type and label names. Values and
/// counts change with every capture; the shape changes only with Materialize.
fn shape(f: &Fixture) -> BTreeMap<&str, String> {
    f.families
        .iter()
        .filter(|fam| !fam.series.is_empty())
        .map(|fam| {
            let labels: BTreeSet<&str> = fam
                .series
                .iter()
                .flat_map(|s| s.labels.iter().map(|(k, _)| k.as_str()))
                .collect();
            let labels: Vec<&str> = labels.into_iter().collect();
            (
                fam.name.as_str(),
                format!("{} {{{}}}", fam.kind.as_str(), labels.join(",")),
            )
        })
        .collect()
}

/// Compare a capture with the committed fixture. A family whose type or label
/// names changed is drift. A family present on one side only is listed but is
/// not drift: the SQL exporter's families exist only while their query returns
/// rows, so one capture cannot tell a removed family from an idle one.
fn check(file: &Path, new: &Fixture) -> Result<bool> {
    let old = match std::fs::read_to_string(file) {
        Ok(text) => Fixture::parse(&text)?,
        Err(_) => {
            println!("  {}: no committed fixture", file.display());
            return Ok(true);
        }
    };
    let (old, new) = (shape(&old), shape(new));
    let mut drift = false;
    for (name, was) in &old {
        match new.get(name) {
            Some(now) if now != was => {
                println!("  ~ {name}: {was} -> {now}");
                drift = true;
            }
            Some(_) => {}
            None => println!("  ? {name} {was}: absent from this capture"),
        }
    }
    for (name, now) in &new {
        if !old.contains_key(name) {
            println!("  ? {name} {now}: not in the committed fixture");
        }
    }
    Ok(drift)
}
