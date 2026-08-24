---
title: "Releasing"
weight: 80
---

# Releasing

Each artifact releases on its own version stream (see [Versioning](../versioning/)).

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

## Choosing the next version {#choosing-the-next-version}

**The placeholder heading is the decision.**
The tooling reads the version out of it and never overrides it — `release` writes a fresh placeholder at `bump_minor()` of what it just released, and that is only a default for the *next* cycle.
Anything other than a minor is driven by editing that heading before the release goes out.

```markdown
## <title> v0.11.1 (Unreleased)   ← edited down from the v0.12.0 the tooling wrote
```

Current policy, pre-1.0:

- **Patch** — small, low-risk changes. Preferred, and kept genuinely small now that releases are published more often.
- **Minor** — a batch of features, or anything a consumer has to react to (a new required value, a changed default, a new Terraform variable).
- **Major** — not stamped yet. Until it is, a breaking change goes in a minor with the break called out in the entry.

> [!WARNING]
>   That heading is the *only* place the intended bump is recorded, and it lives uncommitted in your working tree until you push it.
>   A stray `git checkout -- CHANGELOG.md`, or a tool that rewrites the file, silently reverts the decision to the tooling's minor default — and the next release goes out as a minor with no diff to show why.
>   Commit the edit as its own change when you make it.

Two other things are expressed by editing this heading rather than by a flag: seeding a newly merged or renamed component stream at a starting version, and re-baselining after a component `title` change (see [Versioning](../versioning/)).

## State machine

- **Any merge to `main`** attempts to create or update the `version-update/*` PRs for every component with changes since its last release (a component with no changes gets no PR).
- **Tags** `<component>/vX.Y.Z` are created when a `version-update/*` PR merges (potentially after more extensive CI).
- **GitHub Releases** are created when a tag is created.
- Per-component tags double as the per-component "since" boundary for attribution, so each stream's changelog window is computed from its own last release.

## `propose-bumps` (runs on merge to the default branch)

`mz-monitoring-build propose-bumps` is the command that maintains the version-update PRs. For each changelog-enabled component with changes since its last release tag, it:

- recreates the `version-update/<component>` branch as a **single commit atop the base**, applying that component's [`release`](../versioning/) changelog + version + `uv.lock` edits (the version is not in the branch name);
- force-pushes the branch (stateless) and either opens the PR or refreshes the open one's title/body so the description tracks the new commit.

The PR body is the component's released changelog section. New PRs are labeled `auto-format` (`--label`, empty to disable) so the [auto-format](#auto-format) workflow can fix anything the commit cannot regenerate.

It is **repository-agnostic** — owner/repo and the base commit come from the environment — so another repository can adopt it unchanged.

Required environment:

| Variable | Purpose |
|---|---|
| `CI=true` | The command refuses to run otherwise (set it to emulate CI locally). |
| `GITHUB_TOKEN` | Auth; needs `contents: write` and `pull-requests: write`. |
| `GITHUB_REPOSITORY` | `owner/repo` (set by GitHub Actions). |
| `GITHUB_SHA` | Base commit the branches build on; falls back to `git rev-parse HEAD`. |

`--dry-run` prints the plan and makes no *mutating* GitHub calls (still requires `CI=true`); it does still read PR descriptions for their release notes, so the plan it computes is the one a real run would.
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

## Release notes from PR descriptions

A PR title says what changed; only the author can say what a *consumer* of the released artifact needs to know about it.
That cannot be derived from the diff, so the changelog tooling reads a `### Release Notes` section out of each merged PR's description and nests its bullets under that PR's changelog entry, alongside the PR link.

The [PR template](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/pull_request_template.md) seeds the section with `* None`, which is dropped — so the default is to say nothing, and an author opts in by replacing it.

```markdown
### Release Notes

* `alloy.extraArgs` is new, defaulting to `[]`
  * Ignored by the CRDs chart
```

lands in the changelog as:

```markdown
* Support extra alloy arguments
    * [materialize-monitoring#281](https://github.com/MaterializeInc/materialize-monitoring/pull/281)
    * `alloy.extraArgs` is new, defaulting to `[]`
        * Ignored by the CRDs chart
```

What gets picked up:

- **List items** under the heading, with their relative nesting preserved. Indent width does not matter, only the nesting the author expressed.
  An item may span several source lines — wrapped for line length, or written a sentence per line, as this repo's [Markdown conventions](../contributing/#markdown-conventions) ask — and the continuation is folded back into the item, since that is how a Markdown renderer displays it anyway.
  A blank line, a `---` break, a fenced block, or a raw HTML block closes the item.
- **Headings that carry a link.** This is how renovate summarizes an upstream changelog — one ``### [`v3.2.1`](…)`` heading per released version, usually wrapped in a `<details>` block — so a dependency bump contributes a linked entry per upstream version. Once any heading appears, the prose and bullets *beneath* it are the upstream project's detail rather than notes about this change, so only the linked headings survive.

What gets dropped: HTML comments (so the template's instructions never reach the changelog), an absent or empty section, and a section that says only `None`/`N/A` — along with anything nested under such an entry.
The section ends at the next *unlinked* heading at its own level or shallower (an author's `### Testing`, renovate's `### Configuration` footer), or at the end of the description.

Both of GitHub's merge styles are read, so a squash-merged renovate PR contributes its notes just as a merge-committed one does (see [How changes are attributed](../versioning/#how-changes-are-attributed)).

Notes are an **enrichment, not a gate**. With no `GITHUB_TOKEN`, or when a description cannot be read, the run warns and continues without them rather than failing — and since `propose-bumps` rebuilds its branches from scratch on every merge to the default branch, a transient failure self-heals on the next merge.
The descriptions are fetched once for the union of all components' windows, since one PR often lands in several.

> [!NOTE]
>   The notes are read from the PR description **as it stands when `propose-bumps` runs**, not as it stood at merge time.
>   Editing a merged PR's description therefore still changes what the next run writes, right up until the version-update PR merges — which is the escape hatch when a note was wrong or missing.

## `publish-release` (runs when a version-update PR merges)

`mz-monitoring-build publish-release --component <name> --sha <commit>` reads the component's latest released section from `CHANGELOG.md`, creates the `<component>/vX.Y.Z` tag at `--sha`, and publishes a GitHub Release whose notes are that section (heading dropped — the release name carries it). It is **idempotent**: if the tag already exists it does nothing, and `make_latest=false` since each component is an independent stream.

It runs off the PR *merge* (not pushes to the default branch), via the [`publish-release`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/workflows/publish-release.yaml) workflow gated on `version-update/*` head branches; the component is the branch name minus the `version-update/` prefix and the tag target is the merge commit. The default `GITHUB_TOKEN` is sufficient — nothing needs to chain off the tag or release. Same env contract as `propose-bumps` minus `pull-requests` (it only needs `contents: write`); `--dry-run` prints the tag, notes, and resolved assets without calling GitHub.

A component's `artifacts` globs (in `components.yaml`) are attached to the release as assets.
They resolve against the checked-out tree, so **committed** artifacts (e.g. `pre-rendered/` dashboards, `docs/assets/`) work as-is.
**Build-output** artifacts (e.g. the packaged chart `.tgz`, which is gitignored) are built earlier in the workflow: for chart components the job checks out with LFS, sets up Helm, and runs `helm package charts/<component>` before the release step so the `.tgz` resolves.
We package the committed chart directly rather than `make charts` — its pre-rendered inputs are already committed and LFS-hydrated, so packaging reproduces exactly what is on the release commit without pulling in the generation toolchain (uv/alloy/rust).
A glob matching nothing only warns.

When there are assets the release is created as a **draft**, the assets are uploaded, then it is published (repos with *immutable releases* reject uploads to an already-published release; the tag is created when the draft is published). Idempotency keys on the tag, so to re-publish after a failure you must delete the leftover tag/release first — and a run that died between creating the draft and publishing leaves an orphan draft (no tag) to clean up by hand.

## Chart publishing (OCI, GHCR)

Chart components are additionally published to GitHub Packages as **OCI artifacts**, after the GitHub Release.
The same `helm package` output is pushed with `helm push charts/<component>-<version>.tgz oci://ghcr.io/materializeinc/helm-charts`, landing at `ghcr.io/materializeinc/helm-charts/<component>` (the chart name becomes the repository, the chart version the OCI tag).
GitHub Packages speaks **only** the OCI distribution protocol — there is no classic HTTP (`index.yaml`) Helm repository — which is consistent with how this chart already sources its own subcharts over `oci://ghcr.io/...`.

Consumers install directly from the registry, no `helm repo add` needed:

```console
helm install my-monitoring oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring --version X.Y.Z
```

Login uses the workflow's default `GITHUB_TOKEN` (the `publish-release` workflow grants `packages: write`).
The push runs *after* the release so a registry hiccup never blocks it; re-running the job retries the push while the release step no-ops on the existing tag.
The push overwrites an existing version tag, so a retry is safe.
Newly created GHCR packages are **private** by default — set the package visibility to public (once) so external consumers can pull.

## Auto-format

`propose-bumps` builds branches via the GitHub API, so it cannot run formatters; the bump commit therefore leaves generated artifacts stale (e.g. the `helm-docs` chart README badge after a Chart.yaml version bump). Rather than install a toolchain in `propose-bumps`, the [`auto-format`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/workflows/auto-format.yaml) workflow runs the repo's formatters (`make helm-docs`, `cargo fmt`, `ruff`) on any PR labeled `auto-format` and pushes a single `style:` commit if anything changed. The same mechanism covers GitHub UI edits and renovate PRs — just apply the label.

**Token requirement:** a label/PR event raised by the default `GITHUB_TOKEN` does **not** trigger other workflows (GitHub's loop-prevention).
For `auto-format` to fire from `propose-bumps`, `propose-bumps` must authenticate with a **PAT or GitHub App token** (`MATERIALIZE_BOT_TOKEN`), not the default `GITHUB_TOKEN`.
The auto-format commit is likewise pushed with `MATERIALIZE_BOT_TOKEN` so it triggers the PR's required checks (lint/test) and lets auto-merge proceed.
That push re-triggers `auto-format` once, but the formatters are idempotent, so the second run finds nothing to commit and exits — the loop is bounded to a single no-op run.
If the token is unset the push falls back to the default `GITHUB_TOKEN`, restoring the old no-re-trigger behavior (and leaving required checks unrun on the style commit).

`propose-bumps` still syncs `uv.lock` inline for now; once auto-format reliably handles lockfiles that inline logic can be dropped (deferred — only generated docs were stale in practice).

## The committed-surface check {#the-committed-surface-check}

DEP-127's third deliverable: a PR that changes the customer-facing surface either follows the [deprecation policy](../versioning/#stability-guarantees) or says why not.
There is no CI gate for this — see [why](../design-docs/20260823-deprecation-policy/#enforcement-use-what-is-already-generated) — so it is a review step, deliberately short.

**When it applies.** The PR's diff touches one of these:

| Path | Carries |
|---|---|
| `packages/queries/*.yaml` | alert names, `severity` / `component` values, recording-rule names |
| `terraform/modules/*/variables.tf`, `outputs.tf` | module inputs and outputs |
| `charts/*/pre-rendered/dashboards/` | dashboard identities |
| `charts/*/pre-rendered/metrics/metric-tiers.yaml` | tier names |

These are the paths [CODEOWNERS](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/CODEOWNERS) covers, so the review request arrives on its own.
Adding an identifier needs nothing — the check is only about **renames and removals**, which show up as a delete-plus-add in one of the generated or committed files above.

**What to check.** For each removed or renamed identifier, one of:

- It was announced at least **30 days** ago. Find the `**Deprecated:**` bullet in `CHANGELOG.md` and check the date on that release's tag.
- It is being announced *now*, in which case the PR keeps the old name working and adds the `**Deprecated:**` bullet — removal is a later PR.
- It is exempt, and the PR body says why. The honest exemptions are that nothing ever consumed it, or that it is in the [pre-1.0 batch](../design-docs/20260823-deprecation-policy/#the-pre-10-breaking-change-budget). Say which.

**How a deprecation is recorded.** As a release-note bullet in the PR description, using a `**Deprecated:**` or `**Removed:**` prefix:

```markdown
### Release Notes

* **Deprecated:** `mz-mon-env-top` is now `mz-mon-environment-overview`.
  Both names resolve until 2026-09-23; update dashboard links and embeds.
```

The existing [release-notes harvesting](#release-notes-from-pr-descriptions) carries that into the component's `CHANGELOG.md` section verbatim, and the release's tag date is what the 30 days are counted from.
No separate section, no extra tooling — the prefix is the whole convention.

**Write it like the Terraform module's [upgrade notes](https://github.com/MaterializeInc/materialize-terraform-self-managed#upgrade-notes)**, which name what *did not* change alongside what did ("`grafana_url` keeps its name; its meaning becomes conditional"). Naming the non-breaks is what makes the breaks trustworthy.

**Chart values are the exception.** Their consumer is our own Terraform module, so a rename there is absorbed by the module bump in the same change. What it needs instead is a note to the known direct-`helm install` users; there is no cooldown to serve.

## Cascade and ordering

- Releasing a dependency updates its dependents' version-update PRs (cascade), recording an `Included <dep> @ vPREV..vNEW` entry.
- The wording is "Included" rather than "Updated" because a dependent may reference a dependency version that is queued but not yet released.
- When the tag must exist (e.g. for a release artifact that pins the dependency), release dependencies before their dependents.

## Open questions

- The default next version is a minor bump; a breaking change needs the placeholder version edited manually before release.
- Cascade can fan out: releasing a low-level shared component updates every dependent's version-update PR, so expect merge-order sensitivity across concurrent release PRs.
- `version_paths` now track the latest **released** version (bumped by the version-update PR), not the latest unreleased — reconcile the wording in [Versioning](../versioning/) when the tooling lands.
