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

## Label curation

Curate labels at the source rather than promoting everything and cleaning up downstream.

- **PodMonitors** set an explicit `podTargetLabels` allowlist (the org / cluster / replica ids the dashboards consume), not a blanket promotion.
- **`scrapeconfig-cadvisor.yaml`** uses an explicit node-label allowlist (a `replace` per kept label) instead of `labelmap __meta_kubernetes_node_label_(.+)`, which would drag every node label (karpenter/cluster-autoscaler/hostname) onto every cAdvisor series. It also sets `node` from the node name explicitly, since `__address__` is the apiserver proxy and identical for every node.

Cross-cutting target-phase rules that apply to *all* monitors (pod-liveness drop, `app`-label coalescing) live in the alloy `prometheus.operator.*` components' `rule` blocks (`packages/alloy-pipelines/gateway.yaml`), not repeated here.
See `docs/content/reference/internal/pipelines/metrics.md` for the full relabeling model.

## Relation to the materialize-monitoring Chart

These scrapers are transformed slightly before being included in the helm chart.
This mostly prepends standard prefixes, allows for overrides, and adds some labels.
