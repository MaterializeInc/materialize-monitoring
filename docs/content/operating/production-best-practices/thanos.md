---
title: "Metrics (Thanos)"
weight: 6
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Metrics (Thanos)

This page explains how the bundled Thanos is replicated, sized, spread, and stored, and why.
For the architecture these settings configure, see [Metrics architecture](../../../metrics/architecture/).
What to do about them is in the [production checklist](../checklist/).

## Receive replication {#receive-replication}

Replication, not `receive.mode`, is the availability lever.
It's the single most consequential setting, and the one most often misread.

`receive.mode` is a **topology** choice:

- **`standalone`** (default) is *RouterIngestor* mode — one workload that both routes and ingests.
  It still builds a ketama hashring across the StatefulSet pods (`receive.hashrings.autogen`), so `replicaCount: 3` shards writes across three pods.
- **`split`** separates Router (Deployment) from Ingester (StatefulSet) so they scale independently.

Neither choice, by itself, gives you redundancy.
**The replication factor does.**
The subchart passes `--receive.replication-factor` only in split mode, so standalone would run at Thanos's default of **1**.
This chart sets it to 3 through `receive.extraArgs` instead, which is the shape the following example shows.

Thanos write quorum is `(rf / 2) + 1`:

| Replication factor | Write quorum | Ingester losses tolerated |
|---:|---:|---:|
| 1 | 1 | 0 |
| **2** | **2** | **0** |
| 3 | 2 | 1 |
| 4 | 3 | 1 |
| 5 | 3 | 2 |

**Use an odd factor.**
A factor of 2 requires both copies and tolerates nothing — the same as 1, and on a small ring *worse*: on three pods,
losing one fails ~1/3 of series at RF 1 but ~2/3 at RF 2, because more series depend on any given pod while quorum still demands all of their copies.
Even factors above 2 tolerate no more losses than the odd factor below them; the extra copy buys durability, not availability.

```yaml
thanos:
  receive:
    replicaCount: 3
    extraArgs:
      - --receive.replication-factor=3
```

This is the same shape as Loki, where ingesters are deliberately ephemeral and durability comes from RF 3 alone —
**Thanos Receive is `emptyDir`-backed for the same reason**, and the replication factor is therefore the only thing
standing between you and data loss.
It holds up to `receive.tsdb.retention` (6h) of blocks locally, of which at most 2h has not yet reached object storage.
With RF 1 that window exists in exactly one copy, on one node.
See [Storage: ephemeral by default](#thanos-ephemeral-storage) for why no volume is involved.

> [!WARNING]
>   **Split mode isn't recommended.**
>   `receive.ingester` doesn't inherit from the top-level `receive.*` defaults, and upstream considers the non-merging behavior intentional.
>   In practice that means restating about 31 keys, of which the values schema hard-requires eight sub-objects
>   (`hashrings`, `service`, `vpa`, `persistence`, `podSecurityContext`, `probes`, `serviceMonitor`, `pdb`) before the chart renders at all.
>   Prefer standalone with an odd replication factor.

Every shipped profile restates `receive.extraArgs` in full so the factor survives, and a unit test pins that at all three sizes.
If you override the list yourself, read the warning in [Protective limits](#thanos-protective-limits) first.

## Sizing the metrics backend {#sizing-the-metrics-backend}

**Size by active series and object count — not by node count, and not by throughput.**
This is the opposite axis from Loki, where bytes per second drives everything.
Thanos keys off cardinality, and the derived quantities split across components:

- **Receive memory** → **active series**.
  Budget ~3 KB per series held.
- **Receive CPU and local disk** → **ingested samples/sec**, which is active series ÷ scrape interval.
  Every scrape surface this chart generates defaults to **30s**.
- **Store Gateway and Compactor** → **block volume under retention**.
- **Query and Query Frontend** → **query concurrency**, independent of ingest.

Those four move independently, which is why halving the scrape interval doubles Receive's CPU and disk while leaving its memory untouched.

### Choosing a profile {#choosing-a-thanos-profile}

Nobody knows their series count before they have somewhere to put metrics, so profiles are chosen from **inventory** —
what exists, not how busy it is.
Workload-intensity metrics are the wrong shape for this: they swing on a daily cycle, while cardinality is nearly flat.

| | S (bottom ~20%) | M (middle ~70%, chart defaults) | L (top ~10%) |
|---|---|---|---|
| **Collections** (indexes + materialized views) | ≤ 100 | 100–1,000 | 1,000+ |
| Kubernetes nodes | ≤ 10 | 10–100 | 100+ |
| Pods, all namespaces | ≤ 250 | 250–2,000 | 2,000+ |
| Cluster replicas | ≤ 4 | 4–30 | 30+ |

**Take the largest tier any single row lands in.**
Series is a sum, and one oversized dimension is enough to overrun the ingest path by itself.

Collections is the row that does the work, because Materialize's per-collection metrics carry `collection_id`,
`replica_id`, *and* `worker_id` — so they multiply by replication and by replica size, while sources and sinks stay linear in the object count.
A deployment with a few hundred indexes contributes far more cardinality than one with a few hundred sources.

> [!WARNING]
>   **These are whole-cluster numbers, not Materialize's share of the cluster.**
>   node-exporter, cAdvisor, and kube-state-metrics collect from every node and pod in the cluster, including workloads
>   that have nothing to do with Materialize.
>   Running Materialize on ten nodes of a shared two-hundred-node cluster is an **L**, not an S — and sizing off the
>   Materialize footprint alone is the most common way to pick wrong.

For a number rather than a bracket:

```text
active series ≈ 8,000 × nodes + 600 × collections
```

Calibrated against real deployments spanning two orders of magnitude of series count, and accurate to within ~10% across that range.
It is deliberately two terms with no constant — every other candidate axis measured (per-vCPU, per-replica, per-source)
either tracked one of these two or fell out as noise.
Per-vCPU in particular is actively misleading: vCPU tracks how much work a deployment does, and cardinality tracks how
many objects exist, so the two diverge as deployments grow.

### The envelope each profile is sized for {#thanos-envelope}

| Size | Active series | Samples/s at 30s | Bucket at profile retention |
|---|---|---|---|
| **S** | ~150 k | ~5 k | ~25 GB |
| **M** | ~1.5 M | ~50 k | ~660 GB |
| **L** | ~4 M | ~133 k | ~3 TB |

In a large deployment **Materialize's own metrics are ~85% of everything collected** — its native endpoints plus the SQL
exporters — and the Kubernetes infrastructure terms are a rounding error beside them.
So a conversation about metrics cost at that scale is a conversation about collections, not about nodes or pods.

### Measure to validate {#thanos-measure-to-validate}

Size from the estimator, then re-measure off Receive, which is the same signal the sizing is derived from:

```promql
# active series actually held (compare against the envelope you picked)
sum(prometheus_tsdb_head_series{job="thanos-receive"})

# ingested samples/sec
sum(rate(prometheus_tsdb_head_samples_appended_total{job="thanos-receive"}[5m]))

# series contributed per scrape job — the decomposition, when the total surprises you
sort_desc(sum by (job) (scrape_samples_scraped))
```

Reach for the third when measured series disagree with the estimate.
Its value per target *is* that target's series count, so it decomposes the total without touching every series in the
store — unlike `count({__name__=~".+"})`, which does, and is expensive enough for that to matter.

### Per-size resources {#thanos-per-size-resources}

The following table gives starting points as `replicas × (cpu request / memory request)`; tune them from real usage.
The chart's defaults are the M column, and the `thanos-small` and `thanos-large` profiles are deltas from them.

| Component | S | M | L |
|---|---|---|---|
| Receive (RF 3, `emptyDir`) | 3 × (250m / 1Gi) | 3 × (500m / 4Gi) | 6 × (1500m / 8Gi) |
| Receive `ephemeral-storage` req / limit | 2Gi / 3Gi | 4Gi / 6Gi | 6Gi / 8Gi |
| Store Gateway | 2 × (100m / 1Gi) | 2 × (500m / 3Gi) | 2 × (1500m / 8Gi) |
| Store Gateway **PVC** | 10Gi | 10Gi | 20Gi |
| Compactor (singleton) | 1 × (500m / 1Gi) | 1 × (1 / 2Gi) | 1 × (3 / 8Gi) |
| Compactor **PVC** | 20Gi | 50Gi | 200Gi |
| Query (HPA) | 2–3 × (200m / 512Mi) | 2–5 × (500m / 1Gi) | 3–8 × (1 / 2Gi) |
| Query Frontend | omit | omit | 2–5 × (250m / 512Mi) |

- **At `replicaCount == replicationFactor` every Receive pod holds every series.**
  Sharding begins only above RF, which is why **L** runs six pods rather than three — and why a replica count that is a
  multiple of both the replication factor and the zone count spreads evenly across both.
- **Don't set a tight memory limit on Receive.**
  An OOM-kill drops up to `tsdb.retention` (6h) of locally held blocks, and unlike Loki's ingesters that window has
  nothing protecting it but the replication factor.
  The chart sets requests for scheduling and no memory limit; alert on usage instead.
- **Store Gateway has a memory floor the requests must respect.**
  Thanos defaults `--chunk-pool-size=2GB` and `--index-cache-size=250MB`, and the subchart passes neither, so a stock
  Store Gateway wants ~2.5Gi before it serves anything.
  The **S** profile shrinks those pools through `extraArgs` rather than only lowering the request — a 1Gi request
  against 2.25GB of pools is an OOM, not a small install.
- **The Compactor is vertical-only.**
  It must stay `replicaCount: 1`, because concurrent compactors against one block set corrupt data.
  Its PVC is scratch space for the block group under compaction, which is why it grows faster with size than the other two volumes.

## Spreading across zones {#thanos-topology-spread}

Replication factor 3 is a claim about surviving a lost availability zone, and it is only true if the three replicas are *in* three zones.
Nothing makes that happen by default: a scheduler with no constraint is free to pack all three onto whatever nodes are
cheapest, at which point RF 3 costs three times the memory and protects against nothing but a single pod restart.

| Component | Zones | Hosts | Why |
|---|---|---|---|
| Receive | **hard** (`DoNotSchedule`) | soft | Write quorum (2 of 3) depends on it |
| Store Gateway | soft | soft | Read capacity only, and PVC-backed |
| Query, Query Frontend | soft | soft | Stateless and autoscaled |
| Compactor | none | none | Singleton — nothing to spread against |

**Hard on Receive is the load-bearing choice**, and it is hard for a reason that is easy to miss: a pod that cannot
satisfy the constraint goes `Pending`, and *that* is the signal Karpenter or the cluster-autoscaler uses to provision a node in the deficient zone.
A soft rule cannot summon capacity — it places the pod in the wrong zone and stays quiet about it, which is the failure this exists to prevent.

Two settings on that constraint matter as much as the constraint itself.
`nodeTaintsPolicy: Honor` keeps a node still carrying a startup taint from counting as an available domain, so the
autoscaler is not told a zone is covered when nothing can run there yet.
`matchLabelKeys: [controller-revision-hash]` counts only same-revision pods, so a rolling update does not deadlock its
own skew math against the pods it is replacing — and note the label differs by workload kind: StatefulSets carry
`controller-revision-hash`, Deployments carry `pod-template-hash`, and using the wrong one matches nothing and silently disables the guard.

> [!INFO]
>   **A hard rule works on Receive because Receive uses no PersistentVolume.**
>   A zonal volume cannot be attached from another zone, so a hard zone rule and an AZ-pinned pod pull against each
>   other: the rule says "go to the empty zone", the volume says "you may only run where I am", and the pod stays
>   `Pending` permanently rather than for as long as it takes to add a node.
>   On `emptyDir` there is nothing holding it back, which is why [the storage decision](#thanos-ephemeral-storage) and
>   this one are the same decision viewed twice.
>
>   It is also why the **Store Gateway** — the one Thanos component that still keeps a volume — is soft rather than hard.
>   With `volumeBindingMode: WaitForFirstConsumer` (the default for zonal CSI classes, and what you want) the volume
>   follows the pod and the two agree; with `Immediate` the PVC's zone is chosen *before* scheduling and a hard rule pointing elsewhere deadlocks.
>   Soft degrades on a StorageClass this chart does not control.

Unlike Loki, nothing had to be undone to make room for this: the Thanos subchart ships no default pod anti-affinity, so
there was no hard per-host rule to null out first.

**These constraints cannot move to `thanos.global.topologySpreadConstraints`**, which is where a reader would look for them.
Each one carries its own `labelSelector`, and a global constraint would make every Thanos component count Receive's pods when computing its own skew.
The selector matches on `app.kubernetes.io/component` alone rather than the full label set, because the subchart renders
these through `toYaml` rather than `tpl` — a `{{ include ... }}` would land in the manifest literally, so the release name is unavailable.
Spread is namespace-scoped and this chart assumes [one instance of each backend per
namespace](../stack-wide/#namespace-layout), so the component label is unambiguous.

## Storage: ephemeral by default {#thanos-ephemeral-storage}

Of the three Thanos components with local disk, **only Receive uses node-local `emptyDir`**, with an explicit `ephemeral-storage` budget.
The Store Gateway and the Compactor both keep a PersistentVolume, for different reasons.

| Component | Storage | Why |
|---|---|---|
| Receive | `emptyDir` | Holds *hours* of small, replicated data; durability is the replication factor, not the disk |
| Compactor | **PVC** | Holds *days* of it — the requirement does not fit node ephemeral storage |
| Store Gateway | **PVC** | Index-headers are cheap to lose but slow to rebuild, and there is no write quorum to block |

**The dividing line is how much disk the component needs, not whether its data is reconstructible.**
Neither Receive's un-uploaded window nor the Compactor's scratch is authoritative — the bucket is — so a durability
argument would send both to `emptyDir`.
What separates them is volume: Receive holds a 6h window of one replica's writes, which is gigabytes; the Compactor
works on whole block groups, so it needs the sources *and* the output of a multi-day compaction at once, which is tens of gigabytes.
Node ephemeral storage cannot serve the second.

**A PVC makes an availability-zone failure worse, not merely less flexible.**
That is the reasoning behind all three rows, and it is the opposite of the intuition a volume usually carries.
An EBS volume cannot be attached from another zone, so a pod whose zone is gone stays `Pending` until the zone comes
back rather than rescheduling into a healthy one.
On the write path that converts a recoverable event into an outage that waits on the cloud provider: with RF 3 write
quorum is 2, so two Receive pods stuck `Pending` on dead volumes block writes outright, where two `emptyDir` pods would
have been rescheduled and rejoined the hashring.

This is the same call the chart makes for [Loki's ingesters](../loki/#ingester-durability), for the same reason.
Blocks ship to object storage every 2h, so the window that exists only on local disk is at most 2h — and every replica
uploads its own copy under a distinct `receive_replica` external label, which the Compactor deduplicates into one copy
(see [Compaction and deduplication](#thanos-retention-compaction)).
A pod that returns with an empty volume has lost its copy of that window; the query path still answers from the surviving replicas.

> [!WARNING]
>   **The trade accepted in exchange:** Thanos Receive has **no peer hand-off**, so unlike Loki it will not backfill an
>   emptied pod from its neighbours.
>   That window stays at two copies instead of three until the next block ships.
>   Loki's ingesters recover from peers; Thanos's do not.
>
>   This is why the replication factor is load-bearing rather than merely advisable, and why RF 1 with ephemeral storage
>   is a genuinely unsafe combination rather than just an unwise one.

The Compactor is the case where the AZ argument loses, and it is worth saying why rather than leaving the inconsistency to be discovered.
Everything above applies to it — it is a **hard** singleton, so pinning it to a zone means a zone outage stops
compaction entirely, and while compaction is stopped **retention is not enforced and the bucket grows without bound.**
That is a genuine cost.

It loses anyway, because the alternative is worse: at the medium envelope a 2d compaction group is roughly six 8h blocks
at ~2.4Gi each, and the Compactor needs the sources and the output together — on the order of 30Gi.
A GKE node with a 47Gi boot disk offers about 18.8Gi of *allocatable* ephemeral storage, so an `emptyDir` that size never schedules at all.
**"Compaction pauses during a zone outage" beats "compaction never runs because the pod cannot be placed."**
And the pause is recoverable by hand: the scratch is not authoritative, so a Compactor wedged in a dead zone is unstuck
by deleting its PVC and letting it rebind elsewhere — see [The Thanos Compactor is stuck in a
zone](../../o11y-troubleshooting/#the-thanos-compactor-is-stuck-in-a-zone) for the ordering, which matters because two
Compactors running at once is the one thing that corrupts data.

### Declaring the ephemeral budget {#thanos-ephemeral-budget}

`emptyDir` volumes are invisible to the scheduler unless you say how large they will get, so every ephemeral component
declares `requests.ephemeral-storage` and `limits.ephemeral-storage`.
The two behave differently and the difference matters:

- **`requests.ephemeral-storage`** is what the scheduler places against.
  A node without room refuses the pod, which surfaces as a `Pending` pod with a clear reason instead of a node quietly filling up.
- **`limits.ephemeral-storage`** is enforced by the **kubelet evicting the pod**, not by throttling it.
  Accounting is periodic (`du` every ~10s unless filesystem project quotas are enabled), so a pod can overshoot briefly before eviction.

That second point is why the limits in [Per-size resources](#thanos-per-size-resources) carry deliberate headroom over
the requests rather than matching them: **evicting Receive destroys exactly the un-uploaded window the replication
factor exists to protect**, so a limit set tight enough to bite regularly would manufacture the failure this design avoids.

> [!WARNING]
>   **Size these against a node's *allocatable* ephemeral storage, not its disk size.**
>   The two are nowhere near each other, and the gap is where this goes wrong.
>
>   GKE reserves most of the boot disk for the image filesystem, so a **47Gi disk offers roughly 18.8Gi allocatable**.
>   A request above that cannot schedule anywhere, and the cluster-autoscaler will not rescue it either — no node of
>   that shape would fit, so it declines to scale up and the pod sits `Pending` indefinitely:
>
>   ```text
>   0/5 nodes are available: 4 Insufficient ephemeral-storage.
>   Pod didn't trigger scale-up: 3 Insufficient ephemeral-storage
>   ```
>
>   Check yours before raising anything here:
>
>   ```bash
>   kubectl get nodes -o custom-columns='NODE:.metadata.name,EPH:.status.allocatable.ephemeral-storage'
>   ```
>
>   Remember the budget is shared.
>   Loki's ingesters are `emptyDir`-backed too and declare no `ephemeral-storage` at all, so the scheduler believes they
>   need none — a node can look free and still be full.

Ephemeral storage is shared with container logs and image layers on the same filesystem, so the budget isn't Thanos's alone.
A log-verbose neighbor can evict a Receive pod that was sized correctly in isolation.

Raising Receive's local retention raises its ephemeral request with it, roughly linearly.
Of all the settings on this page, it's the one most likely to make Receive unschedulable.

## Protective limits {#thanos-protective-limits}

Thanos's exposure is the mirror image of Loki's.
There is no per-tenant byte-rate limit to set; the risk is one query fanning out across the bucket and taking the Store Gateway down with it.
All of the following limits are `extraArgs`, set per profile:

| Size | `--store.limits.request-series` | `--store.limits.request-samples` | `--query.max-concurrent` | `--query.timeout` |
|---|---|---|---|---|
| S | 1,000,000 | 50,000,000 | 10 | 2m |
| M | 5,000,000 | 200,000,000 | 20 (default) | 2m |
| L | 30,000,000 | 2,000,000,000 | 40 | 5m |

> [!WARNING]
>   **`extraArgs` is a list, and Helm overwrites lists rather than merging them.**
>   Several components ship non-empty defaults — `receive.extraArgs` carries `--receive.replication-factor=3`,
>   `compactor.extraArgs` carries `--consistency-delay=30m` and the [vertical compaction and deduplication
>   flags](#thanos-retention-compaction), `query.extraArgs` carries `--log.level=info`.
>   Setting `extraArgs` to add a limit **silently drops whatever was already there**, and on Receive that means falling
>   back to Thanos's default replication factor of 1 — the exact failure the quorum table above exists to prevent.
>   Restate the base arguments in full every time.
>   The shipped profiles do.

Receive also offers `--receive.limits-config` with a per-tenant `head_series_limit`, which the profiles deliberately leave unset.
It enforces that cap by querying a meta-monitoring endpoint for `head_series` on an interval, so it is a runtime
dependency on Thanos Query rather than a flag, and it fails in a direction that is not obvious when the dependency is unavailable.

## Retention and downsampling {#retention-and-downsampling}

Each profile ships the following retention per resolution:

| Size | raw | 5m | 1h |
|---|---|---|---|
| S | 14d | 30d | 90d |
| M | 30d | 90d | 365d |
| L | 30d | 180d | 730d |

> [!INFO]
>   **Downsampled volume does not depend on the scrape interval.**
>   A 5m block is five aggregates per 300s and a 1h block five per 3600s, however often you scrape — only raw blocks scale with the interval.
>   So moving from a 60s interval to 30s raises total bucket size by roughly a fifth rather than doubling it, because
>   raw is the smaller share of a retention shape that keeps downsampled data far longer.
>   The cost of finer granularity lands on Receive's CPU and local disk, not on object storage.
>
>   The corollary is that at a 60s scrape interval **5m downsampling saves no storage at all** — five aggregates per
>   300s is the same sample rate as one per 60s.
>   It buys query speed over long ranges.
>   All the real storage reduction is in the 1h tier.

**Raw retention has a floor set by the downsampling thresholds.**
Thanos produces 5m downsamples only from blocks spanning 40h or more, and 1h downsamples only from blocks spanning 10d or more.
Cut raw retention below those and the corresponding tier is never created at all, so long-range queries fall back to
reading raw blocks — slower and more expensive, which is the opposite of the intent.
That is why **S** keeps 14d of raw rather than something smaller.

## Compaction and deduplication {#thanos-retention-compaction}

The Compactor enforces retention.
With it disabled nothing expires, and bucket cost grows without bound.
The chart enables it with the medium row of [Retention and downsampling](#retention-and-downsampling): raw 30d, 5m 90d, 1h 365d.

**Vertical compaction is on** (`--compact.enable-vertical-compaction`).
Receive writes slightly overlapping blocks across every restart, and without vertical compaction the first overlap halts the Compactor.
A halted Compactor enforces no retention and produces no downsamples; `thanos_compact_halted` is the signal.

**Replicas are deduplicated in the bucket** (`--deduplication.replica-label=receive_replica`).
The RF copies of each block compact into one, so the bucket holds one copy of each series.
Thanos Query still deduplicates Receive's recent window on the same label.

Both flags live in `compactor.extraArgs`, so an override that replaces the list has to restate them, as with Receive's replication factor.
The list's other entries, `--log.level=info`, `--log.format=logfmt`, and `--consistency-delay=30m`, match Thanos's own
defaults, so dropping them changes nothing.

Deduplication is irreversible, and on an existing bucket it changes the external labels of compacted blocks.
Downsampled blocks written before the change can't be vertically compacted.
Thanos marks them `no-compact`, and they keep their per-replica copies until retention expires them.

Receive keeps **6h of local TSDB retention**, overriding the subchart's 24h, with WAL compression.
Blocks still ship to object storage every 2h, so local retention is a recent-query cache, not a durability window, and
the Store Gateway serves everything older.
6h is what makes the `emptyDir` budget fit a modest node.
24h at the medium envelope is about 7.3Gi per pod, against about 18.8Gi allocatable on a typical GKE node shared with Loki.

## Scaling past large {#scaling-past-large}

Above roughly 4M active series the profiles stop being a menu and become a starting point.
Three things change, in the order they bite:

1. **Add Receive replicas**, in multiples of the replication factor and the zone count, so streams shard evenly and no
  pod carries the whole series set.
2. **Shard the Store Gateway** once the bucket reaches the multi-TB range.
  Binary index-headers run roughly 1% of block size, so one StatefulSet eventually cannot hold them.
  `storegateway.sharded.hashPartitioning.shards` renders one StatefulSet per shard, and the headless Service,
  ServiceMonitor, and Query's DNS-SRV discovery fan out across them with no further configuration.
  Changing the shard count later renumbers shards and forces recreation, so decide it at install time rather than tuning into it.
3. **The Compactor runs out of room to grow.**
  It is a singleton by necessity, so the only levers are a bigger pod and a bigger volume, and compaction falling behind
  becomes a thing to alert on rather than discover.
  Thanos itself supports sharding compaction across instances with disjoint `--selector.relabel-config` selections, but
  the bundled subchart renders exactly one Compactor StatefulSet and exposes no selector — so that path needs an
  upstream change, or a second Compactor managed outside this chart.

## Cost control is the metric tiers, not the profile {#cost-control-metric-tiers}

The write path filters by importance — `essential`, `recommended`, `extended`, `diagnostic`, `all` — through
`minMetricImportance` on each destination.
The in-cluster Thanos destination deliberately defaults to **`all`**, unlike the external destinations, which default to `recommended`.
Raw metrics are worth their cost while Thanos is an early improvement over what came before, so this default isn't an oversight to correct.

> [!WARNING]
>   **Do not downshift the profile to reduce cost.**
>   The profile sizes the ingest path for the series you actually send, so sending the same series into a smaller path is an OOM rather than a saving.
>   Tighten `minMetricImportance`, re-measure head series, and downshift only once the series count has genuinely fallen.

## See also

- [Metrics architecture](../../../metrics/architecture/): the metrics architecture these settings configure.
- [Storing](../../../metrics/storing/): object storage and retention in depth.
- [Thanos Receive documentation](https://thanos.io/tip/components/receive.md/) (official): hashring, replication, and quorum semantics.
- [Thanos Compactor documentation](https://thanos.io/tip/components/compact.md/) (official): compaction levels,
  downsampling thresholds, and why the singleton constraint exists.
- [Thanos Store Gateway documentation](https://thanos.io/tip/components/store.md/) (official): index-header caching, the
  memory pools, and time and hash partitioning.
