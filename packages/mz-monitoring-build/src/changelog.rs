// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Changelog generation for the component streams declared in
//! `packages/components.yaml`.
//!
//! Merged PRs in a commit range are attributed to a component by longest-prefix
//! match against `content_paths`. Attribution works off each merge's own diff
//! (`<merge>^1..<merge>`) rather than `git log -- <path>`, which is unreliable
//! here: history simplification prunes merges, and the `crates/` -> `packages/`
//! move means today's paths do not match historical ones. The hand-written
//! `CHANGELOG.md` is the authoritative baseline; attribute forward from the
//! `--since` ref (a release boundary).
//!
//! A component bumps when it has a directly-attributed PR. Bumps then cascade
//! (transitively) to changelog-enabled dependents, which record an explicit
//! "Updated <dep> to vX.Y.Z" entry, in addition to listing the PRs that touched
//! its own paths directly. A PR touching several components appears in each, so
//! every release's notes read on their own.
//!
//! By default the command is a dry run: it prints the regenerated `CHANGELOG.md`
//! and the version-file edits it would make. `--write` applies them.

use anyhow::{Context, anyhow};
use indexmap::{IndexMap, IndexSet};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Repo slug used to build PR links in changelog entries.
const REPO_SLUG: &str = "MaterializeInc/materialize-monitoring";

/// Arguments for the `changelog` command.
#[derive(clap::Args)]
pub struct ChangelogArgs {
    /// Path to the component definitions.
    #[arg(long, default_value = "packages/components.yaml")]
    components: PathBuf,

    /// Path to the changelog to read and (with --write) rewrite.
    #[arg(long, default_value = "CHANGELOG.md")]
    changelog: PathBuf,

    /// Git ref to start from (exclusive) — the last release boundary.
    #[arg(long)]
    since: String,

    /// Git ref to collect up to (inclusive).
    #[arg(long, default_value = "HEAD")]
    until: String,

    /// Apply the changes. Without this flag the command is a dry run.
    #[arg(long)]
    write: bool,

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
    #[serde(default)]
    version_paths: Vec<String>,
    #[serde(default)]
    content_paths: Vec<String>,
    /// Paths to exclude from `content_paths` — typically generated outputs that
    /// belong to a dependency (e.g. a chart's `pre-rendered/` tree).
    #[serde(default)]
    content_exclude: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

/// A semantic version `vMAJOR.MINOR.PATCH`. Field order makes the derived `Ord`
/// the natural precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVer {
    /// Parse `X.Y.Z`, tolerating an optional leading `v`.
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().strip_prefix('v').unwrap_or(s.trim());
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn bump_minor(self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    /// `vX.Y.Z` — the form used in `CHANGELOG.md` headings.
    fn changelog(self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// `X.Y.Z` — the form used in `Chart.yaml`/`Cargo.toml`/`pyproject.toml`.
    fn plain(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
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
    /// Human-friendly label: `#15 <title>`, falling back to branch or hash.
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

    /// The text used as a changelog bullet.
    fn bullet_text(&self) -> String {
        self.title
            .clone()
            .or_else(|| self.branch.clone())
            .unwrap_or_else(|| self.hash.chars().take(9).collect())
    }

    /// Stable identity for de-duplicating a PR across a section.
    fn key(&self) -> String {
        match self.number {
            Some(n) => format!("#{n}"),
            None => self.hash.clone(),
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
    let under = |spec: &str| {
        let spec = spec.trim_end_matches('/');
        file == spec || file.starts_with(&format!("{spec}/"))
    };
    let mut best: Option<(usize, &str)> = None;
    for (name, c) in comps {
        if c.content_exclude.iter().any(|ex| under(ex)) {
            continue;
        }
        for cp in &c.content_paths {
            let cp = cp.trim_end_matches('/');
            if under(cp) && best.is_none_or(|(blen, _)| cp.len() > blen) {
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

/// A `## <title> vX.Y.Z [(Unreleased)]` section and its verbatim body lines.
struct Section {
    title: String,
    version: SemVer,
    unreleased: bool,
    body: Vec<String>,
}

/// `CHANGELOG.md` split into a header and its sections.
struct ParsedChangelog {
    header: Vec<String>,
    sections: Vec<Section>,
}

/// Parse a `## <title> vX.Y.Z [(Unreleased)]` heading.
fn parse_heading(line: &str) -> Option<(String, SemVer, bool)> {
    let rest = line.strip_prefix("## ")?;
    let (rest, unreleased) = match rest.strip_suffix(" (Unreleased)") {
        Some(r) => (r, true),
        None => (rest, false),
    };
    let idx = rest.rfind(" v")?;
    let version = SemVer::parse(&rest[idx + 1..])?;
    Some((rest[..idx].trim().to_string(), version, unreleased))
}

fn parse_changelog(text: &str) -> anyhow::Result<ParsedChangelog> {
    let mut header = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    for line in text.lines() {
        if line.starts_with("## ") {
            let (title, version, unreleased) = parse_heading(line)
                .ok_or_else(|| anyhow!("unrecognized changelog heading: {line:?}"))?;
            sections.push(Section {
                title,
                version,
                unreleased,
                body: Vec::new(),
            });
        } else if let Some(section) = sections.last_mut() {
            section.body.push(line.to_string());
        } else {
            header.push(line.to_string());
        }
    }
    Ok(ParsedChangelog { header, sections })
}

/// The version a bumping component should carry: an existing unreleased version
/// is reused, otherwise the latest released version is minor-bumped, otherwise
/// a brand-new stream starts at v0.1.0.
fn next_version(title: &str, parsed: &ParsedChangelog) -> SemVer {
    if let Some(s) = parsed
        .sections
        .iter()
        .find(|s| s.title == title && s.unreleased)
    {
        return s.version;
    }
    parsed
        .sections
        .iter()
        .filter(|s| s.title == title && !s.unreleased)
        .map(|s| s.version)
        .max()
        .map_or(
            SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            SemVer::bump_minor,
        )
}

/// Shared, read-only context for rendering a changelog section.
struct RenderCtx<'a> {
    comps: &'a IndexMap<String, Component>,
    attributed: &'a [Attributed<'a>],
    bumping: &'a IndexSet<String>,
    versions: &'a IndexMap<String, SemVer>,
}

/// Append a PR bullet (title + indented link) at the given indent level.
fn push_pr(out: &mut Vec<String>, pr: &PrMerge, level: usize) {
    let indent = "    ".repeat(level);
    out.push(format!("{indent}* {}", pr.bullet_text()));
    if let Some(n) = pr.number {
        out.push(format!(
            "{indent}    * [materialize-monitoring#{n}](https://github.com/{REPO_SLUG}/pull/{n})"
        ));
    }
}

/// Render a component's unreleased section: its own PR bullets, then a
/// `### Dependencies` subsection that recursively rolls up each bumped
/// changelog-enabled dependency with its PRs nested beneath. A PR already shown
/// as a first-class change here is not repeated under dependencies.
fn render_section(name: &str, c: &Component, ctx: &RenderCtx<'_>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut body = Vec::new();

    for a in ctx.attributed {
        if a.owners.contains_key(name) {
            seen.insert(a.pr.key());
            push_pr(&mut body, a.pr, 0);
        }
    }

    let mut deps = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    render_deps(c, 0, ctx, &mut seen, &mut visited, &mut deps);
    if !deps.is_empty() {
        if !body.is_empty() {
            body.push(String::new());
        }
        body.push("### Dependencies".to_string());
        body.push(String::new());
        body.extend(deps);
    }
    body
}

/// Emit `* Updated <dep> to vX` lines (at `level`) for each bumped
/// changelog-enabled dependency, with its not-yet-seen PRs nested one level
/// beneath and its own dependencies recursed deeper. `visited` guards cycles
/// and keeps each dependency to a single rollup per section.
fn render_deps(
    c: &Component,
    level: usize,
    ctx: &RenderCtx<'_>,
    seen: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    let indent = "    ".repeat(level);
    let mut children: Vec<String> = Vec::new();
    // Emit this component's direct bumped dependencies at the current level...
    for dep in &c.dependencies {
        let is_changelog_dep = ctx.comps.get(dep).is_some_and(|d| d.changelog);
        if !is_changelog_dep || !ctx.bumping.contains(dep) {
            continue;
        }
        if !visited.insert(dep.clone()) {
            continue; // already rolled up elsewhere in this section
        }
        let dc = &ctx.comps[dep];
        out.push(format!(
            "{indent}* Updated {} to {}",
            dc.title,
            ctx.versions[dep].changelog()
        ));
        for a in ctx.attributed {
            if a.owners.contains_key(dep) && seen.insert(a.pr.key()) {
                push_pr(out, a.pr, level + 1);
            }
        }
        children.push(dep.clone());
    }
    // ...then recurse for their transitive dependencies, nested one level deeper.
    for dep in children {
        render_deps(&ctx.comps[&dep], level + 1, ctx, seen, visited, out);
    }
}

/// Rewrite the `version` field of a Chart.yaml / Cargo.toml / pyproject.toml,
/// returning the old version and the new file contents. Errors if no version
/// line is found rather than silently doing nothing.
fn rewrite_version(path: &Path, new: &str) -> anyhow::Result<(String, String)> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let toml = path.extension().and_then(|e| e.to_str()) == Some("toml");

    let mut old = None;
    let mut out = Vec::new();
    for line in text.lines() {
        if old.is_none() {
            if toml && line.starts_with("version = \"") {
                old = Some(
                    line.trim_start_matches("version = \"")
                        .trim_end_matches('"')
                        .to_string(),
                );
                out.push(format!("version = \"{new}\""));
                continue;
            }
            if !toml && line.starts_with("version:") {
                old = Some(line["version:".len()..].trim().to_string());
                out.push(format!("version: {new}"));
                continue;
            }
        }
        out.push(line.to_string());
    }

    let old = old.ok_or_else(|| anyhow!("no version field found in {}", path.display()))?;
    let mut content = out.join("\n");
    if text.ends_with('\n') {
        content.push('\n');
    }
    Ok((old, content))
}

/// Main entrypoint for the `changelog` command.
pub fn changelog(args: ChangelogArgs) -> anyhow::Result<()> {
    let comps_text = std::fs::read_to_string(&args.components)
        .with_context(|| format!("reading {}", args.components.display()))?;
    let comps = serde_yaml_ng::from_str::<ComponentsFile>(&comps_text)
        .with_context(|| format!("parsing {}", args.components.display()))?
        .components;

    let merges = collect_merges(&args.since, &args.until)?;
    let attributed: Vec<Attributed> = merges
        .iter()
        .map(|pr| {
            let mut owners: IndexMap<String, Vec<String>> = IndexMap::new();
            let mut orphans = Vec::new();
            for f in &pr.paths {
                match owner_of(f, &comps) {
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

    // Components with a directly-attributed PR bump...
    let mut bumping: IndexSet<String> = comps
        .iter()
        .filter(|(name, c)| c.changelog && attributed.iter().any(|a| a.owners.contains_key(*name)))
        .map(|(name, _)| name.clone())
        .collect();
    // ...then bumps cascade transitively to changelog-enabled dependents.
    loop {
        let mut added = false;
        for (name, c) in &comps {
            if c.changelog
                && !bumping.contains(name)
                && c.dependencies
                    .iter()
                    .any(|d| comps.get(d).is_some_and(|dc| dc.changelog) && bumping.contains(d))
            {
                bumping.insert(name.clone());
                added = true;
            }
        }
        if !added {
            break;
        }
    }

    let changelog_text = std::fs::read_to_string(&args.changelog)
        .with_context(|| format!("reading {}", args.changelog.display()))?;
    let parsed = parse_changelog(&changelog_text)
        .with_context(|| format!("parsing {}", args.changelog.display()))?;

    // Resolve the version each bumping component will carry.
    let versions: IndexMap<String, SemVer> = bumping
        .iter()
        .map(|name| (name.clone(), next_version(&comps[name].title, &parsed)))
        .collect();

    // Reassemble: header, regenerated unreleased sections (components.yaml
    // order), any existing unreleased sections we did not regenerate, then the
    // released sections verbatim.
    let bumping_titles: IndexSet<&str> = bumping.iter().map(|n| comps[n].title.as_str()).collect();
    let ctx = RenderCtx {
        comps: &comps,
        attributed: &attributed,
        bumping: &bumping,
        versions: &versions,
    };

    let mut out: Vec<String> = parsed.header.clone();
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out.push(String::new());

    for (name, c) in &comps {
        if !c.changelog || !bumping.contains(name) {
            continue;
        }
        out.push(format!(
            "## {} {} (Unreleased)",
            c.title,
            versions[name].changelog()
        ));
        out.push(String::new());
        out.extend(render_section(name, c, &ctx));
        out.push(String::new());
    }
    for s in &parsed.sections {
        if s.unreleased && bumping_titles.contains(s.title.as_str()) {
            continue; // regenerated above
        }
        let suffix = if s.unreleased { " (Unreleased)" } else { "" };
        out.push(format!("## {} {}{suffix}", s.title, s.version.changelog()));
        out.extend(s.body.iter().cloned());
    }
    let mut new_changelog = out.join("\n");
    new_changelog.push('\n');
    // Collapse any run of 3+ blank lines introduced at section seams.
    while new_changelog.contains("\n\n\n\n") {
        new_changelog = new_changelog.replace("\n\n\n\n", "\n\n\n");
    }

    // Compute version-file edits for bumping components.
    let mut version_edits: Vec<(PathBuf, String, String, String)> = Vec::new();
    for name in &bumping {
        let version = versions[name].plain();
        for vp in &comps[name].version_paths {
            let path = PathBuf::from(vp);
            let (old, content) = rewrite_version(&path, &version)?;
            version_edits.push((path, old, version.clone(), content));
        }
    }

    // Report bumps.
    println!(
        "Range {}..{}: {} merged PR(s)",
        args.since,
        args.until,
        merges.len()
    );
    println!("\n== Bumps ==");
    for (name, _) in &comps {
        if bumping.contains(name) {
            println!("  {name} -> {}", versions[name].changelog());
        }
    }

    if args.verbose {
        println!("\n== Per-PR attribution ==");
        for a in &attributed {
            let touched: Vec<&str> = a.owners.keys().map(String::as_str).collect();
            println!("  {} -> [{}]", a.pr.label(), touched.join(", "));
        }
        let orphans: Vec<&Attributed> = attributed
            .iter()
            .filter(|a| !a.orphans.is_empty())
            .collect();
        if !orphans.is_empty() {
            println!("\n== Unattributed paths ==");
            for a in &orphans {
                println!("  {}: {}", a.pr.label(), a.orphans.join(", "));
            }
        }
    }

    if args.write {
        std::fs::write(&args.changelog, &new_changelog)
            .with_context(|| format!("writing {}", args.changelog.display()))?;
        for (path, _old, _new, content) in &version_edits {
            std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
        }
        println!(
            "\nWrote {} and {} version file(s).",
            args.changelog.display(),
            version_edits.len()
        );
    } else {
        println!("\n== Proposed version-file edits (dry run) ==");
        for (path, old, new, _) in &version_edits {
            println!("  {}: {old} -> {new}", path.display());
        }
        println!("\n== Proposed {} (dry run) ==\n", args.changelog.display());
        println!("{new_changelog}");
    }

    Ok(())
}
