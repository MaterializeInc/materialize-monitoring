# Alerting in Self-Managed: Evaluation, Routing, and Customer Extension

date: 2026-09-17



# Alerting in Self-Managed: Evaluation, Routing, and Customer Extension


<table class="param-table">
  <tbody>
        <tr>
          <th>agent</th>
          <td>Claude Opus 5</td>
        </tr>
        <tr>
          <th>author</th>
          <td>Heather Lapointe</td>
        </tr>
        <tr>
          <th>lastmod</th>
          <td>2026-09-17 00:00:00 &#43;0000 UTC</td>
        </tr>
        <tr>
          <th>publishdate</th>
          <td>2026-09-17 00:00:00 &#43;0000 UTC</td>
        </tr>
        <tr>
          <th>status</th>
          <td>Draft</td>
        </tr>
  </tbody>
</table>


This doc proposes that **a default install of this chart page somebody when Materialize breaks**, and describes what has to exist for that sentence to be true.

Three things stand between here and there, and they are independent problems that happen to share a workstream.

**Nothing evaluates rules.**
Every component needed to alert is in the chart, and no two of them are connected.
Alertmanager is deployed and receives nothing, the Loki ruler runs with an empty rule store, the Thanos ruler is off, and the values key that claims to install Prometheus rules is read by no template.

**The rules that exist are the wrong rules.**
The query registry carries 85 alert definitions ported from Materialize Cloud, and roughly a quarter of them name components a self-managed deployment does not run.
A rule set where one alert in four can never fire is not a starting point that can be trimmed; it is a starting point that has to be re-derived.

**A whole class of alert has never been code anywhere.**
Log-derived alerting — panics, correctness violations, data-corruption patterns — exists only as rules clicked into a Grafana UI, because Cloud's alert pipeline emits PromQL rule groups and has no path for LogQL.
Self-managed is the first deployment shape where that class can be defined, reviewed, and shipped like everything else.

The central claim is that **alerting is where composability has to stop meaning optional and start meaning replaceable.**
Every other component in this stack can be turned off and a customer's own put in its place, and turning them all off yields a stack that collects nothing, which is obviously broken.
Turning alerting off yields a stack that looks complete and is silent, which is not obviously anything.

<!--
Agent note: this doc records decisions and their *why*. When a decision lands in code, update the section and
check the matching row in "Chart-side prerequisites".

Four things here are easy to get wrong and are stated deliberately: that `PrometheusRule` has exactly one
consumer in this stack and it is off by default ("Nothing evaluates rules"); that the alerting path runs
through the query path and therefore fails with it ("The evaluator depends on the query path"); that severity
and urgency are different properties owned by different people ("Severity belongs to the alert"); and that
`$__range` does not survive the move out of Grafana ("The range trap"). Revise those rather than softening them.

No customer names, organization identifiers, or environment identifiers appear on this page, including in
examples. The survey of clicked-in rules that motivates "The class that has never been code" found several;
they are described by shape only, and they are the reason "Customer-specific rules never enter this
repository" is a Must rather than a Should.
-->

## Goals

Functional requirements, framed as value-first user stories.
Priority tags (**Must** / **Should** / **Could**) are relative to the first shipped version.

- **[Must] As an operator,** I want a default install to notify somebody when Materialize is unreachable, so that installing the monitoring stack is not the same as installing a dashboard viewer.
- **[Must] As an operator,** I want every rule that ships enabled to be one that can fire in my deployment, so that the rule list is not mostly a list of things that will never happen.
- **[Must] As an operator for whom Materialize is critical infrastructure,** I want a critical alert to reach a pager.
- **[Must] As an operator evaluating Materialize,** I want that same alert to reach a chat channel and nothing else, without editing a rule.
- **[Must] As an operator,** I want to configure the notification tools already in use — incident.io, Slack, PagerDuty, email, a webhook — so that adopting this stack does not mean adopting a new incident tool.
- **[Must] As an operator,** I want my own alert rules installed alongside the shipped ones, so that the deployment-specific alerting every real deployment needs has a supported home.
- **[Must] As a Materialize support engineer,** I want a customer-specific alert to be expressible without being committed to this repository, so that a public repository never carries a customer's name.
- **[Must] As an operator,** I want to retune, relabel, or disable a shipped rule from values, so that disagreeing with one default is not a reason to fork the chart.
- **[Must] As an operator,** I want an alert that fires when alerting itself has stopped working, so that silence and health are distinguishable.
- **[Must] As a security reviewer,** I want notification credentials to be referenced rather than inlined, so that a values file is not a place a Slack token can be committed.
- **[Should] As a Materialize support engineer,** I want log-derived alerts defined as code, so that the rules that catch data-correctness bugs are reviewed rather than clicked.
- **[Should] As an operator,** I want the alert noise from a Materialize upgrade suppressed by the rollout itself rather than by a wall-clock window, because the schedule is mine.
- **[Should] As an operator who already runs Alertmanager,** I want the rulers to notify the one I have.
- **[Should] As a maintainer,** I want the alert-state series to traverse the gateway, so that the call-home alerts level has something to forward.
- **[Should] As a maintainer,** I want one rule definition to serve the docsite, the chart, and Cloud, so that three surfaces cannot disagree about what an alert means.
- **[Could] As an operator,** I want a rule that references a metric my install does not collect to be excluded automatically, rather than shipped and silent.
- **[Could] As an operator,** I want alerts grouped so that one bad node produces one notification rather than forty.

## Technical BLUF

**Two evaluators, one notifier.**
Thanos Ruler evaluates PromQL and Loki Ruler evaluates LogQL, because they are different languages and no component evaluates both.
Both notify a single Alertmanager, which is the only place routing, grouping, inhibition, and silences are configured.

**Thanos Ruler runs stateless.**
It remote-writes its evaluation results to the Alloy gateway rather than keeping a TSDB, matching what the Loki ruler already does.
That puts `ALERTS` in Thanos *and* in front of the gateway's destination fan-out, which is what the call-home alerts level needs and what a ruler shipping blocks to object storage would not give.

**Severity is a property of the alert; urgency is a property of the deployment.**
Rules carry `severity`, which is what the condition means.
A single values key, `alerting.criticality`, selects one of three severity-to-route mappings, which is what the deployment has decided that means.
The same rule set serves a critical-infrastructure install and an evaluation install, and the difference is a routing table.

**The shipped rule set is re-derived rather than inherited.**
A rule ships enabled only when it can fire in a stock self-managed install, means the same thing in every install, and has a documented operator action.
Applicability is checked at build time against the metric registry rather than asserted by a hand-maintained label, because the hand-maintained label already exists and is already wrong.

**Extension is additive in four places**, each independently useful: extra rules, rule overrides, extra receivers, and extra routes.
Customer-specific alerting composes out of those and never enters this repository.

**Log-derived alerts become first-class**, defined in the query registry beside the metric ones and rendered into Loki ruler rule files.
This is the first place at Materialize where they can be.

**Grafana-managed alerting stays supported and is not the default**, because it makes Grafana's database load-bearing for alert state, and a Grafana that loses its database on restart is the shape this chart still ships by default.

## Non-goals

- **An incident-management product.** Alertmanager notifies; escalation, acknowledgement, and on-call rotation belong to whatever the notification reaches.
- **SLO definition and error budgets.** Worth having and a separate design; burn-rate alerting needs recording rules and a stated objective, neither of which exists yet.
- **Anomaly detection or forecasting.** Every rule here is a threshold over a signal with a documented meaning.
- **Alerting on Materialize's SQL-level introspection.** The alerts here read metrics and logs. `mz_internal` is reachable only from a SQL session and belongs to a different collection path.
- **Replacing Cloud's alerting.** Convergence is a goal for the *definitions*; the deployment mechanism stays Pulumi on that side for now.
- **Routing to Materialize.** Forwarding alert state to a Materialize-operated control plane is [call-home](../20260917-call-home-self-managed/), which depends on this and is not part of it.

## What exists today

The honest summary is that every component is present and none of them are wired to each other.

| Surface | State | Consequence |
|---|---|---|
| `packages/queries/materialize-alerts.yaml` | 30 alert definitions | Rendered to the docsite; not deployed |
| `packages/queries/infra-alerts.yaml` | 55 alert definitions | Rendered to the docsite; not deployed |
| `charts/materialize-monitoring/templates/alerts/` | `.gitkeep` | No template emits any alerting resource |
| `charts/materialize-monitoring/pre-rendered/rules/{prometheus,loki,thanos}/` | `.gitkeep` | Nothing is rendered into them; there is no `gen-rules` command |
| `config.rules.prometheus.enabled` | `true` | Read by no template |
| `config.rules.loki.enabled` / `config.rules.thanos.enabled` | `false` | Also read by no template |
| `config.alerts.enabled` | `true` | Also read by no template |
| `thanos.ruler.enabled` | `false` | The PromQL evaluator is off |
| `loki.ruler.enabled` | `true`, 2 replicas, 5Gi PVC | Running. The chart sets no `loki.rulerConfig`, so it has no Alertmanager URL and no rules |
| `tags.alertmanager` | `false`, but in `bundled-backends` and `default` | Deployed by a default install. Configured with a priority class and a 4Gi PVC, and nothing else |
| Alertmanager scrape | Absent | The component is emitting metrics that reach nothing. The roadmap already records this as the smallest and most embarrassing of the collection gaps |
| `docs/content/alerting/` | Four pages, three of which are a heading or the word `TODO` | The customer-facing documentation is a stub |

The alert definitions themselves are good work and are not the problem.
They carry structured descriptions, a `severity` label, a `component` label, `for` durations that look considered, and an inline or referenced query from the registry.
Rendering them into rule files is a small change.

### `PrometheusRule` has exactly one consumer, and it is off

This is the non-obvious part, and it is worth stating before anyone writes the template that the empty `templates/alerts/` directory invites.

The chart installs the Prometheus Operator CRDs, so `PrometheusRule` is a valid resource on any cluster running this stack.
It does not install the Prometheus Operator, and it does not run Prometheus.
The one component that consumes `ServiceMonitor` and `PodMonitor` is Alloy, through `prometheus.operator.servicemonitors` in the gateway pipeline, and Alloy has no rule evaluator at all — it is a collector.

So a template emitting `PrometheusRule` resources today would render valid YAML, apply cleanly, pass every check in CI, and do nothing.

There is one consumer, and it arrives with the Thanos ruler.
The vendored Thanos subchart ships `ruler.autoImportPrometheusRules`, defaulted `true`, which runs a `kubectl` sidecar that watches `PrometheusRule` resources and writes them into the ruler's rule directory.
That is a real path, it needs no Prometheus Operator controller, and it is inert until `thanos.ruler.enabled` is `true`.

The design consequence is that **`thanos.ruler.enabled` is the switch that makes alerting exist**, and every other decision on this page is downstream of turning it on.

### The rule set is Materialize Cloud's rule set

The 85 definitions were ported from `infra/prometheus/alerting.py` in the Cloud repository, where they are deployed as an AWS Managed Prometheus `RuleGroupNamespace` and routed by `tier` and `team` labels.
That provenance shows.

| Group | Count | Present in a stock self-managed install? |
|---|---|---|
| `crdb` | 15 | No. Self-managed uses PostgreSQL for consensus |
| `egress_gateway` | 5 | No. A Cloud networking component |
| `external_uptime` | 3 | No. Marked `deploymentMode: cloud-only` |
| `launchdarkly` | 2 | No. Marked `deploymentMode: cloud-only` |
| `cilium` | 2 | Only on a cluster that runs Cilium, which is a customer choice |
| `coredns` | 1 | Usually, but as a cluster-provided component |

That is 28 of 85 that a stock install cannot fire, and only five of them carry the `deploymentMode: cloud-only` label that exists to say so.
A further set is shaped by Cloud even where the component exists: `env-uptime-sla` and `env-uptime-slo` read `v2_mz_can_connect`, which is produced by a Cloud-side synthetic checker, and `new-clusterd-restarts` is scoped to a release window that self-managed does not have.

None of this is a criticism of the port, which preserved definitions that were worth preserving.
It is an argument that **the default-enabled set has to be chosen rather than inherited**, and that the choosing needs a mechanism rather than a review pass, because a review pass decays.

### The class that has never been code

A survey of the Grafana instance Materialize operates for Cloud found 98 Grafana-managed alert rules.
Fourteen of them query Loki.

They detect panics, a persist filter-pushdown correctness violation, a set of data-corruption error strings, a hanging-query error string, and trace-level logging left enabled.
That list is roughly the highest-severity failure modes the product has, and it is the one list with no definition in any repository.

The reason is structural rather than an oversight.
Cloud's alert pipeline builds a Prometheus rule group and hands it to a managed Prometheus, which evaluates PromQL.
There has never been a place to put a LogQL rule, so log-derived alerts were clicked into Grafana, which will evaluate a query against any datasource.

Three properties of that set are worth recording, because each is a direct consequence of the rules being clicked rather than written.

**They are duplicated per region.** One rule per Loki datasource, three copies of each, kept in agreement by hand.

**The copies have drifted.** Two carry a routing label that sends them to a test receiver rather than the on-call one, and their titles still end in `(copy)` and `(copy 2)`. The three copies of the panic detector do not agree on their own aggregation: two group by namespace, one by namespace and pod.

**Deployment-specific exclusions are compiled into the query.** Two of the panic detectors carry a hardcoded namespace exclusion naming a single environment, inside the LogQL, where nothing reviews it and nothing expires it.

The same survey found alert rules filed in folders named after individual customers, and a notification route matching a customer-specific label.
Those are the load-bearing evidence for two requirements on this page: that customer-specific alerting is a real and recurring need rather than a hypothetical one, and that it must have a home that is not this repository.

## Architecture

```
  ┌──────────────┐        PromQL          ┌──────────────┐
  │ Thanos Query │◀───────────────────────│ Thanos Ruler │
  └──────────────┘                        │  (stateless) │
         ▲                                └──────┬───────┘
         │                                       │ remote_write (ALERTS, recording rules)
         │                                       ▼
  ┌──────────────┐                        ┌──────────────┐
  │Thanos Receive│◀───────────────────────│Alloy Gateway │────▶ destination fan-out
  └──────────────┘      remote_write      └──────────────┘      (Thanos, AMP, OTLP, call-home)
                                                 ▲
                                                 │ remote_write (recording rules)
  ┌──────────────┐        LogQL           ┌──────┴───────┐
  │     Loki     │◀───────────────────────│  Loki Ruler  │
  └──────────────┘                        └──────┬───────┘
                                                 │
         ┌───────────────────────────────────────┘
         │  alerts (both rulers)
         ▼
  ┌──────────────┐   routing tree    ┌────────────────────────────────┐
  │ Alertmanager │──────────────────▶│ receivers: webhook / slack /   │
  │              │  grouping         │ pagerduty / email              │
  │              │  inhibition       └────────────────────────────────┘
  │              │  silences
  └──────────────┘
         ▲
         │ read-only (Alertmanager datasource)
  ┌──────────────┐
  │   Grafana    │
  └──────────────┘
```

Four properties of that shape are the design.

**Alertmanager is the single notification surface.**
An operator configuring where alerts go configures one thing, whether the alert came from a metric or a log line.

**Both rulers remote-write through the gateway.**
The Loki ruler already does this for recording-rule samples.
Thanos Ruler joining it is what makes alert state a signal the destination fan-out can see, rather than a series that exists only inside Thanos.

**Grafana reads Alertmanager rather than owning alerts.**
An `alertmanager` datasource gives Grafana the alert list, the silence UI, and the ability to put firing alerts on a dashboard, with no alert state in Grafana's own database.
This is also how the Cloud Grafana instance is already configured against the managed Alertmanagers, so the shape is familiar.

**The rulers are the only components that need to reach the query path.**
Nothing else in the alerting path depends on Thanos Query or the Loki query frontend being healthy, which bounds the blast radius of the failure described next.

### The evaluator depends on the query path

Thanos Ruler evaluates by issuing PromQL to Thanos Query over the network.
There is no local TSDB to fall back on, because there is no Prometheus in this stack — Alloy scrapes and remote-writes, and Thanos Receive is where the data lands.

The consequence is that **an outage in the query path stops alert evaluation**, and stopping alert evaluation looks exactly like nothing being wrong.
The same is true of the Loki ruler and the Loki read path.

This is not avoidable within this architecture, and it is the strongest argument on this page for the deadman's switch described in [Alerting about alerting](#alerting-about-alerting).
It is also an argument for sizing the query path with the alerting dependency in mind, which is a note the Thanos sizing profiles do not currently carry.

## Two evaluators, one notifier

**Decision: Thanos Ruler for PromQL, Loki Ruler for LogQL, one Alertmanager for both.**

The alternative worth taking seriously is Grafana-managed alerting through the grafana-operator CRDs, and it is treated on its own below.
The alternative *not* worth taking seriously is a single evaluator: no component evaluates both PromQL and LogQL, and building one is not a monitoring-chart project.

| | Thanos Ruler | Loki Ruler |
|---|---|---|
| Language | PromQL | LogQL |
| In the chart today | Present, `enabled: false` | Present, `enabled: true`, no rules |
| Rule source | `ruler.rules` (inline files), or `PrometheusRule` via the import sidecar | The ruler object-storage bucket, or a local rule directory |
| Alertmanager | `ruler.alertmanagers.config`, Thanos format | `loki.rulerConfig.alertmanager_url` |
| Recording rules | Yes, remote-written in stateless mode | Yes, already remote-written to the gateway |
| Tenancy | None | Per tenant. See [Multi-tenant Loki](#multi-tenant-loki-multiplies-the-rule-set) |

Two evaluators is more moving parts than one and it is the honest shape.
The cost is paid in the chart, once, and an operator configuring notifications never sees it.

### Thanos Ruler runs stateless

**Decision: the ruler remote-writes to the Alloy gateway and keeps no TSDB.**

Thanos Ruler has two modes.
The default keeps a local TSDB of rule results and ships blocks to object storage, which is why the subchart defaults it a 10Gi PVC.
Stateless mode remote-writes results instead and holds nothing.

Stateless is correct here for three reasons, in ascending order of how much they matter.

**It removes a stateful workload.** A PVC per replica, with the sizing, storage-class, and portability concerns every PVC in this chart has already generated.

**It puts rule results on the same path as everything else.** Every other series in this stack arrives at Thanos through the gateway. A ruler shipping its own blocks is a second writer into object storage with its own compaction interaction, and the Loki ruler already established the other pattern.

**It is what makes alert state forwardable.** The [call-home](../20260917-call-home-self-managed/#where-the-series-has-to-come-from) design needs `ALERTS` to traverse the gateway, because the destination fan-out is a gateway component. A ruler shipping blocks to object storage puts `ALERTS` in Thanos and out of reach of every destination except Thanos. That design flagged this as a checkable requirement rather than an assumption; this is the check, and the answer is that it constrains the ruler's mode.

**The subchart does not model this.**
`thanos.ruler` exposes no `remoteWrite` key.
It exposes `ruler.extraArgs`, so `--remote-write.config` is reachable, and reaching it that way leaves the PVC and the objstore flag configured for a mode the ruler is no longer in.
Modeling stateless ruler properly is a prerequisite rather than a values trick, and it is the kind of subchart gap this repository has fixed upstream before.

## Severity belongs to the alert, urgency belongs to the deployment

This is the requirement that a Materialize deployment can be critical infrastructure or a thing somebody is playing with, and the rules should not know which.

**Decision: rules carry `severity`; a single values key selects the severity-to-route mapping.**

The rules already carry `severity`, with three values.
Their meanings should be written down, because a label with an intuitive name and no definition drifts within two releases.

| `severity` | Means | Not |
|---|---|---|
| `critical` | Materialize is not doing its job, or is about to stop | A component is unhealthy but the service is not affected |
| `warning` | A condition that will become `critical` if unattended, or a degradation a user can perceive | Something an operator would want to know eventually |
| `notice` | A condition worth knowing about, with no deadline | A condition nobody would act on, which should not be a rule |

`alerting.criticality` then selects what those mean for this deployment.

| `severity` | `critical-infrastructure` | `important` (default) | `evaluation` |
|---|---|---|---|
| `critical` | Page | Notify, high priority | Notify, normal priority |
| `warning` | Notify, high priority | Notify, normal priority | Notify, normal priority |
| `notice` | Notify, normal priority | Notify, low priority | Suppressed |

The columns are receiver *classes*, not receivers.
An operator maps `page`, `high`, `normal`, and `low` onto their own contact points once, and the matrix does the rest.
That indirection is what lets the same values file work for a deployment with a pager and one with a single Slack channel.

Three notes on the table.

**`important` is the default** because it is the assumption that is wrong in the least damaging direction.
A deployment that is really critical infrastructure and routes `critical` to a notification instead of a page has a delayed response.
A deployment that is really an evaluation and pages on `critical` has an operator who turns alerting off.

**`Suppressed` is not dropped.**
The alert still fires, still appears in Alertmanager and on the alerts dashboard, and generates no notification.
The distinction matters during an incident, when the question is what else was true at the time.

**The matrix is values, not a template.**
An operator who wants `warning` on `evaluation` to be low rather than normal edits one cell, and does not have to understand the routing tree to do it.

## Notification channels

**Decision: named receivers as a map, typed for the four shapes Alertmanager supports natively, with a raw passthrough beside them.**

```yaml
alerting:
  criticality: important
  receivers:
    oncall:
      class: page
      type: pagerduty
      routingKeySecret:
        name: mzmon-alerting
        key: pagerduty-routing-key
    platform-alerts:
      class: [high, normal]
      type: slack
      channel: "#platform-alerts"
      apiUrlSecret:
        name: mzmon-alerting
        key: slack-webhook-url
    incident-io:
      class: page
      type: webhook
      url: https://api.incident.io/v2/alert_events/http/<id>
      bearerTokenSecret:
        name: mzmon-alerting
        key: incident-io-token
```

The map-keyed-by-name shape follows `pipeline.metrics.gateway.destination.prometheusRemoteWrite`, which is the established precedent in this chart for "several of these, each configured independently".

`class` is how a receiver attaches to the matrix above, and it takes a list because one channel absorbing several classes is the common small-deployment case.
A class with no receiver is a render-time error, not a silent drop — an unroutable severity is exactly the kind of misconfiguration that is discovered during the incident it should have reported.

### Credentials are referenced, never inlined

**Every credential field on a receiver takes a Secret reference and has no inline form.**

Alertmanager's own configuration is a Secret, so an inline token is not exposed at rest.
It is exposed in the values file, and values files are committed, diffed, pasted into support threads, and rendered into Terraform plans.
Grafana's `assertNoLeakedSecrets` guard exists in this chart for the same reason on a neighbouring surface, and the argument has already been won there.

The cost is that an operator creates a Secret before configuring a receiver.
The Terraform module can create it from a variable marked `sensitive`, which is where the ergonomics belong.

### incident.io is a webhook, and that is the right answer

Materialize routes to incident.io, which accepts Alertmanager's native webhook payload with a bearer token.
There is no incident.io-specific protocol to model.

**Decision: ship `type: webhook` with bearer and basic authentication, and an `incident-io` profile rather than an `incident-io` receiver type.**

Profiles are documentation in this chart, a convention the [Terraform modules design](../20260803-terraform-modules/#profiles-are-documentation-with-one-exception) established and this is a clean case for it.
The profile carries the URL shape, the grouping that produces one incident per condition rather than per series, the label set incident.io keys on, and the Secret the token goes in.
A customer using a different incident tool copies it and changes four lines, which they cannot do with a receiver type they would first have to learn does not fit them.

A typed receiver per vendor is also a maintenance liability that scales with the vendor list and pays off only for vendors whose payload Alertmanager does not already speak.
PagerDuty, Slack, and email are typed because Alertmanager types them.

### External Alertmanager

**Decision: `alerting.alertmanager.mode` takes `bundled` or `external`, mirroring `connections.grafana.mode`.**

A customer running Alertmanager already has routing, receivers, silences, escalation policy, and a team that knows them.
Deploying a second one beside it and asking which alerts arrive from which is a worse outcome than sending ours to theirs.

In `external` mode the chart configures both rulers to notify the given endpoints, renders no Alertmanager, and skips the routing tree entirely — the routing matrix above is bundled-mode configuration, and an external Alertmanager's tree belongs to its owner.
What the chart still owes that operator is the **label contract**: which labels the rules emit, what values they take, and what they mean, so that routing can be written against them.
That contract is a documentation deliverable and is listed as one.

## Choosing what ships enabled

**Decision: `rules.selected` globs over rendered rule groups, defaulting to the self-managed-applicable set, with applicability checked at build time.**

Three mechanisms, in the order they apply.

**Build-time exclusion.** A rule whose query names a metric family that no scrape source in this chart produces cannot fire, and the build knows this. `extract_metrics` already walks the registry and resolves every metric a query names, and `metric-tiers.yaml` already groups the result. Extending that to fail the build — or to file the rule under a non-default group — is a small change to machinery that exists, and it replaces a hand-maintained `deploymentMode` label that is already unreliable at 5 of 28 cases.

**Selection.** `rules.selected` defaults to the applicable set and follows the `dashboards.selected` glob pattern, which is a shape contributors and operators already know.

**Per-rule disable.** `rules.disabled` takes a list of alert names, because the common case is disagreeing with one rule rather than a category.

The build-time check is the load-bearing one.
Selection and disabling are how an operator expresses a preference; the check is what keeps a rule that cannot possibly fire from being shipped as though it might.

### Log-pattern rules are best-effort by construction

A rule matching `|~ "(?i)panic"` depends on a string in a log line, and a log line is not an interface.
Materialize can reword a message in a patch release and silently disable the alert that was watching for it.

Two things follow.

**Every log-pattern rule is `stability: best-effort`**, and none of them graduate to `canonical` while the contract is a string.
The registry's stability field already means "how much this repository commits to it", and a string match is not something to commit to.

**The real fix is upstream.** A structured error code in the log line — a stable field, versioned like a metric name — turns a pattern match into a label match, and turns a rule that decays silently into one that fails loudly. That belongs on the metrics-contract dependency list beside the other asks, and the correctness-violation alerts are the strongest case for it, because they are the ones whose silent decay costs the most.

Until then, the mitigation is a test asserting the patterns still match a corpus of real log lines, which is weaker than a contract and much better than nothing.

## Extension, in four places

The requirement is that a customer can extend and override at the Helm level, and that customer-specific alerting never enters this repository.
That decomposes into four extension points, each small, and the fourth is the one that is usually missing.

### Extra rules

```yaml
rules:
  extra:
    my-pipeline-slas:
      engine: promql
      groups:
        - name: ingest-freshness
          rules: [...]
    my-log-detectors:
      engine: logql
      groups: [...]
```

Passed through verbatim into the rule files the corresponding ruler loads, validated for syntax at render, and otherwise untouched.
This is where a customer's own alerting lives, and where a Materialize support engineer puts a deployment-specific rule — in that deployment's values, reviewed by the people who run it, with a name nobody else has to see.

### Rule overrides

```yaml
rules:
  overrides:
    environment-pod-pending-critical:
      for: 30m
      severity: warning
      labels:
        team: platform
```

**Decision: overrides may change `for`, `severity`, `labels`, `annotations`, and `keepFiringFor`. They may not change the expression.**

The tempting next step is threshold parameterization — exposing the numbers inside an expression as values.
It is deliberately not proposed, because doing it generically requires the renderer to know which literal in an arbitrary PromQL expression is the threshold, and doing it specifically requires every rule author to remember to name one.
Neither is free, and the first is not reliable.

An operator who needs a different threshold disables the shipped rule and adds their own through `rules.extra`, which is three more lines and is honest about what has happened: the rule is now theirs, and a chart upgrade will not change it.
Named thresholds are a reasonable later refinement, added per rule where the demand is demonstrated, and they are not a foundation.

### Extra receivers

Covered above.
A receiver added under `alerting.receivers` participates in the class matrix like any other.

### Extra routes

This is the point that is usually missing, and its absence is what forces forks.

```yaml
alerting:
  routes:
    extra:
      - matchers: ['component="storage"', 'severity="critical"']
        receiver: data-team
        continue: false
```

Extra routes are spliced into the tree **ahead of** the severity matrix, so a specific match wins and the matrix remains the fallback.
That ordering is the whole value: a customer with one team that owns sources and another that owns everything else expresses it in four lines, and every alert not matching still routes correctly.

There is prior art for exactly this problem in the operator CRDs this chart already installs.
`GrafanaNotificationPolicy` carries a `routeSelector` that merges in `GrafanaNotificationPolicyRoute` resources by label, so a route can be added by an object that the chart does not own and does not need to know about.
For the Alertmanager path, a values list is the equivalent and is enough.
For deployments where routes are managed by a separate GitOps pipeline, the CRD path is available and is one of the reasons the Grafana-managed alternative stays supported.

## Log-derived alerts

This class becomes first-class here, and three things about it need stating before someone ports the clicked-in rules.

### The range trap

Every clicked-in Loki rule is shaped like this:

```
sum by (namespace) (count_over_time({namespace=~"environment.+", level="ERROR"} |~ "(?i)panic" [$__range]))
```

`$__range` is a Grafana variable.
It resolves to whatever relative time range the alert rule's query is configured with, and it does not exist in LogQL.
A Loki ruler evaluating that expression fails to parse it.

Porting these rules means writing an explicit window, and an explicit window is a semantic decision rather than a transcription.
The clicked-in rules pair a range with `for: 0s`, so the alert fires on the first evaluation where the count over the range exceeds zero — meaning the range, not `for`, is what controls how long a condition persists after the last matching line.
A ruler rule written as `count_over_time(...[5m]) > 0` with `for: 0s` behaves the same way, and the `5m` is now a number someone chose.

This is worth an explicit note because the failure mode of getting it wrong is an alert that is far noisier or far quieter than the one it replaced, with no indication that anything changed.

### Multi-tenant Loki multiplies the rule set

The bundled Loki runs `auth_enabled: true`, and the pipeline's `tenancy.tenantMap` decides which tenant a log stream is written to.
The default is `static` across all four classes, so a default install has one tenant and one rule set.

`byNamespace` and `byEnvironment` are supported values, and under either of them **every Materialize environment is its own tenant**, the tenant set changes as environments are created, and a Loki rule group is per tenant.
The ruler does not evaluate across tenants.

Two consequences.
**The chart must render the log rule set once per tenant** under those modes, which means the tenant list has to be knowable at render time — it is, for `byEnvironment` against a configured environment list, and it is not for `byNamespace` against namespaces created later.
**A deployment on `byNamespace` may not be able to have complete log alerting**, and saying so is better than shipping a rule set that covers the environments that existed when Helm last ran.

This is a genuine constraint of the tenancy model rather than a gap, and the honest resolution may be that log alerting requires `static` or `byEnvironment`, stated as a documented requirement.

### Where log rules live

In the query registry, beside the metric alerts, in a `materialize-log-alerts.yaml` and an `infra-log-alerts.yaml`.
The registry already models LogQL — `env-logs` and `env-upgrade` are built on LogQL families — and an alert already carries either an inline query or a `queryId`.

That gets the class four things it has never had: review, a docsite entry beside the metric alerts, a single definition rendered to both the chart and, eventually, Cloud, and the ability to change a pattern in one place rather than in three regions' worth of clicked-in copies.

## Maintenance windows do not work when the customer picks the time

The `docs/content/alerting/maintenance.md` page is an empty heading, and the obvious thing to put on it is wrong for this audience.

Cloud suppresses upgrade noise with wall-clock mute timings — a named weekly window per region, during which the SLO routes are muted.
That works because Cloud chooses when to upgrade.

A self-managed customer upgrades when they decide to, which makes a weekly window either useless or permanently open.

**Decision: suppress upgrade noise by inhibition on a rollout signal, with mute timings available as a fallback.**

Alertmanager inhibition suppresses one alert while another is firing.
A rollout-in-progress alert — `notice` severity, routed nowhere, existing only to be inhibited against — turns "an upgrade is happening" into a condition the routing tree can read.

The signal exists.
The `env-upgrade` dashboard is built on generation-scoped metrics and orchestratord's reconciliation state, and its centrepiece is the hydrating-collection count descending toward zero, which is exactly "a rollout is in progress and has not converged".

Two properties make this better than a window and they are worth stating.
**It closes itself.** A window that a customer opens by hand is a window somebody forgets to close, and the alerts that were suppressed for the upgrade stay suppressed for the incident.
**It suppresses the right alerts.** A window mutes a route; inhibition can be written to suppress pod-restart and pending-pod alerts while leaving an unreachable-environment alert firing, which is the distinction an operator actually wants during an upgrade.

Wall-clock mute timings stay available, because a customer with a change-management process and a standing window is a real case and Alertmanager supports it directly.
They are the fallback rather than the recommendation.

## Alerting about alerting

A stack that notices everything except its own failure to notice is the failure mode that makes alerting untrustworthy, and this stack currently has three ways to fail silently.

| Failure | Detected by |
|---|---|
| Alertmanager cannot deliver to a receiver | `alertmanager_notifications_failed_total`, which requires the scrape that does not exist |
| A ruler stops evaluating | `up` on the ruler, plus rule-evaluation failure counters |
| The query path is down, so evaluation returns nothing | Nothing inside the cluster |

The third is the one that matters and it is the one a self-contained stack cannot solve.

**Decision: ship a deadman's switch, and document what it is worth.**

An always-firing `notice`-severity alert, routed to its own receiver, on a short group interval.
Its value is entirely on the receiving side: something outside the cluster has to notice that the periodic notification stopped.

For a deployment routing to incident.io or PagerDuty, that is a heartbeat monitor those products provide, and the switch is genuinely load-bearing.
For a deployment routing to a Slack channel, it is a message every few minutes that somebody mutes within a week.
For a deployment with no external receiver at all, it is worth nothing, and saying so is better than shipping it as a checkbox.

The deadman's switch must be exempt from the severity matrix and from `alerting.criticality`, because a `notice` suppressed on `evaluation` is a deadman's switch that is always dead.

**Alertmanager also has to be scraped.**
The roadmap records this as an open collection gap and calls it the most embarrassing of them, which is fair — it is our own component, deployed by our own chart, emitting metrics to nothing.
It is a prerequisite here rather than a nice-to-have, because two of the three rows above depend on it.

## What this owes call-home

The [call-home design](../20260917-call-home-self-managed/#the-alerts-level) places `alerts` as the lowest genuinely useful consent level and records that it is blocked on rule evaluation rather than on anything in that design.
This is the unblocking, and it carries two obligations.

**`ALERTS` has to traverse the gateway.**
That design flagged it as a checkable requirement; the check is that Thanos Ruler must run stateless and remote-write, which is a decision made here for independent reasons and happens to satisfy it.
A ruler left in its default mode would put `ALERTS` in Thanos and out of reach of every destination in the fan-out, and the level would render correctly and forward nothing.

**Silences are part of the signal.**
An alert silenced locally still produces an `ALERTS` series in the firing state, so a control plane consuming the state series will see firing where a customer has deliberately decided not to care.
The state series alone therefore over-reports, and the design of the alerts level should assume that until the Alertmanager webhook path exists.

## Grafana-managed alerting as an alternative

**Decision: supported, documented, not the default.**

The grafana-operator CRDs this chart already installs cover the whole surface: `GrafanaAlertRuleGroup`, `GrafanaContactPoint`, `GrafanaNotificationPolicy`, `GrafanaNotificationPolicyRoute`, `GrafanaMuteTiming`, and `GrafanaNotificationTemplate`.
It is a complete alternative, not a partial one, and it has two real advantages.

**One rule model across both datasources.** A Grafana alert rule queries whatever datasource it names, so metric rules and log rules are the same kind of object — no second evaluator, no second rule format, no `$__range` trap.

**It matches what Materialize already does.** The clicked-in rules are Grafana rules. Codifying them as `GrafanaAlertRuleGroup` resources is a smaller step than porting them to a Loki ruler.

It is not the default for one reason, which is sufficient.

**Grafana's alert state lives in Grafana's database**, and this chart's default Grafana is SQLite on an `emptyDir`.
Alert state, silences, and rule evaluation history are lost on every restart, upgrade, and reschedule.
The chart already documents that the default Grafana is the safe shape rather than the production shape, and requires `grafana-postgres` or `grafana-pvc` for anything stateful.
Making alerting depend on that is making the most important guarantee in the stack depend on the profile most likely to be skipped.

Two secondary reasons reinforce it.
A deployment using `connections.grafana.mode: external` — a shared platform Grafana, or Grafana Cloud — cannot be given alert rules by this chart in any mode where it does not hold credentials.
And routing through Grafana's embedded Alertmanager means the alerting path depends on Grafana being up, which is a component this chart's own guidance describes as losing nothing important when it restarts.

The documented shape for a customer who wants it: enable Grafana-managed alerting, turn off the rulers and the bundled Alertmanager, and apply `grafana-postgres`.
That is a profile, and it is worth shipping as one.

## Chart-side prerequisites

Work in this repository, roughly in dependency order.

| Item | Why it blocks |
|---|---|
| `gen-rules` command in `mz-monitoring-build`, rendering the registry's alerts into `pre-rendered/rules/{prometheus,loki}/` | Nothing renders rules today; every item below consumes the output |
| Build-time applicability check against the extracted metric set | Decides the default-enabled set; replaces the unreliable `deploymentMode` label |
| `thanos.ruler` enabled by default, wired to Thanos Query and Alertmanager | The switch that makes PromQL alerting exist |
| Stateless Thanos Ruler modeled in the subchart (`remoteWrite`, no PVC, no objstore) | Required for `ALERTS` on the gateway; `extraArgs` reaches it and leaves the rest inconsistent |
| `loki.rulerConfig` with `alertmanager_url` and the rule store | The Loki ruler runs today and notifies nothing |
| Alertmanager configuration surface: receivers map, class matrix, `alerting.criticality`, inhibition, mute timings | The routing half of the feature |
| Alertmanager Secret wiring for receiver credentials | Credentials are referenced, never inlined |
| Alertmanager ServiceMonitor | Two of three meta-alerting rows depend on it, and it is an existing recorded gap |
| Alertmanager NetworkPolicy egress review | The existing policy is deliberately wide; the receiver set now makes the destinations knowable per deployment |
| Deadman's switch rule, exempt from the severity matrix | Distinguishes silence from health |
| `rules.selected` / `rules.disabled` / `rules.extra` / `rules.overrides` | The extension surface |
| `alerting.routes.extra`, spliced ahead of the matrix | The extension point whose absence forces forks |
| `alerting.alertmanager.mode: external` | A customer with Alertmanager should not get a second one |
| Log-alert registry files and the LogQL render path | The class that has never been code |
| Rollout-inhibition rule over the `env-upgrade` generation signals | Maintenance windows that close themselves |
| Alertmanager datasource in Grafana, and an alerts dashboard | Alert state has no view today |
| `incident-io` profile, `grafana-managed-alerting` profile | Profiles are documentation |
| Terraform module surface for receivers and criticality, with `sensitive` credential variables | Where the Secret-creation ergonomics belong |
| Remove `config.rules.*` / `config.alerts.enabled` or make them load-bearing | Four values keys currently read by nothing |

## Testing

The kind E2E tiers can prove most of this, and the parts they cannot are worth naming rather than pretending.

- **Rules render and parse.** Every rendered group is checked with `promtool check rules` and Loki's rule validator, in CI, without a cluster. A rule that does not parse is loaded by a ruler that then serves none of its group.
- **Every shipped rule's metrics exist.** The applicability check, asserted as a test rather than only as a build step, so that a rule added later cannot quietly reintroduce the problem.
- **The rulers load what the chart rendered.** Assert against each ruler's own API that the group count and names match the render, rather than asserting the ConfigMap exists. A ruler that rejected a group reports healthy.
- **An alert reaches Alertmanager.** Install a rule that fires on `vector(1)`, assert it appears in Alertmanager's API within the evaluation and group-wait interval. This is the end-to-end assertion the whole page exists for.
- **The routing matrix routes.** For each of the three criticality settings, assert that a synthetic alert at each severity lands on the expected receiver, read from Alertmanager's own routing-tree API rather than from the rendered config. Rendering the tree correctly and Alertmanager interpreting it as intended are different claims.
- **Extra routes take precedence.** A route added through `alerting.routes.extra` wins over the matrix for a matching alert, and a non-matching alert still reaches the matrix. Ordering bugs here are invisible until the wrong team is paged.
- **An unroutable class fails the render.** A receiver set missing a class that the matrix references is a render-time error, tested as one.
- **Credentials do not appear in the render.** Assert no receiver credential is present in any rendered object except by Secret reference, which is the mechanical half of the inlining rule.
- **`ALERTS` arrives through the gateway.** Assert the series is queryable in Thanos *and* visible to a gateway destination, because landing in Thanos by a second path would satisfy a naive version of this test.
- **The deadman's switch fires and keeps firing.** Assert it is present at every `alerting.criticality` setting, which is the exemption that is easy to lose in a refactor.
- **Silences suppress notification and not state.** Silence an alert, assert no notification and a firing `ALERTS` series. This is the over-reporting behaviour the call-home level inherits, and pinning it keeps it a known property rather than a surprise.
- **Log rules fire on real lines.** Write a line matching each shipped pattern into Loki and assert the rule fires. This is the test that catches an upstream reword, and it is only as good as the corpus.
- **Not covered by any tier:** that a notification reaches a receiver. Every receiver is a third-party endpoint, and a test that posts to a real one is a test that pages somebody. A local webhook receiver proves Alertmanager's delivery path and proves nothing about a vendor's ingestion.

## Documentation to update

- **`alerting/configuring.md`** — currently the word `TODO`. The severity table, the criticality matrix, and the receiver map. This is the page an operator reads once and configures from.
- **`alerting/channels.md`** — currently a heading. One section per receiver type, the Secret shape, and the incident.io profile.
- **`alerting/maintenance.md`** — currently a heading. Inhibition on the rollout signal first, mute timings second, and why that order.
- **A label contract page** — every label a shipped rule emits, its values, and its meaning. Owed to anyone routing in an external Alertmanager, and to anyone writing an extra route.
- **A rule reference** — the shipped set, with what fires it and what to do about it. `reference/stable-metrics/common-alerts.md` renders the definitions today and carries a warning that many of them do not suit every deployment; once the set is culled that warning should become a statement about which set is default and why.
- **`operating/production-best-practices.md`** — alerting is a shared-responsibility item and has no entry. The deadman's switch is worth nothing without an external receiver, and that belongs on a checklist.
- **`architecture.md`** — the alerting path is absent from the architecture page.
- **`reference/internal/roadmap.md`** — ✅ done. The [Rules & alerts](../../roadmap/#rules--alerts) section and a follow-up-documentation entry point here.
- **`reference/internal/versioning.md`** — whether an alert name is a committed surface. An operator's `rules.disabled` entry and an external Alertmanager's routing both name alerts, which makes renaming one a breaking change to something.
- **A migration note for Cloud** — the clicked-in Loki rules are the ones this makes definable. Whether Cloud adopts the registry's log alerts is a separate decision, and the rules being in one place is the precondition for it.

## Open questions

- [ ] **Which rules are in the default-enabled set, exactly?** The mechanism is proposed; the list is not. It should be short enough that an operator reads all of it, and every entry should have an operator action.
- [ ] **Does the build-time applicability check exclude or fail?** Excluding is friendlier and hides a rule that was meant to ship. Failing is louder and turns a scrape-config change into a broken build in an unrelated area.
- [ ] **Is `severity` on the deprecation cycle?** External routing is written against it, which makes adding a fourth value a change to somebody's routing tree.
- [ ] **Are alert names a committed surface?** They appear in `rules.disabled`, in external routing, and in notification payloads people build automation against. Committing them is a real constraint and not committing them makes the extension points unstable.
- [ ] **Does the chart render log rules per tenant, and is `byNamespace` supportable at all?** A namespace created after the last Helm run has no rules under it, and there may be no version of this that is complete.
- [ ] **What is the grouping key?** `[alertname, namespace]` produces one notification per condition per environment, which is right for most rules and wrong for a node-level condition affecting forty pods. Grouping may need to be per rule rather than global.
- [ ] **Does the stateless ruler need a WAL?** The Loki ruler keeps a PVC specifically so its remote-write WAL survives a gateway outage. The same argument applies to the Thanos ruler, and it undercuts the "removes a stateful workload" reason for going stateless.
- [ ] **Two Alertmanager replicas or one?** Gossip-based HA is what the roadmap's hardening item covers, and a single replica holds the only copy of every silence. Alerting that duplicates notifications during a rolling update is a different failure from alerting that loses silences on restart, and the choice depends on which one an operator would rather have.
- [ ] **Where does the runbook live?** Every alert should link to one, and a link into this docsite is a link a customer may not be able to reach from an air-gapped network. Embedding the operator action in the annotation is self-contained and much less useful.
- [ ] **Does Cloud converge on these definitions?** The stated goal is one definition for three surfaces. Cloud's deployment path is Pulumi against managed Prometheus, and converging the definitions without converging the deployment is achievable and is a second consumer of `gen-rules` that nobody has scoped.
- [ ] **What happens to the Cloud-only rules?** Deleting them loses definitions that work in Cloud. Keeping them in this repository under a `cloud-only` selection keeps a public repository carrying rules for components it does not document. Neither is obviously right.
- [ ] **Should `notice` exist?** Three severities with the middle one doing most of the work is the common outcome, and a severity that routes nowhere on two of three criticality settings is close to a label. Collapsing to two would simplify the matrix and would lose the distinction between "act before this becomes an outage" and "know this happened".
- [ ] **Does the deadman's switch ship enabled?** It is worth nothing without an external receiver, and shipping it disabled means the deployments most likely to need it are the ones that never turn it on.

