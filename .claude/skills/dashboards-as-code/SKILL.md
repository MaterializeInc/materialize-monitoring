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
| Palettes, layouts, panel visualization, panel description voice, PromQL **and LogQL** conventions, time-range guards, Kubernetes-event shape, label families, metric quirks, recipes, shared-constants table | [Style Guidelines](../../../docs/content/reference/internal/dashboard/style-guidelines.md) |
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
- [Logs dashboard conventions](../../../docs/content/reference/internal/dashboard/style-guidelines.md#logs-dashboard-conventions)
  — Loki-discovered pickers, `all_value` rules, the non-empty-matcher anchor, and how exclusion switches are wired
- [Time-range guards on expensive rows](../../../docs/content/reference/internal/dashboard/style-guidelines.md#time-range-guards-on-expensive-rows)
  — the paired show/hide rows that keep volume panels off a month-wide range
- [Kubernetes events in Loki](../../../docs/content/reference/internal/dashboard/style-guidelines.md#kubernetes-events-in-loki)
  — labels vs structured metadata, and why an event's namespace is the involved object's
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
| `env-logs` | `grafana/env_logs/` | `mz-mon-env-logs` | Materialize Logs and Events |
| `env-upgrade` | `grafana/env_upgrade/` | `mz-mon-env-upgrade` | Materialize Upgrade |
| `infra-logs` | `grafana/infra_logs/` | `mz-mon-infra-logs` | Infrastructure Logs and Events |

Each is rendered to `charts/…/pre-rendered/dashboards/grafana/<stem>.yaml` (chart) and
`docs/assets/dashboards/grafana/<stem>.json` (docsite). **One file per dashboard** — there was a second, `gcp-`
prefixed set until the clouds stopped differing in panel content, which left it recording nothing but its own name.
The `cloud` render option, the `--cloud` / `--prefix` flags and the `target-cloud` annotation went with it.

**`env-upgrade` is installed by default**, because `dashboards.selected` defaults to `["env-*"]` and the stem matches.
While the operator-side instrumentation is unreleased it degrades unevenly, and the split is worth knowing: **Generations
works fully** (every panel reads metrics that predate the change, and the blue/green split comes from pod names), Events
keeps its Kubernetes Activity row, and Reconciliation is empty apart from its two pre-existing gauges. `MIN_MZ_VERSION`
in `env_upgrade/mod.rs` is `v26.41.0` and must stay in step with the Materialize row of
`docs/content/reference/compatibility.md`. Narrow `dashboards.selected` to `["env-top"]` to hold it back.

The live UID diverged from the codified one before `mz-mon-env-top` became authoritative — see
[UID selection and behavior](../../../docs/content/reference/internal/dashboard/generating.md#uid-selection-and-behavior).

## `env-top` tabs

Six tabs, in declared order. Per-tab shades live in `env_top/theme.rs` — the source of truth, and deliberately the
only place they are written down:

| # | Tab title | Module |
|---|---|---|
| 1 | Summary | `summary.rs` |
| 2 | Kubernetes Workloads | `kubernetes.rs` |
| 3 | Connections / Activity | `connections.rs` |
| 4 | Cluster Objects / Replicas | `clusters.rs` |
| 5 | Compute Objects | `compute.rs` |
| 6 | Sources and Sinks | `sources_sinks.rs` |

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

## `env-upgrade` tabs

Three tabs, ordered by descending altitude: what happened, which side of the rollout is ready, is the operator itself
healthy. Shades come from `env_upgrade/theme.rs`.

| # | Tab title | Module |
|---|---|---|
| 1 | Events | `events.rs` |
| 2 | Generations | `generations.rs` |
| 3 | Reconciliation | `reconciliation.rs` |

**This is the repo's first mixed-datasource dashboard.** Events is Loki, Reconciliation is Thanos, and the two are
separate tabs partly because they are scoped differently — see the namespace note below.

**Events** — the first tab in this repo built on Loki rather than Thanos. Rows narrow from verdict to cause:

1. Event Summary (**header hidden**) — Warning Events, Reconciliation Failures, Lifecycle Transitions
2. Rollout — Lifecycle Transitions (timeseries), Lifecycle Events (logs)
3. Operator Health — Reconciliation Failures (timeseries), Reconciliation Failure Events (logs)
4. Kubernetes Activity — Event Rate by Reason (timeseries), Warning Events (logs)
5. All Events (**collapsed**) — All Events (logs)

Each rate panel sits beside the feed it summarizes, in the same row: the chart says *when*, the feed says *what*.

**Generations** — the two sides of a blue/green rollout, split apart:

1. Rollout Status (**header hidden**) — Active Generations, Currently Hydrating, Max Frontier Lag, Pods
2. Versions — Version by Generation (table)
3. Hydration — Hydrating Collections by Generation, Collections by Generation
4. Freshness — Frontier Lag by Generation
5. Footprint — CPU by Generation, Memory by Generation

**Version by Generation** is the row that says what the rollout is *for*. It reads the `mz_version` label off
`compute_cluster_status`, which each generation's own environmentd reports, so the two sides genuinely disagree during
a rollout — over a window spanning one, the table reads `gen 2 → v26.38.2` beside `gen 3 → v26.40.0-rc.1`. A table
rather than a stat because the value is a *string* and a stat cannot show two of those legibly. Two rows with the
*same* version means a forced rollout rather than an upgrade, which is worth confirming before it costs a rehydration.

**Reconciliation** — the operator's control loop, as counters and histograms:

1. Operator Status (**header hidden**) — Reconciling Replicas, Environments Needing Update, Reconciliation Rate, Failed Passes (Select Time Range)
2. Reconciliation Passes — Pass Outcomes, Failed Passes by Controller
3. Duration — Pass Duration (p50/p90/p99), Step Duration (p99)
4. Steps — Step Activity, Step Failures and Abandonments

## Deployment generations (blue/green)

What the Generations tab is built on, and the `$mzGenerationList` selector that drives it.

**The generation is not a label on anything.** orchestratord records it as the `materialize.cloud/generation`
*annotation*, which neither kube-state-metrics nor cAdvisor nor the event pipeline surfaces. Where it does reach a query
is the object **name**, in two shapes:

| Workload | Name shape |
|---|---|
| environmentd | `<prefix>-environmentd-<generation>-<ordinal>` |
| cluster replica | `<prefix>-cluster-<cluster>-replica-<replica>-gen-<generation>-<ordinal>` |

Three render-context parameters carry that, so the pattern lives in one place and cannot drift:

- `%%{mzGenerationFilter}` — `pod=~".*-(environmentd|gen)-(${mzGenerationList:regex})-[0-9]+"`, for metrics.
- `%%{mzGenerationPattern}` — the same shape as a *capture*, for the `label_replace` that lifts the number into a
  `generation` label panels can group and legend by. A parameter rather than a template function, because the
  `label_replace` has to wrap an inner selector while a function wraps the whole template.
- `%%{mzGenerationEventFilter}` — for events, where the generation is in the object name and the filter is a pipeline
  stage rather than a stream selector.

**Two ad-hoc filters, not one.** An ad-hoc variable resolves its label keys *from a datasource*, so `metricAdhoc`
(Prometheus) cannot offer Loki's stream labels — `env-upgrade` defines `logsAdhoc` beside it, and both sit at the tail of
the controls row as escape hatches rather than steps in the funnel. `logsAdhoc` seeds **no base filter**, unlike the
metrics one: Grafana ANDs a base filter into the query's own selector, and the obvious seed (the environment namespace)
would narrow a stream selector that deliberately spans the operator's namespace too, silently dropping every event the
operator published. Its keys are Loki *stream* labels; structured metadata like `reason` and `kind` is filtered in the
query instead.

`grafana/transform.rs` was promoted out of `env_top/` when the version table became its second consumer — it builds
Grafana transformation JSON and knows nothing about Materialize, so copying it would have started two divergent copies
of the same unschematized blobs.

**The event filter's `or` arm is load-bearing.** Only a handful of the objects a rollout touches carry a generation —
on a representative deployment, 6 of 70 event names — and every operator lifecycle event is filed against the
`Materialize` resource, which carries none. So the filter is
`name=~"<selected>" or name!~"<any generation>"`: keep what belongs to a selected generation, and keep what belongs to
no generation. A bare `name=~` would drop the entire rollout narrative and keep only the pod noise. RE2 has no negative
lookahead, which is why this is an `or` rather than one clever pattern.

Only the four deployment-wide event feeds filter by generation. The operator's own queries do not — their events carry
no generation, so it could only ever be a no-op there.

**`$mzGenerationList` refreshes on time-range change**, alone among the variables here. Which generations exist is a
property of the *window*: the old side is torn down after promotion, so widening the range to cover a rollout is exactly
how its other side comes back into view. It has no `all_value` — a literal like `[0-9]+` would be regex-*escaped* by the
`:regex` format and match nothing.

**Hydration is still the wallclock-lag sentinel**, now split by generation. `mz_dataflow_wallclock_lag_seconds` is
emitted by environmentd, so its `pod` label carries the generation and the split is free. Two things about the series:

- `instance_id!=""` is load-bearing, keeping it to collections attached to a compute instance.
- **Score with `> bool`, do not filter with `>`.** A filtering comparison drops the non-matching series, so `count`
  emits *no sample* once a generation finishes hydrating: the line stops instead of reaching zero, and a stat reducing
  on the last non-null value goes on showing the last count it saw forever. `sum by (generation) (max by (…) (… > bool
  1e15))` scores every collection 1 or 0, so the series stays present and lands on zero — the descent the panel exists
  to show. `env-top`'s unsplit version gets there with `or vector(0)`, which is not an option once the panel groups by
  generation: that appends a series carrying no labels.
- A sparse series also invites a specific misreading — "all emitted points are non-zero" looks exactly like a Thanos
  downsampling artifact and is not one. Values were verified identical across query windows.

## `env-logs` tabs

Two tabs, shaded from `env_logs/theme.rs`. Events deliberately reuses the shade it carries on `env-upgrade`, since
it is the same kind of content — an operator moving between the two is not told otherwise.

| # | Tab title | Module |
|---|---|---|
| 1 | Logs | `logs.rs` |
| 2 | Events | `events.rs` |

**Logs** — Volume (Log Rate, Warning Rate, Log Rate by App, Log Rate by Level), Warnings (feed), All Logs (feed).
**Events** — Activity (rate by reason, rate by namespace), Warnings (feed), All Events (feed).

## `infra-logs` tabs

The first of the `infra-*` family — scoped to the cluster rather than to an environment.
**Not installed by default**: `dashboards.selected` is `["env-*"]`, so the whole family needs a release to widen it.

| # | Tab title | Module |
|---|---|---|
| 1 | Logs | `logs.rs` |
| 2 | Nodes | `nodes.rs` |
| 3 | Events | `events.rs` |

**Logs** — Volume (rate by component, rate by namespace, warning rate), Warnings feed, All Logs feed.
**Nodes** — Journal Volume (rate by unit), Node Warnings feed, Node Journal feed.
**Events** — Activity (by reason, by namespace), Warnings feed, All Events feed.

### Why it is a second dashboard rather than a wider `env-logs`

Two things `env-logs` cannot reach however its pickers are set:

- **The node journal.** Journal lines carry `unit`, `component`, `job`, `level` and `service_name` and **no `namespace`,
  `app` or `container`** — they come from the node, not a pod. Every `env-logs` selector requires a namespace, so those
  lines are excluded by construction. `unit` is their anchor, with `all_value` `.+`, standing in for the namespace
  matcher container-log selectors lean on.
- **Sub-components.** `component` splits `loki` into eight processes (`canary`, `querier`, `ingester`,
  `query-frontend`, `index-gateway`, `compactor`, `distributor`, `ruler`) and `thanos` into three. A Materialize
  environment has none, so adding the picker there would be a control that does nothing.

A third, smaller reason: `container` is the only picker that reaches workloads with no `app` label, which on a
representative install is the whole of `kube-system` (14 containers, `app` empty). It sits in the controls menu.

### What the two dashboards share

The variable **names** and the Kubernetes-**event queries**. `materialize.events.cluster.*` carries no
Materialize-specific filter and is scoped by the same `$logNamespaceList` both dashboards define, so the events half is
one set of definitions serving both. `log_namespaces(opens_on)` takes the opening selection as an argument — the
Materialize pattern for `env-logs`, `.+` for `infra-logs` — which is the *only* intended difference.

The container-log queries are **not** shared: `infra.logs.*` carries the component and container filters, and adding
those to `materialize.logs.*` would oblige `env-logs` to define pickers it has no use for.

## orchestratord reconciliation metrics

What the Reconciliation tab is built on. Sources: `src/orchestratord/src/reconcile.rs` and `metrics.rs` in the
Materialize repo, which carry the authoritative prose in their `help` strings and doc comments.

| Metric | Labels | Notes |
|---|---|---|
| `orchestratord_reconciliations_total` | `controller`, `event_type`, `outcome` | One trip through a controller's work |
| `orchestratord_reconciliation_duration_seconds` | `controller`, `event_type` | Histogram |
| `orchestratord_reconciliation_steps_total` | `controller`, `step`, `outcome` | The named phases within a pass |
| `orchestratord_reconciliation_step_duration_seconds` | `controller`, `step` | Histogram, same buckets |
| `orchestratord_is_leader` | — | Predates the rest |
| `environmentd_needs_update` | — | Predates the rest |

**They carry no organization label**, so the environment picker does not narrow them — one operator reconciles every
environment in the cluster. `%%{mzOperatorNamespaceFilter}` is the only scope that applies, and unlike the *events*
(which are filed in the involved object's namespace) these metrics really do carry `namespace="<operator namespace>"`.
The two tabs therefore scope in opposite directions, which is the trap worth remembering.

**Sum across replicas, always.** Only the leader reconciles; the others export the same families sitting at zero.
`environmentd_needs_update` is explicitly reset on losing the lease so a former leader does not go on publishing its
last observation.

**Outcome vocabulary** (`applied`, `waiting`, `skipped`, `failed`, `abandoned`):

- `waiting` is **success**, not a warning — a rollout spends most of its passes there while the new generation's pods
  come up.
- `abandoned` is **not a failure signal**. A step records it when it did not reach a conclusion, which covers an error
  propagating out *and* a pass cancelled by a leadership handoff or shutdown; a `Drop` cannot tell them apart. Alert on
  `orchestratord_reconciliations_total{outcome="failed"}`, which is recorded from the reconciler's actual result and
  which a cancelled pass never reaches, and read the step counter to *locate* it.

**Duration is not rollout duration.** A pass waiting on pods returns promptly and asks to run again rather than
blocking, so the histogram measures work done per pass. The rollout's wall-clock length is the span between its first
and last transition on the Events tab.

**The buckets are deliberately coarse** — 10ms, 50ms, 250ms, 1s, 5s, 30s. A percentile is therefore the boundary of the
bucket the value fell in, not the value; read it as an order of magnitude. Finer buckets would cost several times the
series for detail no operator question asks for, and steps share the pass's bucket set so a step's latency reads
against the pass it belongs to.

**Test tabs in the scope their dashboard builds them in.** `queries::test_operator_queries()` exists because
`test_queries()` uses `DashboardScope::default()`, where the operator namespace is the pinned literal rather than
`$operatorNamespace` — an assertion about a rendered selector under the default scope is about a rendering that never
ships.

## Notes on the trickier panels

- **Freshness** reads `mz_dataflow_wallclock_lag_seconds`. Collections with no established frontier report a
  `u64::MAX` (`~1.8e19`) sentinel, filtered with `< 1e9`; the metric is a summary carrying `quantile` `0`/`1` only, so
  take `1` for worst-case. Those excluded collections are what Currently Hydrating counts.
- **Source Ingestion by Replica** is a divergence detector: replicas read upstream independently, so one flat at 0
  while its siblings ingest has lost its connection — the aggregate throughput panel hides that.
- **Source Upstream Errors** pairs a commit-failure rate with an `offset_committed > offset_known` disconnect
  indicator, because the broker-unreachable case never reaches the commit step.

## Self-managed metric migration (done)

Migrated off the cloud-only `v2_mz_*` family and `materialize_cloud_organization_id` onto self-managed `mz_*` metrics
and `materialize_cloud_organization_name`, with SQL-derived metrics converged behind `$sqlMetricPrefix`.
The roadmap records it as shipped; every rule that came out of it — the wallclock-lag sentinel behind Currently
Hydrating, duplicate-job dedup on the shared `:6878` endpoint, the datasource `timeInterval` that empties `rate()`
panels, and the prefix rules themselves — lives in the
[style guide](../../../docs/content/reference/internal/dashboard/style-guidelines.md), which is where to look rather
than here.

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
