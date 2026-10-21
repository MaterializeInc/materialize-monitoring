---
title: "Node metrics (node-exporter)"
weight: 4
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Node metrics (node-exporter)

[`node-exporter`](https://github.com/prometheus/node_exporter) is a DaemonSet that reads the host kernel and exposes
what no other collector in this stack sees: swap, memory pressure, node-level network counters, disk I/O, conntrack, and clock sync.
The Alloy gateway scrapes it through the ServiceMonitor the subchart ships.
For where it sits in the stack, see [Architecture](../../../architecture/#node-exporter-node-metrics).
What to do about it is in the [production checklist](../checklist/).

It's a **separate workload on purpose**, and not folded into the Alloy agent even though Alloy's `prometheus.exporter.unix` is the same program.
Keeping them apart keeps their resource limits apart.
Sharing one envelope means a metrics regression starves log collection, which is the signal you most need during the incident it caused.
Separate DaemonSets also have independent eviction and can be excluded from specific node pools.
The trade accepted in exchange is one more per-node workload.

## Coverage {#coverage}

Every other node-exporter setting is a tuning decision.
Coverage is a correctness one, because the failure is silent.
A node the DaemonSet never lands on produces no error anywhere: it has no metrics, and no dashboard shows a hole where a node should be.

The chart keeps the upstream toleration (`{effect: NoSchedule, operator: Exists}`), which tolerates **every**
`NoSchedule` taint, including Materialize's `node.materialize.com/daemonsets-not-scheduled`.
That's deliberate for a node-metrics DaemonSet.
It's also why `node-exporter` is left out of the Terraform module's scheduling fan-out: writing `var.tolerations` into
it would *replace* that list rather than extend it, because Helm merges maps but overwrites lists.

`NoExecute` taints aren't tolerated by default.
That's usually right, because a node being drained for a problem should shed the DaemonSet too.
If you use `NoExecute` for routine pool separation, though, add a toleration or lose those nodes.

## Collectors and cardinality {#collectors}

The chart passes `--collector.disable-defaults` and names each collector it wants, rather than taking the upstream defaults and subtracting.
The reason is drift.
The default set changes with each node_exporter release, and an allowlist means a new default-on collector can't
silently join your cardinality budget on a Renovate bump.
The full list, with the reasoning behind every inclusion *and* every exclusion, is in the [values
reference](../../../reference/helm/materialize-monitoring-values/) under Node Exporter.

**Cardinality scales with vCPU count, not with pod count.**
The per-CPU collectors (`cpu`, `cpufreq`, `schedstat`) run roughly 20–25 series per vCPU and dominate everything else on large instances.
A 96-vCPU node is about 2k series from those alone, before disks and NICs.
That's the opposite shape from cAdvisor and kube-state-metrics, which scale with what's *running* on the node.
Sizing Thanos for node metrics therefore keys off your instance shapes and node count, and is unaffected by workload density.

The following exclusions are the difference between a bounded series count and an unbounded one:

- **Pod veths.**
  On a Cilium node every pod adds an `lxc*` interface to the host network namespace.
  Left unfiltered, `netdev` and `netclass` emit about 10 series each *and churn the interface names on every pod
  restart*, which costs more in TSDB churn than in raw series.
  The chart excludes the per-pod veths and keeps the `cilium_*` devices, which have stable names and carry real signal.
  The per-pod network view comes from cAdvisor.
- **Kubelet per-pod mounts.**
  Every emptyDir, secret, configMap, and projected token is its own mount under `/var/lib/kubelet/pods/<uid>/`, with the same churn problem.
  The chart excludes those and keeps `/var/lib/kubelet` itself, which is the ephemeral-storage filesystem and a real and common node failure.

Swap is covered on both axes, because Materialize swaps deliberately and the level alone doesn't distinguish healthy swapping from thrash.
`swap` and `meminfo` give the level, `vmstat` gives the rate (`pswpin` and `pswpout`), and `pressure` (PSI) is the leading indicator.
The `vmstat` field filter is widened past upstream's default to admit `pgscan_*` and `pgsteal_*`.
Those separate background reclaim (`kswapd`, fine) from direct reclaim (`direct`, an allocating thread is stalled).

Measure your own footprint before sizing:

```promql
# series per node, and the fleet total
count by (instance) ({job="node-exporter"})
count({job="node-exporter"})
```

## Sizing {#sizing}

The chart sets a 64Mi memory request **and** limit, a 10m CPU request, and **no CPU limit**.

There's no CPU limit because this DaemonSet idles between scrapes and then bursts for the length of one collection.
A CPU limit turns that burst into CFS throttling, which surfaces as scrape timeouts on exactly the loaded nodes whose samples matter most.
The 10m request is the average, not the cost of a scrape.

Memory is request equal to limit for the opposite reason.
The working set is flat and bounded, so a limit equal to the request costs nothing and makes the per-node footprint a
number the cluster autoscaler can plan with.
That lands the pod in **Burstable** QoS.
Guaranteed would require the CPU limit, and being evicted slightly earlier under node pressure is the better trade, with
`monitoring-critical` covering the ordering.

Rollouts use `updateStrategy.rollingUpdate.maxUnavailable: 10%` rather than upstream's `1`, so an image bump doesn't
walk a large fleet one node at a time.
Metrics are gap-tolerant and the pod restarts in seconds.

## Exposure {#exposure}

**A NetworkPolicy isn't what keeps port 9100 private.**
The pods run with `hostNetwork: true`, which the network collectors require.
`netdev`, `netclass`, `netstat`, `sockstat`, and `conntrack` all read namespaced files under `/proc/net`, and in a pod
network namespace they'd report the pod's own traffic rather than the node's.

Most CNIs, **Cilium and Calico among them, don't apply pod NetworkPolicy to host-networked pods**, because that traffic
belongs to the node identity rather than the pod's.
The chart ships a policy anyway: ingress from the Alloy gateway on 9100, and egress denied outright, because
node_exporter never initiates a connection.
It's enforced where the CNI supports it and declares intent everywhere else.
The node firewall or security group is the actual control, so don't treat the endpoint as private because a NetworkPolicy exists.

`kube-rbac-proxy` is **off**.
It would authenticate scrapes through TokenReview and SubjectAccessReview over HTTPS.
It's also a second container on *every* node, with a request comparable to node_exporter's, which roughly doubles the
per-node cost of node metrics to protect an endpoint that exposes no secrets.

TLS client authentication is available and deliberately parked.
`kubeRBACProxy.tls.tlsClientAuth` plus `tlsSecret` restricts `/metrics` to a scraper holding a certificate signed by the configured CA.
The chart's [certificates and mTLS profiles](../../securing/#gaps) don't cover node-exporter, so turning it on means
issuing and rotating that certificate yourself, at the per-node cost described earlier.

## Scraping {#scraping}

The ServiceMonitor is how collection happens; without it the DaemonSet runs and is never read.

The Service carries `prometheus.io/scrape: "false"`, overriding the subchart's `"true"`.
With generic annotation-based discovery, a `"true"` here would scrape every target twice under two different `job` labels.
That double-counts silently in any `sum()` or `rate()` over them.
<!-- TODO: revisit this paragraph when generic annotation-based scrape discovery ships (DEP-193). -->

For the same reason, if your cluster already runs a node-exporter, turn this one off rather than running both.
Set `node-exporter.enabled: false`, which is the circuit breaker.
The tag doesn't work, because tags are OR'd and `tags.node-exporter: false` loses to `tags.default: true`.
On the Terraform path, set `install_node_exporter = false`.

## See also

- [Architecture](../../../architecture/#node-exporter-node-metrics): where node-exporter sits in the stack.
- [Values reference](../../../reference/helm/materialize-monitoring-values/): the collector allowlist and the reasoning
  behind each choice, under Node Exporter.
- [node_exporter collectors](https://github.com/prometheus/node_exporter#collectors) (official): every collector, its platform, and its default state.
