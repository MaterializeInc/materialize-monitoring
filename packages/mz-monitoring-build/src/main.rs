// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use clap::{Parser, Subcommand};

mod gen_pipelines;

#[derive(Parser)]
#[command(
    name = "mz-monitoring-build",
    about = "Generate Materialize monitoring artifacts"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render alloy pipeline definitions into config.alloy files.
    GenPipelines(gen_pipelines::GenPipelinesArgs),
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::GenPipelines(args) => gen_pipelines::gen_pipelines(args),
    }
}
