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
//! `--flavor` picks the backend, and each backend has its consumers:
//!
//! * Grafana, `--format yaml`, into each
//!   `charts/materialize-monitoring-dashboards*/pre-rendered/dashboards/grafana/`,
//!   which that Helm chart globs by filename stem. There is one chart per Grafana
//!   folder, so the Makefile renders each with `--folder`.
//! * Grafana, `--format json`, into `docs/assets/dashboards/grafana/`, which the
//!   docsite offers for download. One tree for every folder.
//! * Datadog, JSON only, into `docs/assets/dashboards/datadog/`, which the
//!   docsite offers for download beside the Grafana file of the same stem. The
//!   files are named `datadog-<stem>.json`, because a release uploads both trees
//!   under their file names.
//!
//! Every tree is checked in, so the render is deterministic — see
//! `mz_dashboards::canonical`. Repeated runs over unchanged sources produce
//! byte-identical files, which is what keeps a regeneration reviewable.

use anyhow::Context;
use mz_dashboards::{datadog, grafana};
use mzmon_lib::grafana::folder::Folder;
use mzmon_lib::query::QueryRegistry;
use std::path::PathBuf;

/// Which dashboard backend to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Flavor {
    Grafana,
    Datadog,
}

/// Output format, mirroring `grafana::render::Format` at the CLI boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Yaml,
    Json,
}

impl From<OutputFormat> for grafana::render::Format {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Yaml => grafana::render::Format::Yaml,
            OutputFormat::Json => grafana::render::Format::Json,
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

    /// Dashboard backend to render.
    #[arg(long, value_enum, default_value = "grafana")]
    flavor: Flavor,

    /// Output format. For Grafana, `yaml` (the default) for the charts'
    /// pre-rendered trees and `json` for docs. Datadog renders only `json`.
    #[arg(long, value_enum)]
    format: Option<OutputFormat>,

    /// Specific dashboard(s) to render by filename stem. Defaults to all.
    #[arg(long)]
    dashboard: Vec<String>,

    /// Render only the Grafana dashboards filed in these folders. Defaults to all.
    ///
    /// Each `materialize-monitoring-dashboards*` chart carries one folder's
    /// dashboards, so this is what routes a render to its chart. Combines with
    /// `--dashboard` as an intersection. Datadog dashboards are filed in no
    /// folder, so `--flavor datadog` refuses it.
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

/// The stem and summary of every dashboard a flavor has, in artifact order.
fn catalog(flavor: Flavor) -> Vec<(&'static str, &'static str)> {
    match flavor {
        Flavor::Grafana => grafana::ALL.iter().map(|d| (d.name, d.summary)).collect(),
        Flavor::Datadog => datadog::ALL.iter().map(|d| (d.name, d.summary)).collect(),
    }
}

/// The format a flavor renders, given what was asked for.
fn resolve_format(flavor: Flavor, asked: Option<OutputFormat>) -> anyhow::Result<OutputFormat> {
    match (flavor, asked) {
        (Flavor::Grafana, asked) => Ok(asked.unwrap_or(OutputFormat::Yaml)),
        (Flavor::Datadog, None | Some(OutputFormat::Json)) => Ok(OutputFormat::Json),
        (Flavor::Datadog, Some(OutputFormat::Yaml)) => {
            anyhow::bail!(
                "Datadog dashboards render only as JSON: no chart carries them, so there is \
                 no YAML consumer"
            )
        }
    }
}

/// Render one dashboard of a flavor, already known to exist, to its artifact.
///
/// `None` when `folders` is non-empty and the dashboard is filed in none of them.
/// A Grafana dashboard is built before that check, because the folder is a
/// property of the built dashboard rather than of its registry entry.
fn render_one(
    flavor: Flavor,
    name: &str,
    format: OutputFormat,
    folders: &[String],
    args: &GenDashboardsArgs,
    registry: &QueryRegistry,
) -> anyhow::Result<Option<String>> {
    match flavor {
        Flavor::Grafana => {
            let dashboard = grafana::find(name).context("listed but not found")?;
            let options = grafana::Options {
                sql_metric_prefix: args.sql_metric_prefix.clone(),
            };
            let resource = (dashboard.render)(&options, registry)?;
            if !folders.is_empty() && !folders.iter().any(|folder| folder == resource.folder()) {
                return Ok(None);
            }
            Ok(Some(grafana::render::serialize(
                dashboard.name,
                &resource,
                format.into(),
            )?))
        }
        Flavor::Datadog => {
            let dashboard = datadog::find(name).context("listed but not found")?;
            let options = datadog::Options {
                sql_metric_prefix: args.sql_metric_prefix.clone(),
            };
            Ok(Some(datadog::render::render(
                dashboard, &options, registry,
            )?))
        }
    }
}

/// Main entrypoint for `gen-dashboards`.
pub fn gen_dashboards(args: GenDashboardsArgs) -> anyhow::Result<()> {
    let available = catalog(args.flavor);
    if args.list {
        for (name, summary) in &available {
            println!("{name:<16} {summary}");
        }
        return Ok(());
    }

    let format = resolve_format(args.flavor, args.format)?;
    let extension = match format {
        OutputFormat::Yaml => "yaml",
        OutputFormat::Json => "json",
    };
    let prefix = match args.flavor {
        Flavor::Grafana => "",
        Flavor::Datadog => datadog::render::FILE_PREFIX,
    };
    if args.flavor == Flavor::Datadog && !args.folder.is_empty() {
        anyhow::bail!("--folder selects Grafana folders, and Datadog dashboards are filed in none");
    }

    let selected: Vec<&'static str> = if args.dashboard.is_empty() {
        available.iter().map(|(name, _)| *name).collect()
    } else {
        let mut out = Vec::with_capacity(args.dashboard.len());
        for name in &args.dashboard {
            let found = available
                .iter()
                .find(|(stem, _)| stem == name)
                .with_context(|| {
                    let names: Vec<&str> = available.iter().map(|(stem, _)| *stem).collect();
                    format!(
                        "unknown {:?} dashboard {name:?}; available: {}",
                        args.flavor,
                        names.join(", ")
                    )
                })?;
            out.push(found.0);
        }
        out
    };

    let registry = QueryRegistry::from_directory(&args.queries_dir)
        .with_context(|| format!("loading query registry from {}", args.queries_dir.display()))?;

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
    for name in selected {
        let Some(rendered) = render_one(args.flavor, name, format, &folders, &args, &registry)
            .with_context(|| format!("rendering {name}"))?
        else {
            continue;
        };
        rendered_any = true;
        let path = output_dir.join(format!("{prefix}{name}.{extension}"));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grafana_defaults_to_yaml_and_takes_either() {
        assert_eq!(
            resolve_format(Flavor::Grafana, None).unwrap(),
            OutputFormat::Yaml
        );
        assert_eq!(
            resolve_format(Flavor::Grafana, Some(OutputFormat::Json)).unwrap(),
            OutputFormat::Json
        );
    }

    #[test]
    fn datadog_renders_json_and_refuses_yaml() {
        assert_eq!(
            resolve_format(Flavor::Datadog, None).unwrap(),
            OutputFormat::Json
        );
        assert!(resolve_format(Flavor::Datadog, Some(OutputFormat::Yaml)).is_err());
    }

    #[test]
    fn every_flavor_lists_its_dashboards() {
        assert!(!catalog(Flavor::Grafana).is_empty());
        assert!(!catalog(Flavor::Datadog).is_empty());
    }
}
