---
title: "Available Dashboards"
weight: 1
---

# Available Dashboards

Five dashboards ship today: three scoped to a Materialize environment (`env-*`), and two to the platform underneath it (`infra-*`).
Each one below has its own download links and its own compatibility annotations.

If you are installing the `materialize-monitoring` chart, you do not need to download anything — `dashboards.selected` defaults to `["env-*", "infra-*"]`, which is all of them, and the [Grafana Operator]({{< relref "grafana/grafana-operator.md" >}}) path keeps them in sync rather than importing a point-in-time copy.
Download them when you are putting them into a Grafana you run yourself; [Importing Dashboards]({{< relref "grafana/importing.md" >}}) covers the ways to do that.

## Formats

Only the **Grafana dashboard schema v2** render exists so far, and it needs **Grafana 12 or later** — see [Grafana compatibility]({{< relref "../reference/compatibility.md" >}}#grafana).
Each table below still lists the other formats, so it is clear which ones a dashboard has no render for yet.
The Grafana 10 and 11 (schema v1) renders and the Datadog, Google Cloud Monitoring, and Honeycomb sets are tracked on the [roadmap]({{< relref "../reference/internal/roadmap.md" >}}#dashboards); until they land, [Common Queries]({{< relref "../reference/stable-metrics/common-queries.md" >}}) is the query material to build one yourself.

### Checking your Grafana version

Navigate to the Grafana instance and click on the "Help" menu (represented by a question mark icon) in the left sidebar.
Selecting it will show the current version.

## Materialize environment (`env-*`)

### Materialize Environment Overview (`env-top`)

The high-level summary, and the one to open first.
Its six tabs — Summary, Kubernetes Workloads, Connections / Activity, Cluster Objects / Replicas, Compute Objects, and Sources and Sinks — are there to catch the more obvious problems and point at what needs a closer look, rather than to answer any one of them in depth.
It reads metrics only, and scopes to one environment at a time, with further pickers for namespace, cluster, and replica.

{{< download-dashboards name="env-top" >}}

### Materialize Logs and Events (`env-logs`)

What the Materialize workloads said, as opposed to what they measured, across two tabs: Logs and Events.
This one is Loki-only — it defines no metrics datasource, and its namespace, app, and level pickers are discovered from Loki — so it keeps working when the metrics pipeline is the thing being investigated.
The namespace picker discovers every namespace and merely *defaults* to the Materialize ones, so the monitoring stack and `kube-system` are one selection away.

{{< download-dashboards name="env-logs" >}}

### Materialize Upgrade (`env-upgrade`)

What happened during an upgrade, from the operator's own account of the rollout alongside what the cluster did underneath it.
The Events tab carries Kubernetes events from the operator and environment namespaces, Generations splits a blue/green rollout into its two sides so the question a rollout actually poses — has the new generation caught up yet — can be asked at all, and Reconciliation is the operator's control loop as metrics.
It needs both a metrics and a logs datasource.

> [!INFO]
> The operator signals this dashboard reads require Materialize **`v26.41.0`**.
> On an older release it degrades unevenly rather than going dark, and [compatibility]({{< relref "../reference/compatibility.md" >}}#materialize-product) has the per-tab detail.

{{< download-dashboards name="env-upgrade" >}}

## Infrastructure (`infra-*`)

### Infrastructure Node Detail (`infra-nodes`)

Everything about one node, one node at a time: Summary, CPU, Memory & Swap, Network, Storage, Pods, and Logs & Events.
The Summary tab is deliberately `kubectl describe node` for someone who cannot run it — identity and capacity, utilization sparklines, how much of the node the scheduler has already promised, and the conditions, cordon state, and taints that explain a node accepting no work.
It needs both a metrics and a logs datasource, the latter for the node journal.

{{< download-dashboards name="infra-nodes" >}}

### Infrastructure Logs and Events (`infra-logs`)

The same question as `env-logs`, asked of the platform a Materialize deployment runs on rather than of Materialize: the monitoring stack, the Kubernetes system components, and the node journal, across Logs, Nodes, and Events tabs.
Also Loki-only.
Where `env-logs` opens on the Materialize namespaces, this one subtracts them, so each dashboard answers for one side of the deployment.

{{< download-dashboards name="infra-logs" >}}

## What the annotations mean

Each render carries its compatibility in the dashboard's own `metadata.annotations`, which is what the Annotations column above shows.
The manifests the [Grafana Operator]({{< relref "grafana/grafana-operator.md" >}}) applies carry the same annotations.

| Annotation | Meaning |
|---|---|
| **Minimum Materialize** (`min-mz-version`) | The oldest Materialize release the dashboard's queries assume. Below it, panels lose data rather than the dashboard breaking. |
| **Recommended Materialize** (`rec-mz-version`) | The release at which every panel has something to show. |
| **SQL metric prefix** (`sql-metric-prefix`) | `mz_` on self-managed. It reaches the cluster-discovery variable, which reads a SQL-derived metric; Materialize Cloud renders use `v2_mz_`. |
| **Grafana folder** (`grafana.app/folder`) | The folder the dashboard is filed under. Grafana's own annotation rather than one of ours, and it holds a folder *name* here: the chart rewrites it to that folder's UID at install time — see [Folders]({{< relref "grafana/grafana-operator.md" >}}#folders). Importing the file by hand does no such rewrite, so choose the folder as you import. |
| **Export target** (`target-export`) | Which variant of the render this is. `generic` is the only one — the GKE-specific variants were retired once the Alloy gateway began scraping the kubelet's cAdvisor directly instead of consuming GKE's reduced allowlist, which left one render per dashboard carrying the same `container_*` and `kube_*` families everywhere. |
