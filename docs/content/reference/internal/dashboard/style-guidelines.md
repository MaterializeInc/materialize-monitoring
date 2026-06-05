---
title: "Style Guidelines"
weight: 20
---

# Dashboard Style Guidelines

Conventions for building visually consistent, operator-friendly dashboards. The audience for the dashboards themselves is **Materialize end users**: database-literate operators with basic graph-reading fluency but minimal cloud / Kubernetes / observability expertise. SQL is fair game; jargon like "differential dataflow's arrangement" needs a one-liner explanation when it appears.

## Layouts

Prefer **automatic layouts** over fixed positioning. Dashboard v2 provides more ergonomic options like Tabs and a formal automatic layout system.

- Prefer `AutoGridLayout` over fixed positioning.
- Use `.max_column_count(N)` to tune density. For panels with wide legend tables (multiple calc columns + long pod names), 2 columns per row is a good default; for compact stat panels, the default 3 or bumping to 5 (e.g. workload readiness) is fine.
- **Column-width sizing** (`AutoGridLayout.column_width_mode(...)`):
  - `"narrow"` — rows of mostly-stat panels alongside one or two donuts; keeps the donut from stealing all the horizontal space.
  - `"wide"` — rows of complex panels (timeseries with table legends, histograms, bar charts, tables). Lets each panel get enough room to be readable; on smaller monitors the row scrolls horizontally rather than cramming everything into a too-narrow column.
  - Default (`"standard"`) is fine for typical mixes.
- **Do not** wrap a small set of related panels in nested sub-rows when the auto-layout will tile them correctly — let `AutoGridLayout` handle the 2D wrap.

### Collapsed rows for type-specific drilldowns

When a row only applies to a subset of environments — e.g. Iceberg-sink metrics only matter when Iceberg sinks exist — declare the row collapsed by default with `.collapse(True)`:

```python
def build_iceberg_sinks_row(self):
    return (
        dashboardv2_builders.Row()
        .title("Iceberg Sinks")
        .hide_header(False)
        .collapse(True)  # collapsed; expand on demand
        .layout(...)
    )
```

Operators can expand the row when they need it; the row title acts as documentation that the section exists. This keeps the default page light without losing the type-specific content.

### Dashboard v1 compatibility (IGNORE THIS SECTION)

> Ignore this section until v1 support is desired.

We build dashboards as v2 by default and then provide best-effort compatibility with v1.

For Dashboard v1 compatibility, we use Collapsed rows as a replacement for v2 Tabs.

We do not provide direct positions, but instead calculate grid positions based on a 24-column grid system. The default height of rows is 9.

## Palettes

We offer a few colorblind-friendly palettes for use in dashboards. Grafana does not provide colorblind-friendly palettes by default.

- `packages/grafana-dashboards/dashboards/palette.py` — qualitative + sequential palettes
- `packages/grafana-dashboards/dashboards/threshold.py` — threshold-based color/text mappings

Read the comments in `dashboards.palette` and `dashboards.threshold` for intended usage.

## Tab-level theming

For non-health metrics (counts, totals, capacity, etc.) where there's no intrinsic good/bad coloring, pick a tab-level theme shade and use it across all stat-style panels in that tab. The convention is:

```python
# At module scope, near the top of each tab's file:
COMPUTE_THEME = palette.THEME_PALETTE[3]  # pick a distinct index per tab
```

Pass the shade through to `visualization.sparkline_stat(shade=…)` (see [Sparkline stats](#sparkline-stats)). This gives each tab a visually distinct background hue without re-deriving the choice in every panel.

`palette.THEME_PALETTE` is an alias of `BRIGHT_QUALITATIVE_NONSEQ` (7 entries). The index assigned per tab is the project's source of truth for visual identity — see the dashboard inventory in the `dashboards-as-code` skill for current assignments.

## Variables

Exposed variables should live inside of `dashboard.variables` and be explicitly registered in a given dashboard within the `configure_variables` method (or `configure_datasources` for datasource variables). Variables are global to all panels within the dashboard.

### Advanced controls

For variables which should generally be left on their defaults but may be modifiable for "power users", use the "Controls" section of the variable editor (in v2: `inControlsMenu`; in v1: VariableHide "3").

### Intermediates

Intermediate variables are variables that are computed from other variables and are hidden from the UI (in v2: `hideVariable`; in v1: VariableHide "2").

Constant Variables may contain "chained variables" and may use other variables as part of their definition. This pattern slightly contradicts the documentation which says Constant Variables are "static", but the pattern is useful for reusable snippets.

### Multi-select variables in regex contexts

For multi-select variables (`multi: true`) used in PromQL label matchers, prefer the explicit `:regex` interpolation format when the variable is embedded inside a wider regex string.

Grafana auto-detects the regex format only for the simple direct case `label=~"$var"`. When the variable appears inside a larger pattern, auto-detection does not fire, and bare `$var` resolves to literal `$__all` (or a `{val1,val2}` glob form) that doesn't behave as alternation.

```text
# Direct usage — auto-detected, plain `$var` is fine:
compute_cluster_id=~"$mzClusterList"

# Embedded usage — use `:regex` to get `(val1|val2|…)`:
pod=~".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"
```

This is the same guidance Grafana's own MCP tool surfaces in its dashboard-authoring hints.

## Panel visualization conventions

Shared panel-styling helpers live in `packages/grafana-dashboards/dashboards/visualization.py`. **Prefer importing from there over hand-rolling per-tab versions.** It currently exports:

- `NO_FILTER_MATCH` — standard "no value" string for panels driven by multi-select filters (e.g., "No matches for the current filters").
- `PIE_LEGEND_BUILDER` — pre-configured piechart legend (table layout, right placement, value column).
- `TS_LEGEND_BUILDER` — pre-configured timeseries legend (table layout, bottom placement, Max / Avg / Last calcs).
- `sparkline_stat(shade=…)` — factory returning a `stat.Visualization` with the area-mode sparkline pre-configured and (optionally) a fixed background shade.

### Sparkline stats

For "count" / "total" / "capacity" style metrics, prefer `visualization.sparkline_stat(...)` over a plain stat:

```python
from dashboards import palette, visualization

COMPUTE_THEME = palette.THEME_PALETTE[3]  # one shade per tab

.visualization(
    visualization.sparkline_stat(shade=COMPUTE_THEME)
    .unit("short")
    .min(0)  # anchor the sparkline Y-axis at zero for count-style metrics
)
```

Two non-obvious requirements:

- **Use a range query, not `.instant()`.** Sparklines need a series of points to render; if the query is instant, the panel will show the big number but the sparkline footer will be blank. Donuts / piecharts / single-value panels still want `.instant()` — the rule is "only instant queries when a single point is exactly what's being displayed."
- **`.min(0)` for counts.** Without it, Grafana auto-zooms the sparkline Y-axis to the data's actual range, which makes a count that drifts from 64 to 66 look like a huge swing. Anchor to zero so the magnitude is visible.

### Partitioned sparkline stats

When a sparkline-stat query produces multiple series (e.g. `sum by (session_type) (...)` returning `system` and `user` rows), the stat panel renders one tile per series. In that case set `text_mode=VALUE_AND_NAME` so each tile labels itself with its series name; otherwise you get a row of bare numbers with no indication of which is which.

```python
.visualization(
    visualization.sparkline_stat(shade=MY_THEME)
    .min(0)
    .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
)
```

For single-series sparklines, leave the default `VALUE` text mode — the panel title is the label.

### Timeseries legend

Apply the shared timeseries legend builder to every multi-series timeseries panel:

```python
.visualization(
    timeseries.Visualization()
    .unit("Bps")
    .no_value(CADVISOR_MISSING)
    .legend(visualization.TS_LEGEND_BUILDER)
)
```

Notes:

- **Placement BOTTOM** gives the table room for the per-series name + calc columns without truncation; RIGHT works for short legends only.
- **Avg -> `mean`, Last -> `lastNotNull`.** Plain `last` includes nulls and surprises users when the most recent scrape was missing.

### Donut / pie legend

```python
.visualization(
    piechart_builder.Visualization()
    .pie_type(piechart.PieChartType.DONUT)
    .legend(visualization.PIE_LEGEND_BUILDER)
    .display_labels([piechart.PieChartLabels.NAME, piechart.PieChartLabels.VALUE])
    .no_value(visualization.NO_FILTER_MATCH)
)
```

### "No data" messaging

Every panel that depends on an optional or filterable metric source should set `.no_value("…")` with a self-explanatory reason. Reach for the closest existing constant rather than inventing new wording:

- `visualization.NO_FILTER_MATCH` — multi-select filter excluded everything (cluster/replica/namespace selection).
- `CADVISOR_MISSING` / `KSM_MISSING` (defined in `k8s_resources.py`) — required scrape target is absent.

This way a blank panel tells the operator *why* it is blank.

### Color-mode default

For stat panels showing values that aren't intrinsically good/bad (counts, totals, capacity), use `color_mode=NONE` so the value renders in the default text color rather than green. `visualization.sparkline_stat` already does this. For health metrics, use `color_mode=BACKGROUND` plus an explicit thresholds/mappings palette (see `dashboards.threshold`).

## Writing panel descriptions

Grafana renders panel `.description(...)` text as a hover tooltip and a full info dialog (click the panel's title chevron). It supports **GitHub-flavored Markdown**. Descriptions are the operator's first-line documentation for "what am I looking at" — invest in them.

### Audience

Write for a **Materialize end user**: someone with database experience and basic familiarity reading graphs, but minimal cloud / Kubernetes / observability expertise. Assume SQL fluency. Explain Materialize-side concepts (peek, hydration, arrangement) when they appear. Don't restate the obvious ("Network bandwidth per pod" — they can read the title).

### Structure

Lead with a **bold headline sentence** that captures the panel's whole purpose. Grafana truncates the hover-tooltip preview, so the lead has to carry the punch line on its own. After that, free-form prose covering nominal/anomalous framing and where to look next:

```python
.description(
    "**One-sentence headline of what this panel shows.** "
    "Optional second sentence on why it exists. "
    "Nominal: <expected state>. <Anomaly signal>: <what it means>. "
    "If anomalous, check _Other Panel_ next."
)
```

The four questions every description should try to answer:

- **Why is this panel here?** (operator-facing reason to care)
- **What does nominal look like?** (anchor expectations)
- **What does anomalous look like?** (the signal)
- **What's the next step?** (cross-reference to another panel/tab)

### Markdown conventions

- **Bold** the first-sentence headline: `**Like this.**`
- *Italics* for cross-references between panels: `_Compute Objects -> Arrangements_`
- Backticks for identifiers and code: `` `mz_internal.mz_indexes` ``, `` `cluster_id` ``
- Use **ASCII `->`** in cross-references, **not** Unicode `→`. The arrow shows up in panel titles and descriptions; `→` triggers the ruff `RUF001`/`RUF002` "ambiguous character" lint rules.
- Em-dash `—` is OK inside description bodies and docstrings, but avoid it in *titles* (`RUF001` flags it in panel titles).

### Cross-references

Reference panels by their visible title, italicized, using `->` between tab and panel when crossing tabs:

```text
For per-pod CPU view see _Kubernetes Workloads -> Pod CPU Usage_.
Pair with _Sink Lag_ (in this tab) when investigating commit issues.
```

Bare prose references are easier to follow than HTML/anchored links in the current dashboard ergonomics. Don't include clickable URLs.

### SQL drilldowns

Where a panel surfaces a raw id (`source_id`, `collection_id`, `sink_id`), include the SQL to translate it to a user-friendly name:

```text
Translate `collection_id` to a name via
`SELECT id, name FROM mz_internal.mz_indexes` (or `mz_materialized_views`).
```

### Per-variant descriptions for shared helpers

When a single panel method is called multiple times with different parameters and each variant deserves its own description (e.g. Peek Latency at p50/p90/p99), define a module-level dict keyed by the variant label and have the helper look it up:

```python
_PEEK_LATENCY_DESCRIPTIONS: dict[str, str] = {
    "p50": "**Median read-query latency** — ...",
    "p90": "**90th-percentile read-query latency** — ...",
    "p99": "**Tail read-query latency** — ...",
}

def _peek_latency_panel(self, percentile: float, label: str):
    ...
    .description(_PEEK_LATENCY_DESCRIPTIONS[label])
```

### Shared-helper / mixin descriptions

When a helper function or mixin method registers the same panel on multiple tabs (e.g. `cpu_total_panel` from `KubeResourcesMixin` appears on both Summary and Kubernetes Workloads), the description is shared across all call sites. Either:

- Write a single description that's accurate in both contexts (and call out the differences inline: "On Summary the monitoring exporter is excluded; on K8s it's included.")
- Refactor the helper to accept a `description=` parameter and have each call site pass its own.

The shared-string approach is cheaper; refactor only when the descriptions truly need to diverge.

## PromQL conventions

### Rate intervals

Use `[$__rate_interval]` for `rate()` window selectors. Grafana derives this from the panel's resolution so the rate window adapts to zoom level. Use a literal range (`[5m]`, `[1h]`) only when the panel needs a specific window for semantic reasons — e.g. the "Current CPU Usage (5 min)" summary stat deliberately samples a 5-minute window regardless of zoom.

> **The datasource MUST declare the real scrape interval, or every `rate()` panel silently renders empty.** Grafana computes `$__rate_interval = max($__interval + scrapeInterval, 4 × scrapeInterval)`, where `scrapeInterval` is the datasource's configured "Scrape interval" (`jsonData.timeInterval`). **Left unset it defaults to 15s**, so `$__rate_interval` collapses to ~`1m`. If Prometheus actually scrapes every 60s, a 1-minute window contains a single sample and `rate()` returns nothing — the panel is blank even though the metric has data and traffic is flowing. Fix it at the datasource (one setting, fixes all panels), not per query:
>
> ```yaml
> # grafana datasource provisioning (helm/terraform)
> datasources:
>   - name: Prometheus
>     type: prometheus
>     jsonData:
>       timeInterval: "60s"   # MUST match Prometheus' real scrape_interval
> ```
>
> Keep `timeInterval` in sync with the actual `scrape_interval`. Diagnose a suspected mismatch with `count_over_time(<metric>[1m])` — if it returns `1`, the scrape interval is ≥60s and a `[1m]` rate window can't compute. The per-panel "Min interval" (`minStep`) is a local override of the same value, but the datasource setting is the correct global fix.

### Filtering cAdvisor metrics

The `$containerFilter` constant variable expands to `namespace=~"$mzNamespaceList",container!="",container!="POD"`. This excludes the pod-network-namespace sentinel and the empty-container series cAdvisor reports for pod-level metrics.

That means **don't use `$containerFilter` for `container_network_*` metrics** — those *are* the pod-level metrics it excludes. For network queries, scope only with `namespace=~"$mzNamespaceList"` (plus pod regex matchers as needed).

### Aggregation defaults

- For per-container metrics that you want to see per-pod (CPU, memory), group by `(namespace, pod, container)`.
- For network metrics, group by `(namespace, pod)` — this also drops the per-`interface` cardinality (most pods report at least `eth0` + `lo`).
- For environment-wide rollups, group only by `(namespace)` or `(container)` as appropriate.

### Series cardinality budgets

Prefer aggregating away `collection_id`, `replica_id`, and `worker_id` on environment-wide panels unless a breakdown is the panel's whole point. Large customer environments can have hundreds of collections multiplied by replicas multiplied by workers — keeping that cardinality has caused graphs to fail to load on production dashboards.

The dashboard default is **per-cluster aggregation**; specialists can drill down to specific collections via ad-hoc PromQL when needed. A working dashboard at less granularity is more valuable than a broken one with maximum detail.

Concretely:

- `sum by (instance_id)` rather than `sum by (instance_id, collection_id)`
- `max by (cluster, replica)` rather than per-worker series, *unless* the whole point of the panel is worker drift / skew detection (e.g. the Dataflows "per worker" panel is intentionally per-worker; the aggregate Dataflow Count panel is not).
- For "show me the worst offenders" panels, use `topk(N, …)` rather than letting every series through.

## Filtering by cluster / replica

Materialize cluster pods follow the naming convention `…-cluster-<cluster_id>-replica-<replica_id>-…`. To make the `mzClusterList` and `mzReplicaList` selectors filter cluster pods without hiding system pods (envd, balancer, etc.), use **two queries per panel** with module-level regex constants:

```python
CLUSTER_POD_RE = ".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"
NONCLUSTER_POD_RE = ".*-cluster-.*-replica-.*"

# Query 1 — cluster-replica pods, filtered by selection:
container_cpu_usage_seconds_total{$containerFilter, pod=~"<CLUSTER_POD_RE>"}

# Query 2 — everything else, always shown:
container_cpu_usage_seconds_total{$containerFilter, pod!~"<NONCLUSTER_POD_RE>"}
```

Putting the regex constants at module scope (next to `CADVISOR_MISSING` etc.) keeps them shareable across all panels in the file and prevents drift between numerator and denominator patterns.

## Deployment target: self-managed vs cloud

**The dashboards target self-managed Materialize.** This is the single most important fact for choosing metrics and labels, and it was a late-breaking correction — the original assumptions (below, and in earlier git history) were written against Materialize Cloud and are **wrong for self-managed**:

- **No `v2_mz_*` metrics.** The entire `v2_mz_*` family comes from the cloud-only promsql-exporter and is **absent** on self-managed. Always use the `mz_*` metric exported by environmentd/clusterd directly. (This reverses the old "prefer `v2_mz_` when both exist" guidance.)
- **No `materialize_cloud_organization_id`.** Environments are identified by **`materialize_cloud_organization_name`** (and the k8s namespace they run in, `materialize_cloud_organization_namespace` / `kubernetes_namespace`). The hex org id is cloud-only.
- **No `materialize_cloud_availability_zone`.** AZ/topology is a cloud concept; absent on self-managed.
- **No `cluster_environmentd_materialize_cloud_cluster_name` / `*_replica_name`.** The long-form *id* labels exist; their *name* companions do not — legend/group-by on the ids.

When verifying, query the live instance for what actually exists (`list_prometheus_metric_names`, `list_prometheus_label_names`) rather than trusting a remembered metric name.

## Materialize metric label families

Materialize `mz_*` metrics come from two scraper paths with **different label naming conventions**. Picking the wrong filter is a common failure mode.

**Short-form** (envd-side and most metrics):

- `instance_id` (this is the cluster id)
- `replica_id`
- `replica_full_name` (= `<cluster_name>.<replica_name>`, e.g. `quickstart.r1`) — on some metrics; the only place a friendly cluster name appears on the data-plane metrics.

Examples: `mz_dataflow_elapsed_seconds_total`, `mz_arrangement_record_count`, `mz_active_subscribes`, `mz_compute_controller_*`, `mz_query_total`, `mz_adapter_commands`. Note `mz_compute_peek_duration_seconds_*` has `instance_id` but **no `replica_id`** (envd-side, per-cluster only).

**Long-form** (some clusterd-scraped metrics):

- `cluster_environmentd_materialize_cloud_cluster_id`
- `cluster_environmentd_materialize_cloud_replica_id`
- `cluster_environmentd_materialize_cloud_replica_role`
- `cluster_environmentd_materialize_cloud_size` / `*_scale` / `*_workers`
- `worker_id`

Examples: `mz_arrangement_maintenance_seconds_total`, `mz_compute_replica_history_dataflow_count`, and (expected, unverified — no sources/sinks in the test env) `mz_source_*` / `mz_sink_*`. The `*_cluster_name` / `*_replica_name` companions are **absent on self-managed** — legend and group-by on the `*_cluster_id` / `*_replica_id` labels instead.

**Cluster/replica info metric:** `mz_compute_cluster_status` is the richest — it carries `compute_cluster_id`, `compute_cluster_name`, `compute_replica_id`, `compute_replica_name`, `size`, and `mz_version`. It backs the cluster picker variable and the Cluster Information table.

**Env-scoped counts with NO cluster labels:** `mz_tables_count`, `mz_views_count`, `mz_mzd_views_count` (materialized views), `mz_clusters_count`, `mz_cluster_reps_count`, `mz_active_subscribes`. These get the `ENV_SCOPED_NOTE` callout in descriptions. **No self-managed equivalent exists** for source/sink/index counts or source/sink status (the cloud-only `v2_mz_sources_count` / `v2_mz_sinks_count` / `v2_mz_indexes_count` / `v2_mz_source_status` / `v2_mz_production_object`); panels that need them are kept with a `TODO(self-managed)` and a `no_value`.

**Helper constants for filtering**:

- `_COMPUTE_FILTER` (in `storage_objects.py`) — long-form filter on env + cluster + replica (`materialize_cloud_organization_name` + `cluster_environmentd_materialize_cloud_cluster_id`/`_replica_id`).
- `_ARRANGEMENT_FILTER` (in `compute_objects.py`) — same shape, different module. Originally arrangement-specific, now reused for dataflows.
- These two constants are **the same PromQL fragment** in two places; lifting them to a shared module is a known cleanup candidate.

## Known metric quirks and gotchas

Things that have surprised us during development; worth knowing before touching the relevant panels.

- **`mz_` over `v2_mz_` — always, on self-managed.** The `v2_mz_*` family does not exist here (see [Deployment target](#deployment-target-self-managed-vs-cloud)). This reverses earlier guidance; treat any `v2_mz_*` reference in old code or notes as a bug.
- **"Peek" is the read-query latency metric.** No "query" in the name. `mz_compute_peek_duration_seconds_*` is the histogram for read-query latency on indexed data (the differential-dataflow operation behind `SELECT … FROM <view>`). It is envd-side: it carries `instance_id` but **no `replica_id`**, so peek latency is per-cluster, not per-replica.
- **`mz_storage_objects` is the source/sink catalog metric.** One series per (object, replica), value `1`, with labels `id`, `type` (`source`/`sink`), `object_type` / `connection_type` (postgres/kafka/…), `envelope_type`, `cluster_id`, `replica_id`. It **excludes** the hidden `<name>_progress` subsources, so it's the right metric for counts and type breakdowns: `count(group by (id) (mz_storage_objects{type="source"}))`. It carries **no name and no status** label.
- **Count metrics double-count progress subsources.** `mz_sources_count` / `mz_sinks_count` *do* exist on self-managed (once a source/sink is created), but they fold the hidden `<name>_progress` subsources into their per-`type` counts (3 Postgres sources → `type="postgres"`=6). Use `mz_storage_objects` for accurate counts. `mz_tables_count` / `mz_views_count` / `mz_mzd_views_count` / `mz_clusters_count` / `mz_cluster_reps_count` are fine as-is.
- **Catalog `*_count` metrics only exist once an object of that type does.** `mz_sources_count`, `mz_sinks_count`, and `mz_indexes_count` are absent from a fresh env and appear the moment you create the first source / sink / index — so a metric being missing doesn't mean "no self-managed equivalent," it can mean "none created yet." Confirmed equivalents: `mz_indexes_count` (carries the `relation_type` breakdown — table / view / materialized-view; sum over it then `max` to dedup pods), `mz_sources_count` / `mz_sinks_count` (carry `type`, but **double-count progress subsources** — prefer `mz_storage_objects` for counts, see above). `mz_tables_count` / `mz_views_count` / `mz_mzd_views_count` are stable.
- **No source/sink *status* metric.** The only `*_status` metrics are `mz_compute_cluster_status`, `mz_connection_status`, `mz_balancer_connection_status` (the cloud-only `v2_mz_source_status` has no equivalent). For running/stalled/errored, query `mz_internal.mz_source_statuses` / `mz_sink_statuses` in SQL. Metric-side health signals: `mz_source_offset_commit_failures`, `mz_sink_rdkafka_txerrs` / connects / disconnects.
- **Hydration is SQL-only.** No Prometheus metric exposes per-collection hydration state/time on self-managed: `v2_mz_compute_hydration_time_seconds` is cloud-only, and `mz_compute_controller_hydration_queue_size` is the controller's scheduling queue (drains fast — reads 0 even while 100+ objects are mid-hydration). Use `mz_internal.mz_hydration_statuses` (`WHERE NOT hydrated`) and `mz_internal.mz_compute_hydration_times` in SQL. The metric-side proxy is frontier lag (below).
- **`mz_dataflow_wallclock_lag_seconds` is the freshness signal** — how far each collection's output frontier trails real time. It's a summary with `quantile` `0` (min) / `1` (max) only — take `1` for worst-case. **It emits a u64::MAX sentinel (`~1.8e19`)** for collections with no established frontier (idle / mid-hydration / not yet producing); filter with `< 1e9` or it blows out the axis. Carries `collection_id` + `instance_id` + `replica_id`, but **also a redundant series without `instance_id`** — add `instance_id!=""` to dedup. Backs the Compute Objects -> Freshness row (the `< 1e9` filtered view = collections that *have* a frontier but trail real time). Collections with *no* frontier yet (mid-hydration / stuck) are the sentinel-valued ones filtered out here — they surface instead in the inverted `> 1e15` count (see next bullet).
- **An unreachable source upstream does NOT increment `mz_source_offset_commit_failures`.** That counter only fires when the upstream is reachable but *rejects* the commit. For a broker/DB that's simply unreachable (`BrokerTransportFailure`, severed security group, DNS), the source never reaches the commit step, so commit-failures stays flat at 0 even though the source is `stalled`. The detector that works: **`offset_committed > offset_known`**. Normally `offset_known >= offset_committed`; when the upstream is unreachable the source can't fetch metadata and `offset_known` collapses below `offset_committed`. Use `max by (source_id) (offset_committed) > bool max by (source_id) (offset_known)` for a per-source 0/1 "disconnected" flag (verified: stalled Kafka source -> 1, healthy Postgres sources -> 0). Sources have no transport-error *counter* the way sinks have `mz_sink_rdkafka_txerrs`, so this offset comparison is the closest metric-side "can't reach upstream" signal. It backs the second series of the Storage -> Sources -> Source Upstream Errors panel.
- **Per-replica failures hide inside `sum by (source_id)` aggregates.** Replicas of a multi-replica cluster ingest independently; if one is restarted and can't resume pulling (e.g. a stale Kafka connection), it silently reads 0 while its siblings keep going. The source still reports `Running`, `mz_source_offset_commit_failures` stays 0 (it isn't *failing* to commit, just not pulling), and an aggregate throughput panel looks fine because the healthy replicas carry the volume. The only metric-side tell is a **per-replica** breakdown — `sum by (parent_source_id, cluster_environmentd_materialize_cloud_replica_id) (rate(mz_source_messages_received ...))` — where the dead replica's line drops to 0 (same idea as the per-worker dataflow skew panel). Frontier lag climbs in parallel. Lesson: for ingest/replica health, keep at least one per-replica panel rather than only the per-source rollup.
- **The wallclock-lag sentinel count is a hydration-queue proxy** (and the closest thing to a hydration-state metric on self-managed). Inverting the freshness filter — `count(... mz_dataflow_wallclock_lag_seconds{quantile="1"} > 1e15)` with `instance_id!=""` — counts collections with no established frontier, i.e. still (re)building state. **It spikes briefly on every replica restart and drains back to 0 — that's normal (re)hydration, not breakage.** A count that stays elevated is the genuinely-broken case (a collection that never hydrates, e.g. a source whose `CREATE` didn't finish). It backs the **Currently Hydrating** stat (Summary + Compute -> Hydration) as a *neutral* sparkline — deliberately not alarm-colored, since brief spikes are expected; an earlier red "Stuck Objects" framing was dropped because alarm-on-any false-fired on routine restarts. Metrics carry only `collection_id`; resolve names / true status via `mz_internal.mz_hydration_statuses WHERE NOT hydrated`, `mz_source_statuses` / `mz_sink_statuses`, or the console Objects view.
- **`mz_source_bytes_received.source_id` is the *subsource* id**, not the primary. The primary lives in `parent_source_id`. Postgres sources fan out one bytes_received series per replicated table. Aggregate by `parent_source_id` to get per-primary rates. (No friendly-name join is available — `v2_mz_source_status` is cloud-only — so the legend is `parent_source_id`.)
- **Storage metrics confirm the long-form label family.** `mz_source_*` / `mz_sink_*` use `cluster_environmentd_materialize_cloud_cluster_id` / `_replica_id` (verified live) — so `_COMPUTE_FILTER` is correct. Caveat: the `$mzClusterList` picker is built from `mz_compute_cluster_status` (compute clusters only); a dedicated *ingest* cluster won't appear there, so selecting a specific cluster can hide storage objects. Default "All" shows everything.
- **`mz_sink_oustanding_progress_records` is misspelled** in Materialize itself ("oustanding" not "outstanding"). Don't "fix" the PromQL — match the metric name as-is.
- **`mz_compute_controller_subscribe_count` vs `mz_active_subscribes` trade-off**: the former has `instance_id` (cluster-filterable) but no `session_type`; the latter has `session_type` but no cluster labels. The summary tab uses `mz_active_subscribes` for the session_type donut, accepting the loss of cluster filtering.
- **`s2` is the `mz_catalog_server` cluster** and dominates many panels (commit rates, peek counts, arrangement maintenance, hydration). It's a system cluster and the noise floor is its business-as-usual. Mention this explicitly in panel descriptions where users might mistake it for an anomaly.
- **Duplicate `job` scrapes inflate `sum(rate(...))`.** Some deployments run several Prometheus scrape jobs against the same clusterd `:6878` endpoint with different keep-rules, so a metric can appear under N `job` values (observed: `kubernetes-pods`, `kubernetes-pods-mz-{usage,compute,storage}`). Confirmed multi-job: `mz_source_*`, `mz_sink_*`, `mz_arrangement_*`, `mz_compute_replica_history_*` — a plain `sum(rate(...))` over them reads **N×** the truth. Fix: wrap the inner counter/gauge in **`max without (job) (...)`** before the outer aggregation (no-op when there's one job). `max by (...)` panels and `histogram_quantile` are already job-invariant. **Do not exclude job names by pattern** — the authoritative name varies by deployment, and on at least one instance several metrics (`mz_compute_cluster_status`, `mz_storage_objects`, `mz_dataflow_elapsed_seconds_total`, the `*_count` metrics) live *only* on a "legacy" job, so an exclusion list blanks real panels. Pick the dedup label-set carefully: `max without (job)` keeps every other label; if a metric is also multi-scraped per `instance`, add `instance` to the `without` set.

## PromQL recipes

Reference for patterns we've established that aren't obvious in the language docs.

### Outer-join for label enrichment

When one metric has the value you want and another has the friendly name, you can't always inner-join (some entities may be missing from the name metric). Use a two-query outer-join:

```promql
# Named branch — series with a matching name available
(<value_query>
 * on (<key>) group_left (<name_label>)
 label_replace(<name_query>, "<key>", "$1", "<source_key>", "(.*)")) > 0

# Orphan branch — series without a name match
(<value_query>
 unless on (<key>)
 label_replace(<name_query>, "<key>", "$1", "<source_key>", "(.*)")) > 0
```

Each branch goes into its own `promql_query(...)` in the panel; their legends can differ (e.g., `{{source_name}}` for the named branch and `{{parent_source_id}}` for the orphan). This pattern was used by `_source_bytes_received_panel` to enrich `parent_source_id` with `source_name` from `v2_mz_source_status` — but that status metric is **cloud-only**, so on self-managed the panel keeps just the `parent_source_id` aggregate (no name join). The recipe is still the right shape whenever a self-managed name metric is available.

### Table pivot via `groupingToMatrix`

To turn one row per (entity, dimension) into one row per entity with columns per dimension value (e.g., Success / Errors columns from a `status` label):

```python
.transformation(... labelsToFields keepLabels=[entity, dimension])
.transformation(... merge)
.transformation(... groupingToMatrix
                rowField=entity columnField=dimension valueField=Value)
.transformation(... organize  renameByName={...})
.transformation(... sortBy    ...)
```

After `groupingToMatrix`, the row-identifier column comes out named `<rowField>\<columnField>` literally (one backslash). In Python source that's `"<rowField>\\<columnField>"` (Python escape for one backslash). Real example: `_adapter_commands_by_application_panel` in `connections_activity.py`.

The naive alternative — two queries joined by `joinByField` — produces one Value column **per input frame**, not per query, which is N×M columns instead of 2. We tried that and gave up.

### Histogram quantile aggregated by labels

Standard pattern, but worth pinning the shape because the `sum by` labels matter:

```promql
histogram_quantile(0.99,
  sum by (le, <preserved_labels...>) (
    rate(<metric>_bucket{<filter>}[$__rate_interval])
  )
)
```

Real examples: `_peek_latency_panel` (per `instance_id` — the metric has no `replica_id`), `_iceberg_commit_latency_panel` (aggregated env-wide).

### `or vector(0)` to keep panels non-empty

For stat panels where "no series" should render as `0` rather than "No data":

```promql
count(<series_query>) or vector(0)
```

Real example: `add_currently_hydrating_panel`.

### Per-cluster aggregation that handles label breakdowns

To get a single env-wide count from a metric that may carry breakdown labels (like a `type`/`size` split), without falling for the "max grabs the biggest bucket, not the total" trap:

```promql
max(sum by (instance) (<metric>{$environmentFilter})) or vector(0)
```

`sum by (instance)` collapses all label dimensions per scraper instance, then `max(...)` dedups across multiple exporter pods if there's more than one. Real example: `_env_total_count_query` in `storage_objects.py` (used for `mz_tables_count`; the source/sink count callers have no self-managed metric and read 0 via `or vector(0)`).

### Cluster + non-cluster pod split

For Kubernetes panels (CPU, memory, networking) where you want the cluster/replica selectors to scope cluster pods but not hide infra pods:

```promql
# Cluster pods (filtered)
<metric>{$containerFilter, pod=~".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"}

# Non-cluster pods (always shown)
<metric>{$containerFilter, pod!~".*-cluster-.*-replica-.*"}
```

Constants `CLUSTER_POD_RE` and `NONCLUSTER_POD_RE` in `k8s_resources.py` hold the regex strings — reuse them rather than re-typing.

## Shared module-level constants and helpers

For navigation when looking for a shared building block:

| Where | Name | What it is |
|---|---|---|
| `dashboards/visualization.py` | `NO_FILTER_MATCH` | "No matches for the current filters" string |
| `dashboards/visualization.py` | `PIE_LEGEND_BUILDER` | piechart legend (table, right placement, value column) |
| `dashboards/visualization.py` | `TS_LEGEND_BUILDER` | timeseries legend (table, bottom, Max/Avg/Last calcs) |
| `dashboards/visualization.py` | `sparkline_stat(shade=…)` | stat.Visualization factory with area sparkline |
| `dashboards/palette.py` | `THEME_PALETTE` (alias of `BRIGHT_QUALITATIVE_NONSEQ`) | tab-level theme colors, 7 entries |
| `dashboards/palette.py` | `INCANDESC_SEQUENTIAL`, `Binary`, `TriHealth`, `SUNSET_*` | health/threshold palettes |
| `dashboards/threshold.py` | `health_mapping`, `health_thresholds` | text + color mapping for healthy/degraded/unhealthy |
| `dashboards/threshold.py` | `time_stable_thresholds(seconds=…)` | gray-out for "long ago is fine" |
| `dashboards/threshold.py` | `error_thresholds(max_errors=…)` | gradient for error-count panels |
| `dashboards/threshold.py` | `load_thresholds(max_load=…)` | gradient for load gauges |
| `k8s_resources.py` | `CADVISOR_MISSING`, `KSM_MISSING` | no-value strings for cadvisor / kube-state-metrics gaps |
| `k8s_resources.py` | `CLUSTER_POD_RE`, `NONCLUSTER_POD_RE` | pod-name regex matchers for cluster filtering |
| `compute_objects.py` | `ARRANGEMENT_LABEL_*` constants + `_ARRANGEMENT_FILTER` | long-form cluster label names + filter snippet |
| `compute_objects.py` | `ENV_SCOPED_NOTE` | "Environment-scoped — not affected by…" boilerplate |
| `connections_activity.py` | `_PEEK_LATENCY_DESCRIPTIONS` | per-percentile (p50/p90/p99) description dict for the peek-latency panels |
| `storage_objects.py` | `_COMPUTE_FILTER` | long-form filter snippet (duplicate of `_ARRANGEMENT_FILTER`) |
| `storage_objects.py` | `ENV_SCOPED_NOTE` | **duplicate of the one in `compute_objects.py`** — consolidation candidate |
| `compute_objects.py` | `add_currently_hydrating_panel(dashboard, panel_id, shade=…)` | shared panel factory used by Summary's Environment Health row |
