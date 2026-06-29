---
title: "Collecting"
weight: 10
---

# Collecting Logs & Events

Logs and Kubernetes events enter Loki through its [write path](../#write-path) after Alloy collects and processes them.
This page covers what the [agent](../#alloy-agent) and [gateway](../#alloy-gateway) do, how to send your own logs in, and the knobs you can tune.
See the [logging architecture](../) for how collection fits into the full pipeline.

## What the agent collects

The [`alloy-agent`](../#alloy-agent) runs as a DaemonSet — one pod per node — and gathers two things from each node:

- **Container logs** for the pods scheduled on that node. The agent discovers those pods, tails their log files, and attaches Kubernetes metadata (namespace, pod, container, node, and node attributes such as region, zone, and instance type).
- **The systemd journal** of the host, tagged with the node name and the originating unit.

The agent does only light work locally — metadata relabeling and a per-node rate limit — then forwards everything to the gateway over the Loki push API.
Collection therefore scales with the number of nodes, and no application needs a logging sidecar.

## What the gateway adds

The [`alloy-gateway`](../#alloy-gateway) is where the real processing happens.
In addition to the logs forwarded by the agents, it collects **Kubernetes events** from the API server and turns them into log lines.
It then normalizes log levels, routes high-cardinality fields into [structured metadata](../#storage), applies drop and rate-limit policies, and writes the result to Loki.

> [!INFO]
>   The gateway is the single place where label-family and [cardinality](../../o11y-glossary/#observability-foundations) decisions are made.
>   Keeping that logic in one component is what keeps the log store stable and cheap — see the [logging pipeline reference](../../reference/internal/pipelines/logging/) (internal) for the authoritative stage-by-stage definition.

## Sending your own logs to the gateway

The gateway accepts logs on two endpoints, so sources beyond the agent can feed the same processing pipeline (see [Ingestion interfaces](../#ingestion)):

| Protocol | Endpoint | Typical sender |
|---|---|---|
| Loki push API | `alloy-gateway.$namespace:3100` (`/loki/api/v1/push`) | a Loki-push client, or another `alloy-gateway` |
| OTLP | `alloy-gateway.$namespace:4317` (gRPC), `:4318` (HTTP) | an OpenTelemetry-instrumented application or forwarder |

Anything sent to these endpoints is normalized the same way as node logs, so log shape stays consistent regardless of source.
Replace `$namespace` with the namespace the gateway runs in.

### Chained gateways

Because both endpoints accept gateway-shaped traffic, one `alloy-gateway` can forward to another — for example, a per-cluster gateway forwarding to a central one, or the [remote-only topology](../#alternative-topologies) where the gateway ships logs to a destination outside the cluster.
Point the upstream gateway's writer at the downstream gateway's `:3100` (Loki push) or `:4317`/`:4318` (OTLP) endpoint.

## Tuning collection

- **Per-node rate limit.** The agent caps pod-log throughput per node to protect the pipeline from a single noisy node. The rate (lines/sec) and short-term burst are set with the `AGENT_POD_LOG_RATE_LIMIT` (default `5000`) and `AGENT_POD_LOG_BURST` (default `20000`) environment variables on the agent.
- **Cluster label.** Set `CLUSTER_NAME` in the agent's environment so every collected line carries a stable `cluster` value — important when several clusters write to the same log store.
- **Gateway-side policies.** Level normalization, per-level rate limits, drops, and the label-vs-structured-metadata split are all gateway-pipeline concerns. Change them through the pipeline sources rather than ad hoc, so the behavior stays attributable.

> [!WARNING]
>   What becomes a Loki **label** versus what stays in the body or [structured metadata](../#storage) is the single biggest cost-and-stability lever.
>   Keep volatile attributes (request IDs, trace IDs) out of labels.
>   See [Log stream](../../o11y-glossary/#logs-and-events) and the [logging pipeline reference](../../reference/internal/pipelines/logging/) (internal).

## See more

- [Logging Architecture](../) — collection in the context of the full pipeline.
- [Logging pipeline reference](../../reference/internal/pipelines/logging/) (internal) — the authoritative agent/gateway pipeline definition.
- [Storing](../storing/) — where collected logs go next.
- [Loki components](https://grafana.com/docs/loki/latest/get-started/components/) (official).
