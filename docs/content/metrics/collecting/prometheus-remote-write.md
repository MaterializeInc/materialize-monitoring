---
title: "Prometheus Remote Write"
weight: 10
---

# Prometheus Remote Write

If you already run Prometheus, the simplest way to get its metrics into this stack is to have it remote-write to the gateway.
Nothing needs to change about how that Prometheus discovers or scrapes its targets.

The gateway runs a [`prometheus.receive_http`](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.receive_http/) listener on port **9090**, serving the standard remote-write path:

```
http://alloy-gateway.<namespace>.svc:9090/api/v1/metrics/write
```

## Configuring the sender

In `prometheus.yml`:

```yaml
remote_write:
  - url: http://alloy-gateway.monitoring.svc:9090/api/v1/metrics/write
```

Any remote-write client works — a Prometheus server, Prometheus Agent, Grafana Alloy, or an OpenTelemetry Collector with the `prometheusremotewrite` exporter.

## What happens after it arrives

Received series are converted to OTLP by an internal bridge and join everything the gateway collects itself, so they get the same processing: enrichment, the [importance tiers](../../storing/#the-importance-tiers) that decide which destinations each series reaches, and the egress filter.

That means remote-written metrics are subject to the same [denylist](../../storing/#the-denylist) and tiering as scraped ones.
If a series you push is not appearing at a destination, check those before the transport.

## Securing it

The listener does not authenticate callers.
Access control is [NetworkPolicy](../../../operating/securing/#the-cluster--the-stack), and TLS on this port is governed by `pipeline.metrics.gateway.server.tls` — a **different** value from the OTLP and Loki listeners. See [the ingress overview](../overview/#serving-tls-on-them).

## See also

- [Overview](../overview/) — the four collection paths, and the ports they use.
- [Storing](../../storing/) — where the metrics go once they are in.
- [OpenTelemetry Endpoint](../opentelemetry-endpoint/) — the OTLP equivalent of this page.
