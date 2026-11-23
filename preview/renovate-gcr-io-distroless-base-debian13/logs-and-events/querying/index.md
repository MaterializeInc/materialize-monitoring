# Querying




# Querying Logs

Queries — almost always issued through Grafana — are served by Loki's [read path](../architecture/#read-path).
This page covers how to query logs, how the read path serves them, and how to keep queries fast.
See the [logging architecture](../architecture/) for how the read path fits together.

## Querying through Grafana

In the bundled stack, Grafana ships with a **Loki datasource** pre-provisioned by the Grafana operator, so logs are queryable from Explore and from dashboard panels without extra setup.
Queries are written in [LogQL](../../o11y-glossary/#dashboards-and-queries), Loki's query language:

- A **stream selector** picks log streams by label, e.g. `{namespace="materialize", container="environmentd"}`.
- **Line filters** match within the body, e.g. `|= "error"` or `|~ "timeout|refused"`.
- **Parsers and expressions** extract and reshape fields, e.g. `| json | level="ERROR"`.

Because Loki indexes only labels, **every query should start with a label selector** that narrows the streams — the tighter the selector, the less data the queriers scan.

## How the read path serves a query

A query flows through the components described in the [read path](../architecture/#read-path):

1. The [Loki Query Frontend](../architecture/#loki-query-frontend) receives it, splits a large or long-range query into smaller pieces, and checks its result cache.
2. The [Loki Query Scheduler](../architecture/#loki-query-scheduler) queues the pieces fairly across tenants.
3. [Loki Queriers](../architecture/#loki-querier) execute the pieces — reading recent data from ingesters and historical data from object storage (using the [Loki Index Gateway](../architecture/#loki-index-gateway) to find the right chunks) — then results are merged and deduplicated.

Splitting and caching are why a broad query can still return quickly the second time, and why running at least two query frontends matters for fairness.

## Querying structured metadata

High-cardinality attributes such as `pod`, `node`, `trace_id`, `span_id`, and per-request detail are stored as [structured metadata](../architecture/#storage), not as labels.
They are still queryable — filter on them after a label selector — but they do not inflate [stream cardinality](../../o11y-glossary/#logs-and-events), so you get targeted lookups without the storage-and-stability cost of making them labels.

**`pod` and `node` are the two to know**, because they are what most people reach for first.
Both are structured metadata, so they go after a `|` rather than inside the braces:

```logql
{namespace="materialize-environment", app="environmentd"} | pod=~"mz.*-environmentd-.*"
{namespace="materialize-environment"} | node="gke-mycluster-default-pool-1a2b3c4d-xyz9"
```

Both churn, which is exactly why they are not labels.
A pod name is unbounded and changes on every restart, so indexing it would create a new stream per pod per rollout.
Node names churn on the same order for a different reason — every scale-up mints one and every scale-down retires it — so a cluster that autoscales would accumulate streams for nodes that no longer exist.
In both cases the cost lands on every query against that namespace, not only the ones that filter by pod or node.

`pod` is also the one that *changed*: it was a stream label under its `k8s_pod` spelling until that was removed.

## Keeping queries fast

- **Narrow the stream selector first.** Selecting by `namespace`, `app`, `container` or `level` before line filters is the biggest single speedup.
  Those are the labels you will reach for most; a stream also carries `component`, `job` and `service_name`, node journal logs carry `unit` instead of `namespace`, and a per-environment namespace adds `environment_id`.
  Anything not in that set — `pod`, `node`, `trace_id`, and the rest — is structured metadata and belongs after a `|`.
- **Bound the time range.** Shorter ranges scan fewer chunks; the frontend also caches by range.
- **Let the frontend split.** Long-range queries parallelize across queriers automatically when the frontend is in the path.
- **Scale the read side independently.** Queriers are stateless — add replicas for heavier query load without touching the write path.

> [!INFO]
>   The read path is independent of the write path, so query load and ingest load scale separately.
>   If dashboards feel slow, scale queriers and frontends rather than ingesters.

## See more

- [Logging Architecture](../architecture/) — the read path in context.
- [Storing](../storing/) — what queriers read from.
- [Rules](../rules/) — recurring LogQL evaluated on a schedule.
- [LogQL](https://grafana.com/docs/loki/latest/query/) (official).

