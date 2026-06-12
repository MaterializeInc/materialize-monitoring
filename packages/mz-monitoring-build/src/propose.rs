// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `propose-bumps` — intended to run on merges to the default branch.
//!
//! For each changelog-enabled component with changes since its last release, it
//! recreates a `version-update/<component>` branch as a single commit atop the
//! base and opens (or force-updates) one PR per component. The version number is
//! not part of the branch name, so a branch is reused across versions; each
//! update force-pushes a fresh single commit (no reconciliation with the PR's
//! current state). A component with no changes is left untouched.
//!
//! The command is repository-agnostic: owner/repo and the base commit come from
//! the environment, so another repository can adopt it unchanged.
//!
//! Required environment:
//! - `CI=true` — the command refuses to run otherwise (set it to emulate CI).
//! - `GITHUB_TOKEN` — token with `contents: write` and `pull-requests: write`.
//! - `GITHUB_REPOSITORY` — `owner/repo` (provided by GitHub Actions).
//! - `GITHUB_SHA` — base commit the branches build on (the merge commit on the
//!   default branch); falls back to `git rev-parse HEAD`.
//!
//! `--dry-run` skips all GitHub calls and prints the plan (still requires
//! `CI=true`). GitHub releases/tags are handled by a separate command.

use anyhow::{Context, anyhow};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use std::path::PathBuf;

use crate::versioning::{
    ReleasePlan, latest_released, load_components, parse_changelog, plan_release, rev_parse,
};

const GITHUB_API: &str = "https://api.github.com";

/// Arguments for the `propose-bumps` command.
#[derive(clap::Args)]
pub struct ProposeBumpsArgs {
    /// Path to the component definitions.
    #[arg(long, default_value = "packages/components.yaml")]
    components: PathBuf,

    /// Path to the changelog.
    #[arg(long, default_value = "CHANGELOG.md")]
    changelog: PathBuf,

    /// `uv` lockfile to keep in sync with Python version bumps.
    #[arg(long, default_value = "uv.lock")]
    uv_lock: PathBuf,

    /// Base branch the version-update branches target and merge into.
    #[arg(long, default_value = "main")]
    base: String,

    /// Branch name prefix for the per-component update branches.
    #[arg(long, default_value = "version-update/")]
    branch_prefix: String,

    /// Collect changes up to this ref.
    #[arg(long, default_value = "HEAD")]
    until: String,

    /// Open PRs as drafts.
    #[arg(long)]
    draft: bool,

    /// Enable auto-merge on newly opened PRs (best-effort).
    #[arg(long)]
    automerge: bool,

    /// Print the plan without creating branches or PRs.
    #[arg(long)]
    dry_run: bool,
}

/// A single component's proposed version-update.
struct Proposal {
    name: String,
    branch: String,
    title: String,
    plan: ReleasePlan,
}

/// Build the PR body from a proposal's edit summary.
fn pr_body(p: &Proposal, base: &str) -> String {
    let mut body = format!(
        "Automated version-update for `{}` → {}.\n\nRecreated on each qualifying merge to `{base}`; do not push to this branch.\n\n## Edits\n\n",
        p.name,
        p.plan.released.changelog(),
    );
    for line in &p.plan.summary {
        body.push_str(&format!("- {line}\n"));
    }
    body
}

/// Main entrypoint for `propose-bumps`.
pub fn propose_bumps(args: ProposeBumpsArgs) -> anyhow::Result<()> {
    if std::env::var("CI").as_deref() != Ok("true") {
        anyhow::bail!("refusing to run outside CI; set CI=true to emulate");
    }

    let comps = load_components(&args.components)?;
    let changelog_text = std::fs::read_to_string(&args.changelog)
        .with_context(|| format!("reading {}", args.changelog.display()))?;
    let parsed = parse_changelog(&changelog_text)
        .with_context(|| format!("parsing {}", args.changelog.display()))?;

    let mut proposals: Vec<Proposal> = Vec::new();
    for (name, comp) in &comps {
        if !comp.changelog {
            continue;
        }
        // The since boundary is the tag for the component's latest released
        // version. Without a prior release (or its tag), skip: a first release
        // is bootstrapped manually rather than guessed.
        let Some(released) = latest_released(&comp.title, &parsed) else {
            eprintln!("skip {name}: no prior release; bootstrap its first release manually");
            continue;
        };
        let tag = format!("{name}/{}", released.changelog());
        if rev_parse(&tag).is_none() {
            eprintln!("skip {name}: release tag {tag} not found; create it to set the baseline");
            continue;
        }

        let Some(plan) = plan_release(
            &comps,
            name,
            &args.changelog,
            &args.uv_lock,
            &tag,
            &args.until,
        )?
        else {
            continue; // no changes — leave any existing branch/PR as-is
        };

        proposals.push(Proposal {
            branch: format!("{}{name}", args.branch_prefix),
            title: format!("Release {} {}", comp.title, plan.released.changelog()),
            name: name.clone(),
            plan,
        });
    }

    if proposals.is_empty() {
        println!("No components have changes to propose.");
        return Ok(());
    }

    if args.dry_run {
        for p in &proposals {
            println!(
                "\n# {} -> {} (branch {})",
                p.name,
                p.plan.released.changelog(),
                p.branch
            );
            for line in &p.plan.summary {
                println!("  {line}");
            }
        }
        return Ok(());
    }

    if args.automerge && args.draft {
        eprintln!(
            "note: --automerge is ignored while --draft is set (GitHub rejects auto-merge on draft PRs)"
        );
    }

    let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN not set")?;
    let repository = std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY not set")?;
    let (owner, repo) = repository
        .split_once('/')
        .ok_or_else(|| anyhow!("GITHUB_REPOSITORY must be owner/repo, got {repository:?}"))?;
    let base_sha = std::env::var("GITHUB_SHA")
        .ok()
        .or_else(|| rev_parse("HEAD"))
        .context("cannot resolve base commit (GITHUB_SHA or HEAD)")?;

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let gh = Gh::new(&token)?;
        for p in &proposals {
            gh.push_and_propose(owner, repo, &base_sha, &args, p)
                .await
                .with_context(|| format!("proposing {}", p.name))?;
            println!(
                "proposed {} {} on {}",
                p.name,
                p.plan.released.changelog(),
                p.branch
            );
        }
        anyhow::Ok(())
    })
}

/// A thin authenticated GitHub REST/GraphQL client.
struct Gh {
    client: Client,
}

impl Gh {
    fn new(token: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))?,
        );
        let client = Client::builder()
            .user_agent("mz-monitoring-build")
            .default_headers(headers)
            .build()?;
        Ok(Self { client })
    }

    async fn get(&self, path: &str) -> anyhow::Result<Value> {
        json_ok(
            self.client
                .get(format!("{GITHUB_API}{path}"))
                .send()
                .await?,
        )
        .await
    }

    async fn post(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        json_ok(
            self.client
                .post(format!("{GITHUB_API}{path}"))
                .json(body)
                .send()
                .await?,
        )
        .await
    }

    /// PATCH; returns whether the request succeeded (used for ref force-update,
    /// which 404s when the branch does not exist yet).
    async fn patch_ok(&self, path: &str, body: &Value) -> anyhow::Result<bool> {
        let resp = self
            .client
            .patch(format!("{GITHUB_API}{path}"))
            .json(body)
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    async fn graphql(&self, body: &Value) -> anyhow::Result<Value> {
        json_ok(
            self.client
                .post(format!("{GITHUB_API}/graphql"))
                .json(body)
                .send()
                .await?,
        )
        .await
    }

    /// Create a single commit with the proposal's files atop `base_sha`,
    /// force-update the branch ref to it, and ensure an open PR exists.
    async fn push_and_propose(
        &self,
        owner: &str,
        repo: &str,
        base_sha: &str,
        args: &ProposeBumpsArgs,
        p: &Proposal,
    ) -> anyhow::Result<()> {
        // Tree from the base commit, overriding changed files with inline content.
        let base_commit = self
            .get(&format!("/repos/{owner}/{repo}/git/commits/{base_sha}"))
            .await?;
        let base_tree = base_commit["tree"]["sha"]
            .as_str()
            .ok_or_else(|| anyhow!("base commit {base_sha} has no tree"))?;

        let entries: Vec<Value> = p
            .plan
            .files()
            .iter()
            .map(|(path, content)| {
                json!({
                    "path": path.to_string_lossy().replace('\\', "/"),
                    "mode": "100644",
                    "type": "blob",
                    "content": content,
                })
            })
            .collect();
        let tree = self
            .post(
                &format!("/repos/{owner}/{repo}/git/trees"),
                &json!({ "base_tree": base_tree, "tree": entries }),
            )
            .await?;
        let tree_sha = tree["sha"]
            .as_str()
            .ok_or_else(|| anyhow!("tree create failed"))?;

        let commit = self
            .post(
                &format!("/repos/{owner}/{repo}/git/commits"),
                &json!({ "message": p.title, "tree": tree_sha, "parents": [base_sha] }),
            )
            .await?;
        let commit_sha = commit["sha"]
            .as_str()
            .ok_or_else(|| anyhow!("commit create failed"))?;

        // Force the branch ref to the new commit, creating it if absent.
        let forced = self
            .patch_ok(
                &format!("/repos/{owner}/{repo}/git/refs/heads/{}", p.branch),
                &json!({ "sha": commit_sha, "force": true }),
            )
            .await?;
        if !forced {
            self.post(
                &format!("/repos/{owner}/{repo}/git/refs"),
                &json!({ "ref": format!("refs/heads/{}", p.branch), "sha": commit_sha }),
            )
            .await
            .context("creating branch ref")?;
        }

        // Open a PR if one is not already open for this branch.
        let existing = self
            .get(&format!(
                "/repos/{owner}/{repo}/pulls?state=open&head={owner}:{}",
                p.branch
            ))
            .await?;
        if existing.as_array().is_none_or(|prs| prs.is_empty()) {
            let pr = self
                .post(
                    &format!("/repos/{owner}/{repo}/pulls"),
                    &json!({
                        "title": p.title,
                        "head": p.branch,
                        "base": args.base,
                        "body": pr_body(p, &args.base),
                        "draft": args.draft,
                    }),
                )
                .await
                .context("creating PR")?;
            // GitHub rejects enabling auto-merge on a draft PR, so only attempt
            // it on non-draft PRs (see the note emitted in `propose_bumps`).
            if args.automerge
                && !args.draft
                && let Some(node_id) = pr["node_id"].as_str()
                && let Err(e) = self.enable_automerge(node_id).await
            {
                eprintln!("  auto-merge not enabled for {}: {e}", p.name);
            }
        }
        Ok(())
    }

    /// Enable auto-merge on a PR via GraphQL (best-effort).
    async fn enable_automerge(&self, pr_node_id: &str) -> anyhow::Result<()> {
        let query = "mutation($id: ID!) { \
            enablePullRequestAutoMerge(input: { pullRequestId: $id }) { clientMutationId } \
        }";
        let resp = self
            .graphql(&json!({ "query": query, "variables": { "id": pr_node_id } }))
            .await?;
        if let Some(errors) = resp.get("errors") {
            anyhow::bail!("{errors}");
        }
        Ok(())
    }
}

/// Parse a JSON response, turning non-2xx into an error with the body.
async fn json_ok(resp: reqwest::Response) -> anyhow::Result<Value> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("github {status}: {text}");
    }
    if text.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).with_context(|| format!("parsing github response: {text}"))
}
