---
title: "Querying"
weight: 30
---

# Querying Metrics

Metrics are queried in **PromQL**, almost always through Grafana.

In the bundled stack, Grafana ships with a **Thanos datasource** pre-provisioned by the Grafana operator, so metrics are queryable from Explore and from dashboard panels with no extra setup.
Thanos Query presents a Prometheus-compatible API, so anything that speaks PromQL works against it unchanged — Grafana, `promtool`, an HTTP client, or an existing alerting system.

## Where to start

Most questions are already answered by a query someone wrote for a dashboard.
Two reference pages carry them, both generated from the same query registry the dashboards are built from:

- **[Common Queries](../../reference/stable-metrics/common-queries/)** — the PromQL behind the shipped dashboard panels, and the best starting point for a new panel or an ad-hoc investigation. Each query also renders a **Datadog** tab beside its PromQL.
- **[Common Alerts](../../reference/stable-metrics/common-alerts/)** — alerting expressions with thresholds and severities.
- **[List of Metrics](../../reference/stable-metrics/list-metrics/)** — the metric families those queries draw on.

> [!WARNING]
>   Common Alerts is **reference material, not a shipped rule set**. The expressions are sound but the thresholds are not universal, and the chart does not currently install them as `PrometheusRule` resources. Adopt them selectively rather than wholesale.

Two conventions to know before copying anything: most environments use `mz_` for `mzSqlPrefix` and do not set `mzEnvironmentFilter`.

## What you are querying across

Thanos Query fans a single PromQL request out across two kinds of source and merges the result:

- **Thanos Receive**, holding recent series still in memory or on local disk.
- **The Thanos Store Gateway**, serving historical blocks out of object storage.

A query spanning both is transparent — you do not choose a source. What this does mean is that a long-range query is doing more work than a short one, and that a slow historical query is usually a Store Gateway or object-storage question rather than a PromQL one.

Long-range queries also benefit from **downsampling**: Thanos keeps reduced-resolution copies of older blocks, and picks the appropriate one for the range you asked for. See [Storing](../storing/#retention-and-downsampling).

## Keeping queries fast

- **Bound the time range.** It is the single biggest lever, because it decides how many blocks are touched.
- **Filter by label early.** `cluster`, `namespace`, and `environment_id` narrow the series set before any aggregation runs.
- **Prefer recorded results for anything repeated.** A query evaluated on every dashboard load is a query worth recording once, as a Thanos or Loki recording rule.
- **Watch cardinality at the source, not the query.** A query over a high-cardinality family is slow because the family is large; the fix is at collection time, via `metricRelabelings` on the monitor or the [denylist](../storing/#the-denylist).

## Querying from outside Grafana

Thanos Query's HTTP API is Prometheus-compatible, so the standard endpoints apply:

```bash
kubectl port-forward -n monitoring svc/thanos-query 9090:9090
curl -s 'http://localhost:9090/api/v1/query?query=up' | jq '.data.result[0]'
```

Reach it by `port-forward` rather than by giving it an ingress — it has no authentication of its own. See [Securing](../../operating/securing/#the-cluster--the-stack).

## See also

- [Storing](../storing/) — what the queriers read from, and how retention and downsampling shape it.
- [Scraping](../scraping/) — where the series come from.
- [PromQL](https://prometheus.io/docs/prometheus/latest/querying/basics/) (official).
