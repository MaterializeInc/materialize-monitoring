---
title: "Roadmap"
weight: 60
---

<!-- This roadmap is public. Do not include customer-specific or sensitive information -->

# Roadmap

The goal of `materialize-monitoring` is **first-class, opt-in observability for self-managed Materialize** — logs, metrics, events, and alerts — for customers who want a one-stop-shop, without forcing our stack on customers who already run their own.
This page is the current source of truth for what is built, what is in flight, and what is planned next.

The work spans two Linear projects, and this page tracks both:

- [First-class Observability Infrastructure in Self-Managed](https://linear.app/materializeinc/project/first-class-observability-infrastructure-in-self-managed-5e48691c74a8/overview) — **FCO**, the platform. Building the thing.
- [Operational Observability](https://linear.app/materializeinc/project/operational-observability-abf9af76c03a/overview) — **OO**, the successor. Hardening it, stamping 1.0, and making it answer operational questions.

<!--

## How this maps to the original plan

The May 2026 plan assumed a different shape than what was actually built.
Where docs, tickets, or comments disagree with the repo, the repo wins and this page records why:

| Original plan (May 2026) | As built |
|---|---|
| Grafana dashboards via Jsonnet/Grafonnet (`sources/jsonnet/`) | Python + `grafana-foundation-sdk` + `py-mzmon-lib` (`packages/grafana-dashboards/`) |
| `crates/` Rust workspace + `sources/` input tree | `packages/` monorepo where Rust (`mzmon-lib`) and Python coexist |
| Datadog dashboards via a Datadog Rust SDK | Not pursued; OTLP-forward is the export path (see Pipelines) |
| Four fixed profiles, incl. a `datadog-agent` profile | Profile set deliberately left open; no `datadog-agent` profile |

-->

## Cadence and milestones

Releases track a monthly cadence aligned to the **15th**.

Milestones are named by maturity stage; the date is a soft target.

Linear restarts milestone numbering at M1 for every project, so milestones here carry a project prefix — **FCO-M1**–**FCO-M4** and **OO-M1**–**OO-M3**.
Item tables below reference milestones by those prefixed tags.

### FCO — First-class Observability Infrastructure (the platform)

| Milestone | Target | Deliverables |
|---|---|---|
| **Foundation** (FCO-M1) | June 15 | `env-top` overview dashboard (Summary, Kubernetes, Cluster, Connections, Compute, Storage — including Hydration / Freshness / Sources / Sinks *summaries*); cloud ↔ self-managed convergence via `$sqlMetricPrefix`; typed Alloy **agent** pipeline; **ScrapeConfigs + ServiceMonitors** for metric collection (synced to charts and docs); Hugo docsite; pre-commit suite; per-component versioning/changelog/release automation; Grafana dashboard v1/v2 API support |
| **Production** (FCO-M2) | July 15 (required) | Native **OTLP exporter** support; **productionalized** stack (Thanos + Loki + Alloy); product observability documentation **fully replaced**; base alert rule set; Helm subchart bundling; `renovate` for dependency bumps |
| **Operational Depth** (FCO-M3) | July 31 (stretch) | **Terraform modules** + the collection-parity and kind-E2E work they depend on; charts published to GHCR as OCI artifacts; portable PVC defaults |
| **Maturity** (FCO-M4) | August 31 | Closing the project. Everything not landing with the current customer releases moved to OO |

### OO — Operational Observability (the successor)

| Milestone | Target | Deliverables |
|---|---|---|
| **1.0** (OO-M1) | August 28 | *Hardening:* Grafana reachable (ingress/service + LB) and **persistent** (Postgres or PVC); node and container metrics on the default Alloy path (cAdvisor + node-exporter); the profile set (Thanos sizing, `kind`, scheduling, storage class); the pre-delete finalizer hook; static object-storage credentials in Terraform. *The contract:* **stamp 1.0** on the chart and Terraform module; NetworkPolicy for every component; in-cluster mTLS via cert-manager; the Rust E2E suite; the deprecation policy and contract reference docs; remaining destinations (OTLP, Datadog, auth) exposed through Terraform; Grafana 11 (v1) parity for the public dashboards gallery |
| **Troubleshooting** (OO-M2) | September 25 | **Troubleshooting**, **Logs & Events**, **Upgrades**, and **Networking** dashboards; the Day 2 change operations; the Hydration / Freshness / Sources / Sinks drilldowns (⛓️); Alertmanager adoption; orchestratord and k8s controller instrumentation |
| **Reach** (OO-M3) | October 23 | Native Datadog / GCM / Honeycomb dashboard sets; BYOC gateway-to-gateway architecture and sanitization; adoption in Materialize Cloud via Pulumi; agent→gateway OTLP with a WAL; the Day 1 dashboards; the GitOps CI matrix |

Hardening and the 1.0 contract were briefly separate milestones and were merged back.
They get worked in parallel, and the split was not carrying its weight: the hardening items are the reasons a 1.0 would be premature, and the contract items are what the number actually promises.
Neither ships without the other.

## Status legend

- ✅ Done · 🔨 In progress · ⬜ Planned
- ⛓️ Blocked on an upstream metric-contract dependency (see [Metrics contract](#metrics-contract-upstream-dependency))

## Workstreams

### Dashboards

The `env-top` overview is shipped and carries the cloud ↔ self-managed convergence work.
**Grafana 11 (dashboard v1) parity is a hard requirement** for publicly hosted dashboards — the dashboard sources must continue to render against the v1 dashboard API, not only newer versions — so that the dashboards can be managed in the **Grafana public dashboards gallery**.

| Item | Milestone | Status |
|---|---|---|
| `env-top` overview (6 tabs, incl. Hydration/Freshness/Sources/Sinks summaries) | FCO-M1 | ✅ |
| Cloud ↔ self-managed convergence (`$sqlMetricPrefix`) | FCO-M1 | ✅ |
| GCP / GKE / GMP dashboard + datasource variations | FCO-M2 | ✅ |
| Improved Grafana 11 (dashboard v1) support for the public dashboards gallery ([DEP-206](https://linear.app/materializeinc/issue/DEP-206)) | OO-M1 | 🔨 |
| [Troubleshooting](https://linear.app/materializeinc/issue/DEP-208) — symptom-first entry into the rest | OO-M2 | ⬜ |
| [Logs & Events](https://linear.app/materializeinc/issue/DEP-209) (Loki + Alloy + logs now shipped) | OO-M2 | ⬜ |
| [Upgrades](https://linear.app/materializeinc/issue/DEP-210) (Day 2 ops) — **customer-blocking**, see below | OO-M2 | ⛓️ (rollout half is buildable now) |
| [Networking](https://linear.app/materializeinc/issue/DEP-211) | OO-M2 | ⬜ |
| [Hydration Drilldown](https://linear.app/materializeinc/issue/DEP-212) | OO-M2 | ⛓️ |
| [Freshness Drilldown](https://linear.app/materializeinc/issue/DEP-213) | OO-M2 | ⛓️ |
| [Sources Drilldown](https://linear.app/materializeinc/issue/DEP-214) | OO-M2 | ⛓️ |
| [Sinks Drilldown](https://linear.app/materializeinc/issue/DEP-215) | OO-M2 | ⛓️ |
| [Resizing](https://linear.app/materializeinc/issue/DEP-157) (Day 2 ops) | OO-M2 | ⬜ |
| [Changing sources](https://linear.app/materializeinc/issue/DEP-158) (Day 2 ops) | OO-M2 | ⬜ |
| [Changing external destinations](https://linear.app/materializeinc/issue/DEP-159) (Day 2 ops) | OO-M2 | ⬜ |
| [Managing users](https://linear.app/materializeinc/issue/DEP-160) (Day 2 ops) | OO-M2 | ⬜ |
| Provide [Datadog](https://linear.app/materializeinc/issue/DEP-115) dashboard set | OO-M3 | ⬜ |
| Provide [Google Cloud Monitoring](https://linear.app/materializeinc/issue/DEP-217) dashboard set | OO-M3 | ⬜ |
| Provide [Honeycomb](https://linear.app/materializeinc/issue/DEP-218) dashboard set | OO-M3 | ⬜ |
| [Dependencies](https://linear.app/materializeinc/issue/DEP-224) (Day 1: are Materialize + o11y requirements satisfied?) | OO-M3 | ⬜ |
| [Sizing](https://linear.app/materializeinc/issue/DEP-225) (Day 1) | OO-M3 | ⬜ |
| [Replace dashboard management with a Rust implementation](https://linear.app/materializeinc/issue/DEP-222) | OO-M3 | ⬜ |

We weight **Day 2 operations over Day 1**: upgrades, resizing, changing sources, changing external destinations, and managing users are the operations that matter most for a running deployment.
Day 1 dashboards (Dependencies, Sizing) stay last.

**Upgrades is the sharpest of these and is tracked as Urgent.**
It is the Day 2 operation with the widest blast radius and the least visibility, and we have direct evidence of it blocking a production adoption decision — an operator watching a long upgrade with no way to tell whether it was progressing or stuck, and no idea what to do if it were.
That framing is the requirement: the dashboard has to answer *is it stuck* and *what do I do about it*, not merely display version counts.
"Is it stuck" is usually answered by orchestratord's reconciliation state, which is why this dashboard and the controller instrumentation below should share signals.

Change operation dashboards focus on new objects being added or removed and
initially populated (rather than steady state metrics) with some error detection.

Troubleshooting is the entry point rather than another sibling — it is symptom-first where `env-top` is subsystem-first, and every panel links onward into Logs & Events or the matching drilldown.
That makes it dependent on those existing, so it sequences last within OO-M2.

### Pipelines (Alloy)

Alloy carries both metrics and logs.
The agent and gateway pipelines are in place; the OTLP export path is the near-term work.

| Item | Milestone | Status |
|---|---|---|
| Typed Alloy **agent** pipeline | FCO-M1 | ✅ |
| Native **OTLP exporter** (forwarding workflows evaluated for Honeycomb, Datadog, Google Cloud Observability) | FCO-M2 | ✅ |
| Gateway pipeline (ported from the staging-gateway reference; log processing + loki.source.api / OTLP-log ingress) | FCO-M2 | ✅ |
| Loki (logs) + Thanos (metrics) wiring | FCO-M2 | ✅ |
| Agent **metrics path** + `prometheus.exporter.cadvisor` ([DEP-187](https://linear.app/materializeinc/issue/DEP-187)) — the agent is logs-only today | OO-M1 | ⬜ |
| Agent → gateway transport over **OTLP/gRPC with a node-local WAL** ([DEP-189](https://linear.app/materializeinc/issue/DEP-189); `hostPath`, compaction-bounded); gateway stays stateless and backend fan-outs are unchanged | OO-M3 | ⬜ |
| `otelcol.processor.transform` before the log bridge ([DEP-223](https://linear.app/materializeinc/issue/DEP-223)) — becomes load-bearing once agent logs arrive as OTLP | OO-M3 | ⬜ |
| [Backup log collection path](https://linear.app/materializeinc/issue/CLO-180) for alloy-agent failures — today an agent crash loses the logs explaining why | OO-M3 | ⬜ |

### Scraping (ScrapeConfigs & ServiceMonitors)

Metric collection is configured through two surfaces: **ScrapeConfigs** (consumed manually, e.g. dropped into a Prometheus/Agent config) and **ServiceMonitors / PodMonitors** (consumed by `prometheus-operator`, or by Alloy via `prometheus.operator.servicemonitor`; GCP uses `PodMonitoring`).
These ship as the released **Prometheus Scrapers** component and are bundled into the chart.

| Item | Milestone | Status |
|---|---|---|
| ScrapeConfigs (consumed manually) | FCO-M1 | ✅ |
| ServiceMonitors / PodMonitors (incl. GCP `PodMonitoring`) | FCO-M1 | ✅ |
| Sync scrapers into the charts and docs | FCO-M1 | ✅ |
| **cAdvisor on the bundled path** ([DEP-187](https://linear.app/materializeinc/issue/DEP-187)) — the shipped `ScrapeConfig` is only consumable by Prometheus, and Alloy has no `prometheus.operator.scrapeconfigs` equivalent, so the Kubernetes dashboards have no data on the default Alloy → Thanos path | OO-M1 | ⬜ |
| **node-exporter subchart** ([DEP-188](https://linear.app/materializeinc/issue/DEP-188)) — kept a separate workload rather than folded into the agent so its resource envelope stays known for bin-packed clusters. Ships on the `default` tag with a collector allowlist, a ServiceMonitor, a NetworkPolicy, and the `monitoring-critical` priority class | OO-M1 | ✅ |
| NetworkPolicy for Thanos / Grafana / Alloy / Alertmanager / kube-state-metrics ([DEP-192](https://linear.app/materializeinc/issue/DEP-192)) — Loki and node-exporter have one | OO-M1 | ⬜ |
| Generic `prometheus.io/scrape` discovery ([DEP-193](https://linear.app/materializeinc/issue/DEP-193)), default off, with exclusions generated from the same source as the monitors | OO-M3 | ⬜ |
| Move scrapers to the `materialize-operator` Helm chart ([DEP-221](https://linear.app/materializeinc/issue/DEP-221)) | OO-M3 | ⬜ (long-term) |

The cAdvisor and node-exporter rows are **parity gaps against the stack the Terraform repo shipped before the cutover**, which collected both.
They are functional gaps in the chart's own default path, not Terraform-specific — the Terraform work only makes the bundled path everyone's default, which is what moved them to OO-M1.
node-exporter has landed; cAdvisor is the remaining half, and until it does the Kubernetes container panels stay empty on the bundled path.

Long term, ServiceMonitors belong in the `materialize-operator` Helm chart rather than here.
This repo carries them now to fill the gap, with the intent to hand them off once the operator owns that surface.

### Charts / Helm

**Helm is prioritized over Terraform.**
The umbrella chart loads pre-rendered artifacts and bundles the productionalized stack as subcharts.

| Item | Milestone | Status |
|---|---|---|
| Subchart bundling: Loki, Thanos, Alertmanager, Grafana (+ operator), kube-state-metrics, metrics-server | FCO-M2 | ✅ |
| Generated chart README (values.yaml → README via `helm-docs`) | FCO-M2 | ✅ |
| Distroless Alloy image (FIPS boringcrypto, multi-arch, non-root, GHCR-published) | FCO-M2 | ✅ |
| Pre-install/pre-upgrade `alloy validate` validation hook | FCO-M2 | ✅ |
| Charts published to GHCR as OCI artifacts (`oci://ghcr.io/materializeinc/helm-charts`) + `.tgz` attached to each release | FCO-M3 | ✅ |
| Portable PVC defaults — Alertmanager's volume is sized by the cloud disk floor (4 GiB on GCP Hyperdisk and Azure) rather than by Alertmanager, which needs kilobytes | FCO-M3 | ✅ |
| **Grafana `ingress` / `service` values** ([DEP-196](https://linear.app/materializeinc/issue/DEP-196)) so Grafana is reachable at all — internal by default, public gated on an enforced allowlist, with the `grafana-ingress` profile as the assembled shape. Terraform wiring for cloud LB annotations is the remaining half | OO-M1 | 🔨 |
| **Grafana persistence** ([DEP-202](https://linear.app/materializeinc/issue/DEP-202)) — chart side done: `grafana-postgres` and `grafana-pvc` profiles, plus render-time checks that refuse multi-replica SQLite and RWO-with-rolling-update. Terraform provisioning the database per cloud is the remaining half | OO-M1 | 🔨 |
| **Grafana production shape** ([CLO-111](https://linear.app/materializeinc/issue/CLO-111)) — pinned image so Renovate bumps the server independently, resource requests, PDB, HPA, Image Renderer refused, unpinned-plugin and leaked-secret guards, `grafana.ini` documented as an arbitrary-config passthrough for SSO, and a `grafanaSpec` break-glass for `mode: operator` | OO-M1 | ✅ |
| **Pre-delete hook finalizing the Grafana custom resources** before grafana-operator is deleted, so teardown does not deadlock on finalizers with no remover ([DEP-197](https://linear.app/materializeinc/issue/DEP-197)) | OO-M1 | ⬜ |
| **cert-manager integration (opt-in)** ([DEP-195](https://linear.app/materializeinc/issue/DEP-195)) — `Certificate` resources for agent↔gateway and gateway/Grafana→Loki/Thanos mTLS, server-side TLS on the receiving halves, and file-mounted cert material so renewal takes effect. cert-manager stays an optional dependency the chart encourages rather than requires; the Terraform path enables it by default because that stack already ships it | OO-M1 | ⬜ |

Grafana reachability and persistence are paired deliberately.
Exposing Grafana without a durable backend turns a bundled extra nobody depended on into a primary interface that silently discards everything a user creates in it.

### Terraform

Designed in [Terraform Modules for materialize-monitoring](design-docs/20260803-terraform-modules/).
The **common module lives in this repo** (`terraform/modules/materialize-monitoring`), next to the chart whose value paths it encodes; **per-cloud wrapper modules** live in `materialize-terraform-self-managed` and wrap it.
This replaces the hand-rolled Prometheus + Grafana modules that repo shipped, which vendored a point-in-time dashboard copy and a legacy scrape config.

| Item | Milestone | Status |
|---|---|---|
| Design doc | FCO-M3 | ✅ |
| Common module (chart + CRDs flag, values composition, secrets, outputs) | FCO-M3 | ✅ |
| Terraform tooling in CI (`fmt`, `terraform-docs`, `validate`, and the tier-0 render check) + `terraform/` folded into the `materialize-monitoring` component | FCO-M3 | 🔨 (`tflint` not wired) |
| Per-cloud wrapper modules; retire the legacy modules downstream | FCO-M3 | 🔨 (AWS + GCP built; Azure in review) |
| Terraform install guide + tfvars reference + Terraform ↔ chart version compatibility row | FCO-M3 | 🔨 (in review) |
| Levers beyond the base install: `storage_class`, `google_cloud_metrics` (GCM fan-out with an importance tier), and a values hash that rolls Alloy on a config change | FCO-M3 | ✅ |
| Static object-storage credentials ([DEP-203](https://linear.app/materializeinc/issue/DEP-203)), so a consumer without workload identity does not need `additional_values` | OO-M1 | ⬜ |
| Expose the remaining destinations ([DEP-204](https://linear.app/materializeinc/issue/DEP-204)) — OTLP, Datadog, and the full `authType` set. The chart supports all of them; the module surfaces only `google_cloud_metrics` | OO-M1 | ⬜ |
| S3 account-regional namespaced buckets ([DEP-201](https://linear.app/materializeinc/issue/DEP-201)), blocked on the downstream AWS provider v6 upgrade | OO-M1 | ⬜ |

The module ships as part of the **`materialize-monitoring` component**, not as a component of its own — one version stream covering two artifacts, so `?ref=materialize-monitoring/vX.Y.Z` installs chart `vX.Y.Z` and there is no mapping to maintain between our own two surfaces.
The module derives its chart version from the chart's own `Chart.yaml`, so that coupling is structural rather than a convention someone maintains on each bump.

Qualification happens **here**, not downstream: the Terraform repo's cloud integration tests consume released tags and assume our changes are already qualified.
See [Testing / CI](#testing--ci--devex).

### Rules & alerts

The rule set ships; the routing that turns a firing rule into a page does not.

| Item | Milestone | Status |
|---|---|---|
| Base alert set (severity profiles + runbook stubs) | FCO-M2 | ✅ |
| Loki / Thanos rule sets ([DEP-117](https://linear.app/materializeinc/issue/DEP-117); recording rules first-class) | OO-M2 | ⬜ |
| Alertmanager adoption ([DEP-216](https://linear.app/materializeinc/issue/DEP-216)) — routing tree, receivers, grouping, inhibition, silences | OO-M2 | ⬜ |

Alertmanager is bundled and the rules exist, but nothing routes them anywhere.
Until that lands the alerting story is "we ship rules", which is half a feature.

### Profiles

The profile set is **deliberately not finalized** — final shape is an OO-M1 activity, tracked as one issue ([DEP-190](https://linear.app/materializeinc/issue/DEP-190)) rather than four.
The convention that has settled: **the chart defaults target a medium install**, and profiles are deltas away from it in both directions, each documenting the envelope it is sized for.
Loki follows this today; Thanos has no sizing profiles at all (the chart sets no resources or replica counts for it).

| Item | Milestone | Status |
|---|---|---|
| Loki sizing profiles (`small` / `large`, deltas from the medium defaults) | FCO-M2 | ✅ |
| Thanos sizing profiles (`small` / `large`), mirroring the Loki convention | OO-M1 | ⬜ |
| `kind` profile — CI-appropriate resource sizes only, no feature management, composable with the rest | OO-M1 | ⬜ |
| Scheduling profiles (nodeSelector / tolerations / priorityClassName) and a storage-class profile, fanned out to subcharts | OO-M1 | ⬜ |
| Profile-set finalization | OO-M1 | ⬜ |

Scheduling and storage class are profiles rather than a `global.*` block so the subchart fan-out map is inspectable data that snapshot tests can pin, instead of an unverified projection living in a downstream consumer.

### Testing / CI & DevEx

| Item | Milestone | Status |
|---|---|---|
| Pre-commit suite (ruff, pyright, shellcheck, yamllint, cargo fmt, helm-docs) | FCO-M1 | ✅ |
| Per-component versioning + changelog + release automation (see [Versioning](versioning/) / [Releasing](releasing/)) | FCO-M2 | ✅ |
| `auto-format` workflow (label-driven formatter fixups) | FCO-M2 | ✅ |
| `renovate` for automated dependency bumps | FCO-M2 | ✅ |
| Chart-shape fail-fast: Thanos + Alloy validators wired into `mzmon.validate.collect`, snapshot tests pinning rendered service-account names and workload-identity subject strings | FCO-M3 | ✅ |
| **Tier 0** — plan each Terraform example, extract the composed values, and render the chart against them (`make terraform-render`). Asserts values *land*, which `validate` cannot: a wrong value path is still valid HCL | FCO-M3 | ✅ |
| **kind E2E**, path-filtered behind `e2e-gate`: tier-1 chart variant on `loki-test` + `kind-tier1`; tier-2 generic-cloud substrate (rustfs + CNPG) | FCO-M3 | 🔨 (both bases land; tier-2 root composing substrate + module, and small/medium sizing, outstanding) |
| **Rust E2E suite** ([DEP-185](https://linear.app/materializeinc/issue/DEP-185), `packages/mz-monitoring-e2e`): Grafana API dashboard + datasource-query assertions, Loki / Thanos direct health, Alloy support-bundle inspection, WAL durability across a gateway outage | OO-M1 | ⬜ |
| ArgoCD / FluxCD CI matrix ([DEP-111](https://linear.app/materializeinc/issue/DEP-111), [DEP-118](https://linear.app/materializeinc/issue/DEP-118)) | OO-M3 | ⬜ (very low priority) |

The E2E suite subsumes what was previously tracked as a synthetic-data smoke test ([DEP-119](https://linear.app/materializeinc/issue/DEP-119), now closed as a duplicate).
It asserts **query success everywhere and non-empty results only on self-monitoring series** — Materialize scrapers stay off, since those are integration-tested downstream, so `env-top` assertions are structural while the stack's own telemetry provides real data.

The Rust suite deepens qualification rather than establishing it, which is why it sits in OO-M1 rather than gating the Terraform work.
Tier 0 and the kind tiers are what the Terraform modules close against.

### Observability for our own components

The stack has better visibility into Materialize than into the things that run Materialize.

| Item | Milestone | Status |
|---|---|---|
| [More monitoring for the k8s controllers](https://linear.app/materializeinc/issue/CLO-55) — orchestratord reconciliation timing, stalls, errors and successes, optional timeouts | OO-M2 | ⬜ |
| [Environment lifecycle visibility](https://linear.app/materializeinc/issue/CLO-188) — is a new environment coming up, what is the status of every environment, did bootstrapping succeed and at which step | OO-M2 | ⬜ |

Reconciliation timing is the gap that matters most: today a stalled reconciliation is invisible until someone notices the effect of it downstream.

The two rows are substrate and projection.
Reconciliation metrics describe the mechanics of the controller loop; environment lifecycle describes the objects that loop manages, which is what an operator actually asks about.
Every other dashboard here is scoped to a single environment — nothing today answers "how are all of my environments doing".

This section absorbed a separate project on orchestratord Day 1/2 metrics, closed as a duplicate.
Everything else it scoped — upgrade progress, reconcile monitoring, error rates, installation prerequisites, Day 2 dashboards — was already covered by rows above.

### Adoption / productionalization

FCO's target was a productionalized deployment for Cloud, an internal team, and initial external adopters.
(Specific adopter commitments are tracked out-of-band, not in this public roadmap.)

| Item | Milestone | Status |
|---|---|---|
| Fork source repo and archive the original | FCO-M1 | ✅ |
| Product observability documentation fully replaced (rewrite the recommended path; migration guide off the legacy SQL-exporter surface) | FCO-M2 | ✅ |
| Productionalized for Cloud + internal + initial external adopters ([DEP-122](https://linear.app/materializeinc/issue/DEP-122)) | FCO-M3 | 🔨 |
| Internal monitoring migrated to consume this repo via `values.yaml` ([DEP-125](https://linear.app/materializeinc/issue/DEP-125)) | OO-M3 | ⬜ |
| Adopt in Materialize Cloud via Pulumi ([CLO-182](https://linear.app/materializeinc/issue/CLO-182)) | OO-M3 | ⬜ |

Cloud deploys through Pulumi rather than Terraform or raw Helm, so neither existing consumption path reaches it.
The two Cloud rows are complements: one is *what* Cloud deploys, the other is *how* it gets deployed.

### BYOC

| Item | Milestone | Status |
|---|---|---|
| [Gateway-to-gateway architecture and proposal](https://linear.app/materializeinc/issue/DEP-219) — design doc plus review | OO-M3 | ⬜ |
| [Dual-destination pipeline pattern](https://linear.app/materializeinc/issue/DEP-124) (customer-local + control plane) | OO-M3 | ⬜ |
| [Sanitize telemetry before control-plane egress](https://linear.app/materializeinc/issue/DEP-220) | OO-M3 | ⬜ |

Logs stay inside the customer network; metrics may cross.
The gateway pair is what enforces that boundary, rather than ad-hoc network configuration — and sanitization is what makes "metrics may cross" true, since the `_info` metrics that made dashboards legible are precisely the ones carrying customer names.

## Metrics contract (upstream dependency)

Several dashboards depend on metric instrumentation that lives **upstream in the `materialize` repo, not in this repository**.
The metric/label contract is the public API for everything here, so this dependency shapes the dashboard roadmap directly.
Tracked as [DEP-207](https://linear.app/materializeinc/issue/DEP-207).

The environmentd-native public metrics endpoint delivered **Tier 1** (pre-aggregating clusterd counters into environmentd).
The carry-over is **Tier 2**: roughly 39 signal families that today exist *only* via the SQL-on-scrape sources slated for deletion (legacy `/metrics/mz_*` and the `v2_mz_*` exporter).
To retire those sources, environmentd must emit these natively.
High-leverage asks, in priority order:

- ✅ **`mz_object_info`** (id → fully-qualified name → type) — the single highest-leverage item; **delivered upstream**.
  It gives every other metric a stable `group_left` join target for names.
- ✅ A family of **`_info` metrics** (`mz_cluster_info`, `mz_replica_info`, `mz_source_info`, `mz_sink_info`, …) carrying names and parent-id references; **delivered upstream**.
- ⬜ Native **source/sink status** metrics (no genuine source exists today).
- ⬜ Native **hydration** and **frontier/freshness** signals.
- ⬜ **Label-family harmonization** (short vs long vs very-long forms).

The `_info` family is now available, so name enrichment is unblocked for every panel.
The remaining drilldowns are still ⛓️ gated on the items above: **Sources / Sinks** await native status metrics, and **Hydration / Freshness** await the hydration and frontier signals.

## Versioning, changelog, and releases

**Built.** Each artifact has its own SemVer stream — the Helm chart, the optional CRDs chart, dashboards, pipelines, scrapers, and the shared lib — declared in `packages/components.yaml`.
Full mechanics are in [Versioning](versioning/) and [Releasing](releasing/); this replaces the earlier single-umbrella-chart framing.

- **Per-component streams.** ✅
  Merged PRs are attributed to components by path; `CHANGELOG.md` is the source of truth, with cumulative `Included <dep> @ vPREV..vNEW` dependency rollups.
- **Automation.** ✅
  `mz-monitoring-build propose-bumps` opens one `version-update/<component>` PR per changed component on each merge to main; `publish-release` tags `<component>/vX.Y.Z` and creates a GitHub Release (attaching each component's `artifacts`) when such a PR merges.
- **Downstream pinning.** ✅
  The Terraform modules pin a specific chart version, so Terraform never tracks a moving target.
  The common module ships from this repo **inside the `materialize-monitoring` component** rather than on a stream of its own: the chart version is the release version, and the module's Git tag is that same version.
  Per-cloud wrappers downstream pin the module by Git ref, so a single number identifies both surfaces and there is no window where the pair is mismatched.
  The trade is that a Terraform-only change publishes a chart release, and a breaking module change bumps the chart's major — both handled in the changelog rather than by splitting the stream.
- **Deprecation policy.** ⬜ ([DEP-127](https://linear.app/materializeinc/issue/DEP-127), OO-M1)
  Still to commit: at least one minor-release cycle for breaking changes to the label/metric contract, with a release-process check, and a called-out "customer-facing surface" changelog subsection.
- **Stamping 1.0.** ⬜ ([DEP-205](https://linear.app/materializeinc/issue/DEP-205), OO-M1)
  The [pre-1.0 bump policy](releasing/#choosing-the-next-version) lets breaking changes ride minors.
  At 1.0 that stops, and the label/metric contract, profile semantics, alert names, and chart value paths all acquire a deprecation cycle we owe.
  The window closes on its own: once enough customers have dashboards built on these labels the contract is frozen in practice whether or not it is frozen on paper, so the discipline should land before broad adoption rather than after.

## Follow-up documentation

- [Releasing](releasing/) and [Versioning](versioning/) are written, covering the release mechanics and the per-component model. ✅
- `CHANGELOG.md` exists and is maintained by the release tooling. ✅
- A **customer-facing** contract/deprecation-policy page (in customer terms, distinct from the internal `versioning.md`) is still to write. ⬜
- [Repo Layout](repo-layout/) refreshed against the tree (August 2026), including `terraform/` and `test/`. ✅
  It goes stale easily by design — re-check it whenever a top-level directory moves.
- [Uninstalling](../../../operating/uninstalling/) is written: the grafana-operator finalizer deadlock, the ordered teardown, and recovery. ✅
- [Choosing the next version](releasing/#choosing-the-next-version) records the pre-1.0 bump policy and that the changelog placeholder heading is the decision. ✅
- Alloy's rollout requirement is called out in [Production Best Practices](../../../operating/production-best-practices/#collection-alloy) as an inversion of the normal chart guarantee — the one place the chart cannot own its own rollout. ✅
- A BYOC gateway-to-gateway design doc is owed under `design-docs/`, tracked as [DEP-219](https://linear.app/materializeinc/issue/DEP-219). ⬜
