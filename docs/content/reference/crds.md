---
title: "Custom Resource Definitions (CRDs)"
weight: 50
---

# Custom Resource Definitions (CRDs)

Kubernetes Custom Resource Definitions (CRDs) are a powerful way to extend the Kubernetes API with custom resources that can be managed using standard Kubernetes tools.
`materialize-monitoring` relies on several CRDs to function properly, and this section provides an overview of these CRDs and how to install them.

## ServiceMonitor (monitoring.coreos.com/v1)

ServiceMonitors can be written by any application which indicates
that it should have metrics scraped by Prometheus Operator / Alloy.

By default, `materialize-monitoring` runs Grafana Alloy with a
`prometheus.operator` component on alloy-gateway instances which reads ServiceMonitors and PodMonitors in order to determine what targets to scrape.

ServiceMonitors are preferred over PodMonitors, but both work
relatively the same.
ServiceMonitors just instead look at Service resources for their
EndpointSlices instead of looking at Pods directly.
