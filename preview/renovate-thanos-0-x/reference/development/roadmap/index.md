# Roadmap




<!-- This roadmap is public. Do not include customer-specific or sensitive information -->

<!--
Agent note: this page lists work that is still to do.
When an item ships, delete its row rather than marking it done; the changelog is the record of what shipped.
"What 1.0 ships" is the one rolled-up record of finished work, and it changes only at a major version.
Linear holds each item's status; a row carries only what someone picking the item up needs.
A new design doc adds a row to "Designs in flight" and rows wherever its work lands.
-->

# Roadmap

The goal of `materialize-monitoring` is **first-class, composable observability for self-managed Materialize**: logs, metrics, events and alerts.
It is best-practices-by-default for customers who want a one-stop shop, and it does not force this stack on customers who already run their own.
The same stack is consumed through Helm, through the Terraform module in this repository, and through the per-cloud Terraform wrappers downstream.
Every component can be turned off in favour of one a customer already runs.

This page lists the work still to do, by project and milestone.
What has shipped is summarized once, in [What 1.0 ships](#what-10-ships), and itemized in the [changelog](../../changelog/).
Linear holds the status of each item; this page holds the reasoning that does not fit in a ticket.

## Projects

| Tag | Project | Scope |
|---|---|---|
| **ADOPT** | [Adopt materialize-monitoring in Cloud][adopt] | Materialize Cloud's own monitoring onto this chart, through Pulumi |
| **CLOM** | [Metrics for Cloud][clom] | Materialize-side metrics: the tenant-scoped read path, the label vocabulary, and what Materialize emits |
| **BYOC** | [BYOC Observability][byoc] | The observability control plane, for BYOC and for self-managed call-home |
| **Backlog** | [Observability Backlog][backlog] | Everything else, with no committed date. Each milestone is a candidate project of its own |

Two projects are closed.
[First-class Observability Infrastructure][fco] (**FCO**) built the platform and closed on August 31.
[Operational Observability][oo] (**OO**) hardened it, answered the operational questions, stamped 1.0, and closed on October 6.

Releases track a monthly cadence aligned to the 15th, and milestone dates are soft targets.
A milestone is cited as its project tag and its Linear name, such as **ADOPT · Loki cutover**.

| Status | Meaning |
|---|---|
| 🔨 | In progress |
| ⬜ | Planned |
| ⛓️ | Blocked on upstream instrumentation; see [Metrics contract](#metrics-contract-upstream-dependency) |

## What 1.0 ships

`materialize-monitoring` v1.0.0 stamps the chart and the Terraform module together, since the module's Git tag is the chart version.
From 1.0, a breaking change to the committed surface owes a deprecation cycle under the [stability guarantees](../../stability/).

| Area | At 1.0 |
|---|---|
| Delivery | The umbrella chart, the optional CRDs chart and the dashboards chart, published to GHCR as OCI artifacts. The common Terraform module lives here and is wrapped per cloud in `materialize-terraform-self-managed`, where observability is on by default. Profiles cover sizing, scheduling, storage class, registries, Grafana ingress and persistence, and mTLS |
| Collection | An Alloy agent for pod logs and the node journal, and an Alloy gateway for metrics. The gateway scrapes Materialize, kube-state-metrics, node-exporter, every kubelet's cAdvisor, the AWS VPC CNI, Cilium and kube-proxy. It also pulls provider metrics from CloudWatch, Cloud Monitoring and Azure Monitor. Rust panics are merged into one entry at ingest |
| Destinations | The bundled Loki and Thanos, any number of Prometheus remote-write destinations, OTLP, Datadog and Google Cloud's Telemetry API, each filtered by importance tier |
| Dashboards | `env-top`, `env-logs`, `env-upgrade`, `env-persist` and `env-consensus` for Materialize. `infra-logs`, `infra-nodes`, `infra-net`, `infra-autoscaling`, `infra-karpenter`, `infra-cloud`, `infra-loki` and `infra-alloy` for the platform. All are rendered from Rust against the query registry |
| Alerting | Thanos and Loki rulers notifying a highly available Alertmanager, with severity-to-receiver presets, capability-tagged rules, notification templates that link to Grafana, and a Terraform surface. Thirty-five alerts are in the default set, four of them log-derived, beside the `ext:consensus_*` recording rules. Alert and recording-rule names are a committed surface |
| Security | A NetworkPolicy on every component, opt-in mTLS through cert-manager in three phases, and Trivy scanning of the rendered chart and the published images |
| Qualification | Tier-0 Terraform render checks, kind tiers 1 and 2, and a Rust E2E suite run on kind and on real cloud installs |
| Release | Per-component SemVer streams, automated release PRs, a changelog carrying each PR's release notes, a deprecation policy, and a committed-surface check |

Four designs are fully shipped and now record why the stack is the way it is:
[Loki production infrastructure](../design-docs/20260627-loki-production-infrastructure/),
[Terraform modules](../design-docs/20260803-terraform-modules/),
[TLS authentication](../design-docs/20260816-tls-authentication/),
and the [stability and deprecation policy](../design-docs/20260823-deprecation-policy/).
The designs with work left are under [Designs in flight](#designs-in-flight).

## Cloud adoption

Tracked in [Adopt materialize-monitoring in Cloud][adopt] (**ADOPT**).
Materialize Cloud moves its own monitoring onto this chart, on the same `values.yaml` surface self-managed customers consume.
Cloud deploys through Pulumi, so a Pulumi component wraps the chart the way the Terraform module does ([CLO-182]).
Cloud-only rules and configuration layer through values, with no privileged path ([DEP-125]).
Cloud running the chart customers run is the strongest available test of the customer contract.

The migration is ordered by risk.
Stateless components go first, because each can be replaced in place and rolled back.
Each stateful component then gets a cutover of its own.

| Milestone | What moves | Items |
|---|---|---|
| **Stateless services** | Alloy (agent and gateway), kube-state-metrics and node-exporter replace Cloud's own. Cloud parity needs generic scrape discovery and the OTLP agent→gateway transport with a node-local WAL | 🔨 [CLO-182] · ⬜ [DEP-193] · ⬜ [DEP-189] · ⬜ [DEP-223] |
| **Thanos beside AMP** | Thanos stands up beside AMP, and the gateway fans out to both as components move. AMP keeps the `v2_mz_*` families, so what reads it today is unaffected; Thanos takes canonical `mz_*` only. AMP scales down as its consumers move | ⬜ [DEP-383] |
| **Loki cutover** | Cloud's Loki moves onto the chart's Loki on its existing storage backend, so no log data is copied. Log tenancy moves to `tenantMap: byEnvironment`, which the read path needs | ⬜ [CLO-296] |
| **Alerting** | Cloud's rules run on the chart's rulers and route through its Alertmanager to Cloud's receivers. The Loki rules Cloud clicked into Grafana are already ported by shape in `materialize-log-alerts.yaml` | ⬜ [DEP-125] |

**Grafana is not migrated.**
Cloud's Grafana lives elsewhere and stays there; it gains datasources for Thanos and Loki.

The Thanos milestone has two chart gaps to close first, both in [DEP-383].
The destination map filters by importance tier alone, so nothing can send `v2_mz_*` to AMP and keep it out of Thanos.
`materialize.deploymentMode: cloud` also makes every rule read `v2_mz_`, which a Thanos holding only `mz_*` cannot answer.
Separately, a destination below the `all` tier drops every recorded series and `ALERTS` until [DEP-368] gives them a tier.

## Metrics for Cloud

Tracked in [Metrics for Cloud][clom] (**CLOM**): what Materialize emits, what it is called, and how it is read.

### Tenant-scoped read path

Designed in [A Tenant-Scoped Query API](../design-docs/20260916-tenant-query-api/) and tracked as [DEP-269].
Every consumer of this stack so far is a Grafana the chart deploys, reading backends over a `ClusterIP` Service.
The read path serves the Materialize console and a customer's own Grafana through a tenant-scoped proxy instead.
The proxy is a Rust service in this workspace, tested against `prom-label-proxy` as an oracle.

| Item | Status |
|---|---|
| [DEP-272] Per-family tenancy classification, generated from the query registry | ⬜ |
| [DEP-273] `query-proxy` chart component: JWT verification, issuer values, label enforcement, fail-closed defaults | ⬜ |
| [DEP-274] Separate grants for logs and metrics, and for the classes within each | ⬜ |
| [DEP-275] Exposure controls: the query frontend, per-tenant limits, NetworkPolicy narrowing, a public-exposure guard | ⬜ |
| [DEP-276] The proxy's read-audit stream and a `jti` denylist | ⬜ |
| [DEP-277] A producer for the `audit` log class, which values declare and nothing reads | ⬜ |
| [DEP-278] A published query manifest that Console consumes by query ID | ⬜ |
| [DEP-279] A `query-proxy` profile, Terraform variables, and a minimum sizing envelope | ⬜ |
| A customer-facing page: obtaining a token, the two Grafana datasource shapes, and what a tenant can read | ⬜ |

**The read path is where this stack stops being optional.**
Console reads environment metrics from SQL today, which keeps a short window of history and is unavailable exactly when the environment is.
Reading PromQL and LogQL instead makes a deployment without those endpoints a deployment with a broken Console.
The design resolves that by mandating an interface rather than an implementation.
The interface is a PromQL and a LogQL endpoint carrying the documented label contract, behind the proxy.
Thanos and Loki are the bundled implementation, and a customer already running an equivalent points the proxy at theirs.
This supersedes the earlier plan to expose a Prometheus endpoint for customers to scrape.

### Metric label standardization

One canonical label vocabulary for Materialize's metrics, a reserved-label list, and an authoritative environment key.
The key is `environment_id` from `Materialize.spec.environmentId`, with `environment_name` on `mz_environment_info`.
Designed in *A Label Vocabulary for Materialize Metrics*, in review as [#445].
The work lands in two passes, ordered so that consumers never observe the second.
This repository's monitors first fill the canonical labels where they are empty, and Materialize then adopts the names upstream behind a label lint.

| Where | Items, all ⬜ |
|---|---|
| Design | [DEP-346] review and acceptance |
| This repository | [DEP-349] `labels.yaml` and generated relabel rules · [DEP-351] `environment_id` on every Materialize monitor · [DEP-350] the GMP flavor drops target relabelings · [DEP-354] dashboards, rules and pickers onto `environment_id` · [DEP-355] the registry onto canonical names · [DEP-356] a customer-facing label contract page · [DEP-361] `mz_environment_info` through kube-state-metrics · [DEP-360] retire the aliases after 30 days |
| Upstream | [DEP-348] the orchestratord pod label and `mz_environment_info` · [DEP-347] clusterd's `cluster="compute"` · [DEP-352] the label lint · [DEP-353] `/metrics/public` replica labels · [DEP-357] `cluster_id` · [DEP-358] entity-qualified `_info` labels · [DEP-359] the `cluster` and `honeycomb` const labels · [DEP-362] shard, persist-name and version renames · [DEP-363] an optional `spec.environmentName` · [DEP-364] wallclock lag as a histogram · [DEP-365] `environment_id` on reconciliation metrics |

### Metrics contract (upstream dependency)

Tracked as [DEP-207].
The metric and label contract is the public API for everything here.
Several dashboards depend on instrumentation that lives in `MaterializeInc/materialize`.
About 39 signal families exist only through the SQL-on-scrape exporter path, which Cloud names `v2_mz_*`.
`v2_mz_` is not a newer variant; it names the exporter path, and the target is canonical `mz_`.
environmentd has to emit those families natively before that path can retire.
`mz_object_info` and the `_info` family are delivered, so name enrichment is unblocked everywhere.

| Ask | Unblocks | Status |
|---|---|---|
| Native source and sink status. No sink status metric exists at all | The Sources and Sinks drilldowns | ⛓️ |
| Native hydration and frontier / freshness signals | The Hydration and Freshness drilldowns | ⛓️ |
| Label-family harmonization | Joins across families. Now carried by [Metric label standardization](#metric-label-standardization) | ⬜ |
| Latency histograms for `consensus_scan`, `consensus_truncate`, `blob_delete` and the timestamp oracle, which publish only a mean | Tail latency on `env-persist` and `env-consensus` | ⬜ |
| `balancerd` and `console` metrics on self-managed. None reach Thanos today | Dashboards for the two components users connect through | ⬜ |

`balancerd` does publish connection metrics, whose semantics [CLO-282] questions.
Its half of the last row may therefore be a scrape gap rather than an upstream one.

### Materialize-side emission

| Item | Status |
|---|---|
| [CLO-55] Reconciliation timing, stalls, errors and successes for the k8s controllers, and an optional reconcile timeout | ⬜ |
| [CLO-299] The controller crate logs every failed reconcile at error level, transient Kubernetes errors included | ⬜ |
| [CLO-282] What `mz_balancer_connection_status{status="success"}` means for a refused connection | ⬜ |
| [CLO-256] Reduce less-essential environmentd metrics on very large environments | ⬜ |
| [CLO-255] Compress the metrics endpoint's responses | ⬜ |
| [DEP-221] Move the scrapers into the `materialize-operator` chart, which should own them | ⬜ |

## BYOC

Tracked in [BYOC Observability][byoc] (**BYOC**).
The observability control plane is where telemetry crosses from a customer-run cluster into Materialize's.
One channel serves two segments: BYOC environments, which Materialize operates, and self-managed installs, which opt in.
Designed in [Observability for Bring-Your-Own-Cloud](../design-docs/20260813-byoc-observability/).

### Observability control plane

| Item | Status |
|---|---|
| [DEP-124] The dual-destination pipeline pattern: customer-local plus control plane | ⬜ |
| [DEP-220] Sanitize telemetry before control-plane egress | ⬜ |
| [DEP-236] A trust bundle for a private CA: the roots a corporate proxy or an on-premise object store needs, merged with the internal issuer's | ⬜ |
| [DEP-287] `oauth2.tls` on the destination schema: certificate-authenticated token exchange, shared with call-home | ⬜ |
| [DEP-224] A Day 1 dependencies dashboard: are the Materialize and observability prerequisites satisfied | ⬜ |
| **BYOC · Observability implementation** | Not yet scoped |

**A reduced copy of telemetry crosses into the control plane, and the customer's full-fidelity copy always stays with them.**
Metrics cross selected by importance tier, which already works per destination.
Logs cross as an allowlisted, redacted, level-filtered subset, and log egress is opt-out at a stated cost in support quality.
Redaction attaches to the destination rather than the pipeline, so the reduced copy is a fork of the customer's stream and never a downgrade of it.
The gateway pair enforces the boundary, rather than ad-hoc network configuration.

### Secret filtering

Designed in [Secret Filtering](../design-docs/20260924-log-secret-filtering/) and tracked as [DEP-306].
Credentials are the one exception to redaction at the destination.
They are filtered at ingest, before anything is stored, because a credential in the customer's own Loki is their exposure.
Each control-plane branch runs a second, stricter filter immediately before its writer, and a canary per layer proves both are in the path.

| Item | Status |
|---|---|
| [DEP-307] Typed `loki.secretfilter` and `stage.luhn` in the pipeline schema | ⬜ |
| [DEP-308] The ingest filter ahead of parsing, with the bulk tier ahead of the scan | ⬜ |
| [DEP-309] Close the two raw-line paths: the `sampleDebug` tap and live debugging | ⬜ |
| [DEP-310] Card-number redaction with `stage.luhn` | ⬜ |
| [DEP-311] Rule files: the Materialize rules, the name allowlist, and the egress file | ⬜ |
| [DEP-312] A corpus harness on the pinned Alloy binary, run on every Alloy bump | ⬜ |
| [DEP-313] Shadow mode, counting matches on real logs before any rule enforces | ⬜ |
| [DEP-314] The values surface and fail-open validators | ⬜ |
| [DEP-315] `mz-monitoring-canary`, a canary producer per layer | ⬜ |
| [DEP-316] Alerts, runbooks and a dashboard row | ⬜ |
| [DEP-317] A strict, fail-closed egress filter on each control-plane branch | ⬜ |
| [DEP-318] A backstop overlay for the control-plane gateway | ⬜ |
| [DEP-320] A customer-facing secret-filtering page, and the design's revisions | ⬜ |

### Call-home from self-managed

Designed in [Call-Home](../design-docs/20260917-call-home-self-managed/) and tracked as [DEP-270].
BYOC environments are ones Materialize operates; self-managed installs are invisible to Materialize between escalations.

| Item | Status |
|---|---|
| [DEP-280] The `callHome.level` consent ladder and profiles, defaulting to off, with a render assertion that the default sends nothing | ⬜ |
| [DEP-281] A heartbeat producer and three-part install identity | ⬜ |
| [DEP-282] The alerts level: alert state (`ALERTS`) over the gateway fan-out | ⬜ |
| [DEP-283] A `previewOnly` destination mode: the full chain, counted locally, never sent | ⬜ |
| [DEP-284] An egress meter, a per-level byte-rate ceiling, and a local export-failure alert | ⬜ |
| [DEP-285] A generated egress schedule per level | ⬜ |
| [DEP-286] The OTLP/HTTP default wire, forward-proxy and corporate-CA configuration | ⬜ |
| [DEP-288] The `elevated` window: time-boxed, self-expiring, never set remotely | ⬜ |
| [DEP-289] An Alertmanager webhook receiver path | ⬜ |
| [DEP-290] Terraform variables for the level, the credential reference and the proxy | ⬜ |
| [DEP-291] Control-plane per-tenant limits sized for install count, and a stated response model | ⬜ |
| A customer-facing page: the levels and the schedule for each, previewing, the local meter, and retention, access and deletion | ⬜ |

**The channel is the BYOC channel; the feature is consent.**
A self-managed customer bought software they run themselves, so every byte that leaves is a concession.
The design is a bounded, monotone, locally visible ladder (`off`, `heartbeat`, `alerts`, `metrics`, `diagnostics`) over the existing pipeline.
The cheapest useful level is alerts, which has something to forward now that a default rule set installs.
A TLS-intercepting corporate proxy defeats mTLS outright, which makes token exchange ([DEP-287]) a reachability requirement in this segment.
**Receiving a signal creates an obligation.**
A stated response model, and a stated non-response, is a prerequisite for the alerts level rather than a follow-up.

## Backlog

Tracked in [Observability Backlog][backlog] (**Backlog**): work with no committed date.
Each milestone below is a candidate project of its own, and moves out of the backlog when it is prioritized.

### External dependencies

Designed in [Monitoring Materialize's External Dependencies](../design-docs/20260920-external-dependency-monitoring/) and tracked as [DEP-233].
**The client's measurement of a dependency is the SLI, and the dependency's own telemetry is the diagnosis.**
Several pieces shipped with 1.0: the client vantage point, the provider pulls, and the client and provider dashboards.
So did the `persist-failures` split and the `ext:consensus_*` recording rules.
What remains is the flavor-specific half.
It stays opt-in behind the normalized `ext:*` contract.
That contract keeps seven database flavors and five object stores from multiplying the dashboard and alert set.

| Item | Status |
|---|---|
| [DEP-295] The rest of the `ext:*` layer: `ext:consensus_connections_used_ratio` and the `ext:objstore_*` families | 🔨 |
| [DEP-297] The exporter vantage point: a multi-target `postgres_exporter` subchart, transaction-ID age, per-database sizes, a documented grant | ⬜ |
| [DEP-298] PostgreSQL, CNPG and self-hosted CockroachDB adapters, with the CockroachDB Cloud alerts reclassified as their own adapter | ⬜ |
| [DEP-299] An on-premise object-store adapter for MinIO, Garage and Ceph | ⬜ |
| [DEP-300] Version reporting (`ext:*_version_info`) across every adapter | ⬜ |
| [DEP-303] The `externalDependencies` values block and a `dependency-monitoring` profile | ⬜ |
| [DEP-366] The per-cloud wrappers declare the metadata database in `externalDependencies.consensus` | ⬜ |
| [DEP-368] A metric tier for recorded series, so a filtered destination keeps `ext:*` and `ALERTS` | ⬜ |
| [DEP-304] An `AbortIncompleteMultipartUpload` lifecycle rule on the persist bucket, in `materialize-terraform-self-managed` | ⬜ |

No adapter flavor is the one that waits.
Every shipped wrapper provisions managed PostgreSQL, while CockroachDB and CNPG both run in customer clusters.
The on-premise shape has no Terraform wrapper, but `terraform/test/generic-cloud` already runs it.
Its adapters are therefore the ones this repository can test end to end.

### Dashboards

| Item | Status |
|---|---|
| [DEP-208] Troubleshooting: symptom-first entry into the rest, with every panel linking onward | ⬜ |
| [DEP-157] Resizing (Day 2) | ⬜ |
| [DEP-158] Changing sources (Day 2) | ⬜ |
| [DEP-159] Changing external destinations (Day 2) | ⬜ |
| [DEP-160] Managing users (Day 2) | ⬜ |
| [DEP-212] Hydration drilldown | ⛓️ |
| [DEP-213] Freshness drilldown | ⛓️ |
| [DEP-214] Sources drilldown | ⛓️ |
| [DEP-215] Sinks drilldown | ⛓️ |
| [DEP-225] Sizing (Day 1) | ⬜ |
| [DEP-378] `infra-pods`: one workload's health, logs and metrics | ⬜ |
| [DEP-379] Meta-monitoring for Grafana, Thanos and Alertmanager | ⬜ |
| [DEP-384] `infra-net`'s Cloud Networking rows, from the provider pull | ⬜ |
| [DEP-115] A Datadog dashboard set | ⬜ |
| [DEP-217] A Google Cloud Monitoring dashboard set | ⬜ |
| [DEP-218] A Honeycomb dashboard set | ⬜ |
| [DEP-325] `infra-*` variables that reference variables those dashboards never define | ⬜ |
| [DEP-333] `infra-nodes`' node picker resolves to nothing on EKS | ⬜ |

**Day 2 operations outweigh Day 1.**
Upgrades, resizing, changing sources and destinations, and managing users matter most to a running deployment, so the Day 1 dashboards come last.
The change-operation dashboards focus on objects being added, removed and initially populated, with error detection, rather than on steady state.
Troubleshooting is symptom-first where `env-top` is subsystem-first, so it sequences after the dashboards it links into.
The drilldowns wait on the [metrics contract](#metrics-contract-upstream-dependency), and Freshness is the one customers ask about most.

### Alerting

Designed in [Alerting in Self-Managed](../design-docs/20260917-alerting-self-managed/).
The evaluation and notification path shipped with 1.0, and what remains is breadth and operability.

| Item | Status |
|---|---|
| [DEP-372] A runbook for every shipped alert, under `operating/runbooks/` | ⬜ |
| [DEP-373] A deadman's switch and alerting meta-alerts | ⬜ |
| [DEP-374] Triage of the ported rules that install only when selected | ⬜ |
| [DEP-335] Workloads declare their alert tier through labels, so freshness can join the default set | ⬜ |
| [DEP-375] Inhibit rollout-sensitive alerts while a rollout is in progress | ⬜ |
| [DEP-376] `alerting.alertmanager.mode: external` | ⬜ |
| [DEP-377] `rules.extra` | ⬜ |
| [DEP-339] Alerting through Terraform: the downstream documentation | 🔨 |
| [CLO-266] An alert on 5xx responses from `balancerd` | ⬜ |
| [DEP-340] E2E: the Loki ruler loads the chart's log rules, and each fires on a real line | ⬜ |
| [DEP-345] The Thanos ruler's rule importer never picks up a changed selector | ⬜ |
| [DEP-328] Report the Thanos ruler's `write_relabel_configs` panic upstream, then drop the gateway workaround | ⬜ |

Freshness and source rules exist but stay opt-in.
Calibrated against a production fleet, an absolute threshold fires mostly on clusters that are behind by design.
Only a declared tier can tell those apart.
Sink health and correctness logs beyond the three Materialize log-derived defaults have no rule yet.

### Collection

| Item | Status |
|---|---|
| [DEP-385] A gateway over its memory limiter's soft limit refuses its scrapes for hours: sizing, limiter placement, resharding | 🔨 |
| [DEP-380] CNI monitors for Calico and Azure Network Policy Manager | ⬜ |
| [DEP-126] Trace correlation as structured metadata in the log pipeline, without trace storage | ⬜ |
| [CLO-76] Optional continuous profiling, off by default | ⬜ |
| [DEP-252] Cost visibility through an optional OpenCost component, with its design in review as [#365] | 🔨 |

### Platform

| Item | Status |
|---|---|
| [DEP-338] An image vulnerability gate that fails only the change that can act on it | ⬜ |
| [DEP-382] `security-gate` as a required check on `main` | ⬜ |
| [DEP-249] Trivy suppressions are per check, so they also mask our own templates | ⬜ |
| [DEP-251] SARIF findings point at temporary render paths | ⬜ |
| [DEP-381] `tflint` in Terraform CI | ⬜ |
| [DEP-369] The per-cloud wrappers pass the Kubernetes cluster name as `cluster_name` | ⬜ |
| [DEP-370] Renovate bumps the `mzmon-alloy` tag and leaves its digest | ⬜ |
| [DEP-371] Release PRs leave `Cargo.lock` stale | ⬜ |
| [DEP-111] and [DEP-118] An ArgoCD and FluxCD CI matrix | ⬜ |

## Designs in flight

A design moves to **Shipped** when the rows it owns on this page are gone.

| Design | Status | Remaining work |
|---|---|---|
| [Observability for Bring-Your-Own-Cloud](../design-docs/20260813-byoc-observability/) | Ready | [BYOC](#byoc) |
| [A Tenant-Scoped Query API](../design-docs/20260916-tenant-query-api/) | Ready | [Tenant-scoped read path](#tenant-scoped-read-path) |
| [Alerting in Self-Managed](../design-docs/20260917-alerting-self-managed/) | Ready | [Alerting](#alerting) |
| [Call-Home](../design-docs/20260917-call-home-self-managed/) | Ready | [Call-home from self-managed](#call-home-from-self-managed) |
| [Monitoring Materialize's External Dependencies](../design-docs/20260920-external-dependency-monitoring/) | Ready | [External dependencies](#external-dependencies) |
| [Secret Filtering](../design-docs/20260924-log-secret-filtering/) | Ready | [Secret filtering](#secret-filtering) |
| A Label Vocabulary for Materialize Metrics | Draft, in review as [#445] | [Metric label standardization](#metric-label-standardization) |
| Cost visibility with OpenCost | Draft, in review as [#365] | [Collection](#collection) |

[adopt]: https://linear.app/materializeinc/project/adopt-materialize-monitoring-in-cloud-cce4b8af4db8/overview
[clom]: https://linear.app/materializeinc/project/metrics-for-cloud-070c13ddba6a/overview
[byoc]: https://linear.app/materializeinc/project/byoc-observability-adc226c14418/overview
[backlog]: https://linear.app/materializeinc/project/observability-backlog-53362eb00a84/overview
[fco]: https://linear.app/materializeinc/project/first-class-observability-infrastructure-in-self-managed-5e48691c74a8/overview
[oo]: https://linear.app/materializeinc/project/operational-observability-abf9af76c03a/overview

[#365]: https://github.com/MaterializeInc/materialize-monitoring/pull/365
[#445]: https://github.com/MaterializeInc/materialize-monitoring/pull/445

[CLO-55]: https://linear.app/materializeinc/issue/CLO-55
[CLO-76]: https://linear.app/materializeinc/issue/CLO-76
[CLO-182]: https://linear.app/materializeinc/issue/CLO-182
[CLO-255]: https://linear.app/materializeinc/issue/CLO-255
[CLO-256]: https://linear.app/materializeinc/issue/CLO-256
[CLO-266]: https://linear.app/materializeinc/issue/CLO-266
[CLO-282]: https://linear.app/materializeinc/issue/CLO-282
[CLO-296]: https://linear.app/materializeinc/issue/CLO-296
[CLO-299]: https://linear.app/materializeinc/issue/CLO-299
[DEP-111]: https://linear.app/materializeinc/issue/DEP-111
[DEP-115]: https://linear.app/materializeinc/issue/DEP-115
[DEP-118]: https://linear.app/materializeinc/issue/DEP-118
[DEP-124]: https://linear.app/materializeinc/issue/DEP-124
[DEP-125]: https://linear.app/materializeinc/issue/DEP-125
[DEP-126]: https://linear.app/materializeinc/issue/DEP-126
[DEP-157]: https://linear.app/materializeinc/issue/DEP-157
[DEP-158]: https://linear.app/materializeinc/issue/DEP-158
[DEP-159]: https://linear.app/materializeinc/issue/DEP-159
[DEP-160]: https://linear.app/materializeinc/issue/DEP-160
[DEP-189]: https://linear.app/materializeinc/issue/DEP-189
[DEP-193]: https://linear.app/materializeinc/issue/DEP-193
[DEP-207]: https://linear.app/materializeinc/issue/DEP-207
[DEP-208]: https://linear.app/materializeinc/issue/DEP-208
[DEP-212]: https://linear.app/materializeinc/issue/DEP-212
[DEP-213]: https://linear.app/materializeinc/issue/DEP-213
[DEP-214]: https://linear.app/materializeinc/issue/DEP-214
[DEP-215]: https://linear.app/materializeinc/issue/DEP-215
[DEP-217]: https://linear.app/materializeinc/issue/DEP-217
[DEP-218]: https://linear.app/materializeinc/issue/DEP-218
[DEP-220]: https://linear.app/materializeinc/issue/DEP-220
[DEP-221]: https://linear.app/materializeinc/issue/DEP-221
[DEP-223]: https://linear.app/materializeinc/issue/DEP-223
[DEP-224]: https://linear.app/materializeinc/issue/DEP-224
[DEP-225]: https://linear.app/materializeinc/issue/DEP-225
[DEP-233]: https://linear.app/materializeinc/issue/DEP-233
[DEP-236]: https://linear.app/materializeinc/issue/DEP-236
[DEP-249]: https://linear.app/materializeinc/issue/DEP-249
[DEP-251]: https://linear.app/materializeinc/issue/DEP-251
[DEP-252]: https://linear.app/materializeinc/issue/DEP-252
[DEP-269]: https://linear.app/materializeinc/issue/DEP-269
[DEP-270]: https://linear.app/materializeinc/issue/DEP-270
[DEP-272]: https://linear.app/materializeinc/issue/DEP-272
[DEP-273]: https://linear.app/materializeinc/issue/DEP-273
[DEP-274]: https://linear.app/materializeinc/issue/DEP-274
[DEP-275]: https://linear.app/materializeinc/issue/DEP-275
[DEP-276]: https://linear.app/materializeinc/issue/DEP-276
[DEP-277]: https://linear.app/materializeinc/issue/DEP-277
[DEP-278]: https://linear.app/materializeinc/issue/DEP-278
[DEP-279]: https://linear.app/materializeinc/issue/DEP-279
[DEP-280]: https://linear.app/materializeinc/issue/DEP-280
[DEP-281]: https://linear.app/materializeinc/issue/DEP-281
[DEP-282]: https://linear.app/materializeinc/issue/DEP-282
[DEP-283]: https://linear.app/materializeinc/issue/DEP-283
[DEP-284]: https://linear.app/materializeinc/issue/DEP-284
[DEP-285]: https://linear.app/materializeinc/issue/DEP-285
[DEP-286]: https://linear.app/materializeinc/issue/DEP-286
[DEP-287]: https://linear.app/materializeinc/issue/DEP-287
[DEP-288]: https://linear.app/materializeinc/issue/DEP-288
[DEP-289]: https://linear.app/materializeinc/issue/DEP-289
[DEP-290]: https://linear.app/materializeinc/issue/DEP-290
[DEP-291]: https://linear.app/materializeinc/issue/DEP-291
[DEP-295]: https://linear.app/materializeinc/issue/DEP-295
[DEP-297]: https://linear.app/materializeinc/issue/DEP-297
[DEP-298]: https://linear.app/materializeinc/issue/DEP-298
[DEP-299]: https://linear.app/materializeinc/issue/DEP-299
[DEP-300]: https://linear.app/materializeinc/issue/DEP-300
[DEP-303]: https://linear.app/materializeinc/issue/DEP-303
[DEP-304]: https://linear.app/materializeinc/issue/DEP-304
[DEP-306]: https://linear.app/materializeinc/issue/DEP-306
[DEP-307]: https://linear.app/materializeinc/issue/DEP-307
[DEP-308]: https://linear.app/materializeinc/issue/DEP-308
[DEP-309]: https://linear.app/materializeinc/issue/DEP-309
[DEP-310]: https://linear.app/materializeinc/issue/DEP-310
[DEP-311]: https://linear.app/materializeinc/issue/DEP-311
[DEP-312]: https://linear.app/materializeinc/issue/DEP-312
[DEP-313]: https://linear.app/materializeinc/issue/DEP-313
[DEP-314]: https://linear.app/materializeinc/issue/DEP-314
[DEP-315]: https://linear.app/materializeinc/issue/DEP-315
[DEP-316]: https://linear.app/materializeinc/issue/DEP-316
[DEP-317]: https://linear.app/materializeinc/issue/DEP-317
[DEP-318]: https://linear.app/materializeinc/issue/DEP-318
[DEP-320]: https://linear.app/materializeinc/issue/DEP-320
[DEP-325]: https://linear.app/materializeinc/issue/DEP-325
[DEP-328]: https://linear.app/materializeinc/issue/DEP-328
[DEP-333]: https://linear.app/materializeinc/issue/DEP-333
[DEP-335]: https://linear.app/materializeinc/issue/DEP-335
[DEP-338]: https://linear.app/materializeinc/issue/DEP-338
[DEP-339]: https://linear.app/materializeinc/issue/DEP-339
[DEP-340]: https://linear.app/materializeinc/issue/DEP-340
[DEP-345]: https://linear.app/materializeinc/issue/DEP-345
[DEP-346]: https://linear.app/materializeinc/issue/DEP-346
[DEP-347]: https://linear.app/materializeinc/issue/DEP-347
[DEP-348]: https://linear.app/materializeinc/issue/DEP-348
[DEP-349]: https://linear.app/materializeinc/issue/DEP-349
[DEP-350]: https://linear.app/materializeinc/issue/DEP-350
[DEP-351]: https://linear.app/materializeinc/issue/DEP-351
[DEP-352]: https://linear.app/materializeinc/issue/DEP-352
[DEP-353]: https://linear.app/materializeinc/issue/DEP-353
[DEP-354]: https://linear.app/materializeinc/issue/DEP-354
[DEP-355]: https://linear.app/materializeinc/issue/DEP-355
[DEP-356]: https://linear.app/materializeinc/issue/DEP-356
[DEP-357]: https://linear.app/materializeinc/issue/DEP-357
[DEP-358]: https://linear.app/materializeinc/issue/DEP-358
[DEP-359]: https://linear.app/materializeinc/issue/DEP-359
[DEP-360]: https://linear.app/materializeinc/issue/DEP-360
[DEP-361]: https://linear.app/materializeinc/issue/DEP-361
[DEP-362]: https://linear.app/materializeinc/issue/DEP-362
[DEP-363]: https://linear.app/materializeinc/issue/DEP-363
[DEP-364]: https://linear.app/materializeinc/issue/DEP-364
[DEP-365]: https://linear.app/materializeinc/issue/DEP-365
[DEP-366]: https://linear.app/materializeinc/issue/DEP-366
[DEP-368]: https://linear.app/materializeinc/issue/DEP-368
[DEP-369]: https://linear.app/materializeinc/issue/DEP-369
[DEP-370]: https://linear.app/materializeinc/issue/DEP-370
[DEP-371]: https://linear.app/materializeinc/issue/DEP-371
[DEP-372]: https://linear.app/materializeinc/issue/DEP-372
[DEP-373]: https://linear.app/materializeinc/issue/DEP-373
[DEP-374]: https://linear.app/materializeinc/issue/DEP-374
[DEP-375]: https://linear.app/materializeinc/issue/DEP-375
[DEP-376]: https://linear.app/materializeinc/issue/DEP-376
[DEP-377]: https://linear.app/materializeinc/issue/DEP-377
[DEP-378]: https://linear.app/materializeinc/issue/DEP-378
[DEP-379]: https://linear.app/materializeinc/issue/DEP-379
[DEP-380]: https://linear.app/materializeinc/issue/DEP-380
[DEP-381]: https://linear.app/materializeinc/issue/DEP-381
[DEP-382]: https://linear.app/materializeinc/issue/DEP-382
[DEP-383]: https://linear.app/materializeinc/issue/DEP-383
[DEP-384]: https://linear.app/materializeinc/issue/DEP-384
[DEP-385]: https://linear.app/materializeinc/issue/DEP-385

