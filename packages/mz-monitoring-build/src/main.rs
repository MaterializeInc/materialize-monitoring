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
mod github;
mod propose;
mod publish;
mod versioning;

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
    /// Report which merged PRs each component changelog would collect.
    Changelog(versioning::ChangelogArgs),
    /// Generate a version-update PR's changelog + version bumps for a component.
    Release(versioning::ReleaseArgs),
    /// Create/update one version-update PR per changed component (runs in CI).
    ProposeBumps(propose::ProposeBumpsArgs),
    /// Tag and publish a GitHub Release for a merged version-update (runs in CI).
    PublishRelease(publish::PublishReleaseArgs),
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::GenPipelines(args) => gen_pipelines::gen_pipelines(args),
        Command::Changelog(args) => versioning::changelog(args),
        Command::Release(args) => versioning::release(args),
        Command::ProposeBumps(args) => propose::propose_bumps(args),
        Command::PublishRelease(args) => publish::publish_release(args),
    }
}
