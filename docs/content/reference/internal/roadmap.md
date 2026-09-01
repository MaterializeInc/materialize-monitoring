---
title: "Roadmap"
weight: 60
---

<!-- This roadmap is public. Do not include customer-specific or sensitive information -->

# Roadmap

The goal of `materialize-monitoring` is **first-class, composable observability for self-managed Materialize** — logs, metrics, events, and alerts — best-practices-by-default for customers who want a one-stop-shop, without forcing our stack on customers who already run their own.
Composable is the operative word: the same stack is consumed through Helm with a lot of levers, through our Terraform module, and through the downstream per-cloud Terraform wrappers, and every component can be turned off in favour of one a customer already runs.
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
| Grafana dashboards via Jsonnet/Grafonnet (`sources/jsonnet/`) | Rust against the vendored Grafana schemas (`packages/dashboards/`) |
| `crates/` Rust workspace + `sources/` input tree | `packages/` monorepo; Rust throughout after the Python dashboards were retired |
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
| [Logs & Events](https://linear.app/materializeinc/issue/DEP-209) (Loki + Alloy + logs now shipped) | OO-M2 | ✅ (`env-logs` dashboard ships, `mz-mon-env-logs`, with **Logs** and **Events** tabs. The repo's first Loki-only dashboard: it defines no metrics datasource and its namespace / app / level pickers are Loki-discovered, so it keeps working when the metrics pipeline is the thing being investigated. Materialize-first rather than Materialize-only — the namespace picker discovers every namespace and merely *defaults* to the Materialize ones, so the monitoring stack and `kube-system` are one selection away rather than behind a filter. That is where the answer lives when collection itself is what broke. Ports the shape of the internal cloud dashboards (namespace/app/level pickers, case-insensitive search box, ad-hoc filter) **without** their Snowflake org lookup, which does not exist on self-managed; `organization_name` structured metadata is the equivalent where one is needed. Its Kubernetes-event queries are deliberately separate definitions from `env-upgrade`'s rollout-scoped ones, which carry generation and reporting-controller filters a general browser must not inherit. Verified against a live self-managed install. **Later:** a per-pod drilldown, and log-derived alerting — neither blocks the dashboard, which is why this is done) |
| [Upgrades](https://linear.app/materializeinc/issue/DEP-210) (Day 2 ops) — **customer-blocking**, see below | OO-M2 | 🔨 (`env-upgrade` dashboard exists, `mz-mon-env-upgrade`, with three tabs: **Events**, **Generations**, and **Reconciliation**. This is the repo's first dashboard on Loki, and the first LogQL family in the query registry — Kubernetes events from the operator and environment namespaces, with orchestratord's own lifecycle transitions and reconciliation failures picked out by `reportingcontroller`. **Generations** splits a blue/green rollout into its two sides via a new `$mzGenerationList` selector, so the question the rollout actually poses — has the new generation caught up yet — can be asked at all; its centrepiece is the hydrating-collection count descending toward zero, beside a version-per-generation table that states what the rollout is changing. Verified against a live rollout: gen 2 on `v26.38.2` beside gen 3 on `v26.40.0-rc.1`, 116 of 191 collections hydrating. **Reconciliation** is the operator's control loop as metrics — pass outcomes, durations, and the per-step counters that turn "reconciliation is failing" into "reconciliation is failing *here*". The two tabs are scoped differently on purpose: events are per-resource and therefore per-environment, while the operator's metrics carry no organization label at all, because one operator reconciles every environment in the cluster. Together they answer *is it stuck* from the rollout's own account of itself rather than from version counts. **Installed by default**, since the stem matches the `["env-*"]` pattern `dashboards.selected` ships with. The operator-side events and metrics depend on unreleased Materialize changes ([CLO-188](https://linear.app/materializeinc/issue/CLO-188)), so against a current release it degrades unevenly rather than going dark: **Generations works fully** (its panels read metrics that predate the change, and the blue/green split comes from pod names), Events keeps its Kubernetes Activity row, and Reconciliation is empty but for its two pre-existing gauges. Requires `v26.41.0`, recorded in `compatibility.md`; narrow `dashboards.selected` to `["env-top"]` to hold it back. **Outstanding:** *what do I do about it* beyond what the panel descriptions say, and a Day 2 change-operations view) |
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
| [Replace dashboard management with a Rust implementation](https://linear.app/materializeinc/issue/DEP-222) | OO-M3 | 🔨 (`env-top` is rendered by Rust and is what ships: `mz-monitoring-build gen-dashboards` writes both the chart's YAML and the docsite's JSON, and the `dashboards` workflow asserts the checked-in output is fresh. All 69 panels ported, pinned against the frozen final Python render with seven allow-listed shell divergences — the threshold base step, the errors/load respacing, `cursorSync`, the `$environmentNameList` rename, the variable order, and the `liveNow` / built-in-annotations fields Grafana writes on save — plus seven description fixes, six of them broken cross-references in the baseline's own prose. Cloud variants are retired: they stopped differing in panel content once the gateway began scraping the kubelet's cAdvisor directly instead of consuming GKE's reduced allowlist, so the eleven panels the Python branched on collapsed to a bare `target-cloud` annotation, and that annotation and the `--cloud` / `--prefix` flags behind it were removed. No Makefile target or CI job renders from the Python any more. Panels take both their expressions and their descriptions from the query registry rather than restating them, which surfaced and fixed four real registry defects (a missing job-dedup on 14 rates, a lost regex-escape on the pod matchers, a missing exporter-including capacity query, and a missing name-join on system arrangements). `packages/grafana-dashboards` and `py-mzmon-lib` are deleted; nothing in the repo renders a dashboard from Python. **Outstanding:** the Grafana 11 (v1) gallery renders. The Python had a `to_v1()` path but never wired it to an artifact, so no v1 dashboard has ever shipped — the docsite's v1 table has always been empty) |

### Infrastructure dashboards (`infra-*`)

The table above is the **product** ask: dashboards a Materialize user needs, all of them scoped to an environment and
all named `env-*`.
An infrastructure admin running the cluster underneath has a different set.

The family is `infra-*`, scoped to the cluster rather than to an environment.
`infra-logs` is the first of it and is built; the rest below are still proposals.
Both questions the family raised are settled:

- **`infra-*` ships by default**, alongside `env-*`, in `dashboards.selected`.
  The alternative — useful to whoever operates a self-managed install, noise to whoever only runs Materialize on
  someone else's platform — lost to the fact that a self-managed installation has no other operator.
- **The two audiences are split by default rather than by a switch.**
  `env-logs` opens on the Materialize namespaces and `infra-logs` subtracts them, so each dashboard answers for one
  audience while still reaching the other in a click.

None of the below is ticketed yet.
The "collectable today" column is what a live self-managed install actually exposes, measured rather than assumed —
it is the difference between a dashboard we could build this week and one that needs collection work first.

| Item | Collectable today | Milestone | Status |
|---|---|---|---|
| **Logs & events** — the platform's own logs, the node journal, cluster-wide events | **Yes, shipped.** `infra-logs` (`mz-mon-infra-logs`) exists with **Logs**, **Nodes** and **Events** tabs. Adds the two axes `env-logs` structurally cannot offer: `component`, which splits `loki` into eight processes and `thanos` into three, and the **node journal**, whose lines carry no namespace and so are excluded from every `env-*` selector by construction. Shares its event queries and variable names with `env-logs`; differs in where the namespace picker opens and in carrying an **Exclude Materialize** switch, on by default — the Materialize namespaces are about half of a week's log volume and `materialize-environment` alone out-logs every other namespace, so left in they make the volume panels a picture of Materialize rather than of the platform underneath it. Verified against a live install | OO-M3 | ✅ |
| **Nodes** — health, workloads, bin capacity (logs and journals now covered by `infra-logs`) | **Yes, shipped.** `infra-nodes` (`mz-mon-infra-nodes`) exists with **Summary**, **CPU**, **Memory & Swap**, **Network**, **Storage**, **Pods** and **Logs & Events** tabs, scoped to one node at a time. Built on 282 `node_*` families, the 9 `kube_node_*` ones the KSM label fix unblocked, and the node journal. The Summary tab is deliberately `kubectl describe node` for someone who cannot run it: identity and capacity as info cells, utilization sparklines, radial gauges for how much of the node the scheduler has already promised, and the conditions, cordon state and taints that explain a node accepting no work. It is also the repo's first dashboard to join two identifier conventions — kube-state-metrics names a node `node`, node-exporter names the same machine `instance` — through a hidden `$nodeList` lookup over `node_uname_info`, which let the pre-existing `node-health` and `node-debug` families back it unchanged. Those two families were vetted in the process: all 87 of their expressions were run against a live cluster and all 87 returned data. The **Pods** tab answers what is scheduled here and whether it is well, with requests beside limits — the scheduler's reservation beside the kernel's ceiling. **Later:** fleet views, which are a different question (*which* node) rather than a wider version of this one | OO-M3 | ✅ |
| **Pods** — health, logs, metrics for a single workload | **Partly.** The cAdvisor families are fine; the 41 `kube_pod_*` ones are all present but keyed under `exported_pod` / `exported_namespace`, so a pod picker cannot be built on them until the KSM label collision is fixed | OO-M3 | ⬜ |
| **Meta-monitoring** — every component of this stack | **Yes, and then some.** Grafana 523 families, Loki 439, Thanos 220, Alloy 35. **Alertmanager exposes 0** — it is deployed and not scraped | OO-M3 | ⬜ |
| **Autoscaling** — utilization, controller status, compute cost proxy | **Events only.** No `cluster_autoscaler_*` or `karpenter_*` metrics reach Thanos, but 15 event reasons do (`TriggeredScaleUp`, `NotTriggerScaleUp`, `FailedScaleUp`, `ScaleDown`, `RegisteredNode`, `RemovingNode`, …). HPA is covered by 10 `kube_horizontalpodautoscaler_*` families | OO-M3 | ⬜ |
| **Networking** — throughput, traffic shape, policy metrics | **Partly.** 8 `container_network_*` families carry throughput and errors (`env-top` already plots four of them). **No CNI or NetworkPolicy metrics at all** — no `cilium_*`, no `hubble_*` | OO-M3 | ⬜ |
| **External components** — object store, consensus DB | **Object store yes** (24 `loki_objstore_*`, plus the Thanos equivalents). **Consensus DB effectively no** — only 2 `postgres_exporter_*` *config* metrics land, no database statistics | OO-M3 | ⬜ |

Ordering follows what is buildable and what an operator reaches for first.
**Nodes** and **Meta-monitoring** need no collection work and answer the two questions that block everything else — is
the platform healthy, and is the telemetry itself trustworthy.
**Pods** is the natural drilldown target from both, and from `env-logs`.
**Autoscaling** is buildable now as an events dashboard and becomes a real one when the controller is scraped.
**Networking** and **External components** are gated on the collection gaps below.

#### Collection gaps these depend on

Found while surveying a live install; each is a scrape-side gap in *this* repo rather than an upstream metric contract.

| Gap | Blocks | Notes |
|---|---|---|
| ✅ **kube-state-metrics label collision** — fixed, and now asserted | *was:* Pods, Nodes, and five shipped `env-top` panels | KSM emits its own `namespace` / `pod` / `container` labels describing the object it reports on. The scrape does not set `honorLabels`, so Prometheus's collision rule renames them `exported_namespace` / `exported_pod` and puts the *KSM pod's own* identity in `namespace` / `pod`. Every series therefore reads `namespace="monitoring"`. The data is all there — `kube_pod_info` has 105 series, `kube_pod_status_ready` 315 — but every query in this repo written as `kube_*{namespace=…}` matches nothing. Fixed by setting `honorLabels: true` on both ServiceMonitor endpoints (the vendored 8.4.0 subchart defaults them to `false`). Now asserted by `kube_state::labels_are_honored` and `kube_state::pods_are_distinguishable` in the e2e suite, which between them catch the collision's signature (`exported_*` labels) and its consequence (`kube_pod_info` collapsing to one identity). Verified against a live install *before* the fix: 15 families carrying `exported_*`, and 117 `kube_pod_info` series reporting a single pod. The three `materialize.kubernetes.*` readiness queries are `canonical` on the strength of that coverage |
| Alertmanager is not scraped | Meta-monitoring | Deployed by the chart and emitting nothing to Thanos. The smallest of these and the most embarrassing, since it is our own component |
| No cluster-autoscaler / Karpenter scrape | Autoscaling | Distro-specific: GKE's autoscaler, Karpenter and Cluster Autoscaler each expose different endpoints, so this is a per-flavor scrape source rather than one config |
| No CNI / NetworkPolicy metrics | Networking (policy half) | Needs a CNI that exports them and a scrape source for it. Throughput and traffic shape do not depend on this and can land first |
| Consensus DB exports config only | External components | `postgres_exporter` is present but only its own config metrics arrive; the database statistics it exists to publish do not |

### Materialize components beyond the environment

`env-top` covers `environmentd` and `clusterd`.
The rest of a Materialize deployment has no dashboard, and for the two public ones the reason is the same: **they
expose no metrics that reach Thanos.**
Both appear in `env-logs` and in `env-upgrade`'s reconciliation counters, so today they are observable only as logs and
as the operator's opinion of them.

| Item | Collectable today | Milestone | Status |
|---|---|---|---|
| **balancerd** | Logs only — 0 metric families | OO-M3 | ⛓️ |
| **console** | Logs only — 0 metric families | OO-M3 | ⛓️ |

⛓️ rather than ⬜ because the gap is upstream instrumentation, not collection: there is nothing to scrape.
Both are listed in [Metrics contract](#metrics-contract-upstream-dependency) alongside the other asks.

A further set of Materialize **cloud services** would want dashboards too.
They are deliberately not enumerated here: this repository is public, and neither their names nor their metric
surfaces belong in it.
Track them in the internal planning docs (internal), and if any of them ever ships in a self-managed deployment it
earns a row above instead.

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
The agent and gateway pipelines are in place, the OTLP export path shipped with FCO-M2, and the metrics half now delivers — the gateway scrapes every kubelet's cAdvisor endpoint. The near-term work is the OTLP/gRPC agent→gateway transport with a node-local WAL.

| Item | Milestone | Status |
|---|---|---|
| Typed Alloy **agent** pipeline | FCO-M1 | ✅ |
| Native **OTLP exporter** (forwarding workflows evaluated for Honeycomb, Datadog, Google Cloud Observability) | FCO-M2 | ✅ |
| Gateway pipeline (ported from the staging-gateway reference; log processing + loki.source.api / OTLP-log ingress) | FCO-M2 | ✅ |
| Loki (logs) + Thanos (metrics) wiring | FCO-M2 | ✅ |
| Agent **metrics path** ([DEP-187](https://linear.app/materializeinc/issue/DEP-187)) — superseded rather than deferred: `prometheus.exporter.cadvisor` was removed from the agent, and the gateway scrapes each kubelet's `/metrics/cadvisor` instead. A per-agent cAdvisor cost ~750Mi against a 200Mi logs-only envelope, so the agent stays logs-only by design | OO-M1 | ✅ |
| **Multiple Prometheus remote-write destinations** ([DEP-232](https://linear.app/materializeinc/issue/DEP-232)) — Cloud's transition needs Thanos and Amazon Managed Prometheus written simultaneously, each on its own importance tier | OO-M1 | ✅ (`pipeline.metrics.gateway.destination.prometheusRemoteWrite` is a map keyed by name, defaulting to one `thanos` entry. Each destination renders its own `prometheus.relabel` tier filter feeding its own `prometheus.remote_write`, rather than one component with several `endpoint` blocks: the filter has to sit *upstream* of the WAL for a tier to reduce disk rather than only egress, and separate WALs keep a stuck backend from holding back truncation for the others. `external_labels`, auth, TLS and the credential env vars are all per destination, derived from the name. The tier filter is new on this path — `GATEWAY_UNFILTERED_PROM_METRICS` was written to the env ConfigMap and read by nothing, so `minMetricImportance` on remote-write had never done anything. A leftover pre-map key fails at render, because Helm would otherwise merge it beside `thanos` and silently apply it to nothing) |
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
| **cAdvisor on the bundled path** ([DEP-187](https://linear.app/materializeinc/issue/DEP-187)) — the shipped `ScrapeConfig` is only consumable by Prometheus, and Alloy has no `prometheus.operator.scrapeconfigs` equivalent, so the Kubernetes dashboards had no data on the default Alloy → Thanos path | OO-M1 | ✅ (the gateway scrapes `/metrics/cadvisor` on every kubelet, on by default. Verified on EKS: 7/7 targets up, ~24.6k `container_*` series. kind additionally needs `pipeline.metrics.kubelet.tlsInsecureSkipVerify`, since it signs kubelet certs with a CA the pods do not trust — set in the tier-2 root, not the chart defaults, because leaving verification on is correct everywhere real) |
| **node-exporter subchart** ([DEP-188](https://linear.app/materializeinc/issue/DEP-188)) — kept a separate workload rather than folded into the agent so its resource envelope stays known for bin-packed clusters. Ships on the `default` tag with a collector allowlist, a ServiceMonitor, a NetworkPolicy, and the `monitoring-critical` priority class | OO-M1 | ✅ |
| NetworkPolicy for Thanos / Grafana / Alloy / Alertmanager / kube-state-metrics ([DEP-192](https://linear.app/materializeinc/issue/DEP-192)) — Loki and node-exporter have one | OO-M1 | ✅ (every workload now carries one, on by default. Thanos, Grafana, both Alloy roles and kube-state-metrics through their subcharts' own `networkPolicy` values; Alertmanager, grafana-operator and metrics-server through `templates/networkpolicies.yaml`, since those three ship none upstream. Plus the Grafana alerting-gossip rule the Grafana subchart's template cannot express. Validators follow the Loki pattern: a warning per policy switched off, errors on the shapes that render a policy which silently cannot work) |
| Generic `prometheus.io/scrape` discovery ([DEP-193](https://linear.app/materializeinc/issue/DEP-193)), default off, with exclusions generated from the same source as the monitors | OO-M3 | ⬜ |
| Move scrapers to the `materialize-operator` Helm chart ([DEP-221](https://linear.app/materializeinc/issue/DEP-221)) | OO-M3 | ⬜ (long-term) |

The cAdvisor and node-exporter rows are **parity gaps against the stack the Terraform repo shipped before the cutover**, which collected both.
They are functional gaps in the chart's own default path, not Terraform-specific — the Terraform work only makes the bundled path everyone's default, which is what moved them to OO-M1.
Both have landed and both deliver: ~3.7k `node_*` and ~14k `container_*` series on the tier-2 cluster, and ~24.6k `container_*` on a real EKS cluster.

That closes the parity gap, and it makes `container_*` and `node_*` assertions writable in the E2E suite for the first time — worth adding, because the failure mode across this whole area is *configured but returning nothing*, which no render test can see. Note the trap the chart already documents: a distribution signing kubelet certs with an untrusted CA fails the scrape quietly, so `up{job="cadvisor"}` is the thing to check when bringing up a new one.

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
| **Pre-delete hook finalizing the Grafana custom resources** before grafana-operator is deleted, so teardown does not deadlock on finalizers with no remover ([DEP-197](https://linear.app/materializeinc/issue/DEP-197)). `cleanup.grafanaOperator` runs one `kubectl delete` at `pre-delete` and blocks until the finalizers clear, on upstream's distroless kubectl image | OO-M1 | ✅ |
| **cert-manager integration (opt-in)** ([DEP-195](https://linear.app/materializeinc/issue/DEP-195)) — `Certificate` resources for agent↔gateway and gateway/Grafana→Loki/Thanos mTLS, server-side TLS on the receiving halves, and file-mounted cert material so renewal takes effect. cert-manager stays an optional dependency the chart encourages rather than requires; the Terraform path enables it by default because that stack already ships it | OO-M1 | 🔨 (issuance shipped and off by default: `certificates.enabled` renders per-component `Certificate`s with the full SAN ladder, an opt-in self-signed root, and a separate external issuer for a Grafana behind an L4 LB. `global.clusterDomain` lands with it and propagates into Loki and Thanos. Material is mounted, not env-injected, and the `tls.*File` carriers plus scheme derivation are wired on the gateway's own destinations — `alloy validate` passes on the TLS-enabled render. Server halves shipped for both: `profiles/mtls.values.yaml` assembles Loki's six coupled settings and Thanos Receive's two, and a validator refuses every half-applied combination — `alloy validate` passes on the rendered result. All four gateway ingress listeners — 3100, 4317, 4318, 9090 — now render from Helm and take TLS from values; moving `prometheus.receive_http` out of the pre-rendered pipeline was the last blocker, so **agent→gateway is shipped**. Phases 2 and 3 ship as `mtls-phase2` / `mtls-phase3`, and at phase 3 five listeners refuse a client presenting no certificate. Terraform exposes the whole rollout as `internal_tls` (`off`/`encrypt`/`present`/`authenticate`) and `materialize-terraform-self-managed` defaults to certificates on and `authenticate`. Tier 2 runs phase 3 by default at chart-default lifetimes — the earlier 1h/55m livelocked cert-manager, so renewal is **forced** by deleting the Secret instead. The E2E suite has its own TLS client and asserts issuance, expiry, plaintext refusal, client-certificate enforcement and delivery across a forced renewal; verified on kind and on real EKS and GKE. **Outstanding:** the trust bundle ([DEP-236](https://linear.app/materializeinc/issue/DEP-236)); intra-Loki and intra-Thanos hops; and Loki's HTTP port, which cannot pass phase 2 because the kubelet probes it) |

Grafana reachability and persistence are paired deliberately.
Exposing Grafana without a durable backend turns a bundled extra nobody depended on into a primary interface that silently discards everything a user creates in it.

### Terraform

Designed in [Terraform Modules for materialize-monitoring](../design-docs/20260803-terraform-modules/).
The **common module lives in this repo** (`terraform/modules/materialize-monitoring`), next to the chart whose value paths it encodes; **per-cloud wrapper modules** live in `materialize-terraform-self-managed` and wrap it.
This replaces the hand-rolled Prometheus + Grafana modules that repo shipped, which vendored a point-in-time dashboard copy and a legacy scrape config.

| Item | Milestone | Status |
|---|---|---|
| Design doc | FCO-M3 | ✅ |
| Common module (chart + CRDs flag, values composition, secrets, outputs) | FCO-M3 | ✅ |
| Terraform tooling in CI (`fmt`, `terraform-docs`, `validate`, and the tier-0 render check) + `terraform/` folded into the `materialize-monitoring` component | FCO-M3 | 🔨 (`tflint` not wired) |
| Per-cloud wrapper modules; retire the legacy modules downstream | FCO-M3 | ✅ (all three clouds shipped in `materialize-terraform-self-managed` v11, which also flipped `enable_observability` to **on by default** — opt-out rather than opt-in. v11 pins chart 0.17.0; the legacy Prometheus + Grafana modules are retired) |
| Terraform install guide + tfvars reference + Terraform ↔ chart version compatibility row | FCO-M3 | ✅ |
| Levers beyond the base install: `storage_class`, `google_cloud_metrics` (GCM fan-out with an importance tier), and a values hash that rolls Alloy on a config change | FCO-M3 | ✅ |
| Static object-storage credentials ([DEP-203](https://linear.app/materializeinc/issue/DEP-203)), so a consumer without workload identity does not need `additional_values` | OO-M1 | ✅ (`object_storage_access_key_id` / `_secret_access_key`; tier-0 asserts they reach both backends and that Loki's config becomes a Secret rather than a ConfigMap) |
| Expose the remaining destinations ([DEP-204](https://linear.app/materializeinc/issue/DEP-204)) — OTLP, Datadog, and the full `authType` set. The chart supports all of them; the module surfaced only `google_cloud_metrics` | OO-M1 | ✅ (`otlp_metrics` and `datadog_metrics` join it, each with the same `min_importance` tier. Credentials deliberately do **not** travel through the values — `helm get values` reads those, and they land in Terraform state besides — so the module creates the `mzmon-alloy-gateway-env` Secret the chart mounts `optional: true`, and writes only the *names* of the variables into the values. `otlp_auth_header_secrets` derives its variable name from the header, which is the pattern DEP-232 reuses per destination. A credential change rolls the gateway, since `envFrom` is fixed at container start. Tier 0 asserts each exporter reaches the pipeline and that the credentials are read by it and absent from the release. **Not** surfaced: the OTLP `basic`, `awsSigv4` and `custom` auth types — the module derives `authType` from which credential input is set, so those stay `additional_values` territory) |
| Remote-write destinations through Terraform ([DEP-232](https://linear.app/materializeinc/issue/DEP-232)) — `prometheus_remote_write`, its credentials, and the gateway ServiceAccount annotation SigV4 needs | OO-M1 | ✅ (a map keyed by the same destination name the chart uses, so `thanos` retunes the bundled destination rather than adding a second. Credentials go through the `mzmon-alloy-gateway-env` Secret with the module naming the variables explicitly, per DEP-204's pattern. Tier 0 asserts each declared destination reaches a component, a fan-out edge, its URL variable, and a tier variable that actually differs from `.*`) |
| S3 account-regional namespaced buckets ([DEP-201](https://linear.app/materializeinc/issue/DEP-201)), blocked on the downstream AWS provider v6 upgrade | OO-M1 | ⬜ |

The module ships as part of the **`materialize-monitoring` component**, not as a component of its own — one version stream covering two artifacts, so `?ref=materialize-monitoring/vX.Y.Z` installs chart `vX.Y.Z` and there is no mapping to maintain between our own two surfaces.
The module derives its chart version from the chart's own `Chart.yaml`, so that coupling is structural rather than a convention someone maintains on each bump.

Qualification happens **here**, not downstream: the Terraform repo's cloud integration tests consume released tags and assume our changes are already qualified.
See [Testing / CI](#testing--ci--devex).

### Rules & alerts

The rule set ships; the routing that turns a firing rule into a page does not.

| Item | Milestone | Status |
|---|---|---|
| Base alert set (severity profiles + runbook stubs) | FCO-M2 | 🔨 (the alert **definitions** live in the query registry — `packages/queries/materialize-alerts.yaml` and `infra-alerts.yaml` — and render to the docsite as [Common Alerts](../../stable-metrics/common-alerts/). They are **not shipped as rules**: `config.rules.prometheus.enabled` defaults true but `pre-rendered/rules/prometheus/` is empty and no template emits a `PrometheusRule`, so an install gets no alerts. Previously marked ✅ on the strength of the documentation) |
| Loki / Thanos rule sets ([DEP-117](https://linear.app/materializeinc/issue/DEP-117); recording rules first-class) | OO-M2 | ⬜ |
| Alertmanager adoption ([DEP-216](https://linear.app/materializeinc/issue/DEP-216)) — routing tree, receivers, grouping, inhibition, silences | OO-M2 | ⬜ |
| Alertmanager production hardening ([DEP-226](https://linear.app/materializeinc/issue/DEP-226)) — HA via gossip, resource requests, storage shape, topology spread | OO-M2 | ⬜ |

Alertmanager is bundled and the rules exist, but nothing routes them anywhere.
Until that lands the alerting story is "we ship rules", which is half a feature.

The two Alertmanager items split along "reaching a human" versus "surviving a bad day", and are best worked together.
Adoption is the higher-value half — until routing exists nobody is paged, which is why hardening is the lower priority of the pair despite Alertmanager being a single replica holding the only copy of its silences.

### Profiles

The profile set is **deliberately not finalized** — final shape is an OO-M1 activity, tracked as one issue ([DEP-190](https://linear.app/materializeinc/issue/DEP-190)) rather than four.
The convention that has settled: **the chart defaults target a medium install**, and profiles are deltas away from it in both directions, each documenting the envelope it is sized for.
Loki and Thanos both follow it now.

| Item | Milestone | Status |
|---|---|---|
| Loki sizing profiles (`small` / `large`, deltas from the medium defaults) | FCO-M2 | ✅ |
| Thanos sizing profiles (`small` / `large`), mirroring the Loki convention | OO-M1 | ✅ (`small` had never actually been installed until tier 2 did it, and did not start: `--index-cache-size` alone leaves `max_item_size` at the 125MiB default, so any total below that fails validation. Fixed, and the unit test now pins the full `--index-cache.config`) |
| `kind` profile — CI-appropriate resource sizes only, no feature management, composable with the rest | OO-M1 | ✅ (`kind`, plus `kind-tier1` for the hermetic E2E shape and `no-zone-spread` for clusters whose nodes carry no zone label) |
| Scheduling profiles (nodeSelector / tolerations / priorityClassName) and a storage-class profile, fanned out to subcharts | OO-M1 | ✅ (tier 0 asserts the fan-out lands, since a value written to a path no subchart reads is still valid HCL) |
| Profile-set finalization | OO-M1 | ⬜ |

Scheduling and storage class are profiles rather than a `global.*` block so the subchart fan-out map is inspectable data that snapshot tests can pin, instead of an unverified projection living in a downstream consumer.

### Testing / CI & DevEx

| Item | Milestone | Status |
|---|---|---|
| Pre-commit suite (ruff, pyright, shellcheck, yamllint, cargo fmt, helm-docs) | FCO-M1 | ✅ |
| Per-component versioning + changelog + release automation (see [Versioning](../versioning/) / [Releasing](../releasing/)) | FCO-M2 | ✅ |
| `auto-format` workflow (label-driven formatter fixups) | FCO-M2 | ✅ |
| `renovate` for automated dependency bumps | FCO-M2 | ✅ |
| Chart-shape fail-fast: Thanos + Alloy validators wired into `mzmon.validate.collect`, snapshot tests pinning rendered service-account names and workload-identity subject strings | FCO-M3 | ✅ |
| **Tier 0** — plan each Terraform example, extract the composed values, and render the chart against them (`make terraform-render`). Asserts values *land*, which `validate` cannot: a wrong value path is still valid HCL | FCO-M3 | ✅ |
| **kind E2E**, path-filtered behind `e2e-gate`: tier-1 chart variant on `loki-test` + `kind-tier1`; tier-2 generic-cloud substrate (rustfs + CNPG) | FCO-M3 | ✅ (both tiers install and assert in CI. Tier 2 composes `terraform/test/generic-cloud` with the module via `terraform/test/tier2`, on `sizing = "small"` against rustfs and CNPG; ~5 min green. `make e2e-tier1-down` switches a cluster between tiers, which is needed because both name their CRDs release `mzmon-crds`) |
| **Rust E2E suite** ([DEP-185](https://linear.app/materializeinc/issue/DEP-185), `packages/mz-monitoring-e2e`): Grafana API dashboard + datasource-query assertions, Loki / Thanos direct health, Alloy support-bundle inspection, WAL durability across a gateway outage | OO-M1 | 🔨 (15 assertions, green on tier 1, tier 2, and a real EKS cluster — Loki round trip, Grafana dashboards/datasources/proxied queries, Thanos store fanout and scrape assertions, Alloy support bundles. The Thanos half runs in CI at tier 2. WAL durability across a gateway outage outstanding, as are NetworkPolicy and mTLS, which the design doc assigns to tier 2) |
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
| [Gateway-to-gateway architecture and proposal](https://linear.app/materializeinc/issue/DEP-219) — design doc plus review | OO-M3 | 🔨 ([design doc](../design-docs/20260813-byoc-observability/) drafted; review outstanding) |
| [Dual-destination pipeline pattern](https://linear.app/materializeinc/issue/DEP-124) (customer-local + control plane) | OO-M3 | ⬜ |
| [Sanitize telemetry before control-plane egress](https://linear.app/materializeinc/issue/DEP-220) | OO-M3 | ⬜ |

**A reduced copy of telemetry crosses into the control plane; the customer's full-fidelity copy always stays with them.**
Metrics cross selected by importance tier — the tiers already exist and already work per-destination.
Logs cross as an allowlisted, redacted, level-filtered subset, and log egress is opt-out for customers who will not permit it at any level of redaction, at a stated cost in support quality.

This revises an earlier position that logs never leave the customer network.
Metrics alone do not make an escalation tractable: [Upgrades](#dashboards) above is the worked example of a question ("is it stuck, and what do I do") that metrics describe and logs answer.
What made the original position worth stating is preserved rather than dropped — the claim was never that logs are uniquely sensitive, but that nothing should cross unexamined.

The gateway pair is what enforces the boundary, rather than ad-hoc network configuration.
Sanitization is what makes *anything* crossing safe, since the `_info` metrics that made dashboards legible are precisely the ones carrying customer names, and the same tension applies to log labels.
Redaction attaches to the destination rather than to the pipeline, so the reduced copy is a fork of the customer's stream and never a downgrade of it.

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

- ⬜ **`balancerd` and `console` metrics** — neither exposes anything that reaches Thanos, so the two components a user
  actually connects *through* are observable only as logs. Blocks the dashboards listed under
  [Materialize components beyond the environment](#materialize-components-beyond-the-environment).

The **operator-side** instrumentation the `env-upgrade` dashboard reads is a separate upstream dependency.
Tracked as [CLO-188](https://linear.app/materializeinc/issue/CLO-188):

- 🔨 `orchestratord_reconciliations_total`, `orchestratord_reconciliation_steps_total`, and the two duration histograms.
  Plus Kubernetes events for reconciliation failures and `Materialize` lifecycle transitions.
  Built and verified against a real cluster, but **not yet merged upstream** — so `env-upgrade` ships with most of its panels empty until it lands.
  `orchestratord_is_leader` and `environmentd_needs_update` predate this and are available today.

The `_info` family is now available, so name enrichment is unblocked for every panel.
The remaining drilldowns are still ⛓️ gated on the items above: **Sources / Sinks** await native status metrics, and **Hydration / Freshness** await the hydration and frontier signals.

## Versioning, changelog, and releases

**Built.** Each artifact has its own SemVer stream — the Helm chart, the optional CRDs chart, dashboards, pipelines, scrapers, and the shared lib — declared in `packages/components.yaml`.
Full mechanics are in [Versioning](../versioning/) and [Releasing](../releasing/); this replaces the earlier single-umbrella-chart framing.

- **Per-component streams.** ✅
  Merged PRs are attributed to components by path; `CHANGELOG.md` is the source of truth, with cumulative `Included <dep> @ vPREV..vNEW` dependency rollups.
  Each PR's entry also carries the author's own [release notes](../releasing/#release-notes-from-pr-descriptions), read from the `### Release Notes` section of its description, so the consumer-facing detail travels with the entry.
- **Automation.** ✅
  `mz-monitoring-build propose-bumps` opens one `version-update/<component>` PR per changed component on each merge to main; `publish-release` tags `<component>/vX.Y.Z` and creates a GitHub Release (attaching each component's `artifacts`) when such a PR merges.
- **Downstream pinning.** ✅
  The Terraform modules pin a specific chart version, so Terraform never tracks a moving target.
  The common module ships from this repo **inside the `materialize-monitoring` component** rather than on a stream of its own: the chart version is the release version, and the module's Git tag is that same version.
  Per-cloud wrappers downstream pin the module by Git ref, so a single number identifies both surfaces and there is no window where the pair is mismatched.
  The trade is that a Terraform-only change publishes a chart release, and a breaking module change bumps the chart's major — both handled in the changelog rather than by splitting the stream.
- **Deprecation policy.** ✅ ([DEP-127](https://linear.app/materializeinc/issue/DEP-127), OO-M1)
  Written up in [Stability Guarantees and Deprecation Policy](../design-docs/20260823-deprecation-policy/), landed as [Stability guarantees](../versioning/#stability-guarantees) (policy of record), [Stability and Deprecations](../../stability/) (customer-facing), and [the committed-surface check](../releasing/#the-committed-surface-check).
  Surfaces are graded by how much control we have and by how a break presents — alerts fail silently and get the most care, metrics fail visibly and get the least — with a 30-day cooldown enforced by the generated artifacts we already commit rather than by anything new.
  Query IDs and chart value paths fall outside the customer-facing surface, since their consumers are our own dashboards and our own Terraform module.
  Still open: the **alert and recording-rule naming decision**, which is free only until the alerting path ships, and the upstream window for the load-bearing metric list.
- **Stamping 1.0.** ⬜ ([DEP-205](https://linear.app/materializeinc/issue/DEP-205), OO-M1)
  The [pre-1.0 bump policy](../releasing/#choosing-the-next-version) lets breaking changes ride minors.
  At 1.0 that stops, and the label/metric contract, profile semantics, alert names, and chart value paths all acquire a deprecation cycle we owe.
  The window closes on its own: once enough customers have dashboards built on these labels the contract is frozen in practice whether or not it is frozen on paper, so the discipline should land before broad adoption rather than after.

## Follow-up documentation

- [Releasing](../releasing/) and [Versioning](../versioning/) are written, covering the release mechanics and the per-component model. ✅
- `CHANGELOG.md` exists and is maintained by the release tooling. ✅
- A **customer-facing** contract/deprecation-policy page (in customer terms, distinct from the internal `versioning.md`) is still to write. ⬜
- [Repo Layout](../repo-layout/) refreshed against the tree (August 2026), including `terraform/` and `test/`. ✅
  It goes stale easily by design — re-check it whenever a top-level directory moves.
- [Uninstalling](../../../operating/uninstalling/) is written: the grafana-operator finalizer deadlock, the ordered teardown, and recovery. ✅
- [Choosing the next version](../releasing/#choosing-the-next-version) records the pre-1.0 bump policy and that the changelog placeholder heading is the decision. ✅
- Alloy's rollout requirement is called out in [Production Best Practices](../../../operating/production-best-practices/#collection-alloy) as an inversion of the normal chart guarantee — the one place the chart cannot own its own rollout. ✅
- A BYOC gateway-to-gateway design doc is owed under `design-docs/`, tracked as [DEP-219](https://linear.app/materializeinc/issue/DEP-219). 🔨
  [Observability for Bring-Your-Own-Cloud](../design-docs/20260813-byoc-observability/) is written and in review as a draft; it also covers [DEP-124](https://linear.app/materializeinc/issue/DEP-124) and [DEP-220](https://linear.app/materializeinc/issue/DEP-220).
  The [BYOC](#byoc) section above is updated to match it: a reduced log subset crosses, where the earlier position was that logs never leave the customer network.
