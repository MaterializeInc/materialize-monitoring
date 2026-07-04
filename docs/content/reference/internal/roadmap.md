---
title: "Roadmap"
weight: 60
---

<!-- This roadmap is public. Do not include customer-specific or sensitive information -->

# Roadmap

The goal of `materialize-monitoring` is **first-class, opt-in observability for self-managed Materialize** — logs, metrics, events, and alerts — for customers who want a one-stop-shop, without forcing our stack on customers who already run their own.
This page is the current source of truth for what is built, what is in flight, and what is planned next.
This is generally synced with the [internal Linear Project](https://linear.app/materializeinc/project/first-class-observability-infrastructure-in-self-managed-5e48691c74a8/overview).

<!--

## How this maps to the original plan

The May 2026 plan assumed a different shape than what was actually built.
Where docs, tickets, or comments disagree with the repo, the repo wins and this page records why:

| Original plan (May 2026) | As built |
|---|---|
| Grafana dashboards via Jsonnet/Grafonnet (`sources/jsonnet/`) | Python + `grafana-foundation-sdk` + `py-mzmon-lib` (`packages/grafana-dashboards/`) |
| `crates/` Rust workspace + `sources/` input tree | `packages/` monorepo where Rust (`mzmon-lib`) and Python coexist |
| Datadog dashboards via a Datadog Rust SDK | Not pursued; OTLP-forward is the export path (see Pipelines) |
| Four fixed profiles, incl. a `datadog-agent` profile | Profile set deliberately left open; no `datadog-agent` profile |

-->

## Cadence and milestones

Releases track a monthly cadence aligned to the **15th**.

Milestones are named by maturity stage; the date is a soft target.

| Milestone | Target | Deliverables |
|---|---|---|
| **Foundation** (M1) | June 15 | `env-top` overview dashboard (Summary, Kubernetes, Cluster, Connections, Compute, Storage — including Hydration / Freshness / Sources / Sinks *summaries*); cloud ↔ self-managed convergence via `$sqlMetricPrefix`; typed Alloy **agent** pipeline; **ScrapeConfigs + ServiceMonitors** for metric collection (synced to charts and docs); Hugo docsite; pre-commit suite; per-component versioning/changelog/release automation; Grafana dashboard v1/v2 API support |
| **Production** (M2) | July 15 (required) | Native **OTLP exporter** support; **productionalized** stack (Thanos + Loki + Alloy); product observability documentation **fully replaced**; **Logs & Events** + **Networking** + **Upgrades** (Day 2) dashboards; Grafana 11 (dashboard v1) parity for publicly hosted dashboards (Grafana public dashboards gallery); Helm subchart bundling; `renovate` for dependency bumps |
| **Operational Depth** (M3) | July 31 (stretch) | Day 2 drilldown dashboards: **Hydration**, **Freshness**, **Sources**, **Sinks** (⛓️ gated on upstream Tier 2 instrumentation); Datadog dashboard set; Day 2 ops dashboards (resizing, changing sources/destinations, managing users) |
| **Maturity** (M4) | August 31+ | Day 1 dashboards (Dependencies, Sizing); Tier 2 upstream metric instrumentation; Helm completeness tail; profile-set finalization; Terraform wrapper; v2 items (BYOC, trace correlation, Polar Signals, formal deprecation policy) |

Item tables below reference milestones by their short tag (M1–M4); names and target dates live in the table above.

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
| GCP / GKE / GMP dashboard + datasource variations | M2 | ✅ |
| Improved Grafana 11 (dashboard v1) support for the public dashboards gallery | M2 | 🔨 |
| Logs & Events (requires Loki + Alloy + logs) | M2 | ⬜ |
| Upgrades (Day 2 ops) | M2 | ⛓️ |
| Networking | M3 | ⬜ |
| Hydration Drilldown | M3 | ⛓️ |
| Freshness Drilldown | M3 | ⛓️ |
| Sources Drilldown | M3 | ⛓️ |
| Sinks Drilldown | M3 | ⛓️ |
| Resizing (Day 2 ops) | M3 | ⬜ |
| Changing sources (Day 2 ops) | M3 | ⬜ |
| Changing external destinations (Day 2 ops) | M3 | ⬜ |
| Managing users (Day 2 ops) | M3 | ⬜ |
| Provide Google Cloud Monitoring dashboard set | M3 | ⬜ |
| Provide Datadog dashboard set | M3 | ⬜ |
| Provide Honeycomb dashboard set | M3 | ⬜ |
| Dependencies (Day 1: are Materialize + o11y requirements satisfied?) | M4 | ⬜ |
| Sizing (Day 1) | M4 | ⬜ |
| Replace dashboard management with rust implementation | M4 | ⬜ |

We weight **Day 2 operations over Day 1**: upgrades, resizing, changing sources, changing external destinations, and managing users are the operations that matter most for a running deployment.
Upgrades is pulled into M2; the rest are M3. Day 1 dashboards (Dependencies, Sizing) are M4.

Change operation dashboards focus on new objects being added or removed and
initially populated (rather than steady state metrics) with some error detection.

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

Metric collection is configured through two surfaces: **ScrapeConfigs** (consumed manually, e.g. dropped into a Prometheus/Agent config) and **ServiceMonitors / PodMonitors** (consumed by `prometheus-operator`, or by Alloy via `prometheus.operator.servicemonitor`; GCP uses `PodMonitoring`).
These ship as the released **Prometheus Scrapers** component and are bundled into the chart.

| Item | Milestone | Status |
|---|---|---|
| ScrapeConfigs (consumed manually) | M1 | ✅ |
| ServiceMonitors / PodMonitors (incl. GCP `PodMonitoring`) | M1 | ✅ |
| Sync scrapers into the charts and docs | M1 | ✅ |
| Move scrapers to the `materialize-operator` Helm chart | M4 | ⬜ (long-term) |

Long term, ServiceMonitors belong in the `materialize-operator` Helm chart rather than here.
This repo carries them now to fill the gap, with the intent to hand them off once the operator owns that surface.

### Charts / Helm

**Helm is prioritized over Terraform.**
The umbrella chart loads pre-rendered artifacts and bundles the productionalized stack as subcharts.

| Item | Milestone | Status |
|---|---|---|
| Subchart bundling: Loki, Thanos, Alertmanager, Grafana (+ operator), kube-state-metrics, metrics-server | M2 | ✅ |
| Generated chart README (values.yaml → README via `helm-docs`) | M2 | ✅ |
| Distroless Alloy image (FIPS boringcrypto, multi-arch, non-root, GHCR-published) | M2 | ✅ |
| Pre-install/pre-upgrade `alloy fmt` validation hook | M2 | ⬜ |
| Terraform wrapper module (pins a chart version, own cadence) | M4 | ⬜ |

### Rules & alerts

| Item | Milestone | Status |
|---|---|---|
| Base alert set (severity profiles + runbook stubs) | M2 | ⬜ |
| Loki / Thanos rule sets (recording rules first-class) | M2 | ⬜ |

### Profiles

The profile set is **deliberately not finalized** — it is a stub until the common deployment shapes settle.
Profile finalization is an M4 activity.

### Testing / CI & DevEx

| Item | Milestone | Status |
|---|---|---|
| Pre-commit suite (ruff, pyright, shellcheck, yamllint, cargo fmt, helm-docs) | M1 | ✅ |
| Per-component versioning + changelog + release automation (see [Versioning](versioning/) / [Releasing](releasing/)) | M2 | ✅ |
| `auto-format` workflow (label-driven formatter fixups) | M2 | ✅ |
| `renovate` for automated dependency bumps | M2 | ⬜ |
| Synthetic-data end-to-end smoke test (metrics flow through the chart) | M4 | ⬜ |
| kind / ArgoCD / FluxCD CI matrix | M4 | ⬜ (very low priority) |

### Adoption / productionalization

The M2 target is a productionalized deployment for Cloud, an internal team, and initial external adopters.
(Specific adopter commitments are tracked out-of-band, not in this public roadmap.)

| Item | Milestone | Status |
|---|---|---|
| Productionalized for Cloud + internal + initial external adopters | M2 | 🔨 |
| Product observability documentation fully replaced (rewrite the recommended path; migration guide off the legacy SQL-exporter surface) | M2 | 🔨 |
| Internal monitoring migrated to consume this repo via `values.yaml` | M4 | ⬜ |
| Fork source repo and archive the original | M1 | ✅ |

## Metrics contract (upstream dependency)

Several dashboards depend on metric instrumentation that lives **upstream in the `materialize` repo, not in this repository**.
The metric/label contract is the public API for everything here, so this dependency shapes the dashboard roadmap directly.

The environmentd-native public metrics endpoint delivered **Tier 1** (pre-aggregating clusterd counters into environmentd).
The carry-over is **Tier 2**: roughly 39 signal families that today exist *only* via the SQL-on-scrape sources slated for deletion (legacy `/metrics/mz_*` and the `v2_mz_*` exporter).
To retire those sources, environmentd must emit these natively.
High-leverage asks, in priority order:

- ✅ **`mz_object_info`** (id → fully-qualified name → type) — the single highest-leverage item; **delivered upstream**.
  It gives every other metric a stable `group_left` join target for names.
- ✅ A family of **`_info` metrics** (`mz_cluster_info`, `mz_replica_info`, `mz_source_info`, `mz_sink_info`, …) carrying names and parent-id references; **delivered upstream**.
- ⬜ Native **source/sink status** metrics (no genuine source exists today).
- ⬜ Native **hydration** and **frontier/freshness** signals.
- ⬜ **Label-family harmonization** (short vs long vs very-long forms).

The `_info` family is now available, so name enrichment is unblocked for every panel.
The remaining drilldowns are still ⛓️ gated on the items above: **Sources / Sinks** await native status metrics, and **Hydration / Freshness** await the hydration and frontier signals — which is why they stay M3 (stretch) rather than M2 (required).

## Versioning, changelog, and releases

**Built.** Each artifact has its own SemVer stream — the Helm chart, the optional CRDs chart, dashboards, pipelines, scrapers, and the shared lib — declared in `packages/components.yaml`.
Full mechanics are in [Versioning](versioning/) and [Releasing](releasing/); this replaces the earlier single-umbrella-chart framing.

- **Per-component streams.** ✅
  Merged PRs are attributed to components by path; `CHANGELOG.md` is the source of truth, with cumulative `Included <dep> @ vPREV..vNEW` dependency rollups.
- **Automation.** ✅
  `mz-monitoring-build propose-bumps` opens one `version-update/<component>` PR per changed component on each merge to main; `publish-release` tags `<component>/vX.Y.Z` and creates a GitHub Release (attaching each component's `artifacts`) when such a PR merges.
- **Deprecation policy.** ⬜
  Still to commit: at least one minor-release cycle for breaking changes to the label/metric contract, with a release-process check, and a called-out "customer-facing surface" changelog subsection.
  The label/metric contract is the public API; this discipline should land before broad adoption.
- **Downstream pinning.** ⬜
  The Terraform wrapper (M4) pins specific chart versions with its own cadence, so Terraform never tracks a moving target.

## Follow-up documentation

- [Releasing](releasing/) and [Versioning](versioning/) are written, covering the release mechanics and the per-component model. ✅
- `CHANGELOG.md` exists and is maintained by the release tooling. ✅
- A **customer-facing** contract/deprecation-policy page (in customer terms, distinct from the internal `versioning.md`) is still to write. ⬜
- [Repo Layout](repo-layout/) still needs a refresh as the layout settles. ⬜
