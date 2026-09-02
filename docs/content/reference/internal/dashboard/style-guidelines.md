---
title: "Style Guidelines"
weight: 20
---

# Dashboard Style Guidelines

Conventions for building visually consistent, operator-friendly dashboards.
The audience for the dashboards themselves is **Materialize end users**: database-literate operators with basic
graph-reading fluency but minimal cloud / Kubernetes / observability expertise.
SQL is fair game; jargon like "differential dataflow's arrangement" needs a one-liner explanation when it appears.

## Layouts

Prefer **automatic layouts** over fixed positioning. Dashboard v2 provides more ergonomic options like Tabs and a formal automatic layout system.

- Prefer `AutoGrid` (an `AutoGridLayout`) over fixed positioning.
- Use `AutoGrid::new(N)` to tune density.
  For panels with wide legend tables (multiple calc columns + long pod names), 2 columns per row is a good default; for
  compact stat panels, the default 3 or bumping to 5 (e.g. workload readiness) is fine.
- **Column-width sizing** (`AutoGrid::column_width(...)`):
  - `"narrow"` — rows of mostly-stat panels alongside one or two donuts; keeps the donut from stealing all the horizontal space.
  - `"wide"` — rows of complex panels (timeseries with table legends, histograms, bar charts, tables).
    Lets each panel get enough room to be readable; on smaller monitors the row scrolls horizontally rather than
    cramming everything into a too-narrow column.
  - Default (`"standard"`) is fine for typical mixes.
- **Do not** wrap a small set of related panels in nested sub-rows when the auto-layout will tile them correctly — let the grid handle the 2D wrap.

### Collapsed rows for type-specific drilldowns

When a row only applies to a subset of environments — e.g. Iceberg-sink metrics only matter when Iceberg sinks exist —
declare the row collapsed by default with `.collapsed()`:

```rust
fn iceberg() -> Row {
    // Collapsed; expand on demand.
    Row::new("Iceberg Sinks")
        .collapsed()
        .grid(AutoGrid::new(3).panel("sinks-iceberg-commit-latency", iceberg_commit_latency(q)))
}
```

Operators can expand the row when they need it; the row title acts as documentation that the section exists.
This keeps the default page light without losing the type-specific content.

### Dashboard v1 compatibility (IGNORE THIS SECTION)

> Ignore this section until v1 support is desired.

We build dashboards as v2 by default and then provide best-effort compatibility with v1.

For Dashboard v1 compatibility, we use Collapsed rows as a replacement for v2 Tabs.

We do not provide direct positions, but instead calculate grid positions based on a 24-column grid system. The default height of rows is 9.

## Palettes

We offer a few colorblind-friendly palettes for use in dashboards. Grafana does not provide colorblind-friendly palettes by default.

- `packages/mzmon-lib/src/grafana/palette.rs` — qualitative + sequential palettes
- `packages/mzmon-lib/src/grafana/threshold.rs` — threshold ladders and value mappings

Read the module docs in both for intended usage.

## Tab-level theming

For non-health metrics (counts, totals, capacity, etc.) where there's no intrinsic good/bad coloring, pick a tab-level
theme shade and use it across all stat-style panels in that tab.
The convention is:

```rust
// At the top of each tab's module:
const SHADE: &str = theme::COMPUTE.shade;
```

Pass it to `Panel::shade(…)`. This gives each tab a visually distinct background hue without re-deriving the choice in
every panel.

Each dashboard owns a `theme.rs` naming its tabs and their shades, so the whole assignment is one file rather than a
constant per tab module — that is what makes "is this tab's colour distinct from its neighbours?" answerable by reading
one screen. `palette::THEME` (7 entries) is the pool it draws from. A Summary-style tab that points at other tabs
borrows their shades rather than having one of its own.

## Variables

Exposed variables should live inside of `dashboard.variables` and be explicitly registered in a given dashboard within
the `configure_variables` method (or `configure_datasources` for datasource variables).
Variables are global to all panels within the dashboard.

### Advanced controls

For variables which should generally be left on their defaults but may be modifiable for "power users", use the
"Controls" section of the variable editor (in v2: `inControlsMenu`; in v1: VariableHide "3").

### Intermediates

Intermediate variables are variables that are computed from other variables and are hidden from the UI (in v2:
`hideVariable`; in v1: VariableHide "2").
The hidden discovery variables still use this pattern — e.g. `mzNamespaceList`, which is derived from `environmentIdList`.

**Do not use Constant Variables for reusable PromQL filter snippets.**
We used to express shared label-matcher fragments (`$environmentFilter`, `$containerFilter`, `$clusterFilter`,
`$replicaFilter`) as hidden Constant Variables whose values chained other variables.
Grafana's constant-variable interpolation does not recursively resolve those nested `$…List` references and mangles the
embedded commas/quotes when the value is spliced into a label matcher, so the rendered query broke.
These are **inlined at render time** instead: the query registry's template parameters (`%%{mzEnvironmentFilter}`,
`%%{cAdvisorFilter}`, …) resolve to the label-matcher text before the query reaches the dashboard — see
[Render context]({{< relref "sdks.md" >}}#render-context).
The nested `$…List` references inside those fragments stay as real Grafana variables and resolve at view time; only the wrapper indirection is gone.

**Filter fragments are render-time parameters, not ConstantVariables:** the `$environmentFilter` / `$containerFilter` /
`$clusterFilter` / `$replicaFilter` hidden ConstantVariables were removed — Grafana's constant-variable interpolation
mangled their nested `$…List` refs and embedded commas.
The registry's `%%{mzEnvironmentFilter}` / `%%{cAdvisorFilter}` / `%%{mzClusterList}` / `%%{mzReplicaList}` parameters
resolve to the matcher text before the query reaches the dashboard; the nested `$…List` references inside them stay real
Grafana variables and resolve at view time.

### Multi-select variables in regex contexts

For multi-select variables (`multi: true`) used in PromQL label matchers, prefer the explicit `:regex` interpolation
format when the variable is embedded inside a wider regex string.

Grafana auto-detects the regex format only for the simple direct case `label=~"$var"`.
When the variable appears inside a larger pattern, auto-detection does not fire, and bare `$var` resolves to literal
`$__all` (or a `{val1,val2}` glob form) that doesn't behave as alternation.

```text
# Direct usage — auto-detected, plain `$var` is fine:
compute_cluster_id=~"$mzClusterList"

# Embedded usage — use `:regex` to get `(val1|val2|…)`:
pod=~".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"
```

This is the same guidance Grafana's own MCP tool surfaces in its dashboard-authoring hints.

## Panel visualization conventions

Panel presets live in `packages/mzmon-lib/src/grafana/panel.rs`. **Prefer them over hand-rolling per-tab versions.**
`Panel` is generic over its plugin options, and the constructor picks both the plugin and its defaults:

- `Panel::stat(title)` — area-mode sparkline pre-configured; `.shade(…)` sets the fixed background.
- `Panel::timeseries(title)` — table legend, bottom placement, Max / Avg / Last calcs.
- `Panel::piechart(title)` — donut with a table legend, right placement, value column.
- `Panel::table`, `Panel::gauge`, `Panel::barchart` — the remaining presets.
- `NoValue` — the standard empty-state strings, including `FilterMismatch` ("No matches for the current filters") and
  the collector-requirement variants; `NoValue::Custom` for anything panel-specific.

### Sparkline stats

For "count" / "total" / "capacity" style metrics, `Panel::stat` already carries the area-mode sparkline:

```rust
Panel::stat("Active Indexes")
    .query(q.get("materialize.compute.indexes.count").legend("indexes"))
    .shade(SHADE)
    .unit("short")
    // Anchor the sparkline Y-axis at zero for count-style metrics.
    .min(0.0)
    .build(0)
```

Two non-obvious requirements:

- **Use a range query, not an instant one.** Sparklines need a series of points to render; with an instant query the
  panel shows the big number and a blank sparkline footer. This is a property of the *query*, so it lives on the
  registry entry (`instant: true`) rather than the panel — check it there when a sparkline comes up empty. Donuts,
  piecharts and single-value panels do want `instant`; the rule is "instant only when a single point is exactly what is
  being displayed."
- **`.min(0)` for counts.** Without it, Grafana auto-zooms the sparkline Y-axis to the data's actual range, which makes
  a count that drifts from 64 to 66 look like a huge swing.
  Anchor to zero so the magnitude is visible.

### Partitioned sparkline stats

When a sparkline-stat query produces multiple series (e.g. `sum by (session_type) (...)` returning `system` and `user`
rows), the stat panel renders one tile per series.
In that case set `text_mode=VALUE_AND_NAME` so each tile labels itself with its series name; otherwise you get a row of
bare numbers with no indication of which is which.

```rust
Panel::stat(title)
    .shade(SHADE)
    .min(0.0)
    .text_mode(stat::BigValueTextMode::ValueAndName)
```

For single-series sparklines, leave the default `VALUE` text mode — the panel title is the label.

### Timeseries legend

Apply the shared timeseries legend builder to every multi-series timeseries panel:

```rust
Panel::timeseries("Sink Throughput (committed)")
    .query(q.get("materialize.storage.sinks.throughput").legend("{{name}}"))
    .unit("Bps")
    .no_value(NoValue::RequiresCAdvisor)
```

Notes:

- **Placement BOTTOM** gives the table room for the per-series name + calc columns without truncation; RIGHT works for short legends only.
- **Avg -> `mean`, Last -> `lastNotNull`.** Plain `last` includes nulls and surprises users when the most recent scrape was missing.

### Donut / pie legend

```rust
Panel::piechart("Index Relationship Types")
    .query(q.get("materialize.compute.indexes.by_type").legend("{{relation_type}}"))
    .no_value(NoValue::FilterMismatch)
```

`Panel::piechart` is a donut with the shared legend and name+value labels already set; `.full_pie()` opts out of the
donut hole.

### "No data" messaging

Every panel that depends on an optional or filterable metric source should set `.no_value(...)` with a
self-explanatory reason. Reach for the closest existing `NoValue` variant rather than inventing new wording:

- `NoValue::FilterMismatch` — a multi-select filter excluded everything (cluster / replica / namespace selection).
- `NoValue::RequiresCAdvisor`, `RequiresKubeStateMetrics`, `RequiresCAdvisorAndKubeStateMetrics` — a required scrape
  target is absent, named specifically so the operator knows which one to go install.
- `NoValue::Custom(...)` — anything panel-specific, where empty is not an error ("Hydration Queue is empty").

This way a blank panel tells the operator *why* it is blank.

### Color-mode default

For stat panels showing values that aren't intrinsically good/bad (counts, totals, capacity), leave the colour mode
alone — `Panel::stat` defaults to `None`, so the value renders in the default text colour rather than green. For health
metrics, `.color_background()` plus an explicit threshold ladder or value mapping (see
`mzmon_lib::grafana::threshold`).

## Writing panel descriptions

Grafana renders a panel's description as a hover tooltip and a full info dialog (click the panel's title chevron).
It supports **GitHub-flavored Markdown**. Descriptions are the operator's first-line documentation for "what am I
looking at" — invest in them.

> [!IMPORTANT]
> Descriptions are **not written in the dashboard**. They live on the registry query the panel names, and reach the
> panel through the bridge — see [SDKs and Schemas]({{< relref "sdks.md" >}}#panels-do-not-write-promql).
> Everything below is guidance for writing the registry's `description:` block; a test asserts no panel carries prose
> of its own.

### Audience

Write for a **Materialize end user**: someone with database experience and basic familiarity reading graphs, but minimal
cloud / Kubernetes / observability expertise.
Assume SQL fluency.
Explain Materialize-side concepts (peek, hydration, arrangement) when they appear.
Don't restate the obvious ("Network bandwidth per pod" — they can read the title).

### Structure

The registry's `description` is structured, and `format_description` renders it to the shape below: `summary` in bold
first, then `nominal` / `degraded` / `unhealthy` as labelled paragraphs, then `notes` unlabelled.

Lead `summary` with a sentence that captures the panel's whole purpose. Grafana truncates the hover-tooltip preview, so
it has to carry the punch line on its own.

```yaml
description:
  summary: |
    One sentence on what this shows, and why it exists.
  nominal: |
    What the expected state looks like.
  degraded: |
    The signal, and what it means.
  notes: |
    Caveats, and where to look next: check _Other Tab -> Other Panel_.
```

The fields map onto the four questions below: `summary` answers the first, `nominal` the second, `degraded` /
`unhealthy` the third, and `notes` the fourth. Omit what does not apply — a capacity panel has no `unhealthy` state.

The four questions every description should try to answer:

- **Why is this panel here?** (operator-facing reason to care)
- **What does nominal look like?** (anchor expectations)
- **What does anomalous look like?** (the signal)
- **What's the next step?** (cross-reference to another panel/tab)

### Markdown conventions

- **Bold** the first-sentence headline: `**Like this.**`
- *Italics* for cross-references between panels: `_Compute Objects -> Arrangements_`
- Backticks for identifiers and code: `` `mz_internal.mz_indexes` ``, `` `cluster_id` ``
- Use **ASCII `->`** in cross-references, **not** Unicode `→`. The cross-reference checker in the parity suite
  validates the `->` form against the dashboard's actual tab and row titles, so a Unicode arrow silently escapes that
  check — which is how the baseline shipped six references to tabs that did not exist.
- Em-dash `—` is fine inside description bodies; avoid it in *titles*, where it reads as punctuation noise at panel
  size.

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

When one panel builder is called several times with different parameters and each variant deserves its own prose (Peek
Latency at p50 / p90 / p99), give each variant **its own registry query** and select between them in the builder:

```rust
impl Quantile {
    fn query_id(self) -> &'static str {
        match self {
            Quantile::P50 => "materialize.connections.peek_latency.p50",
            Quantile::P90 => "materialize.connections.peek_latency.p90",
            Quantile::P99 => "materialize.connections.peek_latency.p99",
        }
    }
}
```

Three ids rather than one parameterized query, because each carries its own explanation of what that quantile means —
which is the part a reader needs. The alternative, one query plus a lookup table of prose in the dashboard, puts the
explanation somewhere the query author will not see it.

## PromQL conventions

### Rate intervals

Use `[$__rate_interval]` for `rate()` window selectors.
Grafana derives this from the panel's resolution so the rate window adapts to zoom level.
Use a literal range (`[5m]`, `[1h]`) only when the panel needs a specific window for semantic reasons — e.g. the
"Current CPU Usage (5 min)" summary stat deliberately samples a 5-minute window regardless of zoom.

> **The datasource MUST declare the real scrape interval, or every `rate()` panel silently renders empty.** Grafana
  computes `$__rate_interval = max($__interval + scrapeInterval, 4 × scrapeInterval)`, where `scrapeInterval` is the
  datasource's configured "Scrape interval" (`jsonData.timeInterval`).
  **Left unset it defaults to 15s**, so `$__rate_interval` collapses to ~`1m`.
  If Prometheus actually scrapes every 60s, a 1-minute window contains a single sample and `rate()` returns nothing —
  the panel is blank even though the metric has data and traffic is flowing.
  Fix it at the datasource (one setting, fixes all panels), not per query:
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
> Keep `timeInterval` in sync with the actual `scrape_interval`.
  Diagnose a suspected mismatch with `count_over_time(<metric>[1m])` — if it returns `1`, the scrape interval is ≥60s
  and a `[1m]` rate window can't compute.
  The per-panel "Min interval" (`minStep`) is a local override of the same value, but the datasource setting is the
  correct global fix.

### Filtering cAdvisor metrics

The `%%{cAdvisorFilter}` parameter expands to `namespace=~"$mzNamespaceList",container!="",container!="POD"`. This
excludes the pod-network-namespace sentinel and the empty-container series cAdvisor reports for pod-level metrics.

That means **don't use `CONTAINER_FILTER` for `container_network_*` metrics** — those *are* the pod-level metrics it
excludes.
For network queries, scope only with `namespace=~"$mzNamespaceList"` (plus pod regex matchers as needed).

### Aggregation defaults

- For per-container metrics that you want to see per-pod (CPU, memory), group by `(namespace, pod, container)`.
- For network metrics, group by `(namespace, pod)` — this also drops the per-`interface` cardinality (most pods report at least `eth0` + `lo`).
- For environment-wide rollups, group only by `(namespace)` or `(container)` as appropriate.

### Series cardinality budgets

Prefer aggregating away `collection_id`, `replica_id`, and `worker_id` on environment-wide panels unless a breakdown
is the panel's whole point.
Large customer environments can have hundreds of collections multiplied by replicas multiplied by workers — keeping that
cardinality has caused graphs to fail to load on production dashboards.

The dashboard default is **per-cluster aggregation**; specialists can drill down to specific collections via ad-hoc
PromQL when needed.
A working dashboard at less granularity is more valuable than a broken one with maximum detail.

Concretely:

- `sum by (instance_id)` rather than `sum by (instance_id, collection_id)`
- `max by (cluster, replica)` rather than per-worker series, *unless* the whole point of the panel is worker drift /
  skew detection (e.g. the Dataflows "per worker" panel is intentionally per-worker; the aggregate Dataflow Count panel
  is not).
- For "show me the worst offenders" panels, use `topk(N, …)` rather than letting every series through.

## Filtering by cluster / replica

Materialize cluster pods follow the naming convention `…-cluster-<cluster_id>-replica-<replica_id>-…`.
To make the `mzClusterList` and `mzReplicaList` selectors filter cluster pods without hiding system pods (envd,
balancer, etc.), use **two expressions on one query** — a list-valued `promQL` in the registry:

```yaml
promQL:
  # Cluster-replica pods, filtered by the selection.
  - |
    container_cpu_usage_seconds_total{%%{cAdvisorFilter}, pod=~".*-cluster-%%{mzClusterListRegex}-replica-%%{mzReplicaListRegex}-.*"}
  # Everything else, always shown.
  - |
    container_cpu_usage_seconds_total{%%{cAdvisorFilter}, pod!~".*-cluster-.*-replica-.*"}
```

The two matchers are the same pattern under `=~` and `!~`, so the split is exhaustive and disjoint: no pod is missed
and none is counted twice. The second is deliberately *not* narrowed by the cluster selectors — environmentd and the
balancer belong to no cluster and should not vanish when you focus on one.

Note `mzClusterListRegex`, not `mzClusterList`: the variable is a *fragment* of a larger regex here, so it needs the
`:regex` format modifier the escaped parameter supplies. See [Multi-select variables in regex
contexts](#multi-select-variables-in-regex-contexts).

Keeping both expressions on one registry query rather than two queries is what prevents drift between them.

## Deployment target: self-managed vs cloud

**The dashboards target self-managed Materialize.** This is the single most important fact for choosing metrics and
labels, and it was a late-breaking correction — the original assumptions (below, and in earlier git history) were
written against Materialize Cloud and are **wrong for self-managed**:

- **No `v2_mz_*` metrics.** The entire `v2_mz_*` family comes from the cloud-only promsql-exporter and is **absent** on
  self-managed.
  Always use the `mz_*` metric exported by environmentd/clusterd directly.
  (This reverses the old "prefer `v2_mz_` when both exist" guidance.)
- **No `materialize_cloud_organization_id`.** Environments are identified by **`materialize_cloud_organization_name`**
  (and the k8s namespace they run in, `materialize_cloud_organization_namespace` / `kubernetes_namespace`).
  The hex org id is cloud-only.
- **No `materialize_cloud_availability_zone`.** AZ/topology is a cloud concept; absent on self-managed.
- **No `cluster_environmentd_materialize_cloud_cluster_name` / `*_replica_name`.** The long-form *id* labels exist;
  their *name* companions do not — legend/group-by on the ids.

When verifying, query the live instance for what actually exists (`list_prometheus_metric_names`,
`list_prometheus_label_names`) rather than trusting a remembered metric name.

### Converging cloud and self-managed: the SQL metric prefix

A subset of metrics is **SQL-derived** and differs between environments only by a name prefix: `mz_X` on self-managed
(environmentd `/metrics/mz_*` endpoints) vs `v2_mz_X` in cloud (`new-promsql-exporter`).
To write one query that works in both, prefix those metric names with `%%{mzSqlPrefix}`.

**The prefix is baked in at render time**, not resolved by Grafana at view time.
This replaced an earlier `$sqlMetricPrefix` Grafana query variable that auto-detected the prefix by inspecting which
`…compute_cluster_status` series existed — **Google Managed Prometheus (GMP) cannot run that `query_result(...)` +
regex auto-detection**, so the prefix has to be decided at render time and emitted as a literal metric name.

```yaml
promQL: |
  %%{mzSqlPrefix}compute_cluster_status{%%{mzEnvironmentFilter}}
# renders -> mz_compute_cluster_status{materialize_cloud_organization_name=~"$environmentNameList"}
```

The prefix comes from `--sql-metric-prefix` (default `mz_`), reaching the render through
`DashboardScope::for_prefix`. Nothing is captured at import, so one process can emit both variants — a test builds
`env-top` under both prefixes. No `v2_mz_` artifact ships today; the capability is there when one is wanted.

**Only prefix SQL-derived metrics.** **Genuine instrumentation** (timely/differential counters scraped from
environmentd/clusterd `/metrics`) carries the **same bare `mz_` name in both environments** — prefixing it produces
`v2_mz_…` which doesn't exist in cloud and breaks the panel.

- **Prefix (SQL-derived):** `compute_cluster_status`, the catalog `*_count` metrics
  (`tables`/`views`/`mzd_views`/`indexes`/`sources`/`sinks`/`clusters`/`cluster_reps`/`connections`/`secrets`/`catalog_items`),
  `storage_objects`, `object_id`, `workload_clusters`, the arrangement-introspection family
  (`arrangement_record_count`/`_size_bytes`/…), `dataflow_elapsed_seconds_total`,
  `compute_replica_park_duration_seconds_total`, `compute_hydration_time_seconds`.
- **Do NOT prefix (genuine):** `arrangement_maintenance_seconds_total`, `compute_replica_history_dataflow_count`,
  `compute_peek_duration_seconds_*`, `source_*` /`sink_*` throughput/lag/error metrics, `query_total`,
  `adapter_commands`, `active_sessions` /`active_subscribes`, `compute_controller_hydration_queue_size`,
  `dataflow_wallclock_lag_seconds`.

Quick test: a metric is genuine (don't prefix) if it appears under the plain-`mz_` name on the cloud `materialize` job;
SQL-derived (prefix) if cloud only has it as `v2_mz_`.

Conventions:
- Interpolate `{variables.SQL_METRIC_PREFIX}` into the query f-string (a metric prose name in a panel *description*
  stays the literal self-managed name — don't substitute there).
- Leave a one-line reference comment with the concrete names, e.g. `# mz_tables_count / v2_mz_tables_count`, so the resolved names stay greppable.
- In table transforms, the value-field name is the **resolved** metric, so `excludeByName` must list **both** `mz_X` and `v2_mz_X`.
- This is a convergence shim: once cloud's `new-promsql-exporter` is replaced by native `mz_` instrumentation, the
  prefix collapses to `mz_` everywhere and the config knob retires.

<!--
Open question (raised June 2026): the prefix is currently a module-import-time
constant, so emitting both mz_ and v2_mz_ variants needs separate processes.
A BuildContext threaded through panel construction (carrying sql_metric_prefix
and any other per-variant knobs) would let one run produce multiple variants and
make the dependency explicit rather than reading module/global state. Not yet
decided; revisit when the v2_mz_ variant is actually built.
-->

### Rendering and verifying generation-time substitutions

When a change only rewrites how queries are *generated* (inlining a filter, baking the prefix) but should not change the
*rendered* PromQL, verify it mechanically.
Render the dashboard before and after, apply the expected textual expansion to the baseline (e.g. `${sqlMetricPrefix}` →
`mz_`, `$environmentFilter` → `materialize_cloud_organization_name=~"$environmentIdList"`), and assert the query
bodies are byte-identical and only the intended template variables were removed.
This catches f-string brace-escaping mistakes that lint and type-checks miss.

## Materialize metric label families

Materialize `mz_*` metrics come from two scraper paths with **different label naming conventions**. Picking the wrong filter is a common failure mode.

**Short-form** (envd-side and most metrics):

- `instance_id` (this is the cluster id)
- `replica_id`
- `replica_full_name` (= `<cluster_name>.<replica_name>`, e.g. `quickstart.r1`) — on some metrics; the only place a
  friendly cluster name appears on the data-plane metrics.

Examples: `mz_dataflow_elapsed_seconds_total`, `mz_arrangement_record_count`, `mz_active_subscribes`,
`mz_compute_controller_*`, `mz_query_total`, `mz_adapter_commands`.
Note `mz_compute_peek_duration_seconds_*` has `instance_id` but **no `replica_id` ** (envd-side, per-cluster only).

**Long-form** (some clusterd-scraped metrics):

- `cluster_environmentd_materialize_cloud_cluster_id`
- `cluster_environmentd_materialize_cloud_replica_id`
- `cluster_environmentd_materialize_cloud_replica_role`
- `cluster_environmentd_materialize_cloud_size` / `*_scale` / `*_workers`
- `worker_id`

Examples: `mz_arrangement_maintenance_seconds_total`, `mz_compute_replica_history_dataflow_count`, and (expected,
unverified — no sources/sinks in the test env) `mz_source_*` / `mz_sink_*`.
The `*_cluster_name` / `*_replica_name` companions are **absent on self-managed** — legend and group-by on the
`*_cluster_id` / `*_replica_id` labels instead.

**Cluster/replica info metric:** `mz_compute_cluster_status` is the richest — it carries `compute_cluster_id`,
`compute_cluster_name`, `compute_replica_id`, `compute_replica_name`, `size`, and `mz_version`.
It backs the cluster picker variable and the Cluster Information table.

**Env-scoped counts with NO cluster labels:** `mz_tables_count`, `mz_views_count`, `mz_mzd_views_count` (materialized
views), `mz_clusters_count`, `mz_cluster_reps_count`, `mz_active_subscribes`.
These note their environment scope in the registry query's description.
**No self-managed equivalent exists** for source/sink/index counts or source/sink status (the cloud-only
`v2_mz_sources_count` / `v2_mz_sinks_count` / `v2_mz_indexes_count` / `v2_mz_source_status` / `v2_mz_production_object`
); panels that need them are kept with a `NoValue` explaining the gap.

**Filtering on the long-form labels.** Metrics carrying
`cluster_environmentd_materialize_cloud_cluster_id` / `_replica_id` (storage and dataflow families) are filtered with
`%%{mzClusterList}` / `%%{mzReplicaList}` against those label names — the parameter supplies the *value*, and the query
author writes the label, because the label name differs across the three cluster-id families. See [Materialize metric
label families](#materialize-metric-label-families).

The Python carried this fragment as two duplicate module constants (`_COMPUTE_FILTER` and `_ARRANGEMENT_FILTER`); with
the value parameterized there is one spelling per query and nothing to keep in sync.

## Known metric quirks and gotchas

Things that have surprised us during development; worth knowing before touching the relevant panels.

- **`mz_` over `v2_mz_` — always, on self-managed.** The `v2_mz_*` family does not exist here (see
  [Deployment target](#deployment-target-self-managed-vs-cloud)).
  This reverses earlier guidance; treat any `v2_mz_*` reference in old code or notes as a bug.
- **"Peek" is the read-query latency metric.** No "query" in the name.
  `mz_compute_peek_duration_seconds_*` is the histogram for read-query latency on indexed data (the
  differential-dataflow operation behind `SELECT … FROM <view>`).
  It is envd-side: it carries `instance_id` but **no `replica_id` **, so peek latency is per-cluster, not per-replica.
- **`mz_storage_objects` is the source/sink catalog metric.** One series per (object, replica), value `1`, with labels
  `id`, `type` (`source`/`sink`), `object_type` / `connection_type` (postgres/kafka/…), `envelope_type`, `cluster_id`
  , `replica_id`.
  It **excludes** the hidden `<name>_progress` subsources, so it's the right metric for counts and type breakdowns:
  `count(group by (id) (mz_storage_objects{type="source"}))`.
  It carries **no name and no status** label.
- **Count metrics double-count progress subsources.** `mz_sources_count` / `mz_sinks_count` *do* exist on self-managed
  (once a source/sink is created), but they fold the hidden `<name>_progress` subsources into their per-`type` counts (3
  Postgres sources → `type="postgres"` =6).
  Use `mz_storage_objects` for accurate counts.
  `mz_tables_count` / `mz_views_count` / `mz_mzd_views_count` / `mz_clusters_count` / `mz_cluster_reps_count` are fine
  as-is.
- **Catalog `*_count` metrics only exist once an object of that type does.** `mz_sources_count`, `mz_sinks_count`, and
  `mz_indexes_count` are absent from a fresh env and appear the moment you create the first source / sink / index — so a
  metric being missing doesn't mean "no self-managed equivalent," it can mean "none created yet." Confirmed equivalents:
  `mz_indexes_count` (carries the `relation_type` breakdown — table / view / materialized-view; sum over it then `max`
  to dedup pods), `mz_sources_count` / `mz_sinks_count` (carry `type`, but **double-count progress subsources** —
  prefer `mz_storage_objects` for counts, see above).
  `mz_tables_count` / `mz_views_count` / `mz_mzd_views_count` are stable.
- **No source/sink *status* metric.** The only `*_status` metrics are `mz_compute_cluster_status`,
  `mz_connection_status`, `mz_balancer_connection_status` (the cloud-only `v2_mz_source_status` has no equivalent).
  For running/stalled/errored, query `mz_internal.mz_source_statuses` / `mz_sink_statuses` in SQL.
  Metric-side health signals: `mz_source_offset_commit_failures`, `mz_sink_rdkafka_txerrs` / connects / disconnects.
- **Hydration is SQL-only.** No Prometheus metric exposes per-collection hydration state/time on self-managed:
  `v2_mz_compute_hydration_time_seconds` is cloud-only, and `mz_compute_controller_hydration_queue_size` is the
  controller's scheduling queue (drains fast — reads 0 even while 100+ objects are mid-hydration).
  Use `mz_internal.mz_hydration_statuses` (`WHERE NOT hydrated`) and `mz_internal.mz_compute_hydration_times` in SQL.
  The metric-side proxy is frontier lag (below).
- **`mz_dataflow_wallclock_lag_seconds` is the freshness signal** — how far each collection's output frontier trails
  real time.
  It's a summary with `quantile` `0` (min) / `1` (max) only — take `1` for worst-case.
  **It emits a u64::MAX sentinel (`~1.8e19`)** for collections with no established frontier (idle / mid-hydration / not
  yet producing); filter with `< 1e9` or it blows out the axis.
  Carries `collection_id` + `instance_id` + `replica_id`, but **also a redundant series without `instance_id` ** — add
  `instance_id!=""` to dedup.
  Backs the Compute Objects -> Freshness row (the `< 1e9` filtered view = collections that *have* a frontier but trail
  real time).
  Collections with *no* frontier yet (mid-hydration / stuck) are the sentinel-valued ones filtered out here — they
  surface instead in the inverted `> 1e15` count (see next bullet).
- **An unreachable source upstream does NOT increment `mz_source_offset_commit_failures`.** That counter only fires
  when the upstream is reachable but *rejects* the commit.
  For a broker/DB that's simply unreachable (`BrokerTransportFailure`, severed security group, DNS), the source never
  reaches the commit step, so commit-failures stays flat at 0 even though the source is `stalled`.
  The detector that works: **`offset_committed > offset_known`**.
  Normally `offset_known >= offset_committed`; when the upstream is unreachable the source can't fetch metadata and
  `offset_known` collapses below `offset_committed`.
  Use `max by (source_id) (offset_committed) > bool max by (source_id) (offset_known)` for a per-source 0/1
  "disconnected" flag (verified: stalled Kafka source -> 1, healthy Postgres sources -> 0).
  Sources have no transport-error *counter* the way sinks have `mz_sink_rdkafka_txerrs`, so this offset comparison is
  the closest metric-side "can't reach upstream" signal. It backs the second series of the Storage -> Sources -> Source
  Upstream Errors panel.
- **Per-replica failures hide inside `sum by (source_id)` aggregates.** Replicas of a multi-replica cluster ingest
  independently; if one is restarted and can't resume pulling (e.g. a stale Kafka connection), it silently reads 0 while
  its siblings keep going.
  The source still reports `Running`, `mz_source_offset_commit_failures` stays 0 (it isn't *failing* to commit, just
  not pulling), and an aggregate throughput panel looks fine because the healthy replicas carry the volume.
  The only metric-side tell is a **per-replica** breakdown —
  `sum by (parent_source_id, cluster_environmentd_materialize_cloud_replica_id) (rate(mz_source_messages_received ...))`
  — where the dead replica's line drops to 0 (same idea as the per-worker dataflow skew panel).
  Frontier lag climbs in parallel.
  Lesson: for ingest/replica health, keep at least one per-replica panel rather than only the per-source rollup.
- **The wallclock-lag sentinel count is a hydration-queue proxy** (and the closest thing to a hydration-state metric on
  self-managed).
  Inverting the freshness filter — `count(... mz_dataflow_wallclock_lag_seconds{quantile="1"} > 1e15)` with
  `instance_id!=""` — counts collections with no established frontier, i.e. still (re)building state.
  **It spikes briefly on every replica restart and drains back to 0 — that's normal (re)hydration, not breakage.** A
  count that stays elevated is the genuinely-broken case (a collection that never hydrates, e.g. a source whose `CREATE`
  didn't finish).
  It backs the **Currently Hydrating** stat (Summary + Compute -> Hydration) as a *neutral* sparkline — deliberately not
  alarm-colored, since brief spikes are expected; an earlier red "Stuck Objects" framing was dropped because
  alarm-on-any false-fired on routine restarts.
  Metrics carry only `collection_id`; resolve names / true status via
  `mz_internal.mz_hydration_statuses WHERE NOT hydrated`, `mz_source_statuses` / `mz_sink_statuses`, or the console
  Objects view.
- **`mz_source_bytes_received.source_id` is the *subsource* id**, not the primary.
  The primary lives in `parent_source_id`.
  Postgres sources fan out one bytes_received series per replicated table.
  Aggregate by `parent_source_id` to get per-primary rates.
  (No friendly-name join is available — `v2_mz_source_status` is cloud-only — so the legend is `parent_source_id`.)
- **Storage metrics confirm the long-form label family.** `mz_source_*` / `mz_sink_*` use
  `cluster_environmentd_materialize_cloud_cluster_id` / `_replica_id` (verified live) — so `_COMPUTE_FILTER` is correct.
  Caveat: the `$mzClusterList` picker is built from `mz_compute_cluster_status` (compute clusters only); a dedicated
  *ingest* cluster won't appear there, so selecting a specific cluster can hide storage objects.
  Default "All" shows everything.
- **`mz_sink_oustanding_progress_records` is misspelled** in Materialize itself ("oustanding" not "outstanding").
  Don't "fix" the PromQL — match the metric name as-is.
- **`mz_compute_controller_subscribe_count` vs `mz_active_subscribes` trade-off**: the former has `instance_id`
  (cluster-filterable) but no `session_type`; the latter has `session_type` but no cluster labels.
  The summary tab uses `mz_active_subscribes` for the session_type donut, accepting the loss of cluster filtering.
- **`s2` is the `mz_catalog_server` cluster** and dominates many panels (commit rates, peek counts, arrangement
  maintenance, hydration).
  It's a system cluster and the noise floor is its business-as-usual. Mention this explicitly in panel descriptions
  where users might mistake it for an anomaly.
- **Duplicate `job` scrapes inflate `sum(rate(...))`.** Some deployments run several Prometheus scrape jobs against the
  same clusterd `:6878` endpoint with different keep-rules, so a metric can appear under N `job` values (observed:
  `kubernetes-pods`, `kubernetes-pods-mz-{usage,compute,storage}`).
  Confirmed multi-job: `mz_source_*`, `mz_sink_*`, `mz_arrangement_*`, `mz_compute_replica_history_*` — a plain
  `sum(rate(...))` over them reads **N×** the truth.
  Fix: wrap the inner counter/gauge in **`max without (job) (...)`** before the outer aggregation (no-op when there's
  one job).
  `max by (...)` panels and `histogram_quantile` are already job-invariant.
  **Do not exclude job names by pattern** — the authoritative name varies by deployment, and on at least one instance
  several metrics (`mz_compute_cluster_status`, `mz_storage_objects`, `mz_dataflow_elapsed_seconds_total`, the
  `*_count` metrics) live *only* on a "legacy" job, so an exclusion list blanks real panels.
  Pick the dedup label-set carefully: `max without (job)` keeps every other label; if a metric is also multi-scraped per
  `instance`, add `instance` to the `without` set.

## Logs dashboard conventions

**Loki end to end, and that is the point.** `env-logs` defines *no* metrics datasource and shares nothing with
`environment_scoped` — its namespace, app and level pickers are Loki-discovered. Reading logs is frequently how you work
out why the *metrics* pipeline is broken, so a logs dashboard deriving its scope from Prometheus would go blind exactly
when it is needed. A test asserts no query references `$mzNamespaceList`, `$mzClusterList` or `$environmentNameList`.

**Loki answers a variable differently from Prometheus.** Not `label_values(...)` text but a `{label, stream, type: 1}`
object — `logql_variable_query` builds it, and `LogQueryVariable` is the Loki-side counterpart to `QueryVariable`. A
Prometheus-shaped variable query against Loki resolves to nothing, silently. `stream` may reference other variables,
which is what chains namespace → app/level.

**Materialize-first, not Materialize-only.** `MATERIALIZE_NAMESPACE_PATTERN`
(`.*materialize.*|mz-.*|environment-.*`) is three conventions rather than one, because the naming differs by install:
this repo's charts (`materialize`, `materialize-environment`), the shorter `mz-` prefix, and Cloud's
`environment-<uuid>-0`.
It is a **default selection** rather than a filter on discovery: `env-logs` discovers every namespace and merely opens
on the Materialize ones.
The monitoring stack's own logs are what you need when telemetry itself is failing, and the narrow value is a naming
convention rather than a derived fact — so being wrong about it has to be one selection to recover from, not a blank
dashboard.
An earlier design did gate *discovery* behind a switch; it was removed precisely because a wrong pattern then took the
pickers down with it rather than merely pointing them somewhere unhelpful.

**Every log picker states its own `all_value`; none is left to expand into the discovered values.** An expansion is
*empty* whenever discovery has not run or has failed, and `label=~""` matches only the streams **missing** that label
rather than all of them — so one picker failing to load takes the panels down with it, and it reads as "selects no log
lines" rather than as the error it is.

| Picker | `all_value` | Why |
|---|---|---|
| `logNamespaceList` | `.+` | It is the sole matcher of the app / level / job discovery selectors, so it must not be empty-compatible. Safe because every line carries a namespace — the pipeline coerces cluster-scoped events to `kube-system` rather than omitting the label. |
| `logAppList` | `.*` | `app` is genuinely absent from some streams and `.+` drops them — 2,407 of 30,432 lines in half an hour on a representative install, most of `kube-system`. |
| `logLevelList` | `.*` | Same inclusive form; costs nothing and does not depend on every line carrying a level. |
| `logJobList` | `.+` | The second anchor. Free, since `job` is present on every line, so `.+` and `.*` select identically. |

The constraint bites hardest on anything that **is** the whole stream selector of a discovery query: its permissive
value has to be `.+`, not `.*`, or the variable itself fails to load and every picker chained below it empties out.
This is why namespace discovery is never narrowed by another control.

Watch the shape of the check — `.*materialize.*` *starts* with `.*` but cannot match empty, because it requires a
literal. A pattern is empty-compatible only when stripping every `.*` leaves nothing.

**Every log selector needs a non-empty-compatible matcher.** LogQL rejects one where every matcher can match the empty
string — *"queries require at least one regexp or equality matcher that does not have an empty-compatible value"* — and
a dashboard built from `=~` pickers is exactly that shape. `$logJobList` is the anchor: its `all_value` is `.+` rather
than the discovered values, so it always contributes something non-empty and every panel parses whatever the other
pickers are set to. It doubles as the most direct way to isolate one workload, since `job` is `<namespace>/<container>`.
Verified against a live Loki, including the worst case where every other picker expands to nothing.

The *event* queries need no anchor and must not get this one: they pin `job="loki.source.kubernetes_events"`, already a
non-empty equality matcher, and a second `job` matcher would AND with it and zero the panel the moment a container job
was picked. Tests hold both halves.

**An optional selector fragment must render a no-op matcher, not nothing.**
PromQL tolerates a trailing comma inside `{}`, which is why `%%{excludeEnvironmentFilter}` can render empty and simply
vanish from a metric selector.
LogQL does not — `{namespace=~".+", job=~".+", }` is a parse error — so the same trick blanks every panel that uses it.
`%%{mzLogExcludeNamespaceFilter}` therefore always renders a full matcher, and turns itself off by matching nothing:
`namespace!~"a^"` excludes no namespace, while `namespace!~"$excludeMaterialize"` on `infra-logs` excludes the
deployment.
`a^` rather than `""` because `!~""` would read as *"has a namespace"* and would quietly drop any line missing the
label; `a^` is a pattern no value can match, which is what is actually meant.

**Exclusion needs a negative matcher, not a clever regex.** RE2 has no negative lookahead, so a set of namespaces
cannot be subtracted from inside a `=~` pattern — the exclusion has to be its own `!~` matcher, ANDed alongside the
picker's `=~`.
`infra-logs` carries both: `$logNamespaceList` selects, `$excludeMaterialize` subtracts.
The switch's enabled value is the same pattern `env-logs` *opens on*, which is deliberate — one dashboard selects the
deployment and the other subtracts it, and both agree on what "the deployment" means.
Both positions verified against a live Loki.

**The search box must be harmless when empty.** It renders as `|~ "(?i)$logSearch"`, and an empty pattern matches every
line rather than none — verified against a live Loki, since the opposite would blank the dashboard until something is
typed.

**Warning panels ignore the level picker**, deliberately. They answer "is anything wrong", and a selection of `INFO`
silently zeroing them would make them lie. A test holds that.

**Stream labels vs structured metadata.** `namespace`, `app`, `level`, `container`, `job`, `k8s_*`, `service_name` and
`unit` are stream labels and belong in the selector. `pod`, `node`, `organization_name`, `container_id`, `region`,
`zone`, `detected_level` and friends are structured metadata, filtered after a `|`. `organization_name` is the
self-managed stand-in for the cloud dashboards' Snowflake org lookup, which does not exist here.

**Two event scopes, two query families.** `materialize.events.deployment.*` / `.operator.*` are rollout-scoped and
belong to `env-upgrade`; `materialize.events.cluster.*` is the general browser and belongs to `env-logs`. Separate
definitions on purpose — the rollout queries carry generation and reporting-controller filters that a general browser
must not inherit, or it would quietly drop events for belonging to the wrong side of a rollout.

## Time-range guards on expensive rows

**New precedent, first used on the two logs dashboards' Volume rows.** `grafana/volume_guard.rs` owns it.

Counting log *lines* means reading every one of them — Loki indexes labels, not counts — so a `rate()` panel over a
wide selection decompresses the whole span. Measured on a live cluster, one such panel scans 0.7 GB over six hours,
1.8 GB over a day, 27 GB over a week, and **95 GB over a month, taking 45 seconds**. The log *feeds* beside them are
unaffected at any range: they stop at the first page of matches.

So a volume row carries `Row::only_within(volume_guard::THRESHOLD)` (`7d`), and is **always paired** with
`volume_guard::hidden_row(…)`, which carries the complementary `Row::only_beyond` and a `text` panel explaining the
absence. `only_within` and `only_beyond` are exact complements at the same threshold, so precisely one of the pair is
on screen at any range — a gap would leave the reader staring at nothing, an overlap would draw the expensive panels
*and* a note saying they are hidden.

Two things worth keeping if this pattern spreads:

- **Guard the row, not the panel.** The explanation belongs beside the thing it replaces, and a row is the smallest
  unit that can carry both.
- **The note's job is the remedy, not the announcement.** "Hidden" alone leaves the reader stuck; it has to say how to
  get the panels back (shorten the range, narrow the pickers) and where to go instead. A test asserts that.

`text` was added to `bin/gen-grafana-models.sh` for this — it is the only plugin here that shows no data.

## Kubernetes events in Loki

What the `env-upgrade` Events tab is built on, and the parts that are not guessable.

**Where they come from.** `loki.source.kubernetes_events` in `packages/alloy-pipelines/gateway.yaml` reads events off
the Kubernetes API and forwards them to the main processor, which lifts `reason`, `name`, `kind`, `count`, `node` and
`reportingcontroller` into **structured metadata** and maps the event `type` onto the `level` stream label
(`Normal` → `INFO`, `Warning` → `WARN`). Stream labels are therefore `job="loki.source.kubernetes_events"`, `namespace`
and `level`; everything else a query groups on is structured metadata, which LogQL matches and aggregates the same way.

**An event's namespace is the involved object's, not the reporter's.** This is the one that bites. orchestratord runs
in the operator namespace and reconciles resources in the environments' namespace, so *every event it publishes is
filed in the environment namespace*. Scoping the operator's events by `%%{mzOperatorNamespaceFilter}` returns nothing
— it looks right, renders empty, and gives no hint why. The operator queries scope to both namespaces
(`%%{mzDeploymentNamespaceFilter}`) and pick orchestratord out by
`| reportingcontroller="orchestratord.materialize.cloud"`, which is the reporter's identity and the only field that
actually says where an event came from.

**`line_format` is what makes a feed readable.** A raw event line is logfmt carrying a dozen fields, most of them
resource versions and forwarding addresses. `| line_format "{{.reason}} {{.kind}}/{{.name}} — {{.msg}}"` renders the
three that matter; expanding a line still shows the rest.

**The operator's event vocabulary** (see `src/orchestratord/src/reconcile.rs` and `controller/materialize.rs` in the
Materialize repo): `ReconciliationFailed` from the generic reconciliation wrapper, carrying the error's whole cause
chain; and the lifecycle transitions on the `Materialize` resource — `Applying`, `ReadyToPromote`,
`WaitingForApproval`, `Promoting`, `Applied`, `RolloutTimeout`, `FailedDeploy`. A `FailedDeploy` reports twice, once
with the phase and once with the cause; the reasons tell them apart. Repeats aggregate into one event with a rising
`count` rather than one line each, so a feed under-reports a tight loop — the `count` on the line is how many it
stands for.

**Two namespace controls, scoped differently.** `$operatorNamespace` is a visible single-select discovered from
`label_values(orchestratord_is_leader, namespace)` — the operator is a cluster-wide singleton that no environment
selection narrows. The environment namespace stays the hidden, environment-derived `$mzNamespaceList` that `env-top`
already uses. `%%{mzDeploymentNamespaceFilter}` is the two as **one** matcher; writing both filters side by side
repeats the `namespace` label in one selector, which is an AND and matches nothing.

## A table of current facts is an instant query

A table describing what *is* — pods on a node, their requests and limits, a
version per generation — evaluated over a range repeats every row once per scrape
step, and reads as a table with hundreds of near-identical rows rather than as a
list of facts.
Set `instant: true` on the registry query.

The opposite mistake exists too, and `env-upgrade`'s version table documents it: a
plain instant query evaluates at *now*, where a torn-down deployment generation no
longer exists, so a finished rollout looks like it never happened.
Where the answer must span the picker's window rather than the present moment, the
query keeps a range but collapses it — `max_over_time(...[$__range])` — and the
panel title says so.

Ask which of the two a table is before choosing: *what is true now* takes
`instant`, *what was true anywhere in this window* takes the collapse.

## Several queries in one table need `table_format`

A Table panel fed by more than one query renders **one column of values**, not one
per query, as long as the datasource returns time-series frames — Prometheus's
default.
The rows stack instead of joining, and no transformation fixes it, because the
frames never carried the label columns to join on.

**A dropdown at the foot of a table is the tell.** Prometheus returns one frame
per series, and a Table panel handed several frames renders a frame *picker*
rather than a table. It is easy to miss, because the first frame renders
correctly — the panel looks right and is showing you one series of many.
This bites single-query panels too, whenever the query returns a series per pod,
per taint, per anything.

Two ways out, and either is fine: ask for table format, or consolidate with a
transformation — `merge` joins frames on their shared fields, and `reduce` in
`seriesToRows` mode collapses each to a row. `infra-nodes` asserts that every one
of its tables does one of them.

`PanelQuery::table_format` asks for a table frame instead: label columns beside a
`Value` column, which `merge` then joins into one row per label set with a column
per query.
That is what lets a pod's request sit beside its own limit on `infra-nodes`.

Grafana names those value columns after the query's refId — `Value #query-0`
upward, assigned positionally — so an organize transform is what gives them
readable headers.
Mixed units in one table cannot be a panel default; give each column its own
`unit` override.

### `noValue` fills empty *cells*, not just empty panels

Grafana applies `noValue` per field, so on a table whose columns are legitimately
sparse it lands in every gap rather than standing in for a panel with no data.
On `infra-nodes` that put "kube-state-metrics is required" into the majority of
the limit cells — a collection-failure message for pods that simply set no limit,
and long enough to overflow the column.

Leave it unset on any table that joins several queries.
Blank is the honest rendering of "this row has none", and a genuinely empty panel
still falls back to Grafana's own "No data".
Single-query tables are unaffected: their columns come from one result, so a row
exists in full or not at all, and `noValue` only fires when the whole panel is
empty — which is exactly what it is for.

## Shade single-series panels, never graphs with several lines

`Panel::shade` sets Grafana's `shades` colour mode, which derives every series in
the panel from one hue.
On a stat, a gauge, or a graph drawing one line that is a deliberate identity — it
is how a Summary cell borrows the colour of the tab it points at, and how an info
row reads as one block.
On a graph drawing five CPU modes, one line per core, or one per device, it is
actively harmful: the lines come out as near-identical tints of the tab colour and
cannot be told apart, which is the whole job of a multi-series graph.

Leave those unshaded and let Grafana's classic palette assign contrasting colours.
`infra-nodes` asserts this in `multi_series_panels_are_not_shaded`, scoped to
timeseries panels: a stat whose query carries a templated legend still reduces to
one number and has no lines to confuse.

## Pin the ceiling on a bounded fraction whose nominal is zero

A panel measuring something that should sit at zero — link saturation, PSI
pressure, disk utilization — autoscales to its own noise when left alone, so a
perfectly healthy node renders a dramatic-looking graph whose axis tops out at
0.05%.
The reader cannot tell that from a real problem without reading the axis every
time.

Give any bounded fraction an explicit `.min(0.0)` **and** `.max(1.0)` so the
panel is drawn against the range that matters and a flat-healthy line stays flat.
Unbounded rates (errors and drops per second) keep autoscaling, since there is no
honest ceiling to pin them to and a spike is the thing worth seeing.

## Node identifiers across three families

The dashboard's one real trick. kube-state-metrics calls a node `node="<name>"`; node-exporter calls the same machine
`instance="<ip>:9100"` and carries the name only as `nodename` on `node_uname_info`.
`$node` is the visible picker over the Kubernetes name; `$nodeList` is **hidden** and resolves it to the address
through that metric.
Keeping the inherited `nodeList` name is what lets all 220 `instance=~"$nodeList"` occurrences in `node-health.yaml`
and `node-debug.yaml` back this dashboard unchanged — at the cost of a name that says "list" while holding one address.
Loki knows the node a third way again, as **structured metadata** on journal lines, so the journal filters in the
pipeline (`| node=...`) rather than in the selector; node *events* match on the involved object's name with
`kind="Node"`.

**The node families are vetted.** `node-health` and `node-debug` were authored before any dashboard used them and were
long flagged as unreviewed. All 87 of their expressions were run against a live cluster while this was built and all 87
returned data, as did all 103 rendered Prometheus queries and all 5 Loki ones.

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

## PromQL recipes

Reference for patterns we've established that aren't obvious in the language docs.

### Outer-join for label enrichment

When one metric has the value you want and another has the friendly name, you can't always inner-join (some entities may
be missing from the name metric).
Use a two-query outer-join:

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

Each branch goes into its own `promql_query(...)` in the panel; their legends can differ (e.g., `{{source_name}}` for
the named branch and `{{parent_source_id}}` for the orphan).
This pattern was used by `_source_bytes_received_panel` to enrich `parent_source_id` with `source_name` from
`v2_mz_source_status` — but that status metric is **cloud-only**, so on self-managed the panel keeps just the
`parent_source_id` aggregate (no name join).
The recipe is still the right shape whenever a self-managed name metric is available.

### Table pivot via `groupingToMatrix`

To turn one row per (entity, dimension) into one row per entity with columns per dimension value (e.g., Success / Errors
columns from a `status` label):

```rust
.transformations(vec![
    transform::labels_to_fields(&[entity, dimension]),
    transform::merge(),
    // `emptyValue: zero` matters: an entity with no rows for a dimension value has
    // no series at all, and a blank cell reads as "unknown" rather than "none".
    transform::grouping_to_matrix(entity, dimension, "Value", "zero"),
    transform::organize_renamed(&[ROW_COLUMN, ...], &[(ROW_COLUMN, "Application"), ...]),
    transform::sort_by("Errors", true),
])
```

After `groupingToMatrix`, the row-identifier column comes out named `<rowField>\<columnField>` literally (one
backslash). In Rust that is a raw string, `r"<rowField>\<columnField>"`. Real example: `commands_by_application` in
`connections.rs`.

The naive alternative — two queries joined by `joinByField` — produces one Value column **per input frame**, not per
query, which is N×M columns instead of 2.
We tried that and gave up.

### Histogram quantile aggregated by labels

Standard pattern, but worth pinning the shape because the `sum by` labels matter:

```promql
histogram_quantile(0.99,
  sum by (le, <preserved_labels...>) (
    rate(<metric>_bucket{<filter>}[$__rate_interval])
  )
)
```

Real examples: `materialize.connections.peek_latency.p99` (per `instance_id` — the metric has no `replica_id`),
`materialize.storage.sinks.iceberg.commit_latency` (aggregated env-wide).

Write the quantile with both decimals (`0.50`, not `0.5`); the registry is consistent about it and the parity suite
records the one place the baseline was not.

### `or vector(0)` to keep panels non-empty

For stat panels where "no series" should render as `0` rather than "No data":

In the registry this is the `orZero` template function rather than hand-written, so the parenthesization is uniform:

```yaml
promQL:
  template: |
    count(<series_query>)
  functions:
    - name: orZero
```

Real example: `materialize.compute.hydration.currently_hydrating`.

### Per-cluster aggregation that handles label breakdowns

To get a single env-wide count from a metric that may carry breakdown labels (like a `type` /`size` split), without
falling for the "max grabs the biggest bucket, not the total" trap:

```yaml
promQL:
  template: |
    max(sum by (instance) (<metric>{%%{mzEnvironmentFilter}}))
  functions:
    - name: orZero
```

`sum by (instance)` collapses all label dimensions per scraper instance, then `max(...)` dedups across multiple
exporter pods if there is more than one. Real example: `materialize.storage.tables.count`; the source/sink count
queries have no self-managed metric and read 0 through `orZero`.

### Cluster + non-cluster pod split

See [Filtering by cluster / replica](#filtering-by-cluster--replica) — the two-expression form and the
`mzClusterListRegex` requirement are described there.

## Shared constants and helpers

For navigation when looking for a shared building block. `mzmon-lib` holds what any dashboard can use; a dashboard's own
modules hold what only it needs.

| Where | Name | What it is |
|---|---|---|
| `mzmon-lib` `grafana/panel.rs` | `Panel::{stat,timeseries,piechart,table,gauge,barchart}` | the presets, each with its plugin's defaults |
| `mzmon-lib` `grafana/panel.rs` | `NoValue` | the standard empty-state strings |
| `mzmon-lib` `grafana/palette.rs` | `THEME` (7 entries) | the tab-theme pool |
| `mzmon-lib` `grafana/palette.rs` | `INCANDESCENT`, `SUNSET_*`, `tri_health`, `binary` | health / threshold palettes |
| `mzmon-lib` `grafana/threshold.rs` | `health`, `health_mapping` | text + colour for healthy / degraded / unhealthy |
| `mzmon-lib` `grafana/threshold.rs` | `stability`, `stability_days` | "long ago is fine" ladders, either polarity |
| `mzmon-lib` `grafana/threshold.rs` | `errors`, `load`, `utilization` | gradients for error-count, load and utilization panels |
| `mzmon-lib` `grafana/layout.rs` | `Layout`, `Tab`, `Row`, `AutoGrid` | the layout tree and panel-id assignment |
| `mzmon-lib` `grafana/queries.rs`¹ | `Queries::{get,legended}` | the registry handle every panel goes through |
| dashboard `theme.rs` | one entry per tab | that dashboard's colour assignment, in one place |
| dashboard `selector.rs` | the selector fragments | PromQL fragments the tab modules share |
| dashboard `transform.rs` | `labels_to_fields`, `merge`, `organize*`, `sort_by`, `grouping_to_matrix`, `extract_fields*` | Grafana transformation builders |
| dashboard `field_override.rs` | `by_name(...)` | per-column field overrides |
| dashboard `mod.rs` | `currently_hydrating(q, shade)` | the panel two tabs share |

¹ `queries.rs` is in `packages/dashboards/src/grafana/`, not `mzmon-lib` — it binds a registry to the Grafana render
context, which is a dashboard-crate concern.

The duplicated filter snippets and description constants the Python carried are gone: the selector fragments live in
one `selector.rs` per dashboard, and the prose lives on the registry query.
