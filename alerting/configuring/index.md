# Configuring




# Configuring Alerting

A default install runs two rule evaluators and one notifier.
Thanos Ruler evaluates PromQL, Loki Ruler evaluates LogQL, and both send what they produce to a single Alertmanager.

**The evaluators, the notifier and a default set of bundled rules are all installed.**
Which bundled rules install is configured under `rules`, and where an alert goes under `alerting`.
Until a receiver is configured every alert reaches `mzmon-null`, which notifies nobody.
This page describes the bundled rules, the two evaluators, and how they reach Alertmanager.
[Alert Architecture](../architecture/) describes Alertmanager itself, and [Alert Channels](../channels/) describes routing and receivers.
The remaining work is described in the [alerting design doc](../../reference/internal/design-docs/20260917-alerting-self-managed/) (internal) and tracked under [DEP-216](https://linear.app/materializeinc/issue/DEP-216).

<!-- more -->

<blockquote class="book-hint note">
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in
<a href="https://datatracker.ietf.org/doc/html/rfc2119" rel="external" class="external-link">RFC 2119</a>.
</blockquote>


## The bundled rules

The chart ships alerting rules for Materialize and for the platform under it.
The metric rules are installed as `PrometheusRule` resources, which the Thanos ruler evaluates.
The log-derived rules, which detect panics and correctness violations in Materialize's log lines, are `PrometheusRule` resources too, and the alloy-gateway writes them into the Loki ruler.
[Common Alerts](../../reference/common-alerts/) lists every one, with what it detects.

A rule installs when all of the following hold:

| Condition | Configured by |
|---|---|
| Rules are enabled | `rules.enabled`, default `true` |
| Every capability the rule requires is present | derived from what the chart deploys, plus `rules.capabilities` |
| The rule is in the default set, or selected | `rules.selected` takes alert names, rule-group names, or `*` |
| The rule is not disabled | `rules.disabled` takes alert names |
| A log-derived rule also needs the release's Loki ruler, the alloy-gateway, and a rule store the ruler API can write to | `loki.ruler.enabled`, `alloy-gateway.enabled`, `loki.loki.storage` |
| A rule reading a recorded series also needs one of the recording rules that produce it to install | see [Recorded series](#recorded-series) |

A **capability** is something a deployment contains that a rule needs in order to mean anything, such as a Cilium CNI or a CockroachDB metadata database.
The chart derives the capabilities for what it deploys itself: Materialize's own metrics, kube-state-metrics, cAdvisor, node-exporter, Loki and Alloy, and each cloud provider pull the gateway runs.
Everything else is listed explicitly; the full list and what each means are in the chart's `values.yaml` under `rules.capabilities`.
A selected rule whose capabilities are missing is still not installed, and the render says so.

The **default set** is the rules that have been evaluated against a live self-managed install without firing falsely.
The rest of the bundled rules are available through `rules.selected`, and SHOULD be evaluated against the deployment before being relied on.

Rules about a Materialize environment's pods scope to its namespaces.
The chart takes them from `rules.namespaces.environment`, falling back to `materialize.namespaces` and then to `materialize-system.namespace`.
A deployment whose environments live elsewhere MUST set one of these, or those rules match nothing.
`rules.namespaces.exclude` removes namespaces from alerting altogether.

The infrastructure rules grade a workload by its tier in `rules.infraWorkloads`: `core`, `important`, `nonessential`, and `daemonset` for what every node runs.
The defaults cover the Kubernetes add-ons common on EKS and GKE and this chart's own collectors.
A cluster with other add-ons SHOULD list them, since a workload in no tier is one the tiered rules never grade.
None of the tiered rules is in the default set: they are the `deployment`, `file_descriptors`, `infra_memory`, `infra_pod_health` and `daemonset` groups, and a tier has an effect once one of them is selected.
Each entry is a regex matched against the whole container or Deployment name.

Every bundled rule carries an `audience` label saying who acts on it.

| `audience` | Covers |
|---|---|
| `platform` | The Materialize deployment, its system clusters, and the Kubernetes platform under it |
| `workload` | What runs on the deployment: user clusters' freshness, hydration and sizing, and the sources feeding them |

An [extra route](../channels/#extra-routes) on `audience="workload"` sends the second group to the people who own the clusters.

`rules.overrides` changes an installed rule's `for` and labels, and never its expression.
A deployment whose clusters normally take hours to hydrate is the usual reason for one, since `cluster-hydration-stuck` fires after an hour by default.
An override names an alert, and the render fails on an alert that does not exist.

The Terraform module takes all of `rules.*` as its `alert_rules` input; see [Configuring Alerting through Terraform](../terraform/#tuning-the-bundled-rules).

`materialize.deploymentMode: cloud` switches the SQL-backed metric names the rules read to Materialize Cloud's `v2_mz_` prefix.

## Recorded series

The chart also ships recording rules, which the Thanos ruler evaluates into series of their own.
The first are the `ext:consensus_*` series, which describe the metadata database the same way on every database flavor.
[Recorded Series](../../reference/recorded-series/) lists every one.

A recording rule installs wherever its capabilities are present, whenever `rules.enabled` is true.
It is not in the default set and is never selected, so `rules.selected`, `rules.disabled` and `rules.overrides` do not apply to it.

Materialize's own view of its metadata database needs nothing configured.
The cloud provider's view needs two things: the provider pull, `pipeline.metrics.provider.*`, watching the database, and `externalDependencies.consensus` naming it as a metadata database.

```yaml
externalDependencies:
  consensus:
    - flavor: rds
      resourceId: mzmon-prod-db
```

The second is needed because a pull also watches databases that are not a metadata database, Grafana's among them, and the provider cannot tell them apart.
A database the pull watches and `externalDependencies.consensus` does not name is never recorded as a metadata database.
The render warns when a pull watches databases and none of them is named, and when a named database is not watched.

| `flavor` | `resourceId` |
|---|---|
| `rds` | The DB instance identifier, as listed in `pipeline.metrics.provider.cloudwatch.rds.instances` |
| `cloudsql` | The instance name, as listed in `pipeline.metrics.provider.gcp.cloudSql.instances` |
| `azure-postgres` | The flexible server name, as listed in `pipeline.metrics.provider.azure.postgres.servers` |

An alert can read a recorded series: `consensus-unreachable` reads `ext:consensus_up`, so it installs wherever any adapter recording that series does.

A recorded series that no bundled alert reads has no metric tier.
A metric destination filtering by `minMetricImportance` therefore drops it, and the render warns when that destination is the bundled Thanos.

## Two evaluators, one notifier

No component evaluates both PromQL and LogQL, which is why there are two.

| | Thanos Ruler | Loki Ruler |
|---|---|---|
| Language | PromQL | LogQL |
| Values key | `thanos.ruler` | `loki.ruler`, configured under `loki.loki.rulerConfig` |
| Reads from | Thanos Query | Loki |
| Rule source | `PrometheusRule` resources in the cluster, except `flavor: logql` | `PrometheusRule` resources labelled `flavor: logql`, written into its bucket by the alloy-gateway |
| Notifies | The bundled Alertmanager | The bundled Alertmanager |
| Rule results | Remote-written to the alloy-gateway | Remote-written to the alloy-gateway |

Alertmanager is the single notification surface.
An operator configuring where alerts go configures one thing, whether the alert came from a metric or a log line.

## Both rulers notify every Alertmanager replica

The bundled Alertmanager runs two replicas that gossip silences and the notification log to each other, but not alerts.
Each replica therefore has to receive every alert from the rulers directly.

So neither ruler notifies the load-balanced Service.
Both resolve the headless Service to one address per ready replica, the Thanos ruler through a `dns+` lookup and the
Loki ruler through an SRV lookup with `enable_alertmanager_discovery`.
A deployment overriding either address SHOULD keep that shape; one that points a ruler at the load-balanced Service
leaves a surviving replica without the alerts its peer was holding.
See [Why the rulers address every replica](../architecture/#every-replica).

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

Stateless mode is the subchart's `thanos.ruler.remoteWrite`.
With it, the ruler is passed `--remote-write.config-file` and no `--objstore.config-file`, so it runs no block shipper.
Setting `remoteWrite.enabled: false` reverts the ruler to a local TSDB, which the render warns about.

The remote-write configuration targets the alloy-gateway, and the chart renders it as a Secret that `remoteWrite.secretName` names.
The chart renders it rather than passing it through `remoteWrite.config`, because the gateway's address depends on the gateway's namespace and TLS mode, and the subchart cannot read either.
There are two Secrets, one per TLS mode, and `profiles/mtls.values.yaml` selects the TLS one.
A deployment MAY name a Secret of its own instead, or set `secretName: ""` with `createSecret: true` and its own `config`; the render's checks against the gateway then no longer apply.

Before the subchart modeled it, the chart reached stateless mode through `thanos.ruler.extraArgs` and a `remote-write` entry in `extraVolumes` and `extraVolumeMounts`.
An override that still restates either entry fails the render, because the subchart now renders both itself: Thanos refuses the repeated flag, and the API server refuses the repeated volume name.

## Every bundled rule is a `PrometheusRule`

Both rulers' rules are installed as `PrometheusRule` resources, and the label `mzmon.materialize.cloud/flavor` says which ruler each is for.

| Flavor | Read by | Delivered to |
|---|---|---|
| `promql` | The Thanos ruler's import sidecar | The Thanos ruler's rule directory |
| `logql` | The alloy-gateway's `loki.rules.kubernetes` | The Loki ruler's rule store, through its API |

A deployment's own `PrometheusRule` reaches the right ruler the same way.
One without the label is PromQL, and one labelled `logql` is LogQL.

## Rules reach the Thanos Ruler through an import sidecar

A sidecar lists `PrometheusRule` resources, writes each one's spec into the ruler's rule directory, and triggers a reload when the set changes.
No Prometheus Operator controller is involved, and the chart installs the CRDs it needs.

**The sidecar imports every `PrometheusRule` in the cluster except the LogQL ones.**
That is deliberate: a `PrometheusRule` applied by an operator or by another chart works with no configuration here.
The cost is that a co-resident rule owner — a kube-prometheus-stack, for instance — has its alerts evaluated by this ruler and notified through this Alertmanager.
Where that is not wanted, `thanos.ruler.autoImportPrometheusRules.labelSelector` SHOULD be set to narrow the imported set.

**The exclusion is load-bearing.**
The Thanos ruler reads every file it imports as PromQL, so one LogQL file fails its reload, and it keeps evaluating the rules it loaded before.
The default selector is `mzmon.materialize.cloud/flavor!: logql`.
The subchart joins the selector map into `key=value` pairs, so a key ending in `!` renders as `key!=value`, which also keeps a `PrometheusRule` carrying no flavor label.
A selector replacing the default MUST keep that key, or select `mzmon.materialize.cloud/flavor: promql`; the render fails otherwise.

**A Prometheus Operator in the same cluster may refuse the LogQL rules.**
Its Prometheus selects rules by its own `ruleSelector`, so it does not evaluate them.
Its admission webhook, where one is installed, validates every `PrometheusRule` as PromQL, and rejects a LogQL one when it is applied.
A cluster running one MUST exclude `mzmon.materialize.cloud/flavor: logql` through the webhook's `objectSelector`, or the install fails.

## Rules reach the Loki Ruler through its API

The alloy-gateway's `loki.rules.kubernetes` watches `PrometheusRule` resources labelled `mzmon.materialize.cloud/flavor: logql`, in every namespace.
It writes each one's groups into the Loki ruler through the ruler API, which stores them in the bucket named by `loki.loki.storage.bucketNames.ruler`.
Its Loki rule namespaces are prefixed `mzmon`.
It removes the ones whose `PrometheusRule` is gone and leaves every other namespace alone, so a rule written to the API by anything else survives.

Every gateway replica runs the sync; the writes set whole rule groups, so the replicas converge on the same set.
The gateway addresses the Service that serves the ruler in Loki's deployment mode: `loki-ruler` when distributed, `loki-backend` when simple-scalable, and `loki` for a single binary.
It uses the scheme and TLS settings of its Loki destination, `pipeline.logging.gateway.destination.loki.tls`, and the chart's Loki certificate names `loki-ruler`.

A log-derived rule installs only where the release runs the Loki ruler and the gateway, and where the ruler's rule store accepts writes.
A `local` store does not; that is what a filesystem-only Loki gets, such as the `loki-test` profile.
There the log-derived rules are left out and the render warns.

**A Loki rule group belongs to one tenant**, and the ruler does not evaluate across tenants.
The gateway writes the log-derived rules into each tenant in `rules.logTenants`, which defaults to `pipeline.logging.tenancy.staticTenant`.
Under the default `static` tenancy every line is in that tenant, and nothing needs setting.
Under `byEnvironment` each environment's logs are in a tenant named for its environment id, which the chart cannot discover; those tenants MUST be listed in `rules.logTenants` for the rules to see them, and the render warns while none are.
Under `byNamespace` the tenant set changes as namespaces are created, so no list stays complete.
Deployments that need complete log alerting SHOULD use `static` or `byEnvironment`.

## What an operator configures today

| Setting | Default | What it does |
|---|---|---|
| `thanos.ruler.enabled` | `true` | Deploys the PromQL evaluator. Turning it off makes every `PrometheusRule` in the cluster inert |
| `thanos.ruler.query.urls` | Thanos Query | Deliberately Query rather than Query Frontend. See below |
| `thanos.ruler.alertmanagers.config` | Every replica of the bundled Alertmanager, by `dns+` lookup | Thanos's own Alertmanager configuration format, passed through |
| `thanos.ruler.autoImportPrometheusRules.labelSelector` | `mzmon.materialize.cloud/flavor!: logql` | Which `PrometheusRule` resources to import. The default is all but the LogQL ones |
| `loki.ruler.enabled` | `true` | Deploys the LogQL evaluator |
| `loki.loki.rulerConfig.alertmanager_url` | Every replica of the bundled Alertmanager, by SRV lookup | Clearing it leaves the ruler evaluating recording rules and discarding alerts. The `_http._tcp.` form requires `enable_alertmanager_discovery: true` |
| `loki.loki.rulerConfig.evaluation_interval` | `1m` | How often the Loki ruler evaluates |
| `rules.enabled` | `true` | Installs the bundled rules that apply |
| `rules.capabilities` | `[]` | Capabilities beyond those the chart derives |
| `rules.selected` / `rules.disabled` | `[]` | Bundled rules to add to, or remove from, the default set |
| `rules.overrides` | `{}` | Per-alert `for` and labels, for metric and log rules alike |
| `rules.logTenants` | the static tenant | Loki tenants the gateway writes the log-derived rules into |
| `rules.namespaces.*` | derived | Where Materialize runs, and which namespaces never alert |
| `rules.infraWorkloads.*` | Common EKS and GKE add-ons | Which infrastructure workloads are core, important, non-essential, or on every node |
| `externalDependencies.consensus` | `[]` | Which databases a provider pull watches are a metadata database, for the `ext:consensus_*` series |
| `alerting.*` | No receivers | Where alerts go. See [Alert Channels](../channels/) |

Either ruler MAY be pointed at an Alertmanager the deployment already runs, by overriding its URL.
The bundled Alertmanager can then be excluded with `alertmanager.enabled: false`.
`tags.alertmanager: false` alone does not exclude it, because the tags are OR'd and `tags.default` and `tags.bundled-backends` both include it.

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
| Triage of the rest of the bundled set | Most bundled rules are outside the default set until each is checked against a self-managed install |
| A behavioural test for log rules | Loki has no counterpart to `promtool test rules`, so each log rule is checked against a live install instead |
| Runbooks | Each alert's `runbook_url` points at its entry on [Common Alerts](../../reference/common-alerts/) until runbooks exist |
| A deadman's switch | Stopped evaluation is indistinguishable from nothing being wrong |
| Rollout-signal inhibition | Upgrade noise is suppressed by hand; see [Maintenance Windows](../maintenance/) |

Routing, receivers, grouping, inhibition and silences are configurable today; [Alert Channels](../channels/) and
[Maintenance Windows](../maintenance/) cover them.

