# Compatibility




# Release Compatibility

This shows the compatibility of materialize-monitoring with various
technologies.

It is also where changes we absorb from outside this repo are recorded — a Materialize metric rename, a Grafana schema bump, an upstream chart moving a value.
For what we guarantee about our own names, and the notice you get before one changes, see [Stability and Deprecations](../stability/).

## [materialize-terraform-self-managed](https://github.com/MaterializeInc/materialize-terraform-self-managed)

| materialize-terraform-self-managed | materialize-monitoring | Clouds | Observability default |
|---|---|---|---|
| v11.0.0+ | 0.17.0 | AWS, GCP, Azure | **On** — `enable_observability` defaults to `true` |
| v10.0.0 – v10.1.1 | 0.12.0 – 0.15.0 | AWS, GCP, Azure | Off — opt-in |
| ≤ v9.0.2 | none (a vendored copy of the `env-top` dashboard only) | — | — |

The Terraform module ships **inside** the `materialize-monitoring` component rather than on a version stream of its own, so one number covers both: a module ref of `materialize-monitoring/vX.Y.Z` installs chart `X.Y.Z`. There is no mapping to maintain between the two.

v11 made the monitoring stack **opt-out rather than opt-in** and completed the per-cloud rollout, so Azure no longer uses the previous Prometheus + Grafana modules.

## [Kubernetes](https://kubernetes.io/)

**We support the non-EOL Kubernetes releases — v1.34 and newer.**

The charts declare `kubeVersion: ">=1.27.0-0"`, but that is a floor rather than a support statement: older releases *may* work and are not supported.

Cloud-provided distributions offering extended support past a release's upstream EOL are supported on a **best-effort** basis, and may run into issues.

## [Materialize product](https://github.com/MaterializeInc/materialize)

Dashboards (v0.8.0+) generally require the `mz_object_info` metric introduced in `v26.29.0`,
however they will gracefully degrade without it.

The **`env-upgrade`** dashboard requires **`v26.41.0`** for its operator signals: the reconciliation metrics
(`orchestratord_reconciliations_total` and friends) and the Kubernetes events the operator publishes for reconciliation
failures and `Materialize` lifecycle transitions.

It degrades rather than breaking on an older release, and not uniformly — the floor is narrower than the dashboard:

| Tab | On a release older than `v26.41.0` |
|---|---|
| Generations | **Fully working.** Every panel reads metrics that predate it, and the blue/green split comes from pod names rather than anything the operator newly exports. |
| Events | The Kubernetes Activity row works — those events come from the kubelet and the scheduler. The Rollout and Operator Health rows are empty. |
| Reconciliation | Empty, apart from Reconciling Replicas and Environments Needing Update, whose gauges predate the rest. |

Each dashboard also carries its floor as a `monitoring.materialize.cloud/min-mz-version` annotation, which the docsite's
dashboard table shows.

Scrapers (v0.1.1+) require `app.kubernetes.io/name` labels for environmentd introduced in `v26.24.0`.

## [Grafana](https://grafana.com/)

Dashboards (v0.8.0+) require Grafana v13+ for the dashboard schema v2.
Grafana v12 is generally known to work, but may run into issues.

## [Google Kubernetes Engine (GKE)](https://cloud.google.com/kubernetes-engine)

GKE works generally with the dashboards. Managed collection ([enabling it](https://docs.cloud.google.com/stackdriver/docs/managed-prometheus/setup-managed#enable-mgdcoll-gke)) is on by default from v1.27 and provides the `PodMonitoring` CRDs the GMP scrapers use.

**Container metrics no longer depend on what GMP exposes.**
The Alloy gateway scrapes `/metrics/cadvisor` on every kubelet itself, which yields a fuller set than GMP's default
collection.
That is what retired the GCP-specific dashboard variants: there is now one render per dashboard, carrying the same
`container_*` and `kube_*` families and percentage-based panels everywhere.
See [Container metrics from the kubelet](../../metrics/scraping/#kubelet).

## [Google Cloud Monitoring Dashboards](https://cloud.google.com/monitoring/dashboards)

Importing Grafana dashboards into Google Cloud Monitoring is not yet fully supported.

The shipped dashboards do have some improvements but there are
still some known issues:
* `$__range` and `$__rate_interval` need to be replaced with constants
* Horizontal bar charts are not rendered
* Tabs and rows are not supported, so there is no layout
* Switch variables are not supported (e.g. "show system clusters")
  * This breaks the downstream cluster selector variable

