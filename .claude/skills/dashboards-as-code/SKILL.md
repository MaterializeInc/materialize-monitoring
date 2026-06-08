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

1. Environment Health — Environment Status, Availability, Last Restart, Currently Hydrating (mirror), Max Lag (Select Time Range), Current CPU Usage, Current Memory Usage
2. Environment Info — Materialize Version, Total CPU Capacity, Total Memory

**Kubernetes Workloads**

1. Resources Summary — Total CPU Capacity, Total Memory (includes monitoring)
2. Workload Readiness — Pod Readiness, StatefulSet Readiness, Deployment Readiness
3. Pod Metrics — Pod CPU Usage, Pod Memory Usage
4. Pod Networking — Rx, Tx, Errors, Packet Drops

**Cluster Objects / Replicas**

1. Cluster Summary — Cluster Count, Replica Count
2. Replication / Availability — Replica Sizes (donut). (Replica AZs intentionally unwired — `materialize_cloud_availability_zone` is cloud-only AND AZ semantics confuse the target audience; the `_az_distribution_panel` method is kept but not added to the row.)
3. Cluster Information — Cluster Information table

**Connections / Activity**

1. Connection Summary — Active Sessions, Active Queries, Adapter Command Rate
2. Queries — Distribution donut, Query Rate, Peek Latency p50/p90/p99 (3 separate panels)
3. Adapter Commands — Adapter Commands by Application table

**Compute Objects**

1. Compute Objects Summary — Active MV, Active Indexes, Active Views, Active Subscribes (donut), Index Types (donut)
2. **Freshness** — Frontier Lag by Cluster, Most-Lagged Collections (both from `mz_dataflow_wallclock_lag_seconds`, sentinel-filtered)
3. Hydration — Currently Hydrating, Hydration Queue Size, Slowest Hydrating Collections (top-15 horizontal bar)
4. Dataflows — Dataflow Count, Dataflow Count (per worker), Dataflow Elapsed Rate (log scale)
5. Arrangements — Arrangement Rate, Arrangement Rate (per worker), 3 record-count tables (System / User / Transient)

**Storage Objects**

1. Storage Objects Summary — Active Sources, Active Sinks, Active Tables
2. Sources — Source Types donut, Sources catalog table, Source Bytes Received (rate), Source Ingestion by Replica (`mz_source_messages_received` per replica — divergence detector), Source Upstream Errors (commit-failure rate + `offset_committed > offset_known` disconnect indicator, threshold-colored)
3. Sinks — Sink Types donut, Sink Throughput, Sink Lag (staged minus committed)
4. Iceberg Sinks (**collapsed by default**) — Commit Latency p50/p90/p99, Commit Failures & Conflicts, File & Snapshot Rate
5. Kafka Sinks (**collapsed by default**) — TX Error Rate, Output Buffer, Connect / Disconnect Rate

## Known stubs and orphans

- **`compute_objects.py` Freshness row** — filled with `mz_dataflow_wallclock_lag_seconds` (per-cluster max frontier lag + topk laggiest collections). Note the u64::MAX (`~1.8e19`) sentinel for collections with no established frontier — filtered with `< 1e9`; the metric is a summary with `quantile` `0`/`1` only (take `1` for worst-case).
- **`dataflows.py`** — orphaned after Dataflows became a row inside Compute Objects rather than its own tab. Safe to delete; only referenced from `overview_dashboard.py`'s import history (now removed).

## Self-managed metric migration (done)

The dashboard was migrated off the cloud-only `v2_mz_*` family and `materialize_cloud_organization_id` onto self-managed `mz_*` metrics + `materialize_cloud_organization_name` filtering (see [Deployment target](../../../docs/content/reference/internal/dashboard/style-guidelines.md#deployment-target-self-managed-vs-cloud)). Also fixed: `metrics_datasource()` no longer pins a dev datasource name (`$metricsDatasource` now resolves to the instance default), which was silently breaking every query.

These panels have **no self-managed metric** and are intentionally kept with a `TODO(self-managed)` + `no_value` (they render blank/0 until a metric exists, rather than being deleted):

- Compute Objects: **Slowest Hydrating Collections** (per-collection hydration *time* — `v2_mz_compute_hydration_time_seconds` is cloud-only, no self-managed equivalent confirmed with the team for this release; description points at `mz_internal.mz_compute_hydration_times` SQL). (**Currently Hydrating** was since revived via the wallclock-lag sentinel — see below; **Active Indexes** and **Index Types** were wired to `mz_indexes_count`.)
- Cluster Objects: **Replica Availability Zones** — `materialize_cloud_availability_zone` is cloud-only AND AZ semantics confuse the end-user audience, so it was **intentionally unwired** (method kept, not added to the row). Don't re-add without product sign-off.

**Currently Hydrating = wallclock-lag sentinel count (no status metric exists):** there is no source/sink/object *status* or hydration-state metric on self-managed. But a collection with no established output frontier reports the `mz_dataflow_wallclock_lag_seconds` u64::MAX sentinel (`> 1e15`), so `count(... > 1e15)` (with `instance_id!=""`) is a real-time **hydration-queue proxy**: it **spikes briefly whenever a replica restarts** (dataflows re-hydrating) and drains back to 0 — that's the signal we wanted, NOT "stuck." A count that *stays* elevated is the genuinely-broken case (e.g. `pg_src2`, status `created`, never hydrated — it sits persistently at 1). This backs the revived **Currently Hydrating** stat (Summary mirror + Compute -> Hydration row); a neutral sparkline, deliberately NOT alarm-colored, since brief spikes are normal. Metrics expose only `collection_id`; the description hands off to `mz_internal.mz_hydration_statuses WHERE NOT hydrated` / `mz_source_statuses` / the console Objects view for names. (An earlier separate red "Stuck Objects" stat was removed — same query, but alarm-on-any false-fired on every routine restart.)

Complementary failure-mode signals now exist (none is a status metric — that's SQL-only):
1. **Currently Hydrating** (Summary + Compute -> Hydration) — wallclock-lag sentinel count; brief spike on replica restart = normal (re)hydration, *sustained* non-zero = a collection that never got a frontier (created/failed-to-start, e.g. `pg_src2`).
2. **Frontier Lag** (Compute -> Freshness) — hydrated but falling behind.
3. **Source Upstream Errors** (Storage -> Sources) and the **Kafka/Iceberg sink error panels** — two source signals on one panel: **commit-failure rate** (`mz_source_offset_commit_failures` — upstream reachable but *rejects* the commit) AND a **disconnected 0/1 indicator** (`offset_committed > offset_known` — broker/DB unreachable so `offset_known` collapsed; the `BrokerTransportFailure` stall). The latter is essential: **commit-failures does NOT fire for an unreachable broker** (the source never reaches the commit step), which surprised us mid-testing — a fully broker-down Kafka source sat `stalled` with commit-failures flat at 0, and only the offset-disconnect signal (plus frontier lag) caught it.
4. **Source Ingestion by Replica** (Storage -> Sources, `mz_source_messages_received` per replica) — a *silent* per-replica stall: a restarted replica that can't resume pulling reads 0 while siblings ingest, but the source stays `Running` and aggregates (and commit-failures = 0) hide it. **This was a real gap** — `sum by (source_id)` aggregate panels mask per-replica failures; the per-replica split (like the per-worker dataflow panel) is the only metric-side place it shows. Pairs with climbing Frontier Lag.

The **Storage / "Sources and Sinks" tab** was later rebuilt against live sources/sinks (real RDS/MSK upstreams on cluster `ingest`):

- **Active Sources/Sinks**, **Source Types**, **Sink Types**, and the **Sources** catalog table now use **`mz_storage_objects`** — the progress-free catalog metric (`count(group by (id) (...))`). This fixes the `mz_sources_count`/`mz_sinks_count` progress-subsource double-count (3 PG sources → `type="postgres"`=6).
- **Sources by Status** → renamed **Sources**: there is no source/sink status metric on self-managed, so it's a catalog table (id/type/connection/envelope/cluster); live status is SQL-only (`mz_internal.mz_source_statuses`).
- Throughput/lag/Iceberg/Kafka sink panels: `_COMPUTE_FILTER` (long-form `cluster_environmentd_*` ids) **verified** against live `mz_source_bytes_received` / `mz_sink_bytes_committed`. Caveat: the `$mzClusterList` picker lists compute clusters only, so a storage-only ingest cluster isn't selectable — default "All" shows everything.

**Cloud/self-managed convergence (`$sqlMetricPrefix`, in progress):** SQL-derived metrics differ only by prefix between envs (`mz_X` self-managed / `v2_mz_X` cloud). The `sqlMetricPrefix` template variable (auto-detects via `…compute_cluster_status`) lets one query serve both: `${sqlMetricPrefix}compute_cluster_status`. **Only** prefix SQL-derived metrics (catalog `*_count`, `compute_cluster_status`, `storage_objects`, `object_id`, `workload_clusters`, arrangement-introspection, `dataflow_elapsed`, `compute_hydration_time_seconds`); genuine instrumentation (`arrangement_maintenance`, `source_*`/`sink_*` throughput, `peek_duration`, `query_total`, `wallclock_lag`, …) is bare `mz_` in both envs and must NOT be prefixed (would become a nonexistent `v2_mz_…` in cloud). f-strings escape it `${{sqlMetricPrefix}}`; table `excludeByName` must list both resolved names. See [style guide → Converging cloud and self-managed](../../../docs/content/reference/internal/dashboard/style-guidelines.md#converging-cloud-and-self-managed-sqlmetricprefix).

**Duplicate-job dedup:** this instance runs 4 Prometheus jobs against the same clusterd `:6878` endpoint, so `mz_source_*` / `mz_sink_*` / `mz_arrangement_*` / `mz_compute_replica_history_*` each appear under multiple `job` values and a plain `sum(rate(...))` reads 4×. Fixed by wrapping the inner rate/gauge in `max without (job) (...)` on the affected sum-rate panels (storage source/sink throughput/lag/Iceberg/Kafka, compute arrangement maintenance rate, dataflow elapsed). `max by(...)` and `histogram_quantile` panels are already job-invariant. **Do not** exclude job names by pattern — several metrics (`mz_compute_cluster_status`, `mz_storage_objects`, `mz_dataflow_elapsed_seconds_total`, the `*_count` metrics) live *only* on a "legacy" job here, so an exclusion list blanks real panels. The real root cause is the overlapping scrape config (helm/Prometheus) — fixing it there makes the `max without (job)` wraps no-ops. See [Known metric quirks](../../../docs/content/reference/internal/dashboard/style-guidelines.md#known-metric-quirks-and-gotchas).

**Datasource scrape interval (empty `rate()` panels):** Prometheus here scrapes every 60s, but the Grafana datasource is provisioned (via terraform/helm) without `jsonData.timeInterval`, so it defaults to 15s and `$__rate_interval` collapses to `~1m` — a single sample, so every `rate()`/`increase()` panel renders blank despite live data. Fix is `jsonData.timeInterval: "60s"` on the datasource (matches the real `scrape_interval`); see [Rate intervals](../../../docs/content/reference/internal/dashboard/style-guidelines.md#rate-intervals). **In flight as of this writing** — the shared terraform/helm datasource block may still be unpatched, so a freshly-provisioned stack will show empty rate panels until `timeInterval` is set. Quick check on any instance: `count_over_time(<metric>[1m])` returning `1` means the window is too short.

Local push: `gcx` context **`local-mzmon`** → `http://localhost:13000`. Render+merge+push helper lives at `/tmp/mzmon_push.sh` (renders the module, carries the live `resourceVersion` + folder annotation forward, `gcx dashboards update`). The Grafana MCP is also wired to the same local instance for query verification.

## Reference environments

Materialize developers may have access to an internal shared Grafana with multiple test environments. It can be useful to look at queries in live environments when building dashboards. **Do not use environments without explicit permission.**

When testing against a *cloud* shared env, scope queries to one environment so you don't mix data across tenants. **The dashboards target self-managed Materialize**, where the scoping label is `materialize_cloud_organization_name="..."` (cloud's hex `materialize_cloud_organization_id` does not exist on self-managed, and neither does the `v2_mz_*` metric family). Always verify which labels/metrics actually exist on the instance you're querying with `list_prometheus_label_names` / `list_prometheus_metric_names` before assuming — see [Deployment target: self-managed vs cloud](../../../docs/content/reference/internal/dashboard/style-guidelines.md#deployment-target-self-managed-vs-cloud).

## Cleanup / refactor candidates

Tracked items that are working but could be tidier:

- **`ENV_SCOPED_NOTE` is duplicated** in `compute_objects.py` and `storage_objects.py`. Lift to `visualization.py` (or a sibling `_messages.py` if it grows).
- **`_COMPUTE_FILTER` and `_ARRANGEMENT_FILTER` are the same string** in two modules. Lift to a shared place; rename to something neutral like `_LONGFORM_CLUSTER_FILTER`.
- **`dataflows.py` is orphaned.** Safe to `rm`.
- **Hydration is SQL-only on self-managed.** No Prometheus metric exposes per-collection hydration state/time (`v2_mz_compute_hydration_time_seconds` is cloud-only; `mz_compute_controller_hydration_queue_size` is just the controller queue and reads 0 even with many objects mid-hydration). The two Hydration panels stay backed by the cloud metric (blank here) with descriptions pointing at `mz_internal.mz_hydration_statuses` / `mz_compute_hydration_times` (SQL); the live metric-side proxy is the Freshness row (`wallclock_lag`).
- **`mz-mon-` prefix isn't enforced in `MzDashboard.UID`** values today (the class has `UID = "env-top"` and `MzDashboard.__init__` prefixes it). Consistent across all current dashboards (one). Worth a validator if more dashboards land.
