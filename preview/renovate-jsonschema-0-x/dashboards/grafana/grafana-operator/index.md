# Grafana Operator




# Managing Dashboards with Grafana Operator

The [Grafana Operator](https://grafana.github.io/grafana-operator/) reconciles Kubernetes resources into a Grafana instance over its HTTP API.
`materialize-monitoring` uses it to keep the Materialize dashboards in sync without anyone importing JSON by hand.

This page is the how-to.
For how the operator relates to the bundled Grafana server, and which of the two owns what, read [Grafana Architecture](../architecture/) first.

## Configuring Grafana Operator via `materialize-monitoring` Helm Chart

### Enabling the operator

The operator is part of the `managed-grafana` tag group, which the `default` tag includes.
A default install already has it.

To install the operator without the bundled Grafana server — the usual shape when you have your own Grafana:

```yaml
tags:
  default: true

grafana:
  # Circuit breaker: takes precedence over any tag.
  enabled: false
```

To install *only* the Grafana pieces, with no backends or pipeline:

```yaml
tags:
  default: false
  managed-grafana: true
```

### CRDs

The Grafana Operator CRDs are **not** installed by this chart.
They live in the companion `materialize-monitoring-crds` chart, which vendors a deflated copy of them, and must be installed first:

```bash
helm upgrade --install mzmon-crds charts/materialize-monitoring-crds -n monitoring --create-namespace
```

`grafana-operator.crds.immutable` is pinned to `true` in `values.yaml` to keep the operator's own copies out of this release.
The upstream operator chart has no way to skip its CRDs outright — `immutable` only chooses where they come from — so
leaving it `true` keeps them install-only and lets `--skip-crds` drop them entirely.
Setting it `false` makes this chart template and upgrade the CRDs itself, which fights the CRDs chart for ownership of the same objects.

See [CRDs](../../../reference/crds/) for the full list.

### Watch scope

The operator ships with `WATCH_NAMESPACE=""`, meaning it watches **every namespace in the cluster**.
Restrict it if the cluster runs other Grafana instances you do not want it touching:

```yaml
grafana-operator:
  env:
    - name: WATCH_NAMESPACE
      value: monitoring
```

Scoping the watch also narrows the operator's effective blast radius, but it does not narrow its RBAC — the chart still
grants a ClusterRole over `grafana.integreatly.org/*`.

## Connecting your Grafana Instance to Grafana Operator

The connection is expressed as a single `Grafana` custom resource, which the chart renders from `connections.grafana`.
Pick the mode that matches where your Grafana lives:

```yaml
connections:
  grafana:
    # bundled | external | operator
    mode: external
    external:
      url: https://grafana.example.com
      apiKey:
        name: external-grafana-api-key
        key: apiKey
```

The resource always carries a static `monitoring.materialize.cloud/grafana-instance: mzmon` label, which is also the
default `instanceSelector` for every dashboard the chart ships.
`connections.grafana.labels` merges over it and applies to both sides at once, so use it to narrow the selector rather than to re-state it.

Credentials are secret references only.
Create the Secret before installing:

```bash
kubectl create secret generic external-grafana-api-key -n monitoring --from-literal=apiKey="$GRAFANA_SERVICE_ACCOUNT_TOKEN"
```

Use a Grafana service account token with permission to write dashboards and folders.
`adminUser` + `adminPassword` are supported as an alternative, but a scoped token is preferable.

### Verifying the connection

The operator records connection state on the resource:

```bash
kubectl get grafana -n monitoring -o wide
kubectl describe grafana mzmon-grafana -n monitoring
```

A healthy instance shows the resolved URL and no error conditions.
`the instanceSelector can't find matching grafana` on a dashboard means the selector and the `Grafana` labels disagree.

> [!INFO]
> `helm install --wait` does **not** wait for any of this.
> Helm waits for the operator Deployment; the reconcile into Grafana happens afterwards and fails independently.
> Check the resources above after the release reports success.

## Importing Dashboards via Grafana Operator

With the operator connected, dashboard installation is just a values setting:

```yaml
dashboards:
  selected:
    - env-*
  config:
    grafana:
      enabled: true
      mode: operator
```

Each pattern in `dashboards.selected` is globbed against the pre-rendered dashboards in the chart, and each match
becomes a `GrafanaManifest` resource.

Inspect what was created:

```bash
kubectl get grafanamanifest -n monitoring
```

The dashboard then appears in Grafana within one `resyncPeriod` (5m by default).

### Overriding the target instance

To push dashboards somewhere other than the `Grafana` resource this chart creates, set an explicit selector:

```yaml
dashboards:
  config:
    grafana:
      manifest:
        instanceSelector:
          matchLabels:
            dashboards.materialize.com/instance: platform-grafana
        # Required if that instance lives in another namespace — the chart only
        # infers this for instances it creates itself.
        allowCrossNamespaceImport: true
```

### Dashboard schema version

`dashboards.config.grafana.manifest.apiTarget` selects the dashboard API the manifests declare.
It defaults to `dashboard.grafana.app/v2`, which needs **Grafana 12 or later**.
Against an older Grafana, the operator pushes an object the server does not understand and the dashboard never appears.
Use the [downloadable v1 JSON](../importing/#grafana-10-and-11-dashboard-schema-v1) for Grafana 10 and 11 instead.

### Drift

Operator-managed dashboards are re-pushed every `resyncPeriod`, so UI edits are reverted on the next resync.
To customize, copy the dashboard to a new UID and edit the copy — or fork the source under `packages/dashboards/` and render your own.

