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

The `materialize-sql` scrapers collect SQL-derived metrics from the environmentd `/metrics/mz_compute`, `/metrics/mz_frontier`, `/metrics/mz_storage`, and `/metrics/mz_usage` endpoints. Scrape it as the built-in `mz_support` role.

The **Classic** and **Google Cloud Managed Prometheus** configs carry `username: mz_support` inline, so they need no extra setup.
Prometheus Operator `basicAuth` can only reference a Kubernetes Secret — it has no inline username field — so the `materialize-sql` `PodMonitor` reads the username from a Secret named `materialize-sql-monitor`.
Create it in the namespace the scrapers run in (for example, `materialize`):

```bash
kubectl create secret generic materialize-sql-monitor \
  --namespace materialize \
  --from-literal=username=mz_support
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
