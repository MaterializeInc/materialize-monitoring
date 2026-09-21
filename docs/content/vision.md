---
title: "Vision"
weight: 5
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5
---

# Vision

`materialize-monitoring` exists so that a Materialize deployment somebody else operates is as observable as one Materialize operates itself.
It packages metrics, logs, events, dashboards, and alerts as a Helm chart and a Terraform module.
It installs with the cluster by default.
For a customer who already runs their own observability stack, it reduces to nothing but a documented metric contract.

This page states the direction and the reasoning behind it.
The [Roadmap](../reference/internal/roadmap/) is the source of truth for status, and it is more current than this page by construction.

<!--
Agent note: this page is published. Keep forward-looking items framed as proposals under review.
Statuses go stale here faster than anywhere else in the docsite; prefer deleting a claim to maintaining it.
-->

## The problem

Materialize used to run in one place, operated by one team, watched by dashboards that team maintained by hand.
Self-managed changed the shape of the question.
A deployment now runs in a customer's Kubernetes cluster, on infrastructure the customer owns.
The operator watching it has often never seen a Materialize incident before.
Three costs follow from that, and the project exists to pay them down.

| Cost | What it looks like |
|---|---|
| **Divergence** | Materialize Cloud and self-managed grew separate dashboards, separate rule sets, separate collection pipelines, and separate metric names for the same measurement |
| **Invisibility** | A self-managed install is opaque between escalations, so an escalation starts by asking the customer to gather evidence that was never collected |
| **No default** | The recommended path was a point-in-time dashboard copy and a legacy scrape config, vendored into the installer and maintained by nobody |

The common cause is that observability was a deliverable attached to each deployment rather than a product with a version.
A customer could not upgrade it, Materialize could not test it, and neither party could say what it guaranteed.

## What the project is

**One stack, consumed several ways, with every component replaceable.**
That sentence carries three commitments, and most of the design follows from holding all three at once.

### One stack

There is a single set of pipelines, dashboards, scrape configurations, and alert definitions, generated from a single query registry.
Dashboards are Rust source rendered against the Grafana schemas, not JSON maintained by hand.
Alert and dashboard expressions come from the same registry, so a metric an alert depends on cannot quietly diverge from the panel that explains it.

### Several ways in

| Path | Audience | Shape |
|---|---|---|
| **Helm** | An operator who deploys with Helm, or who wants the full lever set | The umbrella chart, with profiles as sized deltas from a medium default |
| **Terraform module** | A customer installing Materialize with the published modules | On by default from `materialize-terraform-self-managed` v11, opt out with `enable_observability = false` |
| **Per-cloud wrappers** | AWS, GCP, and Azure installs | Downstream modules that wrap the common module and supply what only the cloud knows |
| **The metric contract alone** | A customer with a mature stack of their own | Documented metrics, queries, and scrape configurations, with none of the workloads |

A fourth consumer is proposed rather than built: Materialize Cloud, deploying the same chart through Pulumi.

### Every component replaceable

Loki, Thanos, Grafana, Alertmanager, and the Alloy pipelines are the bundled implementation of an interface, not the interface.
Each one can be switched off in favour of something a customer already runs.
The posture has a real cost, and the project pays it deliberately.
Composability means the project cannot assume any particular backend exists when it designs a feature.
That is why the read path below counts as a change of posture rather than a routine addition.

## Who this is for

The stack has four audiences, and they ask different questions of it.

| Reader | What they are trying to do |
|---|---|
| **A self-managed operator** | Keep a Materialize deployment healthy without a Materialize engineer beside them |
| **An SRE or platform engineer** | Fit Materialize into an observability practice that already exists, on that practice's terms |
| **A field engineer** | Answer a question about a deployment they cannot log into |
| **Materialize support and engineering** | Diagnose an escalation from evidence that was already being collected |

The maintaining team is small, and the audience above is not.
That asymmetry is the argument for treating this as a collaborative repository rather than one team's internal tooling.
The people who meet a failure mode are usually not the people who write the dashboard for it.
A panel, an alert, a query, or a runbook contributed by whoever hit the problem carries context that a metric list cannot supply afterwards.
Contributions from field engineers, SREs, and customers are encouraged on the same footing as contributions from the maintaining team.
The [contributor guide](../reference/internal/contributing/) covers the conventions.

The single-registry design is what makes a small contribution worth making.
A query added to the registry reaches the dashboard, the published documentation, and any alert referencing it.
One contribution is therefore not one panel.

## Why a deployment turns it on

Nothing here is required to run Materialize, and every component of it can be switched off.
The case for turning it on therefore has to be made per deployment, and what makes it is the set of moments a deployment produces on its own.

| The moment | The question it poses | Where the answer comes from otherwise |
|---|---|---|
| An upgrade has been running longer than expected | Is it progressing, or is it stuck | A version count, and a decision made on nerve |
| A replica is resized, or a workload is resharded | Has the new replica caught up, and at what cost | A SQL session against the environment, repeated until it looks settled |
| A source falls behind | Is it the source, the network, or the cluster reading it | Introspection queries against an environment that is already struggling |
| The deployment stops accepting connections | Is Materialize unhealthy, or is it the network or the balancer in front of it | Logs gathered by hand once the incident is over |
| An escalation opens | What was happening in the hour before it broke | A request to the customer to collect evidence nobody was collecting |
| The platform underneath misbehaves | Is it the node, the CNI, the object store, or the metadata database | `kubectl`, and cluster access the person asking may not have |
| The cloud bill arrives | What does Materialize cost, and which part of it is largest | An estimate |

Every one of these arrives whether or not anyone prepared for it.
What the stack changes is whether the evidence was being collected before the question was asked, because most of it cannot be gathered afterwards.
How much of each row is answered today is the roadmap's subject, and several are answered only in part.

## Why Materialize invests in it

The first question is asked by whoever installs the stack.
This one is asked internally, and it has a different answer.

| Reason | The evidence behind it |
|---|---|
| **Adoption decisions turn on operability** | An upgrade no operator could read blocked a production adoption decision, which is the case behind [the upgrade work](#from-displaying-metrics-to-answering-questions) |
| **Support cost scales with blindness** | An install nobody can see is an install whose every escalation starts by asking the customer to go gather evidence |
| **The alternative is the same work once per deployment** | The log alerts that detect panics and correctness violations exist as per-region copies, drifted from each other, maintained by clicking |
| **Two implementations get fixed once between them** | Cloud and self-managed maintain two versions of nearly the same query set, and twenty-six of those queries are identical SQL under identical names |
| **It has stopped being an accessory** | The console's [read path](#the-read-path-where-this-stops-being-optional) makes a PromQL and a LogQL endpoint load-bearing rather than optional |

One argument sits outside the table.
An operator evaluating self-managed asks how they will run it, and that answer is part of the product rather than an appendix to it.
The stack is what makes the answer demonstrable rather than a claim.

## Where it stands today

The platform is built and in use.
The following is the current surface at altitude, and the [Roadmap](../reference/internal/roadmap/) carries the per-item detail.

| Area | State |
|---|---|
| **Collection** | Alloy agent and gateway, node-exporter, kube-state-metrics, kubelet cAdvisor, and self-disabling CNI monitors for the two CNIs that publish anything |
| **Storage** | Loki and Thanos bundled and sized by profile, with fan-out to OTLP, Datadog, Google Cloud Monitoring, and additional Prometheus remote-write destinations |
| **Dashboards** | Six shipped: an environment overview, logs, and upgrades for a Materialize user, plus logs, nodes, and networking for whoever runs the cluster underneath |
| **Packaging** | Chart and CRDs chart published to GHCR, a Terraform module on the same version stream, per-component SemVer, a generated changelog, and a written deprecation policy |
| **Security** | NetworkPolicy on every workload by default, opt-in in-cluster mTLS with a four-stage rollout, and chart and image scanning in CI |
| **Qualification** | Terraform render checks, two `kind` end-to-end tiers, and a Rust assertion suite verified against real EKS and GKE clusters |

Two gaps matter more than the rest, and both shape the next phase.

**Nothing is paged.**
The alert definitions exist and render into the documentation, and no install evaluates them.
Alertmanager is deployed and receives nothing.

**The four questions a Materialize user asks first are still unanswered.**
Hydration, freshness, sources, and sinks each need instrumentation that lives upstream in the Materialize repository rather than here.

## Where it is going

Each theme below is a question this stack cannot answer yet.
The [Roadmap](../reference/internal/roadmap/) tracks the individual items behind each one, with their status and their designs.

### From displaying metrics to answering questions

The shipped overview is organized by subsystem, which suits a reader who already knows which subsystem is at fault.
The next set is organized by symptom and by operation.
A troubleshooting dashboard becomes the entry point, and every panel on it links onward into logs or into the matching drilldown.
Day 2 operations are weighted above Day 1, because a running deployment fails during a change far more often than during an install.

Upgrades is the worked example and the sharpest case.
An operator watching a long upgrade with no way to tell whether it is progressing has a production adoption decision blocked on that uncertainty.
The requirement that came out of it is the requirement for the whole family.
A dashboard has to answer *is it stuck* and *what do I do about it*, rather than display a version count.
The shipped upgrade dashboard answers the first question from the rollout's own account of itself, and the second is outstanding.

### Alerting: from shipping rules to paging a human

Every component needed to alert is in the chart, and no two of them are connected.
Closing that is the highest-value work on this page.

Three separate problems share the workstream.
Nothing evaluates rules, so the rule set is documentation.
The rule set is Materialize Cloud's rule set, and a third of it names components a stock self-managed install does not run.
Log-derived alerting has never been code anywhere at Materialize.
The rules that detect panics and correctness violations exist only as per-region copies clicked into a Grafana instance.

The proposed design takes four positions.

| Position | Why |
|---|---|
| Two evaluators, one notifier | PromQL and LogQL need different evaluators, and an operator needs one place where silences and grouping live |
| Severity belongs to the alert, urgency belongs to the deployment | One rule set cannot otherwise serve both a customer for whom Materialize is critical infrastructure and one who is evaluating it |
| A rule declares what it requires, not who operates it | A CockroachDB rule is for a deployment running CockroachDB, which a self-managed customer may well be |
| Customer-specific alerting composes out of values | Alert rules filed under individual customer names are what happens when it does not |

Alert names become a surface the project owes a deprecation cycle on from the first release that ships rules.
That decision is free only until the alerting path exists, so it is settled as part of the alerting design rather than after it.

### The services Materialize does not run

Every dashboard in the stack stops at the cluster boundary.
The two services a Materialize deployment cannot run without and does not run itself are the metadata database and the object store.
Neither has a dashboard, a working alert, or a collection path beyond what its clients happen to publish.

The central claim of the proposed design is that **the client's measurement of a dependency is the service level indicator**.
The dependency's own telemetry is the diagnosis.
Materialize, Loki, and Thanos already time and count every call they make to both dependencies, on every cloud, with no credentials.
That vantage point can ship on by default.
Everything flavour-specific sits behind a normalized contract.
That is what keeps seven plausible database flavours and five object stores from multiplying the dashboard and alert set.

The field evidence changed the design more than the analysis did.
Two observed failures were the metadata database exhausting disk and CPU.
One was a neighbouring project consuming a shared database instance, which no amount of client-side measurement can attribute.
The most common class was a day-0 setup failure, which is a different problem with a different answer.
The alerts that describe the first pair are well calibrated, carry correct remediation text, and could not have fired for two independent reasons.

### What a deployment costs

Every other workstream measures what a deployment is doing.
This one measures what it costs, which is the question a self-managed operator is asked by whoever owns the cloud bill.
The proposed shape is an optional cost component, off by default.

Deploying a cost exporter is the easy half.
Making the number mean something is the feature, and two problems stand in the way.
The default price for a node is a public list price rather than what the customer was billed, so every figure has to state where it came from.
A cost exporter attributes spend to namespaces and pods, while a Materialize operator asks about clusters, replicas, and cost centers.

The demand behind it is specific rather than a general appeal to efficiency.
Five questions drive it:

- Right-sizing a replica.
- Comparing machine classes across node generations.
- Deciding whether to shard a workload.
- Apportioning Materialize across the departments using it.
- Holding an honest total-cost conversation.

The last one shapes the rest.
A total that quietly excludes the object store, the metadata database, or the monitoring stack is not a total.

### The read path, where this stops being optional

Every workstream above is about collecting telemetry, and every consumer of it so far is a Grafana the project deploys.
The [tenant-scoped query API](../reference/internal/design-docs/20260916-tenant-query-api/) is about reading it from somewhere else.
The two readers are the Materialize console and a customer's own Grafana.

This is the one place the composability posture is genuinely in tension with the product.
The console renders environment metrics from SQL against the environment itself.
That path keeps a short window of history.
It is also unavailable precisely when the environment is.
Reading PromQL and LogQL instead makes a deployment without those endpoints a deployment with a broken console.

The design resolves the tension by mandating an interface rather than an implementation.
A PromQL endpoint and a LogQL endpoint carrying the documented label contract, reachable through a tenant-scoped proxy, is what a deployment owes.
Thanos and Loki are the bundled implementation of it, and a customer already running an equivalent points the proxy at theirs.
One consequence reaches the rest of the project.
The metric and label contract becomes load-bearing for a product surface rather than for dashboards alone.

**Three planned features assume a scheduled reader against the metrics backend, and none of those readers exists.**
Rule evaluation has to read in order to evaluate, this proxy serves reads to the console, and cost attribution issues queries on a schedule.
All three were sized against a backend whose only reader is a Grafana with a human in front of it.
That sizing question is common to all three, and it should be settled once rather than inside whichever lands first.

### Deployments Materialize cannot see

Two workstreams share a channel and differ in what authorizes using it.

**[Bring-your-own-cloud](../reference/internal/design-docs/20260813-byoc-observability/)** covers environments Materialize operates.
They run in the customer's own cloud account, and Materialize needs enough telemetry to operate them.
A reduced copy of telemetry crosses into the control plane, and the customer's full-fidelity copy always stays with them.
Metrics cross selected by importance tier, and logs cross as an allowlisted, redacted, level-filtered subset.
A pair of gateways enforces that boundary rather than ad-hoc network configuration.
Redaction attaches to the destination, so the reduced copy is a fork of the customer's stream rather than a downgrade of it.

**[Call-home](../reference/internal/design-docs/20260917-call-home-self-managed/)** covers installs Materialize does not operate.
A self-managed deployment is invisible between escalations.
The channel is the same, and the feature is consent.
A bring-your-own-cloud customer bought an operated service, so telemetry crossing the boundary is what they purchased.
A self-managed customer bought software they run themselves, and every byte that leaves is a concession.
The design is therefore a bounded, monotone, locally-visible ladder: `off`, `heartbeat`, `alerts`, `metrics`, `diagnostics`.
It defaults to off.
A preview mode and a local egress meter let the customer verify the bound rather than trust it.

Drafting it produced one finding that belongs on this page.
**Receiving a signal creates an obligation.**
Collecting alerts nobody is paged on is worse than collecting nothing.
The customer took a disclosure risk on the assumption that someone is watching.
A stated response model is a prerequisite for the alerts level rather than a follow-up to it.

### One stack instead of two

The divergence in the table at the top of this page is a cost the project is now in a position to remove.
Two changes do it from opposite ends, and both are Cloud-side work rather than changes to this chart.

**Cloud becomes the chart's third consumer.**
Cloud takes the Alloy pipelines and both backends, with Grafana staying external because it is operated outside this project.
Logs lead rather than metrics.
The log path already works end to end, which makes it the cheapest way to prove the chart against real traffic.
The gateway-to-gateway shape it adopts is the same shape bring-your-own-cloud builds redaction on.

**The metric-naming divergence is closed at its source.**
Cloud and self-managed today maintain two implementations of nearly the same query set.
Twenty-six of those queries are identical SQL under identical names.
Every dashboard in this repository threads a prefix parameter for no reason other than to paper over which process served the metric.
Both paths also run expensive SQL on the scrape path against the replicas they are trying to observe.
The goal is one path per metric, named consistently, with no SQL on the scrape path, reached over an authenticated connection.

### 1.0, and what the number promises

The project is pre-1.0, and breaking changes can ride a minor release.
The deprecation policy is written and landed, and stamping the number is not.

At 1.0 the minor-release allowance ends.
The label and metric contract, the profile semantics, the alert names, and the chart value paths all acquire a deprecation cycle.
The window for getting that discipline in place closes on its own.
Once enough customers have dashboards built on these labels, the contract is frozen in practice whether or not it is frozen on paper.
The discipline should therefore land before broad adoption rather than after.

Surfaces are graded by how much control the project has over them and by how a break presents.
Alerts fail silently and get the most care.
Metrics fail visibly and get the least.
See [Stability Guarantees and Deprecation Policy](../reference/stability/) for what a consumer can rely on today.

## What has to come from elsewhere

The metric and label contract is the public interface for everything on this page.
Part of it is instrumented upstream in the Materialize repository rather than here.
That dependency shapes the dashboard roadmap directly.

| Upstream ask | What it unblocks |
|---|---|
| Object and `_info` metrics carrying names and parent references | Delivered. Every other metric now has a stable join target for names |
| Native source and sink status metrics | The sources and sinks drilldowns |
| Native hydration and frontier signals | The hydration and freshness drilldowns |
| Label-family harmonization | Queries that do not have to know which of three label forms a deployment emits |
| `balancerd` and `console` metrics | The two components a user connects *through*, which today are observable only as logs |
| Operator reconciliation metrics and lifecycle events | Most of the upgrade dashboard, which ships with those panels empty until it lands |

The pattern is worth naming.
This project can build a collection path, a dashboard, and an alert for any signal that exists.
It cannot instrument what does not emit, and the gap between those two facts is where most of the blocked work on the roadmap sits.

## Where the detail lives

| Question | Page |
|---|---|
| What is built, in flight, and planned next | [Roadmap](../reference/internal/roadmap/) |
| How the pieces fit together | [Architecture](../architecture/) |
| What the vocabulary on these pages means | [o11y Glossary](../o11y-glossary/) |
| What a consumer can rely on | [Stability Guarantees and Deprecation Policy](../reference/stability/) |
| How to install it | [Getting Started](../getting-started/overview/) |
| Why a larger piece is shaped the way it is | [Design Docs](../reference/internal/design-docs/overview/) |
