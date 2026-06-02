// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use anyhow::Context;
use mzmon_lib::alloy::pipeline::Pipeline;
use std::path::PathBuf;

/// Arguments for gen-pipelines command
#[derive(clap::Args)]
pub struct GenPipelinesArgs {
    /// Directory to write rendered .alloy files into.
    #[arg(long)]
    output_dir: PathBuf,

    /// Directory containing pipeline YAML definitions.
    #[arg(long, default_value = "packages/alloy-pipelines")]
    input_dir: PathBuf,

    /// Specific target(s) to render (by file stem). If omitted, renders all *.yaml.
    #[arg(long)]
    target: Vec<String>,
}

/// Discover pipeline targets by their stems in the input directory.
fn discover_targets(input_dir: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut targets = Vec::new();
    for entry in std::fs::read_dir(input_dir)
        .with_context(|| format!("reading input dir {}", input_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        // only consider .yaml files
        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            // do not include files starting with _
            if !stem.starts_with('_') {
                targets.push(stem.to_string());
            }
        }
    }
    Ok(targets)
}

/// Main entrypoint for the `gen-pipelines` command which renders alloy pipelines
pub fn gen_pipelines(args: GenPipelinesArgs) -> anyhow::Result<()> {
    let targets = if args.target.is_empty() {
        discover_targets(&args.input_dir)? // glob *.yaml stems
    } else {
        args.target
    };

    if targets.is_empty() {
        anyhow::bail!("no pipeline targets found in {}", args.input_dir.display());
    }

    std::fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("creating output dir {}", args.output_dir.display()))?;

    for target in &targets {
        let input = args.input_dir.join(format!("{target}.yaml"));
        let output = args.output_dir.join(format!("{target}.alloy"));

        let yaml = std::fs::read_to_string(&input)
            .with_context(|| format!("reading {}", input.display()))?;
        let pipeline = Pipeline::from_yaml_str(&yaml)
            .with_context(|| format!("parsing {}", input.display()))?;
        let rendered = pipeline
            .render()
            .with_context(|| format!("rendering {}", input.display()))?;

        std::fs::write(&output, rendered)
            .with_context(|| format!("writing {}", output.display()))?;
        println!("{} -> {}", input.display(), output.display());
    }
    Ok(())
}
