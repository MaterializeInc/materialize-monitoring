---
title: "Versioning"
weight: 90
---

# Versioning in materialize-monitoring

All artifacts of this repo have separate versioning streams.
They generally follow semantic versioning (SemVer).

## Components

Components are defined in `packages/components.yaml`.
Each component declares:

- `changelog` — whether the component maintains its own changelog stream.
- `title` — the human-readable name used in `CHANGELOG.md` headings.
- `version_paths` — files whose version field is rewritten when the component is bumped (the *write targets*).
- `content_paths` — the paths whose changes are attributed to this component (the *attribution inputs*).
- `content_exclude` — paths to subtract from `content_paths`, typically generated outputs that belong to a dependency.
- `dependencies` — other components whose bumps cascade into this one.
- `artifacts` — glob patterns (repo-root-relative) for files attached as GitHub Release assets when the component is published; resolved file names must be unique.

Each changed file is attributed to the component with the longest matching `content_paths` entry, after dropping any component that excludes it.
Generated outputs route to their source: the chart excludes its `pre-rendered/` tree, and `pre-rendered/dashboards` and `pre-rendered/pipelines` are claimed by the `dashboards` and `pipelines` components, so a dashboard change appears under Dashboards (and rolls up into the chart via cascade) rather than as a first-class chart change.

A component may have an empty `version_paths` (its version lives only in `CHANGELOG.md`) or `changelog: false` (it is rebuilt on dependency changes but keeps no changelog of its own, like `docs`).

## How changes are attributed

Changes are attributed **per merged pull request**, using that commit's own diff (`<commit>^1..<commit>`), not `git log -- <path>`.
A plain path log is unreliable here for two reasons: Git history simplification prunes merge commits, and the `crates/` → `packages/` move means today's paths do not match historical ones.

PRs are found by walking the **first-parent** history of the range, and both of GitHub's merge styles are recognized, because this repo uses both:

| Style | Parents | Subject | PR title from |
|---|---|---|---|
| Merge commit | 2 | `Merge pull request #N from <branch>` | first non-empty line of the body |
| Squash commit | 1 | the PR title, with a `(#N)` suffix | the subject, suffix stripped |

Most human PRs land as merge commits and most renovate PRs land as squash commits, so recognizing only the first would drop a large share of dependency bumps — including the ones whose descriptions carry the richest [release notes](../releasing/#release-notes-from-pr-descriptions).

A merge commit is itself evidence of an integration event, so one without a `#N` still earns an entry, labeled by branch or short hash.
A lone commit is not — it could be any direct push to the branch — so without a `(#N)` there is no PR to attribute it to and it is skipped.
`changelog --verbose` lists what it skipped under `== Commits with no PR reference ==`, so a genuinely missed PR is visible rather than silently dropped.

A PR is attributed to a component when it changes a path under one of that component's `content_paths`.
Each changed file is assigned to the component with the **longest matching** `content_paths` entry; ties resolve to declaration order in `components.yaml`.
A single PR can therefore appear in several streams when it touches several components' paths.

The current `CHANGELOG.md` is the **authoritative baseline**.
The tooling attributes *forward* from the last released ref rather than trying to reconstruct history.

## Cascade

A component's section lists its first-class changes — the PRs that touched its own paths — followed by a `### Dependencies` subsection.
When a dependency bumps, the dependent bumps too and records an `Included <dep> @ vPREV..vNEW` entry under `### Dependencies`, with that dependency's own PRs nested beneath it, recursively through the dependency graph.
The range spans the dependency's latest released version to the version this release includes; a brand-new dependency with no prior release shows a single version.
"Included" rather than "Updated" because the new version need not be released yet.
This keeps each component's release notes self-contained and cumulative: the detail travels with the rollup, rather than a bare "updated to vX.Y.Z" with no context.
A PR already shown as a first-class change in a section is not repeated under that section's dependencies, and each dependency is rolled up once per section.
When a PR touches two sibling dependencies, the one declared first in `dependencies` claims it; order the main content components ahead of shared/common ones so changes surface in the more specific stream.
A single PR can still appear in several components' sections; that duplication is intentional, so each component's release reads completely on its own.

Under each PR's bullet sit its link and then any **release notes** the author wrote in the PR description, so a reader gets the consumer-facing detail without opening the PR.
See [Release notes from PR descriptions](../releasing/#release-notes-from-pr-descriptions) for what is picked up and what is dropped.

## How versions are synced

Versions are read from `CHANGELOG.md` for each component.
Unreleased sections are `_Changes Pending_` placeholders; a version-update PR populates a placeholder, promotes it to a released section, and rewrites that component's `version_paths` to the released version (see [Releasing](../releasing/)).
Bumping a `pyproject.toml` also rewrites the matching package's `version` in `uv.lock`, so the lockfile does not drift behind the version files.
The next version defaults to a minor bump; **a patch or a major is expressed by editing the placeholder heading**, which the tooling reads and never overrides.
See [Choosing the next version](../releasing/#choosing-the-next-version) for the current pre-1.0 policy and for why that edit is easy to lose.

## Tooling

Both subcommands live in `mz-monitoring-build` and default to a dry run; `--write` (on `release`) applies changes.

- `mz-monitoring-build changelog --since <ref> [--until <ref>] [--verbose]` is **read-only**: it reports which merged PRs each component would collect and the version each would bump to — a preview for validating `components.yaml` against real history.
- `mz-monitoring-build release --component <name> --since <ref> [--write]` generates a `version-update/<component>` PR: it promotes that component's `_Changes Pending_` placeholder in place into a populated released section, inserts a fresh placeholder at the top, and bumps the component's `version_paths`.
  With a `GITHUB_TOKEN` in the environment it also reads the merged PRs' descriptions for their release notes; without one it prints a warning and omits them, so its output is otherwise reproducible offline.

The shared logic (component model, changelog parsing, attribution, cascade rendering, version rewriting) lives in the `versioning` module and is unit-tested without invoking git.
Release-note extraction lives alongside it in `release_notes`, and is likewise unit-tested against real PR-description shapes without touching the network.

## Release PR automation

The orchestration that drives `release` from CI is built: `propose-bumps` runs on every merge to `main` and creates or updates the `version-update/*` PRs for components with changes, `publish-release` creates the `<component>/vX.Y.Z` tag and GitHub Release when such a PR merges.
The per-component tag doubles as that component's `--since` boundary.
See [Releasing](../releasing/) for the full state machine and the workflows that drive it.

## Design principles

**Components will change; the tooling must not care.**
`components.yaml` is the only source of truth for the component set, and the tooling is fully data-driven — no component name is hardcoded.
Merging components (e.g. folding Dashboards, Pipelines, alerts, and scrapers into one "Supplemental Assets" stream) is just unioning their `content_paths` under one entry; renaming is a title change.
`CHANGELOG.md` is keyed by component `title` and is append-only history: the tooling treats a title with no prior section as "start fresh" and leaves sections for retired titles untouched as historical record.
A newly merged or renamed stream seeds its starting version by setting the unreleased version manually in `CHANGELOG.md`.

Renaming has one consequence worth knowing before you do it: the changelog is keyed by `title`, so the lookup for a component's latest release only matches sections carrying the *current* title.
The first release attempt after a rename therefore reports no prior release and skips the component.
The fix is to rename the latest released section's heading to match — older sections can keep the old title, since only the newest one is consulted.

**One shared lib, spanning Rust and Python.**
`mzmon-lib` deliberately covers both ecosystems rather than splitting into per-language streams.
The library is transparent to consumers of the repo — it should not matter to them whether a change was Rust or Python — dependency bumps often land in both at once, and cross-language work (e.g. Datadog dashboards in Rust) touches both.
Per-language splitting is a build-cost optimization we can make later if it is ever warranted; it is not warranted now.

**Some paths are intentionally changelog-exempt.**
`.claude/` (no build impact), `legacy/` (frozen), and most root meta files own no component, so their changes are attributed nowhere by design.
Shared build and CI infra lives in `repo-common` (`changelog: false`), so it is owned but never produces changelog entries or cascade noise.
