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
- `dependencies` — other components whose bumps cascade into this one.

A component may have an empty `version_paths` (its version lives only in `CHANGELOG.md`) or `changelog: false` (it is rebuilt on dependency changes but keeps no changelog of its own, like `docs`).

## How changes are attributed

Changes are attributed **per merged pull request**, using that merge's own diff (`<merge>^1..<merge>`), not `git log -- <path>`.
A plain path log is unreliable here for two reasons: Git history simplification prunes merge commits, and the `crates/` → `packages/` move means today's paths do not match historical ones.

A PR is attributed to a component when it changes a path under one of that component's `content_paths`.
Each changed file is assigned to the component with the **longest matching** `content_paths` entry; ties resolve to declaration order in `components.yaml`.
A single PR can therefore appear in several streams when it touches several components' paths.

The current `CHANGELOG.md` is the **authoritative baseline**.
The tooling attributes *forward* from the last released ref rather than trying to reconstruct history.

## Cascade

When a PR also touches a component's dependency, the dependent is bumped and records an explicit entry — e.g. `Updated mzmon-lib to v0.5.0` — so a reader can see why a dependent moved without a direct change of its own.
Cascade entries are **additive**: a dependent keeps its own direct bullets and adds the dependency notes.

## How versions are synced

Versions are read from `CHANGELOG.md` at the top of the repo for each component.
When a component is bumped, every file in its `version_paths` is rewritten to the new version.
The next version defaults to a minor bump; the unreleased version can be set manually in the changelog if a different bump is wanted.

## Tooling

`mz-monitoring-build changelog --since <ref> [--until <ref>] [--verbose]` reports which merged PRs each component's changelog would collect, including cascade annotations and any paths owned by no component.
It is currently **read-only** — a preview to validate `components.yaml` against real history.
Writing the `CHANGELOG.md` unreleased sections, bumping `version_paths`, and the release-PR automation below are the next increments.

## Release PR automation (TODO)

Release PRs are created automatically as a draft after any content for a component is released.
Release PRs do not have an inherent version but instead grab the latest version from the changelog.
An existing release PR is updated for all subsequent matching PRs merged.
Publishing behavior is further documented in [Releasing](./releasing.md) after merging.

## Design principles

**Components will change; the tooling must not care.**
`components.yaml` is the only source of truth for the component set, and the tooling is fully data-driven — no component name is hardcoded.
Merging components (e.g. folding Dashboards, Pipelines, alerts, and scrapers into one "Supplemental Assets" stream) is just unioning their `content_paths` under one entry; renaming is a title change.
`CHANGELOG.md` is keyed by component `title` and is append-only history: the tooling treats a title with no prior section as "start fresh" and leaves sections for retired titles untouched as historical record.
A newly merged or renamed stream seeds its starting version by setting the unreleased version manually in `CHANGELOG.md`.

**One shared lib, spanning Rust and Python.**
`mzmon-lib` deliberately covers both ecosystems rather than splitting into per-language streams.
The library is transparent to consumers of the repo — it should not matter to them whether a change was Rust or Python — dependency bumps often land in both at once, and cross-language work (e.g. Datadog dashboards in Rust) touches both.
Per-language splitting is a build-cost optimization we can make later if it is ever warranted; it is not warranted now.

**Some paths are intentionally changelog-exempt.**
`.claude/` (no build impact), `legacy/` (frozen), and most root meta files own no component, so their changes are attributed nowhere by design.
Shared build and CI infra lives in `repo-common` (`changelog: false`), so it is owned but never produces changelog entries or cascade noise.
