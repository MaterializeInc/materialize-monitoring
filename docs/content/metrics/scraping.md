---
title: "Scraping"
weight: 15
---

# Scraping Metrics with `materialize-monitoring`

{{< wip >}}

By default, `materialize-monitoring` is configured to scrape metrics from any ServiceMonitor or PodMonitor resources in the cluster.
This allows you to easily add new metrics to your monitoring stack by simply creating a new ServiceMonitor resources.

## Scrape Architecture with Grafana Alloy

{{< wip >}}

`materialize-monitoring` runs Grafana Alloy with `prometheus.operator` components on `alloy-gateway` instances (Deployment) which read ServiceMonitors and PodMonitors in order to determine what targets to scrape.

`alloy-gateway` runs in [clustering mode](https://grafana.com/docs/alloy/latest/get-started/clustering/) by default, which means that
scraping is distributed across all replicas of `alloy-gateway` and the scrape state is shared between them.

<!--
## ServiceMonitor (monitoring.coreos.com/v1)

ServiceMonitors can be written by any application which indicates
that it should have metrics scraped by Prometheus Operator / Alloy.

By default, `materialize-monitoring` runs Grafana Alloy with
`prometheus.operator.*` components on alloy-gateway instances which read ServiceMonitors and PodMonitors in order to determine what targets to scrape.

ServiceMonitors are preferred over PodMonitors, but both work
relatively the same.
ServiceMonitors just instead look at Service resources for their
EndpointSlices instead of looking at Pods directly.
-->

## Manually Configured Scraping

If you are not using default `materialize-monitoring` setup, you can use the following
scrape configuration files as a starting point for your own Prometheus setup.

### Authenticating the SQL metrics endpoint

The `materialize-sql` scrapers collect SQL-derived metrics from the environmentd `/metrics/mz_compute`, `/metrics/mz_frontier`, `/metrics/mz_storage`, and `/metrics/mz_usage` endpoints.
The `/metrics/mz_compute` endpoint evaluates its metrics as the Materialize user the scrape request authenticates as.
That user only sees the clusters it has `USAGE` on, so the metrics only cover those clusters.
To collect compute metrics for every cluster, authenticate as a dedicated monitoring role that has been granted `USAGE` on all of them.

> [!INFO]
>   Only the `mz_compute` endpoint requires credentials today.
>   The `mz_frontier`, `mz_storage`, and `mz_usage` endpoints are scraped without authentication.

#### 1. Create a monitoring role with `USAGE` on every cluster

Connect to Materialize as a user that can manage roles and grant the monitoring role `USAGE` on each cluster:

```sql
CREATE ROLE materialize_monitor;

GRANT USAGE ON CLUSTER quickstart TO materialize_monitor;
-- Repeat for every cluster you want compute metrics for.
```

Re-run the `GRANT USAGE ON CLUSTER ... TO materialize_monitor` statement whenever you add a cluster.
Otherwise that cluster's compute metrics will be missing from the scrape.

> [!INFO]
>   **Alternative: scrape as a superuser.**
>   A superuser bypasses per-object privilege checks, so it sees every cluster — including clusters added later — without any `GRANT USAGE` statements.
>   Either way, the scrape config is identical — only the credentials in the Secret change.

The role authenticates over the SQL protocol, so it also needs login credentials (for example, `CREATE ROLE materialize_monitor WITH PASSWORD '<password>'`).

#### 2. Store the credentials in a Kubernetes Secret

The `materialize-sql` PodMonitor (Prometheus Operator) and the `materialize-sql-mz-compute` PodMonitoring (GMP) reference a Secret named `materialize-sql-monitor` for these credentials.
Create it in the namespace the scrapers run in (for example, `materialize`):

```bash
kubectl create secret generic materialize-sql-monitor \
  --namespace materialize \
  --from-literal=username=materialize_monitor \
  --from-literal=password='<password>'
```

#### 3. Reference the credentials from the scrape config

Each format supplies the credentials differently:

- **Prometheus Operator** — `basicAuth` references the `materialize-sql-monitor` Secret for both the username and password.
  No edit is needed beyond creating the Secret in the same namespace as the PodMonitor.
- **Google Cloud Managed Prometheus** — `basicAuth` reads the password from the same Secret and takes the username inline.
  Replace the placeholder `materialize_monitor` username if you named the role differently.
- **Classic Prometheus** — classic Prometheus cannot read a Kubernetes Secret, so the example uses inline placeholders.
  Replace `REPLACE_WITH_PASSWORD` with the role's password.
  To avoid committing the password, mount it as a file and use `password_file` instead:

  ```yaml
  basic_auth:
    username: materialize_monitor
    password_file: /etc/prometheus/secrets/materialize-sql-monitor/password
  ```

### Which Prometheus Distribution Am I Using?

An easy way to check if you are using Prometheus Operator is to see
if you have `PodMonitor` or `ServiceMonitor` CRDs in your cluster:

```bash
# Get the CRD resource directly:
kubectl get crd podmonitors.monitoring.coreos.com servicemonitors.monitoring.coreos.com
# This may fail with NotFound if you you do not have Prometheus Operator.
# If you get an kind of permission error, you should try the api-resources command:
kubectl api-resources | grep monitoring.coreos.com
# This is empty if you do not have Prometheus Operator
```

> [!INFO]
>   `monitoring.coreos.com` is the group for Prometheus Operator CRDs.

Alternatively, you can check this table of common Prometheus Distributions if you remember how you installed it:

{{< details "List of Common Prometheus Distributions" >}}
| Distribution | Monitor Format | Install Methods | Notes |
|--------------|----------------|-----------------|-------|
| [materialize-monitoring](https://materializeinc.github.io/materialize-monitoring/) | [Prometheus Operator](#prometheus-operator) | Helm, Terraform | The default monitoring stack for Materialize. Uses Grafana Alloy with `prometheus.operator` components. |
| [kube-prometheus-stack](https://github.com/prometheus-community/helm-charts/tree/main/charts/kube-prometheus-stack) | [Prometheus Operator](#prometheus-operator) | Helm | prometheus-community Helm Chart of Prometheus Operator. |
| [kube-prometheus](https://github.com/prometheus-operator/kube-prometheus) | [Prometheus Operator](#prometheus-operator) | Helm | Helm distribution of Prometheus Operator. |
| [prometheus-operator](https://prometheus-operator.dev/) ([Github](https://github.com/prometheus-operator/prometheus-operator)) | [Prometheus Operator](#prometheus-operator) | Manual | The upstream distribution of Prometheus Operator |
| [Bitnami kube-prometheus](https://github.com/bitnami/charts/tree/main/bitnami/kube-prometheus) | [Prometheus Operator](#prometheus-operator) | Helm | Bitnami's Helm distribution of Prometheus Operator. |
| [Bitnami Prometheus chart](https://github.com/bitnami/charts/tree/main/bitnami/prometheus) | [Classic](#classic) | Helm | Bitnami's Helm distribution of Prometheus without Operator. |
| [k8s-monitoring-helm](https://grafana.com/docs/grafana-cloud/monitor-infrastructure/kubernetes-monitoring/configuration/helm-chart-config/) ([Github](https://github.com/grafana/k8s-monitoring-helm/tree/main/charts/k8s-monitoring)) | [Prometheus Operator](#prometheus-operator) | Helm | Grafana's Kubernetes monitoring Helm chart. |
| [prometheus.io](https://prometheus.io/docs/prometheus/latest/installation/) | [Classic](#classic) | Source, Binary, Docker | Download of binary prometheus (brew, apt, et al.) or Docker image |
| [thanos-community helm](https://github.com/thanos-community/helm-charts/tree/master/charts/thanos) | [Prometheus Operator](#prometheus-operator) | Helm | Only if kube-prometheus-stack.enabled=true, otherwise refer to another technology. |
| [Amazon Managed Prometheus (AMP)](https://aws.amazon.com/prometheus/) | [Classic](#classic) | AWS | Amazon's managed service for Prometheus. Not enabled by default. |
| [Google Cloud Managed Service for Prometheus (GMP)](https://cloud.google.com/stackdriver/docs/managed-prometheus) | [Google Cloud Monitoring](#gmp) | Google Cloud (GCP/GKE) | Google's managed service for Prometheus. Enabled by default with GKE. |
| [Grafana Alloy with prometheus.operator](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.servicemonitors/) | [Prometheus Operator](#prometheus-operator) | Grafana Alloy | Grafana Alloy's implementation of Prometheus Operator.|

The following distributions are known to not work at this time:
* VictoriaMetrics
* Grafana Mimir
* Cortex
{{< /details >}}

> [!WARNING]
>   If you are using an OpenTelemetry Database (Honeycomb, Datadog, etc.), you should not use these scrape configurations.
>   Native OTLP collection will be provided in a future release.

### Prometheus Operator Scrape Downloads {#prometheus-operator}

These individual files can be used with a manual `prometheus-operator`
(including `kube-prometheus-stack` and `kube-prometheus`)
setup or a less-common manual Grafana Alloy `prometheus.operator` setup.

{{< scrapers pattern="prometheus-scrapers/prometheus-operator/*.yaml" >}}

#### Installing Prometheus Operator Scrape Configurations

The above files are meant to be used as manifests that can be passed
to `kubectl apply` directly.
The namespace isn't generally too important, but you may elect to
put them alongside your materialize-operator resource.

If your `materialize-operator` is in the `materialize` namespace, you can download each into a direcotory and apply like:
```bash
kubectl apply -f scrapers/ -n materialize
```

### Classic ScrapeConfig Downloads {#classic}

These are classic scrape_configs for non-operator Prometheus setups.
These are placed into your Prometheus configuration (prometheus.yml) as a single scrape_config.

> [!WARNING]
>   These are provided as best-effort convenience. Prometheus Operator
>   Monitors are the preferred implementation.

{{< scrapers pattern="prometheus-scrapers/classic/*.yaml" >}}

See [Prometheus Configuration](https://prometheus.io/docs/prometheus/latest/configuration/configuration/)
for information on how to configure scrape_configs in your Prometheus setup.

### Google Cloud Managed Service for Prometheus PodMonitoring {#gmp}

These are PodMonitoring resources specifically for [Google Cloud Managed Service for Prometheus (GMP)](https://docs.cloud.google.com/stackdriver/docs/managed-prometheus).

> [!WARNING]
>   These are provided as best-effort convenience.

{{< scrapers pattern="prometheus-scrapers/gmp/*.yaml" >}}

> [!INFO]
>   cAdvisor metrics are collected by Google Cloud Managed Service for Prometheus by default.
