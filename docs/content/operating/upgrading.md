---
title: "Upgrading materialize-monitoring"
weight: 40
---

# Upgrading materialize-monitoring

## Ingester rollouts: duration and deploy timeouts

Any change to the ingester pod spec — image, resources, or scaling the replica count — rolls the ingester StatefulSet.
That roll is **ordered and readiness-gated**: pods cycle **one at a time**, and the next is not touched until the previous re-joins the ring and reports Ready.
This is deliberate — with `replication_factor: 3`, one-at-a-time keeps at most one of three replicas down, preserving write quorum throughout the roll.

**Budget roughly one minute per ingester**, plus headroom for any new nodes.
Each pod pays: graceful ring-leave + best-effort flush (up to `terminationGracePeriodSeconds`, ~60s) → start → memberlist join → ring `ACTIVE` → readiness stabilization.
A 6-ingester fleet therefore takes ~5 minutes to roll, and it is **not** bounded by how fast new nodes appear.

That overruns tools that wait on the rollout with a short default:

| Tool | Default wait | Fix |
|---|---|---|
| `helm upgrade --wait` | 5m (`--timeout`) | `--timeout 15m` |
| Flux `HelmRelease` | 5m | raise `spec.timeout` |
| Pulumi `helm.v4.Chart` | resource await | extend `customTimeouts` |
| ArgoCD | async (no wait) | tolerant — shows `Progressing` until healthy |

A `--wait` timeout here means **"still rolling," not "failed"** — the rollout completes correctly; the client just stopped watching.

> [!WARNING]
>   Do not speed the roll by allowing more than one ingester down at a time (`updateStrategy.rollingUpdate.maxUnavailable > 1`, via the alpha `MaxUnavailableStatefulSet` gate).
>   With RF 3 that can drop you to a single healthy replica and break write quorum mid-roll.
>   `zoneAwareReplication` (roll a whole zone at once) is the only quorum-safe burst, at the cost of cross-AZ replication traffic — see the [logging architecture](../../logs-and-events/#loki-ingester).
