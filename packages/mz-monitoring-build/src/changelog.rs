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
//! (transitively) to changelog-enabled dependents, which record an
//! "Included <dep> @ vPREV..vNEW" entry (single version when there is no prior
//! release), in addition to listing the PRs that touched its own paths directly.
//! A PR touching several components appears in each, so every release's notes
//! read on their own.
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
fn latest_released(title: &str, parsed: &ParsedChangelog) -> Option<SemVer> {
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

/// Rebuild the full changelog text: the preserved header, regenerated unreleased
/// sections (in `comps` order) for bumping components, any unreleased sections we
/// did not regenerate, then the released sections verbatim.
fn regenerate(parsed: &ParsedChangelog, ctx: &RenderCtx<'_>) -> String {
    let bumping_titles: IndexSet<&str> = ctx
        .bumping
        .iter()
        .map(|n| ctx.comps[n].title.as_str())
        .collect();

    let mut out: Vec<String> = parsed.header.clone();
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out.push(String::new());

    for (name, c) in ctx.comps {
        if !c.changelog || !ctx.bumping.contains(name) {
            continue;
        }
        out.push(format!(
            "## {} {} (Unreleased)",
            c.title,
            ctx.versions[name].changelog()
        ));
        out.push(String::new());
        out.extend(render_section(name, c, ctx));
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

    let mut text = out.join("\n");
    text.push('\n');
    // Collapse any run of 3+ blank lines introduced at section seams.
    while text.contains("\n\n\n\n") {
        text = text.replace("\n\n\n\n", "\n\n\n");
    }
    text
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

/// Main entrypoint for the `changelog` command.
pub fn changelog(args: ChangelogArgs) -> anyhow::Result<()> {
    let comps_text = std::fs::read_to_string(&args.components)
        .with_context(|| format!("reading {}", args.components.display()))?;
    let comps = serde_yaml_ng::from_str::<ComponentsFile>(&comps_text)
        .with_context(|| format!("parsing {}", args.components.display()))?
        .components;

    let merges = collect_merges(&args.since, &args.until)?;
    let attributed = attribute(&merges, &comps);
    let bumping = compute_bumps(&comps, &attributed);

    let changelog_text = std::fs::read_to_string(&args.changelog)
        .with_context(|| format!("reading {}", args.changelog.display()))?;
    let parsed = parse_changelog(&changelog_text)
        .with_context(|| format!("parsing {}", args.changelog.display()))?;

    // Resolve the version each bumping component will carry, and its latest
    // released version (for the `@ vPREV..vNEW` range on rollups).
    let versions: IndexMap<String, SemVer> = bumping
        .iter()
        .map(|name| (name.clone(), next_version(&comps[name].title, &parsed)))
        .collect();
    let prev: IndexMap<String, SemVer> = bumping
        .iter()
        .filter_map(|name| latest_released(&comps[name].title, &parsed).map(|v| (name.clone(), v)))
        .collect();

    let ctx = RenderCtx {
        comps: &comps,
        attributed: &attributed,
        bumping: &bumping,
        versions: &versions,
        prev: &prev,
    };
    let new_changelog = regenerate(&parsed, &ctx);

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

    // ---- regenerate -----------------------------------------------------

    #[test]
    fn regenerate_rewrites_unreleased_preserves_rest() {
        let text = "# Changelog\n\n## Foo v0.2.0 (Unreleased)\n\n* old\n\n## Bar v0.9.0 (Unreleased)\n\n* keep bar\n\n## Foo v0.1.0\n\n* released foo\n";
        let parsed = parse_changelog(text).unwrap();
        let cs = comps(vec![("foo", comp(true, "Foo", &[], &["foo/"], &[], &[]))]);
        let merges = vec![pr(Some(5), Some("New foo"), None, &["foo/x"])];
        let attributed = attribute(&merges, &cs);
        let bumping = compute_bumps(&cs, &attributed);
        let versions: IndexMap<String, SemVer> = [(
            "foo".to_string(),
            SemVer {
                major: 0,
                minor: 2,
                patch: 0,
            },
        )]
        .into_iter()
        .collect();
        let prev: IndexMap<String, SemVer> = [(
            "foo".to_string(),
            SemVer {
                major: 0,
                minor: 1,
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

        let out = regenerate(&parsed, &ctx);
        assert!(out.contains("## Foo v0.2.0 (Unreleased)"));
        assert!(out.contains("* New foo"));
        assert!(!out.contains("* old")); // regenerated, old content dropped
        assert!(out.contains("## Bar v0.9.0 (Unreleased)")); // unbumped unreleased preserved
        assert!(out.contains("* keep bar"));
        assert!(out.contains("## Foo v0.1.0")); // released preserved
        assert!(out.contains("* released foo"));
        assert!(!out.contains("\n\n\n\n")); // no 4-blank runs
        assert!(out.ends_with('\n'));
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
}
