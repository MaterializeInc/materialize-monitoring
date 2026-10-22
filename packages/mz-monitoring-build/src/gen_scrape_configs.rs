// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `gen-scrape-configs`: render the prometheus-operator Monitors under
//! `packages/prometheus-scrapers/` into one or more consumer formats.
//!
//! Input is always prometheus-operator (auto-detected by `kind`). The output
//! format(s) are chosen with `--format` (repeatable); each writes prefixed files
//! into `--output-dir`:
//!
//! * `classic` — one combined classic Prometheus `scrape_configs` document
//!   (`classic/scrape_config.yaml`), for plain Prometheus / Agent.
//! * `prometheus-operator` — the source manifests, one per monitor
//!   (`prometheus-operator/<stem>.yaml`). A monitor the label vocabulary applies
//!   to is re-serialized with its rules added, which drops the source's
//!   comments; every other monitor is a validated passthrough.
//! * `gmp` — Google Managed Prometheus `PodMonitoring` / `ClusterPodMonitoring`
//!   (`gmp/<resource-name>.yaml`), one resource **per endpoint** (GMP requires
//!   unique ports per resource). Kinds with no GMP equivalent (ServiceMonitor,
//!   ScrapeConfig) are skipped with a logged note.
//!
//! Every format renders the label vocabulary, `labels.yaml` in the input
//! directory, into the monitors it applies to. See [`LabelVocabulary`].

use anyhow::Context;
use mzmon_lib::scrape::classic::config::{GlobalConfig, ScrapeConfigDocument};
use mzmon_lib::scrape::labels::LabelVocabulary;
use mzmon_lib::scrape::transpile::Monitor;
use std::path::PathBuf;

/// Output formats `gen-scrape-configs` can render. The clap kebab spelling is
/// `classic` / `prometheus-operator` / `gmp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Classic,
    PrometheusOperator,
    Gmp,
}

impl OutputFormat {
    /// Filename prefix for this format (the docs shortcode keys off it).
    fn prefix(self) -> &'static str {
        match self {
            OutputFormat::Classic => "classic/",
            OutputFormat::PrometheusOperator => "prometheus-operator/",
            OutputFormat::Gmp => "gmp/",
        }
    }
}

/// Arguments for the `gen-scrape-configs` command.
#[derive(clap::Args)]
pub struct GenScrapeConfigsArgs {
    /// Directory to write the rendered, format-prefixed files into.
    #[arg(long)]
    output_dir: PathBuf,

    /// Directory containing prometheus-operator Monitor YAML definitions.
    #[arg(long, default_value = "packages/prometheus-scrapers")]
    input_dir: PathBuf,

    /// Specific target(s) to include (by file stem). If omitted, includes all *.yaml.
    #[arg(long)]
    target: Vec<String>,

    /// Output format(s) to render (repeatable). Defaults to `classic`.
    #[arg(long, value_enum)]
    format: Vec<OutputFormat>,

    /// `global.scrape_interval` (classic only).
    ///
    /// 30s matches `GMP_DEFAULT_INTERVAL`, the Alloy gateway's
    /// `default_scrape_interval`, and the agent's cAdvisor interval, so every
    /// scrape surface this repo generates agrees.
    #[arg(long, default_value = "30s")]
    scrape_interval: String,

    /// `global.scrape_timeout` (classic only).
    #[arg(long, default_value = "10s")]
    scrape_timeout: String,

    /// `global.evaluation_interval` (classic only).
    #[arg(long, default_value = "1m")]
    evaluation_interval: String,
}

/// The label vocabulary's file stem in the input directory. It is not a monitor.
const VOCABULARY_STEM: &str = "labels";

/// Discover Monitor targets by their `.yaml` stems, sorted for deterministic
/// output. (Skips files starting with `_`, like `gen-pipelines`, and the label
/// vocabulary.)
fn discover_targets(input_dir: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut targets = Vec::new();
    for entry in std::fs::read_dir(input_dir)
        .with_context(|| format!("reading input dir {}", input_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && !stem.starts_with('_')
            && stem != VOCABULARY_STEM
        {
            targets.push(stem.to_string());
        }
    }
    targets.sort();
    Ok(targets)
}

/// A parsed input monitor plus the YAML the `prometheus-operator` format writes
/// for it: the source verbatim, or the re-serialized monitor with the label
/// vocabulary's rules added.
struct ParsedMonitor {
    stem: String,
    yaml: String,
    monitor: Monitor,
}

/// Main entrypoint for `gen-scrape-configs`.
pub fn gen_scrape_configs(args: GenScrapeConfigsArgs) -> anyhow::Result<()> {
    let formats = if args.format.is_empty() {
        vec![OutputFormat::Classic]
    } else {
        args.format.clone()
    };

    let targets = if args.target.is_empty() {
        discover_targets(&args.input_dir)?
    } else {
        let mut t = args.target.clone();
        t.sort();
        t
    };
    if targets.is_empty() {
        anyhow::bail!("no monitor targets found in {}", args.input_dir.display());
    }

    let vocabulary_path = args.input_dir.join(format!("{VOCABULARY_STEM}.yaml"));
    let vocabulary = LabelVocabulary::from_yaml_str(
        &std::fs::read_to_string(&vocabulary_path)
            .with_context(|| format!("reading {}", vocabulary_path.display()))?,
    )
    .map_err(|e| anyhow::anyhow!("parsing {}:\n{e}", vocabulary_path.display()))?;

    // Parse + validate every input once, adding the vocabulary's rules where it
    // applies.
    let mut parsed = Vec::with_capacity(targets.len());
    for stem in &targets {
        let input = args.input_dir.join(format!("{stem}.yaml"));
        let source = std::fs::read_to_string(&input)
            .with_context(|| format!("reading {}", input.display()))?;
        let mut value: serde_json::Value = mzmon_lib::yaml::from_str(&source)
            .map_err(|e| anyhow::anyhow!("parsing {}:\n{e}", input.display()))?;
        let labeled = vocabulary.add_to_monitor(&mut value).map_err(|e| {
            anyhow::anyhow!("adding the label vocabulary to {}:\n{e}", input.display())
        })?;
        let yaml = if labeled {
            format!(
                "# Generated by `mz-monitoring-build gen-scrape-configs` from {stem}.yaml and\n\
                 # {VOCABULARY_STEM}.yaml in packages/prometheus-scrapers/. Edit those, not this file.\n{}",
                mzmon_lib::yaml::to_string(&value)
                    .with_context(|| format!("serializing {stem} with its label rules"))?
            )
        } else {
            source
        };
        let monitor = Monitor::from_value(value)
            .map_err(|e| anyhow::anyhow!("parsing {}:\n{e}", input.display()))?;
        parsed.push(ParsedMonitor {
            stem: stem.clone(),
            yaml,
            monitor,
        });
    }

    std::fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("creating output dir {}", args.output_dir.display()))?;

    for format in &formats {
        match format {
            OutputFormat::Classic => render_classic(&args, &parsed)?,
            OutputFormat::PrometheusOperator => render_prometheus_operator(&args, &parsed)?,
            OutputFormat::Gmp => render_gmp(&args, &parsed)?,
        }
    }
    Ok(())
}

/// One combined classic document (`global` + every monitor's jobs).
fn render_classic(args: &GenScrapeConfigsArgs, parsed: &[ParsedMonitor]) -> anyhow::Result<()> {
    let monitors: Vec<Monitor> = parsed.iter().map(|p| p.monitor.clone()).collect();
    let global = GlobalConfig {
        evaluation_interval: args.evaluation_interval.clone(),
        scrape_interval: args.scrape_interval.clone(),
        scrape_timeout: args.scrape_timeout.clone(),
    };
    let document = ScrapeConfigDocument::from_monitors(global, &monitors)
        .map_err(|e| anyhow::anyhow!("transpiling monitors to classic:\n{e}"))?;
    let rendered = document
        .to_yaml()
        .map_err(|e| anyhow::anyhow!("serializing classic scrape_configs:\n{e}"))?;

    let dir = args.output_dir.join(OutputFormat::Classic.prefix());
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating classic output dir {}", dir.display()))?;
    let output = dir.join("scrape_config.yaml");
    std::fs::write(&output, rendered).with_context(|| format!("writing {}", output.display()))?;
    println!(
        "classic: {} monitor(s) -> {}",
        parsed.len(),
        output.display()
    );
    Ok(())
}

/// One file per source monitor: the source verbatim, comments preserved, or the
/// re-serialized monitor where the label vocabulary added rules.
fn render_prometheus_operator(
    args: &GenScrapeConfigsArgs,
    parsed: &[ParsedMonitor],
) -> anyhow::Result<()> {
    let prefix = OutputFormat::PrometheusOperator.prefix();
    let dir = args.output_dir.join(prefix);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating prometheus-operator output dir {}", dir.display()))?;
    for p in parsed {
        let output = dir.join(format!("{}.yaml", p.stem));
        std::fs::write(&output, &p.yaml)
            .with_context(|| format!("writing {}", output.display()))?;
    }
    println!(
        "prometheus-operator: {} monitor(s) -> {}/{}*.yaml",
        parsed.len(),
        args.output_dir.display(),
        prefix
    );
    Ok(())
}

/// PodMonitoring / ClusterPodMonitoring, one resource per endpoint (GMP requires
/// unique ports within a resource). Files are named by the GMP resource name, so
/// a multi-endpoint PodMonitor fans out to several `<name>-<suffix>.yaml`. Kinds
/// with no GMP equivalent are skipped with a logged note (not an error).
fn render_gmp(args: &GenScrapeConfigsArgs, parsed: &[ParsedMonitor]) -> anyhow::Result<()> {
    let prefix = OutputFormat::Gmp.prefix();
    let dir = args.output_dir.join(prefix);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating GMP output dir {}", dir.display()))?;
    let mut written = 0usize;
    for p in parsed {
        let resources = p
            .monitor
            .to_gmp()
            .map_err(|e| anyhow::anyhow!("converting {} to GMP:\n{e}", p.stem))?;
        if resources.is_empty() {
            eprintln!("gmp: skipping {} (no GMP equivalent for this kind)", p.stem);
            continue;
        }
        for resource in &resources {
            let name = resource
                .metadata
                .name
                .clone()
                .unwrap_or_else(|| p.stem.clone());
            let rendered = mzmon_lib::yaml::to_string(resource)
                .with_context(|| format!("serializing GMP resource {name}"))?;
            let output = dir.join(format!("{name}.yaml"));
            std::fs::write(&output, rendered)
                .with_context(|| format!("writing {}", output.display()))?;
            written += 1;
        }
    }
    println!(
        "gmp: {written} resource(s) -> {}/{}*.yaml",
        args.output_dir.display(),
        prefix
    );
    Ok(())
}
