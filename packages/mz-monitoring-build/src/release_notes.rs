// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Author-written release notes, read out of a merged PR's description.
//!
//! A PR title says what changed; only the author can say what a *consumer* of
//! the released artifact needs to know about it. That cannot be derived from the
//! diff, so the changelog tooling reads a `### Release Notes` section out of
//! each merged PR's description and nests its bullets under that PR's changelog
//! entry (see [`crate::versioning`]).
//!
//! What counts as a note inside that section:
//!
//! - **List items**, with their relative nesting preserved. Indent width does
//!   not matter — only the nesting the author expressed.
//! - **Headings that carry a link.** This is how renovate summarizes an upstream
//!   changelog: one `### [`v3.2.1`](…)` heading per released version, usually
//!   inside a `<details>` block, and at the same level as the `### Release Notes`
//!   heading itself. Once any heading appears, the prose and bullets beneath it
//!   are the upstream project's detail rather than notes about *this* change, so
//!   only the linked headings survive.
//!
//! The section runs to the next *unlinked* heading at its own level or
//! shallower — an author's `### Testing`, or renovate's `### Configuration`
//! footer — or to the end of the description.
//!
//! HTML comments are stripped first, so a PR template's instructions to the
//! author never reach the changelog. A section that is absent, empty, or says
//! only "None"/"N/A" yields nothing at all.

use std::collections::HashMap;

use crate::github::Gh;
use crate::versioning::REPO_SLUG;

/// One release-note bullet: its text and its nesting depth relative to the top
/// of the notes (0 sits alongside the PR link).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Note {
    pub(crate) depth: usize,
    pub(crate) text: String,
}

/// PR number -> its notes. An absent or empty entry means "no notes".
pub(crate) type NoteMap = HashMap<u64, Vec<Note>>;

/// Best-effort lookup of the release notes for a set of merged PRs.
///
/// Reads `GITHUB_TOKEN` for auth and `GITHUB_REPOSITORY` for the repo, falling
/// back to [`REPO_SLUG`] — the repo whose PRs the changelog already links to.
///
/// Notes are an enrichment, not a gate: with no token, or on a failed read, this
/// warns and yields nothing rather than failing the run. `propose-bumps`
/// rebuilds its branches from scratch on every merge to the default branch, so a
/// transient failure self-heals on the next merge.
pub(crate) fn resolve(numbers: &[u64]) -> NoteMap {
    if numbers.is_empty() {
        return NoteMap::new();
    }
    let Ok(token) = std::env::var("GITHUB_TOKEN") else {
        eprintln!("note: GITHUB_TOKEN not set; changelog entries will omit PR release notes");
        return NoteMap::new();
    };
    let repository = std::env::var("GITHUB_REPOSITORY").unwrap_or_else(|_| REPO_SLUG.to_string());
    match fetch(&token, &repository, numbers) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("note: could not read PR release notes ({e}); omitting them");
            NoteMap::new()
        }
    }
}

/// Fetch each PR's description and extract its notes. A PR that cannot be read
/// is warned about and skipped; only transport/auth setup failures error out.
fn fetch(token: &str, repository: &str, numbers: &[u64]) -> anyhow::Result<NoteMap> {
    let gh = Gh::new(token)?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let mut map = NoteMap::new();
        for &number in numbers {
            let pr = match gh.get(&format!("/repos/{repository}/pulls/{number}")).await {
                Ok(pr) => pr,
                Err(e) => {
                    eprintln!("  #{number}: release notes unavailable: {e}");
                    continue;
                }
            };
            let notes = extract(pr["body"].as_str().unwrap_or_default());
            if !notes.is_empty() {
                map.insert(number, notes);
            }
        }
        anyhow::Ok(map)
    })
}

/// Remove HTML comments, including multi-line ones. An unterminated `<!--`
/// swallows the rest of the body, matching how a markdown renderer treats it.
fn strip_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + "-->".len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// An ATX heading's level and text: `### Foo` -> `(3, "Foo")`.
fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim_start();
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    Some((hashes, line[hashes..].strip_prefix(' ')?.trim()))
}

/// A fenced-code-block delimiter. Fence contents are not markdown, so a `#`
/// line inside one must not be mistaken for a heading.
fn is_fence(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("```") || line.starts_with("~~~")
}

/// A list item's indent (in columns, a tab counting as 4) and its text.
/// Accepts `*`/`-`/`+` and ordered `1.`/`1)` markers.
fn list_item(line: &str) -> Option<(usize, &str)> {
    let mut indent = 0;
    let mut rest = "";
    for (i, c) in line.char_indices() {
        match c {
            ' ' => indent += 1,
            '\t' => indent += 4,
            _ => {
                rest = &line[i..];
                break;
            }
        }
    }
    let text = match rest
        .strip_prefix("* ")
        .or_else(|| rest.strip_prefix("- "))
        .or_else(|| rest.strip_prefix("+ "))
    {
        Some(text) => text,
        None => {
            let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                return None;
            }
            rest[digits..]
                .strip_prefix('.')
                .or_else(|| rest[digits..].strip_prefix(')'))?
                .strip_prefix(' ')?
        }
    };
    Some((indent, text.trim()))
}

/// True when an entry says "nothing to note" rather than carrying a note.
/// Decoration (emphasis, code ticks, trailing punctuation) is ignored so that
/// `**None**` and `_N/A._` read the same as `None`.
fn is_placeholder(text: &str) -> bool {
    let bare: String = text
        .chars()
        .filter(|c| !matches!(c, '*' | '_' | '`' | '~' | '.' | '!'))
        .collect();
    matches!(
        bare.trim().to_ascii_lowercase().as_str(),
        "" | "-" | "none" | "n/a" | "na" | "nothing" | "no release notes" | "tbd"
    )
}

/// Drop placeholder entries along with anything nested beneath them: an entry
/// with nothing to say has no children worth keeping either.
fn drop_placeholders(notes: Vec<Note>) -> Vec<Note> {
    let mut out: Vec<Note> = Vec::new();
    let mut skip_below: Option<usize> = None;
    for note in notes {
        if let Some(depth) = skip_below {
            if note.depth > depth {
                continue;
            }
            skip_below = None;
        }
        if is_placeholder(&note.text) {
            skip_below = Some(note.depth);
            continue;
        }
        out.push(note);
    }
    out
}

/// Extract the release notes from a PR description. Empty when the description
/// has no `### Release Notes` section, or the section says nothing.
pub(crate) fn extract(body: &str) -> Vec<Note> {
    let body = strip_comments(body);
    let mut lines = body.lines();
    let mut fenced = false;

    // Locate the section heading. Its level sets the section's extent: the
    // notes run until the next heading at that level or shallower.
    let mut section_level = None;
    for line in lines.by_ref() {
        if is_fence(line) {
            fenced = !fenced;
        } else if !fenced
            && let Some((level, text)) = heading(line)
            && text
                .trim_end_matches(':')
                .trim()
                .eq_ignore_ascii_case("release notes")
        {
            section_level = Some(level);
            break;
        }
    }
    let Some(section_level) = section_level else {
        return Vec::new();
    };

    let mut notes = Vec::new();
    // Source indents seen so far, innermost last: the stack's height is the
    // nesting depth, so any indent width (or mix of them) maps to a level.
    let mut indents: Vec<usize> = Vec::new();
    let mut seen_heading = false;
    // Renovate nests its per-version headings inside `<details>`; those are part
    // of the notes, so only a heading at depth 0 can close the section.
    let mut details = 0usize;

    for line in lines {
        if is_fence(line) {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        details += line.matches("<details").count();
        details = details.saturating_sub(line.matches("</details>").count());

        if let Some((level, text)) = heading(line) {
            // A linked heading is a note wherever it sits — renovate does not
            // always wrap its per-version headings in `<details>`, and it emits
            // them at the same level as the `### Release Notes` heading itself.
            // An unlinked heading at that level or shallower, outside any
            // `<details>`, is a genuine new section and ends the notes.
            let linked = text.contains("](");
            if !linked && details == 0 && level <= section_level {
                break;
            }
            seen_heading = true;
            if linked {
                notes.push(Note {
                    depth: 0,
                    text: text.to_string(),
                });
            }
        } else if !seen_heading && let Some((indent, text)) = list_item(line) {
            while indents.last().is_some_and(|&outer| indent < outer) {
                indents.pop();
            }
            if indents.last() != Some(&indent) {
                indents.push(indent);
            }
            notes.push(Note {
                depth: indents.len() - 1,
                text: text.to_string(),
            });
        }
    }

    drop_placeholders(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(depth, text)` pairs, for terser assertions.
    fn flat(notes: &[Note]) -> Vec<(usize, &str)> {
        notes.iter().map(|n| (n.depth, n.text.as_str())).collect()
    }

    // ---- strip_comments -------------------------------------------------

    #[test]
    fn strip_comments_single_and_multi_line() {
        assert_eq!(strip_comments("a <!-- x --> b"), "a  b");
        assert_eq!(strip_comments("a\n<!--\nx\ny\n-->\nb"), "a\n\nb");
        assert_eq!(strip_comments("a <!-- b"), "a "); // unterminated
        assert_eq!(strip_comments("plain"), "plain");
        assert_eq!(strip_comments("<!--a--><!--b-->c"), "c");
    }

    // ---- heading / is_fence / list_item ---------------------------------

    #[test]
    fn heading_levels_and_rejects() {
        assert_eq!(heading("### Foo"), Some((3, "Foo")));
        assert_eq!(heading("# Foo  "), Some((1, "Foo")));
        assert_eq!(heading("###### Foo"), Some((6, "Foo")));
        assert_eq!(heading("####### Foo"), None); // deeper than h6
        assert_eq!(heading("###Foo"), None); // no space
        assert_eq!(heading("not a heading"), None);
    }

    #[test]
    fn list_item_markers_and_indent() {
        assert_eq!(list_item("* a"), Some((0, "a")));
        assert_eq!(list_item("  - a"), Some((2, "a")));
        assert_eq!(list_item("\t+ a"), Some((4, "a")));
        assert_eq!(list_item("1. a"), Some((0, "a")));
        assert_eq!(list_item("  12) a"), Some((2, "a")));
        assert_eq!(list_item("---"), None); // thematic break
        assert_eq!(list_item("*"), None); // empty marker
        assert_eq!(list_item("text"), None);
        assert_eq!(list_item("   "), None);
    }

    // ---- is_placeholder -------------------------------------------------

    #[test]
    fn is_placeholder_variants() {
        for text in [
            "None", "none", "N/A", "n/a", "NA", "**None**", "_N/A._", "`none`", "-", "",
        ] {
            assert!(is_placeholder(text), "{text:?} should be a placeholder");
        }
        for text in [
            "Nonexistent",
            "None of the alert rules fire twice",
            "Added foo",
        ] {
            assert!(
                !is_placeholder(text),
                "{text:?} should not be a placeholder"
            );
        }
    }

    // ---- extract: no notes ----------------------------------------------

    #[test]
    fn extract_without_section_or_content() {
        assert!(extract("Just a description.\n").is_empty());
        assert!(extract("### Release Notes\n").is_empty());
        assert!(extract("### Release Notes\n\n* None\n").is_empty());
        assert!(extract("### Release Notes\n\n* N/A\n").is_empty());
        // The template's placeholder and its instructions both drop out.
        assert!(
            extract("### Release Notes\n\n<!--\nWrite notes here.\n-->\n\n* None\n").is_empty()
        );
        // A placeholder's children go with it.
        assert!(extract("### Release Notes\n\n* None\n    * nothing here\n").is_empty());
    }

    // ---- extract: author-written bullets --------------------------------

    #[test]
    fn extract_preserves_bullet_hierarchy() {
        let body = "Some prose.\n\n### Release Notes\n\n* Added `alloy.extraArgs`\n  * Defaults to `[]`\n    * Ignored on the CRDs chart\n* Removed the legacy scraper\n";
        assert_eq!(
            flat(&extract(body)),
            vec![
                (0, "Added `alloy.extraArgs`"),
                (1, "Defaults to `[]`"),
                (2, "Ignored on the CRDs chart"),
                (0, "Removed the legacy scraper"),
            ]
        );
    }

    #[test]
    fn extract_stops_at_next_same_or_shallower_heading() {
        let body = "### Release Notes\n\n* Kept\n\n### Testing\n\n* Dropped\n";
        assert_eq!(flat(&extract(body)), vec![(0, "Kept")]);

        let body = "## Release Notes\n\n* Kept\n\n# Checklist\n\n* Dropped\n";
        assert_eq!(flat(&extract(body)), vec![(0, "Kept")]);
    }

    #[test]
    fn extract_ignores_fenced_content() {
        let body = "### Release Notes\n\n* Run the migration\n\n```console\n# not a heading\n* not a bullet\n```\n\n* Then restart\n";
        assert_eq!(
            flat(&extract(body)),
            vec![(0, "Run the migration"), (0, "Then restart")]
        );
    }

    #[test]
    fn extract_accepts_heading_spellings() {
        assert_eq!(
            flat(&extract("#### release notes:\n\n* a\n")),
            vec![(0, "a")]
        );
        assert!(extract("### Release Note\n\n* a\n").is_empty()); // not the section
    }

    // ---- extract: renovate ----------------------------------------------

    #[test]
    fn extract_renovate_keeps_linked_version_headings() {
        // Shape of a renovate body: a table, the notes as linked headings inside
        // a <details>, then renovate's own `### Configuration` footer.
        let body = "\
This PR contains the following updates:

| Package | Update |
|---|---|
| kubernetes | major |

---

### Release Notes

<details>
<summary>hashicorp/terraform-provider-kubernetes (kubernetes)</summary>

### [`v3.2.1`](https://redirect.example.com/CHANGELOG.md#321)

[Compare Source](https://redirect.example.com/compare/v3.2.0...v3.2.1)

BUG FIXES:

- `resource/*`: Fix an identity change error
- `resource/kubernetes_secret_v1`: Fix a create error

### [`v3.2.0`](https://redirect.example.com/CHANGELOG.md#320)

ENHANCEMENTS:

- Added a linux/s390x build target

</details>

---

### Configuration

- Branch creation
  - At any time

---

 - [ ] <!-- rebase-check -->If you want to rebase/retry this PR, check this box
";
        assert_eq!(
            flat(&extract(body)),
            vec![
                (
                    0,
                    "[`v3.2.1`](https://redirect.example.com/CHANGELOG.md#321)"
                ),
                (
                    0,
                    "[`v3.2.0`](https://redirect.example.com/CHANGELOG.md#320)"
                ),
            ]
        );
    }

    #[test]
    fn extract_drops_upstream_detail_under_headings() {
        // Bullets before the first heading are the author's; bullets after it
        // belong to the upstream changelog the heading links to.
        let body =
            "### Release Notes\n\n* Ours\n\n### [`v1.2.3`](https://example.com/cl)\n\n* Theirs\n";
        assert_eq!(
            flat(&extract(body)),
            vec![(0, "Ours"), (0, "[`v1.2.3`](https://example.com/cl)")]
        );
    }

    #[test]
    fn extract_ignores_unlinked_headings_inside_the_section() {
        let body = "### Release Notes\n\n#### Upstream\n\n* dropped\n";
        assert!(extract(body).is_empty());
    }
}
