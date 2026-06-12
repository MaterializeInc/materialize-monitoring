// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Versioning and changelog logic for the component streams declared in
//! `packages/components.yaml`. Hosts two subcommands:
//!
//! - `changelog` — a read-only report of which merged PRs each component would
//!   collect (for validating `components.yaml` against history).
//! - `release` — generates a `version-update/<component>` PR's changelog: it
//!   promotes that component's `_Changes Pending_` placeholder in place into a
//!   released section populated with its changes, inserts a fresh placeholder
//!   at the top, and bumps the component's `version_paths`.
//!
//! Merged PRs in a commit range are attributed to a component by longest-prefix
//! match against `content_paths` (minus `content_exclude`). Attribution works
//! off each merge's own diff (`<merge>^1..<merge>`) rather than
//! `git log -- <path>`, which is unreliable here: history simplification prunes
//! merges, and the `crates/` -> `packages/` move means today's paths do not
//! match historical ones.
//!
//! A component bumps when it has a directly-attributed PR. Bumps then cascade
//! (transitively) to changelog-enabled dependents, which record an
//! "Included <dep> @ vPREV..vNEW" entry (single version when there is no prior
//! release), with the dependency's own PRs nested beneath. A PR touching
//! several components appears in each, so every release's notes read on their
//! own.
//!
//! Both subcommands default to a dry run; `--write` applies the changes.

use anyhow::{Context, anyhow};
use indexmap::{IndexMap, IndexSet};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Repo slug used to build PR links in changelog entries.
const REPO_SLUG: &str = "MaterializeInc/materialize-monitoring";

/// Arguments for the read-only `changelog` report.
#[derive(clap::Args)]
pub struct ChangelogArgs {
    /// Path to the component definitions.
    #[arg(long, default_value = "packages/components.yaml")]
    components: PathBuf,

    /// Path to the changelog (read to show each stream's next version).
    #[arg(long, default_value = "CHANGELOG.md")]
    changelog: PathBuf,

    /// Git ref to start from (exclusive) — the last release boundary.
    #[arg(long)]
    since: String,

    /// Git ref to collect up to (inclusive).
    #[arg(long, default_value = "HEAD")]
    until: String,

    /// Print the per-PR path -> component breakdown.
    #[arg(long)]
    verbose: bool,
}

/// Arguments for the `release` command.
#[derive(clap::Args)]
pub struct ReleaseArgs {
    /// Path to the component definitions.
    #[arg(long, default_value = "packages/components.yaml")]
    components: PathBuf,

    /// Path to the changelog to read and (with --write) rewrite.
    #[arg(long, default_value = "CHANGELOG.md")]
    changelog: PathBuf,

    /// Component to release (a key in components.yaml).
    #[arg(long)]
    component: String,

    /// Git ref to start from (exclusive) — the component's last release.
    #[arg(long)]
    since: String,

    /// Git ref to collect up to (inclusive).
    #[arg(long, default_value = "HEAD")]
    until: String,

    /// `uv` lockfile to keep in sync when a Python `version_paths` is bumped.
    #[arg(long, default_value = "uv.lock")]
    uv_lock: PathBuf,

    /// Apply the changes. Without this flag the command is a dry run.
    #[arg(long)]
    write: bool,
}

/// Load and parse `components.yaml`.
pub(crate) fn load_components(path: &Path) -> anyhow::Result<IndexMap<String, Component>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(serde_yaml_ng::from_str::<ComponentsFile>(&text)
        .with_context(|| format!("parsing {}", path.display()))?
        .components)
}

/// Resolve a git refish to a commit SHA, or `None` if it does not exist.
pub(crate) fn rev_parse(refish: &str) -> Option<String> {
    git(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{refish}^{{commit}}"),
    ])
    .ok()
    .filter(|s| !s.is_empty())
}

#[derive(Debug, Deserialize)]
struct ComponentsFile {
    components: IndexMap<String, Component>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Component {
    #[serde(default)]
    pub(crate) changelog: bool,
    pub(crate) title: String,
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
pub(crate) struct SemVer {
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
    pub(crate) fn changelog(self) -> String {
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

/// Group each PR's changed paths by owning component (paths owned by no
/// component land in `orphans`).
fn attribute<'a>(
    merges: &'a [PrMerge],
    comps: &IndexMap<String, Component>,
) -> Vec<Attributed<'a>> {
    merges
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
        .collect()
}

/// The changelog-enabled components that bump: those with a directly attributed
/// PR, plus their transitive changelog-enabled dependents.
fn compute_bumps(
    comps: &IndexMap<String, Component>,
    attributed: &[Attributed],
) -> IndexSet<String> {
    let mut bumping: IndexSet<String> = comps
        .iter()
        .filter(|(name, c)| c.changelog && attributed.iter().any(|a| a.owners.contains_key(*name)))
        .map(|(name, _)| name.clone())
        .collect();
    loop {
        let mut added = false;
        for (name, c) in comps {
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
    bumping
}

/// A `## <title> vX.Y.Z [(Unreleased)]` section and its verbatim body lines.
struct Section {
    title: String,
    version: SemVer,
    unreleased: bool,
    body: Vec<String>,
}

/// `CHANGELOG.md` split into a header and its sections.
pub(crate) struct ParsedChangelog {
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

pub(crate) fn parse_changelog(text: &str) -> anyhow::Result<ParsedChangelog> {
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
    latest_released(title, parsed).map_or(
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        SemVer::bump_minor,
    )
}

/// The highest released (non-unreleased) version recorded for `title`, if any.
pub(crate) fn latest_released(title: &str, parsed: &ParsedChangelog) -> Option<SemVer> {
    parsed
        .sections
        .iter()
        .filter(|s| s.title == title && !s.unreleased)
        .map(|s| s.version)
        .max()
}

/// Shared, read-only context for rendering a changelog section.
struct RenderCtx<'a> {
    comps: &'a IndexMap<String, Component>,
    attributed: &'a [Attributed<'a>],
    bumping: &'a IndexSet<String>,
    versions: &'a IndexMap<String, SemVer>,
    /// Latest released version per component (absent for brand-new streams);
    /// used to render the `@ vPREV..vNEW` range on dependency rollups.
    prev: &'a IndexMap<String, SemVer>,
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
        let target = ctx.versions[dep];
        // Span the dependency's released version (if any) to its new version;
        // a single version when there is no prior release. "Included" rather
        // than "Updated" because the new version may not be released yet.
        let span = match ctx.prev.get(dep) {
            Some(prev) if *prev != target => {
                format!("{}..{}", prev.changelog(), target.changelog())
            }
            _ => target.changelog(),
        };
        out.push(format!("{indent}* Included {} @ {span}", dc.title));
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

/// Generate the changelog for a `version-update/<component>` PR.
///
/// `target`'s `_Changes Pending_` placeholder is promoted in place to a released
/// section populated with [`render_section`]'s output, a fresh placeholder for
/// the next (minor-bumped) version is inserted at the top, and every other
/// section is preserved verbatim. Returns the new changelog text and the
/// released version (for bumping `version_paths`).
fn apply_version_update(
    parsed: &ParsedChangelog,
    target: &str,
    ctx: &RenderCtx<'_>,
) -> anyhow::Result<(String, SemVer)> {
    let comp = &ctx.comps[target];
    let title = comp.title.as_str();
    let released = parsed
        .sections
        .iter()
        .find(|s| s.title == title && s.unreleased)
        .map(|s| s.version)
        .ok_or_else(|| anyhow!("no pending placeholder for {title} in CHANGELOG"))?;
    let next = released.bump_minor();
    let body = render_section(target, comp, ctx);

    let mut out: Vec<String> = parsed.header.clone();
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out.push(String::new());

    // Fresh placeholder for the next version, at the top.
    out.push(format!("## {title} {} (Unreleased)", next.changelog()));
    out.push(String::new());
    out.push("_Changes Pending_".to_string());
    out.push(String::new());

    for s in &parsed.sections {
        if s.title == title && s.unreleased {
            // Promote this placeholder in place into a released section.
            out.push(format!("## {title} {}", released.changelog()));
            out.push(String::new());
            out.extend(body.iter().cloned());
            out.push(String::new());
        } else {
            let suffix = if s.unreleased { " (Unreleased)" } else { "" };
            out.push(format!("## {} {}{suffix}", s.title, s.version.changelog()));
            out.extend(s.body.iter().cloned());
        }
    }

    let mut text = out.join("\n");
    text.push('\n');
    // Collapse any run of 3+ blank lines introduced at section seams.
    while text.contains("\n\n\n\n") {
        text = text.replace("\n\n\n\n", "\n\n\n");
    }
    Ok((text, released))
}

/// Rewrite the first `version` field in `text`, returning the old version and
/// the new contents. `toml` selects `version = "X"` (TOML) vs `version: X`
/// (YAML). Errors if no version line is found rather than silently doing nothing.
fn rewrite_version_str(text: &str, toml: bool, new: &str) -> anyhow::Result<(String, String)> {
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

    let old = old.ok_or_else(|| anyhow!("no version field found"))?;
    let mut content = out.join("\n");
    if text.ends_with('\n') {
        content.push('\n');
    }
    Ok((old, content))
}

/// File wrapper around [`rewrite_version_str`], selecting TOML vs YAML by
/// extension.
fn rewrite_version(path: &Path, new: &str) -> anyhow::Result<(String, String)> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let toml = path.extension().and_then(|e| e.to_str()) == Some("toml");
    rewrite_version_str(&text, toml, new).with_context(|| format!("in {}", path.display()))
}

/// The `[project].name` of a `pyproject.toml` — the package name `uv.lock`
/// keys on. Takes the first top-level `name = "..."`.
fn pyproject_name(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        l.strip_prefix("name = \"")
            .and_then(|r| r.strip_suffix('"'))
            .map(str::to_string)
    })
}

/// Bump the `version` of a `[[package]]` in a `uv.lock`, matched by package
/// name, returning the old version and the new contents. The version line is
/// the first `version = "..."` after the matching `name = "..."` line.
fn rewrite_lock_version(lock: &str, package: &str, new: &str) -> anyhow::Result<(String, String)> {
    let needle = format!("name = \"{package}\"");
    let mut in_target = false;
    let mut old = None;
    let mut out = Vec::new();
    for line in lock.lines() {
        if old.is_none() && in_target && line.starts_with("version = \"") {
            old = Some(
                line.trim_start_matches("version = \"")
                    .trim_end_matches('"')
                    .to_string(),
            );
            out.push(format!("version = \"{new}\""));
            in_target = false;
            continue;
        }
        if line.trim() == needle {
            in_target = true;
        }
        out.push(line.to_string());
    }

    let old = old.ok_or_else(|| anyhow!("package {package:?} not found in lockfile"))?;
    let mut content = out.join("\n");
    if lock.ends_with('\n') {
        content.push('\n');
    }
    Ok((old, content))
}

/// Read-only `changelog` report: which merged PRs each component would collect
/// in the given range, and the version each would bump to.
pub fn changelog(args: ChangelogArgs) -> anyhow::Result<()> {
    let comps = load_components(&args.components)?;
    let merges = collect_merges(&args.since, &args.until)?;
    let attributed = attribute(&merges, &comps);
    let bumping = compute_bumps(&comps, &attributed);

    let changelog_text = std::fs::read_to_string(&args.changelog)
        .with_context(|| format!("reading {}", args.changelog.display()))?;
    let parsed = parse_changelog(&changelog_text)
        .with_context(|| format!("parsing {}", args.changelog.display()))?;

    println!(
        "Range {}..{}: {} merged PR(s)",
        args.since,
        args.until,
        merges.len()
    );
    println!("\n== Bumps ==");
    for (name, _) in &comps {
        if bumping.contains(name) {
            let version = next_version(&comps[name].title, &parsed);
            println!("  {name} -> {}", version.changelog());
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

    Ok(())
}

/// The complete set of file changes for releasing one component: the rewritten
/// changelog plus its version-file (and uv.lock) bumps.
pub(crate) struct ReleasePlan {
    pub(crate) released: SemVer,
    pub(crate) changelog_path: PathBuf,
    pub(crate) changelog_content: String,
    /// Version-file / uv.lock edits as (path, new content).
    pub(crate) version_files: Vec<(PathBuf, String)>,
    /// Human-readable per-edit lines for dry-run output.
    pub(crate) summary: Vec<String>,
}

impl ReleasePlan {
    /// Every (path, content) pair to write or commit, including the changelog.
    pub(crate) fn files(&self) -> Vec<(PathBuf, String)> {
        let mut files = vec![(self.changelog_path.clone(), self.changelog_content.clone())];
        files.extend(self.version_files.iter().cloned());
        files
    }
}

/// Compute the release for `target` over `since..until`. Returns `Ok(None)` when
/// the component has no changes in range (nothing to release). Reads the
/// changelog and version files; collects merges via git. The caller is
/// responsible for validating that `target` is a changelog-enabled component.
pub(crate) fn plan_release(
    comps: &IndexMap<String, Component>,
    target: &str,
    changelog_path: &Path,
    uv_lock_path: &Path,
    since: &str,
    until: &str,
) -> anyhow::Result<Option<ReleasePlan>> {
    let merges = collect_merges(since, until)?;
    let attributed = attribute(&merges, comps);
    let bumping = compute_bumps(comps, &attributed);
    if !bumping.contains(target) {
        return Ok(None);
    }

    let changelog_text = std::fs::read_to_string(changelog_path)
        .with_context(|| format!("reading {}", changelog_path.display()))?;
    let parsed = parse_changelog(&changelog_text)
        .with_context(|| format!("parsing {}", changelog_path.display()))?;

    let versions: IndexMap<String, SemVer> = bumping
        .iter()
        .map(|name| (name.clone(), next_version(&comps[name].title, &parsed)))
        .collect();
    let prev: IndexMap<String, SemVer> = bumping
        .iter()
        .filter_map(|name| latest_released(&comps[name].title, &parsed).map(|v| (name.clone(), v)))
        .collect();
    let ctx = RenderCtx {
        comps,
        attributed: &attributed,
        bumping: &bumping,
        versions: &versions,
        prev: &prev,
    };

    let (changelog_content, released) = apply_version_update(&parsed, target, &ctx)?;
    let released_plain = released.plain();

    // Version-file edits bump only the released component. Bumping a pyproject
    // also bumps that package's entry in uv.lock, so the lockfile does not drift
    // out of date behind the version files.
    let mut version_files: Vec<(PathBuf, String)> = Vec::new();
    let mut summary: Vec<String> = Vec::new();
    let mut lock_packages: Vec<String> = Vec::new();
    for vp in &comps[target].version_paths {
        let path = PathBuf::from(vp);
        let (old, content) = rewrite_version(&path, &released_plain)?;
        if path.file_name().and_then(|n| n.to_str()) == Some("pyproject.toml")
            && let Some(name) = pyproject_name(&content)
        {
            lock_packages.push(name);
        }
        summary.push(format!("{}: {old} -> {released_plain}", path.display()));
        version_files.push((path, content));
    }

    if !lock_packages.is_empty() && uv_lock_path.exists() {
        let mut lock = std::fs::read_to_string(uv_lock_path)
            .with_context(|| format!("reading {}", uv_lock_path.display()))?;
        for name in &lock_packages {
            let (old, updated) = rewrite_lock_version(&lock, name, &released_plain)
                .with_context(|| format!("in {}", uv_lock_path.display()))?;
            summary.push(format!(
                "{} [{name}: {old} -> {released_plain}]",
                uv_lock_path.display()
            ));
            lock = updated;
        }
        version_files.push((uv_lock_path.to_path_buf(), lock));
    }

    Ok(Some(ReleasePlan {
        released,
        changelog_path: changelog_path.to_path_buf(),
        changelog_content,
        version_files,
        summary,
    }))
}

/// `release` command: generate (and optionally write) a `version-update` PR's
/// changelog and version-file bumps for a single component.
pub fn release(args: ReleaseArgs) -> anyhow::Result<()> {
    let comps = load_components(&args.components)?;
    let target = &args.component;
    let comp = comps
        .get(target)
        .ok_or_else(|| anyhow!("unknown component {target:?}"))?;
    if !comp.changelog {
        anyhow::bail!("component {target:?} has no changelog stream");
    }

    let plan = plan_release(
        &comps,
        target,
        &args.changelog,
        &args.uv_lock,
        &args.since,
        &args.until,
    )?
    .ok_or_else(|| anyhow!("no changes attributed to {target:?} since {}", args.since))?;

    println!("Releasing {target} {}", plan.released.changelog());
    if args.write {
        let files = plan.files();
        for (path, content) in &files {
            std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
        }
        println!("Wrote {} file(s).", files.len());
    } else {
        println!("\n== Proposed edits (dry run) ==");
        for line in &plan.summary {
            println!("  {line}");
        }
        println!(
            "\n== Proposed {} (dry run) ==\n",
            plan.changelog_path.display()
        );
        println!("{}", plan.changelog_content);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- builders -------------------------------------------------------

    fn strs(s: &[&str]) -> Vec<String> {
        s.iter().copied().map(str::to_string).collect()
    }

    fn comp(
        changelog: bool,
        title: &str,
        version_paths: &[&str],
        content_paths: &[&str],
        content_exclude: &[&str],
        dependencies: &[&str],
    ) -> Component {
        Component {
            changelog,
            title: title.to_string(),
            version_paths: strs(version_paths),
            content_paths: strs(content_paths),
            content_exclude: strs(content_exclude),
            dependencies: strs(dependencies),
        }
    }

    fn comps(entries: Vec<(&str, Component)>) -> IndexMap<String, Component> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn pr(
        number: Option<u64>,
        title: Option<&str>,
        branch: Option<&str>,
        paths: &[&str],
    ) -> PrMerge {
        PrMerge {
            hash: "0123456789abcdef".to_string(),
            number,
            branch: branch.map(str::to_string),
            title: title.map(str::to_string),
            paths: strs(paths),
        }
    }

    fn link(n: u64) -> String {
        format!("    * [materialize-monitoring#{n}](https://github.com/{REPO_SLUG}/pull/{n})")
    }

    // ---- SemVer ---------------------------------------------------------

    #[test]
    fn semver_parse_variants() {
        assert_eq!(
            SemVer::parse("1.2.3"),
            Some(SemVer {
                major: 1,
                minor: 2,
                patch: 3
            })
        );
        assert_eq!(
            SemVer::parse("v1.2.3"),
            Some(SemVer {
                major: 1,
                minor: 2,
                patch: 3
            })
        );
        assert_eq!(
            SemVer::parse("  v0.0.0  "),
            Some(SemVer {
                major: 0,
                minor: 0,
                patch: 0
            })
        );
        assert_eq!(SemVer::parse("1.2"), None); // too few
        assert_eq!(SemVer::parse("1.2.3.4"), None); // too many
        assert_eq!(SemVer::parse("x.y.z"), None); // non-numeric
    }

    #[test]
    fn semver_bump_format_and_order() {
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert_eq!(
            v.bump_minor(),
            SemVer {
                major: 1,
                minor: 3,
                patch: 0
            }
        );
        assert_eq!(v.changelog(), "v1.2.3");
        assert_eq!(v.plain(), "1.2.3");
        assert!(SemVer::parse("0.1.0") < SemVer::parse("0.2.0"));
        assert!(SemVer::parse("1.0.0") > SemVer::parse("0.9.9"));
    }

    // ---- parse_subject --------------------------------------------------

    #[test]
    fn parse_subject_variants() {
        assert_eq!(
            parse_subject("Merge pull request #15 from Org/heather/branch"),
            (Some(15), Some("Org/heather/branch".to_string()))
        );
        assert_eq!(parse_subject("#42"), (Some(42), None));
        assert_eq!(parse_subject("no marker here"), (None, None));
        assert_eq!(parse_subject("#abc def"), (None, None)); // non-digit after #
        assert_eq!(parse_subject("title from x"), (None, Some("x".to_string())));
    }

    // ---- PrMerge label / bullet_text / key ------------------------------

    #[test]
    fn prmerge_label_variants() {
        assert_eq!(
            pr(Some(15), Some("Title"), Some("br"), &[]).label(),
            "#15 Title"
        );
        assert_eq!(pr(Some(15), None, Some("br"), &[]).label(), "#15 (br)");
        assert_eq!(
            pr(None, Some("Title"), None, &[]).label(),
            "012345678 Title"
        );
        assert_eq!(pr(None, None, None, &[]).label(), "012345678");
    }

    #[test]
    fn prmerge_bullet_and_key() {
        assert_eq!(pr(Some(1), Some("T"), Some("b"), &[]).bullet_text(), "T");
        assert_eq!(pr(Some(1), None, Some("b"), &[]).bullet_text(), "b");
        assert_eq!(pr(None, None, None, &[]).bullet_text(), "012345678");
        assert_eq!(pr(Some(7), None, None, &[]).key(), "#7");
        assert_eq!(pr(None, None, None, &[]).key(), "0123456789abcdef");
    }

    // ---- owner_of -------------------------------------------------------

    #[test]
    fn owner_of_longest_prefix_and_tie() {
        let cs = comps(vec![
            ("x", comp(true, "X", &[], &["a"], &[], &[])),
            ("y", comp(true, "Y", &[], &["a/b"], &[], &[])),
            ("z", comp(true, "Z", &[], &["a"], &[], &[])), // ties with x on "a"
        ]);
        assert_eq!(owner_of("a/b/c", &cs), Some("y")); // longest wins
        assert_eq!(owner_of("a/x", &cs), Some("x")); // tie -> declaration order
        assert_eq!(owner_of("a", &cs), Some("x")); // exact match on "a"
        assert_eq!(owner_of("other", &cs), None);
    }

    #[test]
    fn owner_of_exclude() {
        let cs = comps(vec![
            (
                "chart",
                comp(true, "Chart", &[], &["chart/"], &["chart/gen/"], &[]),
            ),
            (
                "dash",
                comp(true, "Dash", &[], &["chart/gen/dash/"], &[], &[]),
            ),
        ]);
        assert_eq!(owner_of("chart/values.yaml", &cs), Some("chart"));
        assert_eq!(owner_of("chart/gen/dash/d.json", &cs), Some("dash")); // chart excludes gen/
        assert_eq!(owner_of("chart/gen/rules/r.yaml", &cs), None); // excluded, no other owner
    }

    // ---- parse_heading --------------------------------------------------

    #[test]
    fn parse_heading_variants() {
        assert_eq!(
            parse_heading("## Foo v1.2.3"),
            Some((
                "Foo".to_string(),
                SemVer {
                    major: 1,
                    minor: 2,
                    patch: 3
                },
                false
            ))
        );
        assert_eq!(
            parse_heading("## Foo v1.2.3 (Unreleased)"),
            Some((
                "Foo".to_string(),
                SemVer {
                    major: 1,
                    minor: 2,
                    patch: 3
                },
                true
            ))
        );
        assert_eq!(
            parse_heading("## mzmon-lib (shared library) v0.5.0").map(|(t, _, _)| t),
            Some("mzmon-lib (shared library)".to_string())
        );
        assert_eq!(parse_heading("# Foo v1.0.0"), None); // not "## "
        assert_eq!(parse_heading("## No version"), None); // no " v"
        assert_eq!(parse_heading("## Foo vNaN"), None); // bad version
    }

    // ---- parse_changelog ------------------------------------------------

    #[test]
    fn parse_changelog_splits_header_and_sections() {
        let text = "# Title\n\n## Foo v0.2.0 (Unreleased)\n\n* a\n\n## Foo v0.1.0\n\n* b\n";
        let p = parse_changelog(text).unwrap();
        assert_eq!(p.header, strs(&["# Title", ""]));
        assert_eq!(p.sections.len(), 2);
        assert_eq!(p.sections[0].title, "Foo");
        assert!(p.sections[0].unreleased);
        assert_eq!(
            p.sections[0].version,
            SemVer {
                major: 0,
                minor: 2,
                patch: 0
            }
        );
        assert!(p.sections[0].body.contains(&"* a".to_string()));
        assert!(!p.sections[1].unreleased);
    }

    #[test]
    fn parse_changelog_errors_on_bad_heading() {
        assert!(parse_changelog("# Title\n\n## not a version\n").is_err());
    }

    // ---- next_version ---------------------------------------------------

    #[test]
    fn next_version_rules() {
        let reuse = parse_changelog("## Foo v0.2.0 (Unreleased)\n").unwrap();
        assert_eq!(
            next_version("Foo", &reuse),
            SemVer {
                major: 0,
                minor: 2,
                patch: 0
            }
        );

        let released = parse_changelog("## Foo v0.1.0\n\n## Foo v0.3.0\n").unwrap();
        assert_eq!(
            next_version("Foo", &released),
            SemVer {
                major: 0,
                minor: 4,
                patch: 0
            }
        );

        assert_eq!(
            next_version("Missing", &released),
            SemVer {
                major: 0,
                minor: 1,
                patch: 0
            }
        );
    }

    // ---- attribute ------------------------------------------------------

    #[test]
    fn attribute_owners_and_orphans() {
        let cs = comps(vec![("lib", comp(true, "lib", &[], &["lib/"], &[], &[]))]);
        let merges = vec![pr(
            Some(1),
            Some("t"),
            None,
            &["lib/a", "lib/b", "orphan/x"],
        )];
        let a = attribute(&merges, &cs);
        assert_eq!(a[0].owners.get("lib"), Some(&strs(&["lib/a", "lib/b"])));
        assert_eq!(a[0].orphans, strs(&["orphan/x"]));
    }

    // ---- compute_bumps --------------------------------------------------

    #[test]
    fn compute_bumps_direct_and_cascade() {
        let cs = comps(vec![
            ("lib", comp(true, "lib", &[], &["lib/"], &[], &[])),
            ("pipe", comp(true, "pipe", &[], &["pipe/"], &[], &["lib"])),
            (
                "chart",
                comp(true, "chart", &[], &["chart/"], &[], &["pipe"]),
            ),
            ("common", comp(false, "common", &[], &["common/"], &[], &[])),
        ]);
        // A PR touching only lib bumps lib, cascades to pipe and chart.
        let merges = vec![pr(Some(1), Some("t"), None, &["lib/x"])];
        let a = attribute(&merges, &cs);
        let b = compute_bumps(&cs, &a);
        assert!(b.contains("lib") && b.contains("pipe") && b.contains("chart"));
        assert_eq!(b.len(), 3);

        // changelog:false components never bump, even when touched.
        let merges = vec![pr(Some(2), Some("t"), None, &["common/x"])];
        let a = attribute(&merges, &cs);
        assert!(compute_bumps(&cs, &a).is_empty());
    }

    // ---- push_pr --------------------------------------------------------

    #[test]
    fn push_pr_levels() {
        let mut out = Vec::new();
        push_pr(&mut out, &pr(Some(5), Some("T"), None, &[]), 0);
        assert_eq!(out, vec!["* T".to_string(), link(5)]);

        let mut out = Vec::new();
        push_pr(&mut out, &pr(None, Some("T"), None, &[]), 1);
        assert_eq!(out, vec!["    * T".to_string()]); // no number -> no link
    }

    // ---- render_section / render_deps -----------------------------------

    fn render_scenario() -> (IndexMap<String, Component>, Vec<PrMerge>) {
        let cs = comps(vec![
            ("lib", comp(true, "lib", &[], &["lib/"], &[], &[])),
            (
                "pipe",
                comp(true, "pipe", &[], &["pipe/"], &[], &["lib", "common"]),
            ),
            (
                "chart",
                comp(true, "chart", &[], &["chart/"], &[], &["pipe", "lib"]),
            ),
            ("common", comp(false, "common", &[], &["common/"], &[], &[])),
        ]);
        let merges = vec![
            pr(
                Some(11),
                Some("Lib and pipe work"),
                None,
                &["lib/x", "pipe/y"],
            ),
            pr(Some(12), Some("Lib only"), None, &["lib/z"]),
        ];
        (cs, merges)
    }

    #[test]
    fn render_section_first_class_and_deduped_deps() {
        let (cs, merges) = render_scenario();
        let attributed = attribute(&merges, &cs);
        let bumping = compute_bumps(&cs, &attributed);
        let versions: IndexMap<String, SemVer> = [
            (
                "lib",
                SemVer {
                    major: 0,
                    minor: 5,
                    patch: 0,
                },
            ),
            (
                "pipe",
                SemVer {
                    major: 0,
                    minor: 2,
                    patch: 0,
                },
            ),
            (
                "chart",
                SemVer {
                    major: 0,
                    minor: 2,
                    patch: 0,
                },
            ),
        ]
        .into_iter()
        .map(|(n, v)| (n.to_string(), v))
        .collect();
        // lib has a prior release (renders a range); pipe does not (single).
        let prev: IndexMap<String, SemVer> = [(
            "lib".to_string(),
            SemVer {
                major: 0,
                minor: 4,
                patch: 0,
            },
        )]
        .into_iter()
        .collect();
        let ctx = RenderCtx {
            comps: &cs,
            attributed: &attributed,
            bumping: &bumping,
            versions: &versions,
            prev: &prev,
        };

        // Pipeline: #11 first-class; #12 under lib; #11 not repeated.
        let pipe = render_section("pipe", &cs["pipe"], &ctx);
        assert_eq!(
            pipe,
            vec![
                "* Lib and pipe work".to_string(),
                link(11),
                String::new(),
                "### Dependencies".to_string(),
                String::new(),
                "* Included lib @ v0.4.0..v0.5.0".to_string(),
                "    * Lib only".to_string(),
                format!("    {}", link(12)),
            ]
        );

        // Chart: no first-class; direct deps first; lib shown once (not nested
        // under pipe); #11 under pipe (single version), #12 under lib (range).
        let chart = render_section("chart", &cs["chart"], &ctx);
        assert_eq!(
            chart,
            vec![
                "### Dependencies".to_string(),
                String::new(),
                "* Included pipe @ v0.2.0".to_string(),
                "    * Lib and pipe work".to_string(),
                format!("    {}", link(11)),
                "* Included lib @ v0.4.0..v0.5.0".to_string(),
                "    * Lib only".to_string(),
                format!("    {}", link(12)),
            ]
        );
    }

    // ---- apply_version_update -------------------------------------------

    fn semver(major: u64, minor: u64, patch: u64) -> SemVer {
        SemVer {
            major,
            minor,
            patch,
        }
    }

    #[test]
    fn apply_version_update_promotes_and_inserts_placeholder() {
        let text = "# Changelog\n\n## Foo v0.6.0 (Unreleased)\n\n_Changes Pending_\n\n## Bar v0.9.0 (Unreleased)\n\n_Changes Pending_\n\n## Foo v0.5.0\n\n* old released\n";
        let parsed = parse_changelog(text).unwrap();
        let cs = comps(vec![
            ("foo", comp(true, "Foo", &[], &["foo/"], &[], &[])),
            ("bar", comp(true, "Bar", &[], &["bar/"], &[], &[])),
        ]);
        let merges = vec![pr(Some(20), Some("Cool foo change"), None, &["foo/x"])];
        let attributed = attribute(&merges, &cs);
        let bumping = compute_bumps(&cs, &attributed);
        let versions: IndexMap<String, SemVer> =
            [("foo".to_string(), semver(0, 6, 0))].into_iter().collect();
        let prev: IndexMap<String, SemVer> =
            [("foo".to_string(), semver(0, 5, 0))].into_iter().collect();
        let ctx = RenderCtx {
            comps: &cs,
            attributed: &attributed,
            bumping: &bumping,
            versions: &versions,
            prev: &prev,
        };

        let (out, released) = apply_version_update(&parsed, "foo", &ctx).unwrap();
        assert_eq!(released, semver(0, 6, 0));
        // Fresh placeholder for the next version, at the top.
        assert!(out.contains("## Foo v0.7.0 (Unreleased)\n\n_Changes Pending_"));
        // The v0.6.0 placeholder is promoted in place and populated.
        assert!(out.contains("## Foo v0.6.0\n\n* Cool foo change"));
        assert!(!out.contains("## Foo v0.6.0 (Unreleased)"));
        // Other sections preserved verbatim.
        assert!(out.contains("## Bar v0.9.0 (Unreleased)"));
        assert!(out.contains("## Foo v0.5.0"));
        assert!(out.contains("* old released"));
        assert!(!out.contains("\n\n\n\n"));
        assert!(out.ends_with('\n'));
        // The new placeholder precedes the promoted release in the file.
        assert!(out.find("## Foo v0.7.0").unwrap() < out.find("## Foo v0.6.0\n").unwrap());
    }

    #[test]
    fn apply_version_update_errors_without_placeholder() {
        let parsed = parse_changelog("# Changelog\n\n## Foo v0.5.0\n\n* released\n").unwrap();
        let cs = comps(vec![("foo", comp(true, "Foo", &[], &["foo/"], &[], &[]))]);
        let merges = vec![pr(Some(1), Some("x"), None, &["foo/x"])];
        let attributed = attribute(&merges, &cs);
        let bumping = compute_bumps(&cs, &attributed);
        let versions: IndexMap<String, SemVer> =
            [("foo".to_string(), semver(0, 6, 0))].into_iter().collect();
        let prev: IndexMap<String, SemVer> = IndexMap::new();
        let ctx = RenderCtx {
            comps: &cs,
            attributed: &attributed,
            bumping: &bumping,
            versions: &versions,
            prev: &prev,
        };
        assert!(apply_version_update(&parsed, "foo", &ctx).is_err());
    }

    // ---- rewrite_version_str --------------------------------------------

    #[test]
    fn rewrite_version_str_toml_and_yaml() {
        let (old, new) =
            rewrite_version_str("name = \"x\"\nversion = \"0.1.0\"\n", true, "0.2.0").unwrap();
        assert_eq!(old, "0.1.0");
        assert!(new.contains("version = \"0.2.0\""));
        assert!(new.ends_with('\n'));

        let (old, new) = rewrite_version_str(
            "name: x\nversion: 0.0.1\nappVersion: \"1\"\n",
            false,
            "0.2.0",
        )
        .unwrap();
        assert_eq!(old, "0.0.1");
        assert!(new.contains("version: 0.2.0"));
        assert!(new.contains("appVersion: \"1\"")); // untouched
    }

    #[test]
    fn rewrite_version_str_only_first_and_errors() {
        // Only the first version line is rewritten.
        let (old, new) =
            rewrite_version_str("version = \"1\"\nversion = \"2\"\n", true, "9").unwrap();
        assert_eq!(old, "1");
        assert!(new.contains("version = \"9\"") && new.contains("version = \"2\""));

        // No version field -> error.
        assert!(rewrite_version_str("name = \"x\"\n", true, "1.0.0").is_err());

        // Missing trailing newline is preserved.
        let (_, new) = rewrite_version_str("version = \"1\"", true, "2").unwrap();
        assert!(!new.ends_with('\n'));
    }

    // ---- pyproject_name / rewrite_lock_version --------------------------

    #[test]
    fn pyproject_name_takes_first_top_level_name() {
        let text = "[project]\nname = \"grafana-dashboards\"\nversion = \"0.7.0\"\n";
        assert_eq!(pyproject_name(text).as_deref(), Some("grafana-dashboards"));
        assert_eq!(pyproject_name("[project]\nversion = \"1\"\n"), None);
    }

    #[test]
    fn rewrite_lock_version_bumps_matching_package() {
        let lock = "[[package]]\nname = \"other\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"grafana-dashboards\"\nversion = \"0.0.0\"\nsource = { editable = \"packages/grafana-dashboards\" }\n";
        let (old, new) = rewrite_lock_version(lock, "grafana-dashboards", "0.7.0").unwrap();
        assert_eq!(old, "0.0.0");
        assert!(new.contains("name = \"grafana-dashboards\"\nversion = \"0.7.0\""));
        assert!(new.contains("name = \"other\"\nversion = \"1.0.0\"")); // untouched
        assert!(new.ends_with('\n'));

        // Unknown package -> error.
        assert!(rewrite_lock_version(lock, "missing", "1.0.0").is_err());
    }
}
