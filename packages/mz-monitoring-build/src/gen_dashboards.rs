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
//! * `--format yaml` into each `charts/materialize-monitoring-dashboards*/pre-rendered/dashboards/grafana/`,
//!   which that Helm chart globs by filename stem. There is one chart per Grafana
//!   folder, so the Makefile renders each with `--folder`.
//! * `--format json` into `docs/assets/dashboards/grafana/`, which the docsite
//!   offers for download. One tree for every folder.
//!
//! Both trees are checked in, so the render is deterministic — see
//! `mz_dashboards::grafana::render`. Repeated runs over unchanged sources produce
//! byte-identical files, which is what keeps a regeneration reviewable.
//!
//! Every artifact the chart and the docsite ship comes from here, one file per
//! dashboard. There was briefly a second, `gcp-` prefixed set; it recorded nothing
//! but its own name once the clouds stopped differing in panel content, and was
//! dropped along with the `--cloud` and `--prefix` flags that produced it.

use anyhow::Context;
use mz_dashboards::grafana::{self, Options, render};
use mzmon_lib::grafana::folder::Folder;
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

/// A Grafana folder, mirroring `mzmon_lib::grafana::folder::Folder` at the CLI
/// boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum FolderArg {
    Materialize,
    Infra,
    #[value(name = "meta-o11y")]
    MetaO11y,
    Root,
}

impl From<FolderArg> for Folder {
    fn from(folder: FolderArg) -> Self {
        match folder {
            FolderArg::Materialize => Folder::Materialize,
            FolderArg::Infra => Folder::Infra,
            FolderArg::MetaO11y => Folder::MetaO11y,
            FolderArg::Root => Folder::Root,
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

    /// Specific dashboard(s) to render by filename stem. Defaults to all.
    #[arg(long)]
    dashboard: Vec<String>,

    /// Render only the dashboards filed in these Grafana folders. Defaults to all.
    ///
    /// Each `materialize-monitoring-dashboards*` chart carries one folder's
    /// dashboards, so this is what routes a render to its chart. Combines with
    /// `--dashboard` as an intersection.
    #[arg(long, value_enum)]
    folder: Vec<FolderArg>,

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

    let folders: Vec<String> = args
        .folder
        .iter()
        .map(|folder| Folder::from(*folder).to_string())
        .collect();

    let mut stale = Vec::new();
    let mut rendered_any = false;
    for dashboard in selected {
        // Built before filtering, because the folder is a property of the
        // built dashboard rather than of its registry entry.
        let resource = (dashboard.render)(&options, &registry)
            .with_context(|| format!("rendering {}", dashboard.name))?;
        if !folders.is_empty() && !folders.iter().any(|folder| folder == resource.folder()) {
            continue;
        }
        rendered_any = true;
        let rendered = render::serialize(dashboard.name, &resource, format)
            .with_context(|| format!("rendering {}", dashboard.name))?;
        let path = output_dir.join(format!("{}.{}", dashboard.name, format.extension()));

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

    // A folder that selects nothing would leave its chart empty, which the chart
    // only refuses at install time. A typo or an emptied folder is cheaper to
    // hear about here.
    if !rendered_any {
        anyhow::bail!(
            "no dashboard matched --folder {}{}",
            folders.join(", "),
            if args.dashboard.is_empty() {
                String::new()
            } else {
                format!(" and --dashboard {}", args.dashboard.join(", "))
            }
        );
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
