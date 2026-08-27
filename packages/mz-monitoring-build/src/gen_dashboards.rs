// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `gen-dashboards`: render the dashboards under `packages/dashboards/` into the
//! artifacts that ship.
//!
//! Two consumers, one command:
//!
//! * `--format yaml` into `charts/materialize-monitoring/pre-rendered/dashboards/grafana/`,
//!   which the Helm chart globs by filename stem.
//! * `--format json` into `docs/assets/dashboards/grafana/`, which the docsite
//!   offers for download.
//!
//! Both trees are checked in, so the render is deterministic — see
//! `mz_dashboards::grafana::render`. Repeated runs over unchanged sources produce
//! byte-identical files, which is what keeps a regeneration reviewable.
//!
//! This replaces `python -m dashboards.render` for every artifact the chart and the
//! docsite ship, including the `gcp-` prefixed one.

use anyhow::Context;
use mz_dashboards::grafana::{self, Cloud, Options, render};
use mzmon_lib::query::QueryRegistry;
use std::path::PathBuf;

/// Output format, mirroring `render::Format` at the CLI boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Yaml,
    Json,
}

impl From<OutputFormat> for render::Format {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Yaml => render::Format::Yaml,
            OutputFormat::Json => render::Format::Json,
        }
    }
}

/// Cloud variant, mirroring `grafana::Cloud`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CloudTarget {
    Generic,
    Gcp,
}

impl From<CloudTarget> for Cloud {
    fn from(target: CloudTarget) -> Self {
        match target {
            CloudTarget::Generic => Cloud::Generic,
            CloudTarget::Gcp => Cloud::Gcp,
        }
    }
}

/// Arguments for the `gen-dashboards` command.
#[derive(clap::Args)]
pub struct GenDashboardsArgs {
    /// Directory to write the rendered dashboards into.
    ///
    /// Not needed with `--list`, which only enumerates what is available.
    #[arg(long, required_unless_present = "list")]
    output_dir: Option<PathBuf>,

    /// Output format. `yaml` for the chart's pre-rendered tree, `json` for docs.
    #[arg(long, value_enum, default_value = "yaml")]
    format: OutputFormat,

    /// Filename prefix, for rendering a variant alongside the default
    /// (`--cloud gcp --prefix gcp-`).
    #[arg(long, default_value = "")]
    prefix: String,

    /// Specific dashboard(s) to render by filename stem. Defaults to all.
    #[arg(long)]
    dashboard: Vec<String>,

    /// Cloud metric surface to target.
    ///
    /// Reaches only the `target-cloud` annotation today. The variants used to
    /// differ in panel content, back when GKE's managed collectors shipped a
    /// reduced allowlist; the gateway now scrapes the kubelet's cAdvisor directly,
    /// so every cloud gets the same panels.
    #[arg(long, value_enum, default_value = "generic")]
    cloud: CloudTarget,

    /// SQL-exporter metric prefix. `mz_` self-managed, `v2_mz_` on Cloud.
    #[arg(long, default_value = "mz_")]
    sql_metric_prefix: String,

    /// Directory containing query-registry YAML files.
    ///
    /// Panels take their expressions and descriptions from the registry, so this
    /// is a source input, not a lookup path.
    #[arg(long, default_value = "packages/queries")]
    queries_dir: PathBuf,

    /// List the available dashboards and exit.
    #[arg(long)]
    list: bool,

    /// Render and compare against what is on disk without writing, exiting
    /// non-zero if they differ.
    ///
    /// For CI: proves the checked-in artifacts match their sources, which is the
    /// thing that silently rots when a generator and its output live in one repo.
    #[arg(long)]
    check: bool,
}

/// Main entrypoint for `gen-dashboards`.
pub fn gen_dashboards(args: GenDashboardsArgs) -> anyhow::Result<()> {
    if args.list {
        for dashboard in grafana::ALL {
            println!("{:<16} {}", dashboard.name, dashboard.summary);
        }
        return Ok(());
    }

    let selected: Vec<&'static grafana::Renderable> = if args.dashboard.is_empty() {
        grafana::ALL.iter().collect()
    } else {
        let mut out = Vec::with_capacity(args.dashboard.len());
        for name in &args.dashboard {
            out.push(grafana::find(name).with_context(|| {
                let available: Vec<&str> = grafana::ALL.iter().map(|d| d.name).collect();
                format!(
                    "unknown dashboard {name:?}; available: {}",
                    available.join(", ")
                )
            })?);
        }
        out
    };

    let registry = QueryRegistry::from_directory(&args.queries_dir)
        .with_context(|| format!("loading query registry from {}", args.queries_dir.display()))?;

    let options = Options {
        cloud: args.cloud.into(),
        sql_metric_prefix: args.sql_metric_prefix.clone(),
    };
    let format = render::Format::from(args.format);

    // `required_unless_present = "list"` has already rejected the missing case, and
    // the `--list` path returned above.
    let output_dir = args
        .output_dir
        .as_ref()
        .context("--output-dir is required unless --list is given")?;

    if !args.check {
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("creating output dir {}", output_dir.display()))?;
    }

    let mut stale = Vec::new();
    for dashboard in selected {
        let rendered = render::render(dashboard, &options, &registry, format)
            .with_context(|| format!("rendering {}", dashboard.name))?;
        let path = output_dir.join(format!(
            "{}{}.{}",
            args.prefix,
            dashboard.name,
            format.extension()
        ));

        if args.check {
            match std::fs::read_to_string(&path) {
                Ok(on_disk) if on_disk == rendered => {
                    println!("ok      {}", path.display());
                }
                Ok(_) => {
                    stale.push(format!("{} differs from its source", path.display()));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    stale.push(format!("{} is missing", path.display()));
                }
                Err(e) => {
                    return Err(e).with_context(|| format!("reading {}", path.display()));
                }
            }
            continue;
        }

        std::fs::write(&path, &rendered).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote   {}", path.display());
    }

    if !stale.is_empty() {
        anyhow::bail!(
            "{} checked-in dashboard artifact(s) are out of date:\n  {}\n\nRun `make dashboards`.",
            stale.len(),
            stale.join("\n  ")
        );
    }
    Ok(())
}
