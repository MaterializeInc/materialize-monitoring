// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `gen-rules`: render the query registry's alerts into rule files for the
//! chart (`pre-rendered/rules/`).
//!
//! Writes one `groups:` document per registry file and ruler: PromQL rules into
//! `prometheus/`, which the chart installs as `PrometheusRule` resources for the
//! Thanos ruler, and LogQL rules into `loki/`, which it delivers to the Loki
//! ruler. Beside them, `_index.yaml` lists every rule of both kinds, and the
//! chart reads it to decide which install. The rendering and every check live in
//! [`mzmon_lib::query::rules`]; this only loads, reports and writes.
//!
//! The engine directories are owned by this command: files in them that the
//! current registry no longer produces are removed, so a deleted registry file
//! does not leave its rules shipping.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, bail};
use mzmon_lib::query::registry::QueryRegistry;
use mzmon_lib::query::rules::{RuleEngine, render_rules};

/// The index the chart selects from, beside the engine directories.
const INDEX_FILE: &str = "_index.yaml";

/// Arguments for the `gen-rules` command.
#[derive(clap::Args)]
pub struct GenRulesArgs {
    /// Directory containing query-registry YAML files.
    #[arg(long, default_value = "packages/queries")]
    source_dir: PathBuf,

    /// Output directory: the index is written here, and each engine's rule
    /// files into its own subdirectory (`prometheus/`, `loki/`).
    #[arg(long)]
    out_dir: PathBuf,
}

/// Entry point for `mz-monitoring-build gen-rules`.
pub fn gen_rules(args: GenRulesArgs) -> anyhow::Result<()> {
    let registry = QueryRegistry::from_directory(&args.source_dir)
        .with_context(|| format!("loading query registry from {}", args.source_dir.display()))?;

    let rules = match render_rules(&registry) {
        Ok(rules) => rules,
        Err(errors) => {
            for error in &errors {
                eprintln!("error: {error}");
            }
            let alerts: BTreeSet<&str> = errors.iter().map(|e| e.alert.as_str()).collect();
            bail!(
                "{} problem(s) in {} alert(s); nothing was written",
                errors.len(),
                alerts.len()
            );
        }
    };

    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;

    let mut file_count = 0;
    for engine in RuleEngine::ALL {
        let dir = args.out_dir.join(engine.dir());
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let mut written: BTreeSet<String> = BTreeSet::new();
        for source in rules.sources(engine) {
            let name = format!("{source}.yaml");
            let path = dir.join(&name);
            let contents = rules
                .rule_file_yaml(engine, source)
                .with_context(|| format!("serializing {}/{name}", engine.dir()))?;
            std::fs::write(&path, contents)
                .with_context(|| format!("writing {}", path.display()))?;
            written.insert(name);
        }
        file_count += written.len();
        remove_stale(&dir, &written)?;
    }

    let index_path = args.out_dir.join(INDEX_FILE);
    let index = rules.index_yaml().context("serializing the rule index")?;
    std::fs::write(&index_path, index)
        .with_context(|| format!("writing {}", index_path.display()))?;

    let default_count = rules.rules.iter().filter(|r| r.enabled_by_default).count();
    let log_count = rules
        .rules
        .iter()
        .filter(|r| r.engine == RuleEngine::LogQl)
        .count();
    eprintln!(
        "wrote {} rules ({} LogQL, {} enabled by default) in {} file(s) -> {}",
        rules.rules.len(),
        log_count,
        default_count,
        file_count,
        args.out_dir.display()
    );
    Ok(())
}

/// Remove the rule files in `dir` this registry no longer produces.
fn remove_stale(dir: &std::path::Path, written: &BTreeSet<String>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.ends_with(".yaml") && !written.contains(name) {
            std::fs::remove_file(&path)
                .with_context(|| format!("removing stale {}", path.display()))?;
            eprintln!("removed stale {}", path.display());
        }
    }
    Ok(())
}
