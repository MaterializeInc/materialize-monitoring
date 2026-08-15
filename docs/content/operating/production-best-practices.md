---
title: "Production Best Practices"
weight: 10
---

# Production Best Practices

Production guidance for the `materialize-monitoring` stack, organized by backend.
Every checklist item is tagged with its **primary owner** under the [shared responsibility model](#shared-responsibility-model), and is checked (`[x]`) when the chart already ships it as a default — unchecked items are the deployment-time actions (or still-to-build chart work) that remain.

Today this covers the **collection tier (Alloy)**, **node metrics (node-exporter)**, the bundled **logging backend (Loki)**, the bundled **metrics backend (Thanos)**, and the bundled **Grafana**; an Alertmanager section will follow the same shape.

## Shared responsibility model

Four parties share responsibility for a production deployment.
Checklist items are tagged with the **primary** owner.

| Tag | Party | Owns |
|---|---|---|
| `[upstream]` | **Upstream service** (Loki, Alloy, Grafana) | component behavior: the ring, WAL, compaction, query engine, defaults |
| `[chart]` | **`materialize-monitoring` chart** | topology and wiring, opinionated defaults, config rendering, validation, shipped dashboards/alerts, the profiles |
| `[consumer]` | **Chart consumer** (Terraform / Pulumi / ArgoCD / FluxCD) | cloud resources (bucket, IAM/IRSA, StorageClass, DNS), secret provisioning, version pinning, selecting the profile |
| `[operator]` | **Human operator** (end-user) | size selection, retention budget, tenant policy, incident response, day-2 procedures, capacity watch |

| Area | Upstream | Chart | Consumer | Operator |
|---|---|---|---|---|
| Component logic (ring, WAL, compaction, query) | **owns** | configures | — | — |
| Topology, defaults, validation, dashboards/alerts | — | **owns** | selects profile | — |
| Cloud resources: bucket, IAM/IRSA, StorageClass | — | consumes by name | **owns** | approves |
| Secrets provisioning | — | consumes by name | **provides** | rotates |
| Rolling Alloy after a config change | — | cannot | **owns** | verifies |
| Size, retention budget, tenant policy | — | offers profiles | sets values | **decides** |
| Incident response, upgrades, DR, capacity | — | tooling + alerts | applies changes | **owns** |

### If you install with Terraform {#terraform-consumer}

The [Terraform modules](../../getting-started/terraform/) are a `[consumer]` implementation, so several items below are already done when you use them.
Those are marked **(Terraform: automatic)** — read them to understand what is happening, not as work to do.

Satisfied by the modules today:

| | |
|---|---|
| Object-storage buckets | One per backend, with versioning on |
| Workload identity | IRSA on AWS, Workload Identity on GCP — no static keys |
| Version pinning | The module and the chart are one release, so the module ref names the chart version |
| Retention posture | Bucket lifecycle rules default to off, leaving deletion to each compactor |
| Grafana admin secret | Generated and supplied by name |
| Alloy rollouts | A values hash on both pod templates, so a config change rolls Alloy and an unchanged apply does not |

Still yours, on any install path:

- A usable **StorageClass** must exist — four workloads are PVC-backed (Alertmanager, the Loki ruler, the Thanos Store Gateway and Compactor) and the modules do not create one. "Usable" is not the same as "present": on GCP's C4 and N4 machine families, which accept only Hyperdisk, *every* StorageClass GKE creates by default is Persistent Disk and none of them will attach. The Terraform modules take `storage_class`; see [Getting Started > Terraform](../../getting-started/terraform/#storageclass-on-gcp-c4-and-n4-node-pools).
- **Sizing and retention budgets**, node-pool capacity, and the profile choice.
- **Basic-auth or mTLS secrets** between components, which are not yet wired on any path.
- Everything tagged `[operator]`.

Azure has no wrapper module yet, so an Azure install is a plain `[consumer]` — every item below applies.

## Namespace layout {#namespace-layout}

Each subchart uses a deterministic name (a static `fullnameOverride`, e.g. `loki`, `thanos`, so no release-name prefix), which assumes **one instance of each backend per namespace** — a reasonable constraint for an umbrella infrastructure chart.
Two layouts are supported; both are fine, so pick per your isolation needs.

**Shared namespace (default).** Everything deploys into the release namespace (recommended: `monitoring`). This is the default and the path most installs should take.

**Split namespace.** One namespace per subchart, via the `split-namespace` profile (`-f charts/materialize-monitoring/profiles/split-namespace.values.yaml`), which sets a `namespaceOverride` per component (`loki`, `thanos`, `grafana`, …). Support is best-effort.

| | Shared (default) | Split |
|---|---|---|
| Ops overhead | low — one namespace, one RBAC scope, simpler NetworkPolicy and install | higher — N namespaces, cross-namespace NetworkPolicy/DNS, per-namespace bindings |
| Isolation / trust | components share a trust boundary; larger blast radius | least-privilege between components; contained blast radius |
| Per-component quotas / RBAC | coarse | fine-grained |
| Support | primary | best-effort |

> [!INFO]
>   The layout changes the namespace half of every workload-identity binding: an IRSA / GKE / Azure trust-policy subject targets `system:serviceaccount:<namespace>:<sa>`, where `<namespace>` is the release namespace under the shared layout and the per-component namespace under split. The ServiceAccount name is unaffected (it's the deterministic `fullnameOverride`). See [Logs &amp; Events &gt; Storing](../../logs-and-events/storing/#granting-object-storage-access-workload-identity) and [Metrics &gt; Storing](../../metrics/storing/#thanos-object-storage).

`[consumer]` selects the layout at install time; `[operator]` owns the namespace/RBAC policy it implies.

## Scheduling priority {#scheduling-priority}

The chart creates two PriorityClasses and assigns every long-running workload to one of them.
The split is about **what losing a pod costs you**, not about how important the component feels.

| Class | Value | Assigned to | Losing a pod means |
|---|---:|---|---|
| `monitoring-critical` | 1000000 | `alloy-agent`, `node-exporter`, `alloy-gateway` | a blind spot with no replica to cover it |
| `monitoring-scalable` | 1000 | `loki`, `thanos`, `grafana`, `grafana-operator`, `alertmanager`, `kube-state-metrics` | reduced capacity a surviving replica or a retry absorbs |

The per-node collectors are singletons *per node*.
When one is evicted, nothing else reports that node and the gap never backfills — there is no equivalent of a replica picking up the slack.
The backends are replicated, buffered, or both: Alloy retries its writes, so an evicted Loki ingester costs latency rather than data.
The gateway is graded critical despite being a Deployment, because it is the single egress choke point for every signal in the stack.

Both classes set **`preemptionPolicy: Never`**, which is the load-bearing choice.
Priority still decides scheduling-queue order and, more importantly, which pod the kubelet evicts first under node pressure — but monitoring will never evict a Materialize pod to make room for itself.
Monitoring that takes down the thing it monitors is worse than monitoring that is late.
Both sit well below `system-cluster-critical` (2000000000) and `system-node-critical` (2000001000), so cluster plumbing still outranks the stack.

- [x] `[chart]` Both classes created, and referenced by name from every subchart that accepts a `priorityClassName`.
- [ ] `[operator]` **PriorityClasses are cluster-scoped.** Two releases of this chart in one cluster will fight over these objects. Set `priorityClasses.create: false` on all but one, or rename them per release with `priorityClasses.critical.name` / `priorityClasses.scalable.name`.
- [ ] `[operator]` **If you rename or disable them, update the `priorityClassName` values in the subchart blocks to match.** A `priorityClassName` naming a class that does not exist does not degrade — the API server *rejects* the pod, and the only evidence is an admission error on a ReplicaSet nobody is watching. A render-time check warns when it can tell that has happened.
- [x] `[chart]` The `scheduling` profile carries the whole fan-out — node selector, tolerations, and both class names — with every site aliased off four anchors, so a rename is a two-line edit rather than eleven `priorityClassName` keys across nine subcharts that can half-apply. See [Getting Started > Helm](../../getting-started/helm/#scheduling-profile).
- [ ] `[operator]` If your platform already defines a priority scheme, point the subchart values at your own classes instead. `metrics-server` is left on the upstream `system-cluster-critical` deliberately: it backs the metrics API that HPAs and the Materialize Console read, so it is cluster plumbing rather than monitoring.

## Collection (Alloy)

The Alloy tier collects and processes telemetry before it reaches a backend.
It runs in two roles — the [`alloy-agent`](../../logs-and-events/architecture/#alloy-agent) DaemonSet (one per node) and the [`alloy-gateway`](../../logs-and-events/architecture/#alloy-gateway) Deployment — configured as code (see the [logging pipeline reference](../../reference/internal/pipelines/logging/) (internal)).
The gateway is where the dominant cost/stability lever lives, so most of the care goes there.

### Configuration & change management

- [x] `[chart]` Pipelines are **authored as code** (`packages/alloy-pipelines/*.yaml`) and rendered to `.alloy`; the rendered output is committed and CI asserts it matches a fresh render, so config drift is caught at review time.
- [x] `[chart]` `alloy validate` runs on every rendered pipeline in the build.
- [x] `[chart]` Pre-install/pre-upgrade `alloy validate` hook so a bad config fails the release rather than a running pod.
- [ ] `[operator]` Change pipeline behavior (stages, label families, endpoints) through the YAML sources, never by hand-editing rendered `.alloy` in a running deployment — edits there are lost on the next render and untracked.
- [ ] `[consumer]` **(Terraform: automatic)** **Restart Alloy after a config change.** This is the one place the chart cannot own its own rollout, and it is the reverse of what a chart normally guarantees — see below.

> [!WARNING]
>   **Alloy config changes do not roll the pods.** Everywhere else in this chart, changing a value changes a pod template and Kubernetes rolls it. Alloy is the exception, and a `helm upgrade` that reports success can leave both roles serving the *previous* configuration indefinitely.
>
>   The bundled alloy subchart stamps a `checksum/config` annotation only when it creates the config ConfigMap itself. This chart renders the pipeline ConfigMaps in the umbrella (`mzmon-alloy-{agent,gateway}` and their `-env` pair) and points the subchart at them, so that guard never fires. The parent chart can compute the correct hash — it can read all of `.Values` and `.Files` — but a subchart value is static YAML, so there is nowhere to put it that reaches the pod template. **Only a consumer can close this**, because it holds the values before Helm renders them.
>
>   It must be a **restart**, not a reload. The `-env` ConfigMaps are consumed with `envFrom`, and environment variables are fixed at container start — so a metric-filter change (`minMetricImportance`) is invisible to both Alloy's `/-/reload` endpoint and the config-reloader sidecar. Enabling either would silently no-op on that half of the config surface, which is worse than doing nothing.
>
>   Installing with the [Terraform module](../../getting-started/terraform/) needs no action: it stamps `mzmon.materialize.cloud/values-hash` onto both pod templates, so a config change rolls them and an unchanged apply does not. Installing with Helm directly, restart both roles yourself after any pipeline or filter change:
>
>   ```bash
>   kubectl -n monitoring rollout restart deployment/alloy-gateway daemonset/alloy-agent
>   ```

### Cardinality & rate control (the lever)

- [x] `[chart]` The gateway promotes only a small, stable label set (`level`, `app`, `container`, `namespace` + their `k8s_`-prefixed forms, `environment_id`) and routes everything else identifying to **structured metadata**. Adding a Loki label multiplies [stream cardinality](../../o11y-glossary/#observability-foundations) — default to structured metadata.
- [x] `[chart]` Per-level rate limits keep `INFO`/unknown chatter bounded while letting `ERROR`/`CRITICAL` through; oversized and stale lines are dropped (`longer_than`, `older_than`).
- [x] `[chart]` The agent applies a **per-node** pod-log rate cap (`AGENT_POD_LOG_RATE_LIMIT` / `AGENT_POD_LOG_BURST`) so one noisy node can't starve the pipeline.
- [ ] `[operator]` Understand the backpressure semantics before tuning: `stage.limit` with `drop = false` **queues** (backpressures the sender), `drop = true` **sheds** load. The final safety limit sheds; per-node and per-level limits are the tuning surface.

### Gateway availability & delivery

- [ ] `[operator]` Run the gateway with **≥2 replicas** behind its Service — it holds in-memory buffers, so a single replica is a delivery gap during restarts and node churn.
- [ ] `[operator]` Confirm `loki.write` durability settings (WAL + ret/backoff) survive gateway restarts to your RPO; the write endpoint is set with `GATEWAY_LOKI_DEST` and the ingress port with `ALLOY_LOKI_PORT`.
- [ ] `[consumer]` If sending OTLP, target `:4317` (gRPC) or `:4318` (HTTP); if chaining gateways, point the upstream writer at the downstream `:3100`. See [Collecting](../../logs-and-events/collecting/#sending-your-own-logs-to-the-gateway).
- [ ] `[operator]` `loki.write` auth to a secured/remote destination (`basic_auth`/headers) is **not yet wired** — provide it before shipping to a destination that requires it.
- [x] `[chart]` **Horizontal autoscaling on CPU only** (2–8 replicas, 50%). Memory is deliberately not an HPA metric here, and this is worth understanding before you add it back: the gateway's footprint is dominated by fixed per-process cost rather than per-replica load, so scaling out does not relieve memory. Measured on a 7-node cluster, going from 3 replicas to 6 moved per-pod memory 370Mi → 341Mi while **total** consumption went 1.1Gi → 2.0Gi — each replica added to shed memory brings a whole new baseline with it. With idle at ~62% of the request against a 60% target, the HPA also could not stabilize: the action did not move the metric, so it flapped against `maxReplicas` indefinitely. No target value fixes that.
- [ ] `[operator]` **Relieve gateway memory vertically**, not horizontally: raise `alloy-gateway.alloy.resources` and keep `GOMEMLIMIT` in step at ~80% of the limit. The gateway carries the kubelet cAdvisor scrape, so its heap grows with node count, and at `minReplicas` each pod carries the whole fan-out rather than a shard of it. `GOMEMLIMIT` above the container limit is inert — the kubelet OOM-kills the pod before the runtime ever collects hard.

### Agent placement & durability

- [x] `[chart]` Tolerates every `NoSchedule` taint (`{effect: NoSchedule, operator: Exists}`), so the DaemonSet reaches tainted, spot, and system pools without configuration. Enumerating keys instead would make coverage depend on the chart knowing your taints: a pool it has not heard of is a silent per-node blind spot, and a *bootstrap gate* — a taint applied at boot and lifted once the DaemonSets are up — deadlocks outright, because the taint waits on a pod that is waiting on the taint.
- [ ] `[operator]` `NoExecute` taints are **not** tolerated, which is usually right — a node draining for a problem should shed the agent. `node.kubernetes.io/not-ready` and `node.kubernetes.io/unreachable` are the exceptions, and not ones you control: the DaemonSet controller adds both to every DaemonSet pod regardless of what the chart sets. Add any others through the Terraform module's `tolerations`, which appends rather than replaces.
- [ ] `[operator]` **Verify coverage rather than assume it** after any node-pool change, the same way as for node-exporter: the agent's pod count should equal `count(kube_node_info)`. Narrowing the toleration list for a pool that cannot absorb the agent is a deliberate blind spot — record which pools those are.
- [x] `[chart]` `priorityClassName: monitoring-critical` on both roles. The agent is a per-node singleton, so an eviction is a log gap on that node with nothing to cover it; the gateway is the single egress choke point for every signal. See [Scheduling priority](#scheduling-priority).
- [ ] `[operator]` Persist the agent's file **positions** and journal cursor (hostPath) so a restart resumes where it left off instead of re-tailing (duplicate lines) or skipping (gaps).
- [ ] `[consumer]` Set `CLUSTER_NAME` on the agent so every line carries a stable `cluster` label when several clusters share a log store.

### Security & meta-monitoring

- [x] `[chart]` Distroless Alloy image: FIPS boringcrypto, multi-arch, non-root, GHCR-published.
- [x] `[chart]` ServiceMonitor/PodMonitor (or GCP `PodMonitoring`) for both Alloy roles — scrape Alloy's own component metrics (received/sent bytes, dropped lines, write failures).
- [ ] `[operator]` Alert on gateway write failures and drop counters (`drop_counter_reason`) so shedding or a broken destination is visible rather than silent data loss.

## Node metrics (node-exporter)

For where this sits in the stack, see [Architecture](../../architecture/#node-exporter-node-metrics).

[`node-exporter`](https://github.com/prometheus/node_exporter) is a DaemonSet that reads the host kernel and exposes what no other collector in this stack sees: swap, memory pressure, node-level network counters, disk I/O, conntrack, and clock sync.
The Alloy gateway scrapes it through the ServiceMonitor the subchart ships.

It is a **separate workload on purpose**, and not folded into the Alloy agent even though Alloy's `prometheus.exporter.unix` is the same program.
Keeping them apart keeps their resource limits apart: sharing one envelope means a metrics regression starves log collection, which is the signal you most need during the incident it caused.
Separate DaemonSets also have independent eviction and can be excluded from specific node pools.
The trade accepted in exchange is one more per-node workload.

### Coverage is the thing to get right

Everything else on this list is a tuning decision.
Coverage is a correctness one, because the failure is silent: a node the DaemonSet never lands on produces no error anywhere — it simply has no metrics, and no dashboard shows a hole where a node should be.

The chart keeps the upstream toleration (`{effect: NoSchedule, operator: Exists}`), which tolerates **every** `NoSchedule` taint, including Materialize's `node.materialize.com/daemonsets-not-scheduled`.
That is deliberate for a node-metrics DaemonSet, and it is why `node-exporter` is left out of the Terraform module's scheduling fan-out: writing `var.tolerations` into it would *replace* that list rather than extend it, since Helm merges maps but overwrites lists.

- [x] `[chart]` Tolerates all `NoSchedule` taints, so it reaches tainted, spot, and system pools without configuration.
- [ ] `[operator]` **Verify coverage rather than assume it**, after any node-pool change: `count(up{job="node-exporter"})` against `count(kube_node_info)`. They should be equal; a persistent gap is a pool the DaemonSet is not reaching.
- [ ] `[operator]` `NoExecute` taints are **not** tolerated by default. That is usually right — a node being drained for a problem should shed the DaemonSet too — but if you use `NoExecute` for routine pool separation, add a toleration or lose those nodes.
- [ ] `[operator]` Excluding it from a node pool is a deliberate blind spot, not a saving. Do it only for pools where the per-node budget genuinely cannot absorb ~64Mi, and record which pools those are.

### Collectors and cardinality

The chart passes `--collector.disable-defaults` and names each collector it wants, rather than taking the upstream defaults and subtracting.
The reason is drift: the default set changes with each node_exporter release, and an allowlist means a new default-on collector cannot silently join your cardinality budget on a Renovate bump.
The full list, with the reasoning behind every inclusion *and* every exclusion, is in the [values reference](../../reference/helm/materialize-monitoring-values/) under Node Exporter.

**Cardinality scales with vCPU count, not with pod count.**
The per-CPU collectors (`cpu`, `cpufreq`, `schedstat`) run roughly 20–25 series per vCPU and dominate everything else on large instances — a 96-vCPU node is ~2k series from those alone, before disks and NICs.
This is the opposite shape from cAdvisor and kube-state-metrics, which scale with what is *running* on the node.
Sizing Thanos for node metrics therefore keys off your instance shapes and node count, and is unaffected by workload density.

Two exclusions are worth knowing because they are the difference between a bounded series count and an unbounded one:

- **Pod veths.** On a Cilium node every pod adds an `lxc*` interface to the host network namespace. Left unfiltered, `netdev` and `netclass` emit ~10 series each *and churn the interface names on every pod restart* — which costs more in TSDB churn than in raw series. The chart excludes the per-pod veths and keeps the `cilium_*` devices, which have stable names and carry real signal. The per-pod network view comes from cAdvisor.
- **Kubelet per-pod mounts.** Every emptyDir, secret, configMap, and projected token is its own mount under `/var/lib/kubelet/pods/<uid>/`, with the same churn problem. The chart excludes those and keeps `/var/lib/kubelet` itself, which is the ephemeral-storage filesystem and a real and common node failure.

Measure your own footprint before sizing:

```promql
# series per node, and the fleet total
count by (instance) ({job="node-exporter"})
count({job="node-exporter"})
```

- [x] `[chart]` Allowlist rather than upstream defaults, so a node_exporter release cannot add collectors behind you.
- [x] `[chart]` Swap covered on both axes — `swap` and `meminfo` for the level, `vmstat` for the rate (`pswpin`/`pswpout`), and `pressure` (PSI) as the leading indicator. Materialize swaps deliberately, so the level alone does not distinguish healthy swapping from thrash.
- [x] `[chart]` The `vmstat` field filter is widened past upstream's default to admit `pgscan_*` / `pgsteal_*`, which separate background reclaim (`kswapd`, fine) from direct reclaim (`direct`, an allocating thread is stalled).
- [x] `[chart]` Per-pod veths and kubelet per-pod mounts excluded from the network and filesystem collectors.
- [ ] `[operator]` **On AWS, consider enabling `ethtool`** with `--collector.ethtool.device-include=^(eth|ens|enp)`. It is where the ENA allowance counters live (`bw_in_allowance_exceeded`, `bw_out_allowance_exceeded`, `pps_allowance_exceeded`, `conntrack_allowance_exceeded`) — instance-level network throttling that is invisible in every other metric and directly relevant to a network-hungry workload. It is off by default because the stat set is driver-specific (gVNIC and Azure expose different counters), so validate it on your AMI before relying on it.
- [ ] `[operator]` On bare metal, `edac` (ECC errors) and `rapl` are worth adding; hypervisors do not expose either to guests, which is why they are off.
- [ ] `[operator]` `slabinfo` is off: it costs ~1.5k series per node and needs a root init container to chmod `/proc/slabinfo`. `meminfo` already reports `Slab` / `SReclaimable` / `SUnreclaim`, which answers the alerting question. Turn `slabinfo` on during an investigation, not as a default.

### Sizing

- [x] `[chart]` 64Mi memory request **and** limit, 10m CPU request, **no CPU limit**.
- [ ] `[operator]` Understand why there is no CPU limit before adding one. This is a DaemonSet that idles between scrapes and then bursts for the length of one collection; a CPU limit turns that burst into CFS throttling, which surfaces as scrape timeouts on exactly the loaded nodes whose samples matter most. The 10m request is the average, not the cost of a scrape.
- [ ] `[operator]` Memory is request == limit for the opposite reason: the working set is flat and bounded, so a limit equal to the request costs nothing and makes the per-node footprint a number the cluster autoscaler can plan with. This lands the pod in **Burstable** QoS — Guaranteed would require the CPU limit above, and being evicted slightly earlier under node pressure is the better trade, with `monitoring-critical` covering the ordering.
- [x] `[chart]` `updateStrategy.rollingUpdate.maxUnavailable: 10%` rather than upstream's `1`, so an image bump does not walk a large fleet one node at a time. Metrics are gap-tolerant and the pod restarts in seconds.

### Exposure

> [!WARNING]
>   **A NetworkPolicy is not what keeps port 9100 private.** The pods run with `hostNetwork: true`, which the network collectors require — `netdev`, `netclass`, `netstat`, `sockstat`, and `conntrack` all read namespaced files under `/proc/net`, and in a pod network namespace they would report the pod's own traffic rather than the node's.
>
>   Most CNIs, **Cilium and Calico among them, do not apply pod NetworkPolicy to host-networked pods**, because that traffic belongs to the node identity rather than the pod's. The chart ships a policy anyway — it is enforced where the CNI supports it, and it declares intent everywhere else — but the node firewall or security group is the actual control. Do not treat the endpoint as private because a NetworkPolicy exists.

- [x] `[chart]` NetworkPolicy on by default: ingress restricted to the Alloy gateway on 9100, egress denied outright (node_exporter never initiates a connection).
- [ ] `[consumer]` Under the [split-namespace](#namespace-layout) layout the default ingress rule stops matching — it selects the gateway by pod label within the release namespace. Replace `node-exporter.networkPolicy.ingress` with a rule carrying a `namespaceSelector`.
- [ ] `[operator]` **Confirm port 9100 is not reachable from outside the cluster.** With `hostNetwork`, the exporter listens on the node's interfaces, so this is a security-group / firewall question, not a Kubernetes one.
- [x] `[chart]` `kube-rbac-proxy` is **off**. It would authenticate scrapes via TokenReview/SubjectAccessReview over HTTPS, but it is a second container on *every* node with a request comparable to node_exporter's — roughly doubling the per-node cost of node metrics to protect an endpoint that exposes no secrets. DaemonSet overhead is the constraint being managed.
- [ ] `[operator]` **TLS client authentication is available and deliberately parked.** `kubeRBACProxy.tls.tlsClientAuth` plus `tlsSecret` restricts `/metrics` to a scraper holding a certificate signed by the configured CA — the shape an in-cluster mTLS story would use. It is not enabled pending the cert-manager integration ([DEP-195](https://linear.app/materializeinc/issue/DEP-195)), which is where certificate issuance and renewal for the rest of the stack lands; wiring it here first would mean minting and rotating a CA by hand for one component. Turn it on yourself if your environment requires authenticated scrapes today, accepting the per-node cost above.
- [x] `[chart]` Distroless image (no shell, no package manager), non-root, read-only root filesystem, `automountServiceAccountToken: false`. The container reads the host's `/proc`, `/sys`, and `/`, which makes a shell there a materially better foothold than in most containers.
- [x] `[chart]` The image is pinned explicitly in `values.yaml` (registry, repository, tag) rather than tracking the subchart's `appVersion`, so Renovate bumps the exporter on its own cadence — a chart release is not a prerequisite for a node_exporter CVE fix.

### Scraping and meta-monitoring

- [x] `[chart]` ServiceMonitor enabled. This is how collection happens: without it the DaemonSet runs and is never read.
- [x] `[chart]` `prometheus.io/scrape: "false"` on the Service, overriding the subchart's `"true"`. Once generic annotation-based discovery lands ([DEP-193](https://linear.app/materializeinc/issue/DEP-193)), a `"true"` here would make every target scraped twice under two different `job` labels — which double-counts silently in any `sum()` or `rate()` over them.
- [ ] `[operator]` **If your cluster already runs a node-exporter**, turn ours off rather than running both, for the same double-counting reason. Set `node-exporter.enabled: false` — the circuit breaker, not the tag, because tags are OR'd and `tags.node-exporter: false` loses to `tags.default: true`. On the Terraform path, `install_node_exporter = false`.
- [ ] `[operator]` **Alert on collectors failing, not just on the exporter being up.** `node_scrape_collector_success == 0` means that collector returned nothing while the scrape as a whole succeeded, so `up` stays green and the metrics simply never appear. Some of these are expected and worth allowlisting rather than chasing: `hwmon` is usually empty on cloud VMs, `nvme` needs NVMe devices, and `kernel_hung` needs Linux 6.13+ for `/proc/sys/kernel/hung_task_detect_count`.
- [ ] `[operator]` `schedstat` has two kernel gates and fails quietly on the second. `/proc/schedstat` needs `CONFIG_SCHEDSTATS`, and since Linux 4.6 the counters are additionally off at runtime unless `kernel.sched_schedstats=1`. A missing file shows up in `node_scrape_collector_success`; the sysctl being off does not — the file just reads as zeros.

### See also

- [Architecture](../../architecture/#node-exporter-node-metrics) — where node-exporter sits in the stack.
- [Values reference](../../reference/helm/materialize-monitoring-values/) — the collector allowlist and the reasoning behind each choice, under Node Exporter.
- [node_exporter collectors](https://github.com/prometheus/node_exporter#collectors) (official) — every collector, its platform, and its default state.

## Logging (Loki)

For the architecture these items configure, see [Logs & Events](../../logs-and-events/architecture/).

### Sizing the logging backend

**Size by throughput and burst, not by stored-bucket size.**
Bucket size is a *derived* output (sustained throughput × retention), not an input.
Different parts of Loki are sized off different points of the load envelope:

- **Ingest path** (distributors, ingesters, WAL, ingestion limits) → the **5-minute burst**, with headroom to the regression ceiling.
- **Storage / retention / bucket** → **sustained** throughput.
- **Read path** (frontend, queriers, caches) → query load, independent of ingest.

The 5-minute burst typically runs several times the peak-hour rate, so averages must not drive sizing.
The three tiers are defined by the ingest envelope:

| Size | Sustained (peak-hour) | 5-min burst | Regression ceiling | Typical fit |
|---|---|---|---|---|
| **S** | ~0.25 MiB/s | ~1 MiB/s | ~2 MiB/s | dev / staging |
| **M** | ~0.75 MiB/s | ~3 MiB/s | ~8 MiB/s | mid-size |
| **L** | ~2 MiB/s | ~6 MiB/s | ~17 MiB/s | production SaaS / fleet |

> [!INFO]
>   The **regression ceiling** is the burst you must degrade gracefully against — typically the volume *before* Alloy's cardinality/series reduction is applied.
>   Size the ingest path so a reduction regression throttles via limits rather than crashing the fleet.

#### Measure to validate

Size and re-measure off the distributor — the same signal sizing is derived from. `[operator]`

```promql
# sustained (size storage + retention to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[1h]))[7d:1h] )

# 5-min burst (size ingest path + WAL + limits to this)
max_over_time( sum(rate(loki_distributor_bytes_received_total[5m]))[7d:5m] )
```

If the 5-minute figure climbs toward the regression ceiling, that is the signal to add ingesters (see [§4](#4-ingester-durability--rollouts)) before it bites.

#### Per-size resources

Starting points — `replicas × (cpu request / memory request)`; tune from real usage. `[chart]` defaults, `[operator]`/`[consumer]` override per profile.

| Component | S | M | L |
|---|---|---|---|
| Distributor (stateless) | 2 × (100m / 128Mi) | 2 × (150m / 256Mi) | 3 × (500m / 512Mi) |
| **Ingester** (RF 3, ephemeral) | 3 × (250m / 512Mi) | 3 × (500m / 1Gi) | 3–6 × (1–2 / 4–8Gi) |
| Querier (stateless) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) | 3 × (1 / 1–2Gi) |
| Query-frontend | 2 × (100m / 128Mi) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) |
| Query-scheduler | omit | 2 × (100m / 256Mi) | 2 × (100m / 256Mi) |
| Index-gateway | 1 × (100m / 256Mi) | 2 × (200m / 512Mi) | 2 × (500m / 1Gi), ring mode |
| Compactor (singleton) | 1 × (100m / 256Mi) | 1 × (250m / 512Mi) | 1 × (1 / 2Gi) |
| Ruler (if log rules) | 2 × (100m / 256Mi) | 2 × (100m / 256Mi) | 2 × (250m / 512Mi) |
| memcached-chunks | 1 × 256Mi | 2 × 512Mi | 2–3 × 2Gi |
| memcached-results | 1 × 128Mi | 1 × 256Mi | 2 × 512Mi |
| memcached-index | share results | 1 × 256Mi | 2 × 1Gi |

- Ingesters = **3 minimum** at every size (the `replication_factor` 3 floor), and the **compactor is always a singleton**.
- **Scale ingesters past 3 on memory/cardinality, not bytes** — with N = RF every ingester holds every stream; run N > RF to shard streams and spread the burst.
- **Ingesters are ephemeral** (node-local `emptyDir`, no PVC) — durability is `replication_factor` 3, not disk. This makes ingesters freely reschedulable and sidesteps EBS zonal pinning / slow volume reattach on node replacement.
- **Do not set a tight memory limit on ingesters** — an OOM-kill drops in-memory/WAL-buffered logs. Use generous limits (or none) and alert on usage.

#### Protective limits

Per **tenant** (per environment). The aggregate burst is a fleet-capacity concern handled by ingester count, not by these. `[operator]` sets per profile.

| Size | `ingestion_rate_mb` | `ingestion_burst_size_mb` | `max_global_streams_per_user` |
|---|---|---|---|
| S | 4 | 8 | 5,000 |
| M | 8 | 16 | 10,000 |
| L | 16 | 32 | 25,000 |

### Checklist

#### 1. Topology & sizing

- [ ] `[operator]` Select the profile/size from the table above; record the measured sustained + 5-min burst it is based on.
- [x] `[chart]` Microservice/distributed mode; ingesters ≥ 3; compactor = 1 singleton.
- [ ] `[operator]` Query-scheduler is enabled by default at **M/L**; the **small** profile omits it (the query-frontend's own queue suffices there).
- [x] `[chart]` Ingester memory limit left unset; requests set for scheduling.

#### 2. Schema & storage

- [x] `[chart]` `schema_config`: TSDB, schema **v13**, 24h index period.
- [ ] `[consumer]` **(Terraform: automatic)** Provision the object-storage bucket (S3-compatible / GCS / Azure Blob); single bucket, prefixes `/loki/chunks`, `/loki/ruler`.
- [ ] `[consumer]` **(Terraform: automatic)** Object-store lifecycle policy aligned with (or longer than) Loki retention, so the compactor owns deletion. The modules leave bucket expiry off by default, which is the safe end of that alignment.
- [ ] `[operator]` Treat schema periods as **append-only** — future format changes go in a new period with a `from` date ahead of now, never by editing a past period.
- [ ] `[consumer]` **(Terraform: automatic)** On any backend other than S3, name it in all **three** load-bearing places — `storage.object_store.type`, the newest `schemaConfig` period, and `compactor.delete_request_store`. The chart's defaults are S3-shaped, and a stale one crash-loops the component that reads it with `no s3 endpoint in config file` rather than degrading. A render-time check refuses a mismatched set, and warns on the inert fourth (`storage.type`); see [Selecting the backend](../../logs-and-events/storing/#selecting-the-backend).

#### 3. Replication, ring & placement

- [x] `[chart]` `replication_factor: 3`; ring backend = memberlist (no Consul/etcd).
- [x] `[chart]` Ingester `topologySpreadConstraints`: **hard across zones** (`DoNotSchedule`) so an un-spread pod goes Pending and Karpenter provisions the missing zone; **soft across hosts** (`ScheduleAnyway`), with the chart's default hard per-host anti-affinity dropped by nulling its rule list.
- [ ] `[operator]` Aim for **≥3 zones** for true AZ resilience under RF 3; set `minDomains` to the zone count your node pool can launch in. With 2 zones, know an AZ loss can break write quorum until the ring recovers.
- [ ] `[operator]` If you bring nodes up **tainted** until DaemonSets are healthy, the spread already sets `nodeTaintsPolicy: Honor` (tainted nodes stay out of the skew math) — model the taint as a Karpenter `startupTaint` so it doesn't over-provision. Taints only gate placement.
- [x] `[chart]` PodDisruptionBudget on ingesters (`maxUnavailable: 1`), with a render-time warning if it or the rollout `maxUnavailable` is set > 1.
- [x] `[chart]` `priorityClassName` on every Loki pod (`monitoring-scalable`), so ingesters and the compactor outrank ordinary workloads under node pressure. `loki.global.priorityClassName` covers everything that renders through the chart's `_pod.tpl`; **three components read only their own key and are set separately** — the two memcached StatefulSets and the canary. The canary is the one that bites: left unset it runs at priority 0, *below* ordinary workloads, so the first node under pressure evicts the end-to-end write→read check, which is exactly the signal you want during the incident that caused the pressure. See [Scheduling priority](#scheduling-priority).

#### 4. Ingester durability & rollouts

- [x] `[chart]` **Ingesters are ephemeral** — node-local `emptyDir`, no PVC. Durability is `replication_factor` 3, so a killed/rescheduled ingester's un-flushed data is recovered from its peers.
- [x] `[chart]` **Index-gateway and compactor are also ephemeral** — their local disk is a read-through cache / idempotent working copy of the object-store index, so they reschedule freely across zones (the compactor singleton in particular is never PVC-pinned to one AZ).
- [x] `[chart]` `flush_on_shutdown: true` (best-effort) with a **modest** `terminationGracePeriodSeconds` (~60s). Do **not** rely on a long grace period — enterprise force-kill windows (120/300s) are harmless because replication covers a truncated flush.
- [x] `[chart]` StatefulSet rolls ingesters **one at a time** (`maxUnavailable: 1`); a burst rollout needs `zoneAwareReplication` (zone-at-a-time) or the alpha `MaxUnavailableStatefulSet` gate — neither is in play, and PDBs govern *drains*, not rollout speed.
- [ ] `[operator]` **Budget the roll and raise the deploy timeout.** The serial, readiness-gated ingester roll takes **~1 min per ingester** (so a 6-ingester roll ≈ 5 min) and is not bounded by node provisioning — it overruns Helm's default 5-min `--wait`. Set `helm upgrade --timeout` (or Flux `spec.timeout` / Pulumi `customTimeouts`; ArgoCD is async and tolerant). A wait-timeout here means "still rolling," not "failed." See [Upgrading](../upgrading/).
- [ ] `[operator]` Add ingesters (N > RF) when per-ingester memory/stream-count climbs or to spread the regression burst — streams shard across the ring only when N > RF.
- [ ] `[consumer]` **(Terraform: `storage_class`)** A dynamic-provisioning **StorageClass** still needs to exist for the **one PVC-backed Loki component, the ruler** (it keeps a PVC for its remote-write WAL) — CSI driver installed, not safe to assume on bare clusters, and on GCP C4/N4 no default class is attachable at all.

#### 5. Limits & cardinality

- [ ] `[operator]` Set per-tenant `ingestion_rate_mb` / `ingestion_burst_size_mb` / `max_global_streams_per_user` from the table; remember these are per environment.
- [x] `[chart]` `reject_old_samples: true` + `reject_old_samples_max_age` set.
- [x] `[chart]` The Alloy gateway keeps the label set small and routes high-cardinality fields to structured metadata — the dominant cost/stability lever. See [Collection (Alloy)](#collection-alloy).
- [ ] `[operator]` Alert on `loki_discarded_samples_total` so a limit hit is visible.

#### 6. Retention & compaction

- [x] `[chart]` Compactor `retention_enabled: true`, `delete_request_store`, `retention_delete_delay` configured.
- [ ] `[operator]` Set the global `retention_period` to the storage budget.
- [ ] `[operator]` Configure **tiered (per-stream) retention** — keep `ERROR`/audit streams long, expire high-volume `INFO` fast.

#### 7. Caching

- [x] `[chart]` Results cache (query-frontend), chunks cache, and index/stats cache on the bundled memcached.
- [x] `[chart]` Query-result caching enabled (`cache_results`, `max_cache_freshness_per_query`).
- [ ] `[operator]` Size memcached per the table.

#### 8. Read path

- [x] `[chart]` Query-frontend ≥ 2 for queue fairness; query splitting/parallelism configured.
- [x] `[chart]` Grafana Loki datasource provisioned, pointing at the **query-frontend** Service (bundled nginx loki-gateway is off; datasource wiring still to land).
- [ ] `[operator]` Scale queriers/frontends — not ingesters — when dashboards feel slow.

#### 9. Tenancy & auth

- [x] `[chart]` **One logical tenant per install:** `auth_enabled: true` with a single named `X-Scope-OrgID` (not the implicit `fake` tenant), so a future split is config — not a data migration, since the tenant ID is baked into the object-storage path.
- [ ] `[operator]` Isolation is **label-based** within the tenant (`environment_id`, …); the **hard isolation boundary is the install** (per region/stack). Fine for trusted internal consumers — revisit if per-team or customer-facing access is required (then Grafana LBAC, or tenant-per-environment writes + multi-tenant reads).
- [ ] `[operator]` Per-environment controls are label-based, not tenant-based: per-stream retention (`environment_id`) and per-label rate limits — both **static config**, which is why no `runtime_config` live reload is needed.
- [ ] `[operator]` Watch per-ingester memory against total fleet stream count — one tenant concentrates cardinality, bringing the N > RF lever ([§4](#4-ingester-durability--rollouts)) forward.
- [ ] `[consumer]` Provide the basic-auth (or mTLS) Secret **by name**; the chart consumes, it does not mint.

#### 10. Meta-monitoring

- [ ] `[chart]` ServiceMonitor/PodMonitor (or GCP `PodMonitoring`) for every Loki component.
- [x] `[chart]` `loki-canary` enabled for end-to-end write→read verification.
- [ ] `[chart]` Loki mixin dashboards + alerts installed.
- [ ] `[operator]` Tier-0 alerts wired to paging: ingester unhealthy/flush failures, compactor not running, discarded samples, object-store errors, disk usage. **Loki down is its own incident.**

#### 11. Security & credentials

- [ ] `[consumer]` **(Terraform: automatic on AWS and GCP)** Object-store access via **workload identity** (IRSA / GKE WI / Azure WI) — see [Storing > Granting object-storage access](../../logs-and-events/storing/#granting-object-storage-access-workload-identity) for the per-provider setup; static keys only as a documented escape hatch.
- [ ] `[consumer]` **(Terraform: automatic on AWS and GCP)** No long-lived credentials in the chart; storage secret by reference.
- [ ] `[chart]` `runAsNonRoot`, read-only root filesystem, dropped capabilities on all components.
- [ ] `[operator]` Optional inter-component TLS where the cluster requires it.

##### NetworkPolicy egress (if `networkPolicy.enabled`)

Enabling the NetworkPolicy denies egress by default except what it explicitly allows. Loki needs **external** egress that the base policy does *not* grant, so you must permit it or Loki hangs.

- [x] `[chart]` Egress to **object storage AND the credential endpoint** on 443 via `networkPolicy.externalStorage` (default `ports: [443]`, `cidrs: ["0.0.0.0/0"]`). Tighten `cidrs` per environment (see below).
- [ ] `[operator]` The credential path must be covered, not just the bucket: **IRSA fetches credentials from AWS STS on 443** (GKE/Azure WI have their own token endpoints). Missing STS egress is the classic failure — the compactor blocks at startup fetching credentials for the delete-requests store, its HTTP server never serves, the liveness probe kills it every ~5 min, and it crashloops with misleading `memberlist … WriteTo … i/o timeout` noise. The block is silent (a hanging TCP connect), so it looks like anything *but* a network policy.
- [ ] `[operator]` **Do not rely on an ambient broad-443 egress rule from another workload** (e.g. an application-level "reach the external kube API server" rule). It won't select the Loki namespace, and even where it does it's a load-bearing coincidence: the day someone scopes that rule down, Loki breaks. Declare Loki's egress explicitly.
- [ ] `[operator]` **Tighten for production:** prefer **VPC endpoints (PrivateLink) for S3 + STS** and scope `cidrs` to the VPC / endpoint CIDRs — `0.0.0.0/0:443` is a reasonable default only when egress is already governed at the infra layer (SGs, NAT, egress firewall). On Cilium, FQDN egress (`toFQDNs`) to the S3/STS hostnames is a good tight-but-not-brittle alternative.

#### 12. Day-2: upgrades, migration, DR

- [ ] `[operator]` Upgrade ingesters one-at-a-time with flush (see [§4](#4-ingester-durability--rollouts)).
- [ ] `[operator]` Schema changes = new period, never in place (see [§2](#2-schema--storage)).
- [ ] `[consumer]`/`[operator]` DR = object versioning + cross-region replication + (for audit) Object Lock/WORM; restore = repoint at the bucket. No native snapshot — see [Storing](../../logs-and-events/storing/). The Terraform modules enable **versioning** by default; replication and Object Lock remain yours.
- [ ] `[consumer]` **(Terraform: automatic)** Pin the Loki chart/image version; upgrades are deliberate.

#### 13. Validation

- [ ] `[chart]` `loki -verify-config` runs in CI and as an initContainer before a component serves.
- [ ] `[chart]` `helm template | kubeconform` + `helm lint` in CI.
- [x] `[chart]` Smallest integration profile = single-binary + filesystem (no object store) for hermetic e2e tests.

### See also

- [Logs & Events](../../logs-and-events/architecture/) — the logging architecture these items configure.
- [Storing](../../logs-and-events/storing/) — storage, retention, and disaster recovery in depth.
- [Upgrading](../upgrading/) — cross-cutting upgrade guidance.
- [Loki production deployment](https://github.com/grafana/loki/tree/main/production/ksonnet/loki) (official) — Grafana's reference production config (built for far larger volumes; read it for the patterns, not the magnitudes).

## Metrics (Thanos)

For the architecture these items configure, see [Metrics](../../metrics/).

Thanos is **on par with Loki** in this chart now: sizing profiles ship, every component carries resource requests, and PodDisruptionBudgets, autoscaling, and zone-aware topology spread are all in place.
What remains unchecked below is mostly `[operator]` and `[consumer]` work — decisions and cloud resources the chart cannot make for you.
Treat any unchecked `[chart]` item as the current work list rather than as guidance you are expected to satisfy by hand.

### Receive: replication is the availability lever, not `mode`

The single most consequential setting, and the one most often misread.

`receive.mode` is a **topology** choice:

- **`standalone`** (default) is *RouterIngestor* mode — one workload that both routes and ingests. It still builds a ketama hashring across the StatefulSet pods (`receive.hashrings.autogen`), so `replicaCount: 3` shards writes across three pods.
- **`split`** separates Router (Deployment) from Ingester (StatefulSet) so they scale independently.

Neither choice, by itself, gives you redundancy. **The replication factor does**, and the chart passes `--receive.replication-factor` **only in split mode** — so standalone runs at Thanos's default of **1** unless `receive.extraArgs` sets it.

Thanos write quorum is `(rf / 2) + 1`:

| Replication factor | Write quorum | Ingester losses tolerated |
|---:|---:|---:|
| 1 | 1 | 0 |
| **2** | **2** | **0** |
| 3 | 2 | 1 |
| 4 | 3 | 1 |
| 5 | 3 | 2 |

**Use an odd factor.** A factor of 2 requires both copies and tolerates nothing — the same as 1, and on a small ring *worse*: on three pods, losing one fails ~1/3 of series at RF 1 but ~2/3 at RF 2, because more series depend on any given pod while quorum still demands all of their copies. Even factors above 2 tolerate no more losses than the odd factor below them; the extra copy buys durability, not availability.

```yaml
thanos:
  receive:
    replicaCount: 3
    extraArgs:
      - --receive.replication-factor=3
```

This is the same shape as Loki, where ingesters are deliberately ephemeral and durability comes from RF 3 alone — **Thanos Receive is `emptyDir`-backed for the same reason**, and the replication factor is therefore the only thing standing between you and data loss. It holds up to `receive.tsdb.retention` (6h) of blocks locally, of which at most 2h has not yet reached object storage. With RF 1 that window exists in exactly one copy, on one node. See [Storage: ephemeral by default](#thanos-ephemeral-storage) for why no volume is involved.

> [!WARNING]
>   **Split mode is not recommended yet.** It landed upstream only recently, and `receive.ingester` does not inherit from the top-level `receive.*` defaults — upstream considers the non-merging behavior intentional, so this is unlikely to change. In practice that means restating ~31 keys, of which the values schema hard-requires eight sub-objects (`hashrings`, `service`, `vpa`, `persistence`, `podSecurityContext`, `probes`, `serviceMonitor`, `pdb`) before the chart will render at all. Prefer standalone with an odd replication factor until that changes.

### Sizing the metrics backend

**Size by active series and object count — not by node count, and not by throughput.**
This is the opposite axis from Loki, where bytes per second drives everything.
Thanos keys off cardinality, and the derived quantities split across components:

- **Receive memory** → **active series**. Budget ~3 KB per series held.
- **Receive CPU and local disk** → **ingested samples/sec**, which is active series ÷ scrape interval. Every scrape surface this chart generates defaults to **30s**.
- **Store Gateway and Compactor** → **block volume under retention**.
- **Query and Query Frontend** → **query concurrency**, independent of ingest.

Those four move independently, which is why halving the scrape interval doubles Receive's CPU and disk while leaving its memory untouched.

#### Choosing a profile {#choosing-a-thanos-profile}

Nobody knows their series count before they have somewhere to put metrics, so profiles are chosen from **inventory** — what exists, not how busy it is.
Workload-intensity metrics are the wrong shape for this: they swing on a daily cycle, while cardinality is nearly flat.

| | S (bottom ~20%) | M (middle ~70%, chart defaults) | L (top ~10%) |
|---|---|---|---|
| **Collections** (indexes + materialized views) | ≤ 100 | 100–1,000 | 1,000+ |
| Kubernetes nodes | ≤ 10 | 10–100 | 100+ |
| Pods, all namespaces | ≤ 250 | 250–2,000 | 2,000+ |
| Cluster replicas | ≤ 4 | 4–30 | 30+ |

**Take the largest tier any single row lands in.**
Series is a sum, and one oversized dimension is enough to overrun the ingest path by itself.

Collections is the row that does the work, because Materialize's per-collection metrics carry `collection_id`, `replica_id`, *and* `worker_id` — so they multiply by replication and by replica size, while sources and sinks stay linear in the object count.
A deployment with a few hundred indexes contributes far more cardinality than one with a few hundred sources.

> [!WARNING]
>   **These are whole-cluster numbers, not Materialize's share of the cluster.**
>   node-exporter, cAdvisor, and kube-state-metrics collect from every node and pod in the cluster, including workloads that have nothing to do with Materialize.
>   Running Materialize on ten nodes of a shared two-hundred-node cluster is an **L**, not an S — and sizing off the Materialize footprint alone is the most common way to pick wrong.

For a number rather than a bracket:

```text
active series ≈ 8,000 × nodes + 600 × collections
```

Calibrated against real deployments spanning two orders of magnitude of series count, and accurate to within ~10% across that range.
It is deliberately two terms with no constant — every other candidate axis measured (per-vCPU, per-replica, per-source) either tracked one of these two or fell out as noise.
Per-vCPU in particular is actively misleading: vCPU tracks how much work a deployment does, and cardinality tracks how many objects exist, so the two diverge as deployments grow.

#### The envelope each profile is sized for {#thanos-envelope}

| Size | Active series | Samples/s at 30s | Bucket at profile retention |
|---|---|---|---|
| **S** | ~150 k | ~5 k | ~25 GB |
| **M** | ~1.5 M | ~50 k | ~660 GB |
| **L** | ~4 M | ~133 k | ~3 TB |

In a large deployment **Materialize's own metrics are ~85% of everything collected** — its native endpoints plus the SQL exporters — and the Kubernetes infrastructure terms are a rounding error beside them.
So a conversation about metrics cost at that scale is a conversation about collections, not about nodes or pods.

#### Measure to validate {#thanos-measure-to-validate}

Size from the estimator, then re-measure off Receive — the same signal the sizing is derived from. `[operator]`

```promql
# active series actually held (compare against the envelope you picked)
sum(prometheus_tsdb_head_series{job="thanos-receive"})

# ingested samples/sec
sum(rate(prometheus_tsdb_head_samples_appended_total{job="thanos-receive"}[5m]))

# series contributed per scrape job — the decomposition, when the total surprises you
sort_desc(sum by (job) (scrape_samples_scraped))
```

Reach for the third when measured series disagree with the estimate.
Its value per target *is* that target's series count, so it decomposes the total without touching every series in the store — unlike `count({__name__=~".+"})`, which does, and is expensive enough for that to matter.

#### Per-size resources {#thanos-per-size-resources}

Starting points — `replicas × (cpu request / memory request)`; tune from real usage. `[chart]` defaults, `[operator]`/`[consumer]` override per profile.

| Component | S | M | L |
|---|---|---|---|
| Receive (RF 3, `emptyDir`) | 3 × (250m / 1Gi) | 3 × (500m / 4Gi) | 6 × (1500m / 8Gi) |
| Receive `ephemeral-storage` req / limit | 2Gi / 3Gi | 4Gi / 6Gi | 6Gi / 8Gi |
| Store Gateway | 2 × (100m / 1Gi) | 2 × (500m / 3Gi) | 2 × (1500m / 8Gi) |
| Store Gateway **PVC** | 10Gi | 10Gi | 20Gi |
| Compactor (singleton) | 1 × (500m / 1Gi) | 1 × (1 / 2Gi) | 1 × (3 / 8Gi) |
| Compactor **PVC** | 20Gi | 50Gi | 200Gi |
| Query (HPA) | 2–3 × (200m / 512Mi) | 2–5 × (500m / 1Gi) | 3–8 × (1 / 2Gi) |
| Query Frontend | omit | omit | 2–5 × (250m / 512Mi) |

- **At `replicaCount == replicationFactor` every Receive pod holds every series.** Sharding begins only above RF, which is why **L** runs six pods rather than three — and why a replica count that is a multiple of both the replication factor and the zone count spreads evenly across both.
- **Do not set a tight memory limit on Receive.** An OOM-kill drops up to `tsdb.retention` (6h) of locally-held blocks, and unlike Loki's ingesters that window has nothing protecting it but the replication factor.
- **Store Gateway has a memory floor the requests must respect.** Thanos defaults `--chunk-pool-size=2GB` and `--index-cache-size=250MB`, and the subchart passes neither, so a stock Store Gateway wants ~2.5Gi before it serves anything. The **S** profile shrinks those pools through `extraArgs` rather than only lowering the request — a 1Gi request against 2.25GB of pools is an OOM, not a small install.
- **The Compactor is vertical-only.** It must stay `replicaCount: 1`, because concurrent compactors against one block set corrupt data. Its PVC is scratch space for the block group under compaction, which is why it grows faster with size than the other two volumes.

#### Spreading across zones {#thanos-topology-spread}

Replication factor 3 is a claim about surviving a lost availability zone, and it is only true if the three replicas are *in* three zones.
Nothing makes that happen by default: a scheduler with no constraint is free to pack all three onto whatever nodes are cheapest, at which point RF 3 costs three times the memory and protects against nothing but a single pod restart.

| Component | Zones | Hosts | Why |
|---|---|---|---|
| Receive | **hard** (`DoNotSchedule`) | soft | Write quorum (2 of 3) depends on it |
| Store Gateway | soft | soft | Read capacity only, and PVC-backed |
| Query, Query Frontend | soft | soft | Stateless and autoscaled |
| Compactor | none | none | Singleton — nothing to spread against |

**Hard on Receive is the load-bearing choice**, and it is hard for a reason that is easy to miss: a pod that cannot satisfy the constraint goes `Pending`, and *that* is the signal Karpenter or the cluster-autoscaler uses to provision a node in the deficient zone.
A soft rule cannot summon capacity — it places the pod in the wrong zone and stays quiet about it, which is the failure this exists to prevent.

Two settings on that constraint matter as much as the constraint itself.
`nodeTaintsPolicy: Honor` keeps a node still carrying a startup taint from counting as an available domain, so the autoscaler is not told a zone is covered when nothing can run there yet.
`matchLabelKeys: [controller-revision-hash]` counts only same-revision pods, so a rolling update does not deadlock its own skew math against the pods it is replacing — and note the label differs by workload kind: StatefulSets carry `controller-revision-hash`, Deployments carry `pod-template-hash`, and using the wrong one matches nothing and silently disables the guard.

> [!INFO]
>   **This became possible when Receive stopped using a PersistentVolume.**
>   A zonal volume cannot be attached from another zone, so a hard zone rule and an AZ-pinned pod pull against each other: the rule says "go to the empty zone", the volume says "you may only run where I am", and the pod stays `Pending` permanently rather than for as long as it takes to add a node. On `emptyDir` there is nothing holding it back, which is why [the storage decision](#thanos-ephemeral-storage) and this one are the same decision viewed twice.
>
>   It is also why the **Store Gateway** — the one Thanos component that still keeps a volume — is soft rather than hard. With `volumeBindingMode: WaitForFirstConsumer` (the default for zonal CSI classes, and what you want) the volume follows the pod and the two agree; with `Immediate` the PVC's zone is chosen *before* scheduling and a hard rule pointing elsewhere deadlocks. Soft degrades on a StorageClass this chart does not control.

Unlike Loki, nothing had to be undone to make room for this: the Thanos subchart ships no default pod anti-affinity, so there was no hard per-host rule to null out first.

**These constraints cannot move to `thanos.global.topologySpreadConstraints`**, which is where a reader would look for them.
Each one carries its own `labelSelector`, and a global constraint would make every Thanos component count Receive's pods when computing its own skew.
The selector matches on `app.kubernetes.io/component` alone rather than the full label set, because the subchart renders these through `toYaml` rather than `tpl` — a `{{ include ... }}` would land in the manifest literally, so the release name is unavailable. Spread is namespace-scoped and this chart assumes [one instance of each backend per namespace](#namespace-layout), so the component label is unambiguous.

#### Fewer than two zones {#thanos-few-zones}

The hard zone constraint assumes at least two availability zones, which is every managed cloud default and not much else.
Below that it **fails closed**: the stack does not come up, and the cause reads like a chart bug rather than a property of the cluster.

| Zones | What happens by default | Fix |
|---|---|---|
| 0 (no labels) | Receive and Loki's ingesters are **unschedulable** — a `DoNotSchedule` constraint whose `topologyKey` no node carries has no domain to place into | `min_zones = 0`, or the `no-zone-spread` profile |
| 1 | **Also Pending**, and much less obvious — see below | `min_zones = 1`, or the same profile |
| 2 | Works. The chart's floor | nothing |
| 3+ | Works, but under-protects: `minDomains: 2` is satisfied by a two-zone placement | `min_zones = 3` (or your real count) |

**One zone is exactly as broken as none, and that surprises people.**
When the number of eligible domains is below `minDomains`, Kubernetes treats the global minimum as 0 rather than skipping the check — so a single zone holding all three replicas computes a skew of 3 against a `maxSkew` of 1, and every pod stays `Pending`.
Nothing about the symptom points at a zone count.

Zero-zone clusters are not an edge case worth dismissing: `kind` labels no node this way, and neither do many on-premises distributions.
Hand-labelling nodes to satisfy a monitoring chart is the wrong answer, which is why `min_zones` exists.

Both fixes drop the hard zone rule and **keep the soft host rule**, which matters most on exactly these clusters — with no zones to lose, the node is the only failure domain there is.
On the Terraform path that falls out for free, because `min_zones` filters the one constraint rather than replacing the list; on the Helm path the profile has to restate the host rule, since Helm overwrites lists rather than merging them.

> [!WARNING]
>   **Nothing reminds you when you gain a zone.** The pods keep scheduling happily, spread across nothing, and RF 3 goes on reading like AZ resilience on every dashboard. `min_zones` is the better habit than the profile for this reason alone: it is a number you update when the cluster changes, not a file you have to remember to stop passing.

#### Storage: ephemeral by default {#thanos-ephemeral-storage}

Of the three Thanos components with local disk, **only Receive uses node-local `emptyDir`**, with an explicit `ephemeral-storage` budget.
The Store Gateway and the Compactor both keep a PersistentVolume, for different reasons.

| Component | Storage | Why |
|---|---|---|
| Receive | `emptyDir` | Holds *hours* of small, replicated data; durability is the replication factor, not the disk |
| Compactor | **PVC** | Holds *days* of it — the requirement does not fit node ephemeral storage |
| Store Gateway | **PVC** | Index-headers are cheap to lose but slow to rebuild, and there is no write quorum to block |

**The dividing line is how much disk the component needs, not whether its data is reconstructible.**
Neither Receive's un-uploaded window nor the Compactor's scratch is authoritative — the bucket is — so a durability argument would send both to `emptyDir`.
What separates them is volume: Receive holds a 6h window of one replica's writes, which is gigabytes; the Compactor works on whole block groups, so it needs the sources *and* the output of a multi-day compaction at once, which is tens of gigabytes.
Node ephemeral storage cannot serve the second.

**A PVC makes an availability-zone failure worse, not merely less flexible.**
That is the reasoning behind all three rows, and it is the opposite of the intuition a volume usually carries.
An EBS volume cannot be attached from another zone, so a pod whose zone is gone stays `Pending` until the zone comes back rather than rescheduling into a healthy one.
On the write path that converts a recoverable event into an outage that waits on the cloud provider: with RF 3 write quorum is 2, so two Receive pods stuck `Pending` on dead volumes block writes outright, where two `emptyDir` pods would have been rescheduled and rejoined the hashring.

This is the same call the chart already makes for [Loki's ingesters](#4-ingester-durability--rollouts), for the same reason.
Blocks ship to object storage every 2h, so the window that exists only on local disk is at most 2h — and every replica uploads its own copy under a distinct `replica` external label, which the Compactor deduplicates.
A pod that returns with an empty volume has lost its copy of that window; the query path still answers from the surviving replicas.

> [!WARNING]
>   **The trade accepted in exchange:** Thanos Receive has **no peer hand-off**, so unlike Loki it will not backfill an emptied pod from its neighbours.
>   That window stays at two copies instead of three until the next block ships. Loki's ingesters recover from peers; Thanos's do not.
>
>   This is why the replication factor is load-bearing rather than merely advisable, and why RF 1 with ephemeral storage is a genuinely unsafe combination rather than just an unwise one.

The Compactor is the case where the AZ argument loses, and it is worth saying why rather than leaving the inconsistency to be discovered.
Everything above applies to it — it is a **hard** singleton, so pinning it to a zone means a zone outage stops compaction entirely, and while compaction is stopped **retention is not enforced and the bucket grows without bound.**
That is a genuine cost.

It loses anyway, because the alternative is worse: at the medium envelope a 2d compaction group is roughly six 8h blocks at ~2.4Gi each, and the Compactor needs the sources and the output together — on the order of 30Gi.
A GKE node with a 47Gi boot disk offers about 18.8Gi of *allocatable* ephemeral storage, so an `emptyDir` that size never schedules at all.
**"Compaction pauses during a zone outage" beats "compaction never runs because the pod cannot be placed."**
And the pause is recoverable by hand: the scratch is not authoritative, so a Compactor wedged in a dead zone is unstuck by deleting its PVC and letting it rebind elsewhere — see [The Thanos Compactor is stuck in a zone](../o11y-troubleshooting/#the-thanos-compactor-is-stuck-in-a-zone) for the ordering, which matters because two Compactors running at once is the one thing that corrupts data.

#### Declaring the ephemeral budget {#thanos-ephemeral-budget}

`emptyDir` volumes are invisible to the scheduler unless you say how large they will get, so every ephemeral component declares `requests.ephemeral-storage` and `limits.ephemeral-storage`.
The two behave differently and the difference matters:

- **`requests.ephemeral-storage`** is what the scheduler places against. A node without room refuses the pod, which surfaces as a `Pending` pod with a clear reason instead of a node quietly filling up.
- **`limits.ephemeral-storage`** is enforced by the **kubelet evicting the pod**, not by throttling it. Accounting is periodic (`du` every ~10s unless filesystem project quotas are enabled), so a pod can overshoot briefly before eviction.

That second point is why the limits in [Per-size resources](#thanos-per-size-resources) carry deliberate headroom over the requests rather than matching them: **evicting Receive destroys exactly the un-uploaded window the replication factor exists to protect**, so a limit set tight enough to bite regularly would manufacture the failure this design avoids.

> [!WARNING]
>   **Size these against a node's *allocatable* ephemeral storage, not its disk size.** The two are nowhere near each other, and the gap is where this goes wrong.
>
>   GKE reserves most of the boot disk for the image filesystem, so a **47Gi disk offers roughly 18.8Gi allocatable**. A request above that cannot schedule anywhere, and the cluster-autoscaler will not rescue it either — no node of that shape would fit, so it declines to scale up and the pod sits `Pending` indefinitely:
>
>   ```text
>   0/5 nodes are available: 4 Insufficient ephemeral-storage.
>   Pod didn't trigger scale-up: 3 Insufficient ephemeral-storage
>   ```
>
>   Check yours before raising anything here:
>
>   ```bash
>   kubectl get nodes -o custom-columns='NODE:.metadata.name,EPH:.status.allocatable.ephemeral-storage'
>   ```
>
>   Remember the budget is shared. Loki's ingesters are `emptyDir`-backed too and declare no `ephemeral-storage` at all, so the scheduler believes they need none — a node can look free and still be full.

- [ ] `[operator]` Check the Compactor's ephemeral request against your node shape before selecting `thanos-large` — 100Gi of `emptyDir` needs a node with 100Gi of free ephemeral storage, which many node pools do not have. If yours cannot, re-enable `thanos.compactor.persistence` and accept the AZ pin; that is a considered trade, not a failure.
- [ ] `[operator]` Ephemeral storage is shared with container logs and image layers on the same filesystem, so the budget is not Thanos's alone. A log-verbose neighbour can evict a Compactor that was sized correctly in isolation.

#### Protective limits {#thanos-protective-limits}

Thanos's exposure is the mirror image of Loki's.
There is no per-tenant byte-rate limit to set; the risk is one query fanning out across the bucket and taking the Store Gateway down with it.
All of these are `extraArgs`. `[operator]` sets per profile.

| Size | `--store.limits.request-series` | `--store.limits.request-samples` | `--query.max-concurrent` | `--query.timeout` |
|---|---|---|---|---|
| S | 1,000,000 | 50,000,000 | 10 | 2m |
| M | 5,000,000 | 200,000,000 | 20 (default) | 2m |
| L | 30,000,000 | 2,000,000,000 | 40 | 5m |

> [!WARNING]
>   **`extraArgs` is a list, and Helm overwrites lists rather than merging them.**
>   Several components ship non-empty defaults — `receive.extraArgs` carries `--receive.replication-factor=3`, `compactor.extraArgs` carries `--consistency-delay=30m`, `query.extraArgs` carries `--log.level=info`.
>   Setting `extraArgs` to add a limit **silently drops whatever was already there**, and on Receive that means falling back to Thanos's default replication factor of 1 — the exact failure the quorum table above exists to prevent.
>   Restate the base arguments in full every time. The shipped profiles do.

Receive also offers `--receive.limits-config` with a per-tenant `head_series_limit`, which the profiles deliberately leave unset.
It enforces that cap by querying a meta-monitoring endpoint for `head_series` on an interval, so it is a runtime dependency on Thanos Query rather than a flag, and it fails in a direction that is not obvious when the dependency is unavailable.

#### Retention and downsampling {#retention-and-downsampling}

| Size | raw | 5m | 1h |
|---|---|---|---|
| S | 14d | 30d | 90d |
| M | 30d | 90d | 365d |
| L | 30d | 180d | 730d |

> [!INFO]
>   **Downsampled volume does not depend on the scrape interval.**
>   A 5m block is five aggregates per 300s and a 1h block five per 3600s, however often you scrape — only raw blocks scale with the interval.
>   So moving from a 60s interval to 30s raises total bucket size by roughly a fifth rather than doubling it, because raw is the smaller share of a retention shape that keeps downsampled data far longer.
>   The cost of finer granularity lands on Receive's CPU and local disk, not on object storage.
>
>   The corollary is that at a 60s scrape interval **5m downsampling saves no storage at all** — five aggregates per 300s is the same sample rate as one per 60s. It buys query speed over long ranges. All the real storage reduction is in the 1h tier.

**Raw retention has a floor set by the downsampling thresholds.**
Thanos produces 5m downsamples only from blocks spanning 40h or more, and 1h downsamples only from blocks spanning 10d or more.
Cut raw retention below those and the corresponding tier is never created at all, so long-range queries fall back to reading raw blocks — slower and more expensive, which is the opposite of the intent.
That is why **S** keeps 14d of raw rather than something smaller.

#### Scaling past large {#scaling-past-large}

Above roughly 4M active series the profiles stop being a menu and become a starting point.
Three things change, in the order they bite:

1. **Add Receive replicas**, in multiples of the replication factor and the zone count, so streams shard evenly and no pod carries the whole series set.
2. **Shard the Store Gateway** once the bucket reaches the multi-TB range. Binary index-headers run roughly 1% of block size, so one StatefulSet eventually cannot hold them. `storegateway.sharded.hashPartitioning.shards` renders one StatefulSet per shard, and the headless Service, ServiceMonitor, and Query's DNS-SRV discovery fan out across them with no further configuration. Changing the shard count later renumbers shards and forces recreation, so decide it at install time rather than tuning into it.
3. **The Compactor runs out of room to grow.** It is a singleton by necessity, so the only levers are a bigger pod and a bigger volume, and compaction falling behind becomes a thing to alert on rather than discover. Thanos itself supports sharding compaction across instances with disjoint `--selector.relabel-config` selections, but the bundled subchart renders exactly one Compactor StatefulSet and exposes no selector — so that path needs an upstream change, or a second Compactor managed outside this chart.

#### Cost control is the metric tiers, not the profile {#cost-control-metric-tiers}

The write path filters by importance — `essential`, `recommended`, `extended`, `diagnostic`, `all` — through `minMetricImportance` on each destination.
The in-cluster Thanos destination deliberately defaults to **`all`**, unlike the external destinations, which default to `recommended`.
Raw metrics are worth their cost while Thanos is still an early improvement over what came before; narrowing that is a later and deliberate step, not an oversight to correct.

> [!WARNING]
>   **Do not downshift the profile to reduce cost.**
>   The profile sizes the ingest path for the series you actually send, so sending the same series into a smaller path is an OOM rather than a saving.
>   Tighten `minMetricImportance`, re-measure head series, and downshift only once the series count has genuinely fallen.


### Checklist

#### 1. Ingestion topology & replication

- [x] `[chart]` `receive.mode: standalone` with `replicaCount: 3` and an auto-generated ketama hashring.
- [ ] `[operator]` Set an **odd** `--receive.replication-factor` (3) via `receive.extraArgs`; the chart cannot set it in standalone mode. A render-time check warns at factor 1, warns harder at 2, and errors when the factor exceeds `replicaCount`.
- [ ] `[operator]` Keep `replicaCount >= replicationFactor`. At `replicaCount == replicationFactor` every pod holds every series — good availability, no horizontal capacity.
- [x] `[chart]` PodDisruptionBudgets on **every** component (`thanos.global.pdb`, `maxUnavailable: 1`) — matching the Loki ingester convention. `maxUnavailable` rather than `minAvailable` deliberately: it scales with the replica count, and on the single-replica Compactor `minAvailable: 1` would permit no eviction at all and hang node drains. A validator errors when the Receive budget exceeds what write quorum tolerates, and warns on `minAvailable` for the singleton.
- [x] `[chart]` `topologySpreadConstraints` across zones for Receive — **hard** (`DoNotSchedule`, `minDomains: 2`, `nodeTaintsPolicy: Honor`) so RF 3 actually survives an AZ loss rather than landing three copies in one zone; **soft across hosts** so pods still schedule when nodes are momentarily scarce. Matching the Loki ingester convention. Store Gateway, Query, and Query Frontend get **soft** spread on both axes; the Compactor gets none, being a singleton. See [Spreading across zones](#thanos-topology-spread).
- [ ] `[operator]` **Tell the chart how many zones you actually have.** The default assumes two or more, and below that it fails closed rather than degrading — see [Fewer than two zones](#thanos-few-zones). Terraform: `min_zones`. Helm: the `no-zone-spread` profile.
- [ ] `[operator]` Aim for **≥3 zones** for true AZ resilience under RF 3, and raise `minDomains` to the count your node pool can actually launch in — the chart ships a floor of 2, so with three zones a two-zone placement still satisfies it. Setting it *above* your real count leaves pods Pending forever. With 2 zones, know that an AZ loss can break write quorum (2 of 3) until the ring recovers.
- [x] `[chart]` `priorityClassName` on every Thanos pod (`monitoring-scalable`, via `thanos.global`), so Receive and the Compactor outrank ordinary workloads under node pressure. See [Scheduling priority](#scheduling-priority).

#### 2. Object storage & credentials

- [x] `[chart]` Objstore config rendered into a Secret (`global.objstore.createSecret`), consumed by every component.
- [ ] `[consumer]` **(Terraform: automatic on AWS and GCP)** Supply the bucket and grant access by **workload identity** (IRSA / GKE Workload Identity / Azure Workload Identity) rather than static keys. A render-time check errors when the identity annotation names a different cloud than the objstore backend, and warns when a cloud backend has neither an annotation nor inline credentials.
- [ ] `[consumer]` **(Terraform: `storage_class`)** A dynamic-provisioning **StorageClass** must exist *and be attachable by the nodes it lands on* — for Thanos that is the **Store Gateway only**, since Receive and the Compactor are `emptyDir`-backed. On GCP C4/N4 that rules out every default class; see [Getting Started > Terraform](../../getting-started/terraform/#storageclass-on-gcp-c4-and-n4-node-pools).
- [ ] `[operator]` Those two need **node ephemeral storage** instead, which the requests in [Per-size resources](#thanos-per-size-resources) declare. The Compactor's is the one to check against your node shape: 100Gi at `thanos-large`.

#### 3. Components & read path

- [x] `[chart]` Query, Receive, Store Gateway, and Compactor enabled by default.
- [ ] `[operator]` Enable **Query Frontend** for production read paths (splitting and result caching) — and repoint `connections.datasources.thanos.url` at it, or the cache is deployed and bypassed. A render-time check warns on exactly that mismatch. The `thanos-large` profile does both; at S and M the query-frontend is omitted and Query is the datasource target.
- [ ] `[operator]` Store Gateway is how queries reach historical blocks; disabling it limits reads to what Receive still holds locally.
- [x] `[chart]` **Horizontal autoscaling on Query** (2–5 replicas, 80% CPU), and on Query Frontend once it is enabled — both are stateless, with no ring membership or local state. Store Gateway autoscaling is deliberately **off**: it is a PVC-backed StatefulSet that syncs the bucket index on startup, so scale-up serves nothing until it is warm, and scale-down orphans PVCs.
- [ ] `[operator]` Keep `replicaCount` equal to `autoscaling.minReplicas`. The subchart templates a static `replicas` even alongside an HPA, so every upgrade or GitOps reconcile writes it back — matching the floor makes that reset a no-op instead of a scale blip. A validator warns when the two disagree.

#### 4. Retention & compaction

- [x] `[chart]` Compactor enabled with downsampling retention: raw 30d, 5m 90d, 1h 365d — the medium row of [Retention and downsampling](#retention-and-downsampling).
- [ ] `[operator]` Set those to your storage budget. Retention is enforced by the Compactor — with it disabled nothing expires and bucket cost grows without bound.
- [ ] `[operator]` Keep raw retention above the downsampling thresholds (40h for the 5m tier, 10d for the 1h tier). Below them the tier is never produced and long-range queries silently fall back to raw blocks. See [Retention and downsampling](#retention-and-downsampling).
- [x] `[chart]` Receive TSDB **local retention 6h** (overriding the subchart's 24h) with WAL compression. Blocks still ship to object storage every 2h — retention is a recent-query cache, not a durability window, and the Store Gateway serves everything older. 6h is what makes the `emptyDir` budget fit a modest node: 24h at the medium envelope is ~7.3Gi per pod, against ~18.8Gi allocatable on a typical GKE node shared with Loki.
- [ ] `[operator]` Raising local retention raises the ephemeral request with it, roughly linearly. Check [the allocatable warning](#thanos-ephemeral-budget) first — this is the setting most likely to make Receive unschedulable.

#### 5. Sizing

- [ ] `[operator]` Pick the profile from [Choosing a profile](#choosing-a-thanos-profile) and record the inventory it was based on. `thanos-small` and `thanos-large` are deltas from the chart defaults, which target medium.
- [ ] `[operator]` Re-measure head series against the envelope after install, and after any change in collection count. See [Measure to validate](#thanos-measure-to-validate).
- [x] `[chart]` Resource requests on every enabled component, per [Per-size resources](#thanos-per-size-resources). Thanos sizes off different axes than Loki — active series for Receive memory, samples/sec for its CPU and disk, block volume for Store Gateway and Compactor, query concurrency for Query and Query Frontend.
- [x] `[chart]` No memory limit on Receive, so an OOM-kill cannot drop the un-uploaded block window. Requests are set for scheduling; alert on usage instead.
- [ ] `[operator]` Autoscaling on Query does not remove the need for requests: without them the HPA has no CPU target to measure against, so it never scales.
- [x] `[chart]` **VerticalPodAutoscaler disabled on Receive and Compactor.** The subchart defaults it *on* for exactly those two, in `updateMode: Auto`, so where the VPA CRD is present it rewrites the requests a profile sets and evicts pods to apply them — on a StatefulSet holding up to `tsdb.retention` (6h) of blocks on an `emptyDir` whose only redundancy is the replication factor. The template is CRD-gated, so leaving it on would make the stack behave differently on clusters that have VPA installed than on those that do not, with no signal either way. A unit test asserts this with the CRD declared present, so it fails on the value rather than passing on the gate.
- [x] `[chart]` **Every profile restates `receive.extraArgs` in full**, so the replication factor survives. Helm overwrites lists, and a profile that adds a flag without restating `--receive.replication-factor=3` silently drops to Thanos's default of 1. A unit test pins this at all three sizes.
- [ ] `[operator]` If you re-enable VPA deliberately, use `updateMode: "Off"` for recommendations only, and know that its numbers will disagree with the profile by design.

#### 6. Meta-monitoring

- [x] `[chart]` ServiceMonitors for every Thanos component (`thanos.global.serviceMonitor`).
- [ ] `[operator]` Alert on Receive write failures and quorum errors — with RF 1 or 2 these are the first sign of a lost pod, and they are silent from the dashboards' point of view.

#### 7. Validation

- [x] `[chart]` Render-time validators cover objstore placeholders, backend/identity mismatch, component topology, replication-factor quorum, and writers or datasources aimed at a Thanos that is not deployed.
- [ ] `[chart]` `helm template | kubeconform` in CI (shared with the Loki checklist).

### See also

- [Metrics](../../metrics/) — the metrics architecture these items configure.
- [Storing](../../metrics/storing/) — object storage and retention in depth.
- [Thanos Receive documentation](https://thanos.io/tip/components/receive.md/) (official) — hashring, replication, and quorum semantics.
- [Thanos Compactor documentation](https://thanos.io/tip/components/compact.md/) (official) — compaction levels, downsampling thresholds, and why the singleton constraint exists.
- [Thanos Store Gateway documentation](https://thanos.io/tip/components/store.md/) (official) — index-header caching, the memory pools, and time/hash partitioning.

## Grafana

For the architecture these items configure, see [Grafana Architecture](../../dashboards/grafana/architecture/).

Grafana is the one component here that is not a data store, and that changes what "production" means for it.
Nothing observable is lost when Grafana breaks — the metrics are in Thanos and the logs are in Loki, and the dashboards this chart installs are re-pushed by grafana-operator every `resyncPeriod`.
What *is* at risk is everything a human created through the UI, plus the fact that Grafana is where an incident starts.

The chart defaults are the safe shape, not the production shape.
Three things separate them, and they compound: state, reachability, and authentication.

| | Default | Production |
|---|---|---|
| State | SQLite on `emptyDir`, lost on every restart | External PostgreSQL |
| Reachability | `ClusterIP`; `kubectl port-forward` only | Ingress, TLS-terminated, internal by default |
| Authentication | The generated admin password | An identity provider, with group-mapped roles |
| Replicas | 1 | 2+ behind an HPA — only meaningful once state is external |

**Reachability and persistence are one decision, not two.**
Exposing Grafana without a durable backend turns a bundled extra nobody depended on into the primary interface to the stack — one that silently discards every dashboard, annotation, and API token its users create.
The chart warns at exactly that combination rather than on every install.

### Three shapes, and what each one costs

| Backing store | Set with | Replicas | Suitable for |
|---|---|---|---|
| SQLite on `emptyDir` (**default**) | — | 1 | demos and `kind`; state is lost on every pod restart |
| SQLite on a PersistentVolume | `grafana-pvc` profile | 1 | a single small instance with no database available |
| External PostgreSQL | `grafana-postgres` profile | 2+ | production |

SQLite tolerates exactly one writer, so both SQLite options pin you to a single replica — more than one is a correctness bug rather than availability, and the chart refuses to render it.
A `ReadWriteOnce` volume additionally forces `deploymentStrategy.type: Recreate`, because a rolling update deadlocks: the replacement pod waits for a volume the outgoing pod has not released, and the outgoing pod is not terminated until the replacement is Ready.
External PostgreSQL is the only option that lifts both constraints.
See [State and persistence](../../dashboards/grafana/architecture/#state-and-persistence) for the full wiring, including why IAM database authentication does not remove the need for a password secret.

### Checklist

#### 1. State

- [x] `[chart]` A render-time check **errors** on more than one replica — including an HPA ceiling above one — without `grafana.ini.database` pointed at a shared database.
- [x] `[chart]` A render-time check **errors** on a `ReadWriteOnce` volume paired with a rolling update, and on a volume asked to back several replicas.
- [x] `[chart]` A render-time check **warns** when an exposed Grafana keeps its state on an `emptyDir`, and on a database connection with `ssl_mode` unset or `disable`.
- [x] `[chart]` `grafana-postgres` and `grafana-pvc` profiles ship the assembled shapes.
- [ ] `[consumer]` Provision the **database and an owning user**. Grafana runs its own schema migrations at startup, so a read/write-only grant fails the migration.
- [ ] `[consumer]` Provide the database password as a **Secret**, in the namespace the Grafana *pod* runs in — under `split-namespace` that is `grafana`, not the release namespace. The chart consumes it by name; it does not mint it.
- [ ] `[operator]` Never inline the password into `grafana.ini` — it renders into a **ConfigMap**. Use `$__file{}` against a mounted Secret, or `$__env{}` against `envValueFrom`. The subchart's `assertNoLeakedSecrets` check fails the render if you forget; leave it on.
- [ ] `[operator]` **Back the database up.** It holds every service-account token, alert rule, and annotation. Nothing in this chart replicates it, and a PVC snapshot is the only copy on the `grafana-pvc` path.
- [ ] `[operator]` Switching an existing install from SQLite to PostgreSQL **does not carry state over** — Grafana has no migration between them. Export what matters through the HTTP API first.

#### 2. Reachability

- [x] `[chart]` `grafana.ingress` and `grafana.service` are surfaced, so a Helm-only install has the same capability the Terraform path does.
- [x] `[chart]` **Internal by default:** the Service is `ClusterIP` and no Ingress is rendered.
- [x] `[chart]` **Public requires an allowlist, enforced.** A `LoadBalancer` Service with no `loadBalancerSourceRanges` — and a `NodePort`, which has no allowlist mechanism at all — is a render-time **error**. `connections.grafana.allowPublicAccess: true` downgrades it to a warning; it is an acknowledgement, not a silencer.
- [ ] `[consumer]` **Choose the protocol layer deliberately.** A `LoadBalancer` Service gives you L4 on every cloud and terminates no TLS; an Ingress gives you L7, terminates TLS, and needs a controller you may not have. L7 is what a public Grafana wants, because a WAF and edge authentication are the two things L4 cannot do — but L4 has no request-timeout ceiling, which matters for the long Thanos and Loki queries a panel makes. The `grafana-ingress` profile is the L7 shape; the Terraform wrappers use the Service. See [Ingress and Service are not interchangeable](../../dashboards/grafana/architecture/#ingress-and-service-are-not-interchangeable).
- [ ] `[consumer]` **Terminate TLS in front of Grafana.** It authenticates with a session cookie; without TLS that cookie and the admin password cross the network in the clear. Either an Ingress `tls` block or termination at the load balancer against a cloud-held certificate (ACM on AWS, a managed certificate on GKE). A render-time check warns on an Ingress with no `tls` — it cannot see the second case, so confirm rather than dismiss it. A bare `LoadBalancer` Service with no controller in front is the one shape with no TLS anywhere.
- [ ] `[operator]` **Supply the certificate or the issuer.** The consumer wires it; where the trust comes from — a cert-manager `ClusterIssuer`, an ACM ARN, an uploaded certificate — is a policy decision.
- [ ] `[consumer]` Set `security.cookie_secure: true` **once TLS is in place** — before that the session cookie is never sent and nobody can log in.
- [ ] `[operator]` Set **`grafana.ini.server.root_url`** to the URL users actually reach. Share links, alert notification links, and OAuth redirect URIs are all built from it, and all three break silently when it disagrees with the host. Checked at render time.
- [ ] `[operator]` **Create the DNS record.** Neither the Ingress nor the Service publishes the hostname, and the chart cannot: it has no view of your zone. Unless something like external-dns is already reconciling records in the cluster, this is a manual step that is easy to forget until the certificate fails to issue.

#### 3. Authentication and authorization

See [Authentication](../../dashboards/grafana/auth/) for the wiring.

- [ ] `[operator]` **Configure an identity provider** under `grafana.ini` before exposing Grafana to anyone but yourself. Any `auth.*` section Grafana supports works. A render-time check warns when an exposed Grafana has none.
- [ ] `[consumer]` Provide the OAuth **client secret as a Secret**, referenced with `$__file{}` or `$__env{}`. Never a literal in `grafana.ini` — that renders into a ConfigMap.
- [ ] `[operator]` Map an IdP group claim onto Grafana roles with **`role_attribute_path`**, so group membership does the provisioning and nobody is added by hand. Set `role_attribute_strict: true` once it is right, so a broken claim fails the login instead of quietly falling back to `Viewer`.
- [ ] `[operator]` Keep the **local admin as break-glass**. `disable_login_form: true` still leaves `/login?disableAutoLogin` reachable; know that before an IdP outage takes your incident tooling with it.
- [x] `[chart]` A render-time check **errors** on `auth.anonymous` enabled while Grafana is exposed, unless `connections.grafana.allowPublicAccess` says the exposure is deliberate.
- [ ] `[operator]` Rotate the generated admin password, or supply your own with `grafana.admin.existingSecret`. Grafana reads it once at startup, so a rotation needs a restart.
- [ ] `[operator]` Grafana's own permissions are **not a data boundary**. Every datasource is queryable by anyone who can reach it, so a Viewer in Grafana still reads every metric in Thanos and every log in the tenant. See [Tenancy & auth](#9-tenancy--auth).

#### 4. Availability & sizing

- [x] `[chart]` Resource **requests and limits** are set (100m / 256Mi requests, 1Gi memory limit). No CPU limit: query rendering is bursty and throttling it makes the UI feel broken.
- [x] `[chart]` **PodDisruptionBudget** (`maxUnavailable: 1`), matching the Loki and Thanos convention — it scales with the replica count, and on a singleton `minAvailable: 1` would permit no eviction and hang node drains. A validator warns on `minAvailable` for a single replica, and on several replicas with no budget.
- [x] `[chart]` **HPA** surfaced and off by default, because it is meaningless on SQLite. The `grafana-postgres` profile turns it on (2–5 replicas, 60% CPU). Note the subchart stops rendering a static `replicas` once an HPA exists, so `minReplicas` becomes the effective floor.
- [x] `[chart]` A validator warns when `autoscaling.targetCPU` is set with no CPU request — the HPA measures utilization against the request, so with no request there is no denominator and it never scales.
- [x] `[chart]` **Probes** on `/api/health` (liveness and readiness) come from the subchart and are left at their defaults.
- [x] `[chart]` Grafana-managed **unified alerting is not HA out of the box** — each replica evaluates every rule independently and notifies separately. The `grafana-postgres` profile enables gossip (`headlessService: true` plus `unified_alerting.ha_peers`), and validators warn both on several replicas with no gossip and on `ha_peers` pointing at a headless Service that was never created. Gossip is *not* a chart default: at one replica it is inert and costs a `ha_peer_timeout` settle on every start. The Prometheus rules this chart ships are unaffected either way.
- [ ] `[operator]` Gossip needs pod-to-pod **9094 on TCP and UDP**. A NetworkPolicy that blocks it makes notifications duplicate rather than fail, because the replicas simply never find each other.
- [x] `[chart]` `priorityClassName` on Grafana and grafana-operator (`monitoring-scalable`), so neither is evicted ahead of ordinary workloads — Grafana is where an incident starts. See [Scheduling priority](#scheduling-priority).

#### 5. Images & supply chain

- [x] `[chart]` The Grafana image is **pinned explicitly in `values.yaml`** (registry, repository, tag) rather than tracking the subchart's `appVersion`, so Renovate bumps the server on its own cadence instead of only when a chart release happens to carry one.
- [ ] `[operator]` **Hardened base images** are a drop-in swap: point `image.registry`/`image.repository` at one and keep the tag. Published options that track upstream Grafana versions are [Docker Hardened Images](https://docs.docker.com/dhi/) (subscription; images land in your own org namespace), [Chainguard Images](https://images.chainguard.dev/directory/image/grafana/overview), and [Bitnami Secure Images](https://github.com/bitnami/containers/tree/main/bitnami/grafana) — note the versioned hardened Bitnami tags moved behind a subscription, and the free `docker.io/bitnami` namespace is not the same thing. All of them ship no shell and no package manager, which is the point, and which means start-time plugin installation cannot work — bake plugins into the image instead.
- [ ] `[operator]` **Pin plugin versions** (`name@version`) or bake them in. `grafana.plugins` downloads from grafana.com at every pod start, which is both a startup dependency on a third-party service and a way for a plugin to change underneath a pinned Grafana. A validator warns on an unpinned entry.
- [x] `[chart]` **Image Renderer disabled**, and a validator warns when it is turned on. It is a headless Chromium that fetches URLs on Grafana's behalf — a large attack surface and a server-side request forgery pivot into the cluster network. It has no place in a production deployment.
- [x] `[chart]` The subchart's `testFramework` hook is off; it pulls a `bats` image this chart does not otherwise use or pin.

#### 6. Configuration & dashboards as code

- [x] `[chart]` **grafana-operator manages dashboards and datasources as code** — `GrafanaManifest` and `GrafanaDatasource` resources, reconciled in one direction with Kubernetes as the source of truth. Anything the operator owns is re-pushed on drift; anything it does not is yours to keep.
- [x] `[chart]` `grafana.ini` is a **verbatim passthrough**, so any section Grafana understands — auth, SMTP, feature toggles, unified alerting, user provisioning — is reachable from values and deep-merges over the chart's own keys.
- [x] `[chart]` `connections.grafana.operator.spec` is the **break-glass** for `mode: operator`, where grafana-operator owns the server lifecycle and none of the above applies. Prefer `mode: bundled`.
- [ ] `[operator]` Treat the running Grafana as a **cache, not a source**. Dashboards edited in the UI are overwritten at the next resync; change them in `packages/grafana-dashboards/` instead.

#### 7. Network & meta-monitoring

- [x] `[chart]` ServiceMonitor for Grafana's own metrics.
- [ ] `[chart]` **NetworkPolicy** — the subchart ships `networkPolicy` values but the chart neither enables nor opinionates them yet ([DEP-192](https://linear.app/materializeinc/issue/DEP-192)). Grafana needs ingress from the operator and from whatever fronts it, and egress to Thanos Query, the Loki query frontend, its database, and its identity provider.
- [x] `[chart]` `analytics.reporting_enabled` and `check_for_updates` are off — egress a monitoring stack does not need, and update banners are noise when the version is pinned by the chart.
- [ ] `[operator]` Alert on Grafana being down. It is not a data-loss incident, but it is the interface every other alert is investigated through.

### See also

- [Grafana Architecture](../../dashboards/grafana/architecture/) — connection modes, namespaces, and the resource map.
- [State and persistence](../../dashboards/grafana/architecture/#state-and-persistence) — the PostgreSQL wiring in depth.
- [Grafana configuration reference](https://grafana.com/docs/grafana/latest/setup-grafana/configure-grafana/) (official) — every `grafana.ini` key.
