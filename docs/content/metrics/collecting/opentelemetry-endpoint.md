---
title: "OpenTelemetry Endpoint"
weight: 20
---

# OpenTelemetry Endpoint

The gateway accepts OTLP directly, so an OpenTelemetry Collector — or any application that emits OTLP — can send telemetry into this stack without a Prometheus in the path.

An [`otelcol.receiver.otlp`](https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.receiver.otlp/) component listens on two ports of the `alloy-gateway` Service:

| Endpoint | Protocol |
|---|---|
| `alloy-gateway.<namespace>.svc:4317` | OTLP over gRPC |
| `alloy-gateway.<namespace>.svc:4318` | OTLP over HTTP |

**Both metrics and logs are accepted on both ports.** The receiver routes them onward by signal: metrics into the metrics processing pipeline, logs across a bridge into the logging pipeline. A collector sending both needs one exporter, not two.

Traces are not accepted — this stack does not carry them.

## Sending from an OpenTelemetry Collector

```yaml
exporters:
  otlp:
    endpoint: alloy-gateway.monitoring.svc:4317
    tls:
      insecure: true # see Securing, below

service:
  pipelines:
    metrics:
      exporters: [otlp]
    logs:
      exporters: [otlp]
```

For the HTTP port use the `otlphttp` exporter with `endpoint: http://alloy-gateway.monitoring.svc:4318`.

## What happens after it arrives

OTLP metrics join the gateway's own collection at the processing choke point, so they are treated identically to scraped series from there on: enrichment, the [importance tiers](../../storing/#the-importance-tiers) that decide which destinations each series reaches, and the egress filter.

This is worth knowing in both directions — the same gateway that *receives* OTLP can also *forward* over OTLP to an external backend. Ingesting from a collector and fanning out to [Honeycomb](../../storing/#otlp), [Google Cloud Monitoring](../../storing/#gcm), or [Datadog](../../storing/#datadog) are independent choices, and a gateway can do both at once.

## Securing it

The listeners do not authenticate callers.
Access control is [NetworkPolicy](../../../operating/securing/#the-cluster--the-stack).

TLS on both OTLP ports is governed by `pipeline.logging.gateway.server.tls` — the **logging** tree, even though these ports also carry metrics, because the listener belongs to that tree regardless of payload.
See [the ingress overview](../overview/#serving-tls-on-them).

## See also

- [Overview](../overview/) — the four collection paths, and the ports they use.
- [Prometheus Remote Write](../prometheus-remote-write/) — the same idea over the Prometheus protocol.
- [Storing](../../storing/#other-metric-storage-backends) — forwarding *out* over OTLP.
