// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `gen-scrape-configs`: transpile prometheus-operator Monitors into a single
//! classic Prometheus `scrape_configs` document.
//!
//! Mirrors `gen_pipelines`, but the output is one combined file (a `global`
//! block plus every monitor's jobs) rather than one file per input.

use anyhow::Context;
use mzmon_lib::scrape::classic::config::{GlobalConfig, ScrapeConfigDocument};
use mzmon_lib::scrape::transpile::Monitor;
use std::path::PathBuf;

/// Arguments for the `gen-scrape-configs` command.
#[derive(clap::Args)]
pub struct GenScrapeConfigsArgs {
    /// File to write the combined classic scrape_configs document into.
    #[arg(long)]
    output: PathBuf,

    /// Directory containing prometheus-operator Monitor YAML definitions.
    #[arg(long, default_value = "packages/prometheus-scrapers")]
    input_dir: PathBuf,

    /// Specific target(s) to include (by file stem). If omitted, includes all *.yaml.
    #[arg(long)]
    target: Vec<String>,

    /// `global.scrape_interval`.
    #[arg(long, default_value = "1m")]
    scrape_interval: String,

    /// `global.scrape_timeout`.
    #[arg(long, default_value = "10s")]
    scrape_timeout: String,

    /// `global.evaluation_interval`.
    #[arg(long, default_value = "1m")]
    evaluation_interval: String,
}

/// Discover Monitor targets by their `.yaml` stems, sorted for deterministic
/// combined output. (Skips files starting with `_`, like `gen-pipelines`.)
fn discover_targets(input_dir: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut targets = Vec::new();
    for entry in std::fs::read_dir(input_dir)
        .with_context(|| format!("reading input dir {}", input_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && !stem.starts_with('_')
        {
            targets.push(stem.to_string());
        }
    }
    targets.sort();
    Ok(targets)
}

/// Main entrypoint for `gen-scrape-configs`.
pub fn gen_scrape_configs(args: GenScrapeConfigsArgs) -> anyhow::Result<()> {
    let targets = if args.target.is_empty() {
        discover_targets(&args.input_dir)?
    } else {
        let mut t = args.target;
        t.sort();
        t
    };

    if targets.is_empty() {
        anyhow::bail!("no monitor targets found in {}", args.input_dir.display());
    }

    let mut monitors = Vec::with_capacity(targets.len());
    for target in &targets {
        let input = args.input_dir.join(format!("{target}.yaml"));
        let yaml = std::fs::read_to_string(&input)
            .with_context(|| format!("reading {}", input.display()))?;
        let monitor = Monitor::from_yaml_str(&yaml)
            .map_err(|e| anyhow::anyhow!("parsing {}:\n{e}", input.display()))?;
        monitors.push(monitor);
    }

    let global = GlobalConfig {
        evaluation_interval: args.evaluation_interval,
        scrape_interval: args.scrape_interval,
        scrape_timeout: args.scrape_timeout,
    };
    let document = ScrapeConfigDocument::from_monitors(global, &monitors)
        .map_err(|e| anyhow::anyhow!("transpiling monitors:\n{e}"))?;
    let rendered = document
        .to_yaml()
        .map_err(|e| anyhow::anyhow!("serializing scrape_configs:\n{e}"))?;

    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating output dir {}", parent.display()))?;
    }
    std::fs::write(&args.output, rendered)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!(
        "{} monitor(s) from {} -> {}",
        targets.len(),
        args.input_dir.display(),
        args.output.display()
    );
    Ok(())
}
