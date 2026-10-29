---
title: "Datadog translations"
weight: 20
aliases:
  - /reference/internal/queries/datadog/
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Datadog translations

Every PromQL query in `packages/queries/` carries a `datadogQuery` beside its `promQL`, except the panels that read `ALERTS`.
Datadog alerts are monitors, not a series, so those panels have no Datadog form.
They render side by side as tabs on [Common Queries]({{< relref "../../common-queries.md" >}}).
This page holds the conventions behind them, for you as a contributor adding or correcting one.

The translations were validated against a live Datadog organization receiving a self-managed install's metrics through the gateway's Datadog exporter.
Each one parses, each monitor passes Datadog's monitor validation, and each panel whose metrics reach Datadog was compared with its PromQL on the same install's Thanos.
The install sent only the `essential` and `recommended` tiers, and it runs no Karpenter, CockroachDB, or Cilium.
Queries over those components, and over the `extended`-tier Ceph, CNPG, and postgres-exporter families, are checked for syntax and for the shape of their joins and filters, not for data.

The native Datadog dashboard set is [DEP-115](https://linear.app/materializeinc/issue/DEP-115), on the [roadmap]({{< relref "../roadmap.md" >}}#dashboards).

## What language these are in

The translations use Datadog's metric query syntax, which is what you paste into a dashboard widget or a monitor:

```text
max:mz_dataflow_wallclock_lag_seconds{materialize_cloud_organization_name:your-env AND quantile:1} by {instance_id}
```

They aren't DDSQL.
DDSQL over metrics can't express rates, histogram quantiles, or arithmetic between series, which is most of what these queries do.

Queries that back an **alert** are written as full monitor queries, with the evaluation window from the alert's `for:` and the threshold inlined:

```text
avg(last_5m):(
  avg:%%{mzSqlPrefix}can_connect{%%{excludeEnvironmentFilter}} by {namespace,pod}
) <= 0.1
```

Queries that back a **panel** are bare metric queries, with no window and no comparison.

A dashboard widget's JSON holds a panel query in two parts:
the metric queries, each named, and a formula over those names that carries every function and operator.
`count_not_null(avg:m{…} by {…})` becomes the query `avg:m{…} by {…}` and the formula `count_not_null(query1)`.
A query object that still carries a function renders as "Invalid Formula in Query Object", although the query API evaluates the whole expression.
The widget editor makes this split for you when you paste a whole expression into its query box, so only JSON you import needs the two parts.
The Datadog dashboards split each template this way in `packages/dashboards/src/datadog/formula.rs`,
so the registry keeps one expression per template.

## Layout

A `datadogQuery` is laid out like the `promQL` beside it, so the two read side by side.

- **One line when the PromQL is.** A single-line PromQL gets a single-line monitor query.
- **Otherwise the window opens a parenthesis**, where the PromQL's outer aggregation would. Each operand then takes a line of its own, indented two spaces.
- **A binary operator leads its line**: `/ sum:…`, as in `/ sum by (…) (`. Inside a monitor's parentheses it takes the operand's indent, and in a bare panel query it's indented two spaces past the first operand.
- **The threshold follows the closing parenthesis**: `) > 80`.
- **A line past 160 characters, the repository's YAML line limit, wraps before `by {…}`**, indented two more spaces.
  A filter that is still too long can break inside its braces, between terms.
  Keep a modifier such as `.as_rate()` directly against the closing brace or `by {…}`.

## How metrics reach Datadog

The gateway scrapes Prometheus, bridges to OTLP through `otelcol.receiver.prometheus`, and exports through the Datadog exporter.
For the pipeline, see [Storing Metrics]({{< relref "../../../metrics/storing.md" >}}), and for a working example, see `otel-metrics-fanout.values.yaml`.

The following table lists what the bridge and the exporter do to names, tags, and types:

| | In Datadog | What a translation does |
|---|---|---|
| Metric names | Unchanged, including case and `_total` suffixes, except that `:` becomes `_`: the recorded series `ext:consensus_up` is `ext_consensus_up` | Uses the Prometheus name, with `_` for `:` |
| Labels | Tags with the same names, except `job` and `instance`. Datadog matches tag keys case-sensitively and documents them as lowercased | Uses the label name as the tag, in lowercase |
| `job` | The `service` tag, from the OTLP `service.name` resource attribute | Writes `service` |
| `instance` | The `service.instance.id` tag, from the OTLP `service.instance.id` resource attribute | Writes `service.instance.id` |
| Label values | Lowercased and normalized: spaces and `;` become `_`, characters Datadog doesn't allow in a tag, such as `>`, are dropped, and repeated `_` collapse. `phase="Running"` is `phase:running`, and `backlog > 2hr` is `backlog_2hr` | Writes every literal tag value that way, because filters match case-sensitively |
| Counters | Datadog counts, holding each scrape's increment rather than the running total | Reads them with `.as_rate()` or `.as_count()` |
| Untyped counters | Gauges. node-exporter's `node_netstat_*` and `node_vmstat_*`, Ceph's `ceph_rgw_op_*_obj_lat_sum`, Alloy's `cluster_transport_*_failed_total`, and `mz_dataflow_elapsed_seconds_total` arrive this way | Reads them with `per_second()` |
| Histograms | One distribution under the base name, when the tier allowlist admits the buckets | Reads quantiles with `pNN:`, the mean with `avg:`, and the event rate with `count:` |
| Histograms without buckets | Separate `_count` and `_sum` counts, when the allowlist admits only those | Reads them as counters |

The exporter also tags every series with Kubernetes and host identity: `kube_node`, `kube_namespace`, `pod_name`, `host`, and others.
The translations don't rely on them, because the label-derived tags carry the same identity.

A label named after a tag the exporter sets itself, such as `service` or `host`, shares that tag.
The series then carries both values, such as `service:loki-canary` and `service:loki/loki-canary`, so a filter can match either one, and grouping by the tag multiplies the series.
A translation groups by another label that identifies the same thing, and says so in a comment.

A histogram's shape depends on the destination's tier.
Below the tier of its `_bucket` series, the gateway sends only the `_count` and `_sum` series the tier admits, and they arrive as counts under those names.
At or above it, the histogram arrives whole, as a distribution under the base name, and the `_count` and `_sum` metrics don't exist.
The translations are written for the Datadog destination's default tier, `recommended`, so a query reads whichever form that tier delivers.

A counter's running total doesn't exist in Datadog.
A PromQL query that reads a counter's value directly, such as `restarts_total > 0`, becomes an increment over a window with `.as_count()`.

### Percentiles need enabling per metric

A `pNN:` aggregator on a distribution fails with `missing_aggregation` until percentiles are enabled for that metric.
Enable them in Datadog under **Metrics** > **Summary**, on each distribution a `pNN:` query reads.
Enabled percentiles are billed as custom metrics.

## Filters

Every filter uses Datadog's boolean syntax: terms joined with `AND` and `OR`, negated with `NOT`, and grouped with parentheses.

```text
sum:kube_pod_container_resource_requests{resource:cpu AND container:* AND NOT (pod:vector-* OR pod:parca-agent-*)} by {pod}
```

Datadog also has a comma syntax, in which terms are joined with `,` and negated with `!`, and the two can't mix in one filter.
Only the boolean syntax has `OR` and `IN`, and template parameters splice into filters, so a translation never uses commas or `!`.

The following rules apply inside a filter:

- **`l!=""` is `l:*`.** Prometheus treats an empty label as an absent one, and Datadog rejects an empty value.
  `l=""` is `NOT l:*`.
- **`IN (…)` takes exact values only.** A wildcard inside `IN` doesn't parse, so `l=~"a.*|b.*"` is `(l:a* OR l:b*)`.
- **`*` is a term on its own.** `{* AND namespace:x}` is valid, so a parameter can default to `*`.
- **`l=~".+"` requires the label, and `*` doesn't.** Where a PromQL matcher or a Grafana **All** value of `.+` stands on a label that some series lack, such as cAdvisor's `container`, write `l:*` explicitly.
- **A value with a slash is valid unquoted**, as in `service:monitoring/mzmon-materialize-clusterd`, including inside `IN (…)`.

## Rates and increments

A PromQL `rate()` over a counter is `.as_rate().rollup(sum, 60)`:

```text
sum:mz_query_total{%%{mzEnvironmentFilter}} by {session_type}.as_rate().rollup(sum, 60)
```

**The rollup is required.**
The exporter sets no submission interval on the counts it sends, so Datadog assumes 10 seconds.
On a short time range the rollup bucket is 10 seconds too, and `.as_rate()` divides each scrape's increment by 10 rather than by the scrape interval.
That overstates every rate by the scrape interval over 10: three times for a 30-second scrape, and six for cAdvisor's 60.
`.rollup(sum, 60)` keeps each bucket at least as long as the longest scrape interval, and `.as_rate()` divides by the bucket.
On a longer range Datadog coarsens the bucket, and the rate stays right.

The bucket has to be at least as long as the metric's sampling interval.
The stack's own scrapes run every 60 seconds or faster, so `60` covers them.
A cloud provider pull samples every 300 seconds, so its counters take a longer rollup, such as `.as_rate().rollup(sum, 1800)` beside a PromQL `rate(…[30m])`.

A counter that arrives as a gauge has no `.as_rate()`, because the modifier only applies to counts.
Read it with `per_second()` over a 60-second `max` rollup:

```text
per_second(max:node_vmstat_pgfault{$nodeList} by {service.instance.id}.rollup(max, 60))
```

A PromQL `increase()` is `.as_count()`.
An `increase()` over a fixed window, such as `[1h]` on a panel that shows counts per hour, is `.as_count().rollup(sum, 3600)`.
In a monitor, `.as_count()` requires the `sum` time aggregator, as in `sum(last_1h):`.

## The monitor window

A monitor's window usually stands for the alert's `for:`, so its time aggregator has to demand the condition for the whole window.
The following table gives the aggregator for each kind of condition:

| The condition | Aggregator | Why |
|---|---|---|
| Must stay above a threshold (`>`, `>=`) | `min(last_…)` | `max` would fire on one sample and stay true after recovery |
| Must stay below a threshold (`<`, `<=`) | `max(last_…)` | The mirror case |
| Is an event in a window, such as a restart in the last 10m | `sum`, which `.as_count()` requires | The window is the event's, not a duration the condition holds for |
| Reads the PromQL's own range, such as `avg_over_time(…[15m])` | That range's aggregator | The window is the range, and `for:` is left to the monitor's evaluation delay |

## The translation table

The following table maps each PromQL construct to its Datadog form:

| PromQL | Datadog |
|---|---|
| `metric{l="v"}` | `avg:metric{l:v}` |
| `sum by (l) (m)` | `sum:m{…} by {l}` |
| `rate(c[$__interval])` | `sum:c{…}.as_rate().rollup(sum, 60)` |
| `increase(c[$__range])` | `sum:c{…}.as_count()` |
| `rate(g[5m])` for a counter that arrives as a gauge | `per_second(max:g{…}.rollup(max, 60))` |
| `histogram_quantile(0.99, sum by (le) (rate(h_bucket[…])))` | `p99:h{…}` |
| `rate(h_count[…])` | `count:h{…}.as_rate().rollup(sum, 60)` |
| `count by (o) (group by (i, o) (m))` | `count(max:m{…} by {i,o}, {o})` |
| `sum(max by (k) (m))` | `sum(max:m{…} by {k}, {*})` |
| `sum by (o) (a / b)` | `sum(sum:a{…} by {i,o} / sum:b{…} by {i,o}, {o})` |
| `m > 1e15` | `cutoff_min(m, 1e15)` |
| `max by (o) (m < 1e9)` | `max(cutoff_max(max:m{…} by {every series tag}, 1e9), {o})` |
| `sum by (o) (max by (o, k) (m < 1e9))` | `sum(max(cutoff_max(max:m{…} by {every series tag}, 1e9), {o,k}), {o})` |
| `q > 0`, to drop idle series | `cutoff_min(q, 1e-9)` |
| `a > bool b` | `is_greater(a - b, 0)` |
| `a * on (k) group_left (name) info` | `a * (info * 0 + 1)`, with both grouped by `k` |
| `a and on (k) b` | `a * (b * 0 + 1)`, with `b` grouped by `k` |
| `a and on (k) (b > c)` | `a * (cutoff_min(b, c) * 0 + 1)`, with `b` grouped by `k` |
| `topk(15, q)` | `top(q, 15, 'max', 'desc')` |
| `(q) or vector(0)` | `default_zero(q)`, which yields a zero series even when `q` has no data at all. Grouped, it can't add a missing group |
| `count(x == 0) or 0 * count(x)`, zero only while `x` exists | `count_not_null(cutoff_max(min:x{…} by {…}, 0))` |
| `clamp_min(q, 0)` | `clamp_min(q, 0)` |
| `deriv(q[5m])` | `derivative(q)` |
| `q offset 1h` | `timeshift(q, -3600)` |
| `a >= b`, to keep the points that pass | `cutoff_min(a - b, 0)`, because `cutoff_min` keeps a value equal to its threshold |
| `l=~"a\|b"` | `l IN (a,b)` |
| `l!~"a\|b"` | `l NOT IN (a,b)` |
| `l!="v"` | `NOT l:v` |
| `l=~"^s.*"` | `l:s*` |
| `l!=""` | `l:*` |

### Group-by functions

`sum`, `avg`, `min`, `max`, and `count` with a tag set aggregate across the series of an expression:

```text
max(cutoff_max(max:mz_dataflow_wallclock_lag_seconds{…} by {namespace,pod,instance_id,replica_id,collection_id}, 1e9), {instance_id})
```

They are how a translation aggregates twice, aggregates after a value filter, or divides before aggregating.
The inner query groups by every tag the PromQL's inner step keeps, and the function's tag set is the outer step's.

A PromQL value filter, such as `< 1e9`, applies to each series before any aggregation.
The query inside `cutoff_min` or `cutoff_max` therefore groups by every tag that tells one series from another: `namespace`, `pod`, `instance_id`, `replica_id`, and `collection_id` for wallclock lag.
Grouping by fewer tags merges series before the filter, so one series decides for all of them, and a hydrating replica's sentinel hides the replica that is serving.
Where the PromQL aggregates again after the filter, the group-by functions nest: `max(…, {instance_id,collection_id})` takes each collection's worst surviving reading, and a `sum` or `top` around it then counts each collection once.
A group-by function always takes a tag set, and `{*}` reduces to a single series, as an outer aggregation without `by` does.
Datadog validates a group-by function's tag set against its innermost query, so an outer group-by around a join that contains another group-by fails to validate, even though the join runs on its own.
`is_greater`, `is_less`, and `floor` work inside a group-by function, and `is_equal` doesn't.

### Arithmetic joins

Arithmetic between two grouped queries matches series on their shared tags.
Datadog accepts it only when one query's group-by tags are a subset of the other's, and it rejects any other pairing.
The result takes the larger tag set, so a query grouped by fewer tags fans out across the other's series.

The join is an inner join: a series with no match on the other side is dropped.
Multiplying by `(b * 0 + 1)` keeps `a`'s values where `b` has a series and drops the rest, which is `a and on (k) b`.
When `b` carries extra tags, the result carries them too, which is `group_left`.
An `unless` over a 0/1 metric is a multiplication by the opposite state, such as `max:kube_pod_status_phase{phase NOT IN (succeeded,failed)} by {namespace,pod}`.
A join needs both sides to spell the matched tags the same way, because Datadog can't rename a tag inside a query.

Between queries over count metrics the join is outer instead: a series missing on one side is filled with 0, so `a + b` over counts is a union, and a product with a missing side reads 0 rather than dropping out.
A filtering join therefore needs a gauge on the filtering side, such as `kube_pod_info` or `up`.

Every query in an arithmetic expression carries the same rollup interval.
When one side is a rate with `.rollup(sum, 60)`, give the other side `.rollup(max, 60)` for an indicator or `.rollup(avg, 60)` for a gauge, or the result holds values only at the rate's points and nulls between them.

Two more exceptions to the inner join matter:

- **An operand with no series at all reads as 0.** If `b` returns nothing, as it does when its metric doesn't reach Datadog, `a * (b * 0 + 1)` keeps every series of `a` rather than none.
- **A point a function nulls stays null.** A point that `cutoff_min` removes, or a division by zero, leaves no value through arithmetic with a query grouped by the same tags, so a filter applied before the join carries through.
  An ungrouped side whose points are all removed reads as 0 instead, so `a and on () (b > 0)` has no arithmetic form.

## What doesn't survive the translation

Where a query hits one of the following gaps, the `datadogQuery` carries the closest expressible form, and a comment above it says what was dropped.

**`time()`.** Nothing in Datadog reads the current wall clock inside a query, so anything shaped `time() - <timestamp metric>` isn't derivable.
`materialize.kubernetes.last_restart`, `infra.cockroachdb.backup_missing`, and both `materialize.launchdarkly.stale_*` queries report the raw timestamp instead of the age.
This also removes the pod-start grace periods from several alerts, where the monitor's new-data delay stands in.

**`label_replace`.** Datadog can't rename or extract a tag value.
An id-to-name join through `mz_cluster_info` or `mz_object_info` needs the id tag to have the same name on both sides.
The shipped monitors give it one: every series carries the canonical id, such as `cluster_id` beside `instance_id`, so a join written against the [canonical names](../../../../metrics/scraping/#canonical-labels) matches.
`materialize.environmentd.pod_pending_critical` also loses its generation-suffix deduplication.

**Joins Datadog can't match.** An [arithmetic join](#arithmetic-joins) needs the matched tags to have the same name on both sides, and one side's tags to be a subset of the other's.
It also can't pair two conditions over different windows, such as an exit code that outlives the restart it describes.
Where such a join is what makes a rule correct rather than merely narrower, the `datadogQuery` is a list, and its second entry is the other half to combine in a composite monitor (`a && b`).
`envd-terminated` and `system-cluster-terminated` are the sharpest case: the last exit code outlives the restart it describes, so without the restart-in-window half the monitor keeps firing after recovery.
The exit-code exclusions on `materialize.clusterd.error_kill` and the egress-gateway node restriction on the `infra.egress_gateway.*` queries are dropped the same way.

**Anti-joins against series that may not exist.** `a unless b` keeps the series of `a` that have no `b`, and Datadog's join drops exactly those.
Where `b` is a 0/1 indicator that every series carries, such as `kube_pod_status_phase`, multiplying by the opposite state works.
Otherwise the translation reads `a` alone, and a comment says to compare it with the panel that shows `b`.

**Zero lines.** A PromQL `q or 0 * other` draws 0 while `other` exists and nothing otherwise.
`default_zero()` draws 0 even with no data, and a grouped query can't merge in a second metric's series, so a quiet install reads no data where the PromQL reads 0.

**`absent()`.** Absence is a monitor's no-data notification in Datadog, not a query.
The `datadogQuery` carries a threshold that is never crossed, and a comment says to enable the monitor's `notify_no_data`.

**`or` chains across different metrics.** `materialize.persist.failures` folds sixteen counters into one alert with `label_replace` and `or`, so Datadog needs one monitor per counter, and three representative ones are given.

**Two layers of aggregation in a single number.**
A query value, toplist, sunburst, or table widget reads through Datadog's scalar query API.
That API refuses a group-by function such as `max(query1, {tag})`,
with "it is unsupported to use multi-layer aggregation function max with an aggregator".
A template that aggregates in two layers draws only as a timeseries, so the `env-top` panels built on one are timeseries in Datadog.

**Filtering before a time rollup.** A function such as `cutoff_max` runs after a query's `.rollup()`, so a range query that drops sentinel samples before taking `max_over_time` can't drop them first.
`materialize.info.max_lag` includes the sentinel.

## Template parameters

The `%%{…}` parameters have the same names as in the PromQL, but a Datadog context supplies Datadog values.
Every value that fills a filter is a boolean filter expression, so it composes with `AND` in a template's braces.
The following table gives the shape of each parameter's value:

| Parameter | Datadog value |
|---|---|
| `mzEnvironmentFilter` | A filter term, such as `materialize_cloud_organization_name:your-env` |
| `excludeEnvironmentFilter` | A filter expression, such as `NOT mz_context_org_type:e2e_test`, or `*` for none |
| `cAdvisorFilter` | `container:* AND NOT container:POD`, with a namespace term |
| `excludeHostNetworkPods` | A join factor rather than a filter: `* (max:kube_pod_info{host_network:false} by {namespace,pod}.rollup(max, 60) * 0 + 1)`, which keeps only pods on the pod network |
| `mzEnvironmentNamespaceFilter`, `mzOperatorNamespaceFilter`, and the other namespace filters | A filter term or `IN` expression on `namespace` |
| `mzClusterList`, `mzReplicaList` | A tag value or pattern, with `*` for every cluster |
| `infraCoreWorkloadList` and the other workload tiers | A comma-separated list of exact names, because they sit inside `IN (…)` |
| `interval`, `range`, `rangeWindow` | Bare seconds, for `.rollup()` and `timeshift()` |
| `mzSqlPrefix` | `mz_` on self-managed |

A value containing `OR` is parenthesized, so it can't change the precedence of the template around it.

Dashboard variables that the registry's Grafana queries use appear in the Datadog form as a standalone filter term, such as `{$nodeList AND mode:idle}`.
A Datadog template variable carries its own tag prefix, so the dashboard that defines `$nodeList` decides which tag it filters.
Where the Grafana variable holds part of a value, the Datadog variable holds the whole term: `$cnpgClusterList` names a CNPG cluster in Grafana, so a Datadog dashboard defines it as a pod pattern, such as `pod:CLUSTER-*`.

A dashboard whose templates write the tag themselves fills a parameter with a template variable's value alone.
`env-top` sets `mzClusterList`, `mzReplicaList`, and `mzNamespaceList` to `$cluster.value`, `$replica.value`, and `$namespace.value`,
so `instance_id:%%{mzClusterList}` filters `instance_id` by the picked cluster id.
The query API doesn't expand template variables, so to check a dashboard's queries, substitute the picked values first.

To render a query's Datadog template, build a context on the `Datadog` engine:

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

`doc_context` fills parameters with PromQL sentinels for metric extraction, so a query rendered this way shows its structure, not values you can run.

The `docgen` action isn't useful here: metric extraction parses PromQL, so `--engine datadog` finds nothing.
Metric names are the same in both engines, so extract them from the PromQL side.

## Validate a translation

Check a translation against a Datadog organization that receives a test install's metrics, with the install's Thanos beside it:

1. Fill the template's parameters with Datadog values for the install, and its PromQL's with PromQL values.
2. For an alert, send the monitor query to `POST /api/v1/monitor/validate`, which checks it without creating a monitor.
3. Run the expression through `GET /api/v1/query` over the last hour, and confirm it returns series.
4. For a panel, run the PromQL as an instant query on Thanos, and compare the series count and the recent values.

PromQL keeps series whose value is NaN, such as a quantile over no samples, and Datadog drops them, so a series-count difference there isn't a translation error.

Datadog checks a query over an absent metric for syntax, function arguments, and group-by subsets, but not for the tags an arithmetic join matches on.
For a metric that doesn't reach the organization, check the join's tags against the PromQL, and try the same shape on a metric that does.

<!--
Agent note: validated on 2026-10-21 against a GKE install sending the recommended tier through the
Datadog exporter (Alloy v1.20.1). The rate overstatement, the job/instance renames, and the lowercased tag
values are the three findings most likely to change with an exporter or bridge upgrade; re-check them first.
-->
