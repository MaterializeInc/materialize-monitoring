// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use clap::{Parser, Subcommand};

mod check_queries;

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
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::CheckQueries(args) => check_queries::check_queries(args),
    }
}
