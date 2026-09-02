# Overview




# Collecting Metrics

There are four ways metrics reach `materialize-monitoring`, and most deployments use more than one.

| Path | Use it when | Direction |
|---|---|---|
| [Prometheus Operator](../prometheus-operator/) | An application in the cluster ships — or can be given — a `ServiceMonitor` or `PodMonitor` | The gateway **pulls** |
| [Prometheus Remote Write](../prometheus-remote-write/) | You already run Prometheus and want it to ship into this stack | Something **pushes** |
| [OpenTelemetry Endpoint](../opentelemetry-endpoint/) | You run an OpenTelemetry Collector, or an application that emits OTLP directly | Something **pushes** |
| [Prometheus Scraper](../prometheus-scraper/) | You run your own Prometheus and this stack is not in the path at all | Your Prometheus **pulls** |

The first three land in the same place: the `alloy-gateway` pipeline, which processes and enriches everything uniformly before writing it to [storage](../../storing/).
How it got in makes no difference downstream.

## The gateway's ingress ports

Three listeners, on the `alloy-gateway` Service:

| Port | Component | Accepts |
|---|---|---|
| `4317` | `otelcol.receiver.otlp` | OTLP over gRPC — **metrics and logs** |
| `4318` | `otelcol.receiver.otlp` | OTLP over HTTP — **metrics and logs** |
| `9090` | `prometheus.receive_http` | Prometheus remote-write |

(A fourth, `3100`, takes Loki-format log pushes — see [Logs & Events > Collecting](../../../logs-and-events/collecting/).)

> [!WARNING]
>   **None of these ports authenticate their callers**, and they are reachable from the whole cluster by default, because usually something in it legitimately pushes telemetry.
>   NetworkPolicy is the only access control here. Narrow the ingest ports if nothing in your cluster needs them — see [Securing](../../../operating/securing/#the-cluster--the-stack).

## Serving TLS on them

The listeners are split across the two pipeline trees, and **securing one does not secure the other**:

| Listener | Governed by |
|---|---|
| `loki.source.api` (`3100`), `otelcol.receiver.otlp` (`4317`/`4318`) | `pipeline.logging.gateway.server.tls` |
| `prometheus.receive_http` (`9090`) | `pipeline.metrics.gateway.server.tls` |

The OTLP listeners carry metrics as well as logs but are governed by the *logging* tree, because the listener belongs to that tree even when the payload does not.
Set both if you want every ingress port on TLS.
See [Securing](../../../operating/securing/#certificates).

