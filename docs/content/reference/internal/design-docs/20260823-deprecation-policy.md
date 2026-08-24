---
title: "Stability Guarantees and Deprecation Policy"
weight: 20260823
# draft=false makes it render as a page
# params.status=Accepted — the policy landed; see "Work in this repo" for what remains
draft: false
publishdate: 2026-08-23
lastmod: 2026-08-24
# custom parameters
params:
  author: Heather Lapointe
  status: "Accepted"
---

# Stability Guarantees and Deprecation Policy

{{< param-table >}}

This doc proposes the stability guarantees and deprecation policy owed by [DEP-127](https://linear.app/materializeinc/issue/DEP-127) (OO-M1), listed on the [Roadmap](../../roadmap/#versioning-changelog-and-releases) as the last unbuilt piece of the versioning story.
It is a prerequisite for [DEP-205](https://linear.app/materializeinc/issue/DEP-205) — stamping 1.0 is the moment the guarantees below stop being aspirational — so this should be agreed before that, not alongside it.

Two central claims. First, that **the guarantee has to be graded by how much control we actually have, and stated in a unit that means something across three release streams**. Second, that **it should be graded by how a break presents to the customer, which inverts the emphasis DEP-127 starts from**: a renamed metric leaves a visibly blank panel a customer can fix on their own schedule, while a renamed or removed alert leaves silence — and silence during an incident reads as health. Metrics are among the most forgiving things we publish; alerts are the least.
The query registry's schema already asserts that "changes to Canonical or Best-Effort queries are considered breaking changes", which covers 295 of 296 declarations with no mechanism behind it. Meanwhile the surface customers build dashboards on — `mz_*` metric names and their label families — is defined by the Materialize product, on a *weekly* release cadence, and reaches customers through a third repository moving faster still.
A policy expressed as "one minor release" is close to meaningless against any of those streams, and a policy that claims to freeze what the product defines would be a written promise we cannot keep.
So the proposal grades the surface by control — **owned**, **coordinated**, **passthrough** — and states the window in wall-clock days rather than release counts.

The timing is better than DEP-127 knew when it argued that the window to establish discipline is "before lots of customers query the surface, not after". Two reasons.

**The surface that most needs the strictest guarantee has not shipped at all.** The alerting path is unbuilt in self-managed: the 89 alerts in the registry are references carried over from Cloud, checked in but not plugged into anything, and `pre-rendered/rules/` holds nothing but `.gitkeep`. Alert names, severities, and recording-rule names are still free.

**And what has shipped has almost no adopters yet.** So the practical constraint is not "has it shipped" but "does anything depend on it", and today very little does. That is a real, temporary freedom — and it is [closing on a clock we do not control](#the-pre-10-breaking-change-budget).

<!--
Agent note: this doc records why the policy is shaped the way it is.
The policy of record is versioning.md's "Stability guarantees" section; keep the two consistent.
The counts are the part most likely to drift — re-derive them from packages/queries/ and the schema.
Re-check the "Shipped?" column against pre-rendered/rules/, which is empty as of this doc.
Several arguments here rest on the alerting path being unbuilt, and stop holding the day it ships.
The "almost no adopters" premise behind the breaking-change budget is the most perishable claim here.
Treat it as a statement about August 2026, not a standing fact, and do not let it justify deferring the policy.
Update "Work in this repo" as items land, rather than rewriting the proposal in place.
-->

## Goals

- Say exactly which identifiers a customer may build on, and what we promise about each.
- Commit a deprecation cycle with a floor that survives our release cadence.
- Make a breaking change to the committed surface **visible in the PR diff**, without building anything new to make it so.
- Reuse the machinery already in the repo — the stability ladder, the metric tiers, the per-component changelog — rather than inventing a parallel system.
- Be honest about the passthrough surface instead of over-promising on it.

## Technical BLUF

Four classes of surface, graded by how much control we have:

| Class | Who defines it | What we promise |
|---|---|---|
| **Committed** | This repo generates it | No rename or removal without a full deprecation cycle |
| **Evolving** | This repo generates it, still moving | One release of notice in the changelog; no overlap required |
| **Coordinated** | Materialize product, but we have a channel into it | We carry the break rather than pass it through: dual-publish where we can, and we take the `canonical` dependency list upstream so the metrics our shipped artifacts need get the same window |
| **Passthrough** | Upstream charts, Kubernetes, Grafana | No stability promise; a compatibility declaration and a changelog note when we learn it moved |

The deprecation cycle for the committed surface is **30 days**, then removal in a subsequent minor (pre-1.0) or major (post-1.0).
Deliberately a wall-clock number and not "one minor release" — see [Why the window is wall-clock](#why-the-window-is-wall-clock).

One cooldown, but **[ceremony graded by failure mode](#grade-the-ceremony-by-failure-mode)**: strictest for alerts (silent, safety-critical), lightest for metrics (loud, non-destructive), none for query IDs and chart values, whose only consumers are [our own dashboards and our own Terraform](#where-the-surface-exits).

Adoption is near zero today, so there is a **one-time [pre-1.0 breaking-change budget](#the-pre-10-breaking-change-budget)** to spend on the renames we already know we want — closing on customer upgrade cadence rather than on a decision of ours, since the Terraform default flipped to opt-out on 2026-08-20.

Enforcement builds **nothing new**: every committed identifier already lives in a generated, committed artifact, so a rename already shows up as a diff. Add CODEOWNERS on those paths and a `### Deprecations` PR section, and stop there — see [Enforcement](#enforcement-use-what-is-already-generated).

## Non-goals

- **Freezing `mz_*` metric names or label families.** Not ours to freeze unilaterally — but not disowned either. Covered under [Coordinated](#coordinated-surface-materialize-metrics-and-labels).
- **A support-lifetime or LTS policy.** How long a released version keeps getting fixes is a separate question from what a given version's surface guarantees; [Compatibility](../../../compatibility/) is where support windows live.
- **A policy for the `materialize-terraform-self-managed` stream.** Its upgrade notes are the model this doc borrows from, and [where our surface differs](#where-the-surface-exits) is explained below, but that repo sets its own policy.
- **Retroactive guarantees.** The policy applies from the release in which it lands. Nothing below claims that today's identifiers were already stable.
- **Changing the pre-1.0 bump policy.** Breaking changes continue to ride minors until DEP-205; this doc adds a deprecation cycle *within* that, and describes what changes at 1.0.

## What exists today

More machinery exists than the issue assumes, which is good news — most of the work is connecting and exposing it rather than building it.

**A stability ladder, schema-enforced but unexercised.**
Every query, alert, and recording rule in `packages/queries/` carries a required `stability` field, enumerated in [`mzmon-query.schema.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/packages/mzmon-lib/schemas/query/mzmon-query.schema.yaml):

```
unused → playground → experimental → best-effort → canonical → deprecated → unsupported
```

Across the 10 registry files there are 296 declarations: **248 `best-effort`, 47 `canonical`, 1 `experimental`, and zero `deprecated` or `unsupported`**.
The two terminal states have never been used, so the ladder has an announce step that has never been walked.

The schema also already carries policy claims in prose — that canonical queries require test coverage, that deprecated ones are "warned upon usage", that recording rules are laxer "because they allow the underlying query to change while the recording rule remains stable".
That last one is exactly right and is the model this proposal generalizes: **guarantee the name you publish, not the expression behind it.**

**Metric importance tiers, which are not stability.**
`essential / recommended / extended / diagnostic / all` drive the Helm gateway's per-destination allowlists via the generated `metric-tiers.yaml`.
They answer "is this worth storing", not "may I depend on this".
They are the *only* one of the two axes currently visible to customers, on [List of Metrics](../../../stable-metrics/list-metrics/). Stability is invisible there, which was the earlier draft's complaint — but on the [consumer-chain](#where-the-surface-exits) reading that is fine: `stability` governs internal expectations, and what a customer needs published is the policy, not a per-query badge.

**A prefix-parameterization capability — but no deprecation precedent.**
Queries template their metric prefix as `%%{mzSqlPrefix}`, the tier regexes match `(?:v2_)?mz_`, and dashboards stamp which prefix they were rendered for in a `monitoring.materialize.cloud/sql-metric-prefix` annotation.

It is worth being precise about what this is, because it reads like versioning and is not. **`v2_` was never a version.** It was the prefix on one part of the Cloud platform's SQL-based metric endpoints, and it does not exist in self-managed — so the two prefixes are two *deployments*, not two generations of the same metric. The code's "legacy" and "converged" wording is at best misleading and should be reworded.

What we actually have, then, is the *capability* to render one query definition against two metric namespaces and to record which one an artifact targets. That is genuinely the right shape for a deprecation shim — render both, stamp both — but it has never been used as one. The policy should not cite it as precedent.

**A compatibility-declaration precedent.**
Dashboard artifacts carry `monitoring.materialize.cloud/min-mz-version` and `rec-mz-version`.
An artifact can already state what it needs; the policy should require that of anything depending on a passthrough surface.

**A changelog that can carry this.**
Per-component streams, path-based attribution, and — as of this month — [author-written release notes](../../releasing/#release-notes-from-pr-descriptions) harvested from each PR's description into `CHANGELOG.md`.
A deprecation announcement has somewhere to go, and the version-update PR is where a reviewer would see it.

**What does not exist:** any use of the `deprecated` state, and any written statement of what we guarantee.

## Three release cadences, and why release counts cannot express the window

A customer running self-managed Materialize with this stack installed is tracking **three independent version streams**:

| Stream | Scheme | Observed cadence | What it carries |
|---|---|---|---|
| [Materialize](https://materialize.com/docs/releases/) | `v26.MINOR.PATCH` | **Weekly minors**, Cloud and self-managed alike since v26.1.0 | `mz_*` metrics, their labels, `mz_object_info` and friends |
| [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/materialize-terraform-self-managed/releases) | `vMAJOR.MINOR.PATCH` | **v5 → v11 between 2026-07-17 and 2026-08-21** — six majors in five weeks | How the stack is deployed; the customer's `ref=<tag>` |
| `materialize-monitoring` (this repo) | per-component `v0.MINOR.PATCH` | A minor **proposed on every merge to `main`** | Alerts, dashboards, chart values, the module |

DEP-127's "at least one minor-release deprecation cycle" was written in May 2026, before this repo had any versioning of its own, and it meant **a Materialize minor** — which is one week. Read against this repo it means "until the next merge to main". Read against the Terraform stream, a major is a matter of days.

There is no reading of "one release cycle" that gives a customer usable notice. The unit has to be wall-clock time, and it has to be a number a customer can plan against without tracking any of the three streams.

## Why the window is wall-clock {#why-the-window-is-wall-clock}

**30 days.** Long enough to cover a monthly upgrade cycle, which is the realistic cadence for a customer tracking a stream that ships majors weekly, and short enough that we are not carrying shims indefinitely — the cost of a long window is not paperwork, it is the dual-published code path someone has to keep working.

The release-count floor is dropped entirely rather than kept as a belt-and-braces "one minor *and* 30 days": on this repo's cadence the minor is always satisfied first, so it adds nothing but a second number to explain.

30 rather than a quarter is a deliberate bet that **most of what we publish fails loudly and non-destructively**, so a shorter window costs a customer a blank panel rather than an outage. Where that bet does not hold — alerts — the [risk grading](#grade-the-ceremony-by-failure-mode) adds ceremony rather than adding days.

## Where the surface actually exits to a customer {#where-the-surface-exits}

Most of what looks like a published surface is **internal coupling**, because we are our own biggest consumer:

```
queries ──────> dashboards we maintain ──┐
alerts (unbuilt) ────────────────────────┼──> chart ──> our Terraform module ──> customer roots
profiles, chart values ──────────────────┘                    │
                                                              └──> direct `helm install` (small, tracked)

metrics + labels ────────────────────────────> the customer's own Grafana
alert names, severities ─────────────────────> the customer's own Alertmanager
```

A surface is only customer-facing where the chain **exits to something we do not maintain**. There are three exits, and they are not equally wide:

| Exit | What crosses it | How wide |
|---|---|---|
| **Terraform module** | input variables, outputs | The main one. Consumed downstream by customer roots. |
| **The customer's own observability** | metric names, labels, alert names, severities, dashboard identities | Real, and outside our reach entirely. |
| **Direct `helm install`** | chart value paths, profile names | Narrow — a small enough list to contact individually. |

This resolves what the earlier draft got wrong by counting: **413 documented chart value paths are not 413 customer promises.** Almost all of them are consumed by our own Terraform module, which we bump in the same change. Likewise query IDs: their consumers are dashboards in this repo, so renaming one is a refactor, not a break. Neither belongs in the committed surface on the strength of appearing in a reference page.

**Chart values are the case where the Terraform stream's model actually fits.** [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/materialize-terraform-self-managed#upgrade-notes) runs no deprecation cycle — its v10.0.0 removed two modules outright, "**removed**, not deprecated in place […] pin the previous major until you have migrated" — and for a value path consumed by a module that pins a chart version, pinning genuinely is the whole escape hatch. Direct installers are the exception, and they are trackable, so the right treatment there is **notify the known list**, not run a cycle for a hypothetical stranger.

**What does not have an escape hatch is the observability exit.** A customer's dashboards, alert routes, and silences live in *their* Grafana and *their* Alertmanager. Pinning our chart does not help when the thing that changed is the shape of the data their panels query. That, and only that, is what justifies a cycle stricter than the Terraform stream's.

**What we should borrow wholesale is the notes format.** Those upgrade notes state, per version, what changed, what the impact on an existing deployment is, and — unusually and valuably — what *did not* change: "`grafana_url` keeps its name; its meaning becomes conditional", "`enable_observability` keeps its name and its defaults". Naming the non-breaks is what makes the breaks trustworthy. Our `**Deprecated:**` bullets should read like that.

## Grade the ceremony by failure mode {#grade-the-ceremony-by-failure-mode}

One cooldown number, but not one level of care. What matters is **how a break presents to the customer**, not how many identifiers are involved:

| Surface | Failure mode | Pages? | Treatment |
|---|---|---|---|
| **Alert names, severities** | **Silent.** A removed alert does not error — it stops firing. A re-severitied one routes somewhere else. | Yes, or worse: *stops* paging | Strictest. Cooldown, dual-publish, explicit notes, and a named owner on removal |
| Terraform variables, outputs | Loud — `terraform plan` fails | No | Cooldown + upgrade notes. Pinnable. |
| Dashboard identities | Loud — a bookmark or embed 404s | No | Cooldown + notes |
| Metric names, labels | Visible — a panel goes blank or empty | No | Changelog entry, dual-publish where our layer allows |
| Chart values, profiles | Loud — a values file stops applying | No | Notes, plus direct contact for known direct installers |
| Query IDs | Internal — consumers are in this repo | No | None. Refactor freely. |

**Breaking a metric is much less costly than breaking an alert**, and the policy should say so rather than treating "the label/metric contract is the public API" as the headline. A renamed metric gives a customer an obviously empty panel that they can see, diagnose, and fix on their own schedule. A renamed or removed alert gives them silence — and silence during an incident is indistinguishable from health. The metric break is loud and non-destructive; the alert break is quiet and safety-critical.

This inverts DEP-127's emphasis. The issue leads with labels and metrics as "the public API"; on a failure-mode reading, they are the *most* forgiving thing we publish, and the alerting surface — which has not shipped yet — is the one that warrants real ceremony.

## The surface, by control

This is the inventory the policy attaches to. Counts are current as of this doc and will drift — the generated artifacts listed under [Enforcement](#enforcement-use-what-is-already-generated) are the source of truth.

### Committed surface

Identifiers this repo generates, that a customer's dashboards, silences, alert routes, or Terraform would break on:

| Surface | Identifier | Count | Shipped? |
|---|---|---|---|
| Alerts | `alert:` name, plus `severity` and `component` label *values* | 89 | **No** — defined, not rendered |
| Recording rules | `record:` metric name | 0 | **No** — none defined |
| Terraform module | input variables and outputs | 45 + 12 | Yes |
| Dashboards | `metadata.name` (e.g. `mz-mon-env-top`) | 16 | Yes |
| Annotations | the `monitoring.materialize.cloud/*` key namespace | 6 keys | Yes |
| Metric tiers | the tier names themselves | 5 | Yes |
| Artifact names | chart names, OCI repository paths, per-component tag format | 9 components | Yes |

Two things are deliberately **not** here, on the [consumer-chain](#where-the-surface-exits) reading:

- **Chart value paths and profile names** (413 + 24). Consumed by our own Terraform module, which we bump in the same change. Committed only toward the small, tracked set of direct `helm install` users, and handled by contacting them rather than by a cycle.
- **Query IDs**, including the 47 `canonical` ones. Their consumers are dashboards in this repo. `canonical` remains meaningful — it marks which queries carry test coverage and which metrics are load-bearing for the [upstream ask](#coordinated-surface-materialize-metrics-and-labels) — but the ID itself is not a customer promise.

The **Shipped?** column is the important one, and it is why this policy is cheap to adopt today. The two surfaces with the strongest claim to a hard guarantee — alert names and recording-rule names, the [silent-failure](#grade-the-ceremony-by-failure-mode) ones — have shipped nothing. The alerting path is unbuilt in self-managed; the 89 alerts are Cloud-era references checked in against the day it is built, and `pre-rendered/rules/` contains only `.gitkeep`.

Read it as "free by construction" versus "free by circumstance", not as "guaranteed" versus "not". Nothing in the `Yes` rows is genuinely locked either, because [almost nothing depends on it yet](#the-pre-10-breaking-change-budget) — the difference is that the unshipped rows stay free until we choose to ship, and the shipped rows stop being free without anyone telling us.

Recording rules are the cleanest case, and for the same reason as alerts: nothing has shipped. They are also the strongest case for a hard guarantee on principle — they are metric names we mint ourselves, they land directly in customer PromQL, and the schema already says the expression behind them may change freely, which is exactly the "guarantee the name, not the expression" model this doc generalizes.

### Evolving surface

Ours, and not customer-facing enough to freeze:

- **Query IDs at every stability level**, including `canonical`. Internal coupling — see above.
- **Dashboard internals** — which panels exist, panel element keys, layout, row structure. A customer who forks a dashboard is on their own; the dashboard's *identity* is committed, its contents are not.
- **Chart value paths and profile names**, except toward known direct installers.
- Undocumented values, and anything that exists only to pass a value into a subchart.

This is where the earlier draft's crux — 248 `best-effort` queries the schema calls breaking to change — mostly dissolves. Once query IDs are understood as internal, the schema's claim is not a promise we are failing to keep; it is a claim about the wrong thing. What `stability` should govern is whether a query is *documented and depended on*, which is a real distinction worth keeping, just not a customer guarantee. See [Rollout](#rollout-before-10).

### Coordinated surface: Materialize metrics and labels {#coordinated-surface-materialize-metrics-and-labels}

`mz_*` metric names and their label families are defined by the Materialize product, not here. The [short-form vs long-form split](../../dashboard/style-guidelines/#materialize-metric-label-families) (`instance_id` vs `cluster_environmentd_materialize_cloud_cluster_id`) is an upstream artifact of two scraper paths, and the Roadmap's **label-family harmonization** item is a planned change to it.

But this is not a surface we are merely subject to — we have **influence over the Materialize metric lifecycle**, and that changes what we can promise from "nothing" to something specific and useful:

1. **We know exactly what we depend on, and can hand that list upstream.** `extract-metrics` already walks the registry and emits `metrics.yaml`; projecting it to *just the metrics reachable from `canonical` queries, shipped alerts, and shipped dashboards* produces the load-bearing set. That list — not "all `mz_*` metrics" — is what we ask the product to put a deprecation window on. It is a small, defensible ask precisely because it is scoped.
2. **We carry the break rather than pass it through, where our layer can.** Two mechanisms, neither yet used for this: the prefix parameterization (`%%{mzSqlPrefix}` plus the `sql-metric-prefix` annotation) can render one definition against two namespaces and stamp which an artifact targets — it exists for Cloud-vs-self-managed, not as a rename shim, but the shape is right. And recording rules are the general form: a stable name we mint over an expression free to change underneath. That [none exist yet](#committed-surface) makes this a design choice rather than a retrofit.
3. **Where we cannot shim, we declare.** The `min-mz-version` / `rec-mz-version` annotations already let an artifact state what it needs.

The promise, then: **a Materialize-side metric or label change affecting the load-bearing set does not reach a customer's dashboards as a silent break.** It arrives as a dual-published overlap, or as a compatibility declaration plus a changelog entry, and we advocate upstream for a window on that set matching the one we give on our own.

What this explicitly does *not* promise is that `mz_*` names never change, or that the harmonization will not be disruptive.

### Passthrough surface

- **Subchart value paths** under `thanos.*`, `loki.*`, `grafana.*`, `alertmanager.*` — upstream charts own those, and a subchart bump can rename them.
- **Kubernetes, Grafana, and Prometheus API shapes**, including the Grafana dashboard schema version.

Disclosure, not stability: a change we discover is a changelog entry and a [Compatibility](../../../compatibility/) update.

## The pre-1.0 breaking-change budget {#the-pre-10-breaking-change-budget}

Adoption of `materialize-monitoring` is close to zero today, which means every surface in the table above — shipped or not — can still be renamed for roughly the cost of the rename itself. That is a one-time budget, and it should be spent deliberately rather than discovered to have lapsed.

**It has a start date, and it is three days old.** The stack was opt-in until [`materialize-terraform-self-managed` v11.0.0](https://github.com/MaterializeInc/materialize-terraform-self-managed/releases) on 2026-08-20, which flipped `enable_observability` to default `true` on the `simple` examples as well as `enterprise`. From that release on, a customer who bumps `ref=<tag>` and never set the variable installs the whole stack — Loki, Thanos, Grafana, Alertmanager, Alloy. Adoption is near zero because it was opt-in until last week, not because nobody wants it, and the Terraform stream ships majors weekly.

So the budget closes on customer upgrade cadence, not on a decision of ours. There is no notification. The first time a rename is expensive, it will be expensive because someone already built a dashboard on the old name, and we will find out from them.

**What to spend it on**, in rough order of how much a late fix would cost:

| Change | Why now |
|---|---|
| Alert naming scheme (all 89) | The [silent-failure](#grade-the-ceremony-by-failure-mode) surface, and free until the alerting path ships; a cycle per alert after |
| Alert `severity` semantics | Same reason. Customers route and page on these |
| Recording-rule naming scheme | Nothing minted yet; the strongest surface to get right before the first one exists |
| Terraform variable renames we already know we want | The widest [customer exit](#where-the-surface-exits); loud when it breaks, but real downstream churn |
| Label-family harmonization | A coordinated-surface break either way; cheaper before anyone queries the long-form labels — and forgiving even then |
| Chart value renames | Cheapest of the set: our own Terraform is the consumer, plus a short list to notify |

**What the budget is not** is a standing argument. "Almost no adopters" will remain literally true for some time after it stops being a good reason, and it is exactly the kind of claim that quietly justifies deferring discipline forever. The proposal is that it expires at **1.0 or the first known production adopter, whichever comes first**, and that the policy below is adopted *now* regardless — the cleanup rides inside the policy's pre-1.0 mode, not instead of it.

## The policy

### Committed surface

A rename or removal is a three-step cycle. No step may be skipped, and the steps may not collapse into one release.

1. **Announce.** Set `stability: deprecated` where the surface has the field, and write a release-note bullet in the PR description starting `**Deprecated:**`, naming the replacement. The bullet lands in that release's changelog section, which is the dated record the cooldown is measured from.
2. **Overlap.** Old and new both work for **at least 30 days**. The old identifier keeps functioning — not merely existing — for the whole window.
3. **Remove.** In a later minor (pre-1.0) or major (post-1.0), with a `**Removed:**` bullet. `stability: unsupported` marks the tombstone so the identifier is never silently reused for something else.

**Additions are always free**, at any time, in any release. A policy that taxes additions gets routed around.

**A behavior change is a break.** Repointing an alert at a different expression such that it fires under materially different conditions is a break of that alert even though the name is unchanged — the customer's routing and runbooks are keyed on the name. Tightening a threshold is not; changing what the alert *means* is.

### Evolving surface

One release of notice in the changelog, no overlap required. Promotion to committed is the normal path: an `experimental` query that dashboards come to depend on should become `canonical`, and the promotion itself is not a breaking change.

### Coordinated surface

- A compatibility declaration on any artifact depending on a Materialize-side name.
- A dual-publish shim through our own layer wherever one is possible, held for the same 30 days.
- A changelog entry and a [Compatibility](../../../compatibility/) update either way — required even when no shim is possible, which is the usual case.
- The load-bearing dependency list published and taken upstream, with the ask that changes to it get a comparable window.

There is deliberately **no cooldown obligation on us** here, only on the shim. We do not control when a Materialize-side name changes, so promising a window we cannot enforce would be the same over-claim this doc opens on. What we promise is that the change is disclosed and, where our layer can absorb it, absorbed. The [failure mode](#grade-the-ceremony-by-failure-mode) is what makes that acceptable: a metric break is visible and non-paging.

### Passthrough surface

No stability promise. A changelog entry and a [Compatibility](../../../compatibility/) update when we learn it moved.

### What changes at 1.0

| | Pre-1.0 (today) | Post-1.0 (DEP-205) |
|---|---|---|
| Breaking change to committed surface | Allowed in a minor, after the full cycle | Major only |
| Deprecation cycle | Required (this policy) | Required, unchanged |
| Removal lands in | A later minor | A later major |

The one asymmetry is the [breaking-change budget](#the-pre-10-breaking-change-budget), which exists only pre-1.0 and only while adoption is negligible.

Otherwise the cycle is the same before and after; 1.0 only changes which bump the *removal* is allowed to ride. That is deliberate: it means adopting this policy now builds the habit, and 1.0 becomes a version-numbering decision rather than a process change.

## Enforcement: use what is already generated

An earlier draft of this doc proposed a `gen-surface` command emitting a checked-in `packages/surface.yaml`, diffed in CI, with a `since` field and a `deprecation-exempt` label. **That is the wrong amount of machinery for this repo, and it is dropped.** Recording why, because the reasoning is the useful part:

**The manifest already exists, three times over.** Every committed identifier is already in a generated, committed artifact that pre-commit keeps current:

| Committed surface | Already visible in | Kept current by |
|---|---|---|
| Terraform variables, outputs | `reference/terraform/materialize-monitoring-variables.md` | `terraform-docs`, pre-commit |
| Dashboard identities | `charts/*/pre-rendered/dashboards/grafana/*.yaml` | committed render |
| Metric tier names | `charts/*/pre-rendered/metrics/metric-tiers.yaml` | `gen-metric-tiers`, committed |
| Alert names, severities | `packages/queries/*.yaml` | hand-authored source |
| Annotation keys, artifact names | source | — |

A renamed Terraform variable already appears as a delete-plus-add in a checked-in reference page. A renamed dashboard already appears as a diff in `pre-rendered/`. Building a fourth generated file to restate this would be duplicating a signal we already have, and then owning the duplicate forever.

**And the justification for a gate was weaker than it looked.** The earlier draft asserted "reviewer diligence will not hold this". [CODEOWNERS](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.github/CODEOWNERS) is one person, with a `TODO` saying it should be a team. At that scale the reviewer and the author are the same person, so a CI gate is not catching a disagreement — it is reminding you of a policy you wrote. The failure mode is **forgetting**, not dissent, and forgetting is addressed by a checklist line at a fraction of the cost.

So the proposal, in ascending order of cost, stopping as early as it can:

1. **Nothing to build.** The diffs above are the signal. The committed surface is roughly 180 identifiers, of which the two largest groups have shipped nothing.
2. **CODEOWNERS on the paths that carry committed identifiers** — `packages/queries/`, `terraform/modules/*/variables.tf` and `outputs.tf`, `charts/*/pre-rendered/`. One file, no maintenance, GitHub enforces it, and it makes the existing `TODO` a little less of one.
3. **A prefix convention on release-note bullets** — `**Deprecated:**` and `**Removed:**` — which the [release-notes harvesting](../../releasing/#release-notes-from-pr-descriptions) already carries into `CHANGELOG.md` verbatim. No new PR section, no second harvester, no grouping rule: it works today with what shipped last week. The changelog is then also where the 30-day clock is read from, since release sections are dated by their tags, so there is no separate `since` bookkeeping either.
4. **A `code-review` skill** in `.claude/skills/`, which both Copilot code review and Claude read — so the check applies whoever is reviewing, including people running an agent against our changes rather than the other way round. A prompt rather than a gate, so a false positive costs a comment and not a blocked merge. Its most important content is the *what is not a breakage* list — a review that cries wolf gets ignored.
5. **Deferred: a grep-based CI reminder.** If a mechanical net turns out to be wanted, it is a short script that flags a disappearing `- alert:`, `record:`, or `^variable "` line and comments on the PR. No generated file, no exemption label, no state. Worth building the day it would have caught something, and not before.

**When to escalate:** when there is more than one reviewer, or when the direct-installer list stops being a list you can contact. Both are observable, and neither is true today.

## Rollout before 1.0

Ordered, because some of it gates the rest:

1. **Settle the unshipped identifiers now, while they are free.** Alert names, alert severities, and the recording-rule naming scheme are all pre-guarantee today. This is the single cheapest item on the list and the one with a closing window — it gets expensive the day the alerting path ships. See the [naming question](#open-questions) below.
2. **Triage the `canonical` set — for the upstream ask, not for a customer promise.** Promote the queries that shipped dashboards and alerts genuinely depend on, so the [load-bearing metric list](#coordinated-surface-materialize-metrics-and-labels) we take to the product side is defensible. This is much less pressing than the earlier draft assumed, now that query IDs are not a committed surface.
3. **Correct the schema's over-claim.** "Changes to Canonical or Best-Effort queries are considered breaking changes" should say what it actually governs — documentation and test-coverage expectations, and whether the query's metrics are load-bearing — rather than implying a customer-facing guarantee on query IDs.
4. **State the classes on the customer page** — which surfaces are committed, what the cooldown is, where breaks are announced. This is DEP-127's "mirrored on the docsite" deliverable, and it is prose, not a shortcode change: per-query `stability` stays internal signal now that query IDs are not a customer promise.
5. **Add the CODEOWNERS entries and the `### Deprecations` PR section.** Both are single-file changes.
6. **Publish the load-bearing dependency list and take it upstream.** Project `metrics.yaml` to the metrics reachable from `canonical` queries and shipped artifacts, and agree a window on *that set* with the Materialize side. This is the step that turns the coordinated surface from a caveat into a guarantee, and it is gated on the triage — the list is only defensible once the `canonical` set is honest.
7. **Spend the [breaking-change budget](#the-pre-10-breaking-change-budget)** on the table in that section, deliberately and in one batch rather than as they occur to us. Label-family harmonization is the big one; it is a coordinated-surface break either way, and much cheaper before anyone queries the long-form labels.
8. **Write the two customer-facing documents** (below).

## Work in this repo

| Item | Where | Status |
|---|---|---|
| This proposal | `design-docs/20260823-deprecation-policy.md` | ✅ Accepted |
| Policy of record | [Versioning](../../versioning/#stability-guarantees) | ✅ |
| Customer-facing page | [Stability and Deprecations](../../../stability/) | ✅ |
| Release-process check | [Releasing](../../releasing/#the-committed-surface-check) | ✅ |
| `**Deprecated:**` / `**Removed:**` bullet convention | `.github/pull_request_template.md` | ✅ |
| CODEOWNERS on the committed-surface paths | `.github/CODEOWNERS` | ✅ |
| `code-review` skill, read by Copilot code review and by Claude | `.claude/skills/code-review/SKILL.md` | ✅ |
| Schema wording correction | `mzmon-query.schema.yaml` | ✅ |
| Reword `v2_mz_` as a Cloud prefix, not "legacy" | `query/render.rs`, `gen_metric_tiers.rs` | ✅ |
| **Alert / recording-rule naming decision, before the alerting path ships** | `packages/queries/` | ⬜ The time-sensitive one |
| Load-bearing metric list (`canonical` closure of `metrics.yaml`) | `mz-monitoring-build` | ⬜ |
| Upstream agreement on a window for that list | Materialize product side, not this repo | ⬜ |
| `best-effort` → `canonical` triage | `packages/queries/` | ⬜ |
| Spend the [pre-1.0 budget](#the-pre-10-breaking-change-budget) | various | ⬜ |
| Direct-installer list — owner and location | — | ⬜ |

The policy itself has landed; what remains is the cleanup it enables and the one decision with a closing window.

**A note on the issue's paths.** DEP-127 names `docs/contrib/contract/versioning.md` and `docs/contrib/release.md`, and describes upgrading the former "from *stability guarantees TBD*". Neither path exists any more — the docs moved to `docs/content/reference/internal/` — and the current [Versioning](../../versioning/) page has no stability section at all, TBD or otherwise. The table above is the same three deliverables against the tree as it stands.

## Documentation to update

- [Versioning](../../versioning/) — the policy of record, and a pointer from "How versions are synced".
- [Releasing](../../releasing/) — the release-process check, next to the release-notes section it reuses.
- [Compatibility](../../../compatibility/) — say that it is the disclosure venue for the coordinated and passthrough surfaces. It already maps `materialize-terraform-self-managed` and Materialize versions to ours, so it is the natural place for the three-stream picture.
- [List of Metrics](../../../stable-metrics/list-metrics/) — stability alongside importance, and a note that the two axes are different questions.
- [Roadmap](../../roadmap/#versioning-changelog-and-releases) — link this doc from the DEP-127 bullet.

## Open questions

- **Is 30 days right?** The honest test is whether a customer's actual upgrade interval fits inside it. Nobody tracks a stream shipping majors weekly, so real intervals are plausibly longer than a month — which would argue for more. The counter-argument, and the reason 30 is proposed, is that a missed window on most of our surface costs a blank panel rather than an outage. That argument does not extend to alerts.
- **Does the coordinated promise survive contact with the product's cadence?** Weekly Materialize releases against a 30-day overlap still means a shim spans four or five upstream releases. Whether we can carry that, and how many simultaneously, is the practical limit on the guarantee.
- **Should alert names be kebab-case?** (The most time-sensitive question here.) All 89 are (`crdb-disk-usage-critical`), while the Prometheus ecosystem norm — and every upstream mixin a customer will have seen — is PascalCase (`KubePodCrashLooping`). Customers write Alertmanager routes and silences by pattern-matching `alertname`, so this is a surface decision, not a style one. It costs nothing today and a deprecation cycle per alert once the alerting path ships.
- **Are alert `severity` values committed?** Treating them as committed constrains us — a `warning` we later decide is `critical` becomes a breaking change. The argument for is that customers route on severity, so a silent change misroutes pages. Leaning committed, but it is the entry in the table most likely to be wrong.
- **Who signs off on a Tier-A removal?** CODEOWNERS is the cheap answer, but the point of a policy is that removal is a deliberate decision with a named owner, not a review.
- **Does this bind the Datadog surface too?** Datadog dashboards and queries are generated from the same registry, so the query-level guarantees carry, but Datadog-side identities have not been inventoried.
- **Should label-family harmonization block 1.0?** Doing it after 1.0 means a major bump for a coordinated-surface change. That argues for making it a 1.0 prerequisite, which is a scope claim on DEP-205 this doc should not make unilaterally.
- **What actually closes the breaking-change budget?** "1.0 or the first known production adopter" is proposed, but we do not currently *know* our adopters — nothing reports installs, and the Terraform default flipped only days ago. If we cannot observe adoption, the honest options are to pick a date or to treat 1.0 as the sole gate and get there quickly.
- **How do we maintain the direct-installer list?** The policy leans on it being small enough to contact individually, which is true now and is the kind of thing that stops being true without anyone noticing. It also has no owner and no location today.
