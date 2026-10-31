# Stack-wide settings




# Stack-wide settings

This page explains the settings that cut across every component: the namespace layout, scheduling priority, network
policies, image registries, and the zone count.
What to do about each one is in the [production checklist](../checklist/), ordered by what goes wrong if you skip it.

## Namespace layout {#namespace-layout}

Each subchart uses a deterministic name, a static `fullnameOverride` such as `loki` or `thanos`, so no resource carries a release-name prefix.
That assumes **one instance of each backend per namespace**, which is a reasonable constraint for an umbrella infrastructure chart.
Two layouts are supported; both are fine, so pick per your isolation needs.

**Shared namespace (default).**
Everything deploys into the release namespace (recommended: `monitoring`).
This is the default and the path most installs should take.

**Split namespace.**
One namespace per subchart, via the `split-namespace` profile
(`-f charts/materialize-monitoring/profiles/split-namespace.values.yaml`), which sets a `namespaceOverride` per
component (`loki`, `thanos`, `grafana`, …).
Support is best-effort.

| | Shared (default) | Split |
|---|---|---|
| Ops overhead | low — one namespace, one RBAC scope, simpler NetworkPolicy and install | higher — N namespaces, cross-namespace NetworkPolicy/DNS, per-namespace bindings |
| Isolation / trust | components share a trust boundary; larger blast radius | least-privilege between components; contained blast radius |
| Per-component quotas / RBAC | coarse | fine-grained |
| Support | primary | best-effort |

> [!INFO]
>   The layout changes the namespace half of every workload-identity binding: an IRSA / GKE / Azure trust-policy subject
>   targets `system:serviceaccount:<namespace>:<sa>`, where `<namespace>` is the release namespace under the shared
>   layout and the per-component namespace under split.
>   The ServiceAccount name is unaffected (it's the deterministic `fullnameOverride`).
>   See [Logs &amp; Events &gt;
>   Storing](../../../logs-and-events/storing/#granting-object-storage-access-workload-identity) and [Metrics &gt;
>   Storing](../../../metrics/storing/#thanos-object-storage).

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
Priority still decides scheduling-queue order and, more importantly, which pod the kubelet evicts first under node
pressure — but monitoring will never evict a Materialize pod to make room for itself.
Monitoring that takes down the thing it monitors is worse than monitoring that is late.
Both sit well below `system-cluster-critical` (2000000000) and `system-node-critical` (2000001000), so cluster plumbing still outranks the stack.

## Network policies {#network-policies}

Every workload in the stack carries a NetworkPolicy, and every one of them is on by default.
Most come from the subchart that owns the workload; the three subcharts that ship no policy of their own get one
rendered by this chart, from `templates/networkpolicies.yaml` and configured under `networkPolicies`.

| Component | Configured at | Ingress | Egress |
|---|---|---|---|
| `alloy-agent` | `alloy-agent.networkPolicy` | `12345` from the gateway | unrestricted |
| `alloy-gateway` | `alloy-gateway.networkPolicy` | `3100`/`4317`/`4318`/`9090` from any pod in the cluster; `12345` from itself | unrestricted |
| `loki` | `loki.networkPolicy` | in-namespace, plus the configured namespace selectors | in-namespace, DNS, object storage |
| `thanos` | `thanos.global.networkPolicies` | each component's own service ports, from anywhere | unrestricted |
| `grafana` | `grafana.networkPolicy` | `3000` from anywhere | unrestricted |
| `grafana` (gossip) | `networkPolicies.grafanaGossip` | `9094` TCP+UDP between Grafana pods | — |
| `kube-state-metrics` | `kube-state-metrics.networkPolicy` | `8080`/`8081` from the gateway | DNS and the API server only |
| `node-exporter` | `node-exporter.networkPolicy` | `9100` from the gateway | denied |
| `alertmanager` | `networkPolicies.alertmanager` | `9093`/`9094` in-namespace | in-namespace, DNS, `443`/`587`/`465` outbound |
| `grafana-operator` | `networkPolicies.grafana-operator` | `9090` in-namespace | in-namespace, DNS, `443`/`6443` outbound |
| `metrics-server` | `networkPolicies.metrics-server` | `10250` from anywhere | in-namespace, DNS, `10250`/`443`/`6443` outbound |

**Ingress is tight and egress mostly is not**, and that asymmetry is deliberate rather than unfinished.
Ingress is the direction that protects these endpoints — unauthenticated metrics, log-push, and query APIs — its allowed
peers are knowable from the chart, and a wrong rule fails loudly at the first scrape.
Egress destinations are cluster- and account-specific: the API server's address, kubelet ports on node IPs, an
object-storage endpoint, a notification provider, an external Grafana.
A wrong egress rule fails silently, minutes or hours later, somewhere other than the pod that changed.
So egress is narrowed only where the chart genuinely knows the destination set — `node-exporter` (nothing),
`kube-state-metrics` (DNS and the API server), `loki` (in-namespace, DNS, object storage) — and left open elsewhere,
with `egress.extra` and the subcharts' own egress lists as the place to narrow it against facts only your cluster has.

The gateway is the clearest case for why: it scrapes the kubelet cAdvisor endpoint on every node — port `10250` on a
*node* IP, which no `podSelector` matches — and it discovers `ServiceMonitor` and `PodMonitor` targets cluster-wide, so
its destination set is every pod and port anyone has ever pointed a monitor at, plus whatever external backend `connections` names.

## Images and registries {#images-and-registries}

Every image in the stack is pinned by registry, repository, and tag: in this chart's `values.yaml` where it matters enough to own the cadence,
and in a subchart's defaults otherwise.
Repointing at a mirror or a hardened rebuild is therefore a values change rather than a fork.
Four overlays under `profiles/registry/` do exactly that.
`mirror` moves every image to a private mirror without changing any of them, which is the common case and the only one that can carry the Alloy build.
`chainguard` and `docker-hardened-images` swap in a hardened rebuild — one per vendor that publishes the whole set *and*
keeps the upstream entrypoint and layout ([Chainguard Images](https://images.chainguard.dev/directory), [Docker Hardened
Images](https://docs.docker.com/dhi/)).
`pull-secret` carries the credential half on its own and composes with any of them.

Bitnami Secure Images is deliberately absent — it rebuilds around its own charts, so it is a port rather than a retag;
see [Getting Started > Helm](../../../getting-started/helm/#no-bitnami-profile) for what that costs.
If your mirroring happens in containerd or via an `ImageDigestMirrorSet`, none of this applies: manifests keep naming
the upstream registry and the redirect is invisible to Helm.
See [Getting Started > Helm](../../../getting-started/helm/#registry-profiles) for the composition order.

### No single global reaches every subchart {#pull-secret-globals}

This is the part that looks solved and is not.
Helm propagates `global` into every subchart, but each subchart decides for itself whether to read it, and these do not agree:

| Key | Reached by |
|---|---|
| `global.image.pullSecrets` | `alloy-agent`, `alloy-gateway` |
| `global.imagePullSecrets` | `thanos`, `grafana`, `kube-state-metrics`, `node-exporter` |
| *neither — needs its own key* | `loki`, `grafana-operator`, `alertmanager`, `metrics-server` |

Setting only `global.imagePullSecrets` therefore leaves Loki — the largest pod count in the release — pulling anonymously.
It does not fail the render or the install: it surfaces as `ImagePullBackOff` on the ingesters some minutes after `helm install` reports success.
The `pull-secret` profile sets all three groups, which is the whole reason it exists as a file rather than as two `--set` flags.

The matching trap on the registry side is `global.imageRegistry`.
Loki's image helper coalesces it **ahead of** every per-component `image.registry`, so setting it to reach one mirror
silently flattens all of Loki's images — memcached and the canary included — onto that host and overrides whatever else was configured.
Set per-component registries instead.

### Where a retag is not enough

Hardened images keep the upstream binary and drop everything around it, which is the point and also the failure mode:

- **The UID rarely matches the chart's.**
  Grafana's chart writes `runAsUser: 472` and Loki's `10001`, while a hardened rebuild runs as whatever the vendor chose.
  A mismatch renders and schedules cleanly, then crash-loops on a volume the container cannot write.
  Check the image's UID and align the relevant `securityContext` before pointing one at a cluster that holds data.
- **No package manager means no start-time plugin installation.**
  See [Grafana plugins](../grafana/#plugins).
- **A vendor outside the two profiled here may not be a retag at all.**
  Both shipped profiles keep the upstream entrypoint, arguments, and layout, which is what makes them a values change.
  A vendor that rebuilds around its own charts is a port, and the render can fail before you get as far as the UID — a
  tag that is not plain semver breaks `prometheus-node-exporter` outright unless `node-exporter.version` is set alongside it.

### The three hook Jobs {#image-pull-secret-gaps}

The `pre-delete` cleanup hook and the two Alloy pre-install validator Jobs are workloads this chart renders itself
rather than subchart pods, so they take their pull secrets from both globals directly and need nothing extra set.
Each also accepts a local override appended to the globals — `cleanup.grafanaOperator.image.pullSecrets`, and for the
validators the Alloy pods' own `alloy-{agent,gateway}.image.pullSecrets`, since they run those images.
Names are deduplicated across all of them, so pointing every path at one Secret gives one entry.

The cleanup hook is worth confirming by hand.
It runs at `pre-delete`, so an image it cannot pull does not fail an install you can retry — it hangs `helm uninstall`
on the hook, with the operator still up and the release half-removed.
`helm uninstall --no-hooks` is the way out, and needing it is a worse day than checking first.

## Fewer than two zones {#fewer-than-two-zones}

Loki's ingesters, Thanos Receive, and Alertmanager spread across zones with a hard constraint, so that replication survives losing a zone.
For how that constraint is built, see [Spreading across zones](../thanos/#thanos-topology-spread).
It assumes at least two availability zones, which is every managed cloud default and not much else.
Below that it **fails closed**: the stack doesn't come up, and the cause reads like a chart bug rather than a property of the cluster.

| Zones | What happens by default | Fix |
|---|---|---|
| 0 (no labels) | Receive and Loki's ingesters are **unschedulable** — a `DoNotSchedule` constraint whose `topologyKey` no node carries has no domain to place into | `min_zones = 0`, or the `no-zone-spread` profile |
| 1 | **Also Pending**, and much less obvious — see below | `min_zones = 1`, or the same profile |
| 2 | Works. The chart's floor | nothing |
| 3+ | Works, but under-protects: `minDomains: 2` is satisfied by a two-zone placement | `min_zones = 3` (or your real count) |

**One zone is exactly as broken as none, and that surprises people.**
When the number of eligible domains is below `minDomains`, Kubernetes treats the global minimum as 0 rather than
skipping the check — so a single zone holding all three replicas computes a skew of 3 against a `maxSkew` of 1, and every pod stays `Pending`.
Nothing about the symptom points at a zone count.
Alertmanager's zone rule sets no `minDomains`, so it schedules on a single zone, but with no zone labels both of its replicas stay `Pending` too.

Zero-zone clusters are not an edge case worth dismissing: `kind` labels no node this way, and neither do many on-premises distributions.
Hand-labelling nodes to satisfy a monitoring chart is the wrong answer, which is why `min_zones` exists.

Both fixes drop the hard zone rule and **keep the soft host rule**, which matters most on exactly these clusters — with
no zones to lose, the node is the only failure domain there is.
On the Terraform path that falls out for free, because `min_zones` filters the one constraint rather than replacing the
list; on the Helm path the profile has to restate the host rule, since Helm overwrites lists rather than merging them.

> [!WARNING]
>   **Nothing reminds you when you gain a zone.**
>   The pods keep scheduling happily, spread across nothing, and RF 3 goes on reading like AZ resilience on every dashboard.
>   `min_zones` is the better habit than the profile for this reason alone: it is a number you update when the cluster
>   changes, not a file you have to remember to stop passing.

