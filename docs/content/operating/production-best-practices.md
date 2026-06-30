---
title: "Production Best Practices"
weight: 10
---

# Production Best Practices

This is the production checklist for the bundled **logging backend (Loki)** and the Alloy pipeline that feeds it.
Sizing is **throughput-driven**, and every item is tagged with who owns it under the [shared responsibility model](#shared-responsibility-model).

For the architecture these items configure, see [Logs & Events](../../logs-and-events/).

## Shared responsibility model

Four parties share responsibility for a production deployment.
Checklist items below are tagged with the **primary** owner.

| Tag | Party | Owns |
|---|---|---|
| `[upstream]` | **Upstream service** (Loki, Alloy, Grafana) | component behavior: the ring, WAL, compaction, query engine, defaults |
| `[chart]` | **`materialize-monitoring` chart** | topology and wiring, opinionated defaults, config rendering, validation initContainers, shipped dashboards/alerts, the profiles |
| `[consumer]` | **Chart consumer** (Terraform / Pulumi / ArgoCD / FluxCD) | cloud resources (bucket, IAM/IRSA, StorageClass, DNS), secret provisioning, version pinning, selecting the profile |
| `[operator]` | **Human operator** (end-user) | size selection, retention budget, tenant policy, incident response, day-2 procedures, capacity watch |

| Area | Upstream | Chart | Consumer | Operator |
|---|---|---|---|---|
| Component logic (ring, WAL, compaction, query) | **owns** | configures | — | — |
| Topology, defaults, validation, dashboards/alerts | — | **owns** | selects profile | — |
| Cloud resources: bucket, IAM/IRSA, StorageClass | — | consumes by name | **owns** | approves |
| Secrets provisioning | — | consumes by name | **provides** | rotates |
| Size, retention budget, tenant policy | — | offers profiles | sets values | **decides** |
| Incident response, upgrades, DR, capacity | — | tooling + alerts | applies changes | **owns** |

## Sizing the logging backend

**Size by throughput and burst, not by stored-bucket size.**
Bucket size is a *derived* output (sustained throughput × retention), not an input.
Different parts of Loki are sized off different points of the load envelope:

- **Ingest path** (distributors, ingesters, WAL, ingestion limits) → the **5-minute burst**, with headroom to the regression ceiling.
- **Storage / retention / bucket** → **sustained** throughput.
- **Read path** (frontend, queriers, caches) → query load, independent of ingest.

The 5-minute burst typically runs several times the peak-hour rate, so averages must not drive sizing.
The three tiers are defined by the ingest envelope:

| Size | Sustained (peak-hour) | 5-min burst | Regression ceiling | Typical fit |
|---|---|---|---|---|
| **S** | ~0.25 MiB/s | ~1 MiB/s | ~2 MiB/s | dev / staging |
| **M** | ~0.75 MiB/s | ~3 MiB/s | ~8 MiB/s | mid-size |
| **L** | ~2 MiB/s | ~6 MiB/s | ~17 MiB/s | production SaaS / fleet |

> [!INFO]
>   The **regression ceiling** is the burst you must degrade gracefully against — typically the volume *before* Alloy's cardinality/series reduction is applied.
>   Size the ingest path so a reduction regression throttles via limits rather than crashing the fleet.

### Measure to validate

Size and re-measure off the distributor — the same signal sizing is derived from. `[operator]`

```promql
# sustained (size storage + retention to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[1h]))[7d:1h] )

# 5-min burst (size ingest path + WAL + limits to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[5m]))[7d:5m] )
```

If the 5-minute figure climbs toward the regression ceiling, that is the signal to add ingesters (see [§4](#4-ingester-durability--rollouts)) before it bites.

### Per-size resources

Starting points — `replicas × (cpu request / memory request)`; tune from real usage. `[chart]` defaults, `[operator]`/`[consumer]` override per profile.

| Component | S | M | L |
|---|---|---|---|
| Distributor (stateless) | 2 × (100m / 128Mi) | 2 × (150m / 256Mi) | 3 × (500m / 512Mi) |
| **Ingester** (RF 3) | 3 × (250m / 512Mi), WAL 4Gi | 3 × (500m / 1Gi), WAL 8Gi | 3–6 × (1–2 / 4–8Gi), WAL 32Gi |
| Querier (stateless) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) | 3 × (1 / 1–2Gi) |
| Query-frontend | 2 × (100m / 128Mi) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) |
| Query-scheduler | omit | omit | optional 2 × (100m / 256Mi) |
| Index-gateway | 1 × (100m / 256Mi) | 2 × (200m / 512Mi) | 2 × (500m / 1Gi), ring mode |
| Compactor (singleton) | 1 × (100m / 256Mi) | 1 × (250m / 512Mi) | 1 × (1 / 2Gi) |
| Ruler (if log rules) | 2 × (100m / 256Mi) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) |
| memcached-chunks | 1 × 256Mi | 2 × 512Mi | 2–3 × 2Gi |
| memcached-results | 1 × 128Mi | 1 × 256Mi | 2 × 512Mi |
| memcached-index | share results | 1 × 256Mi | 2 × 1Gi |

- Ingesters = **3 minimum** at every size (the `replication_factor` 3 floor), and the **compactor is always a singleton**.
- **Scale ingesters past 3 on memory/cardinality, not bytes** — with N = RF every ingester holds every stream; run N > RF to shard streams and spread the burst.
- **Do not set a tight memory limit on ingesters** — an OOM-kill drops WAL-buffered logs. Use generous limits (or none) and alert on usage.

### Protective limits

Per **tenant** (per environment). The aggregate burst is a fleet-capacity concern handled by ingester count, not by these. `[operator]` sets per profile.

| Size | `ingestion_rate_mb` | `ingestion_burst_size_mb` | `max_global_streams_per_user` |
|---|---|---|---|
| S | 4 | 8 | 5,000 |
| M | 8 | 16 | 10,000 |
| L | 16 | 32 | 25,000 |

## Checklist

### 1. Topology & sizing

- [ ] `[operator]` Select the profile/size from the table above; record the measured sustained + 5-min burst it is based on.
- [ ] `[chart]` Microservice/distributed mode; ingesters ≥ 3; compactor = 1 singleton.
- [ ] `[operator]` Skip query-scheduler for S/M; consider it only at L.
- [ ] `[chart]`/`[operator]` Ingester memory limit generous or unset; requests ≈ limits for stateless components.

### 2. Schema & storage

- [ ] `[chart]` `schema_config`: TSDB, schema **v13**, 24h index period.
- [ ] `[consumer]` Provision the object-storage bucket (S3-compatible / GCS / Azure Blob); single bucket, prefixes `/loki/chunks`, `/loki/ruler`.
- [ ] `[consumer]` Object-store lifecycle policy aligned with (or longer than) Loki retention, so the compactor owns deletion.
- [ ] `[operator]` Treat schema periods as **append-only** — future format changes go in a new period with a `from` date ahead of now, never by editing a past period.

### 3. Replication, ring & placement

- [ ] `[chart]` `replication_factor: 3`; ring backend = memberlist (no Consul/etcd).
- [ ] `[chart]`/`[operator]` Zone-aware replication + `topologySpreadConstraints` so the 3 ingester replicas land in 3 zones/nodes.
- [ ] `[chart]` PodDisruptionBudget on ingesters (`maxUnavailable: 1`).
- [ ] `[operator]` `priorityClassName` so ingesters/compactor are not evicted under node pressure.

### 4. Ingester durability & rollouts

- [ ] `[chart]` WAL enabled on its own PVC (4Gi S / 8Gi M / 32Gi L); `wal.replay_memory_ceiling` set to the memory request.
- [ ] `[chart]` `flush_on_shutdown: true` and `terminationGracePeriodSeconds` long enough to flush (300–600s).
- [ ] `[chart]` StatefulSet rolls ingesters **one at a time** (`maxUnavailable: 1`).
- [ ] `[consumer]` A dynamic-provisioning **StorageClass** exists (CSI driver installed) — not safe to assume on bare clusters.
- [ ] `[operator]` Add ingesters (N > RF) when per-ingester memory/stream-count climbs or to spread the regression burst.

### 5. Limits & cardinality

- [ ] `[operator]` Set per-tenant `ingestion_rate_mb` / `ingestion_burst_size_mb` / `max_global_streams_per_user` from the table; remember these are per environment.
- [ ] `[chart]` `reject_old_samples: true` + a max age; `max_line_size` matching the gateway drop policy.
- [ ] `[chart]` The Alloy gateway keeps the label set small and routes high-cardinality fields to structured metadata — the dominant cost/stability lever.
- [ ] `[operator]` Alert on `loki_discarded_samples_total` so a limit hit is visible.

### 6. Retention & compaction

- [ ] `[chart]` Compactor `retention_enabled: true`, `compaction_interval`, `delete_request_store` configured.
- [ ] `[operator]` Set the global `retention_period` to the storage budget.
- [ ] `[operator]` Configure **tiered (per-stream) retention** — keep `ERROR`/audit streams long, expire high-volume `INFO` fast.

### 7. Caching

- [ ] `[chart]` Results cache (query-frontend), chunks cache, and index/stats cache on the bundled memcached.
- [ ] `[chart]` `cache_results: true`, `max_cache_freshness_per_query: 10m`.
- [ ] `[operator]` Size memcached per the table.

### 8. Read path

- [ ] `[chart]` Query-frontend ≥ 2 for queue fairness; `split_queries_by_interval` and `tsdb_max_query_parallelism` set.
- [ ] `[chart]` Grafana Loki datasource provisioned, pointing at the **query-frontend** Service (no nginx loki-gateway).
- [ ] `[operator]` Scale queriers/frontends — not ingesters — when dashboards feel slow.

### 9. Tenancy & auth

- [ ] `[chart]` **One logical tenant per install.** `auth_enabled: true` with a single named `X-Scope-OrgID` (not the implicit `fake` tenant), so a future split is config — not a data migration, since the tenant ID is baked into the object-storage path.
- [ ] `[operator]` Isolation is **label-based** within the tenant (`environment_id`, …); the **hard isolation boundary is the install** (per region/stack). Fine for trusted internal consumers — revisit if per-team or customer-facing access is required (then Grafana LBAC, or tenant-per-environment writes + multi-tenant reads).
- [ ] `[operator]` Per-environment controls are label-based, not tenant-based: per-stream retention (`environment_id`) and per-label rate limits — both **static config**, which is why no `runtime_config` live reload is needed.
- [ ] `[operator]` Watch per-ingester memory against total fleet stream count — one tenant concentrates cardinality, bringing the N > RF lever ([§4](#4-ingester-durability--rollouts)) forward.
- [ ] `[consumer]` Provide the basic-auth (or mTLS) Secret **by name**; the chart consumes, it does not mint.

### 10. Meta-monitoring

- [ ] `[chart]` ServiceMonitor/PodMonitor (or GCP `PodMonitoring`) for every Loki component.
- [ ] `[chart]` Loki mixin dashboards + alerts installed; optional `loki-canary` for end-to-end write→read verification.
- [ ] `[operator]` Tier-0 alerts wired to paging: ingester unhealthy/flush failures, compactor not running, discarded samples, object-store errors, WAL disk usage. **Loki down is its own incident.**

### 11. Security & credentials

- [ ] `[consumer]` Object-store access via **workload identity** (IRSA / GKE WI / Azure WI); static keys only as a documented escape hatch.
- [ ] `[consumer]` No long-lived credentials in the chart; storage secret by reference.
- [ ] `[chart]` `runAsNonRoot`, read-only root filesystem, dropped capabilities on all components.
- [ ] `[operator]` Optional inter-component TLS where the cluster requires it.

### 12. Day-2: upgrades, migration, DR

- [ ] `[operator]` Upgrade ingesters one-at-a-time with flush (see [§4](#4-ingester-durability--rollouts)).
- [ ] `[operator]` Schema changes = new period, never in place (see [§2](#2-schema--storage)).
- [ ] `[consumer]`/`[operator]` DR = object versioning + cross-region replication + (for audit) Object Lock/WORM; restore = repoint at the bucket. No native snapshot — see [Storing](../../logs-and-events/storing/).
- [ ] `[consumer]` Pin the Loki chart/image version; upgrades are deliberate.

### 13. Validation

- [ ] `[chart]` `loki -verify-config` runs in CI and as an initContainer before a component serves.
- [ ] `[chart]` `helm template | kubeconform` + `helm lint` in CI.
- [ ] `[chart]` Smallest integration profile = single-binary + filesystem (no object store) for hermetic e2e tests.

## See also

- [Logs & Events](../../logs-and-events/) — the logging architecture these items configure.
- [Storing](../../logs-and-events/storing/) — storage, retention, and disaster recovery in depth.
- [Upgrading](../upgrading/) — cross-cutting upgrade guidance.
- [Loki production deployment](https://github.com/grafana/loki/tree/main/production/ksonnet/loki) (official) — Grafana's reference production config (built for far larger volumes; read it for the patterns, not the magnitudes).
