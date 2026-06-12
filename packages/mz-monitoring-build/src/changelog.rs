// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Changelog attribution for the component streams declared in
//! `packages/components.yaml`.
//!
//! This first increment is a read-only *reporter*: it enumerates the merged
//! PRs in a commit range, attributes each PR's changed paths to a component by
//! longest-prefix match against `content_paths`, and prints what each
//! changelog-enabled component would collect. It deliberately does not yet
//! write `CHANGELOG.md` or bump versions.
//!
//! Attribution works off each merge's own diff (`<merge>^1..<merge>`) rather
//! than `git log -- <path>`, which is unreliable here: history simplification
//! prunes merges, and the `crates/` -> `packages/` move means today's paths do
//! not match historical ones. Treat the hand-written `CHANGELOG.md` as the
//! authoritative baseline and attribute forward from a release ref.
//!
//! Cascade (a dependency also touched by a PR) is *reported* as a
//! `[cascade: ...]` annotation. The write step (next increment) will turn that
//! into an explicit "Updated <dep> to <version>" entry on the dependent.

use anyhow::{Context, anyhow};
use indexmap::IndexMap;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

/// Arguments for the `changelog` command.
#[derive(clap::Args)]
pub struct ChangelogArgs {
    /// Path to the component definitions.
    #[arg(long, default_value = "packages/components.yaml")]
    components: PathBuf,

    /// Git ref to start from (exclusive) — e.g. a release tag or commit.
    #[arg(long)]
    since: String,

    /// Git ref to collect up to (inclusive).
    #[arg(long, default_value = "HEAD")]
    until: String,

    /// Print the per-PR path -> component breakdown.
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, Deserialize)]
struct ComponentsFile {
    components: IndexMap<String, Component>,
}

#[derive(Debug, Deserialize)]
struct Component {
    #[serde(default)]
    changelog: bool,
    title: String,
    /// Files whose version field is rewritten on bump. Consumed by the write
    /// step (not yet wired), so allowed to be unread here.
    #[serde(default)]
    #[allow(dead_code)]
    version_paths: Vec<String>,
    #[serde(default)]
    content_paths: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

/// A merged pull request discovered on the first-parent history.
struct PrMerge {
    hash: String,
    number: Option<u64>,
    branch: Option<String>,
    /// The PR title — GitHub puts it in the merge commit body (`%b`).
    title: Option<String>,
    paths: Vec<String>,
}

impl PrMerge {
    /// A short, human-friendly label for output: `#15 <title>`, falling back to
    /// the branch or short hash when the title is absent.
    fn label(&self) -> String {
        let id = match self.number {
            Some(n) => format!("#{n}"),
            None => self.hash.chars().take(9).collect(),
        };
        match (&self.title, &self.branch) {
            (Some(t), _) => format!("{id} {t}"),
            (None, Some(b)) => format!("{id} ({b})"),
            (None, None) => id,
        }
    }
}

/// Run `git` with the given args and return trimmed stdout.
fn git(args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("running git {args:?}"))?;
    if !out.status.success() {
        return Err(anyhow!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8(out.stdout)?.trim_end().to_string())
}

/// Parse `#<n>` and the `from <branch>` tail out of a merge subject like
/// "Merge pull request #15 from MaterializeInc/heather/sm-cloud-converge".
fn parse_subject(subject: &str) -> (Option<u64>, Option<String>) {
    let number = subject.split('#').nth(1).and_then(|rest| {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    });
    let branch = subject
        .split_once(" from ")
        .map(|(_, b)| b.trim().to_string());
    (number, branch)
}

/// The files a merge introduced, relative to its first parent.
fn changed_paths(hash: &str) -> anyhow::Result<Vec<String>> {
    let parent = format!("{hash}^1");
    let out = git(&["diff", "--name-only", &parent, hash])?;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// Collect first-parent merge commits in `since..until`.
fn collect_merges(since: &str, until: &str) -> anyhow::Result<Vec<PrMerge>> {
    let range = format!("{since}..{until}");
    // \x1f separates fields, \x1e separates records — both rare in commit text.
    let raw = git(&[
        "log",
        "--first-parent",
        "--merges",
        &range,
        "--format=%H%x1f%s%x1f%b%x1e",
    ])?;

    let mut merges = Vec::new();
    for record in raw.split('\u{1e}') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        let mut fields = record.splitn(3, '\u{1f}');
        let hash = fields.next().unwrap_or("").trim();
        let subject = fields.next().unwrap_or("").trim();
        let body = fields.next().unwrap_or("");
        let (number, branch) = parse_subject(subject);
        // The PR title is the first non-empty line of the merge body.
        let title = body
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string);
        let paths = changed_paths(hash)?;
        merges.push(PrMerge {
            hash: hash.to_string(),
            number,
            branch,
            title,
            paths,
        });
    }
    Ok(merges)
}

/// The component that owns `file`: the one whose matching `content_path` is the
/// most specific (longest). Ties resolve to declaration order.
fn owner_of<'a>(file: &str, comps: &'a IndexMap<String, Component>) -> Option<&'a str> {
    let mut best: Option<(usize, &str)> = None;
    for (name, c) in comps {
        for cp in &c.content_paths {
            let cp = cp.trim_end_matches('/');
            let matches = file == cp || file.starts_with(&format!("{cp}/"));
            if matches && best.is_none_or(|(blen, _)| cp.len() > blen) {
                best = Some((cp.len(), name.as_str()));
            }
        }
    }
    best.map(|(_, name)| name)
}

/// One PR with its changed paths grouped by owning component.
struct Attributed<'a> {
    pr: &'a PrMerge,
    owners: IndexMap<String, Vec<String>>,
    orphans: Vec<String>,
}

/// Main entrypoint for the `changelog` command (report-only).
pub fn changelog(args: ChangelogArgs) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(&args.components)
        .with_context(|| format!("reading {}", args.components.display()))?;
    let file: ComponentsFile = serde_yaml_ng::from_str(&text)
        .with_context(|| format!("parsing {}", args.components.display()))?;
    let comps = &file.components;

    let merges = collect_merges(&args.since, &args.until)?;
    println!(
        "Range {}..{}: {} merged PR(s)",
        args.since,
        args.until,
        merges.len()
    );

    let attributed: Vec<Attributed> = merges
        .iter()
        .map(|pr| {
            let mut owners: IndexMap<String, Vec<String>> = IndexMap::new();
            let mut orphans = Vec::new();
            for f in &pr.paths {
                match owner_of(f, comps) {
                    Some(name) => owners.entry(name.to_string()).or_default().push(f.clone()),
                    None => orphans.push(f.clone()),
                }
            }
            Attributed {
                pr,
                owners,
                orphans,
            }
        })
        .collect();

    if args.verbose {
        println!("\n== Per-PR attribution ==");
        for a in &attributed {
            let touched: Vec<&str> = a.owners.keys().map(String::as_str).collect();
            println!("  {} -> [{}]", a.pr.label(), touched.join(", "));
            for (name, paths) in &a.owners {
                println!("      {name}: {}", paths.join(", "));
            }
            if !a.orphans.is_empty() {
                println!("      (unowned): {}", a.orphans.join(", "));
            }
        }
    }

    println!("\n== Per changelog component ==");
    for (name, c) in comps {
        if !c.changelog {
            continue;
        }
        println!("\n## {} ({name})", c.title);
        let mut any = false;
        for a in &attributed {
            if a.owners.contains_key(name) {
                any = true;
                // Only cascade on dependencies that carry a changelog/version of
                // their own — a changelog-less dep (e.g. repo-common) has no
                // version to record an "Updated <dep> to vX" entry against.
                let cascade: Vec<&str> = c
                    .dependencies
                    .iter()
                    .filter(|d| {
                        a.owners.contains_key(d.as_str())
                            && comps.get(d.as_str()).is_some_and(|dc| dc.changelog)
                    })
                    .map(String::as_str)
                    .collect();
                if cascade.is_empty() {
                    println!("  - {}", a.pr.label());
                } else {
                    println!("  - {}  [cascade: {}]", a.pr.label(), cascade.join(", "));
                }
            }
        }
        if !any {
            println!("  (no changes in range)");
        }
    }

    let orphan_prs: Vec<&Attributed> = attributed
        .iter()
        .filter(|a| !a.orphans.is_empty())
        .collect();
    if !orphan_prs.is_empty() {
        println!("\n== Unattributed paths (owned by no component) ==");
        for a in orphan_prs {
            println!("  {}: {}", a.pr.label(), a.orphans.join(", "));
        }
    }

    Ok(())
}
