---
title: "Releasing"
weight: 80
---

# Releasing

Each artifact releases on its own version stream (see [Versioning](versioning/)).

<!-- The state machine below is the intended design; the tooling to drive it is
not built yet. Today `mz-monitoring-build changelog --write` populates and
hoists unreleased sections directly — that write path will be reworked to drive
version-update PRs as described here. Until then, do not run `--write` against
the placeholder-style CHANGELOG.md on main; it would clobber the placeholders. -->

## Changes-pending placeholders

The top of `CHANGELOG.md` holds one **unreleased placeholder per component that has changes**, with the body `_Changes Pending_`.
Placeholders are not populated on `main` — population happens in a version-update PR (below).
A component with no changes since its last release has no placeholder, so the unreleased placeholders are not necessarily the very top entries and rarely-changed components are not churned every cycle.

## Version-update PRs

A "Release `<component>` vX.Y.Z" PR (branch `version-update/<component>`):

- Replaces that component's `_Changes Pending_` placeholder **in place** with the real entries and drops `(Unreleased)`, promoting it to a released section.
- Inserts a fresh `_Changes Pending_` placeholder for the next version at the **top** of the file.
- Bumps the component's `version_paths` to the released version.

The released version stays at its original location; only the new placeholder is hoisted to the top.
A released section can therefore sit above other components' unreleased placeholders — the changelog parser is order-independent, so this is fine.

## State machine

- **Any merge to `main`** attempts to create or update the `version-update/*` PRs for every component with changes since its last release (a component with no changes gets no PR).
- **Tags** `<component>/vX.Y.Z` are created when a `version-update/*` PR merges (potentially after more extensive CI).
- **GitHub Releases** are created when a tag is created.
- Per-component tags double as the per-component "since" boundary for attribution, so each stream's changelog window is computed from its own last release.

## Cascade and ordering

- Releasing a dependency updates its dependents' version-update PRs (cascade), recording an `Included <dep> @ vPREV..vNEW` entry.
- The wording is "Included" rather than "Updated" because a dependent may reference a dependency version that is queued but not yet released.
- When the tag must exist (e.g. for a release artifact that pins the dependency), release dependencies before their dependents.

## Open questions

- The default next version is a minor bump; a breaking change needs the placeholder version edited manually before release.
- Cascade can fan out: releasing a low-level shared component updates every dependent's version-update PR, so expect merge-order sensitivity across concurrent release PRs.
- `version_paths` now track the latest **released** version (bumped by the version-update PR), not the latest unreleased — reconcile the wording in [Versioning](versioning/) when the tooling lands.
