---
title: "o11y Glossary"
weight: 20
---

# Glossary of Observability Terms

A reference for the vocabulary you'll see across these docs. The audience is a Materialize customer or operator: comfortable with SQL, conversant in DBA terminology, working knowledge of Kubernetes. We don't define general database or Kubernetes terms (table, namespace, pod) — only what's specific to monitoring, the Materialize internals you'll meet in panels, or this stack's deployment vocabulary.

## Observability foundations

**Cardinality**
The number of unique time series (or log streams) a metric produces. The dominant cost-and-stability lever for any Prometheus-shaped stack: a single ill-chosen label that takes on millions of values can balloon storage and crash query backends. The materialize-monitoring pipeline applies a cardinality-reduction policy at the gateway layer before metrics egress to expensive backends — see [Operating > Tuning](../operating/tuning/).

**Error**
A signal that something failed — an exception, a 5xx response, a panic. Errors surface in three forms across this stack: log lines at `ERROR` or `WARN` level (see *Log level*), increments on error counters (`*_errors_total`), and elevated rates that trip alerting rules. Per the [RED method](https://grafana.com/blog/2018/08/02/the-red-method-how-to-instrument-your-services/), error rate is one of the three primary indicators (alongside Rate and Duration) worth tracking for any request-serving system.

**I/O**
Input/output — work performed against an external resource (network, disk, another process). I/O metrics in this stack come from cAdvisor (`container_network_*_bytes_total`, `container_fs_*`), node-exporter (block-device counters), and Materialize's own metrics (source bytes received, sink throughput). Disk and network I/O are common bottlenecks for streaming systems.

**Label**
A key-value pair attached to a metric or log line describing one dimension of what was measured. For example, `mz_query_total{cluster_id="u1", query_kind="select"}` has two labels. Labels are how you filter and group in PromQL/LogQL — and the multiplicative source of cardinality.

**Label budget**
A target ceiling on the number of distinct values a given label may take — or, more broadly, on the total cardinality a metric is allowed to produce. Treated as a contract between metric producers and the platform: producers commit to staying under budget, the platform commits to scraping and storing what fits. When a budget is exceeded, this stack's pipeline can drop the offending labels into structured metadata or refuse them entirely. See [Operating > Tuning](../operating/tuning/) for the policy applied here.

**Latency**
The time elapsed between a request being initiated and a response being received. Almost always reported as a *histogram* and queried at p50/p90/p99 — averages hide the tail behavior that matters operationally. Materialize-specific note: read-query latency is exposed as *peek* latency (see *Peek*); there is no metric named "query latency."

**Observability (o11y)**
The practice of inferring a running system's internal state from the signals it externalizes — metrics, logs, and events. "o11y" is the conventional shorthand: the letter `o`, eleven characters, the letter `y`.

**Quantile / Percentile**
A statistical summary describing the value below which a given fraction of observations fall. Latency panels typically show p50 (median), p90, and p99; "p99 = 250ms" means 99% of requests completed in 250ms or less. Computed in PromQL via `histogram_quantile()` against a histogram metric. Don't conflate with averages — averages hide tail behavior, which is usually the thing operators care about.

**Series / time series**
A unique combination of metric name + label values, sampled over time. `mz_query_total{cluster_id="u1"}` and `mz_query_total{cluster_id="u2"}` are two distinct series of the same metric.

**Swap (memory)**
A region of disk the kernel uses as overflow when physical RAM is exhausted. Swapping is generally undesirable for performance-critical workloads — disk is orders of magnitude slower than RAM, so an apparently-running process that's actually swapping can appear catastrophically slow. Most production Kubernetes deployments disable swap entirely; if a node has it enabled, watch `node_memory_SwapUsed_bytes` for early warning of memory pressure.

**Throughput**
The rate at which a system processes work, usually expressed as units per second — bytes/sec, queries/sec, rows/sec. Reported as a *counter* in metrics and queried via `rate(...)`. Sources have ingest throughput; sinks have output throughput; queries have peek-rate throughput.

## Metrics

**Counter**
A metric that only increases (or resets to zero on process restart). Examples: total bytes received, total queries served. Always wrap a counter in `rate(...)` or `increase(...)` in PromQL — the raw value of a counter is rarely directly useful.

**Gauge**
A metric that can go up or down at any time. Examples: current memory usage, currently active connections. Read the value directly; no rate wrapper needed.

**Histogram**
A metric that records the distribution of observed values across configured bucket boundaries. Latency, request-size, and other "spread matters" metrics are almost always histograms. Query a histogram via `histogram_quantile(quantile, sum by (le) (rate(<metric>_bucket[...])))`.

**Metric**
A numeric measurement, identified by a name and a set of labels, that is scraped or pushed at fixed intervals into Prometheus (or another time-series backend). The shape of a metric is captured by its type — counter, gauge, or histogram — and its label set determines its cardinality. Distinct from a *log line* (text, unstructured-by-default, indexed by stream) and a *trace span* (causal, request-scoped).

**Recording rule**
A precomputed PromQL expression evaluated at scrape interval and stored as a new series. Two main uses: speed up expensive queries that appear in many dashboards/alerts, and reduce cardinality before metrics egress to expensive backends. See [Metrics > Rules](../metrics/rules/).

**Sample**
One (timestamp, value) data point in a series. A series is, mechanically, an ordered list of samples.

## Logs and events

**Event**
A discrete occurrence with associated metadata — a deployment, a pod restart, an alert firing. Kubernetes emits events on its API; some of these (notably Pod state changes) flow through this stack alongside container logs.

**Log**
A line (or structured record) emitted by a running process. This stack collects container logs from Materialize's processes (`environmentd`, `clusterd`) as well as the operator and any sidecars. Structured logging (key=value or JSON) queries far better in LogQL than free-form text.

**Log level**
The severity class attached to a log line: typically `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` (some systems add `FATAL` above `ERROR`). Materialize emits at `INFO` by default; production filtering is usually `WARN`/`ERROR` for storage cost, dropping to `INFO` or `DEBUG` for active debugging. Configurable per-process via the standard Rust `RUST_LOG`/log-filter conventions.

**Stream**
The Loki analogue of a Prometheus series: a unique combination of labels under which log entries are ordered by time. As with metrics, label cardinality is the cost driver — keep volatile attributes (request_id, trace_id) in the *log body*, not as Loki labels.

## Dashboards and queries

**Dashboard**
A collection of panels backed by queries, typically laid out in tabs and rows. This stack ships dashboards for both Grafana and DataDog; see [Dashboards](../dashboards/).

**LogQL**
Loki's query language. Borrows PromQL's selector syntax (`{job="mz", container="environmentd"}`) and adds line-filter operators (`|=`, `|~`) and parsing pipelines (`| json | line_format ...`).

**Panel**
A single visualization on a dashboard — a time series, table, donut, single-stat, etc. Most panels are backed by one or more queries.

**PromQL**
Prometheus's query language. Used by Grafana, Alertmanager, recording rules, and alerting rules across this stack. PromQL operates on time series: `rate(metric[5m])` computes a per-second rate; `sum by (label) (...)` aggregates.

## Alerting

**Alert**
A condition expressed as a PromQL (or LogQL) query that, when persistently true for a configured duration (`for: 5m`), signals that something needs attention. An alert *fires*, gets *routed* by Alertmanager to *notification channels*, and is eventually *resolved* (or *silenced*).

**Alerting rule**
The PromQL/LogQL expression plus its severity, labels, and `for:` duration that defines when an alert fires. Distinct from a [recording rule](#metrics) in that an alerting rule's output is a firing/not-firing condition, not a new metric.

**Alertmanager**
The component that receives fired alerts from Prometheus, groups and deduplicates them, applies routing trees, fans out to notification channels, and handles silences and inhibitions.

**Maintenance window / Silence**
A time-bounded suppression of alerts matching a label selector. Used during planned work to prevent expected anomalies from paging the on-call.

**Notification channel**
A destination Alertmanager fans alerts to: PagerDuty, Slack, email, OpsGenie, generic webhook, etc. Configured in Alertmanager's routing tree.

**SLA (Service Level Agreement)**
A commitment made to a customer or downstream consumer about service reliability, often financially backed. Typically looser than the internal SLO that supports it — you might commit 99.9% to a customer (SLA) while targeting 99.95% internally (SLO) to leave buffer for absorbing routine incidents without breaching the agreement.

**SLO (Service Level Objective)**
A target reliability number expressed as a percentage over a time window — e.g., "99.9% of peek queries complete in under 250ms over a 30-day rolling window." SLOs are computed from SLIs (Service Level Indicators — the raw measurements feeding the objective) and typically drive *error budgets* and paging policy. Distinct from an *alerting rule*: an SLO is a goal, an alerting rule is a condition that triggers when the goal is at risk.

## Collection and pipeline

**Alloy**
[Grafana Alloy](https://grafana.com/docs/alloy/) is the collector at the heart of this stack. It scrapes metrics, ingests logs, applies relabeling, and exports to one or more backends (Prometheus remote-write, Loki push, OTLP, the Datadog Agent). Replaces older agents like Grafana Agent and Promtail.

**Exporter**
A small process that translates some other system's state into Prometheus metrics on an HTTP endpoint. `kube-state-metrics`, `node-exporter`, and `cAdvisor` are common examples. Materialize itself exports its metrics natively from `environmentd` and `clusterd` — no separate exporter is required for the Materialize surface.

**Pipeline**
The Alloy configuration that describes how telemetry flows from sources to backends — scrape targets, relabeling rules, filters, and routing. This stack ships several reference pipelines via profiles.

**Profile**
A named pipeline configuration that targets a specific backend shape: `datadog-agent`, `prometheus-remote-write`, `otlp`, or `bundled-stack`. Customers pick a profile via Helm values; the chart wires Alloy and (for the bundled profile) Loki/Thanos/Grafana accordingly.

**Relabeling**
Rewriting label keys and values during ingestion. Used to normalize divergent label naming across deployment modes (self-managed vs. Cloud vs. BYOC) and to drop or coalesce high-cardinality labels before metrics egress to expensive backends.

**Remote write**
The Prometheus protocol for pushing metrics to a downstream backend (another Prometheus, Thanos, Mimir, Datadog, Honeycomb, etc.). The `prometheus-remote-write` profile is the most common shape for customers forwarding metrics to a paid backend.

**Scrape**
The act of pulling metrics from an HTTP `/metrics` endpoint. Prometheus and Alloy both scrape; targets are discovered either statically (in config) or dynamically via ServiceMonitor / PodMonitor resources.

## Stack components

**Datadog / Datadog Agent**
A commercial monitoring backend and its in-cluster collector. The `datadog-agent` profile assumes a customer-managed Datadog Agent and configures Alloy to feed it via OpenMetrics or DogStatsD.

**Grafana**
The dashboard frontend in the bundled stack and the canonical target for the open-source dashboard outputs. Queries Prometheus, Thanos, and Loki; renders panels.

**Loki**
A log-storage backend modeled after Prometheus. Indexes only labels (not log contents), which keeps it cheap at high volume but forces discipline about what becomes a label vs. what stays in the body.

**OpenTelemetry / OTLP**
The vendor-neutral telemetry standard and its wire protocol. The `otlp` profile lets customers running OTel-native backends (Honeycomb-class) receive metrics and logs without standing up Prometheus.

**Prometheus**
The de-facto open-source metrics database. Pull-based scraping, time-series storage, PromQL, alerting rules — the conceptual foundation most of this vocabulary sits on top of.

**Thanos**
A long-term-storage and global-query layer that sits behind Prometheus. Stores blocks in S3-compatible object storage and provides a unified query view across multiple Prometheus instances; ships as part of the bundled stack for customers wanting cross-cluster federation.

## Kubernetes integration

**cAdvisor**
Container Advisor — collects per-container resource usage (CPU, memory, network) from the kubelet. Surfaces metrics like `container_cpu_usage_seconds_total` and `container_memory_working_set_bytes`. Built into the kubelet; no separate install required.

**CRD / CR**
*Custom Resource Definition (CRD)* — a Kubernetes object that extends the API with a new resource type. *Custom Resource (CR)* — an instance of a CRD. This stack relies on CRDs from the Prometheus Operator (`ServiceMonitor`, `PodMonitor`, `PrometheusRule`) and the Grafana Operator (`Grafana`, `Dashboard`, `Datasource`); the `prometheus-crds` subchart ships the Prometheus set for clusters that don't already run the operator.

**kube-state-metrics**
An exporter that surfaces metrics about Kubernetes API objects: pod readiness, deployment replicas desired vs. available, PVC status, etc. Different from `metrics-server`, which provides resource utilization for the horizontal pod autoscaler.

**node-exporter**
An exporter that surfaces per-host (node-level) metrics — CPU utilization, memory, disk usage, filesystem stats, block-device counters, network counters — pulled from the Linux kernel. Typically deployed as a DaemonSet (one pod per node). Distinct from *cAdvisor*, which surfaces per-*container* metrics from the kubelet; node-exporter sees the whole machine.

**Probe**
A Kubernetes health-check mechanism on a container. Three flavors: a *liveness probe* restarts the container when it fails (the process is wedged); a *readiness probe* removes the pod from Service load-balancing (the process is up but not ready to serve); a *startup probe* delays liveness/readiness during slow boots (e.g., during long hydration). Probe failures surface through kube-state-metrics and pod events.

**Prometheus Operator**
A Kubernetes operator that manages Prometheus, Alertmanager, and the CRDs (`ServiceMonitor`, `PodMonitor`, `PrometheusRule`, etc.) that drive their configuration. Optional — Alloy alone can scrape without it — but ubiquitous in Kubernetes monitoring stacks.

**ServiceMonitor / PodMonitor**
Custom resources owned by the Prometheus Operator that tell Prometheus/Alloy what to scrape. ServiceMonitors target Services; PodMonitors target Pods directly. This stack does **not** ship ServiceMonitors for Materialize itself — those live with the Materialize application chart — but Alloy consumes them when present.

**Sidecar**
A secondary container running in the same Kubernetes pod as a primary application container, sharing the pod's network namespace and (often) volumes. In observability, sidecars commonly inject log shippers or proxies — though this stack prefers a DaemonSet (one Alloy per node) over per-pod sidecars for collector workloads, since the DaemonSet shape scales with nodes rather than pods.

## Deployment models

**DIY scraping**
The "I'll scrape `environmentd` and `clusterd` myself" path. Customers who already operate a complete observability stack can ignore most of this repo and just consume the [documented metrics surface](../reference/stable-metrics/). Source-side defaults are kept minimal specifically so this path stays viable.

**Materialize Cloud**
Materialize's fully managed offering. The Cloud team operates its own observability stack built on this repo's primitives plus a private overlay; customers on Cloud don't run monitoring components themselves.

**Self-managed**
Customer-hosted Materialize, installed via the Materialize Terraform module or the Helm operator. The primary audience for this monitoring stack.

## Materialize concepts

These terms come from the Materialize database itself; many show up as labels on metrics or in panel descriptions. Listed here because the dashboards lean on them and because some of the Materialize-side vocabulary is not what a DBA from a row-store background would expect.

**Arrangement**
An in-memory index of intermediate state that a dataflow maintains so it can answer queries quickly. Arrangements are why `SELECT ... FROM <materialized_view>` can be sub-millisecond — the result is already maintained on the cluster. Compute-cluster memory usage is often dominated by arrangement size.

**Cluster (Materialize)**
A named pool of compute resources within a Materialize environment. **Not a Kubernetes cluster.** Workloads (indexes, materialized views, sources, sinks, subscribes) are placed on a Materialize cluster; the cluster has one or more *replicas* providing the actual capacity. In metric labels you'll see this as `cluster_id` or `instance_id`.

**clusterd**
The Materialize process that runs the compute side of a replica — executes dataflows, maintains arrangements, serves peeks. Typically multiple `clusterd` pods per replica (one per worker).

**Console**
Materialize's web application — accessible at `console.materialize.com` for Cloud customers, and at the chart-configured ingress in self-managed installs. Manages clusters and their objects (sources, sinks, materialized views, indexes), provides a SQL shell for ad-hoc queries, and renders its own built-in dashboards focused on user-object activity. Distinct from the Grafana dashboards this stack ships, which are pulled from Prometheus and lean toward component- and infrastructure-level views.

**Dataflow**
The streaming execution plan Materialize compiles a query (or materialized view) into. Built on differential dataflow; each operator processes updates and emits updates. Long-lived dataflows back materialized views and indexes; short-lived dataflows back ad-hoc reads.

**Environment / environmentd**
A Materialize environment is the top-level tenancy unit; typically one per customer or per dev/staging/prod. `environmentd` is the control-plane process that owns the catalog, accepts SQL connections, schedules dataflows onto clusters, and emits most environment-scoped metrics.

**Hydration**
The process of bringing a dataflow's initial state up to current before it can serve queries. A newly created materialized view or index is *hydrating* until it catches up to upstream sources. Hydration time depends on data volume and source freshness; the *Hydration* row on the compute dashboard surfaces this.

**Index**
A maintained data structure that lets queries read a subset of a collection without a full scan. In Materialize, indexes are always backed by an arrangement on a specific cluster — creating an index commits compute and memory to that cluster.

**Materialized view**
A query whose result is incrementally maintained by a dataflow as inputs change. The user writes a SQL query once; Materialize keeps the answer continuously up-to-date.

**Peek**
Materialize's internal term for a read query against a materialized view or indexed table — the differential-dataflow operation that turns "give me the current value of this query" into bytes. Latency metrics like `v2_mz_compute_replica_peek_duration_seconds_*` describe peek latency, which is what users perceive as query latency. **There is no metric named "query latency"** — look for *peek*.

**Replica (Materialize)**
A unit of compute capacity inside a Materialize cluster, sized in *workers*. Multiple replicas in one cluster provide redundancy. **Not a Kubernetes replica** — though under the hood each Materialize replica is implemented as one or more Kubernetes pods.

**s2 / mz_catalog_server**
The Materialize system cluster that runs internal catalog and introspection workloads. It dominates many environment-scoped panels (peek counts, arrangement maintenance, hydration) because it's continuously busy maintaining the catalog. Its noise floor is business-as-usual, not an anomaly — flagged here because confusing s2 activity for user activity is a common false positive.

**Sink**
A streaming write from Materialize to an external system — Kafka, Iceberg, Postgres, MySQL. Sinks have throughput, lag, and (for transactional sinks like Iceberg) commit-latency metrics.

**Source**
A streaming read into Materialize from an external system — Kafka, Postgres CDC, MySQL CDC, webhooks. Some source types (Postgres, MySQL) fan out into multiple *subsources*, one per replicated table.

**Subsource**
A child of a multi-table source — e.g., the per-table replication streams of a Postgres source. Important wrinkle: the `source_id` label on `mz_source_bytes_received` is actually the *subsource* id; the parent id lives in `parent_source_id`. Aggregate by `parent_source_id` to get per-primary-source rates.

**Worker**
A single thread of execution inside a `clusterd` replica. Replica size determines worker count. Per-worker metrics can surface skew or hot-spot issues but multiply cardinality, so dashboards typically aggregate to the cluster or replica level by default; per-worker breakdowns exist where the panel's whole point is detecting drift.
