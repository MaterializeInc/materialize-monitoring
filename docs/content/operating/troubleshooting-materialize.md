---
title: "Troubleshooting Materialize"
weight: 25
---

# Troubleshooting Materialize

For when **Materialize** is unhealthy and the evidence has to come out of this stack — metrics, logs, or Kubernetes
events — rather than out of a SQL session.

Its mirror is [o11y Troubleshooting](../o11y-troubleshooting/), which covers this stack failing to install, start, or
store data.
The tell is what you are asserting about: "Grafana shows nothing" is that page, "Grafana shows a cluster pinned at its
memory limit" is this one.

Both audiences land here.
An operator is looking for a verdict — retry, resize, restart, or escalate — and for evidence someone else can act on.
A Materialize developer is usually looking at someone else's environment without cluster access, and needs the same
evidence to say which subsystem is misbehaving.
The method is identical; only where it stops differs.

## Tools, and what you do not need

**You do not need cluster admin.** Nearly everything below is answerable through Grafana, and the rest needs read access
to one namespace.
If you are being asked to escalate your own privileges to run a diagnosis, something has gone wrong with the diagnosis.

[`gcx`](https://github.com/grafana/grafana-com-cli) is the fastest way to ask this stack a question, and it needs no
Kubernetes access at all — it talks to Grafana, and through Grafana to Thanos and Loki:

```bash
gcx config current-context                 # which stack am I pointed at
gcx metrics query 'up' --context <ctx> --datasource <prom-ds> -o json
gcx logs query '{namespace="materialize-environment"}' \
    --context <ctx> --datasource <loki-ds> --from now-1h --to now -o json
gcx logs labels                            # what stream labels exist here
```

Grafana's Explore view answers the same questions with the same expressions if you would rather click.
The advantage of the CLI is that findings come out quotable — a query someone else can re-run beats a screenshot.

`kubectl` is optional, and namespace-scoped access is enough:

```bash
kubectl -n <mz-namespace> get pods
kubectl -n <mz-namespace> describe pod <pod>        # events, limits, last state
kubectl -n <mz-namespace> logs <pod> --previous     # the crash, not the restart
```

Node-level questions — memory pressure, taints, capacity, whether the machine is the problem — are answerable from the
**Infrastructure Node Detail** dashboard by anyone who can reach Grafana.
That is why it exists; see [Dashboards](../../dashboards/grafana/architecture/#dashboards).

## The first five minutes

Ordered so the cheapest question that could explain everything comes first.

1. **Is it up?**
   *Materialize Environment Overview* → Summary.
   Environment Status and Last Restart Time.
   A restart inside the window explains lag, hydration, and connection errors all at once, and stops you investigating
   three symptoms of one event.
2. **Did something change?**
   *Materialize Upgrade* → Events, or *Materialize Logs and Events* → Events.
   A rollout, an OOM kill, a reschedule.
   Kubernetes events are the cheapest narrative available and usually the answer.
3. **Is it the machine?**
   *Infrastructure Node Detail* → Summary, for the node the pods are on.
   Memory pressure, disk pressure, a cordon, or requests at 100% will make Materialize look broken while nothing about
   Materialize is broken.
4. **Is it lagging, or is it stuck?**
   *Environment Overview* → Compute Objects.
   Currently Hydrating counts collections that have produced no results yet; the freshness panels cover the ones that
   have results and are behind.
   A collection is in exactly one of the two, which is what makes them readable together.
5. **Only then, what is it saying?**
   *Logs and Events*, filtered to warnings, over the window the first four steps identified.

Going to logs first is the common mistake.
Logs are the most detailed and least structured evidence you have, and without a window to read them in they are noise.

## Which dashboard answers which question

| Ask | Dashboard |
|---|---|
| Is this environment healthy, and which subsystem is not | Materialize Environment Overview |
| What did Materialize actually say | Materialize Logs and Events |
| Did a rollout do this, and has the new generation caught up | Materialize Upgrade |
| What did the platform underneath say | Infrastructure Logs and Events |
| Is the machine the problem | Infrastructure Node Detail |

The Environment Overview's Summary tab is built to be the first screen: every panel answers one question and names the
tab that explains it.

## Reading the data without being misled

These are the ways this data is honestly confusing.
Each has cost someone real time.

### Lag carries an enormous sentinel

A collection that has not produced results yet reports `u64::MAX` — about `1.8e19` — in
`mz_dataflow_wallclock_lag_seconds`.
Every shipped panel filters it out with `< 1e9`.
Write your own query without that filter and a single unhydrated collection swamps every real reading by nineteen orders
of magnitude.

Those excluded collections are exactly what the hydration panels count, which is why the two are complementary rather
than redundant.

### The metric is a summary with only two quantiles

`quantile="1"` is the worst case, not a p100 estimate, and there is no median to ask for.
Reading it as a percentile will mislead you about how typical the number is.

### Worst-case and total say different things

The worst-case lag is pinned to whichever single collection is furthest behind, so it barely moves while everything else
converges — a cluster can be recovering steadily with a flat maximum.
The total falls with every collection that catches up.
Read both; that is why both are drawn.

### Summing needs deduplication

A collection served by two replicas reports once per replica, so a `sum` doubles where a `max` does not — and two
replicas is the normal shape during a blue/green rollout, which is exactly when you are summing.
The shipped queries handle this with `sum by (…) (max by (…, collection_id) (…))`.
Copy that shape if you write your own.

### An event's namespace is the involved object's, not the reporter's

The operator runs in one namespace and files events about resources in another.
Scoping operator events to the operator's namespace returns nothing, which reads as "no events" rather than as a
mistake.
See [Logs and Events](../../logs-and-events/querying/).

### Empty and missing look identical

A panel reading zero and a panel whose query matched no series look the same at a glance.
Confirm the series exists before concluding the value is zero:

```promql
count(<metric>)
```

This is not hypothetical — a label collision once left an entire dashboard's worth of panels silently empty with every
query still valid.

## Scoping to one environment

Self-managed scopes by **`materialize_cloud_organization_name`**.
The `materialize_cloud_organization_id` label and the `v2_mz_*` metric family are Materialize Cloud only and do not
exist on self-managed.
Getting this wrong returns an empty result that reads as "healthy".

Confirm what a label actually holds on the instance in front of you before building on it.
It costs nothing and settles the question:

```promql
count by (materialize_cloud_organization_name) (mz_dataflow_wallclock_lag_seconds)
```

## Alert definitions are not alerts

> [!WARNING]
> **No alert rules ship today, so silence proves nothing.**

The alert definitions in this repo render to the docsite and describe real failure modes with real thresholds, but they
are **not installed as rules**.
No chart template emits a `PrometheusRule`, and the pre-rendered rule directories are empty, however
`config.rules.prometheus.enabled` is set.

So never reason "nothing is alerting, therefore it is healthy", and do not send someone to check their alerts.
Read the definitions as *thresholds* — they are the closest thing here to "how bad is this number" — and evaluate their
expressions yourself against Thanos.
That is what the alert would have done.

Some definitions also describe components that only exist in Materialize Cloud, so a self-managed install has no data
behind them either way.

## Handing it over

An escalation is useful in proportion to how reproducible it is.

- **The window.** When it started, and what you were looking at when you found it.
- **The scope.** Environment, cluster, replica, namespace, node.
- **The query, not the screenshot.** A PromQL or LogQL expression someone else can run.
- **What you ruled out.** "The node is fine, there was no rollout, this started at 14:02" is worth more than the graph.

When the finding is node- or cluster-shaped, hand it to whoever runs the cluster, with the node name.
That is the boundary where an operator's access usually ends, and the node dashboard is built to produce exactly that
evidence without needing more.

## Handing over to Materialize support

Self-managed changes the problem: **support cannot see your stack.**
Your Grafana is inside your network, so a dashboard link, a saved snapshot, or "it's on the Freshness tab" are all
unreachable from their side.
The artifact has to travel.

There is **no one-shot support bundle in this stack today**, and that is deliberate rather than an oversight — a deep
one-shot capture of a Materialize environment belongs to Materialize itself rather than to its monitoring.
So the artifact is assembled, and the good news is that it is small: three or four files usually settle a question that
a screen-share would take an hour to.

### Send data, not pictures

A screenshot is the least useful thing you can send, because nobody can re-run it.
In rough order of value:

1. **The numbers.** `gcx` writes query results straight to a file, and they attach to a ticket as-is:

   ```bash
   gcx metrics query '<the expression>' --context <ctx> --datasource <prom-ds> \
       --from '2026-09-02T13:00:00Z' --to '2026-09-02T15:00:00Z' -o json > lag.json
   ```

   From the Grafana UI the same thing is **Panel → Inspect → Data → Download CSV**, which needs no CLI.

2. **The expressions and the window**, in absolute UTC rather than `now-1h` — a relative range means something different
   by the time it is read.
   `--share-link` prints the Explore URL; support cannot open it, but the query inside it is the portable part, so paste
   the PromQL or LogQL itself.

3. **The log lines**, exported the same way, scoped to the window you identified rather than the whole day.

4. **What you ruled out**, from the checklist above.

### When the question is Kubernetes-shaped

If the environment's pods are the subject — pending, restarting, evicted — a cluster-side capture says more than any
metric.
The E2E suite's [`dump-diagnostics.sh`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/test/e2e/dump-diagnostics.sh)
is read-only and repurposable for this:

```bash
NAMESPACES="<mz-namespace> monitoring" KUBE_CONTEXT=<ctx> \
    ./test/e2e/dump-diagnostics.sh support-capture
```

It writes pod inventories, events, node state, storage, per-namespace resources, and pod logs including the previous
container into one directory.
Note that some of what it collects is cluster-wide (`get pods -A`, `get nodes`), so it needs broader read access than
the rest of this page assumes — it is read-only throughout, but it is not namespace-scoped.

### If the question is whether collection is working

That is a different artifact and it already exists: the **Alloy support bundle** on the gateway returns rendered config,
component health, and discovered target counts.
Reach for it when the symptom is missing data rather than bad data — it answers "is this stack collecting at all",
which is the question that otherwise gets mistaken for a Materialize fault.

### Before you attach anything

**Logs and query text can contain your data.**
Object names, SQL text, connection strings, and user identifiers all appear in Materialize logs, and a diagnostics
capture includes ConfigMaps.
Review what you are sending, redact what your policy requires, and prefer the narrowest window that still shows the
problem.
Sending less, sooner, beats sending everything after a legal review.
