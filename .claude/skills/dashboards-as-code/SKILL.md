---
name: dashboards-as-code
description: |
  This skill should be used when building dashboards (Grafana or DataDog).
  Sources live under `packages/grafana-dashboards/`.
---

# Dashboards as Code

Instead of more common ClickOps strategies (manually configuring dashboards in the UI),
we manage dashboards as reproducible source code.

This document is both a **conventions guide** (how to do things) and a
**state snapshot** (what currently exists). Skim "Current Dashboard
State" near the bottom first if you're picking up where someone else
left off.

## Intended Audience

The dashboards target **Materialize end users**: database-literate
operators with basic graph-reading fluency but minimal cloud /
Kubernetes / observability expertise. SQL is fair game; jargon like
"differential dataflow's arrangement" needs a one-liner explanation.
Panel descriptions, titles, and even cluster names should respect that
baseline.

## Targets

We support the following targets:

* Grafana 13 (Dashboard v2 schema) - latest as of April 2026
* Grafana 12 (Dashboard v2beta1 schema)

The following targets are planned but stubs are acceptable for now:

* **UNSUPPORTED** DataDog
* **BEST-EFFORT** Grafana 11 (Dashboard v1 schema)

## Grafana Schemas and Internals

The Grafana ecosystem has been undergoing major transitions in how they manage
dashboard configurations circa 2025; some web results may result in inconsistent
documentation.

The state of affairs that we care about:

* grafonnet jsonnet library was the way to do things through Grafana 11
* grafana-foundation-sdk was introduced for Grafana 12, but is supported back to Grafana 10
* <https://github.com/grafana/grafana-foundation-sdk/> is the repository for grafana-foundation-sdk
* grafana-foundation-sdk is built on top of grafana's [cog codegen framework](https://github.com/grafana/cog/), using cue-based or openapi schemas
* grafana-foundation-sdk is not super mature as of May 2026, but it's usable and ergonomic
* grafana-foundation-sdk documentation and versioning is very messy; always double check work

### Dashboard v1 Schema

Dashboards v1 schema was the schema used in Grafana 10 and 11 (earlier did not have a particular schema).
Grafana 12 supported Dashboard v1 schema by default, but also had an experimental option to use v2beta1 schema.

Dashboard v1 schemas are automatically migrated to Dashboard v2 in later versions of Grafana.

A copy of the Dashboard v1 openapi schema (generated from cog 61ff0a6055fa48f0c7b105fe4a37af637191314f, April 9, 2026)
is in [./references/dashboard.openapi.json](./references/dashboard.openapi.json).

### Dashboard v2 Schema

Grafana 12 previewed the Dashboard v2 schema (as v2beta1), but it was not the default.
Grafana 13 supports the Dashboard v2 schema by default.

Since Grafana 13 (as of April 2026) is the latest and recommended version of Grafana,
we prefer the Dashboard v2 schema for dashboards.

Dashboard v2 cannot be automatically downgraded to Dashboard v1 inside of Grafana;
but we do try to provide compatibility to generate close v1 dashboards as second-class / best-effort.

A copy of the Dashboard v2beta1 openapi schema (generated from cog 61ff0a6055fa48f0c7b105fe4a37af637191314f, April 9, 2026)
is in [./references/dashboardv2beta1.openapi.json](./references/dashboardv2beta1.openapi.json).

A copy of the Dashboard v2 openapi schema (generated from cog 61ff0a6055fa48f0c7b105fe4a37af637191314f, April 9, 2026)
is in [./references/dashboardv2.openapi.json](./references/dashboardv2.openapi.json).

## py-mzmon-lib and Grafana Foundation SDK

For the initial Python implementations of dashboards,
we use grafana-foundation-sdk for the majority of the code.
We use py-mzmon-lib for some shared utilities, best practices,
and fixes.

py-mzmon-lib lives in `packages/py-mzmon-lib` and is included
as a uv workspace.

When deciding to use a particular SDK building block,
be sure to check the available classes/functions and their documentation in py-mzmon-lib.

Do note that as of May 2026, grafana-foundation-sdk has not
yet merged its v2 schema so some tweaks may be necessary to
get things working with the latest Grafana.

## Determinism in Dashboards

We should try to maximize deterministic and idempotent behavior of dashboards.
It is acceptable for a dashboard to be "upgraded" upon import into Grafana,
but we want target a minimal diff if possible.

### UID Selection and Behavior

UIDs should be selected consistently based on the name of the dashboards.
UIDs are not required to be random, but must be unique.

Upgraded graphs should continue using the same UIDs unless they break workflows.

Even though we have different grafana targets, we should not encode the grafana
version in the UID (since they could be upgraded across versions).

UIDs must follow the [strict uid format introduced in 11.2](https://grafana.com/whats-new/2025-05-05-enforcing-stricter-data-source-uid-format/),
latin alphanumeric with dashes and underscores, 40 characters max.
We use the mz-mon- prefix for all UIDs.

**Dashboard v2 caveat:** in v2 the UID is *not* part of the dashboard spec —
it lives in the surrounding Kubernetes-style `metadata.name` on the
`dashboard.grafana.app/v2` resource. The `MzDashboard.UID` value (with the
`mz-mon-` prefix) is what we want as the canonical resource name, but Grafana
will happily auto-generate a UID at first upload if one isn't supplied.
Once a dashboard exists, its UID becomes immutable; the way to "fix" a
mismatched UID is to delete the existing dashboard and re-upload.

### Element Key Stability

In a v2 dashboard, panels are referenced by string keys in
`spec.elements{}` and in `spec.layout.…ElementReference.name`. The Python
source uses human-readable keys (e.g. `"pod-cpu-percent"`); Grafana may
rewrite them to `"panel-<id>"` form on some save paths and leave them alone
on others. Both forms are valid and the round-trip is non-destructive — do
not rely on a specific naming convention when reading dashboards back.

## Code Structure

Dashboards live in their respective packages within `packages/`.

The current python implementation of dashboards live in
`packages/grafana-dashboards` as a `uv` workspace.

Python helpers live in `packages/py-mzmon-lib`.

Within `packages/grafana-dashboards`, the top packages represent
the family of concerns (e.g., `mz_environment` or `infra`).
Within the family, dashboards have their own sub-package (such as `overview`) with the main dashboard entrypoint suffixed with `_dashboard.py` inside of that sub-package.
The full path to a given dashboard will look like:
`packages/grafana-dashboards/dashboards/<family>/<dashboard_name>/<dashboard_name>_dashboard.py`.
The main dashboard class will derive from the `py_mzmon_lib.dashboard.MzDashboard` base class.

Other modules alongside a dashboard will generally be the
tabs (if there are multiple tabs) or particularly intricate rows.

It is acceptable to share panels, rows, or even tabs between dashboards,
but prefer to have the code live within the most appropriate package with others importing it directly.

### Dashboard Variables

Exposed variables should live inside of `dashboard.variables`
and be explicitly registered in a given dashboard within the
`configure_variables` method (or `configure_datasources` in the case
of datasource variables).
These variables are global to all panels within the dashboard.

### Code Quality

We use the following tools to maintain code quality for python:
* `ruff` for linting and formatting (we use very aggressive rules to ensure high quality code)
* `pyright` for type checking
* `pytest` for testing
  * Unit tests are recommended to be placed next to their code with the `_test.py` suffix.

For Python linter configuration, familiarize yourself with `pyproject.toml`
in the root of this repository.

## Layouts

We recommend not using fixed positioning layouts as much as possible and instead
recommend automatic layouts.
Dashboard v2 provides more ergonomic options like Tabs and a formal automatic layout system.

### Dashboard v1 Compatibility (IGNORE THIS SECTION)

NOTICE: Please ignore this section until v1 support is desired.

We build our Dashboards as v2 by default and then provide best-effort compatibility with v1.

For Dashboard v1 compatibility, we use Collapsed rows as a replacement for v2 Tabs.

We do not provide direct positions, but instead calculate grid positions based
on a 24-column grid system.
The default height of rows is 9.

## Palettes

We offer a few colorblind palettes in `grafana-dashboards/dashboards/palette.py`
for use in dashboards and
`grafana-dashboards/dashboards/threshold.py`
for nice consistency and accessibility.

Grafana does not provide colorblind-friendly palettes by default.

Read the comments in `dashboards.palette` and `dashboards.threshold`
for their intended usage.

### Tab-Level Theming

For non-health metrics (counts, totals, capacity, etc.) where there's no
intrinsic good/bad coloring, pick a tab-level theme shade and use it
across all stat-style panels in that tab. The convention is:

```python
# At module scope, near the top of each tab's file:
COMPUTE_THEME = palette.THEME_PALETTE[3]  # pick a distinct index per tab
```

Pass the shade through to `visualization.sparkline_stat(shade=…)` (see
"Panel Visualization Conventions" below). This gives each tab a visually
distinct background hue without re-deriving the choice in every panel.

## Variables

### Advanced Controls

For variables which should generally be left on their defaults, but may be
modifiable for "power users", we use the "Controls" section of the variable editor.
(In v2 dashboards, this is "inControlsMenu". In v1, this is VariableHide "3").

### Intermediates

Intermediate variables are variables that are computed from other variables
and are hidden from the UI.
(In v2 dashboards, this is "hideVariable". In v1, this is VariableHide "2").

Constant Variables may contain "chained variables" and may use
other variables as part of their definition.
This pattern slightly contradicts the documentation
which says Constant Variables are "static".
This pattern is useful for reusable snippets.

### Multi-Select Variables in Regex Contexts

For multi-select variables (`multi: true`) used in PromQL label matchers,
prefer the explicit `:regex` interpolation format when the variable is
embedded inside a wider regex string. Grafana auto-detects the regex
format only for the simple direct case `label=~"$var"`; when the variable
appears inside a larger pattern, the auto-detection does not fire and
bare `$var` resolves to literal `$__all` (or a `{val1,val2}` glob form)
that doesn't behave as alternation.

```
# Direct usage — auto-detected, plain `$var` is fine:
compute_cluster_id=~"$mzClusterList"

# Embedded usage — use `:regex` to get `(val1|val2|…)`:
pod=~".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"
```

This is the same guidance Grafana's own MCP tool surfaces in its
dashboard-authoring hints.

## Generating Dashboards

Dashboards can generally be generated by running the relevant
dashboard module.

These should include a
```
if __name__ == "__main__":
    from grafana_foundation_sdk.cog.encoder import JSONEncoder

    print(JSONEncoder(indent=2).encode(MyDashboard()))  # noqa: T201

```
block (you should include the lint rule,
because prints are disallowed in committed code otherwise).

## Pushing Dashboards to Grafana

The canonical production path is `gcx dashboards update`, which handles
the wrapping and the API call. The notes below cover the ad-hoc /
verification path when you're iterating from a Claude Code session with
the Grafana MCP.

### Use the v2 API directly

`mcp-grafana`'s built-in `get_dashboard_by_uid` and `update_dashboard`
tools convert dashboards to the v1 representation on the way out, which
strips queries from v2-only panel/layout features. For anything that
must round-trip a v2 dashboard, hit the v2 resource API via
`grafana_api_request`:

```
GET /apis/dashboard.grafana.app/v2/namespaces/default/dashboards/<uid>
PUT /apis/dashboard.grafana.app/v2/namespaces/default/dashboards/<uid>
```

PATCH is generally unavailable in our deployments (service accounts only
receive the `update` verb, not `patch`); use the full PUT.

### PUT body shape

PUTs must wrap the dashboard spec in the Kubernetes-style envelope:

```jsonc
{
  "apiVersion": "dashboard.grafana.app/v2",
  "kind": "Dashboard",
  "metadata": {
    "name": "<uid>",
    "namespace": "default",
    "resourceVersion": "<rv from current GET>",
    "annotations": {
      "grafana.app/folder": "<folder uid from current GET>",
      "grafana.app/message": "<one-line summary of this change>"
    }
  },
  "spec": { /* JSONEncoder output of MyDashboard() */ }
}
```

Gotchas:
- **Folder annotation is required on update.** Without
  `metadata.annotations["grafana.app/folder"]`, Grafana treats the PUT as
  a move-to-root and returns 403 *"not allowed to create resource in the
  destination folder"*. Always fetch the current resource first and
  carry the folder annotation forward.
- **Always set `grafana.app/message`.** This is the dashboard's version
  history entry — populate it with a one-line summary describing the
  change in this revision (same role as a git commit message).
- **`resourceVersion` enables optimistic concurrency.** Fetch + PUT, not
  fire-and-forget; otherwise concurrent saves can clobber each other.

### Service account permissions

Reads work with a Viewer-scoped token, but PUT requires Edit on the
destination folder. The clearest error tells you which: *"not allowed to
update resource in the source folder"* = no edit on the existing folder;
*"not allowed to create resource in the destination folder"* = missing
folder annotation or no edit on the target folder.

## PromQL Conventions

### Rate intervals

Use `[$__rate_interval]` for `rate()` window selectors. Grafana derives
this from the panel's resolution so the rate window adapts to zoom
level. Use a literal range (`[5m]`, `[1h]`) only when the panel needs a
specific window for semantic reasons — e.g. the "Current CPU Usage (5
min)" summary stat deliberately samples a 5-minute window regardless of
zoom.

### Filtering cAdvisor metrics

The `$containerFilter` constant variable expands to
`namespace=~"$mzNamespaceList",container!="",container!="POD"`. This
excludes the pod-network-namespace sentinel and the empty-container
series cAdvisor reports for pod-level metrics.

That means **don't use `$containerFilter` for `container_network_*`
metrics** — those *are* the pod-level metrics it excludes. For network
queries, scope only with `namespace=~"$mzNamespaceList"` (plus pod
regex matchers as needed).

### Aggregation defaults

- For per-container metrics that you want to see per-pod (CPU, memory),
  group by `(namespace, pod, container)`.
- For network metrics, group by `(namespace, pod)` — this also drops the
  per-`interface` cardinality (most pods report at least `eth0` + `lo`).
- For environment-wide rollups, group only by `(namespace)` or
  `(container)` as appropriate.

### Series Cardinality Budgets

Prefer aggregating away `collection_id`, `replica_id`, and `worker_id`
on environment-wide panels unless a breakdown is the panel's whole
point. Large customer environments can have hundreds of collections
multiplied by replicas multiplied by workers — keeping that cardinality
has caused graphs to fail to load on production dashboards.

The dashboard default is **per-cluster aggregation**; specialists can
drill down to specific collections via ad-hoc PromQL when needed. A
working dashboard at less granularity is more valuable than a broken
one with maximum detail.

Concretely:

- `sum by (instance_id)` rather than `sum by (instance_id, collection_id)`
- `max by (cluster, replica)` rather than per-worker series, *unless* the
  whole point of the panel is worker drift / skew detection (e.g. the
  Dataflows "per worker" panel is intentionally per-worker; the
  aggregate Dataflow Count panel is not).
- For "show me the worst offenders" panels, use `topk(N, …)` rather
  than letting every series through.

## Filtering by Cluster / Replica

Materialize cluster pods follow the naming convention
`…-cluster-<cluster_id>-replica-<replica_id>-…`. To make the
`mzClusterList` and `mzReplicaList` selectors filter cluster pods without
hiding system pods (envd, balancer, etc.), use **two queries per panel**
with module-level regex constants:

```python
CLUSTER_POD_RE = ".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"
NONCLUSTER_POD_RE = ".*-cluster-.*-replica-.*"

# Query 1 — cluster-replica pods, filtered by selection:
container_cpu_usage_seconds_total{$containerFilter, pod=~"<CLUSTER_POD_RE>"}

# Query 2 — everything else, always shown:
container_cpu_usage_seconds_total{$containerFilter, pod!~"<NONCLUSTER_POD_RE>"}
```

Putting the regex constants at module scope (next to `CADVISOR_MISSING`
etc.) keeps them shareable across all panels in the file and prevents
drift between numerator and denominator patterns.

## Panel Visualization Conventions

Shared panel-styling helpers live in
[`dashboards/visualization.py`](../../../packages/grafana-dashboards/dashboards/visualization.py).
**Prefer importing from there over hand-rolling per-tab versions.** It
currently exports:

- `NO_FILTER_MATCH` — standard "no value" string for panels driven by
  multi-select filters (e.g., "No matches for the current filters").
- `PIE_LEGEND_BUILDER` — pre-configured piechart legend (table layout,
  right placement, value column).
- `TS_LEGEND_BUILDER` — pre-configured timeseries legend (table layout,
  bottom placement, Max / Avg / Last calcs).
- `sparkline_stat(shade=…)` — factory returning a `stat.Visualization`
  with the area-mode sparkline pre-configured and (optionally) a fixed
  background shade.

### Sparkline stats

For "count" / "total" / "capacity" style metrics, prefer
`visualization.sparkline_stat(...)` over a plain stat:

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

- **Use a range query, not `.instant()`.** Sparklines need a series of
  points to render; if the query is instant, the panel will show the big
  number but the sparkline footer will be blank. Donuts / piecharts /
  single-value panels still want `.instant()` — the rule is "only
  instant queries when a single point is exactly what's being
  displayed."
- **`.min(0)` for counts.** Without it, Grafana auto-zooms the
  sparkline Y-axis to the data's actual range, which makes a count
  that drifts from 64 to 66 look like a huge swing. Anchor to zero so
  the magnitude is visible.

### Partitioned sparkline stats

When a sparkline-stat query produces multiple series (e.g. `sum by
(session_type) (...)` returning `system` and `user` rows), the stat
panel renders one tile per series. In that case set
`text_mode=VALUE_AND_NAME` so each tile labels itself with its series
name; otherwise you get a row of bare numbers with no indication of
which is which.

```python
.visualization(
    visualization.sparkline_stat(shade=MY_THEME)
    .min(0)
    .text_mode(common.BigValueTextMode.VALUE_AND_NAME)
)
```

For single-series sparklines, leave the default `VALUE` text mode — the
panel title is the label.

### Timeseries legend

Apply the shared timeseries legend builder to every multi-series
timeseries panel:

```python
.visualization(
    timeseries.Visualization()
    .unit("Bps")
    .no_value(CADVISOR_MISSING)
    .legend(visualization.TS_LEGEND_BUILDER)
)
```

Notes:
- **Placement BOTTOM** gives the table room for the per-series name +
  calc columns without truncation; RIGHT works for short legends only.
- **Avg → `mean`, Last → `lastNotNull`.** Plain `last` includes nulls
  and surprises users when the most recent scrape was missing.

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

Every panel that depends on an optional or filterable metric source
should set `.no_value("…")` with a self-explanatory reason. Reach for
the closest existing constant rather than inventing new wording:

- `visualization.NO_FILTER_MATCH` — multi-select filter excluded
  everything (cluster/replica/namespace selection).
- `CADVISOR_MISSING` / `KSM_MISSING` (defined in `k8s_resources.py`) —
  required scrape target is absent.

This way a blank panel tells the operator *why* it is blank.

### Color-mode default

For stat panels showing values that aren't intrinsically good/bad
(counts, totals, capacity), use `color_mode=NONE` so the value renders
in the default text color rather than green. `visualization.sparkline_stat`
already does this. For health metrics, use `color_mode=BACKGROUND` plus
an explicit thresholds/mappings palette (see `dashboards.threshold`).

### Layouts

- Prefer `AutoGridLayout` over fixed positioning.
- Use `.max_column_count(N)` to tune density. For panels with wide
  legend tables (multiple calc columns + long pod names), 2 columns per
  row is a good default; for compact stat panels, the default 3 or
  bumping to 5 (e.g. workload readiness) is fine.
- **Column-width sizing** (`AutoGridLayout.column_width_mode(...)`):
  - `"narrow"` — rows of mostly-stat panels alongside one or two donuts;
    keeps the donut from stealing all the horizontal space.
  - `"wide"` — rows of complex panels (timeseries with table legends,
    histograms, bar charts, tables). Lets each panel get enough room to
    be readable; on smaller monitors the row will scroll horizontally
    rather than cram everything into a too-narrow column.
  - Default (`"standard"`) is fine for typical mixes.
- **Do not** wrap a small set of related panels in nested sub-rows when
  the auto-layout will tile them correctly — let `AutoGridLayout`
  handle the 2D wrap.

### Collapsed rows for type-specific drilldowns

When a row only applies to a subset of environments — e.g. Iceberg-sink
metrics only matter when Iceberg sinks exist — declare the row collapsed
by default with `.collapse(True)`:

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

Operators can expand the row when they need it; the row title acts as
documentation that the section exists. This keeps the default page light
without losing the type-specific content.

## Writing Panel Descriptions

Grafana renders panel `.description(...)` text as a hover tooltip and a
full info dialog (click the panel's title chevron). It supports
**GitHub-flavored Markdown**. Descriptions are the operator's
first-line documentation for "what am I looking at" — invest in them.

### Audience

Write for a **Materialize end user**: someone with database experience
and basic familiarity reading graphs, but minimal cloud / Kubernetes /
observability expertise. Assume SQL fluency. Explain Materialize-side
concepts (peek, hydration, arrangement) when they appear. Don't
restate the obvious ("Network bandwidth per pod" — they can read the
title).

### Structure

Lead with a **bold headline sentence** that captures the panel's whole
purpose. Grafana truncates the hover-tooltip preview, so the lead has
to carry the punch line on its own. After that, free-form prose
covering nominal/anomalous framing and where to look next:

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
- *Italics* for cross-references between panels:
  `_Compute Objects -> Arrangements_`
- Backticks for identifiers and code:
  `` `mz_internal.mz_indexes` ``, `` `cluster_id` ``
- Use **ASCII `->`** in cross-references, **not** Unicode `→`. The
  arrow shows up in panel titles and descriptions; `→` triggers
  the ruff `RUF001`/`RUF002` "ambiguous character" lint rules.
- Em-dash `—` is OK inside description bodies and docstrings, but
  avoid it in *titles* (`RUF001` flags it in panel titles).

### Cross-references

Reference panels by their visible title, italicized, using `->`
between tab and panel when crossing tabs:

```
For per-pod CPU view see _Kubernetes Workloads -> Pod CPU Usage_.
Pair with _Sink Lag_ (in this tab) when investigating commit issues.
```

Bare prose references are easier to follow than HTML/anchored links
in the current dashboard ergonomics. Don't include clickable URLs.

### SQL drilldowns

Where a panel surfaces a raw id (`source_id`, `collection_id`,
`sink_id`), include the SQL to translate it to a user-friendly name:

```
Translate `collection_id` to a name via
`SELECT id, name FROM mz_internal.mz_indexes` (or `mz_materialized_views`).
```

### Per-variant descriptions for shared helpers

When a single panel method is called multiple times with different
parameters and each variant deserves its own description (e.g. the
Peek Latency panels at p50/p90/p99), define a module-level dict keyed
by the variant label and have the helper look it up:

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

When a helper function or mixin method registers the same panel on
multiple tabs (e.g. `cpu_total_panel` from `KubeResourcesMixin`
appears on both Summary and Kubernetes Workloads), the description is
shared across all call sites. Either:

- Write a single description that's accurate in both contexts (and
  call out the differences inline: "On Summary the monitoring exporter
  is excluded; on K8s it's included.")
- Refactor the helper to accept a `description=` parameter and have
  each call site pass its own.

The shared-string approach is cheaper; refactor only when the
descriptions truly need to diverge.

---

# Current Dashboard State

This section captures the live state of the dashboards in this repo so
the next session has something concrete to start from. Update it when
state changes meaningfully (new dashboard, new tab, retired panel).

## Dashboard Inventory

| Family | Dashboard module | Class | Live UID |
|---|---|---|---|
| `mz_environment` | `overview.overview_dashboard` | `EnvironmentOverviewDashboard` | (auto-assigned at first upload; codified UID is `mz-mon-env-top` but the live one diverged before that became authoritative — see "UID Selection and Behavior") |

The `mz_environment/overview` dashboard has six tabs, in declared order:

| # | Tab title | Module | Theme |
|---|---|---|---|
| 1 | Summary | `summary.py` | (no unique theme; uses health palette and themes from imports) |
| 2 | Kubernetes Workloads | `k8s_resources.py` | `K8S_THEME` = `palette.THEME_PALETTE[0]` (blue) |
| 3 | Cluster Objects / Replicas | `cluster_objects.py` | `CLUSTERS_THEME` = `palette.THEME_PALETTE[2]` (teal) |
| 4 | Connections / Activity | `connections_activity.py` | `CONNECTIONS_THEME` = `palette.THEME_PALETTE[1]` (cyan) |
| 5 | Compute Objects | `compute_objects.py` | `COMPUTE_THEME` = `palette.THEME_PALETTE[3]` (orange) |
| 6 | Storage Objects | `storage_objects.py` | `STORAGE_THEME` = `palette.THEME_PALETTE[4]` (yellow) |

The `Summary` tab re-uses the `KubeResourcesMixin`'s `cpu_total_panel`
and `memory_totals_panel`, and also mirrors the
`add_currently_hydrating_panel(...)` helper from `compute_objects.py`
in its Environment Health row.

### Tab-by-tab row structure

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

### Known stubs and orphans

- **`compute_objects.py` Freshness row** — title-only, reserved for end-to-end freshness/lag metrics. Pick a freshness signal (`mz_internal.mz_materialized_view_refreshes`?) when filling it in.
- **`dataflows.py`** — orphaned after Dataflows became a row inside Compute Objects rather than its own tab. Safe to delete; only referenced from `overview_dashboard.py`'s import history (now removed).

## Reference Environments

Materialize developers may have access to an internal shared
Grafana with multiple test environments.
It can be useful to look at queries in live environments
when building dashboards.
Do not use environments without explicit permission.

Always scope investigative queries with `materialize_cloud_organization_id="..."` when
testing — these are shared envs and you don't want to mix data across
them.

## Materialize Metric Label Families

Materialize metrics come from two scraper paths with **different label
naming conventions**. Picking the wrong filter is a common failure
mode.

**Short-form** (env-scoped pre-calc and envd-side metrics):
- `instance_id` (this is the cluster id)
- `replica_id`

Examples: `v2_mz_compute_hydration_time_seconds`,
`v2_mz_compute_replica_peek_duration_seconds_*`,
`v2_mz_dataflow_elapsed_seconds_total`, `mz_active_subscribes`,
`mz_compute_controller_*`, `mz_query_total`, `mz_adapter_commands`.

**Long-form** (compute-side metrics, scraped from clusterd):
- `cluster_environmentd_materialize_cloud_cluster_id`
- `cluster_environmentd_materialize_cloud_cluster_name`
- `cluster_environmentd_materialize_cloud_replica_id`
- `cluster_environmentd_materialize_cloud_replica_name`
- `cluster_environmentd_materialize_cloud_replica_role`
- `cluster_environmentd_materialize_cloud_workers` (cluster size)
- `worker_id`

Examples: `mz_arrangement_maintenance_seconds_total`,
`mz_compute_replica_history_dataflow_count`, `mz_source_*`,
`mz_sink_*`.

**Pre-calc metrics with NO cluster labels** (env-scoped only):
`v2_mz_sources_count`, `v2_mz_sinks_count`, `v2_mz_tables_count`,
`v2_mz_views_count`, `v2_mz_indexes_count`, `v2_mz_mzd_views_count`,
`v2_mz_source_status`, `mz_active_subscribes`. These get the
`ENV_SCOPED_NOTE` callout in descriptions.

**Helper constants for filtering**:
- `_COMPUTE_FILTER` (in `storage_objects.py`) — long-form filter on env + cluster + replica.
- `_ARRANGEMENT_FILTER` (in `compute_objects.py`) — same shape, different module. Originally arrangement-specific, now reused for dataflows.
- These two constants are **the same PromQL fragment** in two places; lifting them to a shared module is a cleanup candidate.

## Module-Level Constants and Helpers

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

## Known Metric Quirks and Gotchas

Things that have surprised us during development; worth knowing before
touching the relevant panels.

- **"Peek" is the read-query latency metric.** No "query" in the name.
  `v2_mz_compute_replica_peek_duration_seconds_*` is the histogram for
  read-query latency on indexed data (the differential-dataflow
  operation behind `SELECT … FROM <view>`).
- **No `v2_mz_sink_status`** analog of `v2_mz_source_status`. Sink
  panels show `sink_id` rather than friendly names; translate via
  `SELECT id, name FROM mz_sinks`.
- **`v2_mz_source_status` doesn't cover all sources in all envs.**
  Some primary sources visible in `mz_source_bytes_received` (via
  `parent_source_id`) don't appear in `v2_mz_source_status`. The
  Source Bytes Received panel uses an outer-join pattern to handle
  this gracefully (see PromQL Recipes below).
- **`mz_source_bytes_received.source_id` is the *subsource* id**, not
  the primary. The primary lives in `parent_source_id`. Postgres
  sources fan out one bytes_received series per replicated table.
  Aggregate by `parent_source_id` to get per-primary rates.
- **`mz_sink_oustanding_progress_records` is misspelled** in
  Materialize itself ("oustanding" not "outstanding"). Don't "fix" the
  PromQL — match the metric name as-is.
- **`mz_compute_controller_subscribe_count` vs `mz_active_subscribes`
  trade-off**: the former has `instance_id` (cluster-filterable) but
  no `session_type`; the latter has `session_type` but no cluster
  labels. The summary tab uses `mz_active_subscribes` for the
  session_type donut, accepting the loss of cluster filtering.
- **`v2_mz_production_object`** is the only catalog metric with
  `cluster_id` — useful for cluster-filtered counts of indexes,
  materialized views, and sources (`type=` values are `index`,
  `materialized-view`, `source`).
- **`s2` is the `mz_catalog_server` cluster** and dominates many
  panels (commit rates, peek counts, arrangement maintenance,
  hydration). It's a system cluster and the noise floor is its
  business-as-usual. Mention this explicitly in panel descriptions
  where users might mistake it for an anomaly.
- **`v2_mz` vs `mz_` prefix convention**: prefer `v2_mz` when both
  exist. v2 metrics come from the newer promsql-exporter and are
  typically env-level pre-calculations; `mz_` metrics come from
  clusterd/envd scrapers and are richer in cluster/replica labels.
- **`v2_mz_sources_count` and `v2_mz_sinks_count` carry breakdown
  labels** (`type`, `envelope_type`, `size`). A naive `max(...)`
  returns the largest single bucket, not the total. Use
  `max(sum by (instance) (...))` to dedup across exporter pods *and*
  sum over the breakdown labels in one shot. See
  `_env_total_count_query` in `storage_objects.py`.

## PromQL Recipes

Reference for patterns we've established that aren't obvious in the
language docs.

### Outer-join for label enrichment

When one metric has the value you want and another has the friendly
name, you can't always inner-join (some entities may be missing from
the name metric). Use a two-query outer-join:

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

Each branch goes into its own `promql_query(...)` in the panel; their
legends can differ (e.g., `{{source_name}}` for the named branch and
`{{parent_source_id}}` for the orphan). Real example:
`_source_bytes_received_panel` in `storage_objects.py`.

### Table pivot via `groupingToMatrix`

To turn one row per (entity, dimension) into one row per entity with
columns per dimension value (e.g., Success / Errors columns from a
`status` label):

```python
.transformation(... labelsToFields keepLabels=[entity, dimension])
.transformation(... merge)
.transformation(... groupingToMatrix
                rowField=entity columnField=dimension valueField=Value)
.transformation(... organize  renameByName={...})
.transformation(... sortBy    ...)
```

After `groupingToMatrix`, the row-identifier column comes out named
`<rowField>\<columnField>` literally (one backslash). In Python source
that's `"<rowField>\\<columnField>"` (Python escape for one
backslash). Real example: `_adapter_commands_by_application_panel`
in `connections_activity.py`.

The naive alternative — two queries joined by `joinByField` — produces
one Value column **per input frame**, not per query, which is N×M
columns instead of 2. We tried that and gave up.

### Histogram quantile aggregated by labels

Standard pattern, but worth pinning the shape because the `sum by`
labels matter:

```promql
histogram_quantile(0.99,
  sum by (le, <preserved_labels...>) (
    rate(<metric>_bucket{<filter>}[$__rate_interval])
  )
)
```

Real examples: `_peek_latency_panel` (per cluster_id/replica_id),
`_iceberg_commit_latency_panel` (aggregated env-wide).

### `or vector(0)` to keep panels non-empty

For stat panels where "no series" should render as `0` rather than "No
data":

```promql
count(<series_query>) or vector(0)
```

Real example: `add_currently_hydrating_panel`.

### Per-cluster aggregation that handles label breakdowns

To get a single env-wide count from a metric with breakdown labels
(like `v2_mz_sources_count.type`/`envelope_type`/`size`), without
falling for the "max grabs the biggest bucket, not the total" trap:

```promql
max(sum by (instance) (<metric>{$environmentFilter})) or vector(0)
```

`sum by (instance)` collapses all label dimensions per scraper
instance, then `max(...)` dedups across multiple promsql-exporter pods
if there's more than one. Real example: `_env_total_count_query` in
`storage_objects.py`.

### Cluster + non-cluster pod split

For Kubernetes panels (CPU, memory, networking) where you want the
cluster/replica selectors to scope cluster pods but not hide
infra pods:

```promql
# Cluster pods (filtered)
<metric>{$containerFilter, pod=~".*-cluster-${mzClusterList:regex}-replica-${mzReplicaList:regex}-.*"}

# Non-cluster pods (always shown)
<metric>{$containerFilter, pod!~".*-cluster-.*-replica-.*"}
```

Constants `CLUSTER_POD_RE` and `NONCLUSTER_POD_RE` in
`k8s_resources.py` hold the regex strings — reuse them rather than
re-typing.

---

# Cleanup / Refactor Candidates

Tracked items that are working but could be tidier:

- **`ENV_SCOPED_NOTE` is duplicated** in `compute_objects.py` and
  `storage_objects.py`. Lift to `visualization.py` (or a sibling
  `_messages.py` if it grows).
- **`_COMPUTE_FILTER` and `_ARRANGEMENT_FILTER` are the same string**
  in two modules. Lift to a shared place; rename to something neutral
  like `_LONGFORM_CLUSTER_FILTER`.
- **`dataflows.py` is orphaned.** Safe to `rm`.
- **The Compute Objects "Freshness" row is a title-only stub.** Pick a
  freshness signal and fill it in (`mz_materialized_view_lag_seconds`
  in newer Materialize versions, or a derived metric from frontier
  metrics).
- **`mz-mon-` prefix isn't enforced in `MzDashboard.UID`** values today
  (the class has `UID = "env-top"` and `MzDashboard.__init__` prefixes
  it). Consistent across all current dashboards (one). Worth a
  validator if more dashboards land.
