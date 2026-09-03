# Datadog Translations




# Datadog Translations

Every query in `packages/queries/` carries a `datadogQuery` alongside its `promQL`.
They render side by side as tabs on [Common Queries](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/stable-metrics/common-queries/); this
page is the conventions behind them, for whoever is adding or correcting one.

These are **translations, not a tested dashboard set** — there is no Datadog test environment in CI, so nothing here has
been run against a real Datadog account.
Treat them as a starting point you copy into a widget or monitor and correct, not as something that works unedited.

The native Datadog dashboard set is [DEP-115](https://linear.app/materializeinc/issue/DEP-115), scheduled for OO-M3 on
the [roadmap](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/roadmap/).

## What language these are in

Datadog's metric query syntax — what you paste into a dashboard widget or a monitor:

```text
p99:mz_compute_peek_duration_seconds{materialize_cloud_organization_name:your-env} by {instance_id}
```

Not DDSQL.
DDSQL over metrics is preview-only and cannot express rates, histogram quantiles, or arithmetic between series, which is
most of what these queries do.
The schema field was called `datadogSQL` until this landed; it is now `datadogQuery`.

Queries that back an **alert** are written as full monitor queries, with the evaluation window from the alert's `for:` and the threshold inlined:

```text
avg(last_5m):avg:v2_mz_can_connect{*} by {namespace,pod} <= 0.1
```

Queries that back a **panel** are bare metric queries, with no window and no comparison.

## How metrics reach Datadog

The gateway scrapes Prometheus, bridges to OTLP through `otelcol.receiver.prometheus`, and exports through the Datadog exporter.
See [Storing Metrics](/materialize-monitoring/preview/heather-troubleshoot-skill/metrics/storing/) for the pipeline and `otel-metrics-fanout.values.yaml` for a working example.

The naming consequences the translations assume:

| | |
|---|---|
| **Metric names** | Pass through unchanged. `v2_mz_compute_cluster_status` in Prometheus is `v2_mz_compute_cluster_status` in Datadog — no dot conversion, and `_total` suffixes are kept. |
| **Labels** | Become tags with the same names, so `instance_id` stays `instance_id`. |
| **Counters** | Arrive as Datadog counts (the exporter converts cumulative to delta), so they need `.as_rate()` or `.as_count()`. |
| **Histograms** | Collapse from `_bucket`/`_sum`/`_count` to one distribution under the base name, so `histogram_quantile(0.99, …)` becomes the `p99:` aggregator. |

That last point is the one place Datadog is *better* than the PromQL: a percentile is an aggregator prefix rather than a
`histogram_quantile` over a summed bucket rate.

The `.count` suffix used by a few Loki queries (`loki_request_duration_seconds.count`) requires
`histograms::send_aggregation_metrics` on the exporter; without it those two queries have nothing to divide.

## The translation table

| PromQL | Datadog |
|---|---|
| `metric{l="v"}` | `avg:metric{l:v}` |
| `sum by (l) (m)` | `sum:m{…} by {l}` |
| `count(group by (l) (m))` | `count_not_null(avg:m{…} by {l})` |
| `rate(c[$__interval])` | `sum:c{…}.as_rate()` |
| `increase(c[$__range])` | `sum:c{…}.as_count()` |
| `histogram_quantile(0.99, sum by (le) (rate(h_bucket[…])))` | `p99:h{…}` |
| `topk(15, q)` | `top(q, 15, 'max', 'desc')` |
| `(q) or vector(0)` | `default_zero(q)` |
| `clamp_min(q, 0)` | `clamp_min(q, 0)` |
| `deriv(q[5m])` | `derivative(q)` |
| `l=~"a\|b"` | `l IN (a,b)` |
| `l!~"a\|b"` | `l NOT IN (a,b)` |
| `l!="v"` | `!l:v` |
| `l=~"^s.*"` | `l:s*` |

## What does not survive the translation

Datadog's query language is deliberately narrower than PromQL, and these gaps are systematic rather than incidental.
Where a query hits one, the `datadogQuery` carries the closest expressible thing and a comment above it says what was dropped.

**Value filters.** There is no `m > 1e15` or `m < 1e9` that removes series from a result.
`materialize.compute.hydration.currently_hydrating` counts collections parked at the `u64::MAX` sentinel; the Datadog
version counts every collection reporting a lag, which is a different number.
The freshness queries drop their `< 1e9` sentinel exclusion the same way.

**Boolean comparison operators.** `> bool` produces a 0/1 series in PromQL and has no Datadog equivalent.
`materialize.storage.sources.upstream_errors` becomes a raw difference where a positive value means disconnected, rather than a 1.

**`time()`.** Nothing in Datadog reads the current wall clock inside a query, so anything shaped `time() - <timestamp metric>` is not derivable.
`materialize.kubernetes.last_restart`, `infra.cockroachdb.backup_missing`, and both `materialize.launchdarkly.stale_*`
queries report the raw timestamp instead of the age.
This also removes the pod-start grace periods from several alerts — use the monitor's own new-data delay instead.

**`label_replace`.** No renaming or extraction of tag values, so the id→name enrichment (`mzClusterName`, `mzObjectName`
) is gone: Datadog panels show `u123`, not the object's name, unless the metric already carries a name tag.
`materialize.environmentd.pod_pending_critical` also loses its generation-suffix deduplication.

**Joins (`and on (…)`, `group_left`).** Datadog matches tags automatically in arithmetic between two queries, but there
is no way to use one series purely as a filter for another.
This costs the exit-code exclusions on `materialize.clusterd.error_kill`, the swap-headroom condition on
`materialize.clusterd.swap_cluster_oom`, and the egress-gateway node restriction on all five `infra.egress_gateway.*`
queries — those now cover every node, not just the gateway pool.
A composite monitor is the usual way back.

**Nested aggregation.** `count by (size) (group by (id, size) (m))` has no single-query form: Datadog aggregates once.
Where the underlying metric is a 0/1 gauge, `sum:` by the outer label gets close (`materialize.clusters.replicas.sizes`
counts *ready* replicas per size rather than all of them).

**`or` chains across different metrics.** `materialize.persist.failures` folds sixteen counters into one alert with
`label_replace` and `or`; Datadog needs one monitor per counter, so three representative ones are given.

## Template parameters

The `%%{…}` parameters are the same names as the PromQL, but the *values* a Datadog context supplies are tag matchers
rather than label matchers — `mzEnvironmentFilter` renders as `materialize_cloud_organization_name:your-env-name`,
`mzClusterList` as `*` rather than `.+`, and `interval` / `range` as bare seconds for `.rollup()` rather than bracketed
ranges.
`mzmon_lib::query::render::doc_context` carries both sets.

To render a query for Datadog, build a context on the `Datadog` engine:

```rust
use std::path::Path;

use mzmon_lib::query::render::doc_context;
use mzmon_lib::query::{QueryEngine, QueryRegistry};

let registry = QueryRegistry::from_directory(Path::new("packages/queries"))?;
let ctx = doc_context(&registry, QueryEngine::Datadog, "mz_");
let rendered = registry
    .get("materialize.connections.peek_latency.p99")
    .expect("query exists")
    .render(&ctx)?;
println!("{}", rendered[0]);
```

The `docgen` action is not useful here: metric extraction parses PromQL, so `--engine datadog` finds nothing.
Metric names are identical across the two engines anyway, so extract from the PromQL side.

<!--
Nothing here has been validated against a live Datadog account. If a test
environment lands, the sentinel/value-filter and `.count`-suffix assumptions are
the first two things worth checking.
-->

