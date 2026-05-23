---
name: dashboards-as-code
description: |
  Use this skill when building, modifying, reviewing, or pushing Grafana dashboards under `packages/grafana-dashboards/` (Materialize observability dashboards generated from Python via `grafana-foundation-sdk` and `py-mzmon-lib`). Also use it when writing panel descriptions for those dashboards, picking palettes, or working through Materialize-specific PromQL patterns (cluster/replica filtering, peek latency, source/sink metrics, label-family quirks).
---

# Dashboards as Code

This skill is the entry point for the Materialize dashboards-as-code project. **Stable conventions live in the repo docsite** under [`docs/content/reference/internal/dashboard/`](../../../docs/content/reference/internal/dashboard/) — this file is intentionally slim and links into the docsite at heading-level granularity. The non-link content below is the **state snapshot**: what currently exists, what's in flight, and what's queued for cleanup.

## Audience reminder

The **dashboards themselves** target Materialize end users: database-literate operators with basic graph-reading fluency but minimal cloud / Kubernetes / observability expertise. SQL is fair game; jargon like "differential dataflow's arrangement" needs a one-liner explanation. Panel descriptions, titles, and cluster names should respect that baseline.

The **docsite reference pages** target repo contributors (SRE, Field Engineering, CloudOps, Database Engineers) and AI agents reading this skill.

## Where to find what

| Looking for… | Read |
|---|---|
| Grafana target versions, Dashboard v1/v2 schema state, SDK choices | [SDKs and Schemas](../../../docs/content/reference/internal/dashboard/sdks.md) |
| Code structure, UID conventions, push process, `gcx dashboards update` vs ad-hoc v2 API | [Generating and Pushing Dashboards](../../../docs/content/reference/internal/dashboard/generating.md) |
| Palettes, layouts, panel visualization, panel description voice, PromQL conventions, label families, metric quirks, PromQL recipes, module-level constants table | [Style Guidelines](../../../docs/content/reference/internal/dashboard/style-guidelines.md) |
| Testing conventions (currently sparse) | [Testing](../../../docs/content/reference/internal/dashboard/testing.md) |

Frequently needed deep links into the Style Guidelines:

- [Tab-level theming](../../../docs/content/reference/internal/dashboard/style-guidelines.md#tab-level-theming)
- [Multi-select variables in regex contexts](../../../docs/content/reference/internal/dashboard/style-guidelines.md#multi-select-variables-in-regex-contexts)
- [Sparkline stats](../../../docs/content/reference/internal/dashboard/style-guidelines.md#sparkline-stats)
- [Partitioned sparkline stats](../../../docs/content/reference/internal/dashboard/style-guidelines.md#partitioned-sparkline-stats)
- [Writing panel descriptions](../../../docs/content/reference/internal/dashboard/style-guidelines.md#writing-panel-descriptions)
- [Filtering by cluster / replica](../../../docs/content/reference/internal/dashboard/style-guidelines.md#filtering-by-cluster--replica)
- [Materialize metric label families](../../../docs/content/reference/internal/dashboard/style-guidelines.md#materialize-metric-label-families)
- [Known metric quirks and gotchas](../../../docs/content/reference/internal/dashboard/style-guidelines.md#known-metric-quirks-and-gotchas)
- [PromQL recipes](../../../docs/content/reference/internal/dashboard/style-guidelines.md#promql-recipes)
- [Shared module-level constants and helpers](../../../docs/content/reference/internal/dashboard/style-guidelines.md#shared-module-level-constants-and-helpers)

And into Generating:

- [PUT body shape](../../../docs/content/reference/internal/dashboard/generating.md#put-body-shape) — required Kubernetes-style envelope when pushing v2 dashboards via `grafana_api_request`
- [Service account permissions](../../../docs/content/reference/internal/dashboard/generating.md#service-account-permissions) — decoding 403s

## Schema reference files

When uncertain about the exact shape Grafana expects, the cog-generated openapi schemas are bundled here:

- `references/dashboard.openapi.json` — v1
- `references/dashboardv2beta1.openapi.json` — v2beta1
- `references/dashboardv2.openapi.json` — v2

All three generated from cog `61ff0a6055fa48f0c7b105fe4a37af637191314f` (April 9, 2026).

---

# Current Dashboard State

This section captures the live state of the dashboards in this repo so the next session has something concrete to start from. **Update it when state changes meaningfully** (new dashboard, new tab, retired panel, theme reassignment).

## Dashboard inventory

| Family | Dashboard module | Class | Live UID |
|---|---|---|---|
| `mz_environment` | `overview.overview_dashboard` | `EnvironmentOverviewDashboard` | (auto-assigned at first upload; codified UID is `mz-mon-env-top`, but the live one diverged before that became authoritative — see [UID selection and behavior](../../../docs/content/reference/internal/dashboard/generating.md#uid-selection-and-behavior)) |

The `mz_environment/overview` dashboard has six tabs, in declared order:

| # | Tab title | Module | Theme |
|---|---|---|---|
| 1 | Summary | `summary.py` | (no unique theme; uses health palette and themes from imports) |
| 2 | Kubernetes Workloads | `k8s_resources.py` | `K8S_THEME` = `palette.THEME_PALETTE[0]` (blue) |
| 3 | Cluster Objects / Replicas | `cluster_objects.py` | `CLUSTERS_THEME` = `palette.THEME_PALETTE[2]` (teal) |
| 4 | Connections / Activity | `connections_activity.py` | `CONNECTIONS_THEME` = `palette.THEME_PALETTE[1]` (cyan) |
| 5 | Compute Objects | `compute_objects.py` | `COMPUTE_THEME` = `palette.THEME_PALETTE[3]` (orange) |
| 6 | Storage Objects | `storage_objects.py` | `STORAGE_THEME` = `palette.THEME_PALETTE[4]` (yellow) |

The `Summary` tab re-uses the `KubeResourcesMixin`'s `cpu_total_panel` and `memory_totals_panel`, and also mirrors `add_currently_hydrating_panel(...)` from `compute_objects.py` in its Environment Health row.

## Tab-by-tab row structure

**Summary**

1. Environment Health — Environment Status, Availability, Last Restart, Currently Hydrating (mirror), Current CPU Usage, Current Memory Usage
2. Environment Info — Materialize Version, Total CPU Capacity, Total Memory

**Kubernetes Workloads**

1. Resources Summary — Total CPU Capacity, Total Memory (includes monitoring)
2. Workload Readiness — Pod Readiness, StatefulSet Readiness, Deployment Readiness
3. Pod Metrics — Pod CPU Usage, Pod Memory Usage
4. Pod Networking — Rx, Tx, Errors, Packet Drops

**Cluster Objects / Replicas**

1. Cluster Summary — Cluster Count, Replica Count
2. Replication / Availability — Replica Sizes (donut), Replica AZs
3. Cluster Information — Cluster Information table

**Connections / Activity**

1. Connection Summary — Active Sessions, Active Queries, Adapter Command Rate
2. Queries — Distribution donut, Query Rate, Peek Latency p50/p90/p99 (3 separate panels)
3. Adapter Commands — Adapter Commands by Application table

**Compute Objects**

1. Compute Objects Summary — Active MV, Active Indexes, Active Views, Active Subscribes (donut), Index Types (donut)
2. **Freshness** — **STUB row, no panels yet** (placeholder title only)
3. Hydration — Currently Hydrating, Hydration Queue Size, Slowest Hydrating Collections (top-15 horizontal bar)
4. Dataflows — Dataflow Count, Dataflow Count (per worker), Dataflow Elapsed Rate (log scale)
5. Arrangements — Arrangement Rate, Arrangement Rate (per worker), 3 record-count tables (System / User / Transient)

**Storage Objects**

1. Storage Objects Summary — Active Sources, Active Sinks, Active Tables
2. Sources — Source Types donut, Sources by Status table, Source Bytes Received (rate)
3. Sinks — Sink Types donut, Sink Throughput, Sink Lag (staged minus committed)
4. Iceberg Sinks (**collapsed by default**) — Commit Latency p50/p90/p99, Commit Failures & Conflicts, File & Snapshot Rate
5. Kafka Sinks (**collapsed by default**) — TX Error Rate, Output Buffer, Connect / Disconnect Rate

## Known stubs and orphans

- **`compute_objects.py` Freshness row** — title-only, reserved for end-to-end freshness/lag metrics. Pick a freshness signal (`mz_internal.mz_materialized_view_refreshes`?) when filling it in.
- **`dataflows.py`** — orphaned after Dataflows became a row inside Compute Objects rather than its own tab. Safe to delete; only referenced from `overview_dashboard.py`'s import history (now removed).

## Reference environments

Materialize developers may have access to an internal shared Grafana with multiple test environments. It can be useful to look at queries in live environments when building dashboards. **Do not use environments without explicit permission.**

Always scope investigative queries with `materialize_cloud_organization_id="..."` when testing — these are shared envs and you don't want to mix data across them.

## Cleanup / refactor candidates

Tracked items that are working but could be tidier:

- **`ENV_SCOPED_NOTE` is duplicated** in `compute_objects.py` and `storage_objects.py`. Lift to `visualization.py` (or a sibling `_messages.py` if it grows).
- **`_COMPUTE_FILTER` and `_ARRANGEMENT_FILTER` are the same string** in two modules. Lift to a shared place; rename to something neutral like `_LONGFORM_CLUSTER_FILTER`.
- **`dataflows.py` is orphaned.** Safe to `rm`.
- **The Compute Objects "Freshness" row is a title-only stub.** Pick a freshness signal and fill it in (`mz_materialized_view_lag_seconds` in newer Materialize versions, or a derived metric from frontier metrics).
- **`mz-mon-` prefix isn't enforced in `MzDashboard.UID`** values today (the class has `UID = "env-top"` and `MzDashboard.__init__` prefixes it). Consistent across all current dashboards (one). Worth a validator if more dashboards land.
