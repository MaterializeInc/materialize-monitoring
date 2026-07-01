---
title: "First-Class Production Loki Infrastructure"
weight: 20260627
# draft=false makes it render as a page
# params.status=Draft is to indicate that the design is not final
draft: false
publishdate: 2024-06-27
lastmod: 2024-06-28
# custom parameters
params:
  author: Heather Lapointe
  status: "Draft"
---

# First-Class Production Loki Infrastructure

{{< param-table >}}

This doc captures the design decisions and open questions for shipping a productionalizable Loki inside the `materialize-monitoring` umbrella chart.
It is the design rationale behind the `loki` subchart wiring in `charts/materialize-monitoring/Chart.yaml`; day-2 operational procedures (backup/restore, upgrades, migration) will live in customer-facing docs and are only sketched here.

<!--
Agent note: this doc records decisions and their *why*. When a decision lands in code (values, templates, profiles), update the relevant section and check the corresponding open question. The per-component `existing*` audit table is meant to be filled in as we wrap each component.
-->

## Goals

Functional requirements, framed as value-first user stories.
Each describes *what* a user needs and *why it matters*; the [Technical BLUF](#technical-bluf) and the sections below describe *how* we deliver it.
Priority tags (**Must** / **Should** / **Could**) are relative to the **Production (M2)** milestone.

Two stakeholder classes consume this, and most requirements serve both:

- **Teams running self-managed Materialize** as an operational, business-critical system — small platform teams with engineering-led, GitOps-driven workflows.
- **Materialize's own Cloud / SRE platform team**, operating a fleet of many distinct environments and adopting this stack as the way they run observability.

The headline value for both is incident response: when something the business relies on misbehaves, logs are how it gets explained.
For the Cloud team the bar is higher still — the log store is tier-0 infrastructure they depend on every day, so its own availability and cost behavior are first-class requirements, not afterthoughts.

- **[Must] As an on-call engineer debugging a live incident,** I want to query an environment's logs alongside its metrics in Grafana, so that I can root-cause a problem in a business-critical system without SSHing into pods or hand-correlating across tools.
- **[Must] As a small platform team,** I want to stand up a production-grade log store from one opinionated, supported configuration, so that we can operate it reliably without becoming Loki topology experts or growing headcount to do so.
- **[Must] As a platform team,** I want to deploy and reconcile the stack with the delivery tool we already standardized on — Helm, Pulumi, ArgoCD, or FluxCD — so that adopting observability does not force a change to our deployment workflow and behaves identically whichever reconciler applies it.
- **[Must] As an operator,** I want logs to land in object storage I control, with access granted through cloud workload identity, so that I get durable retention with no long-lived credentials living in the cluster.
- **[Must] As an operator of a business-critical system,** I want a faulty logging-pipeline config to be caught before it reaches running nodes, so that a single bad change can never blind us by taking down log collection cluster-wide.
- **[Should] As an operator without cloud identity federation,** I want a supported way to supply storage credentials directly, so that I can still run in on-prem or restricted environments where workload identity is unavailable.
- **[Should] As an architect committing to this stack for the long term,** I want upgrades to complete without losing logs that were in flight, so that I can rely on log completeness across routine maintenance.
- **[Should] As a budget owner,** I want to control how long logs are retained, so that storage cost stays predictable as log volume grows.
- **[Should] As anyone relying on the logs during an incident,** I want confidence that what is collected is complete and queryable, so that the absence of a log means the event did not happen — not that the pipeline silently dropped it.
- **[Should] As a developer running CI,** I want to exercise the full logging path against a real-but-throwaway log store with no cloud dependencies, so that integration tests are hermetic and fast.
- **[Could] As a security or compliance owner responding to an incident,** I want to preserve a tamper-evident copy of logs and demonstrate they were not altered, so that they hold up as evidence in an audit or investigation.
- **[Could] As an operator recovering from a failure,** I want a clear recovery path if the log store or its storage backend is lost, so that I can restore service and know exactly what data is and is not recoverable.

Additional requirements from Materialize's own Cloud / SRE platform team, as a fleet operator:

- **[Must] As the Cloud platform team,** I want fleet observability to run on the same stack we ship to self-managed users, so that we maintain one observability approach rather than two divergent ones — and what we operate daily is exactly what we support.
- **[Must] As the Cloud SRE team,** I want the log store treated as tier-0 infrastructure — highly available and self-monitoring with its own alerting — because we depend on logs daily for usage reporting, troubleshooting, analysis, and incident response, and the log store going down is itself an incident.
- **[Must] As the Cloud SRE team operating at fleet scale,** I want series/label reduction and tiered retention built in, so that log cost stays bounded, the log store stays stable under load, and dashboard queries stay fast.
- **[Should] As the Cloud SRE team,** I want a composable view of logs across many distinct environments, so that I can triage, analyze, and report across the whole fleet from one place rather than environment-by-environment.

The series-reduction-plus-tiered-retention requirement is grounded in a prior internal logging-pipeline effort where exactly that approach measurably cut cost, reduced log-store instability, and sped up dashboard queries.
It is therefore treated as a baseline expectation here, not an optimization to defer — which is why it is tagged **Must** and why the Alloy gateway's cardinality-reduction role and a tiered-retention storage policy are load-bearing parts of the design rather than tuning knobs.

## Technical BLUF

- Run Loki in **microservice mode by default**, productionalizable on AWS, GCP, and Azure (in that priority order).
- **Wrap** the upstream `grafana-community/loki` chart as a subchart — track and patch upstream rather than fork.
- Work correctly across four deployment targets — **Helm CLI, Pulumi (`helm.v4.Chart`), ArgoCD, and FluxCD** — none of which share an ordering or hook model.
- Require **S3-compatible object storage** in production; allow **monolithic + filesystem** for integration testing, where Loki is treated as a black-box interface.
- Express deployment shapes as **composable profile values files** (e.g. `integration-test`, `aws-prod`) rather than a proliferation of single-purpose flags.

## Non-goals

- **loki-operator.** Built primarily for OpenShift, little productionalized usage, and no articulated upgrade story. Not adopted.
- **Forking the upstream deployment.** If an upstream field is broken or missing, we prefer to patch upstream (as we already do for `podAntiAffinity` and resource defaults in the cloud repo) rather than maintain a fork.
- **A single ordering annotation that works everywhere.** It does not exist (see below); we design for convergence instead.

## The ordering reality

`config.kubernetes.io/depends-on`, `argocd.argoproj.io/sync-wave`, and Helm hook weights are all **advisory efficiency hints, not guarantees**.
There is no mechanism honored by all four targets, and they disagree even on Helm hooks:

| Mechanism | Helm CLI | Flux (helm-controller) | ArgoCD | Pulumi `helm.v4.Chart` |
|---|---|---|---|---|
| Helm hooks (`pre-install`/`pre-upgrade`) | ✅ full lifecycle | ✅ | ⚠️ converted to PreSync/PostSync; delete + PVC semantics differ | ❌ degrade to ordinary resources — no lifecycle, no wait, delete-policy ignored |
| `helm.sh/hook-weight` | ✅ | ✅ | partial | ❌ |
| `argocd.argoproj.io/sync-wave` | ❌ | ❌ | ✅ | ❌ |
| `config.kubernetes.io/depends-on` | ❌ | ❌ | ❌ | native `dependsOn` used instead |

**Design consequence:** the chart must converge correctly with **zero ordering guarantees** — crashloop-and-retry is the real ordering mechanism in Kubernetes.
Any ordering annotation is rendered as an *optional* hint, gated behind a `gitops.tool` value, and never load-bearing for correctness.

Because the Pulumi target is `helm.v4.Chart` (chosen deliberately, to track every resource individually), this is not hypothetical: under Pulumi the Helm hook lifecycle does not run at all.

## Validation strategy

Validation happens at three tiers; the tier is chosen by what *must* be true and what each target can guarantee.

1. **CI (primary, tool-agnostic).**
   Render the chart and run `alloy validate` / `loki -verify-config` before anything is ever applied.
   This catches the common case earliest and is identical across all four targets.
2. **Pre-rollout Jobs (required for the high-blast-radius case).**
   The driving concern is the **OTLP auth blocks in the Alloy agent**: a bad config rolled out to a `DaemonSet` can break logging across an entire customer cluster.
   That risk justifies a real pre-rollout validation **Job** even though hooks are unreliable across targets — we accept that the Job is a true gate only on Helm CLI / Flux, and elsewhere it runs as an ordinary resource without strict ordering.
   To make this safe regardless of hook support, pair the Job with mechanism (3).
   Loki itself needs comparatively little validation and is not a primary driver here.
3. **initContainer (the portable backstop).**
   An initContainer on the component pod, using the same image, runs the relevant `validate`/`verify-config` before the main container starts.
   This is the only "validate before this rolls" mechanism that rides the **pod lifecycle** rather than the GitOps tool's hook support, so it behaves identically under all four targets.
   Combined with immutable hashed config (below), a bad config means new pods fail init/readiness while the old pods keep serving — the safe failure mode.

Diagnostic/log-capture Jobs keep the `hook-delete-policy: before-hook-creation` convention (logs survive regardless of success or failure), but their output is treated as advisory, and their names are hashed so each config revision is a fresh object and stale ones are pruned.

## Secrets strategy

The "generate a random password only if the Secret doesn't already exist" pattern relies on `lookup`, which returns empty at render time under **both ArgoCD and Pulumi `helm.v4.Chart`**, and the hook-Job fallback does not run under Pulumi.
So that pattern cannot give stable-after-first-install secrets across our targets.

**Decision: consume secrets by name by default.**
The chart references an `existingSecret` (e.g. the basic-auth password shared between components); it does not mint it.
Bootstrapping the secret is out-of-band and documented: external-secrets, SealedSecrets/SOPS, or a one-time `kubectl create secret`.

This is not fully settled.
Deterministic-random inputs work well under **Terraform and Pulumi** (each has its own stable random providers), so we may later offer an opt-in generate path for those targets — but ArgoCD and Helm-CLI lack a clean equivalent, so **consume-by-name remains the portable default**.

## Config immutability (blue/green for config)

We want hashed config-object names so that old and new config coexist during a rollout: a pod that restarts mid-rollout keeps the **old, known-good** config instead of picking up a new bad one.
The constraint is that we cannot force an upstream subchart's resource names to include a hash unless that subchart cooperates (we do not control whether it calls `tpl`).

**Approach:** where a component exposes an `existingConfigMap`/`existingSecret`-style hook, **disable the subchart's own object and inject a hash-named one** from the parent chart (which does call `tpl`).
Where a component does *not* expose such a hook, we fall back to in-place mutation plus a `checksum/config` pod annotation — which triggers a rollout but does not protect a pod that restarts mid-rollout.

The up-front task is a per-component audit (table below).

### Per-component `existing*` audit

Fill in as each component is wrapped.
"Hashed config" is only achievable where the answer is yes.

| Component | `existingConfigMap`? | `existingSecret`? | Notes |
|---|---|---|---|
| distributor | ⬜ | ⬜ | |
| ingester | ⬜ | ⬜ | stateful; WAL PVC |
| querier | ⬜ | ⬜ | |
| query-frontend | ⬜ | ⬜ | |
| compactor | ⬜ | ⬜ | singleton |
| ruler | ⬜ | ⬜ | own storage prefix |
| gateway | ⬜ | ⬜ | nginx today — see topology |
| memcached | ⬜ | ⬜ | |

## Storage and credentials

- **S3-compatible required in production.** "S3-capable" means anything exposing the S3 API that Loki already supports (AWS S3, MinIO, Ceph, R2, …), plus native **GCS** and **Azure Blob**.
- **Integration testing** uses monolithic + filesystem; Loki is a black box at that boundary, so no object store is required there.
- **Single bucket, prefixed** by path: `/loki/chunks`, `/loki/ruler` (an `admin/` prefix only matters if we ever go enterprise). A single role scoped to the bucket is the simpler default; prefix-scoped IAM is available if least-privilege is needed.
- **AWS account-namespaced buckets** are now GA and are the **recommended** bucket shape; they are standard S3 addressing to Loki's client, so no special handling.
- **Credentials:** prefer **IRSA / GKE Workload Identity / Azure Workload Identity** (no static keys). But we must support **manually configured credentials** as a documented escape hatch, including the unrecommended env-var paths. Document the IAM + role + policy + IRSA wiring per cloud clearly, since these are the steps our Terraform module satisfies automatically but bare deployments do not.

## Deployment topology

- **Microservice mode by default** via the upstream chart's distributed deployment mode.
- **`replication_factor` 3** implies ≥3 ingesters and, for real AZ resilience, **≥3 zones** — losing a zone should cost at most one of three replicas. With only two zones an AZ loss can drop two of three and break write quorum (quorum is 2 for RF 3) until the ring recovers; reads still succeed on the surviving replica.
- **Compactor is a singleton** (retention + compaction), run **ephemerally** so it can float freely between zones instead of being PVC-pinned to one — compaction is idempotent, off the critical path, and our production index is ~30MB, so re-downloading it each cycle is negligible.
- **Ingesters are ephemeral.** No PVC — node-local `emptyDir` for the WAL and not-yet-flushed chunks; durability comes from `replication_factor` 3, not disk. A rescheduled ingester starts fresh and the ring backfills from peers. This deliberately trades per-ingester crash recovery for freely-reschedulable pods, avoiding EBS zonal pinning and the slow (up to ~6-minute) detach/attach that PVC-backed ingesters suffer during node replacement. Consequently `terminationGracePeriodSeconds` is kept **modest** and `flush_on_shutdown` is best-effort — correctness does not depend on the grace window (a truncated flush is covered by the other replicas), so enterprise force-kill windows (120/300s) are harmless.
- **What lands in object storage under RF 3:** all replicas flush independently; chunk object keys are content/identity-addressed, so byte-identical chunks collapse to the same key (idempotent write), and any residual overlap from divergent chunk boundaries is deduplicated by the querier at read time. There is no elected or delegated flusher.
- **Placement:** ingesters spread **hard across zones** (`DoNotSchedule` topologySpread — a Pending pod is what makes Karpenter provision a node in the deficient zone; soft rules cannot summon capacity) and **soft across hosts** (`ScheduleAnyway`, with the chart's default hard per-host anti-affinity removed via `affinity: null`, since `{}` is a no-op against the subchart default). `nodeTaintsPolicy: Honor` keeps startup-tainted nodes out of the skew math until their taint clears; model that taint as a Karpenter `startupTaint` so it does not over-provision while waiting. `zoneAwareReplication` is **not** used — topologySpread gives the AZ spread without the cross-zone replication bandwidth cost, which is irrelevant at our size.
- **Schema:** standardize on **TSDB v13**. Schema periods are append-only with a future `from` date and can never be changed retroactively — this is the primary day-2 footgun and a candidate for a validation check that refuses to mutate a past period. v13 is also the prerequisite for **structured metadata**, which aligns with the Alloy log-processing direction.
- **Caches:** ship our **own memcached** rather than assume an external one.
- **No live config reload.** Productionalized Loki has shown almost no need for dynamic per-tenant overrides; we accept restart-to-reconfigure and avoid the `runtime_config` live-reload surface for now.
- **Ephemeral by default; the ruler is the exception.** Ingesters, index-gateway, and compactor all run on node-local `emptyDir` — their on-disk state is a cache (index-gateway), an idempotent working copy (compactor), or replication-backed buffer (ingester WAL) of object storage, so they reschedule freely across zones with no PVC. **Only the ruler keeps a dynamic PVC**, for its remote-write WAL: recording-rule samples buffered there when the metric store is unreachable are genuinely useful durability in the run-up to an incident, exactly when dropping derived signals hurts most.
- **StorageClass:** because only the ruler is PVC-backed, `storageClassName` matters just for it — but it still matters: a default StorageClass with dynamic provisioning is **not** safe to assume on-prem/bare clusters, and even managed EKS may lack one unless the EBS CSI addon is installed (gp2 default is gone; gp3 via CSI). The ruler PVC does re-introduce zonal pinning for that one component; acceptable because rule evaluation catching up after a delay is tolerable and the ring reshards its groups to the surviving ruler.

### Gateway / ingress

The upstream loki-gateway is an **nginx** reverse proxy doing path-based routing, and we do not want nginx here (though we are using it for loki-gateway today).
Cloud ingress is also fragmenting (ALBs not in use where expected, Gateway API adoption slow, nginx explicitly unwanted).

**Decision: the alloy-gateway is the canonical write-path gateway.**
We already bundle Alloy in a gateway role, so writes flow agent → **alloy-gateway** → distributor, and the bundled nginx loki-gateway is dropped from the write path.
The read path points Grafana's datasource at the **query-frontend** Service directly; a `LoadBalancer` Service default is sufficient otherwise.
Reserve a separate gateway only if a single external hostname or auth termination is required, and prefer the cloud LB / Gateway API / Envoy over re-introducing nginx.

## Sizing

**Size by throughput and burst, not by bucket size.**
Bucket size is a *derived* output — sustained throughput × retention — not a sizing input; our first instinct to key the tiers off bucket size was wrong.

Different parts of Loki are sized off different points of the load envelope:

- **Ingest path** (distributors, ingesters, WAL, ingestion limits) → the **5-minute burst**, with headroom to the regression ceiling.
- **Storage / retention / bucket** → **sustained** throughput.
- **Read path** (frontend, queriers, caches) → query load, independent of ingest.

The three t-shirt sizes are defined by the ingest envelope:

| Size | Sustained (peak-hour) | 5-min burst | Regression ceiling | Anchor |
|---|---|---|---|---|
| S | ~0.25 MiB/s | ~1 MiB/s | ~2 MiB/s | dev / staging-class |
| M | ~0.75 MiB/s | ~3 MiB/s | ~8 MiB/s | mid |
| L | ~2 MiB/s | ~6 MiB/s | ~17 MiB/s | measured — Materialize SaaS production |

**L is anchored on measured production** (distributor-received bytes): peak ~6 GiB/hr (~1.7 MiB/s), a 5-minute burst of ~1.8 GiB (~6.1 MiB/s), and a pre-Alloy 5-minute burst of ~5 GiB (~17 MiB/s) that sets the regression ceiling.
The 5-minute burst runs ~3.6× the peak-hour rate, so averages must never drive sizing — the burst does.
The pre-Alloy regression ceiling is the figure to degrade gracefully against, and it quantifies why the Alloy reduction is load-bearing rather than an optimization.

Two consequences for the topology:

- **Per-tenant limits ≠ aggregate capacity.** `ingestion_rate_mb` and friends are per tenant (per environment); the aggregate burst is a fleet-capacity concern handled by distributor and ingester count, not by the per-tenant limit.
- **Scale ingesters past 3 on memory, not bytes.** With `replication_factor` 3 and exactly 3 ingesters, every ingester holds every stream, so per-ingester memory tracks total stream cardinality. When that is the constraint — or to spread the regression burst and shrink blast radius — run N > RF so streams shard across the ring.

Per-component resource starting points, the protective limit values, the measurement queries, and the full operator checklist (with the shared-responsibility model) live in [Operating > Production Best Practices](../../../../operating/production-best-practices/).

## Tenancy

**Decision: one logical Loki tenant per install; isolation is label-based within it, and the hard isolation boundary is the install (per region/stack).**

This is driven by two things pulling the same direction:

- The Grafana Loki datasource manages a **single tenant per datasource**, so an arbitrary/growing set of tenants does not map cleanly onto Grafana.
- More fundamentally, our **composable view across many environments** is a requirement, and real `X-Scope-OrgID` multi-tenancy fights it — cross-tenant reads require enumerating tenants (`a|b|c`), which is fragile and expensive as the environment count grows.

So one tenant + label segmentation (`environment_id`, `organization_name`, …) is not a workaround for the plugin; it is the shape that matches "see the whole fleet in one place."

The per-environment controls people reach for multi-tenancy to get are available **per-label** within a single tenant, and we use them:

- **Per-stream retention** keyed on `environment_id` (the tiered-retention plan).
- **Per-label rate limits** (`by_label_name`) — already applied on `namespace` in the pipeline.
- **Deletion** by label matcher (`environment_id="…"`) for per-environment/compliance deletes.

Accepted caveats:

- **Isolation is soft.** Any query against the tenant can see every environment's logs unless label-filtered. Acceptable for trusted internal SRE/operators; **not** sufficient for per-team need-to-know or customer-facing log access. If that becomes a requirement, the path is Grafana **LBAC** (Enterprise/Cloud) for a single-datasource model, or tenant-per-environment writes + multi-tenant reads.
- **Cardinality concentrates.** One tenant aggregates every environment's streams into one index space, which brings the [sizing](#sizing) **N > RF** ingester-memory lever forward — watch per-ingester memory against total fleet stream count.
- **The tenant ID is baked into the object-storage path.** Switching tenant models later relocates chunks/index — a migration, not a flag flip.

Forward-compat: set **`auth_enabled: true` with one explicit named `X-Scope-OrgID`** (not the implicit `fake` single-tenant tenant), so a future split writes *new* tenants alongside rather than migrating existing data.

This also **reinforces the no-live-reload decision**: with a single tenant the per-tenant override need largely disappears — overrides become per-stream retention (compactor config) and per-label limits, both static.

**Flip trigger:** move to real multi-tenancy only when (a) hard isolation/compliance/per-team or customer-facing access is required, or (b) per-tenant limit/retention isolation is needed that labels cannot express.

## Profiles

- Profiles live in `charts/materialize-monitoring/profiles/` as **composable values files** (e.g. `integration-test.values.yaml`, `aws-prod.values.yaml`), expanded via `helm-docs`.
- **Composition is ordered** (later file wins); this works cleanly with ArgoCD `valuesFiles`, and we have tricks to enforce ordering on the other targets.
- **Example guard:** rather than `-f aws-prod --set thisIsAnExample=false` (awkward to express in ArgoCD's `valuesFiles`), prefer a **fail-closed** sentinel — example profiles set `thisIsAnExample: true`, real profiles set it false, and the chart errors (via NOTES.txt / a `fail` template) when it is unset or true. An unconfigured install should fail rather than silently run an example.

## Day-2 operations (to be expanded in customer docs)

- **Disaster recovery, not "backup."** Loki has no native snapshot: chunks are immutable, the WAL is ephemeral, and the index is rebuilt from object storage. "Recovery" = durable object storage + **versioning** + (optionally) **cross-region replication**; restore = repoint at the bucket. The docs should describe bucket configuration as the DR primitive, not a `loki backup` button.
- **Security / audit events.** Document what the log store guarantees during an incident: **Object Lock / WORM (compliance mode)** for tamper-evident logs, **versioning** for integrity, and how to **freeze/preserve** a bucket. Pair with retention/compactor and the deletion API for compliance deletes.
- **Upgrades / migration.** Document supported version skew during rolling upgrades and the schema-period append-only constraint. Called out explicitly because the targets cannot guarantee ordered upgrades.

## Meta-monitoring

We are a monitoring product, so Loki self-observability is on by default: ServiceMonitor/PodMonitor (GCP `PodMonitoring`), the Loki mixin (dashboards + alerts), and optionally `loki-canary` for end-to-end write→read verification.

## Open questions

- [ ] Which Loki chart **version** to pin? The repository is settled — `grafana-community/loki` is the new canonical chart location (migrated this year) and is the modern microservice path via distributed deployment mode. The `^15.0.0` version in `charts/materialize-monitoring/Chart.yaml` is still scaffolding and needs to be resolved against the current latest.
- [ ] `values.schema.json` for the wrapped values — deferred (YAGNI for the first pass; subcharts make it awkward). Revisit once the value surface stabilizes.
- [ ] Whether to offer an opt-in deterministic-random secret-generation path for Terraform/Pulumi while keeping consume-by-name as the default.
- [ ] Final gateway decision (skip-gateway vs. non-nginx gateway).
- [ ] Per-component `existing*` audit (table above) — fill in as each component is wrapped.
