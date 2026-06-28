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

- **S3-compatible required in production.** "S3-capable" means anything exposing the S3 API that Loki already supports (AWS S3, MinIO, Ceph, R2, …), plus native **GCS** (priority 2) and **Azure Blob** (priority 3).
- **Integration testing** uses monolithic + filesystem; Loki is a black box at that boundary, so no object store is required there.
- **Single bucket, prefixed** by path: `/loki/chunks`, `/loki/ruler` (an `admin/` prefix only matters if we ever go enterprise). A single role scoped to the bucket is the simpler default; prefix-scoped IAM is available if least-privilege is needed.
- **AWS account-namespaced buckets** are now GA and are the **recommended** bucket shape; they are standard S3 addressing to Loki's client, so no special handling.
- **Credentials:** prefer **IRSA / GKE Workload Identity / Azure Workload Identity** (no static keys). But we must support **manually configured credentials** as a documented escape hatch, including the unrecommended env-var paths. Document the IAM + role + policy + IRSA wiring per cloud clearly, since these are the steps our Terraform module satisfies automatically but bare deployments do not.

## Deployment topology

- **Microservice mode by default** via the upstream chart's distributed deployment mode.
- **`replication_factor` 3** implies ≥3 ingesters; document the bootstrap/quorum implications for small/integration installs.
- **Compactor is a singleton** (retention + compaction).
- **Ingesters are stateful:** WAL needs PVCs; graceful rollout needs flush-on-shutdown, a long `terminationGracePeriod`, and PDBs to avoid losing un-flushed data.
- **Schema:** standardize on **TSDB v13**. Schema periods are append-only with a future `from` date and can never be changed retroactively — this is the primary day-2 footgun and a candidate for a validation check that refuses to mutate a past period. v13 is also the prerequisite for **structured metadata**, which aligns with the Alloy log-processing direction.
- **Caches:** ship our **own memcached** rather than assume an external one.
- **No live config reload.** Productionalized Loki has shown almost no need for dynamic per-tenant overrides; we accept restart-to-reconfigure and avoid the `runtime_config` live-reload surface for now.
- **StorageClass:** assume a default StorageClass with dynamic provisioning in managed cloud, but make `storageClassName` first-class and document that it is **not** safe to assume on-prem/bare clusters, and that even managed EKS may lack a default unless the EBS CSI addon is installed (gp2 default is gone; gp3 via CSI).

### Gateway / ingress

The upstream loki-gateway is an **nginx** reverse proxy doing path-based routing, and we do not want nginx here (though we are using it for loki-gateway today).
Cloud ingress is also fragmenting (ALBs not in use where expected, Gateway API adoption slow, nginx explicitly unwanted).

**Decision: the alloy-gateway is the canonical write-path gateway.**
We already bundle Alloy in a gateway role, so writes flow agent → **alloy-gateway** → distributor, and the bundled nginx loki-gateway is dropped from the write path.
The read path points Grafana's datasource at the **query-frontend** Service directly; a `LoadBalancer` Service default is sufficient otherwise.
Reserve a separate gateway only if a single external hostname or auth termination is required, and prefer the cloud LB / Gateway API / Envoy over re-introducing nginx.

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
