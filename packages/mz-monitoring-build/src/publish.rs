// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `publish-release` — intended to run when a `version-update/<component>` PR
//! merges (not on every push to the default branch).
//!
//! It reads the component's latest released section from `CHANGELOG.md`, creates
//! the `<component>/vX.Y.Z` tag at the merge commit, and publishes a GitHub
//! Release whose notes are that section. Idempotent: if the tag already exists
//! it does nothing. The release is created with `make_latest=false` since each
//! component is an independent stream.
//!
//! Required environment:
//! - `CI=true` — the command refuses to run otherwise (set it to emulate CI).
//! - `GITHUB_TOKEN` — token with `contents: write`.
//! - `GITHUB_REPOSITORY` — `owner/repo`.
//! - `GITHUB_SHA` — commit to tag; overridden by `--sha`, falls back to HEAD.

use anyhow::{Context, anyhow};
use serde_json::json;
use std::path::PathBuf;

use crate::github::Gh;
use crate::versioning::{latest_released, load_components, parse_changelog, rev_parse, section_of};

/// Arguments for the `publish-release` command.
#[derive(clap::Args)]
pub struct PublishReleaseArgs {
    /// Path to the component definitions.
    #[arg(long, default_value = "packages/components.yaml")]
    components: PathBuf,

    /// Path to the changelog.
    #[arg(long, default_value = "CHANGELOG.md")]
    changelog: PathBuf,

    /// Component being published (a key in components.yaml).
    #[arg(long)]
    component: String,

    /// Commit to tag (defaults to GITHUB_SHA, then HEAD).
    #[arg(long)]
    sha: Option<String>,

    /// Print what would be tagged/released without calling GitHub.
    #[arg(long)]
    dry_run: bool,
}

/// Main entrypoint for `publish-release`.
pub fn publish_release(args: PublishReleaseArgs) -> anyhow::Result<()> {
    if std::env::var("CI").as_deref() != Ok("true") {
        anyhow::bail!("refusing to run outside CI; set CI=true to emulate");
    }

    let comps = load_components(&args.components)?;
    let comp = comps
        .get(&args.component)
        .ok_or_else(|| anyhow!("unknown component {:?}", args.component))?;
    if !comp.changelog {
        anyhow::bail!("component {:?} has no changelog stream", args.component);
    }

    let changelog_text = std::fs::read_to_string(&args.changelog)
        .with_context(|| format!("reading {}", args.changelog.display()))?;
    let parsed = parse_changelog(&changelog_text)
        .with_context(|| format!("parsing {}", args.changelog.display()))?;

    let released = latest_released(&comp.title, &parsed).ok_or_else(|| {
        anyhow!(
            "no released version for {:?} in {}",
            args.component,
            args.changelog.display()
        )
    })?;
    let tag = format!("{}/{}", args.component, released.changelog());
    let heading = format!("## {} {}", comp.title, released.changelog());
    let section = section_of(&changelog_text, &heading);
    if section.is_empty() {
        anyhow::bail!("no changelog section found for {heading:?}");
    }
    // Release notes are the section without its heading (the release has a name).
    let notes = section
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    let name = format!("{} {}", comp.title, released.changelog());

    let sha = args
        .sha
        .clone()
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .or_else(|| rev_parse("HEAD"))
        .context("cannot resolve commit to tag (--sha / GITHUB_SHA / HEAD)")?;

    println!("Publishing {tag} at {sha}");
    if args.dry_run {
        println!("\n# {name}\n\n{notes}");
        return Ok(());
    }

    let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN not set")?;
    let repository = std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY not set")?;
    let (owner, repo) = repository
        .split_once('/')
        .ok_or_else(|| anyhow!("GITHUB_REPOSITORY must be owner/repo, got {repository:?}"))?;

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let gh = Gh::new(&token)?;
        // Idempotent: a pre-existing tag means this version was already published.
        if gh
            .exists(&format!("/repos/{owner}/{repo}/git/ref/tags/{tag}"))
            .await?
        {
            println!("tag {tag} already exists; nothing to publish");
            return anyhow::Ok(());
        }
        // Creating the release also creates the tag at `target_commitish`.
        gh.post(
            &format!("/repos/{owner}/{repo}/releases"),
            &json!({
                "tag_name": tag,
                "target_commitish": sha,
                "name": name,
                "body": notes,
                "draft": false,
                "make_latest": "false",
            }),
        )
        .await
        .context("creating release")?;
        println!("published release {tag}");
        anyhow::Ok(())
    })
}
