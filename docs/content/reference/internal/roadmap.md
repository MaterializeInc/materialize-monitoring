---
title: "Roadmap"
weight: 60
---

<!-- This roadmap is public. Do not include customer-specific or sensitive information -->

# Roadmap

The goal of `materialize-monitoring` is **first-class, opt-in observability for self-managed Materialize** — logs, metrics, events, and alerts — for customers who want a one-stop-shop, without forcing our stack on customers who already run their own.
This page is the current source of truth for what is built, what is in flight, and what is planned next.
It supersedes the original Linear project (internal), which captured an earlier architecture that has since diverged (see below).

## How this maps to the original plan

The May 2026 plan assumed a different shape than what was actually built.
Where docs, tickets, or comments disagree with the repo, the repo wins and this page records why:

| Original plan (May 2026) | As built |
|---|---|
| Grafana dashboards via Jsonnet/Grafonnet (`sources/jsonnet/`) | Python + `grafana-foundation-sdk` + `py-mzmon-lib` (`packages/grafana-dashboards/`) |
| `crates/` Rust workspace + `sources/` input tree | `packages/` monorepo where Rust (`mzmon-lib`) and Python coexist |
| Datadog dashboards via a Datadog Rust SDK | Not pursued; OTLP-forward is the export path (see Pipelines) |
| Four fixed profiles, incl. a `datadog-agent` profile | Profile set deliberately left open; no `datadog-agent` profile |

## Cadence and milestones

Releases track a monthly cadence aligned to the **15th**.

| Milestone | Date/Coverage | Deliverables |
|---|---|---|
| Milestone 1 (M1) | **June 15** (baseline) | `env-top` overview dashboard (Summary, Kubernetes, Cluster, Connections, Compute, Storage — including Hydration / Freshness / Sources / Sinks *summaries*); cloud ↔ self-managed convergence via `$sqlMetricPrefix`; typed Alloy **agent** pipeline; **ScrapeConfigs + ServiceMonitors** for metric collection (synced to charts and docs); Hugo docsite; pre-commit suite; Grafana dashboard v1/v2 API support |
| Milestone 2 (M2) | **July 15 — required** | Native **OTLP exporter** support; **productionalized** stack (Thanos + Loki + Alloy); product observability documentation **fully replaced**; **Logs & Events** + **Networking** dashboards; Grafana 11 (dashboard v1) parity for publicly hosted dashboards (Grafana public dashboards gallery); Helm subchart bundling; `renovate` for dependency bumps |
| Milestone 3 (M3) | **July 15 — stretch** | Day 2 drilldown dashboards: **Hydration**, **Freshness**, **Sources**, **Sinks** (⛓️ gated on upstream Tier 2 instrumentation) |
| Milestone 4 (M4) | **August 15+** | Day 1 dashboards (Dependencies, Sizing); Day 2 ops dashboards (upgrades, resizing, changing sources/destinations, managing users); Tier 2 upstream metric instrumentation; Helm completeness tail; profile-set finalization; Terraform wrapper; v2 items (BYOC, trace correlation, Polar Signals, formal deprecation policy) |

Item tables below reference milestones by number (M1–M4); the dates live only in the table above.

## Status legend

- ✅ Done · 🔨 In progress · ⬜ Planned
- ⛓️ Blocked on an upstream metric-contract dependency (see [Metrics contract](#metrics-contract-upstream-dependency))

## Workstreams

### Dashboards

The `env-top` overview is shipped and carries the cloud ↔ self-managed convergence work.
**Grafana 11 (dashboard v1) parity is a hard requirement** for publicly hosted dashboards — the dashboard sources must continue to render against the v1 dashboard API, not only newer versions — so that the dashboards can be managed in the **Grafana public dashboards gallery**.

| Item | Milestone | Status |
|---|---|---|
| `env-top` overview (6 tabs, incl. Hydration/Freshness/Sources/Sinks summaries) | M1 | ✅ |
| Cloud ↔ self-managed convergence (`$sqlMetricPrefix`) | M1 | ✅ |
| Improved Grafana 11 (dashboard v1) support for the public dashboards gallery | M2 | 🔨 |
| Logs & Events (requires Loki + Alloy + logs) | M2 | ⬜ |
| Networking | M2 | ⬜ |
| Hydration Drilldown | M3 | ⛓️ |
| Freshness Drilldown | M3 | ⛓️ |
| Sources Drilldown | M3 | ⛓️ |
| Sinks Drilldown | M3 | ⛓️ |
| Dependencies (Day 1: are Materialize + o11y requirements satisfied?) | M4 | ⬜ |
| Sizing | M4 | ⬜ |

We weight **Day 2 operations over Day 1**: upgrades, resizing, changing sources, changing external destinations, and managing users are the operations that matter most for a running deployment.
Day 2 ops dashboards covering these land in the M4 window.

### Pipelines (Alloy)

Alloy carries both metrics and logs.
The agent pipeline is in place; the gateway pipeline and the OTLP export path are the near-term work.

| Item | Milestone | Status |
|---|---|---|
| Typed Alloy **agent** pipeline | M1 | ✅ |
| Native **OTLP exporter** (forwarding workflows evaluated for Honeycomb, Datadog, Google Cloud Observability) | M2 | ⬜ |
| Gateway pipeline (port the real processor from the Python reference) | M2 | 🔨 |
| Loki (logs) + Thanos (metrics) wiring | M2 | ⬜ |

### Scraping (ScrapeConfigs & ServiceMonitors)

Metric collection is configured through two surfaces: **ScrapeConfigs** (consumed manually, e.g. dropped into a Prometheus/Agent config) and **ServiceMonitors** (consumed by `prometheus-operator`, or by Alloy via `prometheus.operator.servicemonitor`).
Both are needed ASAP — they were targeted for the M1 baseline — and must be synced into the charts and the docs.

| Item | Milestone | Status |
|---|---|---|
| ScrapeConfigs (consumed manually) | M1 (ASAP) | 🔨 |
| ServiceMonitors (consumed by `prometheus-operator` or Alloy `prometheus.operator.servicemonitor`) | M1 (ASAP) | 🔨 |
| Sync ScrapeConfigs + ServiceMonitors into the charts and docs | M1 (ASAP) | 🔨 |
| Move ServiceMonitors to the `materialize-operator` Helm chart | M4 | ⬜ (long-term) |

Long term, ServiceMonitors belong in the `materialize-operator` Helm chart rather than here.
This repo carries them now to fill the gap, with the intent to hand them off once the operator owns that surface.

### Charts / Helm

**Helm is prioritized over Terraform.**
The umbrella chart loads pre-rendered artifacts and bundles the productionalized stack as subcharts.

| Item | Milestone | Status |
|---|---|---|
| Subchart bundling: Loki, Thanos, Alertmanager, Grafana, kube-state-metrics | M2 | ⬜ |
| Distroless Alloy image + pre-install/pre-upgrade `alloy fmt` validation hook | M2 | ⬜ |
| `helm-readme-sync` (values.yaml → generated README) | M2 | ⬜ |
| Terraform wrapper module (pins a chart version, own cadence) | M4 | ⬜ |

### Rules & alerts

| Item | Milestone | Status |
|---|---|---|
| Base alert set (severity profiles + runbook stubs) | M2 | ⬜ |
| Loki / Thanos rule sets (recording rules first-class) | M2 | ⬜ |

### Profiles

The profile set is **deliberately not finalized** — it is a stub until the common deployment shapes settle.
There is **no `datadog-agent` profile**; OTLP forwarding is the export path.
Profile finalization is an M4 activity.

### Testing / CI & DevEx

| Item | Milestone | Status |
|---|---|---|
| Pre-commit suite (ruff, pyright, shellcheck, yamllint, cargo fmt, helm-docs) | M1 | ✅ |
| `renovate` for automated dependency bumps | M2 | ⬜ |
| Synthetic-data end-to-end smoke test (metrics flow through the chart) | M4 | ⬜ |
| kind / ArgoCD / FluxCD CI matrix | M4 | ⬜ (very low priority) |

### Adoption / productionalization

The M2 target is a productionalized deployment for Cloud, an internal team, and initial external adopters.
(Specific adopter commitments are tracked out-of-band, not in this public roadmap.)

| Item | Milestone | Status |
|---|---|---|
| Productionalized for Cloud + internal + initial external adopters | M2 | 🔨 |
| Product observability documentation fully replaced (rewrite the recommended path; migration guide off the legacy SQL-exporter surface) | M2 | ⬜ |
| Internal monitoring migrated to consume this repo via `values.yaml` | M4 | ⬜ |
| Fork source repo and archive the original | M4 | ⬜ |

## Metrics contract (upstream dependency)

Several dashboards depend on metric instrumentation that lives **upstream in the `materialize` repo, not in this repository**.
The metric/label contract is the public API for everything here, so this dependency shapes the dashboard roadmap directly.

The environmentd-native public metrics endpoint delivered **Tier 1** (pre-aggregating clusterd counters into environmentd).
The carry-over is **Tier 2**: roughly 39 signal families that today exist *only* via the SQL-on-scrape sources slated for deletion (legacy `/metrics/mz_*` and the `v2_mz_*` exporter).
To retire those sources, environmentd must emit these natively.
High-leverage asks, in priority order:

- **`mz_object_info`** (id → fully-qualified name → type) — the single highest-leverage item.
  It gives every other metric a stable `group_left` join target for names.
- A family of **`_info` metrics** (`mz_cluster_info`, `mz_replica_info`, `mz_source_info`, `mz_sink_info`, …) carrying names and parent-id references.
- Native **source/sink status** metrics (no genuine source exists today).
- Native **hydration** and **frontier/freshness** signals.
- **Label-family harmonization** (short vs long vs very-long forms).

The **Hydration / Freshness / Sources / Sinks drilldowns** above are ⛓️ gated on this work, which is why they are M3 (stretch) rather than M2 (required).

## Release and changelog strategy

Mechanics live in [Releasing](releasing/); the strategy is:

- **Release unit:** the umbrella Helm chart, on the monthly **15th** cadence.
- **Versioning keyed to the contract.**
  Chart SemVer reflects changes to the customer-facing surface — labels, metric names, profile semantics, alert names, dashboard JSON structure — not internal churn.
  Subchart bumps flow through `Chart.lock` (renovate-assisted).
- **Deprecation policy.**
  Any breaking change to that surface gets **at least one minor-release deprecation cycle**.
  The release process check requires a PR touching the customer-facing surface to either follow the policy or carry an explicit justification.
  The label/metric contract is load-bearing for every customer dashboard built on top of it; the time to establish this discipline is before broad adoption, not after.
- **Changelog.**
  A `changelog.md` in keep-a-changelog style, with a called-out **"Customer-facing surface"** subsection per release so consumers can scan for contract-affecting changes at a glance.
- **Downstream pinning.**
  The Terraform wrapper pins a specific chart version with its own update cadence, so Terraform never tracks a moving chart target.
  Helm-first; the wrapper is an M4 item.

## Follow-up documentation

Tracked, not detailed here:

- Flesh out [Releasing](releasing/) with the release mechanics.
- Create `changelog.md`.
- Write the contract `versioning.md` (deprecation policy, in customer-facing terms).
- Refresh [Repo Layout](repo-layout/) as the layout evolves.
