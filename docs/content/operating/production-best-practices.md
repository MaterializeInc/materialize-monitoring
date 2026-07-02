---
title: "Production Best Practices"
weight: 10
---

# Production Best Practices

Production guidance for the `materialize-monitoring` stack, organized by backend.
Every checklist item is tagged with its **primary owner** under the [shared responsibility model](#shared-responsibility-model), and is checked (`[x]`) when the chart already ships it as a default — unchecked items are the deployment-time actions (or still-to-build chart work) that remain.

Today this covers the bundled **logging backend (Loki)**; metrics (Thanos), Grafana, and Alertmanager sections will follow the same shape.

## Shared responsibility model

Four parties share responsibility for a production deployment.
Checklist items are tagged with the **primary** owner.

| Tag | Party | Owns |
|---|---|---|
| `[upstream]` | **Upstream service** (Loki, Alloy, Grafana) | component behavior: the ring, WAL, compaction, query engine, defaults |
| `[chart]` | **`materialize-monitoring` chart** | topology and wiring, opinionated defaults, config rendering, validation, shipped dashboards/alerts, the profiles |
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

## Logging (Loki)

For the architecture these items configure, see [Logs & Events](../../logs-and-events/).

### Sizing the logging backend

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

#### Measure to validate

Size and re-measure off the distributor — the same signal sizing is derived from. `[operator]`

```promql
# sustained (size storage + retention to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[1h]))[7d:1h] )

# 5-min burst (size ingest path + WAL + limits to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[5m]))[7d:5m] )
```

If the 5-minute figure climbs toward the regression ceiling, that is the signal to add ingesters (see [§4](#4-ingester-durability--rollouts)) before it bites.

#### Per-size resources

Starting points — `replicas × (cpu request / memory request)`; tune from real usage. `[chart]` defaults, `[operator]`/`[consumer]` override per profile.

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

- Ingesters = **3 minimum** at every size (the `replication_factor` 3 floor), and the **compactor is always a singleton**.
- **Scale ingesters past 3 on memory/cardinality, not bytes** — with N = RF every ingester holds every stream; run N > RF to shard streams and spread the burst.
- **Ingesters are ephemeral** (node-local `emptyDir`, no PVC) — durability is `replication_factor` 3, not disk. This makes ingesters freely reschedulable and sidesteps EBS zonal pinning / slow volume reattach on node replacement.
- **Do not set a tight memory limit on ingesters** — an OOM-kill drops in-memory/WAL-buffered logs. Use generous limits (or none) and alert on usage.

#### Protective limits

Per **tenant** (per environment). The aggregate burst is a fleet-capacity concern handled by ingester count, not by these. `[operator]` sets per profile.

| Size | `ingestion_rate_mb` | `ingestion_burst_size_mb` | `max_global_streams_per_user` |
|---|---|---|---|
| S | 4 | 8 | 5,000 |
| M | 8 | 16 | 10,000 |
| L | 16 | 32 | 25,000 |

### Checklist

#### 1. Topology & sizing

- [ ] `[operator]` Select the profile/size from the table above; record the measured sustained + 5-min burst it is based on.
- [x] `[chart]` Microservice/distributed mode; ingesters ≥ 3; compactor = 1 singleton.
- [ ] `[operator]` Query-scheduler is enabled by default at **M/L**; the **small** profile omits it (the query-frontend's own queue suffices there).
- [x] `[chart]` Ingester memory limit left unset; requests set for scheduling.

#### 2. Schema & storage

- [x] `[chart]` `schema_config`: TSDB, schema **v13**, 24h index period.
- [ ] `[consumer]` Provision the object-storage bucket (S3-compatible / GCS / Azure Blob); single bucket, prefixes `/loki/chunks`, `/loki/ruler`.
- [ ] `[consumer]` Object-store lifecycle policy aligned with (or longer than) Loki retention, so the compactor owns deletion.
- [ ] `[operator]` Treat schema periods as **append-only** — future format changes go in a new period with a `from` date ahead of now, never by editing a past period.

#### 3. Replication, ring & placement

- [x] `[chart]` `replication_factor: 3`; ring backend = memberlist (no Consul/etcd).
- [x] `[chart]` Ingester `topologySpreadConstraints`: **hard across zones** (`DoNotSchedule`) so an un-spread pod goes Pending and Karpenter provisions the missing zone; **soft across hosts** (`ScheduleAnyway`), with the chart's default hard per-host anti-affinity dropped by nulling its rule list.
- [ ] `[operator]` Aim for **≥3 zones** for true AZ resilience under RF 3; set `minDomains` to the zone count your node pool can launch in. With 2 zones, know an AZ loss can break write quorum until the ring recovers.
- [ ] `[operator]` If you bring nodes up **tainted** until DaemonSets are healthy, the spread already sets `nodeTaintsPolicy: Honor` (tainted nodes stay out of the skew math) — model the taint as a Karpenter `startupTaint` so it doesn't over-provision. Taints only gate placement.
- [x] `[chart]` PodDisruptionBudget on ingesters (`maxUnavailable: 1`), with a render-time warning if it or the rollout `maxUnavailable` is set > 1.
- [ ] `[operator]` `priorityClassName` so ingesters/compactor are not evicted under node pressure.

#### 4. Ingester durability & rollouts

- [x] `[chart]` **Ingesters are ephemeral** — node-local `emptyDir`, no PVC. Durability is `replication_factor` 3, so a killed/rescheduled ingester's un-flushed data is recovered from its peers.
- [x] `[chart]` **Index-gateway and compactor are also ephemeral** — their local disk is a read-through cache / idempotent working copy of the object-store index, so they reschedule freely across zones (the compactor singleton in particular is never PVC-pinned to one AZ).
- [x] `[chart]` `flush_on_shutdown: true` (best-effort) with a **modest** `terminationGracePeriodSeconds` (~60s). Do **not** rely on a long grace period — enterprise force-kill windows (120/300s) are harmless because replication covers a truncated flush.
- [x] `[chart]` StatefulSet rolls ingesters **one at a time** (`maxUnavailable: 1`); a burst rollout needs `zoneAwareReplication` (zone-at-a-time) or the alpha `MaxUnavailableStatefulSet` gate — neither is in play, and PDBs govern *drains*, not rollout speed.
- [ ] `[operator]` **Budget the roll and raise the deploy timeout.** The serial, readiness-gated ingester roll takes **~1 min per ingester** (so a 6-ingester roll ≈ 5 min) and is not bounded by node provisioning — it overruns Helm's default 5-min `--wait`. Set `helm upgrade --timeout` (or Flux `spec.timeout` / Pulumi `customTimeouts`; ArgoCD is async and tolerant). A wait-timeout here means "still rolling," not "failed." See [Upgrading](../upgrading/).
- [ ] `[operator]` Add ingesters (N > RF) when per-ingester memory/stream-count climbs or to spread the regression burst — streams shard across the ring only when N > RF.
- [ ] `[consumer]` A dynamic-provisioning **StorageClass** still needs to exist for the **one PVC-backed component, the ruler** (it keeps a PVC for its remote-write WAL) — CSI driver installed, not safe to assume on bare clusters.

#### 5. Limits & cardinality

- [ ] `[operator]` Set per-tenant `ingestion_rate_mb` / `ingestion_burst_size_mb` / `max_global_streams_per_user` from the table; remember these are per environment.
- [x] `[chart]` `reject_old_samples: true` + `reject_old_samples_max_age` set.
- [ ] `[chart]` The Alloy gateway keeps the label set small and routes high-cardinality fields to structured metadata — the dominant cost/stability lever. *(Gateway pipeline in flight.)*
- [ ] `[operator]` Alert on `loki_discarded_samples_total` so a limit hit is visible.

#### 6. Retention & compaction

- [x] `[chart]` Compactor `retention_enabled: true`, `delete_request_store`, `retention_delete_delay` configured.
- [ ] `[operator]` Set the global `retention_period` to the storage budget.
- [ ] `[operator]` Configure **tiered (per-stream) retention** — keep `ERROR`/audit streams long, expire high-volume `INFO` fast.

#### 7. Caching

- [x] `[chart]` Results cache (query-frontend), chunks cache, and index/stats cache on the bundled memcached.
- [x] `[chart]` Query-result caching enabled (`cache_results`, `max_cache_freshness_per_query`).
- [ ] `[operator]` Size memcached per the table.

#### 8. Read path

- [x] `[chart]` Query-frontend ≥ 2 for queue fairness; query splitting/parallelism configured.
- [ ] `[chart]` Grafana Loki datasource provisioned, pointing at the **query-frontend** Service (bundled nginx loki-gateway is off; datasource wiring still to land).
- [ ] `[operator]` Scale queriers/frontends — not ingesters — when dashboards feel slow.

#### 9. Tenancy & auth

- [ ] `[chart]` **One logical tenant per install:** `auth_enabled: true` with a single named `X-Scope-OrgID` (not the implicit `fake` tenant), so a future split is config — not a data migration, since the tenant ID is baked into the object-storage path. *(Decision recorded; not yet wired into values.)*
- [ ] `[operator]` Isolation is **label-based** within the tenant (`environment_id`, …); the **hard isolation boundary is the install** (per region/stack). Fine for trusted internal consumers — revisit if per-team or customer-facing access is required (then Grafana LBAC, or tenant-per-environment writes + multi-tenant reads).
- [ ] `[operator]` Per-environment controls are label-based, not tenant-based: per-stream retention (`environment_id`) and per-label rate limits — both **static config**, which is why no `runtime_config` live reload is needed.
- [ ] `[operator]` Watch per-ingester memory against total fleet stream count — one tenant concentrates cardinality, bringing the N > RF lever ([§4](#4-ingester-durability--rollouts)) forward.
- [ ] `[consumer]` Provide the basic-auth (or mTLS) Secret **by name**; the chart consumes, it does not mint.

#### 10. Meta-monitoring

- [ ] `[chart]` ServiceMonitor/PodMonitor (or GCP `PodMonitoring`) for every Loki component.
- [x] `[chart]` `loki-canary` enabled for end-to-end write→read verification.
- [ ] `[chart]` Loki mixin dashboards + alerts installed.
- [ ] `[operator]` Tier-0 alerts wired to paging: ingester unhealthy/flush failures, compactor not running, discarded samples, object-store errors, disk usage. **Loki down is its own incident.**

#### 11. Security & credentials

- [ ] `[consumer]` Object-store access via **workload identity** (IRSA / GKE WI / Azure WI) — see [Storing > Granting object-storage access](../../logs-and-events/storing/#granting-object-storage-access-workload-identity) for the per-provider setup; static keys only as a documented escape hatch.
- [ ] `[consumer]` No long-lived credentials in the chart; storage secret by reference.
- [ ] `[chart]` `runAsNonRoot`, read-only root filesystem, dropped capabilities on all components.
- [ ] `[operator]` Optional inter-component TLS where the cluster requires it.

##### NetworkPolicy egress (if `networkPolicy.enabled`)

Enabling the NetworkPolicy denies egress by default except what it explicitly allows. Loki needs **external** egress that the base policy does *not* grant, so you must permit it or Loki hangs.

- [x] `[chart]` Egress to **object storage AND the credential endpoint** on 443 via `networkPolicy.externalStorage` (default `ports: [443]`, `cidrs: ["0.0.0.0/0"]`). Tighten `cidrs` per environment (see below).
- [ ] `[operator]` The credential path must be covered, not just the bucket: **IRSA fetches credentials from AWS STS on 443** (GKE/Azure WI have their own token endpoints). Missing STS egress is the classic failure — the compactor blocks at startup fetching credentials for the delete-requests store, its HTTP server never serves, the liveness probe kills it every ~5 min, and it crashloops with misleading `memberlist … WriteTo … i/o timeout` noise. The block is silent (a hanging TCP connect), so it looks like anything *but* a network policy.
- [ ] `[operator]` **Do not rely on an ambient broad-443 egress rule from another workload** (e.g. an application-level "reach the external kube API server" rule). It won't select the Loki namespace, and even where it does it's a load-bearing coincidence: the day someone scopes that rule down, Loki breaks. Declare Loki's egress explicitly.
- [ ] `[operator]` **Tighten for production:** prefer **VPC endpoints (PrivateLink) for S3 + STS** and scope `cidrs` to the VPC / endpoint CIDRs — `0.0.0.0/0:443` is a reasonable default only when egress is already governed at the infra layer (SGs, NAT, egress firewall). On Cilium, FQDN egress (`toFQDNs`) to the S3/STS hostnames is a good tight-but-not-brittle alternative.

#### 12. Day-2: upgrades, migration, DR

- [ ] `[operator]` Upgrade ingesters one-at-a-time with flush (see [§4](#4-ingester-durability--rollouts)).
- [ ] `[operator]` Schema changes = new period, never in place (see [§2](#2-schema--storage)).
- [ ] `[consumer]`/`[operator]` DR = object versioning + cross-region replication + (for audit) Object Lock/WORM; restore = repoint at the bucket. No native snapshot — see [Storing](../../logs-and-events/storing/).
- [ ] `[consumer]` Pin the Loki chart/image version; upgrades are deliberate.

#### 13. Validation

- [ ] `[chart]` `loki -verify-config` runs in CI and as an initContainer before a component serves.
- [ ] `[chart]` `helm template | kubeconform` + `helm lint` in CI.
- [x] `[chart]` Smallest integration profile = single-binary + filesystem (no object store) for hermetic e2e tests.

### See also

- [Logs & Events](../../logs-and-events/) — the logging architecture these items configure.
- [Storing](../../logs-and-events/storing/) — storage, retention, and disaster recovery in depth.
- [Upgrading](../upgrading/) — cross-cutting upgrade guidance.
- [Loki production deployment](https://github.com/grafana/loki/tree/main/production/ksonnet/loki) (official) — Grafana's reference production config (built for far larger volumes; read it for the patterns, not the magnitudes).
