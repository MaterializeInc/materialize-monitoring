# Prometheus-style Scrapers

This package contains Kubernetes manifests for Prometheus-style scrapers.
The manifests are written in prometheus-operator format (v1 ServiceMonitor, v1 PodMonitor, and v1alpha1 ScrapeConfig).

A standard materialize-monitoring Helm deployment uses Grafana Alloy's
`prometheus.operator.*` components to simulate prometheus operator scraping
behavior before shuttling information to the metric pipeline.

WARNING: Monitors should be moved to the materialize-operator helm chart
rather than live here.

## Adding New Scrapers

This should only contain gaps in monitoring behavior.
It is desirable to have Monitors live with their respective applications
rather than in this repository.

Prefer ServiceMonitors over PodMonitors where possible.
Older versions of Materialize do not expose easily queriable labels
on services, so pod monitors are forced to be used.
ServiceMonitors, contrary to their name, actually select pods based on their
endpoints.

## Relation to the materialize-monitoring Chart

These scrapers are transformed slightly before being included in the helm chart.
This mostly prepends standard prefixes, allows for overrides, and adds some labels.
