# Dependencies




# materialize-monitoring Dependencies

What has to exist before `materialize-monitoring` installs, and who provides it.

Almost everything on this page is the `[consumer]` half of the [shared responsibility model](/materialize-monitoring/preview/heather-llms-txt/operating/production-best-practices/#shared-responsibility-model): cloud resources, secrets, and cluster facilities the chart consumes **by name** but never creates.
That is worth walking before the first install rather than after, because the common failure mode is not a rejected install — it is a chart that renders cleanly and a workload that crash-loops or sits `Pending` minutes later.

> [!TIP]
>  Installing with the [Terraform modules](/materialize-monitoring/preview/heather-llms-txt/getting-started/terraform/)? They create most of this for you — buckets, identity bindings, the Grafana database, the load balancer.
>  Skip to [If you install with Terraform](#terraform) for what is left.

## At a glance

| Dependency | Needed when | Provided by |
|---|---|---|
| [A supported Kubernetes](#kubernetes) — v1.34+ | Always | Your cluster |
| [CRDs](#crds) — Prometheus Operator + Grafana Operator | Always | The `materialize-monitoring-crds` chart |
| [Object storage](#object-storage) + credentials | Unless both backends are external | You, or Terraform |
| [A StorageClass the nodes can attach](#persistent-volumes) | Unless every volume is disabled | Your cluster |
| [Materialize and `materialize-operator`](#materialize) | To scrape Materialize at all | You |
| [The `materialize-sql-monitor` Secret](#the-sql-metrics-credential) | For the SQL-derived metrics | You |
| [PostgreSQL](#postgresql-for-grafana) | For a durable, multi-replica Grafana | You, or Terraform |
| [cert-manager](#cert-manager) | Only with `certificates.enabled` | You |
| [Zone labels on nodes](#zone-labels) | Unless you apply `no-zone-spread` | Your cluster |

## Kubernetes {#kubernetes}

**We support the non-EOL Kubernetes releases — v1.34 and newer.** Helm 3 is assumed throughout.

The charts declare `kubeVersion: ">=1.27.0-0"`, but that is a floor rather than a support statement: older releases *may* work and are not supported.
Cloud distributions offering extended support past a release's upstream EOL are supported best-effort and may run into issues.
See [Compatibility](/materialize-monitoring/preview/heather-llms-txt/reference/compatibility/).

Capacity matters more than version. The chart defaults target a **medium** install and request roughly **33Gi of memory and 9 CPU** in total, which no small cluster and no standard CI runner will schedule — the pods simply sit `Pending`, which reads like a chart bug rather than a capacity one.
Pick a [sizing profile](/materialize-monitoring/preview/heather-llms-txt/getting-started/helm/#sizing) that matches the cluster you actually have.

## CRDs {#crds}

Two sets of Custom Resource Definitions have to exist before the main chart installs:

* **Prometheus Operator CRDs** — `ServiceMonitor`, `PodMonitor`, `ScrapeConfig`, `PrometheusRule`, and friends. These describe what gets scraped; the Alloy gateway reads them.
* **Grafana Operator CRDs** — `Grafana`, `GrafanaDatasource`, `GrafanaManifest`, `GrafanaDashboard`, and friends. The chart creates these to provision Grafana and its dashboards.

A second chart, `materialize-monitoring-crds`, installs them, so their lifecycle is managed separately from the stack that uses them.
Install it first, then install the main chart with `--skip-crds`:

```bash
helm install mzmon-crds oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-crds --namespace monitoring
```

`--skip-crds` on the main chart is not optional tidiness.
The bundled Grafana Operator ships its own copy of the Grafana CRDs and offers no way to opt out of them, so without it a fresh install creates those definitions behind the CRDs chart's back and the two releases contend for the same cluster-scoped objects.

To check what a cluster already has:

```bash
kubectl get crds | grep -E 'monitoring\.coreos\.com|grafana\.integreatly\.org'
```

[Custom Resource Definitions](/materialize-monitoring/preview/heather-llms-txt/reference/crds/) is the full inventory — every kind, what the stack does with each, how to disable the ones you do not need, and the two groups (Google Managed Prometheus, cert-manager) that this chart deliberately does not install.

## Object storage {#object-storage}

Loki and Thanos both keep their data in object storage.
You need it unless **both** backends are external — pointing logs at an existing Loki and metrics at an existing Prometheus-compatible store leaves nothing to persist locally.

**Two buckets (or containers) are the norm, one per backend.** Loki and Thanos want different lifecycle rules, and per-bucket IAM keeps each backend out of the other's data.

### Granting access

On a managed Kubernetes service, grant access through **workload identity** rather than static keys.
The binding is per-cloud, and the trust-policy subjects have to match the chart's rendered ServiceAccount names:

* [Logs & Events → Storing](/materialize-monitoring/preview/heather-llms-txt/logs-and-events/storing/#granting-object-storage-access-workload-identity) — IRSA, GKE Workload Identity, and Entra Workload ID
* [Metrics → Storing](/materialize-monitoring/preview/heather-llms-txt/metrics/storing/#authentication) — the same three for Thanos

Without a cloud identity provider there is no identity to bind, so credentials become **static keys supplied as a Secret**.
That is the documented escape hatch rather than the recommended path; both backends read them by reference, and the Terraform module exposes them as `object_storage_access_key_id` / `object_storage_secret_access_key`.

Any S3-compatible endpoint works. This repository's own tier-2 E2E runs against [rustfs](https://github.com/rustfs/rustfs), so a self-hosted MinIO, Ceph, or rustfs is a supported shape rather than an untested one.

> [!WARNING]
>  **Naming the backend is the step that most often goes wrong.**
>  The chart's defaults are S3-shaped, so any other backend has to be named in three load-bearing places, and none of them fails softly: the client is chosen by name and then validated against a config that was never populated, so the component crash-loops.
>  See [Selecting the backend](/materialize-monitoring/preview/heather-llms-txt/logs-and-events/storing/#selecting-the-backend).

The example profiles do all of it correctly and are the shortest path:

| Profile | Backend |
|---|---|
| `profiles/aws-example.values.yaml` | S3 + IRSA |
| `profiles/gcp-example.values.yaml` | GCS + GKE Workload Identity |
| `profiles/azure-example.values.yaml` | Blob + Entra Workload ID |
| `profiles/aws-amp-fanout.values.yaml` | S3 for logs; metrics to the bundled Thanos **and** Amazon Managed Prometheus at once |

## Persistent volumes {#persistent-volumes}

Most of this stack is volumeless on purpose — durability comes from the replication factor or from object storage, and a volume that cannot cross availability zones makes a zone failure worse rather than safer.

Four workloads do claim one, so a **StorageClass has to exist**:

| Workload | What the volume holds |
|---|---|
| Alertmanager | Silences and the notification log — the only state here that is genuinely not reconstructible |
| The Loki ruler | Its remote-write WAL, buffering recording-rule samples while the metric store is unreachable |
| The Thanos Store Gateway | The on-disk index-header cache: cheap to lose, slow to rebuild |
| The Thanos Compactor | Scratch for the block group under compaction — tens of GiB, more than a typical node's allocatable ephemeral storage |

Use `profiles/storage-class.values.yaml` (one anchor to edit) or the Terraform module's `storage_class` variable, which carries the same map.

> [!WARNING]
>  **A class that cannot serve the nodes is not the same as a missing class.**
>  On GCP's C4 and N4 machine families, which accept only Hyperdisk, *every* StorageClass GKE creates by default is Persistent Disk and none of them will attach — the PVC binds and the pod never starts.

Changing the class later does not move the volumes: `volumeClaimTemplates` are immutable, so the old PVCs must be deleted first, discarding their contents.

## Materialize {#materialize}

The stack runs happily with no Materialize in the cluster — it just has nothing to scrape.
To collect Materialize metrics you need `materialize-operator` installed, and the chart needs to know where it and the environments live:

```yaml
materialize-operator:
  namespace: materialize
materialize-system:
  namespace: materialize-environment
materialize:
  namespaces: [] # empty means every namespace the chart can read
```

Version requirements are in [Compatibility](/materialize-monitoring/preview/heather-llms-txt/reference/compatibility/). In short: the scrapers need the `environmentd` labels introduced in Materialize **v26.24.0**, and the dashboards want the `mz_object_info` metric introduced in **v26.29.0** — without it they degrade gracefully rather than break.

### The SQL metrics credential {#the-sql-metrics-credential}

The `materialize-sql` scrapers read SQL-derived metrics from `environmentd` as the built-in, password-less `mz_support` role.
Prometheus Operator `basicAuth` can only reference a Secret — it has no inline fields — so this one Secret has to exist, in the namespace the scrapers run in:

```bash
kubectl create secret generic materialize-sql-monitor \
  --namespace materialize \
  --from-literal=username=mz_support \
  --from-literal=password=
```

The empty `password` is deliberate, and not a placeholder.
**This endpoint does not validate passwords** — the username selects the role the queries run as, and the password field is never read.
It has to be present only because Alloy's scrape-config generation rejects an absent password reference with `resource name may not be empty`.

Set `materialize.environmentdSQL.secret.create: false` if you provision the Secret yourself.
See [Scraping](/materialize-monitoring/preview/heather-llms-txt/metrics/scraping/#authenticating-the-sql-metrics-endpoint), and [Securing](/materialize-monitoring/preview/heather-llms-txt/operating/securing/#materialize-metrics-endpoint) for the access-control consequence.

## Conditional dependencies

### PostgreSQL for Grafana {#postgresql-for-grafana}

Grafana keeps its own state — users, service accounts and tokens, annotations, dashboard versions and permissions, alert-rule state — in a database separate from the observability data in Thanos and Loki.

The chart default is SQLite on an `emptyDir`, which loses that state on every pod restart.
SQLite on a PersistentVolume survives restarts but tolerates one writer, capping you at a single replica.
**An external PostgreSQL is the only shape that lifts both constraints**, which is why it is the production one — and why exposing Grafana without it turns a bundled extra nobody depended on into a primary interface that silently discards what users build in it.

`profiles/grafana-postgres.values.yaml` assembles it. Three things it does not create:

1. A database and a user that **owns** it — Grafana runs its own schema migrations at startup, so a read/write-only grant fails the migration.
2. A Secret holding the password, in the namespace the Grafana *pod* runs in.
3. Network reach from that pod to the database.

> [!INFO]
>  RDS and Cloud SQL both support IAM database authentication, but Grafana reads its password once at startup and has no refresh hook, so the 15-minute (RDS) or one-hour (Cloud SQL) token expiry breaks the first reconnect. Use a static password.

The Terraform modules provision a dedicated small PostgreSQL instance for this, so on that path it is handled.

### cert-manager {#cert-manager}

Only needed with `certificates.enabled`, which **defaults to `false`** — with it off, cert-manager is not a dependency of this chart in any sense and nothing TLS-related renders.

With it on, the chart renders `Certificate` resources for in-cluster mTLS, and optionally a self-signed root.
This is gated on a value rather than an API-capability probe, deliberately: enabling it without cert-manager present fails at apply time with a resource name in the error, which is a better outcome than silently rendering nothing.
See [Securing](/materialize-monitoring/preview/heather-llms-txt/operating/securing/#certificates).

### Reaching Grafana {#reaching-grafana}

Grafana is the only component in this stack meant for a human, and the only one the chart will help you expose.
An Ingress or LoadBalancer Service is a prerequisite for anyone actually using it, and **DNS, TLS, and an identity provider remain yours** on every path, including Terraform.

`profiles/grafana-ingress.values.yaml` assembles the shape — internal by default, public gated behind an enforced allowlist.
Pair it with a persistence profile above; the render-time checks will tell you if you exposed Grafana without authentication or without a durable backend.
See [Securing](/materialize-monitoring/preview/heather-llms-txt/operating/securing/) for what each exposure combination is graded as.

## Cluster-shape prerequisites

These are properties of the cluster rather than things you install. Each one produces a *quiet* failure, which is why they are listed.

### Zone labels {#zone-labels}

Thanos Receive and Loki's ingesters spread **hard** across zones (`DoNotSchedule`), so that an un-spread pod goes `Pending` and the autoscaler provisions the missing zone.

On a cluster whose nodes carry **no zone label, or only one zone**, those pods stay `Pending` permanently.
Apply `profiles/no-zone-spread.values.yaml` — see [Production Best Practices](/materialize-monitoring/preview/heather-llms-txt/operating/production-best-practices/#thanos-few-zones).

### Kubelet certificates {#kubelet-certificates}

Container metrics arrive because the Alloy gateway scrapes `/metrics/cadvisor` on every kubelet, on by default.
A distribution that signs kubelet certificates with a CA the pods do not trust **fails that scrape quietly** — nothing errors at install, the Kubernetes dashboards are simply empty.

`kind` is the known case. Check `up{job="cadvisor"}` when bringing up a new cluster, and set `pipeline.metrics.kubelet.tlsInsecureSkipVerify: true` only where you have to; leaving verification on is correct everywhere real.

### A cluster that already serves the metrics API {#metrics-api}

`tags.default` installs `metrics-server`, and the `v1beta1.metrics.k8s.io` APIService it registers is cluster-singleton.
Most managed clusters already provide it — GKE always, EKS via addon, AKS by default.
Set `metrics-server.enabled: false` there.

`metrics-server` is deliberately left on the upstream `system-cluster-critical` priority class, because it backs the metrics API that HPAs and the Materialize Console read: it is cluster plumbing rather than monitoring.

### GKE managed collection {#gke}

On GKE, managed collection is on by default from v1.27 and provides the `PodMonitoring` CRDs the GMP scrapers use.

Container metrics do not depend on it. The gateway scrapes each kubelet's `/metrics/cadvisor` endpoint itself, which yields a fuller set than GMP's default collection, so the GCP-optimized dashboards carry the same metric families as the standard ones.
See [Container metrics from the kubelet](/materialize-monitoring/preview/heather-llms-txt/metrics/scraping/#kubelet).

## If you install with Terraform {#terraform}

The [Terraform modules](/materialize-monitoring/preview/heather-llms-txt/getting-started/terraform/) cover the cloud-resource dependencies above: buckets, the IAM role or service account with its workload-identity binding, the Grafana database, an internal load balancer, and the CRDs chart.

What stays yours on that path:

* **DNS, TLS, and an identity provider** for Grafana.
* **The `materialize-sql-monitor` Secret**, if you want the SQL-derived metrics.
* **Cluster shape** — zone labels, a StorageClass the nodes can attach, capacity for the sizing tier you pick.
* **Rolling Alloy after a pipeline or filter change.** The module hashes the composed values to do this for you; on the Helm path it is yours, and it is the one place the chart cannot own its own rollout. See [Production Best Practices](/materialize-monitoring/preview/heather-llms-txt/operating/production-best-practices/#collection-alloy).

## Next steps

Once the dependencies are in place: [Installing via Terraform](/materialize-monitoring/preview/heather-llms-txt/getting-started/terraform/) or [Installing via Helm](/materialize-monitoring/preview/heather-llms-txt/getting-started/helm/).

