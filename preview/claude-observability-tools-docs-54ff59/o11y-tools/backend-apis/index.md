# Backend APIs




# Backend APIs

This page is for operators with Kubernetes access to the monitoring namespace.
Thanos Query, Loki, and Alertmanager each serve an HTTP API, and you can reach each one with `kubectl` without going through Grafana.
Use this path when Grafana is down, when the stack itself is what you're debugging, or when a script needs the raw API.


<table class="param-table">
  <tbody>
        <tr>
          <th>Interface</th>
          <td>HTTP API</td>
        </tr>
        <tr>
          <th>Audience</th>
          <td>Operators debugging the stack</td>
        </tr>
        <tr>
          <th>Cluster access</th>
          <td>Port-forward or exec in each backend&rsquo;s namespace</td>
        </tr>
        <tr>
          <th>Grafana access</th>
          <td>None</td>
        </tr>
        <tr>
          <th>Ease of use</th>
          <td>Lowest: <code>kubectl</code>, and PromQL or LogQL</td>
        </tr>
        <tr>
          <th>Stability</th>
          <td>Stable upstream APIs</td>
        </tr>
  </tbody>
</table>


## Before you start

> [!WARNING]
> The backends have no authentication of their own, so anyone who can reach one reads everything it serves.
> Reach them through `kubectl`, and never through an ingress.
> For more information, see [Securing](../../operating/securing/#the-cluster--the-stack).

To port-forward, your Kubernetes identity needs `get` and `list` on `services` and `pods`,
and `create` on `pods/portforward`, in the backend's namespace.
The `amtool` examples need `create` on `pods/exec` instead.

The examples on this page use the `monitoring` namespace.
Under the `split-namespace` profile, each backend runs in its own namespace: `thanos`, `loki`, and `alertmanager`.

## Query metrics from Thanos Query

Thanos Query serves the [Prometheus HTTP API](https://prometheus.io/docs/prometheus/latest/querying/api/),
so anything that speaks PromQL works against it.

To query metrics, follow these steps:

1. Forward a local port to Thanos Query:

   ```bash
   kubectl --namespace monitoring port-forward svc/thanos-query 9090:9090
   ```

1. Run a query with `promtool` or `curl`:

   ```bash
   promtool query instant http://localhost:9090 'up{namespace="materialize-environment"}'
   ```

   ```bash
   curl -s http://localhost:9090/api/v1/query --data-urlencode 'query=up' | jq '.data.result[0]'
   ```

Thanos Query also serves its own query UI at `http://localhost:9090` while the port-forward runs.

For queries worth running, see [Common queries](../../reference/common-queries/).
For what Thanos Query reads from, see [Querying metrics](../../metrics/querying/).

## Query logs from Loki

The Loki gateway serves the [Loki HTTP API](https://grafana.com/docs/loki/latest/reference/loki-http-api/).
The bundled Loki runs with `auth_enabled: true`, so every request needs a tenant in the `X-Scope-OrgID` header.
The pipeline writes to the `loki` tenant unless `pipeline.logging.tenancy.staticTenant` says otherwise.

To query logs, follow these steps:

1. Forward a local port to the Loki gateway:

   ```bash
   kubectl --namespace monitoring port-forward svc/loki-gateway 3100:8080
   ```

1. Run a query with `logcli` or `curl`:

   ```bash
   logcli query --addr=http://localhost:3100 --org-id=loki --since=1h --limit=50 \
     '{namespace="materialize-environment", app="environmentd"}'
   ```

   ```bash
   curl -s -H 'X-Scope-OrgID: loki' http://localhost:3100/loki/api/v1/query_range \
     --data-urlencode 'query={namespace="materialize-environment"}' --data-urlencode 'limit=20'
   ```

A request without the header fails with `no org id`.
For the labels a log stream carries, and the attributes that are structured metadata instead, see [Querying logs](../../logs-and-events/querying/).

## Read alerts and silences from Alertmanager

`amtool` ships in the Alertmanager image, with a configuration that points it at the local Alertmanager.
Run it in the pod rather than through a port-forward:

```bash
kubectl --namespace monitoring exec alertmanager-0 -c alertmanager -- amtool alert query
```

```bash
kubectl --namespace monitoring exec alertmanager-0 -c alertmanager -- amtool silence query
```

To create and expire silences, see [Maintenance windows](../../alerting/maintenance/#through-amtool).

