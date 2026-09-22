# Configuring




# Configuring Alerting

A default install runs two rule evaluators and one notifier.
Thanos Ruler evaluates PromQL, Loki Ruler evaluates LogQL, and both send what they produce to a single Alertmanager.

**The evaluators are wired; the rules and the routing are not built yet.**
`pre-rendered/rules/` ships empty, and the bundled Alertmanager carries no routing tree, so an alert that fires today reaches a null receiver.
What this page describes is the path an alert will travel, and the parts of it an operator configures now.
The remaining work is tracked under [DEP-216](https://linear.app/materializeinc/issue/DEP-216).

<!-- more -->

<blockquote class="book-hint note">
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in
<a href="https://datatracker.ietf.org/doc/html/rfc2119" rel="external" class="external-link">RFC 2119</a>.
</blockquote>


## Two evaluators, one notifier

No component evaluates both PromQL and LogQL, which is why there are two.

| | Thanos Ruler | Loki Ruler |
|---|---|---|
| Language | PromQL | LogQL |
| Values key | `thanos.ruler` | `loki.ruler`, configured under `loki.loki.rulerConfig` |
| Reads from | Thanos Query | Loki |
| Rule source | `PrometheusRule` resources in the cluster | The ruler's object-storage bucket |
| Notifies | The bundled Alertmanager | The bundled Alertmanager |
| Rule results | Remote-written to the alloy-gateway | Remote-written to the alloy-gateway |

Alertmanager is the single notification surface.
An operator configuring where alerts go configures one thing, whether the alert came from a metric or a log line.

## Both rulers depend on the query path

Thanos Ruler evaluates by issuing PromQL to Thanos Query over the network, and Loki Ruler queries Loki the same way.
Neither keeps a local copy of the data it evaluates against.

An outage in either query path therefore stops alert evaluation, and stopped evaluation is indistinguishable from nothing being wrong.
Thanos Query and the Loki read path SHOULD be sized with that dependency in mind, and an external heartbeat is the only reliable check that evaluation is still happening.

## The Thanos Ruler runs stateless

The ruler keeps no TSDB of its own.
It remote-writes `ALERTS`, `ALERTS_FOR_STATE` and every recording-rule result to the alloy-gateway, which is the same listener the collection agents write to.

This matters for three reasons.
Rule results reach every metric destination the gateway fans out to, rather than only the bundled Thanos.
The ruler holds no PersistentVolumeClaim.
And forwarding alert state off-cluster becomes possible at all, because that forwarding happens at the gateway.

Stateless mode is reached through `thanos.ruler.extraArgs`, because the upstream subchart models no `remoteWrite` key.
**That argument is load-bearing.**
A deployment that sets `thanos.ruler.extraArgs` MUST carry `--remote-write.config-file` forward; dropping it reverts the ruler to a local TSDB, which the render warns about but cannot prevent.

## Rules reach the Thanos Ruler as `PrometheusRule` resources

A sidecar lists `PrometheusRule` resources, writes each one's spec into the ruler's rule directory, and triggers a reload when the set changes.
No Prometheus Operator controller is involved, and the chart installs the CRDs it needs.

This is the only consumer of a `PrometheusRule` in this stack.
Alloy reads `ServiceMonitor` and `PodMonitor` and has no rule evaluator, and no Prometheus runs here.

**The sidecar imports every `PrometheusRule` in the cluster.**
That is deliberate: a `PrometheusRule` applied by an operator or by another chart works with no configuration here.
The cost is that a co-resident rule owner — a kube-prometheus-stack, for instance — has its alerts evaluated by this ruler and notified through this Alertmanager.
Where that is not wanted, `thanos.ruler.autoImportPrometheusRules.labelSelector` SHOULD be set to narrow the imported set.

## Loki rules come from object storage

The Loki ruler reads its rule groups from the bucket named by `loki.loki.storage.bucketNames.ruler`.
Nothing in the chart writes rules into that bucket yet.

A Loki rule group belongs to one tenant, and the ruler does not evaluate across tenants.
Where `pipeline.tenancy.tenantMap` selects `byNamespace`, every Materialize namespace becomes its own tenant and the tenant set changes as namespaces are created, so a rule set installed at deploy time covers only the namespaces that existed then.
Deployments that need complete log alerting SHOULD use `static` or `byEnvironment`.

## What an operator configures today

| Setting | Default | What it does |
|---|---|---|
| `thanos.ruler.enabled` | `true` | Deploys the PromQL evaluator. Turning it off makes every `PrometheusRule` in the cluster inert |
| `thanos.ruler.query.urls` | Thanos Query | Deliberately Query rather than Query Frontend. See below |
| `thanos.ruler.alertmanagers.config` | The bundled Alertmanager | Thanos's own Alertmanager configuration format, passed through |
| `thanos.ruler.autoImportPrometheusRules.labelSelector` | `{}` | Which `PrometheusRule` resources to import. Empty means all of them |
| `loki.ruler.enabled` | `true` | Deploys the LogQL evaluator |
| `loki.loki.rulerConfig.alertmanager_url` | The bundled Alertmanager | Clearing it leaves the ruler evaluating recording rules and discarding alerts |
| `loki.loki.rulerConfig.evaluation_interval` | `1m` | How often the Loki ruler evaluates |

Either ruler MAY be pointed at an Alertmanager the deployment already runs, by overriding its URL.
The bundled Alertmanager can then be excluded with `tags.alertmanager: false`, or `alertmanager.enabled: false`.

## The rulers do not follow the query frontend

Thanos Query Frontend splits and caches queries, which is correct for a dashboard and wrong for an evaluator.
A cached range served to a rule is an alert firing, or failing to fire, on data up to a cache TTL stale, and neither component reports that it happened.

So where `thanos.queryFrontend.enabled` is true, the Grafana datasource moves to the frontend and the ruler stays on Thanos Query.
That divergence is intentional and is applied by the `thanos-large` profile.

## Running the rulers across namespaces

The `split-namespace` profile places each subchart in its own namespace, which turns both ruler hops into cross-namespace traffic.
The profile opens Alertmanager to the ruler namespaces and retargets both rulers' addresses.

One hop it cannot close is the Loki ruler's egress.
The Loki subchart's Alertmanager egress policy selects a pod label that no pod carries in Distributed mode, and the subchart exposes no general egress hook, so the ruler is confined to same-namespace egress.
A deployment using `split-namespace` MUST either supply its own NetworkPolicy for the Loki ruler's egress or set `loki.networkPolicy.enabled: false` and police Loki from outside the chart.

## What is not built yet

| Missing | Consequence |
|---|---|
| The shipped rule set | Nothing fires unless an operator supplies rules |
| `gen-rules` | The alert definitions in the query registry render to [Common Alerts](../../reference/stable-metrics/common-alerts/) and are not deployed |
| Alertmanager routing, receivers, grouping, inhibition | A firing alert reaches a null receiver |
| Log-derived alert definitions | Panic and correctness detection is not yet expressible here |
| Runbook links | An alert with no stated action is half an alert |
| An Alertmanager scrape | Delivery failures are not observable |

[Alert Channels](../channels/) and [Maintenance Windows](../maintenance/) cover the parts of that list closest to an operator, and are stubs until the routing surface exists.

