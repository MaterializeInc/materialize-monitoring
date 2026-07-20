---
title: "o11y Glossary"
weight: 20
---

<!--
Agent note: Be sure to read CLAUDE.md for markdown style.
Use h5 for term headings and provide stable anchors.
Interlink terms as used.
-->

# Glossary of Observability Terms

A reference for the vocabulary you'll see across these docs.
The audience is a Materialize customer or operator: comfortable with SQL, conversant in DBA terminology, working knowledge of Kubernetes.
We don't define general database or Kubernetes terms (table, namespace, pod) — only what's specific to monitoring, the Materialize internals you'll meet in panels, or this stack's deployment vocabulary.

## Observability foundations

##### Cardinality {#cardinality}
The number of unique time series (or log streams) a metric produces.
The dominant cost-and-stability lever for any Prometheus-shaped stack: a single ill-chosen label that takes on millions of values can balloon storage and crash query backends.
The materialize-monitoring pipeline applies a cardinality-reduction policy at the gateway layer before metrics egress to expensive backends — see [Operating > Tuning](../operating/tuning/).

##### Error {#error}
A signal that something failed — an exception, a 5xx response, a panic.
Errors surface in three forms across this stack: log lines at `ERROR` or `WARN` level (see [Log level](#log-level)), increments on error counters (`*_errors_total`), and elevated rates that trip alerting rules.
Per the [RED method](https://grafana.com/blog/2018/08/02/the-red-method-how-to-instrument-your-services/), error rate is one of the three primary indicators (alongside Rate and Duration) worth tracking for any request-serving system.

##### I/O {#io}
Input/output — work performed against an external resource (network, disk, another process).
I/O metrics in this stack come from [cAdvisor](#cadvisor) (`container_network_*_bytes_total`, `container_fs_*`), [node-exporter](#node-exporter) (block-device counters), and Materialize's own metrics (source bytes received, sink throughput).
Disk and network I/O are common bottlenecks for streaming systems.

##### Label {#label}
A key-value pair attached to a metric or log line describing one dimension of what was measured.
For example, `mz_query_total{cluster_id="u1", query_kind="select"}` has two labels.
Labels are how you filter and group in [PromQL](#promql)/[LogQL](#logql) — and the multiplicative source of [cardinality](#cardinality).

##### Label budget {#label-budget}
A target ceiling on the number of distinct values a given label may take — or, more broadly, on the total [cardinality](#cardinality) a metric is allowed to produce.
Treated as a contract between metric producers and the platform: producers commit to staying under budget, the platform commits to scraping and storing what fits.
When a budget is exceeded, this stack's pipeline can drop the offending labels into structured metadata or refuse them entirely.
See [Operating > Tuning](../operating/tuning/) for the policy applied here.

##### Latency (generically) {#latency}
The time elapsed between a request being initiated and a response being received.
Almost always reported as a [histogram](#histogram-metric) and queried at p50/p90/p99 — averages hide the tail behavior that matters operationally.
Materialize-specific note: read-query latency is exposed as [peek](#peek) latency; there is no metric named "query latency."

##### Observability / o11y {#observability}
The practice of inferring a running system's internal state from the signals it externalizes — metrics, logs, and events.
"o11y" is the conventional shorthand: the letter `o`, eleven characters, the letter `y`.

##### Quantile / Percentile {#quantile}
A statistical summary describing the value below which a given fraction of observations fall.
Latency panels typically show p50 (median), p90, and p99; "p99 = 250ms" means 99% of requests completed in 250ms or less.
Computed in [PromQL](#promql) via `histogram_quantile()` against a [histogram metric](#histogram-metric).
Don't conflate with averages — averages hide tail behavior, which is usually the thing operators care about.

##### Series / time series {#series}
A unique combination of metric name + label values, sampled over time.
`mz_query_total{cluster_id="u1"}` and `mz_query_total{cluster_id="u2"}` are two distinct series of the same metric.

##### Swap (memory) {#swap}
A region of disk the kernel uses as overflow when physical RAM is exhausted.
Swapping is generally undesirable for performance-critical workloads — disk is orders of magnitude slower than RAM, so an apparently-running process that's actually swapping can appear catastrophically slow.
Most production Kubernetes deployments disable swap entirely; if a node has it enabled, watch `node_memory_SwapUsed_bytes` for early warning of memory pressure.

##### Throughput {#throughput}
The rate at which a system processes work, usually expressed as units per second — bytes/sec, queries/sec, rows/sec.
Reported as a [counter](#counter-metric) in metrics and queried via `rate(...)`.
Sources have ingest throughput; sinks have output throughput; queries have peek-rate throughput.

## Metrics

##### Counter Metric {#counter-metric}
A metric that only increases (or resets to zero on process restart).
Examples: total bytes received, total queries served.
Always wrap a counter in `rate(...)` or `increase(...)` in [PromQL](#promql) — the raw value of a counter is rarely directly useful.

##### Gauge Metric {#gauge-metric}
A metric that can go up or down at any time.
Examples: current memory usage, currently active connections.
Read the value directly; no rate wrapper needed.

##### Histogram Metric {#histogram-metric}
A metric that records the distribution of observed values across configured bucket boundaries.
Latency, request-size, and other "spread matters" metrics are almost always histograms.
Query a histogram via `histogram_quantile(quantile, sum by (le) (rate(<metric>_bucket[...])))`.

##### Metric {#metric}
A numeric measurement, identified by a name and a set of labels, that is scraped or pushed at fixed intervals into [Prometheus](#prometheus) (or another time-series backend).
The shape of a metric is captured by its type — counter, gauge, or histogram — and its label set determines its [cardinality](#cardinality).
Distinct from a [log line](#log) (text, unstructured-by-default, indexed by stream) and a *trace span* (causal, request-scoped).

##### Recording rule {#recording-rule}
A precomputed [PromQL](#promql) expression evaluated at scrape interval and stored as a new series.
Two main uses: speed up expensive queries that appear in many dashboards/alerts, and reduce [cardinality](#cardinality) before metrics egress to expensive backends.
See [Metrics > Rules](../metrics/rules/).

##### Sample Metric {#sample-metric}
One (timestamp, value) data point in a [series](#series).
A series is, mechanically, an ordered list of samples.

## Logs and events

##### Event {#event}
A discrete occurrence with associated metadata — a deployment, a pod restart, an alert firing.
Kubernetes emits events on its API; some of these (notably Pod state changes) flow through this stack alongside container logs.

##### Log {#log}
A line (or structured record) emitted by a running process.
This stack collects container logs from Materialize's processes (`environmentd`, `clusterd`) as well as the operator and any sidecars.
Structured logging (key=value or JSON) queries far better in [LogQL](#logql) than free-form text.

##### Log level {#log-level}
The severity class attached to a log line: typically `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` (some systems add `FATAL` above `ERROR`).
Materialize emits at `INFO` by default; production filtering is usually `WARN`/`ERROR` for storage cost, dropping to `INFO` or `DEBUG` for active debugging.
Configurable per-process via the standard Rust `RUST_LOG`/log-filter conventions.

##### Log Stream {#log-stream}
The [Loki](#loki) analogue of a Prometheus [series](#series): a unique combination of labels under which log entries are ordered by time.
As with metrics, label [cardinality](#cardinality) is the cost driver — keep volatile attributes (request_id, trace_id) in the *log body*, not as Loki labels.

## Dashboards and queries

##### Dashboard {#dashboard}
A collection of panels backed by queries, typically laid out in tabs and rows.
This stack ships dashboards for both [Grafana](#grafana) and [Datadog](#datadog); see [Dashboards](../dashboards/).

##### Dashboard Panel {#dashboard-panel}
A single visualization on a dashboard — a time series, table, donut, single-stat, etc.
Most panels are backed by one or more queries.

##### LogQL {#logql}
Loki's query language.
Borrows [PromQL](#promql)'s selector syntax (`{job="mz", container="environmentd"}`) and adds line-filter operators (`|=`, `|~`) and parsing pipelines (`| json | line_format ...`).

##### PromQL {#promql}
Prometheus's query language.
Used by [Grafana](#grafana), [Alertmanager](#alertmanager), [recording rules](#recording-rule), and [alerting rules](#alerting-rule) across this stack.
PromQL operates on time series: `rate(metric[5m])` computes a per-second rate; `sum by (label) (...)` aggregates.

## Alerting

##### Alert {#alert}
A condition expressed as a [PromQL](#promql) (or [LogQL](#logql)) query that, when persistently true for a configured duration (`for: 5m`), signals that something needs attention.
An alert *fires*, gets *routed* by [Alertmanager](#alertmanager) to [notification channels](#notification-channel), and is eventually *resolved* (or [silenced](#silence)).

##### Alerting rule {#alerting-rule}
The [PromQL](#promql)/[LogQL](#logql) expression plus its severity, labels, and `for:` duration that defines when an alert fires.
Distinct from a [recording rule](#recording-rule) in that an alerting rule's output is a firing/not-firing condition, not a new metric.

##### Alertmanager {#alertmanager}
The component that receives fired alerts from [Prometheus](#prometheus), groups and deduplicates them, applies routing trees, fans out to [notification channels](#notification-channel), and handles silences and inhibitions.

##### Maintenance window / Silence {#silence}
A time-bounded suppression of alerts matching a label selector.
Used during planned work to prevent expected anomalies from paging the on-call.

##### Notification channel {#notification-channel}
A destination [Alertmanager](#alertmanager) fans alerts to: PagerDuty, Slack, email, OpsGenie, generic webhook, etc.
Configured in Alertmanager's routing tree.

##### SLA (Service Level Agreement) {#sla}
A commitment made to a customer or downstream consumer about service reliability, often financially backed.
Typically looser than the internal [SLO](#slo) that supports it — you might commit 99.9% to a customer (SLA) while targeting 99.95% internally (SLO) to leave buffer for absorbing routine incidents without breaching the agreement.

##### SLO (Service Level Objective) {#slo}
A target reliability number expressed as a percentage over a time window — e.g., "99.9% of peek queries complete in under 250ms over a 30-day rolling window."
SLOs are computed from SLIs (Service Level Indicators — the raw measurements feeding the objective) and typically drive *error budgets* and paging policy.
Distinct from an [alerting rule](#alerting-rule): an SLO is a goal, an alerting rule is a condition that triggers when the goal is at risk.

## Collection and pipeline

##### Alloy {#alloy}
[Grafana Alloy](https://grafana.com/docs/alloy/) is the collector at the heart of this stack.
It scrapes metrics, ingests logs, applies [relabeling](#relabeling), and exports to one or more backends ([Prometheus](#prometheus) remote-write, [Loki](#loki) push, [OTLP](#otlp), the [Datadog Agent](#datadog)).
Replaces older agents like Grafana Agent and Promtail.

##### Monitoring Profile {#monitoring-profile}
A preset values file under `charts/materialize-monitoring/profiles/`, applied with `helm -f`, that configures the chart for a particular deployment shape — object-storage backend, split namespaces, an external Grafana, Loki sizing, and so on.
The bundled backend stack ([Loki](#loki), [Thanos](#thanos), [Grafana](#grafana), [Alertmanager](#alertmanager), plus the [Alloy](#alloy) pipeline) is enabled by default through the `tags.default` group; profiles layer on top.
Distinct from a *profile* in the continuous-profiling sense (Pyroscope, etc.).

##### Prometheus Exporter {#prometheus-exporter}
A small process that translates some other system's state into [Prometheus](#prometheus) metrics on an HTTP endpoint.
[`kube-state-metrics`](#kube-state-metrics), [`node-exporter`](#node-exporter), and [`cAdvisor`](#cadvisor) are common examples.
Materialize itself exports its metrics natively from `environmentd` and `clusterd` — no separate exporter is required for the Materialize surface.

##### Relabeling {#relabeling}
Rewriting label keys and values during ingestion.
Used to normalize divergent label naming across deployment modes (self-managed vs. Cloud vs. BYOC) and to drop or coalesce high-[cardinality](#cardinality) labels before metrics egress to expensive backends.

##### Remote write {#remote-write}
The [Prometheus](#prometheus) protocol for pushing metrics to a downstream backend (another Prometheus, [Thanos](#thanos), Mimir, [Datadog](#datadog), Honeycomb, etc.).
The `prometheus-remote-write` profile is the most common shape for customers forwarding metrics to a paid backend.

##### Scrape {#scrape}
The act of pulling metrics from an HTTP `/metrics` endpoint.
[Prometheus](#prometheus) and [Alloy](#alloy) both scrape; targets are discovered either statically (in config) or dynamically via [ServiceMonitor / PodMonitor](#servicemonitor) resources.

##### Telemetry Pipeline {#telemetry-pipeline}
The [Alloy](#alloy) configuration that describes how telemetry flows from sources to backends — scrape targets, relabeling rules, filters, and routing.
This stack ships several reference pipelines, each parameterized as a [monitoring profile](#monitoring-profile).

## Stack components

##### Datadog / Datadog Agent {#datadog}
A commercial monitoring backend and its in-cluster collector.
The `datadog-agent` profile assumes a customer-managed Datadog Agent and configures [Alloy](#alloy) to feed it via OpenMetrics or DogStatsD.

##### Grafana {#grafana}
The dashboard frontend in the bundled stack and the canonical target for the open-source dashboard outputs.
Queries [Prometheus](#prometheus), [Thanos](#thanos), and [Loki](#loki); renders panels.

##### Loki {#loki}
A log-storage backend modeled after [Prometheus](#prometheus).
Indexes only labels (not log contents), which keeps it cheap at high volume but forces discipline about what becomes a label vs. what stays in the body.

##### OpenTelemetry / OTLP {#otlp}
The vendor-neutral telemetry standard and its wire protocol.
The `otlp` profile lets customers running OTel-native backends (Honeycomb-class) receive metrics and logs without standing up [Prometheus](#prometheus).

##### Prometheus {#prometheus}
The de-facto open-source metrics database.
Pull-based scraping, time-series storage, [PromQL](#promql), alerting rules — the conceptual foundation most of this vocabulary sits on top of.

##### Thanos {#thanos}
A long-term-storage and global-query layer that sits behind [Prometheus](#prometheus).
Stores blocks in S3-compatible object storage and provides a unified query view across multiple Prometheus instances; ships as part of the bundled stack for customers wanting cross-cluster federation.

## Cloud Technologies

Managed metric/log backends offered by the cloud providers.
Relevant when a customer wants to forward telemetry into their existing cloud console rather than run the bundled stack.
The recurring theme: the managed-Prometheus options ([GMP](#gmp), [AMP](#amp)) are sample-billed, PromQL-native, and cardinality-tolerant, while the native custom-metric surfaces ([GCM](#gcm)'s `workload.googleapis.com/` domain, [CloudWatch](#cloudwatch)) bill per metric and reward the same [cardinality](#cardinality) discipline as [Datadog](#datadog).

##### AMP (Amazon Managed Service for Prometheus) {#amp}
AWS's managed, Prometheus-compatible metrics store.
Ingests via Prometheus [remote-write](#remote-write) authenticated with AWS SigV4 (hence the `sigv4` auth option on the remote-write destination), retains ~150 days, and is queried with PromQL directly or through [Grafana](#grafana).
The AWS analogue of [GMP](#gmp); like GMP it doesn't scrape — you point [Alloy](#alloy) (or a Prometheus agent) at it.

##### CloudWatch {#cloudwatch}
AWS's native monitoring service — metrics, logs, alarms, and dashboards in one product.
The AWS analogue of [GCO](#gco) (spanning both [GCM](#gcm) and [GCL](#gcl)).
Custom metrics are billed per-metric and per-API-call, so prefer [AMP](#amp) for high-volume Prometheus-shaped data and reserve CloudWatch custom metrics for a small, low-[cardinality](#cardinality) set.

##### GCL (Google Cloud Logging) {#gcl}
Google Cloud's managed log store (formerly Stackdriver Logging), the logging half of [GCO](#gco).
The OTel googlecloud exporter can route logs here in the same pipeline that sends metrics to [GCM](#gcm).
The GCP analogue of [Loki](#loki), or of AWS [CloudWatch](#cloudwatch) Logs.

##### GCM (Google Cloud Monitoring) {#gcm}
Google Cloud's managed metrics product (formerly Stackdriver Monitoring), the metrics half of [GCO](#gco) — both the store and the query/dashboard/alerting surface.
Metrics live under type domains: `workload.googleapis.com/` (written by the OTel googlecloud exporter — per-MiB ingestion billing, a metric-descriptor model with a ~30-label cap and a per-minute descriptor-creation throttle) and `prometheus.googleapis.com/` (written by [GMP](#gmp)).
Both domains are dashboardable and alertable within GCM.
[Cardinality](#cardinality) drives cost — see [Operating > Tuning](../operating/tuning/).

##### GCO (Google Cloud Observability) {#gco}
Google's umbrella observability suite (formerly Stackdriver, then "Google Cloud's operations suite") — spans [GCM](#gcm) (metrics), [GCL](#gcl) (logging), plus Trace, Profiler, and Error Reporting.
When someone says "Stackdriver," they mean some part of GCO.
The GCP analogue of AWS [CloudWatch](#cloudwatch).

##### GMP (Google Managed Service for Prometheus) {#gmp}
Google's managed, Prometheus-compatible metrics store, backed by Monarch.
Not a separate product from [GCM](#gcm) but an ingestion path *into* it, landing data under the `prometheus.googleapis.com/` domain where it's queryable by PromQL and usable in GCM dashboards and alerts.
Billed per sample ingested — cheaper and more cardinality-tolerant than the `workload.googleapis.com/` domain — retains 24 months, and (like [AMP](#amp)) doesn't scrape: you push to it via [remote-write](#remote-write) or [OTLP](#otlp).
The natural long-term store for GCP-centralized customers, replacing a self-run [Thanos](#thanos).

## Kubernetes integration

##### cAdvisor {#cadvisor}
Container Advisor — collects per-container resource usage (CPU, memory, network) from the kubelet.
Surfaces metrics like `container_cpu_usage_seconds_total` and `container_memory_working_set_bytes`.
Built into the kubelet; no separate install required.

##### CRD / CR {#crd}
*Custom Resource Definition (CRD)* — a Kubernetes object that extends the API with a new resource type.
*Custom Resource (CR)* — an instance of a CRD.
This stack relies on CRDs from the [Prometheus Operator](#prometheus-operator) (`ServiceMonitor`, `PodMonitor`, `PrometheusRule`) and the Grafana Operator (`Grafana`, `Dashboard`, `Datasource`); the `prometheus-crds` subchart ships the Prometheus set for clusters that don't already run the operator.

##### kube-state-metrics {#kube-state-metrics}
An exporter that surfaces metrics about Kubernetes API objects: pod readiness, deployment replicas desired vs. available, PVC status, etc.
Different from `metrics-server`, which provides resource utilization for the horizontal pod autoscaler.

##### Kubernetes Probe {#kubernetes-probe}
A Kubernetes health-check mechanism on a container.
Three flavors: a *liveness probe* restarts the container when it fails (the process is wedged); a *readiness probe* removes the pod from Service load-balancing (the process is up but not ready to serve); a *startup probe* delays liveness/readiness during slow boots (e.g., during long [hydration](#hydration)).
Probe failures surface through [kube-state-metrics](#kube-state-metrics) and pod events.

##### Kubernetes Sidecar {#kubernetes-sidecar}
A secondary container running in the same Kubernetes pod as a primary application container, sharing the pod's network namespace and (often) volumes.
In observability, sidecars commonly inject log shippers or proxies — though this stack prefers a DaemonSet (one [Alloy](#alloy) per node) over per-pod sidecars for collector workloads, since the DaemonSet shape scales with nodes rather than pods.

##### node-exporter {#node-exporter}
An exporter that surfaces per-host (node-level) metrics — CPU utilization, memory, disk usage, filesystem stats, block-device counters, network counters — pulled from the Linux kernel.
Typically deployed as a DaemonSet (one pod per node).
Distinct from [cAdvisor](#cadvisor), which surfaces per-*container* metrics from the kubelet; node-exporter sees the whole machine.

##### Prometheus Operator {#prometheus-operator}
A Kubernetes operator that manages [Prometheus](#prometheus), [Alertmanager](#alertmanager), and the CRDs (`ServiceMonitor`, `PodMonitor`, `PrometheusRule`, etc.) that drive their configuration.
Optional — [Alloy](#alloy) alone can scrape without it — but ubiquitous in Kubernetes monitoring stacks.

##### ServiceMonitor / PodMonitor {#servicemonitor}
Custom resources owned by the [Prometheus Operator](#prometheus-operator) that tell Prometheus/Alloy what to scrape.
ServiceMonitors target Services; PodMonitors target Pods directly.
This stack does **not** ship ServiceMonitors for Materialize itself — those live with the Materialize application chart — but [Alloy](#alloy) consumes them when present.

## Deployment models

##### DIY scraping {#diy-scraping}
The "I'll scrape `environmentd` and `clusterd` myself" path.
Customers who already operate a complete observability stack can ignore most of this repo and just consume the [documented metrics surface](../reference/stable-metrics/).
Source-side defaults are kept minimal specifically so this path stays viable.

##### Materialize Cloud {#materialize-cloud}
Materialize's fully managed offering.
The Cloud team operates its own observability stack built on this repo's primitives plus a private overlay; customers on Cloud don't run monitoring components themselves.

##### Self-managed {#self-managed}
Customer-hosted Materialize, installed via the Materialize Terraform module or the Helm operator.
The primary audience for this monitoring stack.

## Materialize concepts

These terms come from the Materialize database itself; many show up as labels on metrics or in panel descriptions.
Listed here because the dashboards lean on them and because some of the Materialize-side vocabulary is not what a DBA from a row-store background would expect.

##### Arrangement {#arrangement}
An in-memory index of intermediate state that a [dataflow](#dataflow) maintains so it can answer queries quickly.
Arrangements are why `SELECT ... FROM <materialized_view>` can be sub-millisecond — the result is already maintained on the cluster.
Compute-cluster memory usage is often dominated by arrangement size.

##### Cluster (Materialize) {#cluster}
A named pool of compute resources within a Materialize environment.
**Not a Kubernetes cluster.**
Workloads (indexes, materialized views, sources, sinks, subscribes) are placed on a Materialize cluster; the cluster has one or more [replicas](#replica) providing the actual capacity.
In metric labels you'll see this as `cluster_id` or `instance_id`.

##### clusterd {#clusterd}
The Materialize process that runs the compute side of a [replica](#replica) — executes [dataflows](#dataflow), maintains [arrangements](#arrangement), serves [peeks](#peek).
Typically multiple `clusterd` pods per replica (one per [worker](#worker)).

##### Console {#console}
Materialize's web application — accessible at `console.materialize.com` for Cloud customers, and at the chart-configured ingress in self-managed installs.
Manages clusters and their objects (sources, sinks, materialized views, indexes), provides a SQL shell for ad-hoc queries, and renders its own built-in dashboards focused on user-object activity.
Distinct from the [Grafana](#grafana) dashboards this stack ships, which are pulled from Prometheus and lean toward component- and infrastructure-level views.

##### Dataflow {#dataflow}
The streaming execution plan Materialize compiles a query (or materialized view) into.
Built on differential dataflow; each operator processes updates and emits updates.
Long-lived dataflows back materialized views and indexes; short-lived dataflows back ad-hoc reads.

##### Environment / environmentd {#environment}
A Materialize environment is the top-level tenancy unit; typically one per customer or per dev/staging/prod.
`environmentd` is the control-plane process that owns the catalog, accepts SQL connections, schedules dataflows onto clusters, and emits most environment-scoped metrics.

##### Hydration {#hydration}
The process of bringing a [dataflow](#dataflow)'s initial state up to current before it can serve queries.
A newly created materialized view or index is *hydrating* until it catches up to upstream sources.
Hydration time depends on data volume and source freshness; the *Hydration* row on the compute dashboard surfaces this.

##### Index {#index}
A maintained data structure that lets queries read a subset of a collection without a full scan.
In Materialize, indexes are always backed by an [arrangement](#arrangement) on a specific cluster — creating an index commits compute and memory to that cluster.

##### Materialized view {#materialized-view}
A query whose result is incrementally maintained by a [dataflow](#dataflow) as inputs change.
The user writes a SQL query once; Materialize keeps the answer continuously up-to-date.

##### Peek {#peek}
Materialize's internal term for a read query against a materialized view or indexed table — the differential-dataflow operation that turns "give me the current value of this query" into bytes.
Latency metrics like `v2_mz_compute_replica_peek_duration_seconds_*` describe peek latency, which is what users perceive as query latency.
**There is no metric named "query latency"** — look for *peek*.

##### Replica (Materialize) {#replica}
A unit of compute capacity inside a Materialize cluster, sized in [workers](#worker).
Multiple replicas in one cluster provide redundancy.
**Not a Kubernetes replica** — though under the hood each Materialize replica is implemented as one or more Kubernetes pods.

##### s2 / mz_catalog_server {#catalog-server}
The Materialize system cluster that runs internal catalog and introspection workloads.
It dominates many environment-scoped panels (peek counts, arrangement maintenance, hydration) because it's continuously busy maintaining the catalog.
Its noise floor is business-as-usual, not an anomaly — flagged here because confusing s2 activity for user activity is a common false positive.

##### Sink {#sink}
A streaming write from Materialize to an external system — Kafka, Iceberg, Postgres, MySQL.
Sinks have throughput, lag, and (for transactional sinks like Iceberg) commit-latency metrics.

##### Source {#source}
A streaming read into Materialize from an external system — Kafka, Postgres CDC, MySQL CDC, webhooks.
Some source types (Postgres, MySQL) fan out into multiple [subsources](#subsource), one per replicated table.

##### Subsource {#subsource}
A child of a multi-table source — e.g., the per-table replication streams of a Postgres source.
Important wrinkle: the `source_id` label on `mz_source_bytes_received` is actually the *subsource* id; the parent id lives in `parent_source_id`.
Aggregate by `parent_source_id` to get per-primary-source rates.

##### Worker {#worker}
A single thread of execution inside a [`clusterd`](#clusterd) replica.
Replica size determines worker count.
Per-worker metrics can surface skew or hot-spot issues but multiply [cardinality](#cardinality), so dashboards typically aggregate to the cluster or replica level by default; per-worker breakdowns exist where the panel's whole point is detecting drift.
