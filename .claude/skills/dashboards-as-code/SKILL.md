---
name: dashboards-as-code
description: |
  Use this skill when building, modifying, reviewing, or pushing Grafana dashboards under `packages/dashboards/` (Materialize observability dashboards generated from Rust against the vendored Grafana schemas). Also use it when writing the panel descriptions and PromQL those dashboards draw from the query registry under `packages/queries/`, picking palettes, or working through Materialize-specific PromQL patterns (cluster/replica filtering, peek latency, source/sink metrics, label-family quirks).
---

# Dashboards as Code

This skill is the entry point for the Materialize dashboards-as-code project.
**Stable conventions live in the repo docsite** under
[`docs/content/reference/internal/dashboard/`](../../../docs/content/reference/internal/dashboard/) — this file is
intentionally slim and links into the docsite at heading-level granularity.
The non-link content below is the **state snapshot**: what currently exists, what's in flight, and what's queued for
cleanup.

## Audience reminder

The **dashboards themselves** target Materialize end users: database-literate operators with basic graph-reading fluency
but minimal cloud / Kubernetes / observability expertise.
SQL is fair game; jargon like "differential dataflow's arrangement" needs a one-liner explanation.
Panel descriptions, titles, and cluster names should respect that baseline.

The **docsite reference pages** target repo contributors (SRE, Field Engineering, CloudOps, Database Engineers) and AI agents reading this skill.

## Where to find what

| Looking for… | Read |
|---|---|
| Grafana target versions, Dashboard v1/v2 schema state, SDK choices | [SDKs and Schemas](../../../docs/content/reference/internal/dashboard/sdks.md) |
| Code structure, UID conventions, push process, `gcx dashboards update` vs ad-hoc v2 API | [Generating and Pushing Dashboards](../../../docs/content/reference/internal/dashboard/generating.md) |
| Palettes, layouts, panel visualization, panel description voice, PromQL conventions, label families, metric quirks, PromQL recipes, shared-constants table | [Style Guidelines](../../../docs/content/reference/internal/dashboard/style-guidelines.md) |
| How a panel gets its query and prose from the registry | [SDKs → Panels do not write PromQL](../../../docs/content/reference/internal/dashboard/sdks.md#panels-do-not-write-promql) |
| The query registry itself: schema, engines, templating, consumers | [Queries](../../../docs/content/reference/internal/queries/overview.md) |
| What each test suite covers, the frozen baseline, artifact freshness | [Testing](../../../docs/content/reference/internal/dashboard/testing.md) |

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
- [Shared constants and helpers](../../../docs/content/reference/internal/dashboard/style-guidelines.md#shared-constants-and-helpers)

And into Generating:

- [PUT body shape](../../../docs/content/reference/internal/dashboard/generating.md#put-body-shape) — required
  Kubernetes-style envelope when pushing v2 dashboards via `grafana_api_request`
- [Service account permissions](../../../docs/content/reference/internal/dashboard/generating.md#service-account-permissions) — decoding 403s

## Schema reference files

When uncertain about the exact shape Grafana expects, read the cog-generated JSON Schema (draft-07) documents vendored
at `packages/mzmon-lib/schemas/grafana/`.
The three dashboard documents there are:

- `dashboard.jsonschema.json` — v1
- `dashboardv2beta1.jsonschema.json` — v2beta1
- `dashboardv2.jsonschema.json` — v2

The other 52 documents in the same directory cover the panel, datasource, and `common` packages — that is where a
panel's `options` and `fieldConfig` shapes live, not in the dashboard documents.

`packages.json` beside them maps each document to its Grafana plugin id — the value that goes in `VizConfigKind.group`
or `DataQueryKind.group`.
Read it rather than assuming the document name: `annotationslist` publishes as `annolist`.

The whole set is vendored from the `grafana/grafana-foundation-sdk` release tag `v0.0.18` (June 12, 2026), generated there by cog `v0.1.20`.
The v2 and v2beta1 documents track Grafana `v13.0.2`; v1 tracks Grafana `v11.6.0`.
Re-vendor with `bin/fetch-grafana-schemas.sh`; Renovate maintains the tag pinned in that script, and `PROVENANCE.md` records the current pin.

Rust types are generated from these schemas into `packages/mzmon-lib/src/grafana/generated/` by `bin/gen-grafana-models.sh`.
See [Rust models](../../../docs/content/reference/internal/dashboard/sdks.md#rust-models) for why the layout is one
module per document, and for the schema quirks that leak into any code built on them.

---

# Current Dashboard State

This section captures the live state of the dashboards in this repo so the next session has something concrete to start
from.
**Update it when state changes meaningfully** (new dashboard, new tab, retired panel, theme reassignment).

## Dashboard inventory

| Artifact stem | Module | UID | Title |
|---|---|---|---|
| `env-top` | `grafana/env_top/` | `mz-mon-env-top` | Materialize Environment Overview |

Rendered to `charts/…/pre-rendered/dashboards/grafana/env-top.yaml` (chart) and `docs/assets/dashboards/grafana/env-top.json`
plus `gcp-env-top.json` (docsite). The two clouds differ only in the `target-cloud` annotation.

The live UID diverged from the codified one before `mz-mon-env-top` became authoritative — see
[UID selection and behavior](../../../docs/content/reference/internal/dashboard/generating.md#uid-selection-and-behavior).

Six tabs, in declared order. Shades come from `env_top/theme.rs`, which is the source of truth for visual identity:

| # | Tab title | Module | Shade |
|---|---|---|---|
| 1 | Summary | `summary.rs` | none of its own — borrows the shade of whichever tab each panel points at |
| 2 | Kubernetes Workloads | `kubernetes.rs` | `KUBERNETES` `#0077BB` (blue) |
| 3 | Connections / Activity | `connections.rs` | `CONNECTIONS` `#33BBEE` (cyan) |
| 4 | Cluster Objects / Replicas | `clusters.rs` | `CLUSTERS` `#009988` (teal) |
| 5 | Compute Objects | `compute.rs` | `COMPUTE` `#EE7733` (orange) |
| 6 | Sources and Sinks | `sources_sinks.rs` | `SOURCES_SINKS` `#CCBB44` (yellow) |

The Summary tab's CPU/memory capacity panels borrow the Kubernetes shade, and its Currently Hydrating panel is the
same definition the Compute tab uses (`env_top/mod.rs`), with the shade as the only parameter.

## Tab-by-tab row structure

Generated from the rendered artifact; regenerate rather than hand-editing when the layout changes.

**Summary**

1. Environment Health — Environment Status, Environment Availability (Select Time Range), Last Restart Time, Currently
   Hydrating, Max Lag (Select Time Range), Current CPU Usage (5 min), Current Memory Usage
2. Environment Info — Materialize Version, Total CPU Capacity, Total Memory

**Kubernetes Workloads**

1. Resources Summary (**header hidden**) — Total CPU Capacity, Total Memory
2. Workload Readiness (**header hidden**) — Pod Readiness, StatefulSet Readiness, Deployment Readiness
3. Pod Metrics — Pod CPU Usage, Pod Memory Usage
4. Pod Networking — Pod Network Rx, Pod Network Tx, Pod Network Errors, Pod Network Packet Drops

**Connections / Activity**

1. Connection Summary (**header hidden**) — Active Sessions, Active Queries, SQL Control Plane Command Rate
2. Queries — Query Distribution (by statement_type), Query Rate (by statement_type / session_type), Peek Latency (p50),
   Peek Latency (p90), Peek Latency (p99)
3. SQL Control Plane Commands — SQL Control Plane Commands by Application (one column: a wide table needs the room)

**Cluster Objects / Replicas**

1. Cluster Summary (**header hidden**) — Cluster Count, Replica Count
2. Replication / Availability — Replica Sizes
3. Cluster Information — Cluster Information

**Compute Objects**

1. Compute Objects Summary (**header hidden**) — Active Materialized Views, Active Indexes, Active Views, Active Subscribes, Index Relationship Types
2. Freshness — Freshness Lag by Cluster, Most-Lagged Collections
3. Hydration — Currently Hydrating, Hydration Queue Size, Slowest Hydrating Collections
4. Dataflows — Dataflow Count, Dataflow Count (per worker), Dataflow Elapsed Rate
5. Arrangements — Arrangement Maintenance Rate, Arrangement Maintenance Rate (per worker), System / User / Transient Collections — Record Counts

**Sources and Sinks**

1. Storage Objects Summary (**header hidden**) — Active Sources, Active Sinks, Active Tables
2. Sources — Source Types, Sources, Source Bytes Received (rate), Source Ingestion by Replica, Source Upstream Errors
3. Sinks — Sink Types, Sink Throughput (committed), Sink Lag (staged minus committed)
4. Iceberg Sinks (**collapsed**) — Iceberg Commit Latency (p50 / p90 / p99), Iceberg Commit Failures & Conflicts, Iceberg File & Snapshot Rate
5. Kafka Sinks (**collapsed**) — Kafka TX Error Rate, Kafka Output Buffer (messages), Kafka Connect / Disconnect Rate

Replica AZs are intentionally unwired: `materialize_cloud_availability_zone` is cloud-only, and AZ semantics confuse
the target audience.

## Notes on the trickier panels

- **Freshness** reads `mz_dataflow_wallclock_lag_seconds`. Collections with no established frontier report a
  `u64::MAX` (`~1.8e19`) sentinel, filtered with `< 1e9`; the metric is a summary carrying `quantile` `0`/`1` only, so
  take `1` for worst-case. Those excluded collections are what Currently Hydrating counts.
- **Source Ingestion by Replica** is a divergence detector: replicas read upstream independently, so one flat at 0
  while its siblings ingest has lost its connection — the aggregate throughput panel hides that.
- **Source Upstream Errors** pairs a commit-failure rate with an `offset_committed > offset_known` disconnect
  indicator, because the broker-unreachable case never reaches the commit step.

## Self-managed metric migration (done)

The dashboard was migrated off the cloud-only `v2_mz_*` family and `materialize_cloud_organization_id` onto self-managed
`mz_*` metrics + `materialize_cloud_organization_name` filtering (see
[Deployment target](../../../docs/content/reference/internal/dashboard/style-guidelines.md#deployment-target-self-managed-vs-cloud)
).
Also fixed: `metrics_datasource()` no longer pins a dev datasource name (`$metricsDatasource` now resolves to the
instance default), which was silently breaking every query.

These panels have **no self-managed metric** and are intentionally kept with a `NoValue` explaining the gap (they render
blank/0 until a metric exists, rather than being deleted):

- Compute Objects: **Slowest Hydrating Collections** (per-collection hydration *time* —
  `v2_mz_compute_hydration_time_seconds` is cloud-only, no self-managed equivalent confirmed with the team for this
  release; description points at `mz_internal.mz_compute_hydration_times` SQL).
  (**Currently Hydrating** was since revived via the wallclock-lag sentinel — see below; **Active Indexes** and **Index
  Types** were wired to `mz_indexes_count`.)
- Cluster Objects: **Replica Availability Zones** — `materialize_cloud_availability_zone` is cloud-only AND AZ semantics
  confuse the end-user audience, so it is **intentionally unwired**.
  Don't re-add without product sign-off.

**Currently Hydrating = wallclock-lag sentinel count (no status metric exists):** there is no source/sink/object
*status* or hydration-state metric on self-managed.
But a collection with no established output frontier reports the `mz_dataflow_wallclock_lag_seconds` u64::MAX sentinel
(`> 1e15`), so `count(... > 1e15)` (with `instance_id!=""`) is a real-time **hydration-queue proxy**: it **spikes
briefly whenever a replica restarts** (dataflows re-hydrating) and drains back to 0 — that's the signal we wanted, NOT
"stuck." A count that *stays* elevated is the genuinely-broken case (e.g. `pg_src2`, status `created`, never hydrated
— it sits persistently at 1).
This backs the revived **Currently Hydrating** stat (Summary mirror + Compute -> Hydration row); a neutral sparkline,
deliberately NOT alarm-colored, since brief spikes are normal. Metrics expose only `collection_id`; the description
hands off to `mz_internal.mz_hydration_statuses WHERE NOT hydrated` / `mz_source_statuses` / the console Objects view
for names.
(An earlier separate red "Stuck Objects" stat was removed — same query, but alarm-on-any false-fired on every routine
restart.)

Complementary failure-mode signals now exist (none is a status metric — that's SQL-only):
1. **Currently Hydrating** (Summary + Compute -> Hydration) — wallclock-lag sentinel count; brief spike on replica
   restart = normal (re)hydration, *sustained* non-zero = a collection that never got a frontier
   (created/failed-to-start, e.g. `pg_src2`).
2. **Frontier Lag** (Compute -> Freshness) — hydrated but falling behind.
3. **Source Upstream Errors** (Storage -> Sources) and the **Kafka/Iceberg sink error panels** — two source signals on
   one panel: **commit-failure rate** (`mz_source_offset_commit_failures` — upstream reachable but *rejects* the commit)
   AND a **disconnected 0/1 indicator** (`offset_committed > offset_known` — broker/DB unreachable so `offset_known`
   collapsed; the `BrokerTransportFailure` stall).
   The latter is essential: **commit-failures does NOT fire for an unreachable broker** (the source never reaches the
   commit step), which surprised us mid-testing — a fully broker-down Kafka source sat `stalled` with commit-failures
   flat at 0, and only the offset-disconnect signal (plus frontier lag) caught it.
4. **Source Ingestion by Replica** (Storage -> Sources, `mz_source_messages_received` per replica) — a *silent*
   per-replica stall: a restarted replica that can't resume pulling reads 0 while siblings ingest, but the source stays
   `Running` and aggregates (and commit-failures = 0) hide it.
   **This was a real gap** — `sum by (source_id)` aggregate panels mask per-replica failures; the per-replica split
   (like the per-worker dataflow panel) is the only metric-side place it shows.
   Pairs with climbing Frontier Lag.

The **Storage / "Sources and Sinks" tab** was later rebuilt against live sources/sinks (real RDS/MSK upstreams on cluster `ingest`):

- **Active Sources/Sinks**, **Source Types**, **Sink Types**, and the **Sources** catalog table now use
  **`mz_storage_objects`** — the progress-free catalog metric (`count(group by (id) (...))`).
  This fixes the `mz_sources_count` /`mz_sinks_count` progress-subsource double-count (3 PG sources → `type="postgres"`
  =6).
- **Sources by Status** → renamed **Sources**: there is no source/sink status metric on self-managed, so it's a catalog
  table (id/type/connection/envelope/cluster); live status is SQL-only (`mz_internal.mz_source_statuses`).
- Throughput/lag/Iceberg/Kafka sink panels filter on the long-form `cluster_environmentd_*` ids, **verified** against
  live `mz_source_bytes_received` / `mz_sink_bytes_committed`.
  Caveat: the `$mzClusterList` picker lists compute clusters only, so a storage-only ingest cluster isn't selectable —
  default "All" shows everything.

**Cloud/self-managed convergence (SQL metric prefix):** SQL-derived metrics differ only by prefix between envs (`mz_X`
self-managed / `v2_mz_X` cloud).
The prefix is **baked in at render time** via the registry's `%%{mzSqlPrefix}` parameter, fed by `--sql-metric-prefix`
(default `mz_`).
This replaced the old `$sqlMetricPrefix` Grafana variable (auto-detected via `…compute_cluster_status`), which GMP
can't run since it can't do `query_result(...)` detection.
**Only** prefix SQL-derived metrics (catalog `*_count`, `compute_cluster_status`, `storage_objects`, `object_id`,
`workload_clusters`, arrangement-introspection, `dataflow_elapsed`, `compute_hydration_time_seconds`); genuine
instrumentation (`arrangement_maintenance`, `source_*` /`sink_*` throughput, `peek_duration`, `query_total`,
`wallclock_lag`, …) is bare `mz_` in both envs and must NOT be prefixed (would become a nonexistent `v2_mz_…` in
cloud).
Table `excludeByName` must list both resolved names.
Nothing is captured at import, so one process can emit both variants; no `v2_mz_` artifact ships today.
See
[style guide → Converging cloud and self-managed](../../../docs/content/reference/internal/dashboard/style-guidelines.md#converging-cloud-and-self-managed-the-sql-metric-prefix)
.

**Filter fragments are render-time parameters, not ConstantVariables:** the `$environmentFilter` / `$containerFilter` /
`$clusterFilter` / `$replicaFilter` hidden ConstantVariables were removed — Grafana's constant-variable interpolation
mangled their nested `$…List` refs and embedded commas.
The registry's `%%{mzEnvironmentFilter}` / `%%{cAdvisorFilter}` / `%%{mzClusterList}` / `%%{mzReplicaList}` parameters
resolve to the matcher text before the query reaches the dashboard; the nested `$…List` references inside them stay real
Grafana variables and resolve at view time.
See [style guide → Intermediates](../../../docs/content/reference/internal/dashboard/style-guidelines.md#intermediates)
.

**Duplicate-job dedup:** this instance runs 4 Prometheus jobs against the same clusterd `:6878` endpoint, so
`mz_source_*` / `mz_sink_*` / `mz_arrangement_*` / `mz_compute_replica_history_*` each appear under multiple `job`
values and a plain `sum(rate(...))` reads 4×.
Fixed by wrapping the inner rate/gauge in `max without (job) (...)` in the registry queries behind the affected sum-rate
panels (storage source/sink throughput/lag/Iceberg/Kafka, compute arrangement maintenance rate, dataflow elapsed).
`max by(...)` and `histogram_quantile` panels are already job-invariant.
**Do not** exclude job names by pattern — several metrics (`mz_compute_cluster_status`, `mz_storage_objects`,
`mz_dataflow_elapsed_seconds_total`, the `*_count` metrics) live *only* on a "legacy" job here, so an exclusion list
blanks real panels.
The real root cause is the overlapping scrape config (helm/Prometheus) — fixing it there makes the `max without (job)`
wraps no-ops.
See
[Known metric quirks](../../../docs/content/reference/internal/dashboard/style-guidelines.md#known-metric-quirks-and-gotchas)
.

**Datasource scrape interval (empty `rate()` panels):** Prometheus here scrapes every 60s, but the Grafana datasource is
provisioned (via terraform/helm) without `jsonData.timeInterval`, so it defaults to 15s and `$__rate_interval`
collapses to `~1m` — a single sample, so every `rate()` /`increase()` panel renders blank despite live data.
Fix is `jsonData.timeInterval: "60s"` on the datasource (matches the real `scrape_interval`); see
[Rate intervals](../../../docs/content/reference/internal/dashboard/style-guidelines.md#rate-intervals).
**In flight as of this writing** — the shared terraform/helm datasource block may still be unpatched, so a
freshly-provisioned stack will show empty rate panels until `timeInterval` is set.
Quick check on any instance: `count_over_time(<metric>[1m])` returning `1` means the window is too short.

Local push: `gcx` context **`local-mzmon`** → `http://localhost:13000`.
Render with `mz-monitoring-build gen-dashboards --format json`, then carry the live `resourceVersion` + folder
annotation forward on the PUT — see
[PUT body shape](../../../docs/content/reference/internal/dashboard/generating.md#put-body-shape).
The Grafana MCP is wired to the same local instance for query verification.

## Reference environments

Materialize developers may have access to an internal shared Grafana with multiple test environments.
It can be useful to look at queries in live environments when building dashboards.
**Do not use environments without explicit permission.**

When testing against a *cloud* shared env, scope queries to one environment so you don't mix data across tenants.
**The dashboards target self-managed Materialize**, where the scoping label is
`materialize_cloud_organization_name="..."` (cloud's hex `materialize_cloud_organization_id` does not exist on
self-managed, and neither does the `v2_mz_*` metric family).
Always verify which labels/metrics actually exist on the instance you're querying with `list_prometheus_label_names` /
`list_prometheus_metric_names` before assuming — see
[Deployment target: self-managed vs cloud](../../../docs/content/reference/internal/dashboard/style-guidelines.md#deployment-target-self-managed-vs-cloud)
.

## Cleanup / refactor candidates

Tracked items that are working but could be tidier:

- **Hydration is SQL-only on self-managed.** No Prometheus metric exposes per-collection hydration state or time
  (`v2_mz_compute_hydration_time_seconds` is cloud-only; `mz_compute_controller_hydration_queue_size` is just the
  controller queue and reads 0 even with many objects mid-hydration). Slowest Hydrating Collections stays backed by the
  cloud metric (blank here) with a description pointing at `mz_internal.mz_hydration_statuses` /
  `mz_compute_hydration_times`; the live metric-side proxy is the Freshness row (`wallclock_lag`).
- **The `mz-mon-` UID prefix is not validated.** One dashboard, consistent today. Worth a check if more land.
- **`packages/ref-alloy-pipelines/` Python is dead.** It imports `py_mzmon_lib.alloy.config_dsl`, a module that no
  longer exists; the `.alloy` files beside it are still the behavioral porting reference. Unrelated to the dashboards,
  but it is the last Python in `packages/`.

Resolved by the Rust port, listed so they are not re-filed: the duplicated `ENV_SCOPED_NOTE` and long-form cluster
filter (prose lives on the registry query, filters are parameters), and the orphaned `dataflows.py`.
