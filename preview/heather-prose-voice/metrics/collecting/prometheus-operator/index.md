# Prometheus Operator




# Prometheus Operator (ServiceMonitors and PodMonitors)

This is the default path, and the one to reach for first when the thing you want to scrape runs in the cluster.

The gateway runs [`prometheus.operator.servicemonitors`](https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.servicemonitors/) and `prometheus.operator.podmonitors` components, which watch the cluster for `ServiceMonitor` and `PodMonitor` resources and scrape whatever they describe.
Creating one of those resources is the whole integration — nothing about the gateway needs to change, and no config has to be reloaded.

You do **not** need to run `prometheus-operator` itself. Alloy implements the same discovery against the same CRDs. Only the [CRDs](../../../reference/crds/) have to be installed.

## Most applications already ship one

Before writing anything, check whether the application's own chart can create the monitor for you.
It is a near-universal convention in Helm charts, usually behind a value like:

```yaml
serviceMonitor:
  enabled: true
```

or `metrics.serviceMonitor.enabled`, or `prometheus.monitor.enabled`.
The naming varies; the pattern does not.
Turning that flag on is the cleanest option available, because the monitor then stays correct as the chart's ports and labels change.

This stack does exactly that for its own subcharts, and ships monitors for the Materialize workloads — see [Scraping](../../scraping/).

## Writing your own

When an application has no such flag, a `ServiceMonitor` is a short document. It selects a `Service` by label and names the port to scrape:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: my-app
  namespace: my-app
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: my-app
  endpoints:
    - port: metrics       # the *name* of the port on the Service
      path: /metrics
      interval: 30s
```

A `PodMonitor` is the same shape but selects pods directly, which is what you want when the workload has no Service in front of it:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: PodMonitor
metadata:
  name: my-app
  namespace: my-app
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: my-app
  podMetricsEndpoints:
    - port: metrics
      path: /metrics
```

Apply it with `kubectl apply -f`. The gateway picks it up within a discovery cycle; no restart is needed.

Prefer a `ServiceMonitor` where both are possible — it follows the Service's endpoints, so it keeps working through changes a pod-label selector would miss. See [ServiceMonitor vs PodMonitor](../../../reference/crds/#servicemonitor-vs-podmonitor).

## Scoping and relabeling

Monitors are discovered across the cluster by default.
Target-phase relabeling belongs **in the monitor itself** rather than in gateway config — that is what keeps the rule next to the thing it describes:

```yaml
  endpoints:
    - port: metrics
      relabelings:
        - action: labeldrop
          regex: pod_template_hash
      metricRelabelings:
        - action: drop
          sourceLabels: [__name__]
          regex: go_gc_.*
```

Dropping metrics you will never query is the cheapest cardinality lever available, and doing it at the target means the series is never stored anywhere.

## Verifying

A monitor that matches nothing fails silently — there is no error, just no data.

```bash
# Does the gateway see any targets for it?
kubectl port-forward -n monitoring svc/alloy-gateway 12345:12345
# then open http://localhost:12345 and check the component's targets
```

Or query `up{job="my-app"}` in Grafana. An absent result means discovery matched nothing; `up == 0` means it matched and the scrape failed, which is a very different problem.

## See also

- [Overview](../overview/) — the four collection paths.
- [Scraping](../../scraping/) — the monitors this chart ships, and the kubelet container-metrics path.
- [Custom Resource Definitions](../../../reference/crds/) — what has to be installed for any of this to work.

