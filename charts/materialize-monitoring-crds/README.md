# materialize-monitoring-crds

This provides the CRDs used by the `materialize-monitoring` Helm chart.

These are almost entirely for upstream projects like
Prometheus Operator and Grafana Operator.

It can be challenging to manage CRDs within a Helm Chart,
so we also provide a kubectl/Kustomization installation method as well (TODO).
It is recommended to install this as a separate step before the
main `materialize-monitoring` chart.

These CRDs are limited to the following:
- ServiceMonitor (monitoring.coreos.com/v1)
- PodMonitor (monitoring.coreos.com/v1)
- ScrapeConfig (monitoring.coreos.com/v1)
- PrometheusRule (monitoring.coreos.com/v1)

## Compatibility

### materialize-monitoring

This chart is intended to be compatible with materialize-monitoring
by default.

`materialize-monitoring` is also expected to be compatible with another Prometheus Operator CRD or Grafana Operator CRD provider.

### Grafana Alloy with prometheus.operator component

This provides the set of CRDs required for Grafana Alloy to provide
prometheus.operator support: ServiceMonitor, PodMonitor, and ScrapeConfig.

### Prometheus Operator

The prometheus-operator project requires more CRDs than this project
provides.
You should instead not use the ServiceMonitor, PodMonitor, and ScrapeConfig CRDs from this chart and instead use the ones provided by your Prometheus Operator (e.g., kube-prometheus-stack helm chart or prometheus-operator helm/kustomize).

### Grafana Operator

All CRDs for Grafana Operator are provided in this chart with
the expectation that Grafana Operator is provided by the `materialize-monitoring` chart.

If you are managing `grafana-operator` yourself, you may wish
to use the CRDs from that project instead.

## Switching to another CRD provider

If you are switching providers for your Prometheus Operator CRDs,
you can adopt those resources by including `--take-ownership`
in your `helm install`/`helm upgrade` command for the new provider.

### Example: installing `kube-prometheus-stack` and adopting CRDs

If you are installing `kube-prometheus-stack` and want to adopt the CRDs from this chart, you can run the following command:

```bash
helm upgrade --install -n prometheus kube-prometheus prometheus-community/kube-prometheus-stack --take-ownership ...
```

## Uninstallation

Standard helm uninstallation will delete this chart, but CRDs are
kept around using the `helm.sh/resource-policy: keep` annotation.
This prevents their associated Custom Resources from being forcibly
deleted as well.
In some cases, this would orphan the resources, but monitoring
is a particular case where the CRs do not have extra compute resources
associated with them, so this is a perfectly safe operation.
