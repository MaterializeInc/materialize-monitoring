// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `extract-metrics`: the Rust equivalent of
//! `python3 -m py_mzmon_lib.registry.query_cli docgen`.
//!
//! Loads the query registry from a directory of YAML files, renders every
//! metric query through the documentation [`TemplateContext`], extracts the
//! metrics each references, and writes the aggregated `metrics.yaml` artifact.

use std::path::PathBuf;

use anyhow::Context;
use mzmon_lib::query::docgen::extract_metric_docs;
use mzmon_lib::query::model::QueryEngine;
use mzmon_lib::query::registry::QueryRegistry;
use mzmon_lib::query::render::doc_context;

/// Arguments for the `extract-metrics` command.
#[derive(clap::Args)]
pub struct ExtractMetricsArgs {
    /// Directory containing query-registry YAML files.
    #[arg(long, default_value = "packages/queries")]
    source_dir: PathBuf,

    /// Output directory for the generated `metrics.yaml`.
    #[arg(long)]
    out_dir: PathBuf,

    /// Query engine to render for. Metric extraction is PromQL-based; other
    /// engines are accepted for parity with the Python CLI.
    #[arg(long, value_enum, default_value_t = EngineArg::Promql)]
    engine: EngineArg,
}

/// The query engine choices, mirroring the Python `--engine` flag.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum EngineArg {
    Promql,
    Datadog,
    Honeycomb,
    Logql,
}

impl EngineArg {
    fn to_engine(self) -> QueryEngine {
        match self {
            EngineArg::Promql => QueryEngine::PromQl,
            EngineArg::Datadog => QueryEngine::Datadog,
            EngineArg::Honeycomb => QueryEngine::Honeycomb,
            EngineArg::Logql => QueryEngine::LogQl,
        }
    }
}

/// Entry point for `mz-monitoring-build extract-metrics`.
pub fn extract_metrics(args: ExtractMetricsArgs) -> anyhow::Result<()> {
    let registry = QueryRegistry::from_directory(&args.source_dir)
        .with_context(|| format!("loading query registry from {}", args.source_dir.display()))?;
    eprintln!(
        "loaded {} queries from {}",
        registry.len(),
        args.source_dir.display()
    );

    let ctx = doc_context(&registry, args.engine.to_engine());
    let outcome = extract_metric_docs(&registry, &ctx);

    // A bad query is skipped, not fatal (mirrors the Python docgen try/except).
    for (id, err) in &outcome.errors {
        eprintln!("warning: skipped query {id}: {err}");
    }

    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating output dir {}", args.out_dir.display()))?;
    let out_path = args.out_dir.join("metrics.yaml");
    let yaml = outcome.to_yaml().context("serializing metrics.yaml")?;
    std::fs::write(&out_path, yaml).with_context(|| format!("writing {}", out_path.display()))?;

    eprintln!(
        "extracted {} metrics from {} queries -> {}",
        outcome.metrics.len(),
        registry.len(),
        out_path.display()
    );
    Ok(())
}
