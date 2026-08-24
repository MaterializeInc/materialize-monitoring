---
title: "Stability and Deprecations"
weight: 35
---

# Stability and Deprecations

This page says what you can safely build on, and what happens when something has to change.

`materialize-monitoring` is **pre-1.0**. Versions are per-artifact (see [Compatibility](../compatibility/) for how they line up with Materialize and the Terraform modules), and until 1.0 a breaking change may ship in a minor release. What the version number does *not* change is the notice you get: the deprecation cycle below applies now.

## What we guarantee

| You can build on | Guarantee |
|---|---|
| **Alert names**, and their `severity` and `component` label values | Not renamed or removed without the cycle below |
| **Recording-rule metric names** | Not renamed or removed without the cycle below. The expression behind a recording rule may change; the name it publishes will not |
| **Terraform module inputs and outputs** | Not renamed or removed without the cycle below |
| **Dashboard identities** (a dashboard's `name`, e.g. `mz-mon-env-top`) | Stable, so links and embeds keep working. What is *inside* a dashboard — which panels, which layout — is not |
| **Metric-importance tier names** (`essential`, `recommended`, `extended`, `diagnostic`, `all`) | Stable |
| **Chart and image names**, OCI paths, and the `<component>/vX.Y.Z` tag format | Stable |
| The `monitoring.materialize.cloud/*` annotation namespace | Stable |

## The deprecation cycle

1. **Announced.** The release's entry in the [Changelog](../changelog/) carries a `**Deprecated:**` note that names the replacement.
2. **Both work for at least 30 days.** The old name keeps functioning, not merely existing.
3. **Removed** in a later release, with a `**Removed:**` note.

A **behavior change counts as a break.** If an alert keeps its name but starts firing under materially different conditions, that goes through the cycle too — your routing and runbooks are keyed on the name, so a silent change of meaning is the same problem as a rename. Tightening a threshold is not a break; changing what the alert *means* is.

Additions are not breaking changes and can arrive in any release.

## What we do not guarantee

**Materialize's own metric names and labels.** `mz_*` metrics come from Materialize itself, which has its [own weekly release cadence](https://materialize.com/docs/releases/). We do not control those names, so we cannot freeze them. What we do instead:

- Where our layer can publish both the old and new name at once, we do, for the same 30 days.
- Where it cannot, the change is called out in the [Changelog](../changelog/) and in [Compatibility](../compatibility/).
- Dashboards and alerts declare the Materialize version they need, so an artifact never silently depends on a metric your deployment does not have.

In practice a metric change is the mildest kind of break: a panel goes empty, which is visible and fixable on your own schedule. Nothing pages, and nothing is lost.

**Helm chart value paths.** Most installations reach the chart through the [Terraform module](../terraform/), which pins a chart version — so a value rename is absorbed by upgrading the module, and pinning the module defers it entirely. If you run `helm install` directly, tell us: the list of direct installers is short enough that we will contact you before renaming a value you use, which is a better guarantee than a policy.

**Subchart values** (`loki.*`, `thanos.*`, `grafana.*`, `alertmanager.*`) belong to those upstream charts and can change when we bump them.

**Dashboard internals and query definitions.** If you fork one of our dashboards, you own the fork. Panels, layout, and the PromQL inside them change freely.

## Where changes are announced

- **[Changelog](../changelog/)** — every release, per artifact. `**Deprecated:**` and `**Removed:**` notes appear here first.
- **[Compatibility](../compatibility/)** — version pairings with Materialize, the Terraform modules, Kubernetes, and Grafana, and where upstream changes we absorbed are recorded.
- **[Terraform module upgrade notes](https://github.com/MaterializeInc/materialize-terraform-self-managed#upgrade-notes)** — if you deploy through the modules, this is the page to read before bumping `ref`.

## If something breaks anyway

We are pre-1.0 with a small installed base, so if you hit a break that did not go through the cycle above, it is likelier to be our mistake than your misreading. [Open an issue](https://github.com/MaterializeInc/materialize-monitoring/issues) — that is a bug in this policy, and we would rather hear it than not.
