---
title: "Dependencies"
weight: 20
---

# materialize-monitoring Dependencies

## Installing CRDs

The following CRDs are required for `materialize-monitoring`
to function properly.

* Prometheus Operator CRDs — `ServiceMonitor`, `PodMonitor`, `ScrapeConfig`, `PrometheusRule`, and friends, used to describe what gets scraped and what gets alerted on.
* Grafana Operator CRDs — `Grafana`, `GrafanaDashboard`, `GrafanaDatasource`, `GrafanaFolder`, and friends, used to provision dashboards and datasources.

A second `materialize-monitoring-crds` Helm chart is provided to install these CRDs separately from the main `materialize-monitoring` chart, which is recommended to manage the lifecycle of these CRDs separately from the main chart.

Install it before the main chart, and pass `--skip-crds` when installing the main chart.
The bundled Grafana Operator ships its own copy of the Grafana CRDs and offers no way to opt out of them; `--skip-crds` is what keeps the two charts from contending for the same objects.

## Configuring Storage

Before you start, you need to be able to store your metrics and logs
somewhere.

#### >> I am running in a cloud environment with a managed Kubernetes service (EKS, GKE, AKS, etc.)

TODO: setup bucket with IRSA

#### >> I am running in an on-premises Kubernetes cluster with access to cloud object storage (S3, GCS, Azure Blob Storage, etc.)

TODO: setup bucket with service account credentials
