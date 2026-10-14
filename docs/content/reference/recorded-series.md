---
title: "Recorded Series"
weight: 43
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Recorded Series

These are the series the chart's recording rules write, generated from the query registry.
A recorded series is a metric name this repository mints, so it reads the same whatever produced the measurement underneath it.
The Thanos ruler evaluates the rules and writes the results back through the alloy-gateway, so a recorded series is queryable wherever the gateway's other metrics are.

Every `ext:*` series is part of the normalized layer for an external dependency.
One name is recorded by each **adapter** that can measure it, and the `flavor` label names the adapter.

| Label | On | Holds |
|---|---|---|
| `flavor` | Every `ext:*` series | The adapter that recorded it: `persist` is Materialize's own measurement, `rds`, `cloudsql` and `azure-postgres` are a cloud provider's, and `postgres` is the postgres-exporter's |
| `namespace` | Series from `persist` | The Materialize environment's namespace |
| `resource` | Series from a cloud provider or the exporter | The database's name at the provider, or the exporter target's name, as declared in `externalDependencies.consensus` |
| `database` | `ext:consensus_database_bytes` | The name of one database on the instance |
| `version` | `ext:consensus_version_info` | The version the database server reports |

An adapter that cannot measure a series records nothing for it, rather than a zero.
The provider and exporter adapters record only the databases `externalDependencies.consensus` declares; see [Configuring Alerting](../../alerting/configuring/#recorded-series).

Recorded-series names are covered by the [stability policy](../stability/) from the release that first ships them, like alert names.

{{% list-records %}}
