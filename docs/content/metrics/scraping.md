---
title: "Scraping"
weight: 15
---

# Scraping Metrics with `materialize-monitoring`

By default, `materialize-monitoring` is configured to scrape metrics from any ServiceMonitor or PodMonitor resources in the cluster.
This allows you to easily add new metrics to your monitoring stack by simply creating a new ServiceMonitor resources.

## Scrape Architecture with Grafana Alloy

`materialize-monitoring` runs Grafana Alloy with a `prometheus.operator` component on `alloy-gateway` instances (Deployment) which reads ServiceMonitors and PodMonitors in order to determine what targets to scrape.

`alloy-gateway` runs in [clustering mode](https://grafana.com/docs/alloy/latest/get-started/clustering/) by default, which means that
scraping is distributed across all replicas of `alloy-gateway` and the scrape state is shared between them.

## ServiceMonitor (monitoring.coreos.com/v1)

ServiceMonitors can be written by any application which indicates
that it should have metrics scraped by Prometheus Operator / Alloy.

By default, `materialize-monitoring` runs Grafana Alloy with a
`prometheus.operator` component on alloy-gateway instances which reads ServiceMonitors and PodMonitors in order to determine what targets to scrape.

ServiceMonitors are preferred over PodMonitors, but both work
relatively the same.
ServiceMonitors just instead look at Service resources for their
EndpointSlices instead of looking at Pods directly.
