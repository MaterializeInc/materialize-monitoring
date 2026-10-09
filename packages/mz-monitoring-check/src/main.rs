// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod check_queries;
mod terraform_render;

#[derive(Parser)]
#[command(
    name = "mz-monitoring-check",
    about = "Check Materialize monitoring inputs for schema and consistency issues"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate query-registry YAML files against the query schema.
    CheckQueries(check_queries::CheckQueriesArgs),
    /// Plan each Terraform example and assert its values land in the rendered chart.
    TerraformRender(terraform_render::TerraformRenderArgs),
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::CheckQueries(args) => match check_queries::check_queries(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Error: {err:?}");
                ExitCode::FAILURE
            }
        },
        Command::TerraformRender(args) => terraform_render::terraform_render(args),
    }
}
