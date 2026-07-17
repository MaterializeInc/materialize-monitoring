---
title: "Metrics"
weight: 10
---

# Metrics Pipelines

Metrics are processed primarily in **otelcol**: Prometheus ingest and the final write use the `prometheus.*` family, but everything between is converted to OTLP and shaped by `otelcol.processor.*` components (we found little value in doing the processing with `prometheus.*` blocks).
They are authored the same way as log pipelines — see [Authoring]({{< relref "authoring.md" >}}) for the pipeline model, the strict-attributes policy, and the `raw:` escape hatch.

<!-- The logs-side runtime conventions (label families, retention) live in logging.md; this page is their metrics-side analog plus the component reference. -->

## Gateway topology

The gateway carries metrics alongside logs (`packages/alloy-pipelines/gateway.yaml`).
Prometheus ingest is bridged into OTLP, processed in otelcol, then converted back to Prometheus for the write:

```
prometheus.receive_http "gateway"             (pushed remote-write, :9090) ─┐
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
                     prometheus.relabel "egress" ─→ prometheus.remote_write "destination"
```

The operator components enable `clustering` (target load spread across the alloy cluster) and read their scrape defaults from the environment (`GATEWAY_SCRAPE_INTERVAL` / `GATEWAY_SCRAPE_TIMEOUT`, via `coalesce`).
`inputMetricProcessor` is the single choke point (the otelcol analog of the loki-side `inputProcessor`); a `memory_limiter` + `batch` pair manages the outbound stream; `otelcol.processor.filter "egress"` is a type-neutral seam so the destination can be swapped without editing the committed pipeline (mirrors the loki side).
`outputBridge` sets `add_metric_suffixes=false` so names survive the OTLP round-trip unchanged.

## The destination is Helm-templated, not schema-validated

This is the important boundary.
Only the **processing** pipeline (`gateway.yaml` → `pre-rendered/pipelines/gateway.alloy`) goes through `mz-monitoring-build`, so only it gets JSONSchema validation and typed rendering.
The **destination** — the `otelcol.processor.filter "egress"` (metrics) and `loki.process "egress"` (logs) swap seams, the prometheus tail (`prometheus.relabel "egress"` → `prometheus.remote_write "destination"`), and the `loki.write "destination"` sink — is rendered by the Helm helper `charts/materialize-monitoring/templates/_alloy_helpers.tpl` at install time, driven by `.Values.pipeline.metrics.gateway.destination.prometheusRemoteWrite` (and the `logging.*.loki` analog).

That templated alloy text **never passes through our schema**.
Its only safety net is the pre-validate jobs (`charts/.../templates/pipelines/validator/job-validate-gateway.yaml`), which run `alloy validate` on the assembled configMap at deploy time.
So when you change destination rendering, the feedback loop is `alloy validate` (via those jobs / a real load), not the schema.

`gateway-dest-stub.yaml` is **not** what deploys — it is a committed stand-in for the egress seam so `make pipelines` can render `gateway.alloy` and `alloy validate` it jointly at build/CI time (`gateway.alloy` dangles the `egress` ref on its own).
Keep the stub roughly in step with the helper, but the helper is the source of truth for what actually runs.

### Destination auth

The metrics `remote_write` helper (`mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest`) supports `authType` of `none`, `basicAuth`, `bearer`, and `sigv4` (secrets sourced from env vars).
`sigv4` targets Amazon Managed Prometheus: set `authType: sigv4` + `sigv4.region` (optional `roleArn`) and bind the gateway ServiceAccount via IRSA — the AWS default credential chain then picks up the injected web-identity token, so no static keys.
See the user-facing [Storing](../../../metrics/storing/) page for the values.
(In a *hand-authored* pipeline — outside the chart destination — the same is reachable via a `raw:` `sigv4` block nested in the endpoint; see below.)

## Where relabeling lives (three phases)

Metric relabeling splits across three places; putting a rule in the wrong one is the most common mistake.

| Phase | Sees | Home | Use for |
|---|---|---|---|
| **Target** (pre-scrape) | `__meta_kubernetes_*` | the PodMonitor/ServiceMonitor CRs (`relabelings`, `podTargetLabels`); the operator components' `rule` blocks for cross-cutting rules; the cAdvisor ScrapeConfig | which targets to scrape, promoting pod/node labels, per-target renames |
| **Metric** (post-scrape) | final label set only | the otelcol processing at `otelcol.processor.filter "inputMetricProcessor"` (add `filter`/`transform` there) | cross-cutting hygiene, cost governance (metric-name drops), dashboard-contract normalization |
| **Identity** | — | `external_labels` on the `remote_write` destination | install/cluster/region stamps |

`inputMetricProcessor` is intentionally a **rule-free passthrough** today: sources are assumed not to push junk labels in the first place, per-target hygiene lives in the (curated) CRs, and node-label curation lives in the cAdvisor ScrapeConfig.
It's where the metric `filter`/`transform` work (cardinality tiers) will land as genuinely cross-cutting rules come up.

Note: identity is stamped as a `cluster` `external_labels` entry on the `remote_write` destination, sourced from `env.CLUSTER_NAME` (default `default`).

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

> Load-testing note: `alloy validate` does not catch capsule-type mismatches (see [Authoring]({{< relref "authoring.md" >}})).
> Because the destination is Helm-templated and skips the schema entirely, treat `alloy validate` (the pre-validate jobs) as the real check for destination changes, and load-test the first live metrics pipeline with a real `alloy run`.
