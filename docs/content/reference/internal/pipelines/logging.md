---
title: "Logging"
weight: 20
---

# Logging Pipeline

This is the authoritative reference for the log-processing pipelines that run on `alloy-agent` and `alloy-gateway`.
The customer-facing [Logs & Events](../../../../logs-and-events/architecture/) section links here for pipeline detail; this page is for repo contributors.

<!--
Agent note: this page documents pipeline *conventions and shape*, not the full per-application match table — that lives in the pipeline sources and changes often. When you change pipeline behavior, attribute it via the implementing PR (see "Attribution and adoption status") rather than restating the whole config here.
-->

## Where the pipelines live

The pipelines are authored as code (see [Authoring](../authoring/)) and rendered to `config.alloy` by `mz-monitoring-build gen-pipelines`:

- `packages/alloy-pipelines/agent.yaml` — the agent pipeline (adopted).
- `packages/alloy-pipelines/gateway.yaml` — the gateway processing pipeline (adopted).
- `packages/alloy-pipelines/gateway-dest-stub.yaml` — the gateway's default egress tail (the passthrough seam + a default `loki.write`), split out so the write destination can be swapped at chart-assembly time. It is not a standalone config; it is validated jointly with `gateway.alloy` (see [Destinations](#destinations)).
- `packages/ref-alloy-pipelines/*.alloy` — the rendered reference pipelines from Cloud, used as the porting target. Not adopted and not checked in as the source of truth; treat the `.alloy` files as a behavioral reference only. The gateway was ported from `staging-gateway.alloy` (the fresher successor to `processor.alloy`).
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

- `loki.source.api` on port **3100** (configurable via `ALLOY_LOKI_PORT`) — Loki push traffic from agents (and any other Loki-push client, including a chained upstream gateway).
- `otelcol.receiver.otlp` on ports **4317** (gRPC) and **4318** (HTTP) — OTLP logs from instrumented applications or forwarders, bridged into the loki pipeline via `otelcol.exporter.loki`.
- `loki.source.kubernetes_events` — Kubernetes events, processed as log lines.

Every component in these pipelines is typed and schema-validated, including the ingress components and the `loki.write` sink.
The `raw:` escape (see [Authoring](../authoring/)) remains available and is currently unused.

### Processing conventions

These are the conventions a contributor must preserve when editing the gateway `loki.process` pipeline:

- **Level normalization.** A per-application `stage.match` extracts the level, then a series of `stage.replace` rules normalize it to one of `CRITICAL`, `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`. A heuristic regex backfills `UNKNOWN` levels, and the success/failure of that heuristic is recorded as structured metadata.
- **Drops and limits.** Lines older than the ingestion backlog window or larger than the per-line ceiling are dropped; per-level rate limits keep `INFO`/unknown chatter bounded while letting `ERROR`/`CRITICAL` through.
- **Label families.** Only a small, stable set is promoted to **Loki labels**: `level`, `app`, `container`, `namespace`, `component`, `job` and `service_name`, plus `unit` on node journal logs (which carry no `namespace`) and `environment_id` for environment namespaces. Everything else identifying — `pod`, `node`, `pod_id`, `container_id`, `region`, `zone`, `nodepool`, `trace_id`, `span_id`, `error`, `msg`, … — is routed to **structured metadata** so it stays queryable without inflating stream cardinality. The `k8s_`-prefixed aliases (`k8s_namespace`, `k8s_app`, `k8s_container`, `k8s_pod`) were **removed**: the first three duplicated the unprefixed labels exactly, and `k8s_pod` was the only stream label a pod name ever had — a pod name is unbounded and changes on every restart, so it belonged in structured metadata from the start. Asserted by `loki::gateway_labels` in the e2e suite, which fails if any of them returns.
- **Timestamps.** Parsed from the source line (`ts`/`timestamp`) as `RFC3339`/`RFC3339Nano` where the application provides one.

> [!WARNING]
>   The label-vs-structured-metadata split is the dominant cost-and-stability lever.
>   Adding a new Loki label multiplies [cardinality](../../../../o11y-glossary/#observability-foundations) — default to structured metadata and promote to a label only when it is low-cardinality and used as a selector.

### Multi-line records

A Rust panic is written to stderr as a single buffer and split back into one log line per newline by the container runtime.
Read line by line it arrives as roughly twenty unrelated entries.
The header naming the thread and the source location is then indistinguishable from the backtrace around it.
`stage.multiline` reassembles it ahead of every other stage.

The stage is configured by `firstline`, a regular expression matching the **start** of a record.
A line that matches begins a new entry, and a line that does not is appended to the entry in progress.
`firstline` must therefore match every ordinary line of the stream it is applied to.
The continuation lines of a panic are exactly the lines it must not match.

Two log formats appear on the Materialize services, and one expression covers both.

| Stream | An ordinary line starts with | Matched by |
|---|---|---|
| `environmentd`, `clusterd`, `balancerd` | `{`, being tracing JSON | `^\{` |
| `materialize-operator` | an RFC 3339 timestamp, being tracing's plain format | `^\d{4}-\d{2}-\d{2}T` |
| a panic on any of them | an RFC 3339 timestamp, then `thread '…' panicked at` | `^\d{4}-\d{2}-\d{2}T` |

The stage is scoped by container rather than by namespace, because `firstline` encodes a log format.
A stream whose format never matches `firstline` is unaffected, because entries pass through one at a time until the first match.
A stream whose format matches only intermittently is the case that does damage.
Every line between two matches is absorbed into the earlier one.

Three ordering constraints hold the placement in the pipeline.

| Constraint | Reason |
|---|---|
| Merge ahead of `stage.limit` | The limiter drops entries. A limiter that sheds continuation lines leaves a backtrace with holes in it, which reads as complete and is not |
| Merge ahead of the per-level rate limits | A merged panic costs the limiter one entry rather than twenty, so a crash stops competing with ordinary logs for the same budget |
| Keep `longer_than` after the merge | The ceiling then applies to the merged entry. That is what discards the separate pathological panic that emits multi-MiB lines, which this pipeline does not carry |

`max_wait_time` is the mechanism that delivers a panic rather than a timeout that rarely fires.
While a process is alive, each new `firstline` match flushes its predecessor immediately.
A panic is the last thing a process writes, so nothing follows it to push it out and the block is held until `max_wait_time` elapses.
The agent goes on tailing the log file after the container exits, so the wait is safe.

Merging runs on the gateway rather than on the agent because `firstline` depends on the log format, and the agent is deliberately format-agnostic.
The cost is that the gateway runs two or more replicas behind a service.
A panic whose lines straddle two of the agent's write batches can reach two replicas and remain two entries.
The failure mode is a backtrace split in two rather than one lost.

A merged panic is then classified.
`level` is set to `CRITICAL`, the value the normalization rules already resolve `fatal` and `crit` to.
That level is exempt from both per-level rate limits.
`msg` is set to the source location and the panic message, and `panic_thread` and `panic_location` are recorded as structured metadata.
Classification runs after the per-application blocks, because those set `msg` from JSON that a panic does not contain.
The regular expression is anchored at the start of the entry, so a log line that merely quotes a panic is not mistaken for one.

### Destinations

`inputProcessor` does not forward to a sink directly. It forwards to `loki.process.egress.receiver` — a **type-neutral passthrough seam** — plus the local debug tap. The seam and the actual sink live in `gateway-dest-stub.yaml`, split out so the destination can be swapped at chart-assembly time without editing the processing pipeline. Because `gateway.alloy` references a component it does not define, the two files are validated **jointly** (`make pipelines` concatenates them and runs `alloy validate`, the way alloy loads a config directory).

- **Logs → Loki.** The default stub wires `loki.process "egress"` → `loki.write "destination"` to the bundled Loki distributor; the endpoint is configurable via `GATEWAY_LOKI_DEST` (falling back to an in-cluster default). Auth (`basic_auth`) is deferred. In the [remote-only topology](../../../../logs-and-events/architecture/#alternative-topologies), point `GATEWAY_LOKI_DEST` at an external OTLP/Loki destination.
- **Swapping the destination.** A deployment renders its own egress tail — keeping the `loki.process "egress"` label as the contract — and points its `forward_to` at any `loki.LogsReceiver`: a different `loki.write`, an `otelcol.receiver.loki.<label>.receiver` bridge, or a fan-out to several sinks. The target must be a real component reference; it cannot be a runtime env string (`forward_to` is a capsule, so alloy rejects a string at load).
- **Recording-rule metrics → long-term metric store.** The [Loki Ruler](../../../../logs-and-events/architecture/#ruler) remote-writes recording-rule samples back through the gateway, which forwards them to Thanos via `prometheus.remote_write` alongside the metrics pipeline (see [Metrics](../metrics/)). *(Design target — this leg is not yet wired in `gateway.yaml`.)*

Tunable inputs: `ALLOY_LOKI_PORT` (default `3100`), `GATEWAY_LOKI_DEST` (default in-cluster Loki push URL).

## Attribution and adoption status

Pipeline behavior is **change-tracked via pull requests** against the sources above; cite the implementing PR when you change a stage, a label family, or an endpoint, rather than only editing prose here.
Per-component history is captured in the repo `CHANGELOG.md` and the [Releasing](../../releasing/) flow.

Current status (see the [Roadmap](../../roadmap/)):

- **Agent pipeline** — adopted in `packages/alloy-pipelines/agent.yaml`.
- **Gateway pipeline** — adopted in `packages/alloy-pipelines/gateway.yaml` (processing) + `packages/alloy-pipelines/gateway-dest-stub.yaml` (default egress tail), ported from `packages/ref-alloy-pipelines/staging-gateway.alloy` (the `sample_processor` debug-sampling variant was intentionally not ported). The `inputProcessor` block renders line-for-line against the reference, apart from the multi-line handling above, which the reference does not have. Still deferred: `loki.write` auth, and the recording-rule remote-write leg.

## See more

- [Authoring](../authoring/) — schema model and how to extend it.
- [Metrics](../metrics/) — the metrics-side pipeline conventions.
- [Logs & Events](../../../../logs-and-events/architecture/) — the customer-facing architecture this pipeline feeds.
