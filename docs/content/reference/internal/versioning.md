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
See [Choosing the next version](../releasing/#choosing-the-next-version) for the current pre-1.0 policy and for why that edit is easy to lose, and [Stability guarantees](#stability-guarantees) for what a version bump is allowed to change.

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

## Stability guarantees {#stability-guarantees}

The published version of this is [Stability and Deprecations](../../stability/); the rationale, the surface inventory, and the alternatives considered are in [the design doc](../design-docs/20260823-deprecation-policy/).
This section is the policy of record.

**Surfaces are graded by how much control we have over them.**

| Class | What it covers | Obligation |
|---|---|---|
| **Committed** | Alert names and their `severity`/`component` values; recording-rule names; Terraform inputs and outputs; dashboard identities; metric-tier names; artifact and OCI names; the `monitoring.materialize.cloud/*` namespace | Full cycle below |
| **Coordinated** | `mz_*` metric names and labels — the Materialize product defines them | Disclose always; dual-publish for 30 days wherever our layer can. No cooldown obligation on us, since we do not control upstream timing |
| **No promise** | Query IDs and chart value paths (consumers are our own dashboards and our own Terraform); dashboard internals; subchart values; Kubernetes, Grafana, and Prometheus API shapes | Changelog note when it changes |

**The cycle, for the committed surface:**

1. **Announce** — `stability: deprecated` where the field exists, plus a release-note bullet starting `**Deprecated:**` that names the replacement.
2. **Overlap** — old and new both *work* for at least **30 days**, measured from that release's tag date.
3. **Remove** — a later minor (pre-1.0) or major (post-1.0), with a `**Removed:**` bullet.
   `stability: unsupported` tombstones the identifier so it is never reused for different semantics.

Additions are free, in any release.
A **behavior change is a break**: an alert that keeps its name but fires under materially different conditions goes through the cycle, because routing and runbooks key on the name.

**Ceremony is graded by failure mode, not by surface size.**
A renamed metric leaves a visibly blank panel that a customer can diagnose and fix on their own schedule.
A renamed or removed alert leaves *silence*, and silence during an incident is indistinguishable from health.
So alerts get the strictest treatment — cooldown, dual-publish, explicit notes, a named owner on removal — and metrics get the lightest.
This deliberately inverts the "the label/metric contract is the public API" framing: on a failure-mode reading, metrics are among the most forgiving things we publish.

**Chart values are handled by contact, not by cycle.**
Their consumer is our own Terraform module, which pins a chart version, so a rename is absorbed by bumping the module.
The exception is direct `helm install` users, and that list is short enough to notify individually.

**Enforcement builds nothing.**
Every committed identifier already appears in a generated, committed artifact — `terraform-docs` output for module variables, `pre-rendered/` for dashboards and tiers, `packages/queries/` for alerts — so a rename already shows as a diff in the PR.
[CODEOWNERS](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/CODEOWNERS) covers those paths, and the [release-process check](../releasing/#the-committed-surface-check) is the reviewer's half.

### Designing to avoid the cycle

The cycle is the fallback, not the goal.
Most breaking changes are avoidable at design time, and these are the levers worth reaching for first.

**Publish a recording rule, not a raw metric.**
The strongest lever we have, and unused — the registry defines zero recording rules today.
A `record:` name is ours, and the schema already says the expression behind it may change freely.
So an artifact that reads `mzobject:compute_peek_latency_seconds:p99` instead of a raw `mz_*` metric turns an upstream rename into an expression edit with no customer-visible change at all.
That converts a [coordinated-surface](#stability-guarantees) risk, which we do not control, into an owned one we fully control.

**Add; do not rename.**
Additions are free in any release.
A new identifier alongside the old costs nothing, and the cycle only starts when you want the old one *gone* — which is often not required.
Leaving a superseded alert or output in place is cheaper than removing it, and honest as long as it still works.

**Do not publish what you do not need.**
Every committed identifier is a permanent obligation.
A Terraform output added "just in case", or an alert shipped before anyone routes on it, is a promise nobody asked for.
The cheapest non-breaking change is one to a surface that was never published.

**Template what varies.**
`%%{mzSqlPrefix}` is the working example: one query definition renders against either the `mz_` or `v2_mz_` namespace, so the difference never reaches a name we publish.
Where a value differs by deployment, or looks likely to move, parameterize rather than bake it in.

**Ship new behavior opt-in.**
Adding a value whose default preserves current behavior is not breaking; changing a default is.
Introduce it default-off, then flip the default in a later major.

**Name for the condition, not the grade.**
29 of the 89 alerts end in `-critical`, `-high`, or `-elevated`, and six families differ only by that suffix — so re-grading any of them forces a rename.
Severity belongs in the `severity` label, and the name should describe what is wrong.
This does not remove the exposure, since severity values are themselves committed, but it shrinks it: a label change misroutes an alert that still fires under a name people can still find, where a rename breaks both at once.
Alerts are unshipped, so this is free to fix now and a cycle per alert later.

**When a rename is genuinely required, accept both.**
Keep the old identifier working rather than merely present — for a Terraform variable that means retaining it and letting the new one win, not deleting it and documenting the replacement.
No module variable does this yet, so it is a pattern to establish rather than one to copy.

**Pre-1.0.** Breaking changes may ride minors until [1.0](https://linear.app/materializeinc/issue/DEP-205); the cycle still applies.
Adoption is currently low enough that renames we already know we want should be batched and taken now — see the design doc's [breaking-change budget](../design-docs/20260823-deprecation-policy/#the-pre-10-breaking-change-budget).

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
