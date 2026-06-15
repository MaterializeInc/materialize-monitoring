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

## ServiceMonitor (monitoring.coreos.com/v1)

ServiceMonitors can be written by any application which indicates
that it should have metrics scraped by Prometheus Operator / Alloy.

By default, `materialize-monitoring` runs Grafana Alloy with
`prometheus.operator.*` components on alloy-gateway instances which read ServiceMonitors and PodMonitors in order to determine what targets to scrape.

ServiceMonitors are preferred over PodMonitors, but both work
relatively the same.
ServiceMonitors just instead look at Service resources for their
EndpointSlices instead of looking at Pods directly.

## prometheus.operator Scrape Downloads

These individual files can be used with a manual `prometheus-operator`
(including `kube-prometheus-stack` and `kube-prometheus`)
setup or a less-common manual Grafana Alloy `prometheus.operator` setup.

> [!SUCCESS]
>   These are provided in materialize-monitoring by default.
>   You should not need to download them.
>   These are only for advanced manual cases or reference.

{{< scrapers >}}

## Classic ScrapeConfig Downloads

These are classic scrape_configs for non-operator Prometheus setups.

> [!WARNING]
>   These are provided as best-effort convenience. Prometheus Operator
>   Monitors are the preferred implementation.

{{< scrapers classic=true >}}
