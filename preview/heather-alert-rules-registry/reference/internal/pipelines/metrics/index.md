# Metrics




# Metrics Pipelines

Metrics are processed primarily in **otelcol**: Prometheus ingest and the final write use the `prometheus.*` family, but everything between is converted to OTLP and shaped by `otelcol.processor.*` components (we found little value in doing the processing with `prometheus.*` blocks).
They are authored the same way as log pipelines — see [Authoring](/materialize-monitoring/preview/heather-alert-rules-registry/reference/internal/pipelines/authoring/) for the pipeline model, the strict-attributes policy, and the `raw:` escape hatch.

<!-- The logs-side runtime conventions (label families, retention) live in logging.md; this page is their metrics-side analog plus the component reference. -->

## Gateway topology

The gateway carries metrics alongside logs (`packages/alloy-pipelines/gateway.yaml`).
Prometheus ingest is bridged into OTLP, processed in otelcol, then converted back to Prometheus for the write.

The bridge refuses a sample without both `job` and `instance`, and fails the whole remote-write batch it arrived in with a 500.
Scraped series always carry both, and pushed ones need not, so `prometheus.relabel "receiveIdentity"` fills them for pushed samples.
It recognises the Thanos ruler by `ruler_replica` and gives its results `job="thanos-ruler"` and a constant `instance`.
The constant keeps Thanos Query's deduplication on `ruler_replica` working.
Anything else pushed without them gets `job="remote-write"` and `instance="unknown"`.
The Loki ruler fills its own in `loki.loki.rulerConfig`.
The Thanos ruler cannot, because a `replace` in its `write_relabel_configs` panics it at startup.

```
prometheus.receive_http "gateway"             (pushed remote-write, :9090)
  → prometheus.relabel "receiveIdentity"      (fill job/instance)          ─┐
prometheus.operator.podmonitors "default"     (PodMonitor CRs)             ─┤
prometheus.operator.servicemonitors "default" (ServiceMonitor CRs)         ─┴─→ otelcol.receiver.prometheus "inputBridge"  (Prometheus → OTLP) ─┐
otelcol.receiver.otlp                          (OTLP metrics) ─────────────────────────────────────────────────────────────────────────────┤
                                                                                                                                            ▼
                     otelcol.processor.filter "inputMetricProcessor"  (otelcol-side processing choke point)
                                               │
                                               ▼
                     otelcol.processor.memory_limiter "outputMemoryLimiter"  (refuse at 75%, backpressure receivers)
                                               │
                                               ▼
                     otelcol.processor.batch "outputBatch"
                                               │
                                               ▼
                     otelcol.processor.filter "egress"            (type-neutral swap seam)
                                               │
                                               ▼
                     otelcol.exporter.prometheus "outputBridge"   (OTLP → Prometheus, add_metric_suffixes=false)
                                               │
                                               ▼
                     prometheus.relabel "egress"                  (per-destination fan-out)
                                    │           │
                                    ▼           ▼
              prometheus.relabel "<name>"   prometheus.relabel "<name>"   (tier filter, one per destination)
                                    │           │
                                    ▼           ▼
         prometheus.remote_write "<name>"   prometheus.remote_write "<name>"
```

Cloud provider pulls join the same bridge when enabled: each is an instance of a custom component whose body ends in the shared `provider_scrape` (an `instance` pin and a clustered `prometheus.scrape`) → `inputBridge`.
They are covered under [Cloud provider pulls](#cloud-provider-pulls) below.

The operator components enable `clustering` (target load spread across the alloy cluster) and read their scrape defaults from the environment (`GATEWAY_SCRAPE_INTERVAL` / `GATEWAY_SCRAPE_TIMEOUT`, via `coalesce`).
`inputMetricProcessor` is the single choke point (the otelcol analog of the loki-side `inputProcessor`); a `memory_limiter` + `batch` pair manages the outbound stream; `otelcol.processor.filter "egress"` is a type-neutral seam so the destination can be swapped without editing the committed pipeline (mirrors the loki side).
`outputBridge` sets `add_metric_suffixes=false` so names survive the OTLP round-trip unchanged.

Everything above `prometheus.relabel "egress"` is shared; everything below it is per destination.
`pipeline.metrics.gateway.destination.prometheusRemoteWrite` is a map keyed by name, and each entry renders **two** components labelled with that name: a `prometheus.relabel` carrying its importance-tier `keep` rule, feeding a `prometheus.remote_write` carrying its endpoint, auth, and TLS.
The default map holds one entry, `thanos`.

The filter is upstream of the component rather than a `write_relabel_config` inside its endpoint, which is the whole reason for one component per destination rather than one component with several `endpoint` blocks.
`write_relabel_config` runs on the way *out* of the write-ahead log, so a shared component would buffer the full firehose to disk for every destination whatever tier it asked for — and a stuck endpoint would hold back WAL truncation for all the others.
The cost is a WAL per destination, sized to what that destination actually takes.

## The destination is Helm-templated, not schema-validated

This is the important boundary.
Only the **processing** pipeline (`gateway.yaml` → `pre-rendered/pipelines/gateway.alloy`) goes through `mz-monitoring-build`, so only it gets JSONSchema validation and typed rendering.
The **destination** — the `otelcol.processor.filter "egress"` (metrics) and `loki.process "egress"` (logs) swap seams, the prometheus tail (`prometheus.relabel "egress"` → the per-destination `prometheus.relabel "<name>"` → `prometheus.remote_write "<name>"`), and the `loki.write "destination"` sink — is rendered by the Helm helper `charts/materialize-monitoring/templates/_alloy_helpers.tpl` at install time, driven by `.Values.pipeline.metrics.gateway.destination.prometheusRemoteWrite` (and the `logging.*.loki` analog).

That templated alloy text **never passes through our schema**.
Its only safety net is the pre-validate jobs (`charts/.../templates/pipelines/validator/job-validate-gateway.yaml`), which run `alloy validate` on the assembled configMap at deploy time.
So when you change destination rendering, the feedback loop is `alloy validate` (via those jobs / a real load), not the schema.

`gateway-dest-stub.yaml` is **not** what deploys — it is a committed stand-in for the egress seam so `make pipelines` can render `gateway.alloy` and `alloy validate` it jointly at build/CI time (`gateway.alloy` dangles the `egress` ref on its own).
Keep the stub roughly in step with the helper, but the helper is the source of truth for what actually runs.

### Destination auth

The metrics `remote_write` helper (`mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest`) supports `authType` of `none`, `basicAuth`, `bearer`, and `sigv4` (secrets sourced from env vars), **per destination**.
Every destination resolves through `mzmon.alloyGateway.promDest.resolve`, which is the one place the per-destination defaults and the derived environment variable names (`GATEWAY_PROM_DEST_<NAME>`, `GATEWAY_PROMETHEUS_DEST_<NAME>_PASSWORD`, `GATEWAY_UNFILTERED_PROM_METRICS_<NAME>`) are written down — the env ConfigMap and the pipeline both read it, so they cannot disagree about a name.
It resolves each field with `dig` rather than `mergeOverwrite` on purpose: `mergeOverwrite` will not override with a zero value, so `enabled: false` and `tls.verify: false` would both be silently discarded.
`sigv4` targets Amazon Managed Prometheus: set `authType: sigv4` + `sigv4.region` (optional `roleArn`) and bind the gateway ServiceAccount via IRSA — the AWS default credential chain then picks up the injected web-identity token, so no static keys.
See the user-facing [Storing](../../../../metrics/storing/) page for the values.
(In a *hand-authored* pipeline — outside the chart destination — the same is reachable via a `raw:` `sigv4` block nested in the endpoint; see below.)

## Cloud provider pulls

`pipeline.metrics.provider.{cloudwatch,gcp}` pull provider metrics into the gateway, feeding `otelcol.receiver.prometheus "inputBridge"`.
The operator-facing page is [Cloud Provider Metrics](../../../../metrics/collecting/cloud-provider-metrics/).

**The pulls are custom components, and the first use of the pattern** described in [Authoring](/materialize-monitoring/preview/heather-alert-rules-registry/reference/internal/pipelines/authoring/#custom-components-declare).
`packages/alloy-pipelines/gateway-provider.yaml` declares them, and it deploys: the chart includes the rendered `gateway-provider.alloy` in every gateway config, where it is inert until instantiated.
The chart's `mzmon.alloyGateway.pipeline.provider` renders only instances, one per resource named in values, each a flat block of arguments.

| Component | Instances | Pulls |
|---|---|---|
| `provider_scrape` | Inside the others | The `instance` pin and the clustered scrape every pull ends in |
| `provider_cloudwatch_rds` | One per RDS instance | 12 metrics, 13 series, from one `GetMetricStatistics` call each |
| `provider_cloudwatch_s3` | One per bucket | `BucketSizeBytes` in standard storage and `NumberOfObjects`, daily |
| `provider_gcp_cloudsql` | One per project | CPU, memory, disk, backends, transaction-ID use and `up`, for every listed instance through one filter |
| `provider_gcp_gcs` | One per project | `storage/v2/total_bytes` and `total_count`, for every listed bucket through one filter |

A `declare` body cannot repeat a block, so the metric sets are fixed in the module rather than values, and CloudWatch takes one instance, and one exporter, per resource.
Cloud Monitoring takes one per service, because a single filter names every resource.
`gateway-provider-stub.yaml` instantiates each component once so `make pipelines` checks the argument names the chart passes; keep it in step with the helper's instances.

Behaviours that decide the shape, all measured against Alloy v1.19.2 source and a v1.20.0 run:

| Behaviour | Consequence |
|---|---|
| Both exporters call the provider **on scrape** | A clustered `prometheus.scrape` gives each target one owner, so the provider is called once per interval however many replicas there are |
| The exporter target's labels are identical across replicas | Clustering works. `instance` is an MD5 of the exporter's arguments, so it is still pinned with a relabel: otherwise any config change re-labels every series |
| CloudWatch `decoupled_scraping` polls on a timer in **every** replica | Never rendered |
| CloudWatch `nil_to_zero` defaults to **true** in Alloy | Set false on every job. A resource with no published datapoints produces no series either way; the setting matters for value-less datapoints |
| CloudWatch static jobs use `GetMetricStatistics` and ignore job-level `period`/`length` | Both are set per `metric` |
| A CloudWatch `static` label must be an identifier and becomes the `name` label | Every job is labelled `rds` or `s3`, and the resource is in `dimension_*` and `instance`. Instance labels are `rds_<id>` / `s3_<bucket>` with other characters mapped to `_`, and the render refuses two resources that collide |
| GCP `metrics_prefixes` are prefixes; `extra_filters` split on the **first** colon | One `one_of(...)` filter per service, rendered only beside that service's prefixes. `database_id` keeps its `project:instance` colon |
| GCP samples carry Cloud Monitoring's timestamps; DELTA metrics count only the newest point per pull | `api/request_count` is left out; queries use `last_over_time` |
| Remote write's WAL watcher forwards only samples with `T > startTimestamp` (Prometheus `tsdb/wlog/watcher.go`, vendored at v0.313.2 by Alloy v1.19.2) | A sample stamped before the gateway started is never sent, so each restart leaves a gap as long as the provider's lag: minutes for Cloud SQL, over ten for GCS. Measured on the first rollout |
| A failed provider call still answers the scrape with HTTP 200 | `up` does not report pull health |
| YACE's request counters are process-wide and only appear in exporter output | With one exporter per resource, every target a replica owns reports that replica's running total, so the series cannot be summed across `instance`, and no pod-labelled copy exists on Alloy's own `/metrics`. Cost comes from the configuration: 12 calls per RDS instance and 2 per bucket per interval |
| The GCP exporter resolves Application Default Credentials when it is **built**; CloudWatch resolves on each pull | With no ADC source at all (no `GOOGLE_APPLICATION_CREDENTIALS`, no metadata server) the GCP exporter fails to build and the gateway fails its initial load. On GKE the metadata server always answers, so it only degrades there. The render warns, since it cannot tell a direct Workload Identity principal from an install outside Google Cloud |
| `alloy validate` type-checks and never builds a component, so it passes an argument that `alloy run` then refuses (a `scrape_timeout` longer than the interval, two components with the same label) | The initial load fails and the gateway exits, taking every log and metric with it. The pre-validate job cannot see this, so `mzmon.alloy.validate.provider` checks both at render |

Provider families have no registry query, so `mzmon.alloyGateway.metricFilter` appends their name patterns (`aws_rds_.*`, `stackdriver_gcs_bucket_.*`, and so on) at the tier each provider's `metricImportance` assigns.

## Where relabeling lives (three phases)

Metric relabeling splits across three places; putting a rule in the wrong one is the most common mistake.

| Phase | Sees | Home | Use for |
|---|---|---|---|
| **Target** (pre-scrape) | `__meta_kubernetes_*` | the PodMonitor/ServiceMonitor CRs (`relabelings`, `podTargetLabels`); the operator components' `rule` blocks for cross-cutting rules; the cAdvisor ScrapeConfig | which targets to scrape, promoting pod/node labels, per-target renames |
| **Metric** (post-scrape) | final label set only | the otelcol processing at `otelcol.processor.filter "inputMetricProcessor"` (add `filter`/`transform` there) | cross-cutting hygiene, cost governance (metric-name drops), dashboard-contract normalization |
| **Identity** | — | `external_labels` on each `remote_write` destination | install/cluster/region stamps |

`inputMetricProcessor` is intentionally a **rule-free passthrough** today: sources are assumed not to push junk labels in the first place, per-target hygiene lives in the (curated) CRs, and node-label curation lives in the cAdvisor ScrapeConfig.
It's where the metric `filter`/`transform` work (cardinality tiers) will land as genuinely cross-cutting rules come up.

Note: identity is stamped as a `cluster` `external_labels` entry on every `remote_write` destination, sourced from
`env.CLUSTER_NAME`, which the chart fills from `clusterName` (default `default`).
A destination's own `externalLabels` map adds to it, and setting `cluster` there replaces the environment-derived value for that destination alone.

### Node-label curation (cAdvisor)

`packages/prometheus-scrapers/scrapeconfig-cadvisor.yaml` uses an explicit node-label **allowlist** rather than a blanket `labelmap __meta_kubernetes_node_label_(.+)`.
The blanket form promoted every node label (karpenter scheduling hints, cluster-autoscaler flags, hostname, …) onto every cAdvisor series; the allowlist keeps only the dimensions the dashboards use (`topology_kubernetes_io_{region,zone}`, `karpenter_sh_{nodepool,capacity_type}`, `node_kubernetes_io_instance_type`) under their original promoted names, and sets `node` from the node name explicitly (the `__address__` is the apiserver proxy, identical for every node, so node identity has to come from the node name).
Add rows as dashboards need more dimensions; a missing source yields an empty value, which Prometheus drops, so extra rows are harmless.

## Available components

Typed and validated (schema: `packages/mzmon-lib/schemas/alloy/prometheus.schema.yaml`, sugar: `packages/mzmon-lib/src/alloy/components/prometheus.rs`):

| Component | Purpose |
|---|---|
| `prometheus.echo` | Debug sink — prints samples to stdout (`format: text \| openmetrics`). |
| `prometheus.scrape` | Scrapes `targets` and forwards; typed `basic_auth` / `tls_config` / `clustering` sub-blocks and `bearer_token` / `bearer_token_file`. |
| `prometheus.operator.podmonitors` | Discovers `PodMonitor` CRs and scrapes them; typed `clustering` / `selector` (match_labels) / `scrape` / `rule` sub-blocks. |
| `prometheus.operator.servicemonitors` | Same for `ServiceMonitor` CRs; adds `kubernetes_role`. |
| `prometheus.relabel` | Rewrites metric labels via shared `rule` blocks; forwards downstream. |
| `prometheus.receive_http` | Serves a remote-write endpoint; typed `http` server sub-block. |
| `prometheus.remote_write` | Delivers metrics to remote-write endpoints; typed `endpoint` (`url` + scalars). |
| `prometheus.exporter.cadvisor` | Runs cAdvisor in-process. |
| `prometheus.exporter.cloudwatch` | Pulls CloudWatch through YACE; typed `static` jobs with `metric` and `role` sub-blocks. `discovery`, `custom_namespace` and `decoupled_scraping` via `raw:`. |
| `prometheus.exporter.gcp` | Pulls Cloud Monitoring through `stackdriver_exporter`; attributes only. |
| `declare` / `custom` | Define and instantiate a custom component; see [Authoring](/materialize-monitoring/preview/heather-alert-rules-registry/reference/internal/pipelines/authoring/#custom-components-declare). |

The operator `scrape` block's `default_scrape_interval` / `default_scrape_timeout` accept an expression (`{env: …}`), not just a literal duration.
Metrics receivers (`forward_to`, and the `receiver` exported by echo/relabel/remote_write/receive_http) are the `MetricsReceiver` capsule — the metrics analog of the logs-side `LogsReceiver`.
They render as **bare refs**, e.g. `forward_to = [prometheus.remote_write.default.receiver]`.

## Deferred to the `raw:` escape

Scope follows the "only stable, in-cluster, most-likely-used" guidance.
These are reachable today via a `raw:` block and can be graduated to typed schema as usage demands:

- **remote_write endpoint auth/TLS/tuning** — `basic_auth`, `bearer_token`, `oauth2`, `authorization`, `sigv4`, `azuread`, `tls_config`, `queue_config`, `metadata_config`, `wal`, `write_relabel_config`.
  These nest inside the typed `endpoint` via its `blocks:` list, so adding auth does **not** require rewriting the whole endpoint as `raw:`. For example, AMP + IRSA:

  ```yaml
  - endpoint:
      url: https://aps-workspaces.us-east-1.amazonaws.com/workspaces/ws-…/api/v1/remote_write
      blocks:
        - raw:
            component: sigv4
            attributes:
              region: us-east-1   # empty otherwise → AWS default chain → IRSA token
  ```

- **operator `client` block** — the Kubernetes API/auth client.
  Not needed in-cluster (alloy uses the pod's service account); reachable via `raw:` if ever required.
- **scrape `oauth2` / `authorization`** — only `basic_auth` and `tls_config` are typed on `prometheus.scrape`.
- **operator `selector` `match_expression`** — only `match_labels` is typed; set-based selectors use a nested `raw:` block.
- **receive_http `tls`** — the server-side TLS block.

> Load-testing note: `alloy validate` does not catch capsule-type mismatches (see [Authoring](/materialize-monitoring/preview/heather-alert-rules-registry/reference/internal/pipelines/authoring/)).
> Because the destination is Helm-templated and skips the schema entirely, treat `alloy validate` (the pre-validate jobs) as the real check for destination changes, and load-test the first live metrics pipeline with a real `alloy run`.

