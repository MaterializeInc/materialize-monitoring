---
title: "Available Dashboards"
weight: 1
---

# Available Dashboards

Thirteen dashboards ship today: five scoped to a Materialize environment (`env-*`), and eight to the platform underneath it (`infra-*`).
Two of the eight watch the monitoring stack itself rather than anything it collects, and are filed in the **Meta Observability** folder.
Each one below has its own download links and its own compatibility annotations.

If you are installing the `materialize-monitoring-dashboards` chart, you do not need to download anything — its `selected` defaults to `["env-*", "infra-*"]`, which is all of them, and the [Grafana Operator]({{< relref "grafana/grafana-operator.md" >}}) path keeps them in sync rather than importing a point-in-time copy.
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

### Materialize Persist (Storage) (`env-persist`)

Object storage, as a Materialize environment experiences it, across four tabs: Overview, Operations, Compaction, and Storage.
Persist is Materialize's storage layer: every durable collection is stored as data files in the persist bucket, and every read, write, and delete Materialize makes against the bucket is measured by the process that made it.
Those measurements are identical on S3, GCS, Azure Blob, and S3-compatible stores, and need no cloud credentials.

The Overview tab answers whether the bucket is failing Materialize: failed operations, read and write latency, write stalls, and compaction failures, with the store's own error text from the logs beneath them.
The Storage tab is Materialize's own accounting of what the stored data is for — current data, data kept only for a reader still looking at an older version, and data nothing refers to.
The bucket's own size, as the provider bills it, is on `infra-cloud`.
It needs both a metrics and a logs datasource.

{{< download-dashboards name="env-persist" >}}

### Materialize Consensus (Metadata) (`env-consensus`)

The metadata database, as a Materialize environment experiences it, across four tabs: Overview, Operations, Connections, and State and Cleanup.
Persist records the current state of every durable collection in the metadata database and commits a new version of that record on every change, and the timestamp oracle keeps every query's timestamps in the same database.
Both clients are measured here, the same way on RDS, Cloud SQL, Azure Flexible Server, CNPG, and CockroachDB.

The Overview tab's verdict row separates the common failures: connection errors, which on a new install are almost always the metadata backend URL, its credentials, or the network path; a rising commit tail, which is the database slowing down; and calls queued for a pooled connection.
The Connections tab shows each process's connection pool, which is what the database counts against its `max_connections`.
State and Cleanup shows whether the table persist writes to is growing, which happens when a reader holds old versions that cleanup cannot delete.
Its logs row carries the database's own error text, which is what distinguishes `connection refused` from `remaining connection slots are reserved`.
It needs both a metrics and a logs datasource.

{{< download-dashboards name="env-consensus" >}}

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

### Infrastructure Networking (`infra-net`)

How traffic moves through the cluster, and what stops it, across six tabs: Overview, Kubernetes, CNI, Node Networking, Cloud Networking, and Security.
Pod and Service traffic come from cAdvisor and kube-state-metrics and are the same everywhere.
The nodes' own interfaces, connection tables, and kernel receive path are read across the whole fleet rather than one machine at a time, which is the difference between this dashboard's Node Networking tab and `infra-nodes`.

**The CNI tab adapts to the cluster it is open on.**
A cluster's container network interface differs per cloud, and the metrics describing each share no names with the others.
The scrape configs label every series they collect with the dataplane they came from, the dashboard discovers that label into its `Dataplane` picker, and each vendor's rows render only where that vendor was found.
A cluster whose CNI exports nothing gets a single row explaining which case it is in, because that is not always a fault: GKE Dataplane V2 runs Cilium and disables the agent's Prometheus endpoint, so nothing can be collected from it.

The Security tab is split the same way and for the same reason.
Which NetworkPolicy objects exist comes from kube-state-metrics and is available everywhere; which packets a policy actually dropped can only come from the CNI.
A cluster showing policies and no enforcement metrics has not demonstrated that any of them work.

Cloud Networking is partly stubbed.
The Kubernetes side of a load balancer — that one was created, and the address it was given — is real; what the load balancer is doing lives at the cloud provider, and collecting it is tracked on the [roadmap]({{< relref "../reference/internal/roadmap.md" >}}#collection-gaps-these-depend-on).

{{< download-dashboards name="infra-net" >}}

### Infrastructure Cloud Provider (`infra-cloud`)

What the cloud provider reports about the metadata database and the buckets a Materialize deployment depends on, across three tabs: Metadata Database, Object Storage, and Collection.
It reads the CloudWatch, Cloud Monitoring, and Azure Monitor metrics the gateway pulls when [cloud provider metrics]({{< relref "../metrics/collecting/cloud-provider-metrics.md" >}}) are enabled, and renders rows only for the provider it finds.
Provider collection is off by default; with none configured, each tab explains what enabling it adds.

The Metadata Database tab leads with the headroom every provider publishes — CPU against the 60% steady-state target Materialize's sizing guidance sets, open connections beside the number Materialize holds, and transaction-ID consumption — followed by the provider's own storage, memory, and disk I/O rows.
The Object Storage tab draws each bucket's billed size beside the data Materialize accounts for; on GCS it also splits live data from noncurrent versions and soft-deleted objects, which a persist bucket can accumulate in quantity.
Provider data is minutes old, and bucket size a day old, so this dashboard says why a dependency is struggling rather than whether it is; `env-consensus` and `env-persist` answer the second question and are the ones to open first.

{{< download-dashboards name="infra-cloud" >}}

### Infrastructure Autoscaling (`infra-autoscaling`)

Whether the cluster's nodes are keeping up with its pods, on any cloud, across six tabs: Overview, Node Pools, Pending Pods, Workload Autoscaling, Cloud Capacity, and Events.
Open it when a Materialize cluster replica will not start, since that is usually a pod waiting for a node.

It is built from what every cluster has, so it reads the same whether Karpenter, GKE's or AKS's cluster autoscaler adds the nodes:
- **Pods waiting for a node, and why.** The scheduler's own explanation for each pod it could not place, followed by what the autoscaler did about it.
- **Nodes by pool, instance type and zone.** Also how full each pool is to the scheduler: requests against allocatable, which is what decides whether a pod fits, rather than usage.
- **HorizontalPodAutoscalers.** Their current, desired, minimum and maximum replicas, and any that cannot read the metric they scale on.
- **What the cloud says about capacity.** EC2 status checks and node group sizes, Compute Engine quota, and AKS's own autoscaler gauges, when [cloud provider metrics]({{< relref "../metrics/collecting/cloud-provider-metrics.md" >}}) are enabled for them.

The per-pool panels need kube-state-metrics to publish node labels, which the chart configures; on an install from before that they say so.

{{< download-dashboards name="infra-autoscaling" >}}

### Karpenter (`infra-karpenter`)

How Karpenter adds, replaces and removes the cluster's nodes on EKS, across five tabs: Overview, Provisioning, Disruption, Controller, and Events and Logs.
It needs the Karpenter ServiceMonitor, which the self-managed Terraform's `karpenter` module creates, and renders nothing but an explanation on a cluster without Karpenter.

- **Provisioning:** what Karpenter launched and what EC2 refused, and which instance types EC2 briefly stopped offering in a zone. Also how long each stage of a launch took, from EC2 accepting it to the node being ready.
- **Disruption:** consolidation, drift and expiry, and what blocks them. On a Materialize cluster that is mostly the `do-not-disrupt` annotation the operator puts on every Materialize pod, since moving a replica means rehydrating it. That reading is the design working; a PodDisruptionBudget blocking the same nodes for hours is not.

{{< download-dashboards name="infra-karpenter" >}}

## Meta monitoring (`infra-*`)

These two dashboards watch the monitoring stack rather than anything it collects.
Their subject can take its own instrumentation down with it, so an empty panel on either can mean the component that would have reported a problem is the one that failed.
Every panel on both states its own empty-state text for that reason.

### Loki Meta Monitoring (`infra-loki`)

Whether the log store is healthy, and which half of it broke if not, across five tabs: Overview, Writes, Reads, Storage, and Logs.
The Overview tab leads with scrape health and with the canary, which writes a line and reads it back and is the only end-to-end check on the store.
It needs both a metrics and a logs datasource.

{{< download-dashboards name="infra-loki" >}}

### Alloy Meta Monitoring (`infra-alloy`)

Whether telemetry collection is healthy, and which stage of it broke if not, across eight tabs: Overview, Log Pipeline, Metric Pipeline, Ingest, Components, Resources, Events, and Logs.
The two pipeline tabs follow the data: a log line from a node through the agent and the gateway into Loki, and a sample from a scrape target through the gateway into each metrics destination.
Ingest covers what other senders push into the gateway over log push, Prometheus remote write, and OTLP, including remote-write senders' own view of their queues.
Components covers configuration loads, uptime, pipeline health, and the gateway's clustering; Resources measures each collector against its CPU, memory, and `GOMEMLIMIT` limits; Events is what Kubernetes reported about the collectors' pods and workloads, and about the configuration-validation Jobs that run before every install and upgrade.
It needs both a metrics and a logs datasource.

**Everything on this dashboard reaches Grafana through the gateway.**
The gateway scrapes Alloy's own metrics and forwards Alloy's own logs, so a dashboard that is empty throughout most likely means the gateway is down.
`kubectl get pods -l app.kubernetes.io/name=alloy-gateway` in the monitoring namespace is the check that does not depend on it.

{{< download-dashboards name="infra-alloy" >}}

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
