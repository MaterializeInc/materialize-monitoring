# Custom Resource Definitions (CRDs)




# Custom Resource Definitions (CRDs)

Kubernetes Custom Resource Definitions (CRDs) extend the Kubernetes API with custom resources that can be managed with standard Kubernetes tooling.
`materialize-monitoring` both **reads** custom resources — to decide what to scrape — and **creates** them — to provision Grafana, its datasources, and its dashboards.
Either way the definitions have to exist before the main chart installs, which is why they ship in a chart of their own.

This page is the inventory: what gets installed, what the stack actually does with each kind, and what it deliberately leaves alone.
For the install procedure in context, see [Dependencies](/materialize-monitoring/preview/renovate-grafana-monorepo/getting-started/dependencies/) and [Installing via Helm](/materialize-monitoring/preview/renovate-grafana-monorepo/getting-started/helm/).

## How they are installed

CRDs ship in a **separate chart**, `materialize-monitoring-crds`, so their lifecycle is managed independently of the stack that consumes them.
Install it first, then install the main chart with `--skip-crds`:

```bash
helm install mzmon-crds oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-crds --namespace monitoring
helm install mzmon oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring --namespace monitoring --skip-crds
```

`--skip-crds` is not optional tidiness.
The bundled Grafana Operator subchart ships its own copy of the Grafana CRDs and offers no way to opt out of them, so without it a fresh install of the main chart creates those definitions behind the CRDs chart's back, and the two releases then contend for the same cluster-scoped objects.

> [!WARNING]
>  Every CRD carries `helm.sh/resource-policy: keep`, so uninstalling the CRDs chart leaves the definitions in place.
>  This is deliberate: deleting a CRD cascades to **every object of that kind in the cluster**, including ones other charts and your own manifests created.
>  Removing them is a separate, explicit step — see [Uninstalling](/materialize-monitoring/preview/renovate-grafana-monorepo/operating/uninstalling/).

### Turning individual CRDs off

Both sets take the same shape, so they are configured identically from the parent chart: `crds.annotations` applies to every CRD in the set, and `crds.<plural>.enabled` toggles one.

```yaml
prometheus-operator-crds:
  crds:
    thanosrulers:
      enabled: false
grafana-operator-crds:
  crds:
    grafanalibrarypanels:
      enabled: false
```

This is worth doing when another chart in your cluster already owns a definition, or when you want only the kinds this stack actually uses.
The tables below mark which those are.

## Prometheus Operator CRDs

API group `monitoring.coreos.com`, vendored from the upstream [prometheus-community chart](https://github.com/prometheus-community/helm-charts/tree/main/charts/prometheus-operator-crds).
Values key: `prometheus-operator-crds`.

| Kind | Version | Used by this stack |
|---|---|---|
| `ServiceMonitor` | `v1` | **Read.** The Alloy gateway discovers scrape targets from them |
| `PodMonitor` | `v1` | **Read and created.** The chart creates them for the Materialize workloads; the gateway reads them |
| `ScrapeConfig` | `v1alpha1` | Shipped as an artifact only — [see below](#scrapeconfig) |
| `PrometheusRule` | `v1` | Reserved — [see below](#prometheusrule) |
| `Probe` | `v1` | No |
| `Alertmanager` | `v1` | No |
| `AlertmanagerConfig` | `v1alpha1` | No |
| `Prometheus` | `v1` | No |
| `PrometheusAgent` | `v1alpha1` | No |
| `ThanosRuler` | `v1` | No |

The "No" rows are present because the upstream chart is the canonical source for the whole set, not because anything here needs them.
They describe workloads that `prometheus-operator` manages, and **this stack does not run `prometheus-operator`**: Alertmanager and Thanos come in as ordinary subchart workloads, and Alloy takes the place of a Prometheus server.
If nothing else in your cluster wants them, switch them off with the toggles above.

### ServiceMonitor vs PodMonitor

ServiceMonitors can be written by any application to indicate that it should have its metrics scraped.
By default `materialize-monitoring` runs Grafana Alloy with `prometheus.operator.servicemonitors` and `prometheus.operator.podmonitors` components on the gateway, which read both kinds to determine what to scrape.

ServiceMonitors are preferred, but the two behave much the same.
A ServiceMonitor selects `Service` resources and scrapes the endpoints behind them; a PodMonitor selects pods directly.
The scrapers this repo ships for Materialize are PodMonitors, because the Materialize workloads are not all fronted by a Service.

### ScrapeConfig

Alloy has no `prometheus.operator.scrapeconfigs` equivalent — it reads ServiceMonitors and PodMonitors only.
The `ScrapeConfig` this repo ships (`scrapeconfig-cadvisor.yaml`) is therefore an artifact for a **Prometheus** consumer rather than something the chart installs on the default Alloy path.

On the default path the gateway scrapes each kubelet's `/metrics/cadvisor` endpoint directly instead, which is on by default and needs no custom resource.
See [Scraping](/materialize-monitoring/preview/renovate-grafana-monorepo/metrics/scraping/#classic) for the downloads.

### PrometheusRule

`config.rules.prometheus.enabled` defaults to `true` and is the switch for installing the bundled recording and alerting rules as `PrometheusRule` resources.

The rule content is **not in the chart yet**, so nothing renders one today — the CRD and the value are in place ahead of it.
Progress is tracked in the [Rules & alerts](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/roadmap/#rules--alerts) workstream.

## Grafana Operator CRDs

API group `grafana.integreatly.org`, all at version `v1beta1`.
These are deflated from the upstream grafana-operator chart into a local subchart, because upstream publishes no standalone CRDs chart for them.
Values key: `grafana-operator-crds`.

| Kind | Used by this stack |
|---|---|
| `Grafana` | **Created.** The Grafana instance the operator reconciles |
| `GrafanaDatasource` | **Created.** The Thanos and Loki datasources |
| `GrafanaManifest` | **Created.** How the bundled dashboards are delivered — [see below](#dashboards-arrive-as-grafanamanifest-not-grafanadashboard) |
| `GrafanaDashboard` | Available for dashboards you manage yourself |
| `GrafanaFolder` | Available — folders that dashboards and rules are filed under |
| `GrafanaAlertRuleGroup` | Available — alert rules grouped for evaluation |
| `GrafanaContactPoint` | Available — alerting notification targets |
| `GrafanaNotificationPolicy` | Available — the alerting notification tree |
| `GrafanaNotificationPolicyRoute` | Available — individual routes within that tree |
| `GrafanaNotificationTemplate` | Available — message templates for notifications |
| `GrafanaMuteTiming` | Available — recurring windows that suppress notifications |
| `GrafanaLibraryPanel` | Available — reusable panels shared across dashboards |
| `GrafanaServiceAccount` | Available — service accounts and tokens inside Grafana |

Unlike the Prometheus set, the "Available" rows here are all serviceable: the operator this chart runs reconciles every one of them, so they are the supported way to extend Grafana alongside the bundled content.

The operator only acts on resources whose `instanceSelector` matches, so the chart derives that selector from the labels on the `Grafana` resource it creates — the selector and the instance render from one source and cannot drift.
See [Grafana Operator](/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/grafana-operator/) and [Grafana Architecture](/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/architecture/).

### Dashboards arrive as `GrafanaManifest`, not `GrafanaDashboard`

This surprises people, so it is worth stating plainly.
The chart wraps each pre-rendered dashboard in a `GrafanaManifest`, whose `spec.template` carries the dashboard object verbatim with its `apiVersion` set from `dashboards.config.grafana.manifest.apiTarget` (`dashboard.grafana.app/v2` by default).

`GrafanaManifest` is the generic carrier for a Grafana API object, which is what lets the same chart ship dashboard schema v2 without the operator needing a v2-aware spec of its own.
`GrafanaDashboard` still works and is the natural choice for dashboards you author yourself — it simply is not how the bundled set travels.

## CRDs this chart does not install

Two groups the stack can use are **not** part of the CRDs chart, because something else owns them.

| Group | Kinds | Who installs them |
|---|---|---|
| `monitoring.googleapis.com/v1` | `PodMonitoring`, `ClusterPodMonitoring` | GKE, when [managed collection](https://docs.cloud.google.com/stackdriver/docs/managed-prometheus/setup-managed) is enabled — on by default since GKE v1.27 |
| `cert-manager.io/v1` | `Certificate`, `Issuer`, `ClusterIssuer` | [cert-manager](https://cert-manager.io/), which you install yourself |

**Google Managed Prometheus.** The repo ships `PodMonitoring` and `ClusterPodMonitoring` artifacts for the GMP path, but no template installs them — they are downloads for a GMP consumer, the same way the classic `ScrapeConfig` is. See [Scraping](/materialize-monitoring/preview/renovate-grafana-monorepo/metrics/scraping/#gmp).

**cert-manager.** The chart renders `Certificate` resources, and optionally an `Issuer` or `ClusterIssuer`, only when `certificates.enabled` is set — it defaults to `false`, and with it off cert-manager is not a dependency of this chart in any sense.
The rendering is deliberately *not* gated on an API-capability probe, so `helm template` against no cluster produces exactly what a live install does.
That means enabling it without cert-manager present fails at apply time rather than silently rendering nothing, which is the intended behaviour.
See [Securing](/materialize-monitoring/preview/renovate-grafana-monorepo/operating/securing/#certificates).

## Checking what is installed

```bash
kubectl get crds -l app.kubernetes.io/part-of=materialize-monitoring
kubectl get crds | grep -E 'monitoring\.coreos\.com|grafana\.integreatly\.org'
```

To see whether the operator has accepted the resources the chart created:

```bash
kubectl get grafanas,grafanadatasources,grafanamanifests -n monitoring
kubectl get podmonitors -n monitoring
```

An empty result where you expected dashboards usually means the `instanceSelector` did not match, not that the resource failed to apply.
[o11y Troubleshooting](/materialize-monitoring/preview/renovate-grafana-monorepo/operating/o11y-troubleshooting/) covers that path.

