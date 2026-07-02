---
title: "Logging"
weight: 20
---

# Logging Pipeline

This is the authoritative reference for the log-processing pipelines that run on `alloy-agent` and `alloy-gateway`.
The customer-facing [Logs & Events](../../../../logs-and-events/) section links here for pipeline detail; this page is for repo contributors.

<!--
Agent note: this page documents pipeline *conventions and shape*, not the full per-application match table — that lives in the pipeline sources and changes often. When you change pipeline behavior, attribute it via the implementing PR (see "Attribution and adoption status") rather than restating the whole config here.
-->

## Where the pipelines live

The pipelines are authored as code (see [Authoring](../authoring/)) and rendered to `config.alloy` by `mz-monitoring-build gen-pipelines`:

- `packages/alloy-pipelines/agent.yaml` — the agent pipeline (adopted).
- `packages/alloy-pipelines/gateway.yaml` — the gateway pipeline (in-flight; currently a stub).
- `packages/ref-alloy-pipelines/*.alloy` — the rendered reference pipelines from Cloud, used as the porting target. Not adopted and not checked in as the source of truth; treat the `.alloy` files as a behavioral reference only.
- `packages/mzmon-lib/schemas/alloy/` — the JSONSchema the YAML validates against.

## Agent pipeline

The agent collects node-local data and forwards it to the gateway with minimal processing:

- **Sources:** `loki.source.journal` (host systemd journal) and `loki.source.file` over pods discovered with `discovery.kubernetes` (role `pod`, filtered to the local node).
- **Relabeling:** `discovery.relabel` normalizes Kubernetes metadata into stable label names (`namespace`, `pod`, `container`, `node`, `app`, `component`) and node attributes (`region`, `zone`, `instance_type`, `nodepool`, …), and builds the `__path__` to the pod log files.
- **Light processing:** `stage.cri` parses CRI-formatted lines; `stage.limit` applies a per-node rate cap; `stage.static_labels` tags the `cluster`.
- **Forward:** `loki.write` to the gateway at `http://alloy-gateway.$namespace.svc:3100/loki/api/v1/push`.

Tunable inputs: `AGENT_POD_LOG_RATE_LIMIT` (default `5000`), `AGENT_POD_LOG_BURST` (default `20000`), `CLUSTER_NAME`, `HOSTNAME`.

## Gateway pipeline

The gateway is where normalization, cardinality reduction, and routing happen.

### Receivers

- `loki.source.api` on port **3100** — Loki push traffic from agents (and any other Loki-push client, including a chained upstream gateway).
- `otelcol.receiver.otlp` on ports **4317** (gRPC) and **4318** (HTTP) — OTLP logs from instrumented applications or forwarders.
- `loki.source.kubernetes_events` — Kubernetes events, processed as log lines.

### Processing conventions

These are the conventions a contributor must preserve when editing the gateway `loki.process` pipeline:

- **Level normalization.** A per-application `stage.match` extracts the level, then a series of `stage.replace` rules normalize it to one of `CRITICAL`, `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`. A heuristic regex backfills `UNKNOWN` levels, and the success/failure of that heuristic is recorded as structured metadata.
- **Drops and limits.** Lines older than the ingestion backlog window or larger than the per-line ceiling are dropped; per-level rate limits keep `INFO`/unknown chatter bounded while letting `ERROR`/`CRITICAL` through.
- **Label families.** Only a small, stable set is promoted to **Loki labels**: `level`, `app`, `container`, `namespace`, and their `k8s_`-prefixed forms (`k8s_namespace`, `k8s_app`, `k8s_container`, `k8s_pod`), plus `environment_id` for environment namespaces. Everything else identifying — `pod`, `node`, `pod_id`, `container_id`, `region`, `zone`, `nodepool`, `trace_id`, `span_id`, `error`, `msg`, … — is routed to **structured metadata** so it stays queryable without inflating stream cardinality.
- **Timestamps.** Parsed from the source line (`ts`/`timestamp`) as `RFC3339`/`RFC3339Nano` where the application provides one.

> [!WARNING]
>   The label-vs-structured-metadata split is the dominant cost-and-stability lever.
>   Adding a new Loki label multiplies [cardinality](../../../../o11y-glossary/#observability-foundations) — default to structured metadata and promote to a label only when it is low-cardinality and used as a selector.

### Destinations

- **Logs → Loki.** `loki.write` to the bundled Loki distributor (or, in the [remote-only topology](../../../../logs-and-events/#alternative-topologies), to an external OTLP/Loki destination).
- **Recording-rule metrics → long-term metric store.** The [Loki Ruler](../../../../logs-and-events/#ruler) remote-writes recording-rule samples back through the gateway, which forwards them to Thanos via `prometheus.remote_write` alongside the metrics pipeline (see [Metrics](../metrics/)).

## Attribution and adoption status

Pipeline behavior is **change-tracked via pull requests** against the sources above; cite the implementing PR when you change a stage, a label family, or an endpoint, rather than only editing prose here.
Per-component history is captured in the repo `CHANGELOG.md` and the [Releasing](../../releasing/) flow.

Current status (see the [Roadmap](../../roadmap/)):

- **Agent pipeline** — adopted in `packages/alloy-pipelines/agent.yaml`.
- **Gateway pipeline** — in-flight (M2). `packages/alloy-pipelines/gateway.yaml` is a stub; the behavior described above is the porting target from `packages/ref-alloy-pipelines/processor.alloy`. Confirm endpoints and stage details against the gateway PR when it lands.

## See more

- [Authoring](../authoring/) — schema model and how to extend it.
- [Metrics](../metrics/) — the metrics-side pipeline conventions.
- [Logs & Events](../../../../logs-and-events/) — the customer-facing architecture this pipeline feeds.
