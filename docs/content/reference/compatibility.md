---
title: "Compatibility"
weight: 40
---

# Release Compatibility

This shows the compatibility of materialize-monitoring with various
technologies.

## [materialize-terraform-self-managed](https://github.com/MaterializeInc/materialize-terraform-self-managed)

No version of materialize-terraform-self-managed includes
materialize-monitoring currently with the exception of its dashboard.

## [Materialize product](https://github.com/MaterializeInc/materialize)

Dashboards (v0.8.0+) generally require the `mz_object_info` metric introduced in `v26.29.0`,
however they will gracefully degrade without it.

Scrapers (v0.1.1+) require `app.kubernetes.io/name` labels for environmentd introduced in `v26.24.0`.

## [Grafana](https://grafana.com/)

Dashboards (v0.8.0+) require Grafana v13+ for the dashboard schema v2.
Grafana v12 is generally known to work, but may run into issues.

## [Google Kubernetes Engine (GKE)](https://cloud.google.com/kubernetes-engine)

GKE (v1.29+ known to work) is known to work generally with the dashboards if [metric collection is enabled](https://docs.cloud.google.com/stackdriver/docs/managed-prometheus/setup-managed#enable-mgdcoll-gke) (on by default starting in v1.27).
GKE does not expose all cAdvisor and kube-state-metrics metrics, so
the GCP-optimized dashboards do not include some %-based metrics that
would normally be available.

## [Google Cloud Monitoring Dashboards](https://cloud.google.com/monitoring/dashboards)

Importing Grafana dashboards into Google Cloud Monitoring is not yet fully supported.

The GCP optimized dashboards do have some improvements but there are
still some known issues:
* `$__range` and `$__rate_interval` need to be replaced with constants
* Horizontal bar charts are not rendered
* Tabs and rows are not supported, so there is no layout
* Switch variables are not supported (e.g. "show system clusters")
  * This breaks the downstream cluster selector variable
