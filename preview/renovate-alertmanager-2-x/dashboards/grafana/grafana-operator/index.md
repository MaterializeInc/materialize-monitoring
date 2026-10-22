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

The dashboards ship in **separate charts**, one per Grafana folder, each installed as its own release beside `materialize-monitoring`.
Helm stores a release in a Kubernetes Secret and a Secret may not exceed 1 MiB, which the rendered set outgrew.

The following table lists the charts and the dashboards each carries:

| Chart | Folder | Dashboards |
|---|---|---|
| `materialize-monitoring-dashboards` | `materialize` | The Materialize environment, `env-*` |
| `materialize-monitoring-dashboards-infra` | `infra` | The platform underneath it: nodes, networking, autoscaling, cloud services, and the platform's logs and events |
| `materialize-monitoring-dashboards-meta` | `meta-o11y` | The monitoring stack itself: Alloy, Loki, and Thanos |

Install the ones you want:

```bash
helm install mzmon-dashboards oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-dashboards \
  --namespace monitoring
helm install mzmon-dashboards-infra oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-dashboards-infra \
  --namespace monitoring
helm install mzmon-dashboards-meta oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-dashboards-meta \
  --namespace monitoring
```

Their defaults match a `materialize-monitoring` release named `mzmon` installed with *its* defaults.
The charts can't read the other release's values, so where either differs, you spell out the values that move with it.
All of the charts take the same values, so one values file serves them all:

```yaml
grafana:
  enabled: true
  mode: operator
  instanceSelector:
    matchLabels:
      monitoring.materialize.cloud/grafana-instance: mzmon
  folderUids:
    infra: mzmon-infra
    materialize: mzmon-materialize
    meta-o11y: mzmon-meta-o11y
```

Each chart uses the `folderUids` entry for its own folder and ignores the rest.

`selected` narrows what a chart installs.
It defaults to `["*"]`, and each pattern in it is globbed against the pre-rendered dashboards in that chart.
Each match becomes a `GrafanaManifest` resource.

Inspect what was created:

```bash
kubectl get grafanamanifest -n monitoring
```

The dashboard then appears in Grafana within one `resyncPeriod` (5m by default).

### Overriding the target instance

To push dashboards somewhere other than the `Grafana` resource the `materialize-monitoring` chart creates, set an explicit selector in two places.
The folders take it from the `materialize-monitoring` release:

```yaml
dashboards:
  config:
    grafana:
      manifest:
        instanceSelector:
          matchLabels:
            dashboards.materialize.com/instance: platform-grafana
        # Required if that instance lives in another namespace. The chart only
        # infers this for instances it creates itself.
        allowCrossNamespaceImport: true
```

The dashboards take it from each dashboards chart's own values:

```yaml
grafana:
  instanceSelector:
    matchLabels:
      dashboards.materialize.com/instance: platform-grafana
  allowCrossNamespaceImport: true
```

If you don't want the chart's own `Grafana` resource alongside yours, also set `connections.grafana.enabled: false`.

### Folders

The dashboards are filed into folders rather than dropped at the root of the Grafana.
The **`materialize-monitoring` chart** creates the folders as `GrafanaFolder` resources, from `dashboards.config.grafana.folders`.
They stay there rather than moving with the dashboards, because a folder is what a dashboard is filed into rather than part of it.
Each folder's dashboards ship in a dashboards chart of their own, as the following table shows:

| Folder | Holds | Chart |
|---|---|---|
| **Materialize** | Materialize itself: the `env-*` dashboards | `materialize-monitoring-dashboards` |
| **Infrastructure** | The platform underneath Materialize | `materialize-monitoring-dashboards-infra` |
| **Meta Observability** | The monitoring stack watching itself: `infra-alloy`, `infra-loki`, and `infra-thanos`. Nested under Infrastructure | `materialize-monitoring-dashboards-meta` |

```bash
kubectl get grafanafolder -n monitoring
```

Placement travels **with** the dashboard, not with the manifest: each render carries a `grafana.app/folder` annotation
naming the folder it belongs in.
Grafana reads that annotation as a folder **UID**, and a UID depends on the release, so it is not something the render
can know.
What the render writes is the folder's *name*: `materialize`, `infra`, or `meta-o11y`.
The dashboards chart rewrites it to the real UID from its `grafana.folderUids` on the way to the operator, exactly as it rewrites `apiVersion`:

```yaml
# in the pre-rendered dashboard              # in the GrafanaManifest the operator applies
grafana.app/folder: materialize      →       grafana.app/folder: mzmon-materialize
```

So the UID is free to move — under a different release name, or onto a folder you already own via `existingUid` — and
the dashboards follow it without being re-rendered.

What is not free to move is the **name**.
It is the one string the render and the charts must agree on, and renaming a key under `folders` drops the annotation
from every dashboard that names it, leaving them at the root.
The name also decides which dashboards chart carries a dashboard, because each chart carries one folder's dashboards.
Adopt an existing folder with `existingUid` rather than renaming the key:

```yaml
dashboards:
  config:
    grafana:
      folders:
        materialize:
          existingUid: "3e7b4fe1-ca90-4125-a8ab-06567c1971b5"
```

To nest the whole set under a folder you already have, give each top-level entry a parent:

```yaml
dashboards:
  config:
    grafana:
      folders:
        materialize:
          parent:
            folderUID: "3e7b4fe1-ca90-4125-a8ab-06567c1971b5"
```

Folders are an operator-mode feature — the standalone Grafana chart has no resource to create them with.
`create: false` keeps an entry as a resolution target without creating the folder, which is the shape to use when
something outside this chart owns it; removing the entry outright (`materialize: null`) is what drops those dashboards
to the root.

Whatever UIDs come out of that map have to be repeated in the dashboards chart's `grafana.folderUids`, which is the
only way that chart learns them.
The `materialize-monitoring` install notes print them for exactly this reason.

### Dashboard schema version

The dashboards chart's `grafana.apiTarget` selects the dashboard API the manifests declare.
It defaults to `dashboard.grafana.app/v2`, which needs **Grafana 12 or later**.
Against an older Grafana, the operator pushes an object the server does not understand and the dashboard never appears.
No schema v1 render is published yet for Grafana 10 and 11 — see [Available Dashboards](../../all/#formats).

### Drift

Operator-managed dashboards are re-pushed every `resyncPeriod`, so UI edits are reverted on the next resync.
To customize, copy the dashboard to a new UID and edit the copy — or fork the source under `packages/dashboards/` and render your own.

