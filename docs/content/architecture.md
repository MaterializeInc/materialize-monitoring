---
title: "Architecture"
weight: 18
---

# Architecture Overview

This section provides an overview of the architecture of `materialize-monitoring` and its components.

## `materialize-monitoring` Helm Umbrella Chart

An Umbrella Helm Chart is a Helm Chart that orchestrates the installation of multiple dependent charts.

The `materialize-monitoring` Helm Chart is an Umbrella Chart that orchestrates the installation of the following dependent charts:
- `alloy-agent` (Grafana Alloy, Agent DaemonSet): o11y Pipelines
- `alloy-gateway` (Grafana Alloy, Gateway Deployment): o11y Pipelines
- `metrics-server` (metrics-server): cAdvisor/container runtime Metrics
- `kube-state-metrics` (kube-state-metrics): Kubernetes Metrics
- `node-exporter` (node-exporter): Node Metrics
- `loki` (Grafana Loki): Default Logging Infrastructure
- `thanos` (Thanos): Default Metrics Storage and Querying Infrastructure
- `grafana` (Grafana): Default Dashboarding and Visualization Infrastructure
- `grafana-operator` (Grafana Operator): Dashboards-as-Code Infrastructure
- `alertmanager` (Prometheus Alertmanager): Default Alerting Infrastructure

In addition to these dependent charts, `materialize-monitoring`
also provides many opionated configurations such as o11y pipelines, Grafana dashboards, Scrape configurations, and Prometheus recording and alerting rules.

## `alloy-agent`: Grafana Alloy Agent DaemonSet

`alloy-agent` is a [Grafana Alloy](https://grafana.com/docs/alloy/latest/introduction/) Agent DaemonSet that runs on every node in the cluster and is responsible for collecting logs from the node and forwarding them to the [`alloy-gateway`](#alloy-gateway-grafana-alloy-gateway-deployment).

## `alloy-gateway`: Grafana Alloy Gateway Deployment

`alloy-gateway` is a [Grafana Alloy](https://grafana.com/docs/alloy/latest/introduction/) Gateway Deployment that is responsible for the main observability pipeline processing and forwarding.

Logging responsibilities of `alloy-gateway` include:
* A [`loki.source.api`](https://grafana.com/docs/alloy/latest/reference/components/loki/loki.source.api/) component receives logs from [`alloy-agent`](#alloy-agent-grafana-alloy-agent-daemonset) and processes them as logs.
* A [`loki.source.kubernetes_events`](https://grafana.com/docs/alloy/latest/reference/components/loki/loki.source.kubernetes_events/) component collects Kubernetes events and processes them as logs.
* A [`loki.process`](https://grafana.com/docs/alloy/latest/reference/components/loki/loki.process/) pipeline performs log processing
* A [`loki.write`](https://grafana.com/docs/alloy/latest/reference/components/loki/loki.write/) component forwards logs to log storage (e.g., [Grafana Loki](#loki))

Metrics responsibilities of `alloy-gateway` include:
* [`prometheus.operator.servicemonitors`](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.servicemonitors/) and [`prometheus.operator.podmonitors`](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.podmonitors/) components read ServiceMonitors and PodMonitors in order to determine what targets to scrape for metrics and then scrapes those targets.
* A [`prometheus.enrich`](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.enrich/) pipeline performs metric processing and enrichment on scraped metrics.
* A [`prometheus.remote_write`](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.remote_write/) component forwards metrics to metric storage (e.g., [Thanos](#thanos)).
* An [`otelcol.exporter.otlp`](https://grafana.com/docs/alloy/latest/reference/components/otelcol/otelcol.exporter.otlp/) component supports forwarding to an external OTLP endpoint (e.g., Honeycomb, Datadog, New Relic, etc.) for metrics and logs.

Alloy supports further customization to integrate with an existing
observability infrastructure.

## `metrics-server`: Container Metrics API

[`metrics-server`](https://github.com/kubernetes-sigs/metrics-server) is a Kubernetes Metrics API implementation that collects resource usage metrics from the kubelet on each node and exposes them via the Kubernetes Metrics API.

Do note that the `metrics-server` is primarily intended for decision-based components (like Horizontal Pod Autoscaler) and does not store historical metrics data.
Nonetheless, Materialize relies on cluster-local metrics about its containers
so this is required to not rely on external metrics sources for this data.

## `kube-state-metrics`: Kubernetes Metrics

[`kube-state-metrics`](https://kubernetes.io/docs/concepts/cluster-administration/kube-state-metrics/) is a service that listens to the Kubernetes API server and generates metrics about the state of the objects in the cluster (e.g., Deployments, Pods, Services, etc.).

This does not provide information about resource usage of individual
containers.

## `node-exporter`: Node Metrics

[`node-exporter`](https://github.com/prometheus/node_exporter) is a Prometheus exporter that collects hardware and OS metrics from the nodes in the cluster.

## `loki`: Grafana Loki

[Grafana Loki](https://grafana.com/docs/loki/latest/) is a fully functional
log aggregation system.

Loki is included in `materialize-monitoring` as its default logging
backend.

Refer to [Loki Architecture](https://grafana.com/docs/loki/latest/get-started/architecture/) for more details on the architecture of Loki.

The Loki Write path includes:
* A `loki-write` statefulset that receives logs
  * The `Distributor` subcomponent receives logs and distributes them to the `Ingester` subcomponents.
  * The `Ingester` subcomponent processes incoming logs and writes them to storage.
    It can also serve recent logs for queries.

The Loki Read path includes:
* An optional `loki-query-frontend` deployment that runs the `Query Frontend`.
  * The `Query Frontend` subcomponent receives queries and performs query splitting and fan-out to the `Querier` subcomponents.
    It may consult the `Index Gateway` for query sharding.
* A `loki-read` scalable deployment that receives queries via the Loki API and reads them from storage.
  * The `Querier` subcomponent handles LogQL queries.
    It talks to `Ingesters` for recent logs and to the storage layer for historical logs.
* Additional cache components can be used for query performance (`chunks-cache`, `results-cache`).

The other parts of the Loki Backend include:
* A `loki-gateway` deployment serves metadata queries.
  * The `Index Gateway` subcomponent maintains an index of log metadata
* A `loki-backend` statefulset runs backend components.
  * The `Compactor` subcomponent compacts log data in the storage layer to optimize for cost and performance.
    It also handles retention and deletion of older logs.
  * The `Ruler` subcomponent evaluates alerting and recording rules against incoming logs.

Loki writes its data to object storage (e.g., S3, GCS, Azure Blob Storage, etc.) for long-term storage and scalability.

## `thanos`: Thanos

[Thanos](https://thanos.io/tip/thanos/getting-started.md/) is a highly available Prometheus setup with long-term storage capabilities.

Thanos is included in `materialize-monitoring` as its default metrics storage and querying backend.

Refer to [Thanos Design](https://thanos.io/tip/thanos/design.md/) for more details on the architecture of Thanos.

The Thanos Receiver path includes:
* A `thanos-receive` statefulset that receive metrics in Prometheus Remote Write format.
  * The `Shipper` subcomponent writes metrics to the object storage layer.
  * The `Store API` subcomponent provides an API for querying recent metrics.

The Thanos Query path includes:
* An optional `thanos-query-frontend` deployment is an optional caching and fan-out layer for queries.
* A `thanos-query` scalable deployment that receives queries.
  * The `Query API` subcomponent handles PromQL queries.
  * The `Store API` component is used for gRPC internal communication between components.
* A `thanos-storegateway` deployment that serves metrics from the object storage layer.

Additional components include:
* A `thanos-compactor` singleton deployment that operates against the storage layer to compact, manage retention, and downsample metrics.
* A `thanos-ruler` deployment that runs the `Ruler` component for alerting and recording rules.
  * The `Ruler` subcomponent evaluates alerting and recording rules against incoming metrics.

## `grafana`: Grafana

[Grafana](https://grafana.com/docs/grafana/latest/) is a multi-platform open source analytics and interactive visualization web application.

Grafana is included in `materialize-monitoring` as its main dashboarding and visualization tool.

Grafana is mainly deployed as a Deployment and is recommended to
be backed with a compatible database for durability and scalability.

We use `grafana-operator` to manage resources on a Grafana deployment.

## `grafana-operator`: Grafana Operator

[Grafana Operator](https://grafana.github.io/grafana-operator/docs/) is a Kubernetes Operator that manages Grafana instances and their resources (e.g., Dashboards, Datasources, etc.) as Kubernetes Custom Resources.

The operator itself is just a simple Kubernetes Deployment named `grafana-operator` that watches for Grafana Custom Resources and applies them to the Grafana instance.

It manages these kinds of resources:
- A `Grafana` defines how to set up a Grafana instance or connect to
  an existing Grafana instance.
- A `GrafanaManifest` defines a k8s-style (12+) Grafana Dashboard that can be applied to the Grafana instance.
- A `GrafanaDashboard` defines an old-style (<12) Grafana Dashboard that can be applied to the Grafana instance.
- A `GrafanaDatasource` defines a Grafana Datasource that can be applied to the Grafana instance.
  We typically configure a datasource for Thanos and Loki.

## `alertmanager`: Prometheus Alertmanager

[Prometheus Alertmanager](https://prometheus.io/docs/alerting/latest/alertmanager/) is a tool that handles alerts sent by Prometheus and other monitoring systems.

TODO: determine architecture and integration of Alertmanager in `materialize-monitoring`.
