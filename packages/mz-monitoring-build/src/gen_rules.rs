// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `gen-rules`: render the query registry's alerts into Prometheus rule files
//! for the chart (`pre-rendered/rules/prometheus/`).
//!
//! Writes one `groups:` document per registry file that defines alerts, plus
//! `_index.yaml`, which the chart's `templates/alerts/prometheusrules.yaml`
//! reads to decide which rules install. The rendering and every check live in
//! [`mzmon_lib::query::rules`]; this only loads, reports and writes.
//!
//! The output directory is owned by this command: files in it that the current
//! registry no longer produces are removed, so a deleted registry file does not
//! leave its rules shipping.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, bail};
use mzmon_lib::query::registry::QueryRegistry;
use mzmon_lib::query::rules::render_rules;

/// The index the chart selects from, beside the rule files.
const INDEX_FILE: &str = "_index.yaml";

/// Arguments for the `gen-rules` command.
#[derive(clap::Args)]
pub struct GenRulesArgs {
    /// Directory containing query-registry YAML files.
    #[arg(long, default_value = "packages/queries")]
    source_dir: PathBuf,

    /// Output directory for the rendered rule files and their index.
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

    let mut written: BTreeSet<String> = BTreeSet::new();
    for source in rules.sources() {
        let name = format!("{source}.yaml");
        let path = args.out_dir.join(&name);
        let contents = rules
            .rule_file_yaml(source)
            .with_context(|| format!("serializing {name}"))?;
        std::fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
        written.insert(name);
    }
    let index_path = args.out_dir.join(INDEX_FILE);
    let index = rules.index_yaml().context("serializing the rule index")?;
    std::fs::write(&index_path, index)
        .with_context(|| format!("writing {}", index_path.display()))?;
    written.insert(INDEX_FILE.to_string());

    // Remove rule files this registry no longer produces.
    for entry in std::fs::read_dir(&args.out_dir)? {
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

    let default_count = rules.rules.iter().filter(|r| r.enabled_by_default).count();
    eprintln!(
        "wrote {} rules ({} enabled by default) in {} file(s) -> {}",
        rules.rules.len(),
        default_count,
        written.len() - 1,
        args.out_dir.display()
    );
    Ok(())
}
