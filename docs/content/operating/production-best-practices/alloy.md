---
title: "Collection (Alloy)"
weight: 3
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Collection (Alloy)

The Alloy tier collects and processes telemetry before it reaches a backend.
It runs in two roles: the [`alloy-agent`](../../../logs-and-events/architecture/#alloy-agent) DaemonSet, one per node,
and the [`alloy-gateway`](../../../logs-and-events/architecture/#alloy-gateway) Deployment.
Both are configured as code; for more information, see the [logging pipeline reference](../../../reference/development/pipelines/logging/).
The gateway holds the dominant cost and stability lever, so most of the care goes there.

This page explains how the tier behaves.
What to do about it is in the [production checklist](../checklist/).

## Restart after a config change {#restart-after-a-config-change}

**Alloy config changes don't roll the pods.**
Everywhere else in this chart, changing a value changes a pod template and Kubernetes rolls it.
Alloy is the exception, and a `helm upgrade` that reports success can leave both roles serving the *previous* configuration indefinitely.

The bundled Alloy subchart stamps a `checksum/config` annotation only when it creates the config ConfigMap itself.
This chart renders the pipeline ConfigMaps in the umbrella (`mzmon-alloy-{agent,gateway}` and their `-env` pair) and
points the subchart at them, so that guard never fires.
The parent chart can compute the correct hash, because it can read all of `.Values` and `.Files`.
A subchart value is static YAML, though, so there is nowhere to put the hash that reaches the pod template.
**Only a consumer can close this**, because it holds the values before Helm renders them.

It must be a **restart**, not a reload.
The `-env` ConfigMaps are consumed with `envFrom`, and environment variables are fixed at container start.
A metric-filter change (`minMetricImportance`) is therefore invisible to both Alloy's `/-/reload` endpoint and the config-reloader sidecar.
Enabling either would silently no-op on that half of the config surface, which is worse than doing nothing.

If you install with the [Terraform module](../../../getting-started/terraform/), you don't need to act.
It stamps `mzmon.materialize.cloud/values-hash` onto both pod templates, so a config change rolls them and an unchanged apply doesn't.
If you install with Helm directly, restart both roles yourself after any pipeline or filter change:

```bash
kubectl -n monitoring rollout restart deployment/alloy-gateway daemonset/alloy-agent
```

Make every pipeline change through the YAML sources under `packages/alloy-pipelines/`.
A hand edit to the rendered `.alloy` in a running deployment is untracked, and the next render overwrites it.

## Cardinality and rate control {#cardinality}

The gateway promotes only a small, stable label set to Loki labels: `level`, `app`, `container`, `namespace`, and `environment_id`.
Everything else that identifies a line goes to **structured metadata**.
Each added Loki label multiplies [stream cardinality](../../../o11y-glossary/#observability-foundations), so default to structured metadata.

Rate limits apply at three points:

- **Per node**, in the agent: `AGENT_POD_LOG_RATE_LIMIT` and `AGENT_POD_LOG_BURST`, so one noisy node can't starve the pipeline.
- **Per level**: `INFO` and unknown-level chatter stays bounded while `ERROR` and `CRITICAL` pass.
- **A final safety limit** behind both.

The pipeline also drops stale lines (`older_than`) and cuts oversized lines to fit Loki (`stage.truncate`).

Understand the backpressure semantics before you tune any of these.
`stage.limit` with `drop = false` **queues**, which backpressures the sender.
With `drop = true` it **sheds** load.
The final safety limit sheds; the per-node and per-level limits are the tuning surface.

## Gateway autoscaling {#gateway-autoscaling}

The gateway scales horizontally **on CPU only**, from 2 to 8 replicas at a 50% target.
Memory is deliberately not an HPA metric, and it's worth understanding why before you add it back.

Memory utilization doesn't say whether a gateway is short of memory, because the Go runtime lets the heap grow toward `GOMEMLIMIT` either way.
Measured on a 7-node cluster, going from 3 replicas to 6 moved per-pod memory from 370Mi to 341Mi while **total**
consumption went from 1.1Gi to 2.0Gi.
With idle at about 62% of the request against a 60% target, the HPA also couldn't stabilize.
The action didn't move the metric, so it flapped against `maxReplicas` indefinitely, and no target value fixes that.

**Scale-down is slow**: it happens only after an hour of low CPU, and one pod per ten minutes.
Every scale event reshards the scrape targets.
A pod keeps the series of the targets it gave up until its write-ahead log checkpoints, so a gateway that flaps holds both shares at once.
Scale-up stays fast.

## Gateway memory {#gateway-memory}

Size gateway memory vertically, from the series it holds.
Raise `alloy-gateway.alloy.resources`, and `GOMEMLIMIT` and the memory limiter follow: `AUTOMEMLIMIT` sets `GOMEMLIMIT` to 80% of the limit.

A pod's live heap is about 200MiB plus about 2.6KiB per series.
At `minReplicas`, each pod holds its share of every series in the cluster.
Keep that under about half of `GOMEMLIMIT`.
Past it, the garbage collector runs constantly, and the CPU it spends makes the autoscaler scale out for no other reason.
The 2Gi default fits a medium install of about a quarter of a million series.

An explicit `GOMEMLIMIT` in `extraEnv` overrides the derived one and isn't kept in step with the limit.

## Agent placement {#agent-placement}

The agent DaemonSet tolerates every `NoSchedule` taint (`{effect: NoSchedule, operator: Exists}`), so it reaches
tainted, spot, and system pools without configuration.
Enumerating taint keys instead would make coverage depend on the chart knowing your taints.
A pool it hasn't heard of is a silent per-node blind spot.
A *bootstrap gate*, which is a taint applied at boot and lifted after the DaemonSets are up, deadlocks outright, because
the taint waits on a pod that's waiting on the taint.

`NoExecute` taints aren't tolerated, which is usually right: a node draining for a problem should shed the agent.
`node.kubernetes.io/not-ready` and `node.kubernetes.io/unreachable` are the exceptions, and not ones you control.
The DaemonSet controller adds both to every DaemonSet pod regardless of what the chart sets.
To add others, use the Terraform module's `tolerations`, which appends rather than replaces.

## Delivery to Loki {#delivery}

The gateway's write endpoint is set with `GATEWAY_LOKI_DEST`, and its ingress port with `ALLOY_LOKI_PORT`.
To authenticate to a secured or remote destination, set `pipeline.logging.gateway.destination.loki.authType` to
`basicAuth`, `bearer`, `oauth2`, or `sigv4`, with the matching credentials block.

To send your own logs, target the gateway's OTLP ports, `:4317` for gRPC or `:4318` for HTTP.
To chain gateways, point the upstream writer at the downstream gateway's `:3100`.
For more information, see [Sending your own logs to the gateway](../../../logs-and-events/collecting/#sending-your-own-logs-to-the-gateway).
