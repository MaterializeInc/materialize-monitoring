---
title: "Logging (Loki)"
weight: 5
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Logging (Loki)

This page explains how the bundled Loki is sized and configured, and why.
For the architecture these settings configure, see [Logs and events](../../../logs-and-events/architecture/).
What to do about them is in the [production checklist](../checklist/).

## Sizing the logging backend {#sizing-the-logging-backend}

**Size by throughput and burst, not by stored-bucket size.**
Bucket size is a *derived* output (sustained throughput × retention), not an input.
Different parts of Loki are sized off different points of the load envelope:

- **Ingest path** (distributors, ingesters, WAL, ingestion limits) → the **5-minute burst**, with headroom to the regression ceiling.
- **Storage / retention / bucket** → **sustained** throughput.
- **Read path** (frontend, queriers, caches) → query load, independent of ingest.

The 5-minute burst typically runs several times the peak-hour rate, so averages must not drive sizing.
The ingest envelope defines the three sizes:

| Size | Sustained (peak-hour) | 5-min burst | Regression ceiling | Typical fit |
|---|---|---|---|---|
| **S** | ~0.25 MiB/s | ~1 MiB/s | ~2 MiB/s | dev / staging |
| **M** | ~0.75 MiB/s | ~3 MiB/s | ~8 MiB/s | mid-size |
| **L** | ~2 MiB/s | ~6 MiB/s | ~17 MiB/s | production SaaS / fleet |

> [!INFO]
>   The **regression ceiling** is the burst you must degrade gracefully against — typically the volume *before* Alloy's
>   cardinality/series reduction is applied.
>   Size the ingest path so a reduction regression throttles via limits rather than crashing the fleet.

### Measure to validate {#measure-to-validate}

Size and re-measure off the distributor, which is the same signal the sizing is derived from:

```promql
# sustained (size storage + retention to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[1h]))[7d:1h] )

# 5-min burst (size ingest path + WAL + limits to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[5m]))[7d:5m] )
```

If the 5-minute figure climbs toward the regression ceiling, add ingesters before it bites.
For why that helps, see [Ingester durability and rollouts](#ingester-durability).

### Per-size resources {#per-size-resources}

The following table gives starting points as `replicas × (cpu request / memory request)`; tune them from real usage.
The chart's defaults are the M column, and the `loki-small` and `loki-large` profiles carry the others.

| Component | S | M | L |
|---|---|---|---|
| Distributor (stateless) | 2 × (100m / 128Mi) | 2 × (150m / 256Mi) | 3 × (500m / 512Mi) |
| **Ingester** (RF 3, ephemeral) | 3 × (250m / 512Mi) | 3 × (500m / 1Gi) | 3–6 × (1–2 / 4–8Gi) |
| Querier (stateless) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) | 3 × (1 / 1–2Gi) |
| Query-frontend | 2 × (100m / 128Mi) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) |
| Query-scheduler | omit | 2 × (100m / 256Mi) | 2 × (100m / 256Mi) |
| Index-gateway | 1 × (100m / 256Mi) | 2 × (200m / 512Mi) | 2 × (500m / 1Gi), ring mode |
| Compactor (singleton) | 1 × (100m / 256Mi) | 1 × (250m / 512Mi) | 1 × (1 / 2Gi) |
| Ruler (if log rules) | 2 × (100m / 256Mi) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) |
| memcached-chunks | 1 × 256Mi | 2 × 512Mi | 2–3 × 2Gi |
| memcached-results | 1 × 128Mi | 1 × 256Mi | 2 × 512Mi |
| memcached-index | share results | 1 × 256Mi | 2 × 1Gi |

The following rules hold at every size:

- **Ingesters are 3 minimum**, the `replication_factor` 3 floor, and the **compactor is always a singleton**.
- **Scale ingesters past 3 on memory and cardinality, not bytes.**
  With N = RF every ingester holds every stream; run N > RF to shard streams and spread the burst.
- **Don't set a tight memory limit on ingesters.**
  An OOM-kill drops in-memory and WAL-buffered logs.
  The chart leaves the limit unset and sets requests for scheduling; alert on usage instead.
- **The query-scheduler is omitted at S**, where the query-frontend's own queue suffices.

### Protective limits {#protective-limits}

These limits apply per **tenant**, which is per environment.
The aggregate burst is a fleet-capacity concern handled by ingester count, not by these.

| Size | `ingestion_rate_mb` | `ingestion_burst_size_mb` | `max_global_streams_per_user` |
|---|---|---|---|
| S | 4 | 8 | 5,000 |
| M | 8 | 16 | 10,000 |
| L | 16 | 32 | 25,000 |

## Ingester durability and rollouts {#ingester-durability}

**Ingesters are ephemeral**: node-local `emptyDir`, with no PVC.
Durability comes from `replication_factor` 3, not from disk, so a killed or rescheduled ingester's unflushed data is recovered from its peers.
That makes ingesters freely reschedulable, and sidesteps EBS zonal pinning and slow volume reattach on node replacement.

The index-gateway and the compactor are ephemeral too.
Their local disk is a read-through cache, or an idempotent working copy of the object-store index, so they reschedule freely across zones.
The compactor singleton in particular is never PVC-pinned to one zone.

Ingesters set `flush_on_shutdown: true` as a best effort, with a **modest** `terminationGracePeriodSeconds` of about 60s.
Don't rely on a long grace period.
Enterprise force-kill windows of 120s or 300s are harmless, because replication covers a truncated flush.

The StatefulSet rolls ingesters **one at a time** (`maxUnavailable: 1`).
A faster rollout needs `zoneAwareReplication`, which rolls a zone at a time, or the alpha `MaxUnavailableStatefulSet` gate.
Neither is in play, and PDBs govern *drains*, not rollout speed.
The serial, readiness-gated roll takes about **1 minute per ingester**, so a 6-ingester roll takes about 5 minutes.
It isn't bounded by node provisioning, and it overruns Helm's default 5-minute `--wait`, so a wait timeout here means "still rolling", not "failed".
For the timeout settings on each deploy tool, see [Upgrading](../../upgrading/).

Streams shard across the ring only when there are more ingesters than the replication factor.
Add ingesters when per-ingester memory or stream count climbs, or to spread the regression burst.

## Schema periods {#schema-periods}

The chart's `schema_config` uses TSDB, schema **v13**, and a 24h index period.
Treat schema periods as **append-only**.
A future format change goes in a new period with a `from` date ahead of now, never in an edit to a past period.

On any object store other than S3, the backend has to be named in three places that the chart's S3-shaped defaults
otherwise fill: `storage.object_store.type`, the newest `schemaConfig` period, and `compactor.delete_request_store`.
A stale one crash-loops the component that reads it with `no s3 endpoint in config file` rather than degrading.
A render-time check refuses a mismatched set and warns on the inert fourth, `storage.type`.
For more information, see [Selecting the backend](../../../logs-and-events/storing/#selecting-the-backend).

## Tenancy and isolation {#tenancy}

Each install is **one logical tenant**: `auth_enabled: true` with a single named `X-Scope-OrgID`, not the implicit `fake` tenant.
The tenant ID is baked into the object-storage path, so naming it now makes a future split a config change rather than a data migration.

Isolation within the tenant is **label-based**, by `environment_id` and similar labels, and the **hard isolation
boundary is the install**, per region or stack.
That's fine for trusted internal consumers.
Per-team or customer-facing access needs more: Grafana label-based access control, or tenant-per-environment writes with multi-tenant reads.

Per-environment controls are label-based too, not tenant-based: per-stream retention by `environment_id`, and per-label rate limits.
Both are static config, which is why Loki needs no `runtime_config` live reload.

One tenant concentrates cardinality.
Watch per-ingester memory against the total fleet stream count, because it brings the N > RF lever in [Ingester
durability and rollouts](#ingester-durability) forward.

## Object-storage egress {#object-storage-egress}

Enabling Loki's NetworkPolicy denies egress except what it explicitly allows.
Loki needs **external** egress to reach object storage, which the base policy doesn't grant, so the chart adds it
through `networkPolicy.externalStorage`, by default `ports: [443]` and `cidrs: ["0.0.0.0/0"]`.

That rule has to cover the credential path as well as the bucket.
**IRSA fetches credentials from AWS STS on 443**, and GKE and Azure workload identity have their own token endpoints.
Missing STS egress is the classic failure.
The compactor blocks at startup fetching credentials for the delete-requests store, so its HTTP server never serves.
The liveness probe kills it about every 5 minutes, and it crash-loops with misleading `memberlist … WriteTo … i/o timeout` noise.
The block is silent, a hanging TCP connect, so it looks like anything *but* a network policy.

Don't rely on an ambient broad-443 egress rule from another workload, such as an application-level rule for reaching an
external Kubernetes API server.
It won't select the Loki namespace.
Even where it does, it's a load-bearing coincidence, and Loki breaks the day someone scopes that rule down.
Declare Loki's egress explicitly.

`0.0.0.0/0:443` is a reasonable default only where egress is already governed at the infrastructure layer, by security
groups, NAT, or an egress firewall.
For production, prefer **VPC endpoints (PrivateLink) for S3 and STS**, and scope `cidrs` to the VPC or endpoint CIDRs.
On Cilium, FQDN egress (`toFQDNs`) to the S3 and STS hostnames is a tight alternative that isn't brittle.

## See also

- [Logs and events](../../../logs-and-events/architecture/): the logging architecture these settings configure.
- [Storing](../../../logs-and-events/storing/): storage, retention, and disaster recovery in depth.
- [Upgrading](../../upgrading/): cross-cutting upgrade guidance.
- [Loki production deployment](https://github.com/grafana/loki/tree/main/production/ksonnet/loki) (official): Grafana's reference production config.
  It's built for far larger volumes, so read it for the patterns, not the magnitudes.
