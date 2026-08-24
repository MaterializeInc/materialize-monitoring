---
name: code-review
description: |
  This skill should be used when reviewing a pull request or diff.
  Catches renames and removals on the customer-facing surface that owe a deprecation cycle, and says what is not a breakage.
---

# Code Review

This skill is the review-time layer, not a replacement for the domain skills.
For depth on a change, defer to the one that owns it:

- [`chart-development`](../chart-development/SKILL.md) — templates, `values.yaml`, subcharts, helm-unittest
- [`platform-development`](../platform-development/SKILL.md) — Terraform modules, E2E tiers, and chart changes with consequences for either
- [`dashboards-as-code`](../dashboards-as-code/SKILL.md) — Grafana dashboards
- [`pipelines-as-code`](../pipelines-as-code/SKILL.md) — Alloy pipelines
- [`yaml-development`](../yaml-development/SKILL.md) — YAML/KYAML conventions

What this skill adds is what no domain skill sees: whether the diff breaks something a customer already depends on.
It is one lens over a review, not the whole of one.

## The committed customer-facing surface

Renaming or removing one of these owes a deprecation cycle — announce, keep both working **30 days**, then remove.
The policy of record is the **Stability guarantees** section of `docs/content/reference/internal/versioning.md`.
The reviewer's checklist is **The committed-surface check** in `docs/content/reference/internal/releasing.md`.

| Where | Pattern | Why it matters |
|---|---|---|
| `packages/queries/**` | `- alert: <name>` | customers route, silence, and page on it |
| `packages/queries/**` | `severity:` / `component:` on an existing alert | label values customers route on |
| `packages/queries/**` | `record: <name>` | a metric name that lands in customer PromQL |
| `terraform/modules/*/variables.tf`, `outputs.tf` | `variable "<name>"`, `output "<name>"` | the widest customer exit — consumed by downstream roots |
| `charts/*/pre-rendered/dashboards/**` | `name:` under `metadata:` | dashboard identity, used in links and embeds |
| `charts/*/pre-rendered/metrics/metric-tiers.yaml` | a tier key | tier names are referenced in values |

**A rename shows up as a delete plus an add in the same diff.**
That is the case to catch.
It reads as an edit rather than a removal, which is exactly why it slips through.

**A behavior change is also a break.**
An alert that keeps its name but whose expression, `for:` duration, or thresholds change such that it fires under materially different conditions has broken the customer's routing and runbooks just as surely as a rename.
Tightening a threshold is fine; changing what the alert *means* is not.

## What is not a breakage

Everything below is in scope for review on its own merits — correctness, naming, tests, docs.
This list says only that it **owes no deprecation cycle**, so do not raise the policy against it.
False positives on the policy cost more than misses, because it is new and largely self-enforced: a review that cries wolf gets ignored.

- **Additions.**
  New alerts, variables, outputs, dashboards, or tier entries are free in any release.
  The cycle is only about renames and removals.
- **Query `id:` changes.**
  Query IDs are not customer-facing; the only consumers are dashboards in this repo.
- **A recording rule's expression changing** while its `record:` name stays.
  Explicitly allowed — we guarantee the published name, not the expression.
- **Chart value paths** (`values.yaml`, `profiles/`).
  Their consumer is our own Terraform module, which pins a chart version, so a rename is absorbed by the module bump in the same change.
  Direct `helm install` users are handled by contacting a short known list, not by a cycle.
- **Dashboard internals** — panels, layout, element keys, the PromQL inside a panel.
  Only `metadata.name` is committed.
- **`mz_*` metric names** appearing inside queries.
  Materialize defines those; we disclose changes rather than freeze them.
- **Reordering, reformatting, comment-only edits.**
  No identifier changed.

## Never ask for a CHANGELOG.md edit

`CHANGELOG.md` is **generated**.
`propose-bumps` rewrites it on every merge to the default branch, so a hand-written entry is overwritten and lost.
The single exception is the unreleased placeholder heading, edited to choose the next version rather than to add content.

A deprecation is announced by writing a bullet in the **pull request description**, under `### Release Notes`:

```markdown
### Release Notes

* **Deprecated:** `mz-mon-env-top` is now `mz-mon-environment-overview`.
  Both names resolve for 30 days; update dashboard links and embeds.
```

The release-notes harvesting copies that into the component's changelog section by itself.
So ask for a bullet in the PR description, and treat `CHANGELOG.md` as read-only evidence of what already shipped.

## What to ask for

When something does need flagging, ask for **one** of:

1. A `**Deprecated:**` bullet in the PR description naming the replacement, with the old identifier still working.
   Removal is a separate PR at least 30 days later.
2. A `**Removed:**` bullet, where a previously released changelog section already shows the `**Deprecated:**` announcement 30+ days ago.
3. A stated exemption in the PR body.
   The honest ones are "nothing ever consumed this" and "part of the pre-1.0 rename batch" — ask which, rather than accepting "it's fine".

Suggest wording that names what did **not** change alongside what did, the way the [Terraform module's upgrade notes](https://github.com/MaterializeInc/materialize-terraform-self-managed#upgrade-notes) do.
Naming the non-breaks is what makes the breaks trustworthy.

## Suggest the alternative, not just the cycle

A flagged break is usually avoidable, so lead with the design that dodges it.
The reasoning behind each is in **Designing to avoid the cycle** in `docs/content/reference/internal/versioning.md`.

- **Reading a raw `mz_*` metric?**
  Suggest minting a recording rule and pointing the artifact at that instead.
  We guarantee the `record:` name and may change the expression under it freely, so a future upstream rename becomes an expression edit with no customer-visible change.
  Zero recording rules exist yet, so there is no precedent to match — this is the moment to set one.
- **Renaming for clarity?**
  Suggest adding the new name and leaving the old one working.
  Additions are free; the cycle only starts when the old identifier has to *go*, which is often not actually required.
- **Adding an output, variable, or alert nobody consumes yet?**
  Ask whether it needs to be published at all.
  Every committed identifier is a permanent obligation, and the cheapest non-breaking change is one to a surface that was never published.
- **Hard-coding something deployment-specific?**
  Suggest templating it, the way `%%{mzSqlPrefix}` renders one query against either metric namespace.
- **Changing a default?**
  Suggest a new value that defaults to current behavior, with the default flipped in a later major.
- **An alert name ending `-critical` / `-high` / `-elevated`?**
  Severity in the identifier means re-grading forces a rename.
  Suggest naming for the condition and carrying the grade in the `severity` label.
  This shrinks the exposure rather than removing it — severity values are committed too — but a label change misroutes an alert that still fires under a findable name, where a rename breaks both.
- **A rename that genuinely must happen?**
  Ask that the old identifier keep *working*, not merely be documented.
  For a Terraform variable that means retaining it and letting the new one take precedence.

## Context that prevents wrong calls

- **The alerting path is not yet rendered** in self-managed — `charts/*/pre-rendered/rules/` is empty.
  The 89 alerts in `packages/queries/` are Cloud-era references, so renaming one today breaks nobody.
  Still ask for the note: the naming decision wants to be deliberate before the path ships, because it stops being free that day.
- **Alert names are kebab-case** (`crdb-disk-usage-critical`) against a PascalCase ecosystem norm.
  A PR proposing the switch is a *wanted* change, not a violation.
  It just needs to be all of them at once rather than one at a time.
- **`v2_mz_` is not a version.**
  It is the prefix on part of the Cloud platform's SQL metric endpoints and does not exist on self-managed.
  Do not read `(?:v2_)?mz_` as legacy-versus-current, and do not suggest migrating off it.
- **This repo is pre-1.0 and adoption is low**, so a breaking change may legitimately ride a minor.
  That is not a finding on its own — the missing cycle is.

## Severity

Grade by how a break presents, not by how many identifiers it touches.

- **Alerts fail silently.**
  A removed alert does not error, it stops firing, and silence during an incident is indistinguishable from health.
  Highest.
- **Terraform variables, dashboard identities, and chart values fail loudly.**
  A plan fails, a bookmark 404s, a values file stops applying.
  Medium.
- **Metric changes are visible and non-destructive.**
  A panel goes blank, which the customer can see and fix on their own schedule.
  Lowest — do not escalate a metric rename to the level of an alert rename.
