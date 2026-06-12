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

## `propose-bumps` (runs on merge to the default branch)

`mz-monitoring-build propose-bumps` is the command that maintains the version-update PRs. For each changelog-enabled component with changes since its last release tag, it:

- recreates the `version-update/<component>` branch as a **single commit atop the base**, applying that component's [`release`](versioning/) changelog + version + `uv.lock` edits (the version is not in the branch name);
- force-pushes the branch (stateless — it never reconciles the PR's current state) and opens one PR per component if none is open.

The PR body is the component's released changelog section. New PRs are labeled `auto-format` (`--label`, empty to disable) so the [auto-format](#auto-format) workflow can fix anything the commit cannot regenerate.

It is **repository-agnostic** — owner/repo and the base commit come from the environment — so another repository can adopt it unchanged.

Required environment:

| Variable | Purpose |
|---|---|
| `CI=true` | The command refuses to run otherwise (set it to emulate CI locally). |
| `GITHUB_TOKEN` | Auth; needs `contents: write` and `pull-requests: write`. |
| `GITHUB_REPOSITORY` | `owner/repo` (set by GitHub Actions). |
| `GITHUB_SHA` | Base commit the branches build on; falls back to `git rev-parse HEAD`. |

`--dry-run` prints the plan and makes no GitHub calls (still requires `CI=true`).
`--draft` opens PRs as drafts (the default in our workflow for now); draft state blocks accidental merges.
`--automerge` best-effort enables auto-merge on newly opened PRs.

A minimal workflow:

```yaml
on:
  push:
    branches: [main]
permissions:
  contents: write
  pull-requests: write
jobs:
  propose-bumps:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # tags + full history for attribution
      - run: cargo run -p mz-monitoring-build -- propose-bumps --draft --automerge
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Bootstrapping:** the per-component "since" boundary is the tag `<component>/v<latest released>`. Create those tags at the current release point before the first run (e.g. `git tag mzmon-lib/v0.5.0 <commit>`); a component with no prior release or missing tag is skipped with a message. `propose-bumps` does **not** create tags or releases — that is `publish-release` below.

## `publish-release` (runs when a version-update PR merges)

`mz-monitoring-build publish-release --component <name> --sha <commit>` reads the component's latest released section from `CHANGELOG.md`, creates the `<component>/vX.Y.Z` tag at `--sha`, and publishes a GitHub Release whose notes are that section (heading dropped — the release name carries it). It is **idempotent**: if the tag already exists it does nothing, and `make_latest=false` since each component is an independent stream.

It runs off the PR *merge* (not pushes to the default branch), via the [`publish-release`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/workflows/publish-release.yaml) workflow gated on `version-update/*` head branches; the component is the branch name minus the `version-update/` prefix and the tag target is the merge commit. The default `GITHUB_TOKEN` is sufficient — nothing needs to chain off the tag or release. Same env contract as `propose-bumps` minus `pull-requests` (it only needs `contents: write`); `--dry-run` prints the tag and notes without calling GitHub.

## Auto-format

`propose-bumps` builds branches via the GitHub API, so it cannot run formatters; the bump commit therefore leaves generated artifacts stale (e.g. the `helm-docs` chart README badge after a Chart.yaml version bump). Rather than install a toolchain in `propose-bumps`, the [`auto-format`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/workflows/auto-format.yaml) workflow runs the repo's formatters (`make helm-docs`, `cargo fmt`, `ruff`) on any PR labeled `auto-format` and pushes a single `style:` commit if anything changed. The same mechanism covers GitHub UI edits and renovate PRs — just apply the label.

**Token requirement:** a label/PR event raised by the default `GITHUB_TOKEN` does **not** trigger other workflows (GitHub's loop-prevention). For `auto-format` to fire from `propose-bumps`, `propose-bumps` must authenticate with a **PAT or GitHub App token**, not the default `GITHUB_TOKEN`. The auto-format commit itself is pushed with the default `GITHUB_TOKEN`, which conveniently does not re-trigger the workflow.

`propose-bumps` still syncs `uv.lock` inline for now; once auto-format reliably handles lockfiles that inline logic can be dropped (deferred — only generated docs were stale in practice).

## Cascade and ordering

- Releasing a dependency updates its dependents' version-update PRs (cascade), recording an `Included <dep> @ vPREV..vNEW` entry.
- The wording is "Included" rather than "Updated" because a dependent may reference a dependency version that is queued but not yet released.
- When the tag must exist (e.g. for a release artifact that pins the dependency), release dependencies before their dependents.

## Open questions

- The default next version is a minor bump; a breaking change needs the placeholder version edited manually before release.
- Cascade can fan out: releasing a low-level shared component updates every dependent's version-update PR, so expect merge-order sensitivity across concurrent release PRs.
- `version_paths` now track the latest **released** version (bumped by the version-update PR), not the latest unreleased — reconcile the wording in [Versioning](versioning/) when the tooling lands.
