# materialize-monitoring Changelog

<!-- This repo uses different versioning streams for its artifacts.
Artifacts are mapped out in packages/components.yaml.
Unreleased sections are placeholders ("_Changes Pending_") until a
version-update/<component> PR populates and releases them; that PR also bumps
the component's version_paths. See reference/internal/versioning.md and
reference/internal/releasing.md.
-->

## mzmon-lib (shared library) v0.13.0 (Unreleased)

_Changes Pending_

## materialize-monitoring (Helm chart + Terraform module) v0.32.0 (Unreleased)

_Changes Pending_

## materialize-monitoring (Helm chart + Terraform module) v0.31.0

* Expose Materialize pod labels on kube_pod_labels
    * [materialize-monitoring#479](https://github.com/MaterializeInc/materialize-monitoring/pull/479)
    * `kube_pod_labels` now names the Materialize cluster and replica a pod runs, as `cluster_id`, `replica_id` and `replica_size`, and `environment_id` once the Materialize operator sets the `materialize.cloud/environment-id` pod label.
        * Published by a new `pods` entry in `kube-state-metrics.metricLabelsAllowlist` and renamed from the `label_*` form by `kube-state-metrics.prometheus.monitor.http.metricRelabelings`. Helm replaces lists, so an override of either restates the chart's entries.
        * Join a `kube_pod_*` family onto it on `namespace` and `pod`, and wrap `kube_pod_labels` in `group by` first, or the join fails under more than one kube-state-metrics replica.
    * New Terraform input `kube_state_metrics_pod_labels` adds pod labels, such as an ownership label for cost-center allocation, to that allowlist entry without restating it. Each arrives as `label_<key>`.
* Google Cloud metrics: export over OTLP to the Telemetry API, typed
    * [materialize-monitoring#474](https://github.com/MaterializeInc/materialize-monitoring/pull/474)
    * **Changed:** the Google Cloud metrics destination writes somewhere else. `googleCloudExporter` (Terraform `google_cloud_metrics`) now sends OTLP to Google's Telemetry API instead of using `otelcol.exporter.googlecloud`, and metrics land as `prometheus.googleapis.com/<name>/<kind>` instead of `workload.googleapis.com/mzmon/<name>`. **Cloud Monitoring dashboards, alerting policies and anything else reading the old metric types stop receiving data and have to be repointed.** The old types are not deleted.
        * **Enable the `telemetry.googleapis.com` API on the project before upgrading with this destination on.** Without it every export is refused, and nothing else fails. `roles/monitoring.metricWriter` is still the only role needed.
        * Metrics are billed per sample ingested instead of per byte, about $75 a month at `recommended` on a test install where the old export cost about $2,400.
        * Series carry the `prometheus_target` labels (`project_id`, `location`, `cluster`, `namespace`, `job`, `instance`) and `collected_by="materialize-monitoring"`.
        * Counter values start from zero at the gateway's first scrape, so they differ from Thanos; `rate()` and `increase()` agree.
        * New values `googleCloudExporter.project` and `googleCloudExporter.location` are needed only off GKE.
    * **Changed:** there is no metric prefix any more. `googleCloudExporter.prefix` is removed from the chart values, along with `instrumentation_library_labels`, `skip_create_descriptor` and `service_resource_labels`; setting any of them renders a warning and does nothing.
    * **Deprecated:** Terraform `google_cloud_metrics.prefix` is ignored and warns at plan time; remove it. There is no replacement, because the Telemetry API has no prefix to choose. Everything else in `google_cloud_metrics` is unchanged.
    * **Changed:** the gateway's scrapes honor metric metadata, so every OTLP destination (Google Cloud, Datadog, generic OTLP) receives typed metrics: counters as cumulative sums, histograms as histograms. Metric names in Datadog change accordingly. Thanos and other remote-write destinations are unchanged.
    * `alloy-gateway.alloy.stabilityLevel` now defaults to `experimental`, which the scrapes require; the chart refuses any other level.
    * The agent and gateway now run the Alloy v1.20.0 image their `image.tag` names. Since the bump to `v1.20.0-mz3`, a stale `image.digest` had kept both on Alloy v1.19.2.
    * `denyMetrics` entries match per Prometheus series against typed histograms: denying `foo_bucket` still keeps `foo_count` and `foo_sum`.
* Update grafana-operator to v5.25.0
    * [materialize-monitoring#460](https://github.com/MaterializeInc/materialize-monitoring/pull/460)
* Update docker.io/kiwigrid/k8s-sidecar Docker tag to v2.11.2
    * [materialize-monitoring#458](https://github.com/MaterializeInc/materialize-monitoring/pull/458)
    * [`v2.11.2`](https://redirect.github.com/kiwigrid/k8s-sidecar/releases/tag/2.11.2)
    * [`v2.11.1`](https://redirect.github.com/kiwigrid/k8s-sidecar/releases/tag/2.11.1)
    * [`v2.11.0`](https://redirect.github.com/kiwigrid/k8s-sidecar/releases/tag/2.11.0)
    * [`v2.10.3`](https://redirect.github.com/kiwigrid/k8s-sidecar/releases/tag/2.10.3)
* Update kube-state-metrics Helm Chart to v8.6.0
    * [materialize-monitoring#461](https://github.com/MaterializeInc/materialize-monitoring/pull/461)
* Update ghcr.io/materializeinc/mzmon-alloy Docker tag to v1.20.0
    * [materialize-monitoring#459](https://github.com/MaterializeInc/materialize-monitoring/pull/459)
* Update quay.io/prometheus-operator/prometheus-config-reloader Docker tag to v0.94.1
    * [materialize-monitoring#464](https://github.com/MaterializeInc/materialize-monitoring/pull/464)
* Update quay.io/prometheus/alertmanager Docker tag to v0.34.1
    * [materialize-monitoring#453](https://github.com/MaterializeInc/materialize-monitoring/pull/453)
    * [`v0.34.1`](https://redirect.github.com/prometheus/alertmanager/releases/tag/v0.34.1): 0.34.1 / 2026-09-17

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate hyper-util to v0.1.21
        * [materialize-monitoring#404](https://github.com/MaterializeInc/materialize-monitoring/pull/404)
        * [`v0.1.21`](https://redirect.github.com/hyperium/hyper-util/blob/HEAD/CHANGELOG.md#0121-2026-09-24)

## materialize-monitoring (Helm chart + Terraform module) v0.30.0

* Shard the Loki ruler's rule groups across its two replicas
    * [materialize-monitoring#444](https://github.com/MaterializeInc/materialize-monitoring/pull/444)
    * The Loki ruler's two replicas now divide the rule groups between them (`loki.loki.rulerConfig.enable_sharding: true`), so each rule is evaluated once rather than once per replica. Before this, every Loki alert reached Alertmanager twice.
        * A ruler that stops cleanly hands its groups over within one evaluation. One lost with its node hands them over after two minutes, and they are not evaluated until then.
        * Recording-rule samples from the Loki ruler carry `instance="loki-ruler"` rather than the pod name, and `POD_NAME` is removed from `loki.ruler.extraEnv`. An override of `write_relabel_configs` that still names `${POD_NAME}` should restate the env var or switch to the constant.
        * The upgrade rolls every Loki component, since they share one configuration.
* Update grafana Docker tag to v12.11.2
    * [materialize-monitoring#450](https://github.com/MaterializeInc/materialize-monitoring/pull/450)
* Thanos: datasource scrape interval, distinct SQL endpoint series, ruler on subchart remoteWrite
    * [materialize-monitoring#447](https://github.com/MaterializeInc/materialize-monitoring/pull/447)
    * The Thanos Grafana datasource now sets `jsonData.timeInterval` to the slower of `pipeline.metrics.kubelet.scrapeInterval` and `pipeline.metrics.kubeProxy.scrapeInterval` (60s by default), so `rate()` panels over cAdvisor and kube-proxy metrics draw continuously instead of as scattered points. Panels over faster-scraped metrics get a 60s minimum step. `connections.datasources.thanos.jsonData.timeInterval` overrides it, and the render warns when an override is shorter than the chart's value.
    * Series from the environmentd SQL `PodMonitor` (`/metrics/mz_compute`, `mz_frontier`, `mz_storage`, `mz_usage`) carry a new `metrics_path` label, so that job's `up` and `scrape_*` have one series per path rather than one shared series.
    * The Thanos ruler's stateless mode is now the subchart's `thanos.ruler.remoteWrite`, and the ruler no longer passes `--objstore.config-file` or runs a block shipper.
        * The remote-write configuration is a Secret, `thanos-ruler-remote-write-v2` or `-v2-tls`, named by `thanos.ruler.remoteWrite.secretName`; it was a ConfigMap mounted through `thanos.ruler.extraVolumes`. `profiles/mtls.values.yaml` now sets `secretName`.
        * `thanos.ruler.extraArgs` no longer carries `--remote-write.config-file`, and `extraVolumes` / `extraVolumeMounts` no longer carry a `remote-write` entry. A values override that still restates either now fails the render with what to remove. Terraform installs are unaffected; Helm installs upgrading with `--reuse-values` or a copied mTLS override need the entries dropped.
        * To write rule results elsewhere, name another Secret in `secretName`, or set `secretName: ""` with `createSecret: true` and the configuration in `config`.
* Renovate: unblock pending updates, automerge crates, split lock files; DEP-324 pin subchart images
    * [materialize-monitoring#446](https://github.com/MaterializeInc/materialize-monitoring/pull/446)
    * Every subchart image the chart renders under its shipped profiles is now pinned in `values.yaml` (`loki.loki.image`, `loki.lokiCanary.image`, `loki.memcached.image`, `loki.memcachedExporter.image`, `loki.sidecar.image`, `thanos.global.image`, `grafana-operator.image`, `kube-state-metrics.image`, `metrics-server.image`, `grafana.initChownData.image`). Rendered images are unchanged. These are the subcharts' own value paths, so existing overrides keep applying.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
    * docs: flatten reference/stable-metrics into reference, and reorder by use
        * [materialize-monitoring#442](https://github.com/MaterializeInc/materialize-monitoring/pull/442)
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0

## Dashboards (Helm chart) v0.19.0 (Unreleased)

_Changes Pending_

## materialize-monitoring (Helm chart + Terraform module) v0.29.0

* Update docker.io/grafana/grafana Docker tag to v13.2.3
    * [materialize-monitoring#440](https://github.com/MaterializeInc/materialize-monitoring/pull/440)
    * [`v13.2.3`](https://redirect.github.com/grafana/grafana/blob/HEAD/CHANGELOG.md#1323-2026-09-29)
* dashboards: add Infrastructure Autoscaling and Karpenter
    * [materialize-monitoring#425](https://github.com/MaterializeInc/materialize-monitoring/pull/425)
    * Two new dashboards, installed by the default `infra-*` pattern: **Infrastructure Autoscaling** (`infra-autoscaling`), on every cloud, and **Karpenter** (`infra-karpenter`), which has data on EKS where Karpenter's ServiceMonitor is applied.
    * kube-state-metrics now publishes `kube_node_labels` for each provisioner's pool label, the instance type and the zone, set by the new default `kube-state-metrics.metricLabelsAllowlist`. An install that brings its own kube-state-metrics needs the same allowlist for the Node Pools tab.
* Re-enable the nginx Loki gateway so Grafana lists Loki rules
    * [materialize-monitoring#432](https://github.com/MaterializeInc/materialize-monitoring/pull/432)
    * The Loki gateway (nginx) is enabled by default, and Grafana's Loki datasource points at it: `connections.datasources.loki.url` defaults to `http://loki-gateway.<namespace>.svc:8080`. Grafana's alerting UI now lists the Loki ruler's rules and their state, read-only.
        * The gateway routes only Grafana's reads and the ruler's rule and alert state. Pushes, rule definitions, ring pages, flushes and deletes are refused.
        * A values file that sets `loki.gateway.enabled: false` now fails the render until `connections.datasources.loki.url` points at the query frontend (`http://loki-query-frontend.<namespace>.svc:3100`).
        * New `loki.gateway.nginxConfig.tls`: certificate paths for the gateway's listener and its connections to Loki. `profiles/mtls.values.yaml` and `mtls-phase2.values.yaml` set them; phase 2 also has Grafana present its certificate on the Loki datasource.
        * `loki-gateway` is added to `certificates.components.loki.services`.
        * Under `profiles/split-namespace.values.yaml`, `loki.networkPolicy.ingress.namespaceSelector` must admit the `grafana` namespace: it now governs Grafana's access to the gateway.
        * `profiles/registry/chainguard.values.yaml` pins the gateway's nginx tag to `1.31.6`. Under `profiles/registry/docker-hardened-images.values.yaml`, restate `loki.gateway.image.tag` to a variant your mirror holds.
    * The Terraform module's `logs_url` output is now the Loki gateway (`http://loki-gateway.<namespace>.svc.cluster.local:8080`), and `https` whenever `internal_tls` is not `off`. It was `http` under every `internal_tls` before, which was wrong for the TLS stages.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * dashboards: say object, freshness, and orphaned instead of collection, lag, and leaked
        * [materialize-monitoring#435](https://github.com/MaterializeInc/materialize-monitoring/pull/435)
        * Dashboard panels now use Materialize's product terms: _object_ for collection, _freshness_ for lag, and _orphaned_ for leaked persist data. Several panel titles on `env-top`, `env-upgrade`, `env-persist`, and `env-consensus` changed accordingly; dashboard UIDs and queries did not.

## materialize-monitoring (Helm chart + Terraform module) v0.28.0

* Render LogQL alerts from the query registry and deliver them to the Loki ruler
    * [materialize-monitoring#426](https://github.com/MaterializeInc/materialize-monitoring/pull/426)
    * Bundled log-derived alerts, evaluated by the Loki ruler. `materialize-panic`, `data-correctness-error` and `persist-filter-pushdown-violation` are in the default set. `trace-logging-enabled` installs when selected.
    * Every bundled `PrometheusRule` now carries `mzmon.materialize.cloud/flavor: promql` or `logql`. The alloy-gateway writes `logql` ones into the Loki ruler through its API, including a deployment's own. They install only where the ruler's rule store accepts writes; a `local` store, which a filesystem-only Loki gets, leaves them out with a render warning.
    * `thanos.ruler.autoImportPrometheusRules.labelSelector` defaults to `mzmon.materialize.cloud/flavor!: logql`. A replacement selector has to keep that key, or select `flavor: promql`. A cluster running a Prometheus Operator admission webhook has to exclude `flavor: logql` from it.
    * New `rules.logTenants`: the Loki tenants the log-derived rules are written into. Empty means `pipeline.logging.tenancy.staticTenant`. List them under `byEnvironment` tenancy.
    * The generated rule index moved from `pre-rendered/rules/prometheus/_index.yaml` to `pre-rendered/rules/_index.yaml`, and records each rule's `engine`.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * chore(deps): update rust crate tokio-rustls to v0.26.6
        * [materialize-monitoring#427](https://github.com/MaterializeInc/materialize-monitoring/pull/427)

## Container Images v0.7.0 (Unreleased)

_Changes Pending_

## materialize-monitoring (Helm chart + Terraform module) v0.27.0

* Expose alerting rules, routing and receiver credentials through Terraform
    * [materialize-monitoring#424](https://github.com/MaterializeInc/materialize-monitoring/pull/424)
    * The Terraform module takes the chart's alerting configuration:
        * `alert_rules` sets which bundled rules install and their overrides, namespaces and infrastructure workload tiers.
        * `alerting` sets the routing preset, receivers, extra routes, inhibit rules, time intervals, templates and Alertmanager's `global` block.
    * `alerting_receiver_secrets` is new: a `sensitive` map the module turns into the `alertmanager-receivers` Secret, so receiver credentials stay out of the Helm values.
        * Receivers read each key as `/etc/alertmanager/secrets/alertmanager-receivers/<key>`, and the plan fails on a key the map does not set.
        * Leave it empty to manage that Secret some other way.
    * `alertmanager_namespace` is new: where that Secret is created. Set it to `alertmanager` under the chart's `split-namespace` profile.
* DEP-233 Pull instance availability from each cloud's monitoring API
    * [materialize-monitoring#422](https://github.com/MaterializeInc/materialize-monitoring/pull/422)
    * Pull what each cloud publishes about instance availability, off by default:
        * `pipeline.metrics.provider.cloudwatch.eks.clusters` pulls EC2 status checks for every node of the listed EKS clusters, their managed node groups' sizes, and the region's On-Demand vCPU usage. It needs `cloudwatch:GetMetricData`, `cloudwatch:ListMetrics`, `tag:GetResources` and `autoscaling:DescribeAutoScalingGroups`.
        * `pipeline.metrics.provider.gcp.compute.regions` pulls per-family CPU and local-SSD quota, usage against limit, and refusals. `roles/monitoring.viewer` already covers it.
        * `pipeline.metrics.provider.azure.aks.clusters` pulls the AKS cluster autoscaler's gauges and each node VM's availability. It needs Monitoring Reader on each cluster and its node resource group.
        * The new families (`aws_ec2_*`, `aws_autoscaling_*`, `aws_usage_*`, `stackdriver_compute_googleapis_com_location_*`, `azure_microsoft_containerservice_managedclusters_*` and `azure_microsoft_compute_virtualmachinescalesets_*`) take each provider's `metricImportance`.
* Run Grafana on a read-only root, and give every pod a securityContext
    * [materialize-monitoring#423](https://github.com/MaterializeInc/materialize-monitoring/pull/423)
    * **Grafana runs on a read-only root filesystem.** The chart mounts an `emptyDir` at `/tmp` through `grafana.extraEmptyDirMounts`. A values file that sets that list replaces the chart's, so it has to keep the `/tmp` entry; the render fails if it does not.
    * **Grafana's bundled datasource plugins stay at the image's versions.** `grafana.ini.plugins.preinstall_auto_update` defaults to `false`. Previously Grafana replaced Prometheus, Loki and the other bundled datasources with the newest release on grafana.com at every start.
    * **New render errors** when Grafana has a read-only root and nothing is mounted at `/tmp`, when preinstall auto-update is back on, or when `grafana.env.GF_AWS_PROFILES` is set without a mount at `/usr/share/grafana/.aws`. A warning fires when `grafana.containerSecurityContext.readOnlyRootFilesystem` is turned off.
    * **`RuntimeDefault` seccomp and a pod-level `securityContext`** on grafana-operator, every Thanos component, metrics-server, node-exporter and the pre-validate job, and seccomp on the Alloy containers. node-exporter drops every capability and forbids privilege escalation.
    * **grafana-operator requests resources** (10m CPU, 64Mi memory, 512Mi memory limit). It was BestEffort.
* dashboards: add Persist, Consensus and Cloud Provider dashboards
    * [materialize-monitoring#419](https://github.com/MaterializeInc/materialize-monitoring/pull/419)
    * Three new dashboards, installed by default: **Materialize Persist (Storage)** (`env-persist`) and **Materialize Consensus (Metadata)** (`env-consensus`), Materialize's own view of object storage and the metadata database, and **Infrastructure Cloud Provider** (`infra-cloud`), which draws the metrics collected by `pipeline.metrics.provider`.
    * **Infrastructure Networking** (`infra-net`) now shows its CNI and Security vendor rows. They were hidden on every cluster, which read as "No Dataplane Metrics".
    * About 50 persist and timestamp-oracle metric families now ship to destinations at `minMetricImportance: recommended`, and the cloud provider families named by `infra-cloud` join the `diagnostic` tier. `pipeline.metrics.provider.*.metricImportance` still decides every tier above `diagnostic`.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0

## materialize-monitoring (Helm chart + Terraform module) v0.26.0

* DEP-301 Pull Azure Monitor metrics into the gateway
    * [materialize-monitoring#417](https://github.com/MaterializeInc/materialize-monitoring/pull/417)
    * **New `pipeline.metrics.provider.azure`**, off by default.
        * It pulls a fixed, minimal set of PostgreSQL Flexible Server and Blob Storage metrics from Azure Monitor, for the servers and storage accounts listed under it. Nothing is discovered.
        * Join on `resourceName`. `instance` is `postgres`, `blob_capacity` or `blob_requests`.
        * See [Cloud Provider Metrics](https://materializeinc.github.io/materialize-monitoring/metrics/collecting/cloud-provider-metrics/).
    * **Credentials come from the gateway pod's identity, never from values.** The identity needs Monitoring Reader on each named resource.
        * Workload identity needs both the `azure.workload.identity/client-id` annotation on `alloy-gateway.serviceAccount` and the `azure.workload.identity/use: "true"` label in `alloy-gateway.controller.podLabels`.
        * The Terraform module sets the label whenever the annotation is present.
    * **Azure families default to the `extended` tier**, like the other providers.
* Alert on freshness, hydration and replica health, split by who acts on it
    * [materialize-monitoring#418](https://github.com/MaterializeInc/materialize-monitoring/pull/418)
    * The default set gains ten alerting rules: `environmentd-down`, `environmentd-not-scraped`, `system-cluster-falling-behind`, `system-cluster-stale`, `system-cluster-hydration-stuck`, `system-cluster-memory-near-limit`, `cluster-replica-not-ready`, `cluster-hydration-stuck`, `cluster-memory-near-limit` and `cluster-replica-oomkilled`.
        * `cluster-falling-behind`, `cluster-stale`, `cluster-memory-high`, `cluster-cpu-high` and `source-disconnected` are new and install only when selected.
        * The user-cluster rules install as a third `PrometheusRule`, `<release>-materialize-workload-alerts`.
    * Every bundled alert carries an `audience` label, `platform` or `workload`, for routing with `alerting.routes.extra`. Alerts about a cluster also carry `cluster_name`.
    * `rules.overrides.<alert>` is new: it sets a rule's `for` and adds or replaces its labels. A deployment whose clusters take longer than an hour to hydrate should lengthen `cluster-hydration-stuck`'s `for`.
* Install the bundled alerting rules as PrometheusRules, gated by capability
    * [materialize-monitoring#408](https://github.com/MaterializeInc/materialize-monitoring/pull/408)
    * The chart now installs alerting rules. `rules.enabled`, default `true`, renders the bundled alerts as `PrometheusRule` resources, which the Thanos ruler evaluates.
        * Eighteen rules are in the default set; the rest install only when named in `rules.selected`.
        * A rule installs only where every capability it requires is present. Capabilities for what the chart deploys are derived; others, such as `crdb-dedicated` or `cilium`, are listed in `rules.capabilities`.
        * The names of the rules in the default set are now part of the committed surface.
    * `rules.capabilities`, `rules.selected`, `rules.disabled`, `rules.namespaces.{environment,operator,exclude}` and `rules.infraWorkloads.{core,important,nonessential,daemonset}` are new.
        * Environment namespaces default to `materialize.namespaces`, then to `materialize-system.namespace`. A deployment whose environments live elsewhere needs to set one of them, or environment-scoped rules match nothing.
        * `rules.infraWorkloads` says which infrastructure workloads the rules treat as core, important, non-essential, or on every node. The defaults cover common EKS and GKE add-ons and this chart's own collectors; list your cluster's own add-ons in the matching tier.
    * `materialize.deploymentMode` now decides the metric prefix the rules read on SQL-backed metrics: `mz_`, or `v2_mz_` for `cloud`. Any other value fails the render.
    * **Removed:** `config.rules.prometheus.enabled`, `config.rules.loki.enabled`, `config.rules.thanos.enabled` and `config.alerts.enabled`, which no template ever read. Use `rules.enabled`. A values file that still sets them renders with a warning.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
    * Adopt the registry's parameters in the alerts, fix what that exposed, and generate the rules
        * [materialize-monitoring#413](https://github.com/MaterializeInc/materialize-monitoring/pull/413)
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Add an alerting render context, capabilities and gen-rules to the query registry
        * [materialize-monitoring#412](https://github.com/MaterializeInc/materialize-monitoring/pull/412)

## Dashboards (Helm chart) v0.18.0

* dashboards: add Infrastructure Autoscaling and Karpenter
    * [materialize-monitoring#425](https://github.com/MaterializeInc/materialize-monitoring/pull/425)
    * Two new dashboards, installed by the default `infra-*` pattern: **Infrastructure Autoscaling** (`infra-autoscaling`), on every cloud, and **Karpenter** (`infra-karpenter`), which has data on EKS where Karpenter's ServiceMonitor is applied.
    * kube-state-metrics now publishes `kube_node_labels` for each provisioner's pool label, the instance type and the zone, set by the new default `kube-state-metrics.metricLabelsAllowlist`. An install that brings its own kube-state-metrics needs the same allowlist for the Node Pools tab.
* Render LogQL alerts from the query registry and deliver them to the Loki ruler
    * [materialize-monitoring#426](https://github.com/MaterializeInc/materialize-monitoring/pull/426)
    * Bundled log-derived alerts, evaluated by the Loki ruler. `materialize-panic`, `data-correctness-error` and `persist-filter-pushdown-violation` are in the default set. `trace-logging-enabled` installs when selected.
    * Every bundled `PrometheusRule` now carries `mzmon.materialize.cloud/flavor: promql` or `logql`. The alloy-gateway writes `logql` ones into the Loki ruler through its API, including a deployment's own. They install only where the ruler's rule store accepts writes; a `local` store, which a filesystem-only Loki gets, leaves them out with a render warning.
    * `thanos.ruler.autoImportPrometheusRules.labelSelector` defaults to `mzmon.materialize.cloud/flavor!: logql`. A replacement selector has to keep that key, or select `flavor: promql`. A cluster running a Prometheus Operator admission webhook has to exclude `flavor: logql` from it.
    * New `rules.logTenants`: the Loki tenants the log-derived rules are written into. Empty means `pipeline.logging.tenancy.staticTenant`. List them under `byEnvironment` tenancy.
    * The generated rule index moved from `pre-rendered/rules/prometheus/_index.yaml` to `pre-rendered/rules/_index.yaml`, and records each rule's `engine`.
* dashboards: add Persist, Consensus and Cloud Provider dashboards
    * [materialize-monitoring#419](https://github.com/MaterializeInc/materialize-monitoring/pull/419)
    * Three new dashboards, installed by default: **Materialize Persist (Storage)** (`env-persist`) and **Materialize Consensus (Metadata)** (`env-consensus`), Materialize's own view of object storage and the metadata database, and **Infrastructure Cloud Provider** (`infra-cloud`), which draws the metrics collected by `pipeline.metrics.provider`.
    * **Infrastructure Networking** (`infra-net`) now shows its CNI and Security vendor rows. They were hidden on every cluster, which read as "No Dataplane Metrics".
    * About 50 persist and timestamp-oracle metric families now ship to destinations at `minMetricImportance: recommended`, and the cloud provider families named by `infra-cloud` join the `diagnostic` tier. `pipeline.metrics.provider.*.metricImportance` still decides every tier above `diagnostic`.
* Alert on freshness, hydration and replica health, split by who acts on it
    * [materialize-monitoring#418](https://github.com/MaterializeInc/materialize-monitoring/pull/418)
    * The default set gains ten alerting rules: `environmentd-down`, `environmentd-not-scraped`, `system-cluster-falling-behind`, `system-cluster-stale`, `system-cluster-hydration-stuck`, `system-cluster-memory-near-limit`, `cluster-replica-not-ready`, `cluster-hydration-stuck`, `cluster-memory-near-limit` and `cluster-replica-oomkilled`.
        * `cluster-falling-behind`, `cluster-stale`, `cluster-memory-high`, `cluster-cpu-high` and `source-disconnected` are new and install only when selected.
        * The user-cluster rules install as a third `PrometheusRule`, `<release>-materialize-workload-alerts`.
    * Every bundled alert carries an `audience` label, `platform` or `workload`, for routing with `alerting.routes.extra`. Alerts about a cluster also carry `cluster_name`.
    * `rules.overrides.<alert>` is new: it sets a rule's `for` and adds or replaces its labels. A deployment whose clusters take longer than an hour to hydrate should lengthen `cluster-hydration-stuck`'s `for`.
* Install the bundled alerting rules as PrometheusRules, gated by capability
    * [materialize-monitoring#408](https://github.com/MaterializeInc/materialize-monitoring/pull/408)
    * The chart now installs alerting rules. `rules.enabled`, default `true`, renders the bundled alerts as `PrometheusRule` resources, which the Thanos ruler evaluates.
        * Eighteen rules are in the default set; the rest install only when named in `rules.selected`.
        * A rule installs only where every capability it requires is present. Capabilities for what the chart deploys are derived; others, such as `crdb-dedicated` or `cilium`, are listed in `rules.capabilities`.
        * The names of the rules in the default set are now part of the committed surface.
    * `rules.capabilities`, `rules.selected`, `rules.disabled`, `rules.namespaces.{environment,operator,exclude}` and `rules.infraWorkloads.{core,important,nonessential,daemonset}` are new.
        * Environment namespaces default to `materialize.namespaces`, then to `materialize-system.namespace`. A deployment whose environments live elsewhere needs to set one of them, or environment-scoped rules match nothing.
        * `rules.infraWorkloads` says which infrastructure workloads the rules treat as core, important, non-essential, or on every node. The defaults cover common EKS and GKE add-ons and this chart's own collectors; list your cluster's own add-ons in the matching tier.
    * `materialize.deploymentMode` now decides the metric prefix the rules read on SQL-backed metrics: `mz_`, or `v2_mz_` for `cloud`. Any other value fails the render.
    * **Removed:** `config.rules.prometheus.enabled`, `config.rules.loki.enabled`, `config.rules.thanos.enabled` and `config.alerts.enabled`, which no template ever read. Use `rules.enabled`. A values file that still sets them renders with a warning.
* Adopt the registry's parameters in the alerts, fix what that exposed, and generate the rules
    * [materialize-monitoring#413](https://github.com/MaterializeInc/materialize-monitoring/pull/413)

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate promql-parser to 0.11.0
        * [materialize-monitoring#428](https://github.com/MaterializeInc/materialize-monitoring/pull/428)
        * [`v0.11.0`](https://redirect.github.com/GreptimeTeam/promql-parser/releases/tag/v0.11.0)
    * chore(deps): update rust crate tokio-rustls to v0.26.6
        * [materialize-monitoring#427](https://github.com/MaterializeInc/materialize-monitoring/pull/427)
    * DEP-233 Pull instance availability from each cloud's monitoring API
        * [materialize-monitoring#422](https://github.com/MaterializeInc/materialize-monitoring/pull/422)
        * Pull what each cloud publishes about instance availability, off by default:
            * `pipeline.metrics.provider.cloudwatch.eks.clusters` pulls EC2 status checks for every node of the listed EKS clusters, their managed node groups' sizes, and the region's On-Demand vCPU usage. It needs `cloudwatch:GetMetricData`, `cloudwatch:ListMetrics`, `tag:GetResources` and `autoscaling:DescribeAutoScalingGroups`.
            * `pipeline.metrics.provider.gcp.compute.regions` pulls per-family CPU and local-SSD quota, usage against limit, and refusals. `roles/monitoring.viewer` already covers it.
            * `pipeline.metrics.provider.azure.aks.clusters` pulls the AKS cluster autoscaler's gauges and each node VM's availability. It needs Monitoring Reader on each cluster and its node resource group.
            * The new families (`aws_ec2_*`, `aws_autoscaling_*`, `aws_usage_*`, `stackdriver_compute_googleapis_com_location_*`, `azure_microsoft_containerservice_managedclusters_*` and `azure_microsoft_compute_virtualmachinescalesets_*`) take each provider's `metricImportance`.
    * DEP-301 Pull Azure Monitor metrics into the gateway
        * [materialize-monitoring#417](https://github.com/MaterializeInc/materialize-monitoring/pull/417)
        * **New `pipeline.metrics.provider.azure`**, off by default.
            * It pulls a fixed, minimal set of PostgreSQL Flexible Server and Blob Storage metrics from Azure Monitor, for the servers and storage accounts listed under it. Nothing is discovered.
            * Join on `resourceName`. `instance` is `postgres`, `blob_capacity` or `blob_requests`.
            * See [Cloud Provider Metrics](https://materializeinc.github.io/materialize-monitoring/metrics/collecting/cloud-provider-metrics/).
        * **Credentials come from the gateway pod's identity, never from values.** The identity needs Monitoring Reader on each named resource.
            * Workload identity needs both the `azure.workload.identity/client-id` annotation on `alloy-gateway.serviceAccount` and the `azure.workload.identity/use: "true"` label in `alloy-gateway.controller.podLabels`.
            * The Terraform module sets the label whenever the annotation is present.
        * **Azure families default to the `extended` tier**, like the other providers.
    * Add an alerting render context, capabilities and gen-rules to the query registry
        * [materialize-monitoring#412](https://github.com/MaterializeInc/materialize-monitoring/pull/412)

## materialize-monitoring (Helm chart + Terraform module) v0.25.1

* Update Helm release alloy to v1.13.0; bump to v0.25.1
    * [materialize-monitoring#409](https://github.com/MaterializeInc/materialize-monitoring/pull/409)
    * [`v1.13.0`](https://redirect.github.com/grafana/helm-charts/releases/tag/tempo-1.13.0)

## materialize-monitoring (Helm chart + Terraform module) v0.25.0

* DEP-229 Update thanos to v0.46.0 so its selectors stop matching Loki
    * [materialize-monitoring#402](https://github.com/MaterializeInc/materialize-monitoring/pull/402)
    * **Fixed: Thanos no longer selects Loki's pods.** Thanos's compactor, query-frontend and ruler selected on component and release name alone, which Loki's pods of the same names also carry.
        * **Node drains could not evict a Loki ruler.** Each was covered by both the `loki-ruler` and `thanos-ruler` PodDisruptionBudgets, and the eviction API refuses a pod with two. The Loki query frontend had the same problem wherever `thanos.queryFrontend.enabled` was set.
        * Thanos's NetworkPolicies, which allow all egress, also applied to the Loki ruler and compactor, so Loki's own egress rules did not restrict them.
        * Thanos Query no longer dials the Loki rulers' gRPC port, which logged a warning every five seconds per ruler.
    * **Upgrading needs one manual step.** Deployment and StatefulSet selectors are immutable, so the upgrade fails at the first Thanos workload unless those workloads are first deleted with `--cascade=orphan`. The pods keep running and are re-adopted without a restart, except in a workload whose template was changed outside Helm (usually by `kubectl rollout restart`), which rolls once. See [Upgrading](https://materializeinc.github.io/materialize-monitoring/operating/upgrading/#thanos-selectors-name-the-chart).
        * `kubectl -n monitoring delete statefulset,deployment -l app.kubernetes.io/part-of=thanos,app.kubernetes.io/instance=mzmon --cascade=orphan`
        * An upgrade that already failed on this recovers the same way, then runs again.
    * Resource names, pod labels, the Thanos image and the `thanos-thanos` ServiceAccount that workload identity bindings name are unchanged.
    * [`v0.46.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.46.0)
    * [`v0.45.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.45.0)
    * [`v0.44.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.44.0)
    * [`v0.43.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.43.0)
    * [`v0.42.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.42.0)
    * [`v0.41.1`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.41.1)
    * [`v0.41.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.41.0)
    * [`v0.40.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.40.0)
    * [`v0.39.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.39.0)
    * [`v0.38.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.38.0)
    * [`v0.37.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.37.0)
    * [`v0.36.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.36.0)
    * [`v0.35.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.35.0)
    * [`v0.34.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.34.0)
    * [`v0.33.1`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.33.1)
    * [`v0.33.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.33.0)
    * [`v0.32.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.32.0)
    * [`v0.31.0`](https://redirect.github.com/thanos-community/helm-charts/releases/tag/thanos-0.31.0)
* DEP-301 Pull CloudWatch and Cloud Monitoring metrics into the gateway
    * [materialize-monitoring#396](https://github.com/MaterializeInc/materialize-monitoring/pull/396)
    * **New `pipeline.metrics.provider.cloudwatch` and `pipeline.metrics.provider.gcp`**, both off by default.
        * They pull a fixed, minimal set of RDS and S3 metrics from CloudWatch, and of Cloud SQL and GCS metrics from Cloud Monitoring, for the resources listed under each. Nothing is discovered.
        * On CloudWatch series `instance` is the resource; join on `dimension_DBInstanceIdentifier` or `dimension_BucketName`.
        * See [Cloud Provider Metrics](https://materializeinc.github.io/materialize-monitoring/metrics/collecting/cloud-provider-metrics/).
    * **Credentials come from the gateway pod's identity, never from values.**
        * CloudWatch needs `cloudwatch:GetMetricStatistics`, and `iam:ListAccountAliases` for the `account_alias` label, through IRSA, EKS Pod Identity, or static keys in the `mzmon-alloy-gateway-env` Secret.
        * GCP needs `roles/monitoring.viewer` through Workload Identity.
    * **Outside Google Cloud, a GCP pull needs `GOOGLE_APPLICATION_CREDENTIALS`.** With no credential at all, the exporter cannot start and the gateway fails to load.
    * **Provider families default to the `extended` tier.** They reach the bundled Thanos, and not a destination that filters at `recommended` or `essential`, unless `metricImportance` is raised.
    * **Provider data is minutes old.** Query it with `last_over_time(...[15m])` or wider. Each gateway restart leaves a gap in Cloud Monitoring series as long as that lag.
* Make both rulers' remote-write to the gateway work under the mTLS profiles
    * [materialize-monitoring#403](https://github.com/MaterializeInc/materialize-monitoring/pull/403)
    * Both rulers' remote-write to the gateway now works under `profiles/mtls.values.yaml` (Terraform: `internal_tls`), and the gateway now accepts rule results it had been rejecting for lacking `job` and `instance`. `ALERTS` and recording-rule results reach every metrics destination.
    * The Thanos ruler's remote-write ConfigMaps are renamed `thanos-ruler-remote-write-v2` and `thanos-ruler-remote-write-v2-tls`. A deployment that restates `thanos.ruler.extraVolumes` must use the new name; the render says which.
    * Installing with Helm directly: restart the Alloy gateway after upgrading, as for any pipeline change.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
    * Add an Alloy meta-monitoring dashboard, and fix infra-loki's level picker
        * [materialize-monitoring#397](https://github.com/MaterializeInc/materialize-monitoring/pull/397)
        * **New dashboard: Alloy Meta Monitoring** (`mz-mon-infra-alloy`), in the Meta Observability folder. Collector health, both pipelines hop by hop, ingest over log push, remote write and OTLP, component and configuration state with uptime, gateway clustering, resource limits including `GOMEMLIMIT`, Kubernetes events including the pre-install validation Jobs, and Alloy's own logs.
        * Fixed the Level picker on Loki Meta Monitoring, which offered only "All".
        * Metric tiers now include the Alloy families the new dashboard reads. `loki_source_file_read_bytes_total` leaves the tiers, and the generic Go runtime families it adds are `extended`, so a metered destination on the default tier does not receive every target's copy.
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate thiserror to v2.0.21
        * [materialize-monitoring#399](https://github.com/MaterializeInc/materialize-monitoring/pull/399)
        * [`v2.0.21`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.21)

## materialize-monitoring (Helm chart + Terraform module) v0.24.0

* DEP-226 Serve Alertmanager's API over TLS through the mTLS profiles
    * [materialize-monitoring#394](https://github.com/MaterializeInc/materialize-monitoring/pull/394)
    * Alertmanager can serve its API over TLS: `alerting.server.tls`, turned on by `profiles/mtls.values.yaml` (Terraform: `internal_tls`). Phase 2 verifies client certificates when given; the port stops there, like Loki's HTTP port.
    * The mTLS profiles now move both rulers, the probes, the config reloader, the ServiceMonitor and the Grafana datasource to TLS with it. Turning it on rolls Alertmanager, both rulers and every Loki component.
    * `amtool` inside the Alertmanager pod no longer needs `--alertmanager.url`.
* Make Alertmanager highly available, and configurable from `alerting`
    * [materialize-monitoring#393](https://github.com/MaterializeInc/materialize-monitoring/pull/393)
    * The bundled Alertmanager now runs two gossiping replicas, spread across zones and hardened. Clusters whose nodes carry no zone label need the `no-zone-spread` profile (Terraform: `min_zones = 0`).
    * Alertmanager's resources are renamed to `alertmanager` and start on new volumes; the orphaned `storage-<release>-alertmanager-0` PVC can be deleted.
    * **New `alerting` key** configures routing: `receivers` (Alertmanager's own receiver config plus a `class`), `preset`/`presets` (severity to class), and `routes.extra`. Receiver credentials are read from Secrets through `*_file` fields, and an inline one fails the render.
    * **New top-level `clusterName`** labels every log line, metric and alert as `cluster`, replacing setting `pipeline.env.CLUSTER_NAME` directly (Terraform's `cluster_name` now writes it). Give each cluster its own name.
    * Grafana gains an Alertmanager datasource, and Alertmanager's own metrics are scraped.
* DEP-216 Turn on both rule evaluators and wire them to Alertmanager
    * [materialize-monitoring#384](https://github.com/MaterializeInc/materialize-monitoring/pull/384)
    * **Thanos Ruler is now deployed by default** (`thanos.ruler.enabled: true`, 2 replicas). It runs stateless — no PersistentVolumeClaim — and remote-writes rule results to the alloy-gateway. Set `thanos.ruler.enabled: false` to keep the previous shape.
    * **Every `PrometheusRule` in the cluster is now evaluated and notified through the bundled Alertmanager.** This is the upstream import default. On a cluster running another rule owner, such as kube-prometheus-stack, set `thanos.ruler.autoImportPrometheusRules.labelSelector` to narrow the set.
    * **A new image is pulled by default:** `docker.io/alpine/kubectl`, for the rule-import sidecar. It must carry a shell and `curl`, so a distroless `kubectl` will crash-loop. The four `profiles/registry/` overlays already address it.
    * **The Loki ruler now notifies Alertmanager** (`loki.loki.rulerConfig.alertmanager_url`) and remote-writes recording-rule samples to the alloy-gateway. It previously did neither.
    * **`split-namespace` retargets both rulers** and opens Alertmanager to the ruler namespaces. The Loki ruler's own egress cannot be opened from values in Distributed mode — supply a NetworkPolicy for it, or set `loki.networkPolicy.enabled: false`.
    * The Terraform `storage_class` variable now also reaches `thanos.ruler`, so re-enabling its persistence picks up the class rather than silently missing it.
* Fall back to CLUSTER_NAME for the gateway's log cluster label
    * [materialize-monitoring#385](https://github.com/MaterializeInc/materialize-monitoring/pull/385)
* Add a Loki meta-monitoring dashboard, and split dashboards into their own chart
    * [materialize-monitoring#383](https://github.com/MaterializeInc/materialize-monitoring/pull/383)
    * **Dashboards now install from a separate chart.** `materialize-monitoring-dashboards` is a release of its own, installed beside `materialize-monitoring` in the same namespace. The umbrella chart no longer creates dashboards; a release that upgrades without installing the new chart will have its dashboards removed. Helm stores a release in a Kubernetes Secret and a Secret may not exceed 1 MiB, which the rendered set outgrew.
        * Terraform installs it automatically — set `enable_dashboards = false` to opt out.
        * `dashboards.selected` → the new chart's `selected`.
        * `dashboards.config.grafana.manifest.apiTarget` → the new chart's `grafana.apiTarget`.
        * `dashboards.config.datadog` is removed; it drove nothing.
        * The new chart cannot read the umbrella release's values, so `grafana.instanceSelector` and `grafana.folderUids` must match it. The umbrella chart's install notes print the folder UIDs.
        * Dashboard UIDs are unchanged, so saved links, playlists and alerts keep working.
    * **New dashboard: Loki Meta Monitoring** (`mz-mon-infra-loki`), in the Meta Observability folder. Ingest, queries, object storage, retention, and Loki's own logs.
    * **Fixed: the Loki canary and both memcached exporters were never scraped under `profiles/mtls`.** The subchart's single ServiceMonitor applied one `scheme` to every target, including three that only serve plaintext. They are now collected by a separate monitor, and carry `prometheus.io/service-monitor: "false"` plus `monitoring.materialize.cloud/scrape-scheme: plaintext` on their Services. Installs using mTLS gain `loki_canary_*` and `memcached_*` series that were previously absent.
    * New Terraform inputs: `enable_dashboards`, `dashboards_chart_version`, `dashboards_selected`, `dashboards_instance_selector`, `dashboards_allow_cross_namespace_import`.

### Dependencies

* Included Pipelines @ v0.12.0..v0.13.0
    * Reassemble and classify Rust panics, and parse tracing's plain text format
        * [materialize-monitoring#376](https://github.com/MaterializeInc/materialize-monitoring/pull/376)
        * Rust panics from Materialize services now arrive as a **single log entry** rather than one entry per line of the backtrace, carrying `level=CRITICAL` and `panic_thread` / `panic_location` as structured metadata. The `msg` names the source location and the panic message.
        * `balancerd` and `materialize-operator` logs now carry a parsed `level` and `target`. Both previously landed as `level="UNKNOWN"` for every line.
        * **React to this if you filter or size on log level.** Those services' `WARN` and `ERROR` lines are no longer swept into the `UNKNOWN` rate-limit bucket, which drops, so they now reach Loki reliably and ingested volume from the operator namespace rises. A saved query or alert matching `level="UNKNOWN"` on these services will stop matching.
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate jsonschema to 0.57.0
        * [materialize-monitoring#390](https://github.com/MaterializeInc/materialize-monitoring/pull/390)
        * [`v0.57.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0570---2026-09-22)

## Dashboards (Helm chart) v0.17.0

* Add an Alloy meta-monitoring dashboard, and fix infra-loki's level picker
    * [materialize-monitoring#397](https://github.com/MaterializeInc/materialize-monitoring/pull/397)
    * **New dashboard: Alloy Meta Monitoring** (`mz-mon-infra-alloy`), in the Meta Observability folder. Collector health, both pipelines hop by hop, ingest over log push, remote write and OTLP, component and configuration state with uptime, gateway clustering, resource limits including `GOMEMLIMIT`, Kubernetes events including the pre-install validation Jobs, and Alloy's own logs.
    * Fixed the Level picker on Loki Meta Monitoring, which offered only "All".
    * Metric tiers now include the Alloy families the new dashboard reads. `loki_source_file_read_bytes_total` leaves the tiers, and the generic Go runtime families it adds are `extended`, so a metered destination on the default tier does not receive every target's copy.
* Add a Loki meta-monitoring dashboard, and split dashboards into their own chart
    * [materialize-monitoring#383](https://github.com/MaterializeInc/materialize-monitoring/pull/383)
    * **Dashboards now install from a separate chart.** `materialize-monitoring-dashboards` is a release of its own, installed beside `materialize-monitoring` in the same namespace. The umbrella chart no longer creates dashboards; a release that upgrades without installing the new chart will have its dashboards removed. Helm stores a release in a Kubernetes Secret and a Secret may not exceed 1 MiB, which the rendered set outgrew.
        * Terraform installs it automatically — set `enable_dashboards = false` to opt out.
        * `dashboards.selected` → the new chart's `selected`.
        * `dashboards.config.grafana.manifest.apiTarget` → the new chart's `grafana.apiTarget`.
        * `dashboards.config.datadog` is removed; it drove nothing.
        * The new chart cannot read the umbrella release's values, so `grafana.instanceSelector` and `grafana.folderUids` must match it. The umbrella chart's install notes print the folder UIDs.
        * Dashboard UIDs are unchanged, so saved links, playlists and alerts keep working.
    * **New dashboard: Loki Meta Monitoring** (`mz-mon-infra-loki`), in the Meta Observability folder. Ingest, queries, object storage, retention, and Loki's own logs.
    * **Fixed: the Loki canary and both memcached exporters were never scraped under `profiles/mtls`.** The subchart's single ServiceMonitor applied one `scheme` to every target, including three that only serve plaintext. They are now collected by a separate monitor, and carry `prometheus.io/service-monitor: "false"` plus `monitoring.materialize.cloud/scrape-scheme: plaintext` on their Services. Installs using mTLS gain `loki_canary_*` and `memcached_*` series that were previously absent.
    * New Terraform inputs: `enable_dashboards`, `dashboards_chart_version`, `dashboards_selected`, `dashboards_instance_selector`, `dashboards_allow_cross_namespace_import`.

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * DEP-301 Pull CloudWatch and Cloud Monitoring metrics into the gateway
        * [materialize-monitoring#396](https://github.com/MaterializeInc/materialize-monitoring/pull/396)
        * **New `pipeline.metrics.provider.cloudwatch` and `pipeline.metrics.provider.gcp`**, both off by default.
            * They pull a fixed, minimal set of RDS and S3 metrics from CloudWatch, and of Cloud SQL and GCS metrics from Cloud Monitoring, for the resources listed under each. Nothing is discovered.
            * On CloudWatch series `instance` is the resource; join on `dimension_DBInstanceIdentifier` or `dimension_BucketName`.
            * See [Cloud Provider Metrics](https://materializeinc.github.io/materialize-monitoring/metrics/collecting/cloud-provider-metrics/).
        * **Credentials come from the gateway pod's identity, never from values.**
            * CloudWatch needs `cloudwatch:GetMetricStatistics`, and `iam:ListAccountAliases` for the `account_alias` label, through IRSA, EKS Pod Identity, or static keys in the `mzmon-alloy-gateway-env` Secret.
            * GCP needs `roles/monitoring.viewer` through Workload Identity.
        * **Outside Google Cloud, a GCP pull needs `GOOGLE_APPLICATION_CREDENTIALS`.** With no credential at all, the exporter cannot start and the gateway fails to load.
        * **Provider families default to the `extended` tier.** They reach the bundled Thanos, and not a destination that filters at `recommended` or `essential`, unless `metricImportance` is raised.
        * **Provider data is minutes old.** Query it with `last_over_time(...[15m])` or wider. Each gateway restart leaves a gap in Cloud Monitoring series as long as that lag.
    * Update Rust crate thiserror to v2.0.21
        * [materialize-monitoring#399](https://github.com/MaterializeInc/materialize-monitoring/pull/399)
        * [`v2.0.21`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.21)
    * Update Rust crate jsonschema to 0.57.0
        * [materialize-monitoring#390](https://github.com/MaterializeInc/materialize-monitoring/pull/390)
        * [`v0.57.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0570---2026-09-22)
    * Fall back to CLUSTER_NAME for the gateway's log cluster label
        * [materialize-monitoring#385](https://github.com/MaterializeInc/materialize-monitoring/pull/385)
    * Reassemble and classify Rust panics, and parse tracing's plain text format
        * [materialize-monitoring#376](https://github.com/MaterializeInc/materialize-monitoring/pull/376)
        * Rust panics from Materialize services now arrive as a **single log entry** rather than one entry per line of the backtrace, carrying `level=CRITICAL` and `panic_thread` / `panic_location` as structured metadata. The `msg` names the source location and the panic message.
        * `balancerd` and `materialize-operator` logs now carry a parsed `level` and `target`. Both previously landed as `level="UNKNOWN"` for every line.
        * **React to this if you filter or size on log level.** Those services' `WARN` and `ERROR` lines are no longer swept into the `UNKNOWN` rate-limit bucket, which drops, so they now reach Loki reliably and ingested volume from the operator namespace rises. A saved query or alert matching `level="UNKNOWN"` on these services will stop matching.

## materialize-monitoring (Helm chart + Terraform module) v0.23.0

* Clean up GrafanaFolder on uninstall
    * [materialize-monitoring#375](https://github.com/MaterializeInc/materialize-monitoring/pull/375)
    * Fixed an uninstall hang introduced in v0.22.0: the `pre-delete` cleanup hook did not delete `GrafanaFolder` resources, so their grafana-operator finalizers outlived the operator and left the release namespace stuck in `Terminating`. Installs that already hit this can recover by clearing the finalizers by hand — see [Uninstalling](https://materializeinc.github.io/materialize-monitoring/operating/uninstalling/#recovering-a-stuck-teardown).
* Upgrade helm-unittest to v1.1.2 and pin it for Renovate
    * [materialize-monitoring#373](https://github.com/MaterializeInc/materialize-monitoring/pull/373)
* DEP-211 Add infra-net dashboard, and the CNI collection it needs
    * [materialize-monitoring#366](https://github.com/MaterializeInc/materialize-monitoring/pull/366)

### Dependencies

* Included Dashboards @ v0.16.0..v0.17.0
* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0

## Container Images v0.6.0

* chore(deps): update dependency grafana/alloy to v1.20.0
    * [materialize-monitoring#406](https://github.com/MaterializeInc/materialize-monitoring/pull/406)
    * [`v1.20.0`](https://redirect.github.com/grafana/alloy/releases/tag/v1.20.0)
* Update debian:13 Docker digest to 9cc0800
    * [materialize-monitoring#379](https://github.com/MaterializeInc/materialize-monitoring/pull/379)

## materialize-monitoring (Helm chart + Terraform module) v0.22.0

* Update docker.io/grafana/grafana Docker tag to v13.2.2
    * [materialize-monitoring#359](https://github.com/MaterializeInc/materialize-monitoring/pull/359)
    * [`v13.2.2`](https://redirect.github.com/grafana/grafana/releases/tag/v13.2.2): 13.2.2
* Update docker.io/grafana/grafana Docker tag to v13.2.1
    * [materialize-monitoring#321](https://github.com/MaterializeInc/materialize-monitoring/pull/321)
    * [`v13.2.1`](https://redirect.github.com/grafana/grafana/compare/v13.2.0...v13.2.1)
* Move the chart's alloy image to v1.19.2-mz3
    * [materialize-monitoring#348](https://github.com/MaterializeInc/materialize-monitoring/pull/348)
    * The bundled Alloy image moves from `v1.18.1-mz2` to `v1.19.2-mz3`, which is Alloy v1.19.2 on a rebased distroless base. This is an Alloy minor upgrade for anyone who has not overridden `alloy-agent.image` / `alloy-gateway.image`; see the [Alloy v1.19 release notes](https://github.com/grafana/alloy/releases/tag/v1.19.0).
* Add Dashboard Folders; Update tags
    * [materialize-monitoring#329](https://github.com/MaterializeInc/materialize-monitoring/pull/329)
    * Added several GrafanaFolder resources (default enabled: mzmon-infra, mzmon-materialize, mzmon-meta-o11y)
    * Changed monitoring tag to mzmon within dashboards

### Dependencies

* Included Dashboards @ v0.15.0..v0.16.0
    * Show total lag in env-top / env-upgrade
        * [materialize-monitoring#312](https://github.com/MaterializeInc/materialize-monitoring/pull/312)
        * Add new queries around max lag (materialize.compute.freshness.lag_total_by_cluster, materialize.generations.lag.total)
        * Show max lag as new panels (including per-cluster breakdown) in env-top and env-upgrade dashboards
* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate clap to v4.6.7
        * [materialize-monitoring#355](https://github.com/MaterializeInc/materialize-monitoring/pull/355)
        * [`v4.6.7`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.6...clap_complete-v4.6.7)
    * Update Rust crate rustls to v0.23.45
        * [materialize-monitoring#349](https://github.com/MaterializeInc/materialize-monitoring/pull/349)
    * Update Rust crate jsonschema to 0.56.0
        * [materialize-monitoring#267](https://github.com/MaterializeInc/materialize-monitoring/pull/267)
        * [`v0.56.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0560---2026-09-10)
        * [`v0.55.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0551---2026-09-08)
        * [`v0.55.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0550---2026-09-06)
        * [`v0.54.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0540---2026-09-06)
        * [`v0.53.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0530---2026-09-02)
        * [`v0.52.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0521---2026-08-30)
        * [`v0.52.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0520---2026-08-26)
        * [`v0.51.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0510---2026-08-23)
        * [`v0.50.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0501---2026-08-22)
        * [`v0.50.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0500---2026-08-20)
    * Update Rust crate reqwest to v0.13.5
        * [materialize-monitoring#335](https://github.com/MaterializeInc/materialize-monitoring/pull/335)
        * [`v0.13.5`](https://redirect.github.com/seanmonstar/reqwest/blob/HEAD/CHANGELOG.md#v0135)
    * Update Rust crate rustls to v0.23.44
        * [materialize-monitoring#332](https://github.com/MaterializeInc/materialize-monitoring/pull/332)
    * Update Rust crate tokio-rustls to v0.26.5
        * [materialize-monitoring#326](https://github.com/MaterializeInc/materialize-monitoring/pull/326)
    * Add some sample prose in heather's voice
        * [materialize-monitoring#331](https://github.com/MaterializeInc/materialize-monitoring/pull/331)
    * Update Rust crate indexmap to v2.14.2
        * [materialize-monitoring#328](https://github.com/MaterializeInc/materialize-monitoring/pull/328)
        * [`v2.14.2`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2142-2026-09-04)

## Dashboards (Helm chart) v0.16.0

* DEP-211 Add infra-net dashboard, and the CNI collection it needs
    * [materialize-monitoring#366](https://github.com/MaterializeInc/materialize-monitoring/pull/366)
* Add Dashboard Folders; Update tags
    * [materialize-monitoring#329](https://github.com/MaterializeInc/materialize-monitoring/pull/329)
    * Added several GrafanaFolder resources (default enabled: mzmon-infra, mzmon-materialize, mzmon-meta-o11y)
    * Changed monitoring tag to mzmon within dashboards
* Show total lag in env-top / env-upgrade
    * [materialize-monitoring#312](https://github.com/MaterializeInc/materialize-monitoring/pull/312)
    * Add new queries around max lag (materialize.compute.freshness.lag_total_by_cluster, materialize.generations.lag.total)
    * Show max lag as new panels (including per-cluster breakdown) in env-top and env-upgrade dashboards

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate clap to v4.6.7
        * [materialize-monitoring#355](https://github.com/MaterializeInc/materialize-monitoring/pull/355)
        * [`v4.6.7`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.6...clap_complete-v4.6.7)
    * Update Rust crate rustls to v0.23.45
        * [materialize-monitoring#349](https://github.com/MaterializeInc/materialize-monitoring/pull/349)
    * Update Rust crate jsonschema to 0.56.0
        * [materialize-monitoring#267](https://github.com/MaterializeInc/materialize-monitoring/pull/267)
        * [`v0.56.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0560---2026-09-10)
        * [`v0.55.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0551---2026-09-08)
        * [`v0.55.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0550---2026-09-06)
        * [`v0.54.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0540---2026-09-06)
        * [`v0.53.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0530---2026-09-02)
        * [`v0.52.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0521---2026-08-30)
        * [`v0.52.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0520---2026-08-26)
        * [`v0.51.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0510---2026-08-23)
        * [`v0.50.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0501---2026-08-22)
        * [`v0.50.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0500---2026-08-20)
    * Update Rust crate reqwest to v0.13.5
        * [materialize-monitoring#335](https://github.com/MaterializeInc/materialize-monitoring/pull/335)
        * [`v0.13.5`](https://redirect.github.com/seanmonstar/reqwest/blob/HEAD/CHANGELOG.md#v0135)
    * Update Rust crate rustls to v0.23.44
        * [materialize-monitoring#332](https://github.com/MaterializeInc/materialize-monitoring/pull/332)
    * Update Rust crate tokio-rustls to v0.26.5
        * [materialize-monitoring#326](https://github.com/MaterializeInc/materialize-monitoring/pull/326)
    * Add some sample prose in heather's voice
        * [materialize-monitoring#331](https://github.com/MaterializeInc/materialize-monitoring/pull/331)
    * Update Rust crate indexmap to v2.14.2
        * [materialize-monitoring#328](https://github.com/MaterializeInc/materialize-monitoring/pull/328)
        * [`v2.14.2`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2142-2026-09-04)

## Dashboards v0.15.0

* DEP-242 Add infra-nodes dashboard
    * [materialize-monitoring#311](https://github.com/MaterializeInc/materialize-monitoring/pull/311)
    * Adds an infra-nodes dashboard that is installed by default

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate indexmap to v2.14.1
        * [materialize-monitoring#309](https://github.com/MaterializeInc/materialize-monitoring/pull/309)
        * [`v2.14.1`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2141-2026-08-28)

## materialize-monitoring (Helm chart + Terraform module) v0.21.0

* DEP-242 Add infra-nodes dashboard
    * [materialize-monitoring#311](https://github.com/MaterializeInc/materialize-monitoring/pull/311)
    * Adds an infra-nodes dashboard that is installed by default
* DEP-209 Add Infrastructure Logs & Events Dashboard
    * [materialize-monitoring#307](https://github.com/MaterializeInc/materialize-monitoring/pull/307)
    * Adds new infra-logs dashboard that is enabled by default

### Dependencies

* Included Dashboards @ v0.15.0..v0.16.0
* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate indexmap to v2.14.1
        * [materialize-monitoring#309](https://github.com/MaterializeInc/materialize-monitoring/pull/309)
        * [`v2.14.1`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2141-2026-08-28)
    * Update Rust crate hyper to v1.11.1
        * [materialize-monitoring#302](https://github.com/MaterializeInc/materialize-monitoring/pull/302)
        * [`v1.11.1`](https://redirect.github.com/hyperium/hyper/blob/HEAD/CHANGELOG.md#v1111-2026-08-27)

## Pipelines v0.13.0 (Unreleased)

_Changes Pending_

## Container Images v0.5.0

* Update gcr.io/distroless/base-debian13 Docker digest to 0ebad35
    * [materialize-monitoring#340](https://github.com/MaterializeInc/materialize-monitoring/pull/340)

## Dashboards v0.14.0

* DEP-209 Add Infrastructure Logs & Events Dashboard
    * [materialize-monitoring#307](https://github.com/MaterializeInc/materialize-monitoring/pull/307)
    * Adds new infra-logs dashboard that is enabled by default

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate hyper to v1.11.1
        * [materialize-monitoring#302](https://github.com/MaterializeInc/materialize-monitoring/pull/302)
        * [`v1.11.1`](https://redirect.github.com/hyperium/hyper/blob/HEAD/CHANGELOG.md#v1111-2026-08-27)

## Prometheus Scrapers v0.5.0 (Unreleased)

_Changes Pending_

## mzmon-lib (shared library) v0.12.0

* Record the normalized ext:consensus_* series, with a producer for the registry's rules: branch
    * [materialize-monitoring#475](https://github.com/MaterializeInc/materialize-monitoring/pull/475)
    * New recording rules record the metadata (consensus) database as normalized `ext:consensus_*` series, listed on the new Recorded Series reference page. They install wherever `rules.enabled` is true and their source is present; no selection is needed.
        * `ext:consensus_up` and `ext:consensus_commit_latency_seconds:p99` come from Materialize's own calls, carry `flavor="persist"` and `namespace`, and need no configuration.
        * `ext:consensus_up`, `ext:consensus_storage_used_ratio` and `ext:consensus_xid_used_ratio` from the CloudWatch, Cloud Monitoring and Azure Monitor pulls carry `flavor` (`rds`, `cloudsql`, `azure-postgres`) and `resource`, where the provider publishes each.
    * New value `externalDependencies.consensus`, a list of `{flavor, resourceId}` naming which databases a provider pull watches are a metadata database. The provider-sourced `ext:consensus_*` series record only these, so a pull that also watches Grafana's database no longer needs to be told apart by hand. The render warns when a pull watches databases and none is named.
    * `rules.capabilities` gains three derived capabilities, `cloudwatch`, `cloud-monitoring` and `azure-monitor`, present when the matching `pipeline.metrics.provider.*` pull is enabled.
* Google Cloud metrics: export over OTLP to the Telemetry API, typed
    * [materialize-monitoring#474](https://github.com/MaterializeInc/materialize-monitoring/pull/474)
    * **Changed:** the Google Cloud metrics destination writes somewhere else. `googleCloudExporter` (Terraform `google_cloud_metrics`) now sends OTLP to Google's Telemetry API instead of using `otelcol.exporter.googlecloud`, and metrics land as `prometheus.googleapis.com/<name>/<kind>` instead of `workload.googleapis.com/mzmon/<name>`. **Cloud Monitoring dashboards, alerting policies and anything else reading the old metric types stop receiving data and have to be repointed.** The old types are not deleted.
        * **Enable the `telemetry.googleapis.com` API on the project before upgrading with this destination on.** Without it every export is refused, and nothing else fails. `roles/monitoring.metricWriter` is still the only role needed.
        * Metrics are billed per sample ingested instead of per byte, about $75 a month at `recommended` on a test install where the old export cost about $2,400.
        * Series carry the `prometheus_target` labels (`project_id`, `location`, `cluster`, `namespace`, `job`, `instance`) and `collected_by="materialize-monitoring"`.
        * Counter values start from zero at the gateway's first scrape, so they differ from Thanos; `rate()` and `increase()` agree.
        * New values `googleCloudExporter.project` and `googleCloudExporter.location` are needed only off GKE.
    * **Changed:** there is no metric prefix any more. `googleCloudExporter.prefix` is removed from the chart values, along with `instrumentation_library_labels`, `skip_create_descriptor` and `service_resource_labels`; setting any of them renders a warning and does nothing.
    * **Deprecated:** Terraform `google_cloud_metrics.prefix` is ignored and warns at plan time; remove it. There is no replacement, because the Telemetry API has no prefix to choose. Everything else in `google_cloud_metrics` is unchanged.
    * **Changed:** the gateway's scrapes honor metric metadata, so every OTLP destination (Google Cloud, Datadog, generic OTLP) receives typed metrics: counters as cumulative sums, histograms as histograms. Metric names in Datadog change accordingly. Thanos and other remote-write destinations are unchanged.
    * `alloy-gateway.alloy.stabilityLevel` now defaults to `experimental`, which the scrapes require; the chart refuses any other level.
    * The agent and gateway now run the Alloy v1.20.0 image their `image.tag` names. Since the bump to `v1.20.0-mz3`, a stale `image.digest` had kept both on Alloy v1.19.2.
    * `denyMetrics` entries match per Prometheus series against typed histograms: denying `foo_bucket` still keeps `foo_count` and `foo_sum`.
* Update Rust crate hyper-util to v0.1.21
    * [materialize-monitoring#404](https://github.com/MaterializeInc/materialize-monitoring/pull/404)
    * [`v0.1.21`](https://redirect.github.com/hyperium/hyper-util/blob/HEAD/CHANGELOG.md#0121-2026-09-24)
* Update Rust crate jsonschema to 0.58.0
    * [materialize-monitoring#407](https://github.com/MaterializeInc/materialize-monitoring/pull/407)
    * [`v0.58.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0582---2026-09-28)
    * [`v0.58.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0581---2026-09-26)
    * [`v0.58.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0580---2026-09-25)
* Renovate: unblock pending updates, automerge crates, split lock files; DEP-324 pin subchart images
    * [materialize-monitoring#446](https://github.com/MaterializeInc/materialize-monitoring/pull/446)
    * Every subchart image the chart renders under its shipped profiles is now pinned in `values.yaml` (`loki.loki.image`, `loki.lokiCanary.image`, `loki.memcached.image`, `loki.memcachedExporter.image`, `loki.sidecar.image`, `thanos.global.image`, `grafana-operator.image`, `kube-state-metrics.image`, `metrics-server.image`, `grafana.initChownData.image`). Rendered images are unchanged. These are the subcharts' own value paths, so existing overrides keep applying.
* docs: flatten reference/stable-metrics into reference, and reorder by use
    * [materialize-monitoring#442](https://github.com/MaterializeInc/materialize-monitoring/pull/442)
* dashboards: say object, freshness, and orphaned instead of collection, lag, and leaked
    * [materialize-monitoring#435](https://github.com/MaterializeInc/materialize-monitoring/pull/435)
    * Dashboard panels now use Materialize's product terms: _object_ for collection, _freshness_ for lag, and _orphaned_ for leaked persist data. Several panel titles on `env-top`, `env-upgrade`, `env-persist`, and `env-consensus` changed accordingly; dashboard UIDs and queries did not.
* dashboards: add Infrastructure Autoscaling and Karpenter
    * [materialize-monitoring#425](https://github.com/MaterializeInc/materialize-monitoring/pull/425)
    * Two new dashboards, installed by the default `infra-*` pattern: **Infrastructure Autoscaling** (`infra-autoscaling`), on every cloud, and **Karpenter** (`infra-karpenter`), which has data on EKS where Karpenter's ServiceMonitor is applied.
    * kube-state-metrics now publishes `kube_node_labels` for each provisioner's pool label, the instance type and the zone, set by the new default `kube-state-metrics.metricLabelsAllowlist`. An install that brings its own kube-state-metrics needs the same allowlist for the Node Pools tab.
* Update Rust crate promql-parser to 0.11.0
    * [materialize-monitoring#428](https://github.com/MaterializeInc/materialize-monitoring/pull/428)
    * [`v0.11.0`](https://redirect.github.com/GreptimeTeam/promql-parser/releases/tag/v0.11.0)
* Render LogQL alerts from the query registry and deliver them to the Loki ruler
    * [materialize-monitoring#426](https://github.com/MaterializeInc/materialize-monitoring/pull/426)
    * Bundled log-derived alerts, evaluated by the Loki ruler. `materialize-panic`, `data-correctness-error` and `persist-filter-pushdown-violation` are in the default set. `trace-logging-enabled` installs when selected.
    * Every bundled `PrometheusRule` now carries `mzmon.materialize.cloud/flavor: promql` or `logql`. The alloy-gateway writes `logql` ones into the Loki ruler through its API, including a deployment's own. They install only where the ruler's rule store accepts writes; a `local` store, which a filesystem-only Loki gets, leaves them out with a render warning.
    * `thanos.ruler.autoImportPrometheusRules.labelSelector` defaults to `mzmon.materialize.cloud/flavor!: logql`. A replacement selector has to keep that key, or select `flavor: promql`. A cluster running a Prometheus Operator admission webhook has to exclude `flavor: logql` from it.
    * New `rules.logTenants`: the Loki tenants the log-derived rules are written into. Empty means `pipeline.logging.tenancy.staticTenant`. List them under `byEnvironment` tenancy.
    * The generated rule index moved from `pre-rendered/rules/prometheus/_index.yaml` to `pre-rendered/rules/_index.yaml`, and records each rule's `engine`.
* chore(deps): update rust crate tokio-rustls to v0.26.6
    * [materialize-monitoring#427](https://github.com/MaterializeInc/materialize-monitoring/pull/427)
* DEP-233 Pull instance availability from each cloud's monitoring API
    * [materialize-monitoring#422](https://github.com/MaterializeInc/materialize-monitoring/pull/422)
    * Pull what each cloud publishes about instance availability, off by default:
        * `pipeline.metrics.provider.cloudwatch.eks.clusters` pulls EC2 status checks for every node of the listed EKS clusters, their managed node groups' sizes, and the region's On-Demand vCPU usage. It needs `cloudwatch:GetMetricData`, `cloudwatch:ListMetrics`, `tag:GetResources` and `autoscaling:DescribeAutoScalingGroups`.
        * `pipeline.metrics.provider.gcp.compute.regions` pulls per-family CPU and local-SSD quota, usage against limit, and refusals. `roles/monitoring.viewer` already covers it.
        * `pipeline.metrics.provider.azure.aks.clusters` pulls the AKS cluster autoscaler's gauges and each node VM's availability. It needs Monitoring Reader on each cluster and its node resource group.
        * The new families (`aws_ec2_*`, `aws_autoscaling_*`, `aws_usage_*`, `stackdriver_compute_googleapis_com_location_*`, `azure_microsoft_containerservice_managedclusters_*` and `azure_microsoft_compute_virtualmachinescalesets_*`) take each provider's `metricImportance`.
* dashboards: add Persist, Consensus and Cloud Provider dashboards
    * [materialize-monitoring#419](https://github.com/MaterializeInc/materialize-monitoring/pull/419)
    * Three new dashboards, installed by default: **Materialize Persist (Storage)** (`env-persist`) and **Materialize Consensus (Metadata)** (`env-consensus`), Materialize's own view of object storage and the metadata database, and **Infrastructure Cloud Provider** (`infra-cloud`), which draws the metrics collected by `pipeline.metrics.provider`.
    * **Infrastructure Networking** (`infra-net`) now shows its CNI and Security vendor rows. They were hidden on every cluster, which read as "No Dataplane Metrics".
    * About 50 persist and timestamp-oracle metric families now ship to destinations at `minMetricImportance: recommended`, and the cloud provider families named by `infra-cloud` join the `diagnostic` tier. `pipeline.metrics.provider.*.metricImportance` still decides every tier above `diagnostic`.
* DEP-301 Pull Azure Monitor metrics into the gateway
    * [materialize-monitoring#417](https://github.com/MaterializeInc/materialize-monitoring/pull/417)
    * **New `pipeline.metrics.provider.azure`**, off by default.
        * It pulls a fixed, minimal set of PostgreSQL Flexible Server and Blob Storage metrics from Azure Monitor, for the servers and storage accounts listed under it. Nothing is discovered.
        * Join on `resourceName`. `instance` is `postgres`, `blob_capacity` or `blob_requests`.
        * See [Cloud Provider Metrics](https://materializeinc.github.io/materialize-monitoring/metrics/collecting/cloud-provider-metrics/).
    * **Credentials come from the gateway pod's identity, never from values.** The identity needs Monitoring Reader on each named resource.
        * Workload identity needs both the `azure.workload.identity/client-id` annotation on `alloy-gateway.serviceAccount` and the `azure.workload.identity/use: "true"` label in `alloy-gateway.controller.podLabels`.
        * The Terraform module sets the label whenever the annotation is present.
    * **Azure families default to the `extended` tier**, like the other providers.
* Alert on freshness, hydration and replica health, split by who acts on it
    * [materialize-monitoring#418](https://github.com/MaterializeInc/materialize-monitoring/pull/418)
    * The default set gains ten alerting rules: `environmentd-down`, `environmentd-not-scraped`, `system-cluster-falling-behind`, `system-cluster-stale`, `system-cluster-hydration-stuck`, `system-cluster-memory-near-limit`, `cluster-replica-not-ready`, `cluster-hydration-stuck`, `cluster-memory-near-limit` and `cluster-replica-oomkilled`.
        * `cluster-falling-behind`, `cluster-stale`, `cluster-memory-high`, `cluster-cpu-high` and `source-disconnected` are new and install only when selected.
        * The user-cluster rules install as a third `PrometheusRule`, `<release>-materialize-workload-alerts`.
    * Every bundled alert carries an `audience` label, `platform` or `workload`, for routing with `alerting.routes.extra`. Alerts about a cluster also carry `cluster_name`.
    * `rules.overrides.<alert>` is new: it sets a rule's `for` and adds or replaces its labels. A deployment whose clusters take longer than an hour to hydrate should lengthen `cluster-hydration-stuck`'s `for`.
* Adopt the registry's parameters in the alerts, fix what that exposed, and generate the rules
    * [materialize-monitoring#413](https://github.com/MaterializeInc/materialize-monitoring/pull/413)
* Add an alerting render context, capabilities and gen-rules to the query registry
    * [materialize-monitoring#412](https://github.com/MaterializeInc/materialize-monitoring/pull/412)
* Add an Alloy meta-monitoring dashboard, and fix infra-loki's level picker
    * [materialize-monitoring#397](https://github.com/MaterializeInc/materialize-monitoring/pull/397)
    * **New dashboard: Alloy Meta Monitoring** (`mz-mon-infra-alloy`), in the Meta Observability folder. Collector health, both pipelines hop by hop, ingest over log push, remote write and OTLP, component and configuration state with uptime, gateway clustering, resource limits including `GOMEMLIMIT`, Kubernetes events including the pre-install validation Jobs, and Alloy's own logs.
    * Fixed the Level picker on Loki Meta Monitoring, which offered only "All".
    * Metric tiers now include the Alloy families the new dashboard reads. `loki_source_file_read_bytes_total` leaves the tiers, and the generic Go runtime families it adds are `extended`, so a metered destination on the default tier does not receive every target's copy.
* DEP-301 Pull CloudWatch and Cloud Monitoring metrics into the gateway
    * [materialize-monitoring#396](https://github.com/MaterializeInc/materialize-monitoring/pull/396)
    * **New `pipeline.metrics.provider.cloudwatch` and `pipeline.metrics.provider.gcp`**, both off by default.
        * They pull a fixed, minimal set of RDS and S3 metrics from CloudWatch, and of Cloud SQL and GCS metrics from Cloud Monitoring, for the resources listed under each. Nothing is discovered.
        * On CloudWatch series `instance` is the resource; join on `dimension_DBInstanceIdentifier` or `dimension_BucketName`.
        * See [Cloud Provider Metrics](https://materializeinc.github.io/materialize-monitoring/metrics/collecting/cloud-provider-metrics/).
    * **Credentials come from the gateway pod's identity, never from values.**
        * CloudWatch needs `cloudwatch:GetMetricStatistics`, and `iam:ListAccountAliases` for the `account_alias` label, through IRSA, EKS Pod Identity, or static keys in the `mzmon-alloy-gateway-env` Secret.
        * GCP needs `roles/monitoring.viewer` through Workload Identity.
    * **Outside Google Cloud, a GCP pull needs `GOOGLE_APPLICATION_CREDENTIALS`.** With no credential at all, the exporter cannot start and the gateway fails to load.
    * **Provider families default to the `extended` tier.** They reach the bundled Thanos, and not a destination that filters at `recommended` or `essential`, unless `metricImportance` is raised.
    * **Provider data is minutes old.** Query it with `last_over_time(...[15m])` or wider. Each gateway restart leaves a gap in Cloud Monitoring series as long as that lag.
* Update Rust crate thiserror to v2.0.21
    * [materialize-monitoring#399](https://github.com/MaterializeInc/materialize-monitoring/pull/399)
    * [`v2.0.21`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.21)
* Update Rust crate jsonschema to 0.57.0
    * [materialize-monitoring#390](https://github.com/MaterializeInc/materialize-monitoring/pull/390)
    * [`v0.57.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0570---2026-09-22)
* Fall back to CLUSTER_NAME for the gateway's log cluster label
    * [materialize-monitoring#385](https://github.com/MaterializeInc/materialize-monitoring/pull/385)
* Add a Loki meta-monitoring dashboard, and split dashboards into their own chart
    * [materialize-monitoring#383](https://github.com/MaterializeInc/materialize-monitoring/pull/383)
    * **Dashboards now install from a separate chart.** `materialize-monitoring-dashboards` is a release of its own, installed beside `materialize-monitoring` in the same namespace. The umbrella chart no longer creates dashboards; a release that upgrades without installing the new chart will have its dashboards removed. Helm stores a release in a Kubernetes Secret and a Secret may not exceed 1 MiB, which the rendered set outgrew.
        * Terraform installs it automatically — set `enable_dashboards = false` to opt out.
        * `dashboards.selected` → the new chart's `selected`.
        * `dashboards.config.grafana.manifest.apiTarget` → the new chart's `grafana.apiTarget`.
        * `dashboards.config.datadog` is removed; it drove nothing.
        * The new chart cannot read the umbrella release's values, so `grafana.instanceSelector` and `grafana.folderUids` must match it. The umbrella chart's install notes print the folder UIDs.
        * Dashboard UIDs are unchanged, so saved links, playlists and alerts keep working.
    * **New dashboard: Loki Meta Monitoring** (`mz-mon-infra-loki`), in the Meta Observability folder. Ingest, queries, object storage, retention, and Loki's own logs.
    * **Fixed: the Loki canary and both memcached exporters were never scraped under `profiles/mtls`.** The subchart's single ServiceMonitor applied one `scheme` to every target, including three that only serve plaintext. They are now collected by a separate monitor, and carry `prometheus.io/service-monitor: "false"` plus `monitoring.materialize.cloud/scrape-scheme: plaintext` on their Services. Installs using mTLS gain `loki_canary_*` and `memcached_*` series that were previously absent.
    * New Terraform inputs: `enable_dashboards`, `dashboards_chart_version`, `dashboards_selected`, `dashboards_instance_selector`, `dashboards_allow_cross_namespace_import`.
* Reassemble and classify Rust panics, and parse tracing's plain text format
    * [materialize-monitoring#376](https://github.com/MaterializeInc/materialize-monitoring/pull/376)
    * Rust panics from Materialize services now arrive as a **single log entry** rather than one entry per line of the backtrace, carrying `level=CRITICAL` and `panic_thread` / `panic_location` as structured metadata. The `msg` names the source location and the panic message.
    * `balancerd` and `materialize-operator` logs now carry a parsed `level` and `target`. Both previously landed as `level="UNKNOWN"` for every line.
    * **React to this if you filter or size on log level.** Those services' `WARN` and `ERROR` lines are no longer swept into the `UNKNOWN` rate-limit bucket, which drops, so they now reach Loki reliably and ingested volume from the operator namespace rises. A saved query or alert matching `level="UNKNOWN"` on these services will stop matching.
* DEP-211 Add infra-net dashboard, and the CNI collection it needs
    * [materialize-monitoring#366](https://github.com/MaterializeInc/materialize-monitoring/pull/366)
* Update Rust crate clap to v4.6.7
    * [materialize-monitoring#355](https://github.com/MaterializeInc/materialize-monitoring/pull/355)
    * [`v4.6.7`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.6...clap_complete-v4.6.7)
* Update Rust crate rustls to v0.23.45
    * [materialize-monitoring#349](https://github.com/MaterializeInc/materialize-monitoring/pull/349)
* Update Rust crate jsonschema to 0.56.0
    * [materialize-monitoring#267](https://github.com/MaterializeInc/materialize-monitoring/pull/267)
    * [`v0.56.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0560---2026-09-10)
    * [`v0.55.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0551---2026-09-08)
    * [`v0.55.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0550---2026-09-06)
    * [`v0.54.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0540---2026-09-06)
    * [`v0.53.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0530---2026-09-02)
    * [`v0.52.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0521---2026-08-30)
    * [`v0.52.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0520---2026-08-26)
    * [`v0.51.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0510---2026-08-23)
    * [`v0.50.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0501---2026-08-22)
    * [`v0.50.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0500---2026-08-20)
* Update Rust crate reqwest to v0.13.5
    * [materialize-monitoring#335](https://github.com/MaterializeInc/materialize-monitoring/pull/335)
    * [`v0.13.5`](https://redirect.github.com/seanmonstar/reqwest/blob/HEAD/CHANGELOG.md#v0135)
* Update Rust crate rustls to v0.23.44
    * [materialize-monitoring#332](https://github.com/MaterializeInc/materialize-monitoring/pull/332)
* Update Rust crate tokio-rustls to v0.26.5
    * [materialize-monitoring#326](https://github.com/MaterializeInc/materialize-monitoring/pull/326)
* Add some sample prose in heather's voice
    * [materialize-monitoring#331](https://github.com/MaterializeInc/materialize-monitoring/pull/331)
* Add Dashboard Folders; Update tags
    * [materialize-monitoring#329](https://github.com/MaterializeInc/materialize-monitoring/pull/329)
    * Added several GrafanaFolder resources (default enabled: mzmon-infra, mzmon-materialize, mzmon-meta-o11y)
    * Changed monitoring tag to mzmon within dashboards
* Update Rust crate indexmap to v2.14.2
    * [materialize-monitoring#328](https://github.com/MaterializeInc/materialize-monitoring/pull/328)
    * [`v2.14.2`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2142-2026-09-04)
* Show total lag in env-top / env-upgrade
    * [materialize-monitoring#312](https://github.com/MaterializeInc/materialize-monitoring/pull/312)
    * Add new queries around max lag (materialize.compute.freshness.lag_total_by_cluster, materialize.generations.lag.total)
    * Show max lag as new panels (including per-cluster breakdown) in env-top and env-upgrade dashboards
* DEP-242 Add infra-nodes dashboard
    * [materialize-monitoring#311](https://github.com/MaterializeInc/materialize-monitoring/pull/311)
    * Adds an infra-nodes dashboard that is installed by default
* Update Rust crate indexmap to v2.14.1
    * [materialize-monitoring#309](https://github.com/MaterializeInc/materialize-monitoring/pull/309)
    * [`v2.14.1`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2141-2026-08-28)
* DEP-209 Add Infrastructure Logs & Events Dashboard
    * [materialize-monitoring#307](https://github.com/MaterializeInc/materialize-monitoring/pull/307)
    * Adds new infra-logs dashboard that is enabled by default
* Update Rust crate hyper to v1.11.1
    * [materialize-monitoring#302](https://github.com/MaterializeInc/materialize-monitoring/pull/302)
    * [`v1.11.1`](https://redirect.github.com/hyperium/hyper/blob/HEAD/CHANGELOG.md#v1111-2026-08-27)
* DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
    * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
    * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
* DEP-209 Create a dashboard for Materialize logs and events
    * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
    * Adds the new mz-mon-env-logs dashboard, installed by default
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default

## Dashboards v0.13.0

* DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
    * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
    * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
* DEP-209 Create a dashboard for Materialize logs and events
    * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
    * Adds the new mz-mon-env-logs dashboard, installed by default
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)

## mzmon-lib (shared library) v0.11.0

* DEP-222 Remove the old python implementation for dashboards fully
    * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
* Release Dashboards v0.12.0
    * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)

## materialize-monitoring (Helm chart + Terraform module) v0.20.0

* Update Helm release alloy to v1.12.1
    * [materialize-monitoring#279](https://github.com/MaterializeInc/materialize-monitoring/pull/279)
    * [`v1.12.1`](https://redirect.github.com/grafana/helm-charts/releases/tag/alloy-1.12.1)
    * [`v1.12.0`](https://redirect.github.com/grafana/helm-charts/releases/tag/tempo-1.12.0)
* DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
    * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
    * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default

### Dependencies

* Included Dashboards @ v0.13.0..v0.14.0
    * DEP-209 Create a dashboard for Materialize logs and events
        * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
        * Adds the new mz-mon-env-logs dashboard, installed by default
    * DEP-222 Port Materialize Environment Overview to rust sdk
        * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
        * Dashboards have been rewritten under a different dashboard framework
            * Dashboard queries have adopted queries from our query registry
    * DEP-222 Rust implementation of Grafana Dashboard framework
        * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
* Included Pipelines @ v0.12.0..v0.13.0
    * DEP-241 Remove deprecated alias `k8s_*` labels from logging pipeline; drop pod to metadata only
        * [materialize-monitoring#300](https://github.com/MaterializeInc/materialize-monitoring/pull/300)
        * **Removed:** the `k8s_namespace`, `k8s_app`, and `k8s_container` Loki stream labels, deprecated to consumers in April, 2026. Use `namespace`, `app` and `container` respectively
        * **Removed:** the `k8s_pod` label has been fully removed with expectation to use the `pod` _structured metadata_. Prefer the `namespace` and `app` labels for label filters. Remember that you can always filter by structured metadata like `{app="my-app"} | pod="my-app-pod-12345"`
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
    * Release Dashboards v0.12.0
        * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)

## materialize-monitoring (Helm chart + Terraform module) v0.19.0

* DEP-232 Support multiple prometheus RemoteWrite destinations
    * [materialize-monitoring#272](https://github.com/MaterializeInc/materialize-monitoring/pull/272)
    * terraform: add new variables for tuning prometheus remote_write destinations (as a map)
    * **breaking** helm: move prometheusRemoteEntries destinations down one to a keyed map `pipeline.metrics.gateway.destination.prometheusRemoteWrite` -> `pipeline.metrics.gateway.destination.prometheusRemoteWrite.thanos`
* Ensure auto-format on version-update PRs does not update Chart.lock
    * [materialize-monitoring#262](https://github.com/MaterializeInc/materialize-monitoring/pull/262)
* Update Helm release metrics-server to ^3.14.0
    * [materialize-monitoring#260](https://github.com/MaterializeInc/materialize-monitoring/pull/260)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.11.0..v0.12.0
    * DEP-127 Policy for deprecations and breaking changes
        * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)

## materialize-monitoring (Helm chart + Terraform module) v0.18.1

* Fix snapshots and memcached registry in latest loki
    * [materialize-monitoring#257](https://github.com/MaterializeInc/materialize-monitoring/pull/257)

## Pipelines v0.12.0

* DEP-241 Remove deprecated alias `k8s_*` labels from logging pipeline; drop pod to metadata only
    * [materialize-monitoring#300](https://github.com/MaterializeInc/materialize-monitoring/pull/300)
    * **Removed:** the `k8s_namespace`, `k8s_app`, and `k8s_container` Loki stream labels, deprecated to consumers in April, 2026. Use `namespace`, `app` and `container` respectively
    * **Removed:** the `k8s_pod` label has been fully removed with expectation to use the `pod` _structured metadata_. Prefer the `namespace` and `app` labels for label filters. Remember that you can always filter by structured metadata like `{app="my-app"} | pod="my-app-pod-12345"`
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default
* DEP-232 Support multiple prometheus RemoteWrite destinations
    * [materialize-monitoring#272](https://github.com/MaterializeInc/materialize-monitoring/pull/272)
    * terraform: add new variables for tuning prometheus remote_write destinations (as a map)
    * **breaking** helm: move prometheusRemoteEntries destinations down one to a keyed map `pipeline.metrics.gateway.destination.prometheusRemoteWrite` -> `pipeline.metrics.gateway.destination.prometheusRemoteWrite.thanos`
* DEP-127 Policy for deprecations and breaking changes
    * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
        * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
        * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
    * DEP-209 Create a dashboard for Materialize logs and events
        * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
        * Adds the new mz-mon-env-logs dashboard, installed by default
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
    * Release Dashboards v0.12.0
        * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Port Materialize Environment Overview to rust sdk
        * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
        * Dashboards have been rewritten under a different dashboard framework
            * Dashboard queries have adopted queries from our query registry
    * DEP-222 Rust implementation of Grafana Dashboard framework
        * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)

## materialize-monitoring (Helm chart + Terraform module) v0.18.0

* DEP-195 Implement TLS across stack
    * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
* DEP-192 Implement networkpolicies across applications
    * [materialize-monitoring#252](https://github.com/MaterializeInc/materialize-monitoring/pull/252)
* Update docker.io/grafana/grafana Docker tag to v13.2.0
    * [materialize-monitoring#253](https://github.com/MaterializeInc/materialize-monitoring/pull/253)
* Update astral-sh/setup-uv action to v10
    * [materialize-monitoring#240](https://github.com/MaterializeInc/materialize-monitoring/pull/240)
* Provide Datadog queries in documentation
    * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.11.0..v0.12.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)

## materialize-monitoring (Helm chart + Terraform module) v0.17.0

* DEP-204 Expose OTLP and Datadog configs to Terraform
    * [materialize-monitoring#247](https://github.com/MaterializeInc/materialize-monitoring/pull/247)

## Container Images v0.4.0

* Update dependency grafana/alloy to v1.19.2
    * [materialize-monitoring#278](https://github.com/MaterializeInc/materialize-monitoring/pull/278)
    * [`v1.19.2`](https://redirect.github.com/grafana/alloy/releases/tag/v1.19.2)
    * [`v1.19.0`](https://redirect.github.com/grafana/alloy/releases/tag/v1.19.0)
* Update debian:13 Docker digest to f324c7f
    * [materialize-monitoring#287](https://github.com/MaterializeInc/materialize-monitoring/pull/287)

## materialize-monitoring (Helm chart + Terraform module) v0.16.2

* DEP-231 Document and provide examples for DHI and Chainguard images
    * [materialize-monitoring#245](https://github.com/MaterializeInc/materialize-monitoring/pull/245)
* DEP-195 Proposal for TLS authentication
    * [materialize-monitoring#244](https://github.com/MaterializeInc/materialize-monitoring/pull/244)
* DEP-203 DEP-185 Run E2E tests against Tier 2 Terraform; support static s3 creds
    * [materialize-monitoring#241](https://github.com/MaterializeInc/materialize-monitoring/pull/241)
* DEP-230 Fix node logs on bottlerocket
    * [materialize-monitoring#239](https://github.com/MaterializeInc/materialize-monitoring/pull/239)
* DEP-185 Add an E2E test suite
    * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
* DEP-230 Fixes for collecting node logs (journald)
    * [materialize-monitoring#234](https://github.com/MaterializeInc/materialize-monitoring/pull/234)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.10.0..v0.11.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0

## materialize-monitoring (Helm chart + Terraform module) v0.16.1

* DEP-227 DEP-197 Fix s3 endpoint; agent tolerations; relax gateway HPA; add pre-delete for GFx resources
    * [materialize-monitoring#231](https://github.com/MaterializeInc/materialize-monitoring/pull/231)

## Pipelines v0.11.0

* DEP-195 Implement TLS across stack
    * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)

### Dependencies

* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
    * Provide Datadog queries in documentation
        * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
    * DEP-185 Add an E2E test suite
        * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)

## Container Images v0.3.0

* DEP-230 Fixes for collecting node logs (journald)
    * [materialize-monitoring#234](https://github.com/MaterializeInc/materialize-monitoring/pull/234)
* Update debian:13 Docker digest to 34cd9e9
    * [materialize-monitoring#212](https://github.com/MaterializeInc/materialize-monitoring/pull/212)

## materialize-monitoring (Helm chart + Terraform module) v0.16.0

* Update loki helm chart to v18.8.0
    * [materialize-monitoring#225](https://github.com/MaterializeInc/materialize-monitoring/pull/225)
* Update ghcr.io/materializeinc/mzmon-alloy Docker tag to v1.18.1
    * [materialize-monitoring#224](https://github.com/MaterializeInc/materialize-monitoring/pull/224)
* Ensure thanos fits onto default self-managed nodes
    * [materialize-monitoring#223](https://github.com/MaterializeInc/materialize-monitoring/pull/223)
* DEP-187 Scrape cadvisor from kubelet instead of via daemonset
    * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
* Update docker.io/grafana/grafana Docker tag to v13.1.3
    * [materialize-monitoring#221](https://github.com/MaterializeInc/materialize-monitoring/pull/221)
* DEP-190 Provide separate sizing profiles for thanos
    * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
* Update docker.io/grafana/grafana Docker tag to v13.1.2
    * [materialize-monitoring#208](https://github.com/MaterializeInc/materialize-monitoring/pull/208)
* Convert raw blocks into structured configs
    * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.10.0..v0.11.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)

## Pipelines v0.10.0

* DEP-187 Scrape cadvisor from kubelet instead of via daemonset
    * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
* Convert raw blocks into structured configs
    * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)

### Dependencies

* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * DEP-190 Provide separate sizing profiles for thanos
        * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)

## materialize-monitoring (Helm chart + Terraform module) v0.15.0

* CLO-180 Support sidechannel logging path (agent doesn't read its own logs)
    * [materialize-monitoring#202](https://github.com/MaterializeInc/materialize-monitoring/pull/202)
* DEP-187 Collect cAdvisor metrics with Alloy
    * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
* Upgrade all subcharts to latest version (Loki 15->18, etc)
    * [materialize-monitoring#198](https://github.com/MaterializeInc/materialize-monitoring/pull/198)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.9.0..v0.10.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0

## mzmon-lib (shared library) v0.10.0

* DEP-222 Port Materialize Environment Overview to rust sdk
    * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
    * Dashboards have been rewritten under a different dashboard framework
        * Dashboard queries have adopted queries from our query registry
* DEP-222 Rust implementation of Grafana Dashboard framework
    * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
* DEP-222 Add generated grafana models
    * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
* DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
    * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
* DEP-127 Policy for deprecations and breaking changes
    * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
* DEP-237 Support additional release notes in changelogs
    * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)
* Update Rust crate rustls-pki-types to v1.15.1
    * [materialize-monitoring#256](https://github.com/MaterializeInc/materialize-monitoring/pull/256)
* DEP-195 Implement TLS across stack
    * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
* Update Rust crate jsonschema to v0.49.9
    * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
    * [`v0.49.9`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0499---2026-08-09)
    * [`v0.49.8`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0498---2026-08-08)
    * [`v0.49.7`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0497---2026-08-07)
    * [`v0.49.6`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0496---2026-08-06)
    * [`v0.49.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0495---2026-08-05)
* Provide Datadog queries in documentation
    * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
* DEP-185 Add an E2E test suite
    * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
* Update Rust crate thiserror to v2.0.20
    * [materialize-monitoring#229](https://github.com/MaterializeInc/materialize-monitoring/pull/229)
    * [`v2.0.20`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.20)
* DEP-187 Scrape cadvisor from kubelet instead of via daemonset
    * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
* Update Rust crate clap to v4.6.6
    * [materialize-monitoring#219](https://github.com/MaterializeInc/materialize-monitoring/pull/219)
    * [`v4.6.6`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#466---2026-08-06)
* DEP-190 Provide separate sizing profiles for thanos
    * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
* Update Rust crate jsonschema to v0.49.4
    * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)
    * [`v0.49.4`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0494---2026-08-04)
* Convert raw blocks into structured configs
    * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)
* DEP-187 Collect cAdvisor metrics with Alloy
    * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)

## materialize-monitoring (Helm chart + Terraform module) v0.14.0

* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.8.0..v0.9.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)

## materialize-monitoring (Helm chart + Terraform module) v0.13.1

* CLO-111 Remaining fixes for grafana LB/persistence
    * [materialize-monitoring#194](https://github.com/MaterializeInc/materialize-monitoring/pull/194)

## materialize-monitoring (Helm chart + Terraform module) v0.13.0

* Update docker.io/grafana/grafana Docker tag to v13.1.1
    * [materialize-monitoring#193](https://github.com/MaterializeInc/materialize-monitoring/pull/193)
* CLO-111 DEP-202 DEP-196 Production defaults for Grafana
    * [materialize-monitoring#192](https://github.com/MaterializeInc/materialize-monitoring/pull/192)
* More documentation cleanups
    * [materialize-monitoring#187](https://github.com/MaterializeInc/materialize-monitoring/pull/187)

## materialize-monitoring (Helm chart + Terraform module) v0.12.0

* DEP-184 Add Azure Profiles and TF Module Support
    * [materialize-monitoring#183](https://github.com/MaterializeInc/materialize-monitoring/pull/183)

## materialize-monitoring (Helm chart + Terraform module) v0.11.1

* DEP-186 Add an E2E suite for kind + helm/tf
    * [materialize-monitoring#181](https://github.com/MaterializeInc/materialize-monitoring/pull/181)
* DEP-123 Support GCM and handle alloy rolls
    * [materialize-monitoring#179](https://github.com/MaterializeInc/materialize-monitoring/pull/179)

## materialize-monitoring (Helm chart + Terraform module) v0.11.0

* DEP-182 Remaining fixes to get GCP TF working
    * [materialize-monitoring#177](https://github.com/MaterializeInc/materialize-monitoring/pull/177)

## materialize-monitoring (Helm chart + Terraform module) v0.10.0

* DEP-182 materialize-monitoring Terraform Module
    * [materialize-monitoring#171](https://github.com/MaterializeInc/materialize-monitoring/pull/171)

## materialize-monitoring (Helm chart + Terraform module) v0.9.0

* DEP-191 Improve validation for thanos and alloy
    * [materialize-monitoring#170](https://github.com/MaterializeInc/materialize-monitoring/pull/170)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.8.0..v0.9.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)

## materialize-monitoring Optional CRDs v0.4.0 (Unreleased)

_Changes Pending_

## materialize-monitoring Helm Chart v0.8.0

* CLO-111 Grafana Documentation with datasources and bundled resources
    * [materialize-monitoring#164](https://github.com/MaterializeInc/materialize-monitoring/pull/164)
* Move grafana-operator CRDs into the materialize-monitoring-crds chart
    * [materialize-monitoring#161](https://github.com/MaterializeInc/materialize-monitoring/pull/161)
* CLO-153 Add profile example for otel pipelines
    * [materialize-monitoring#159](https://github.com/MaterializeInc/materialize-monitoring/pull/159)

## Pipelines v0.9.0

* CLO-180 Support sidechannel logging path (agent doesn't read its own logs)
    * [materialize-monitoring#202](https://github.com/MaterializeInc/materialize-monitoring/pull/202)
* DEP-187 Collect cAdvisor metrics with Alloy
    * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)

### Dependencies

* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)

## Pipelines v0.8.0

* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Implement a Query Registry for reducing total metric set
        * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)

## Prometheus Scrapers v0.4.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
        * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
        * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
    * DEP-209 Create a dashboard for Materialize logs and events
        * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
        * Adds the new mz-mon-env-logs dashboard, installed by default
    * DEP-210 Upgrade visibility dashboard
        * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
        * Adds a new mz-env-upgrade dashboard, enabled by default
            * Requires materialize v26.41.0
        * Enable sql scrapers by default
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
    * Release Dashboards v0.12.0
        * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Port Materialize Environment Overview to rust sdk
        * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
        * Dashboards have been rewritten under a different dashboard framework
            * Dashboard queries have adopted queries from our query registry
    * DEP-222 Rust implementation of Grafana Dashboard framework
        * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
    * DEP-127 Policy for deprecations and breaking changes
        * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)
    * Update Rust crate rustls-pki-types to v1.15.1
        * [materialize-monitoring#256](https://github.com/MaterializeInc/materialize-monitoring/pull/256)
    * DEP-195 Implement TLS across stack
        * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
        * [`v0.49.9`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0499---2026-08-09)
        * [`v0.49.8`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0498---2026-08-08)
        * [`v0.49.7`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0497---2026-08-07)
        * [`v0.49.6`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0496---2026-08-06)
        * [`v0.49.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0495---2026-08-05)
    * Provide Datadog queries in documentation
        * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
    * DEP-185 Add an E2E test suite
        * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
    * Update Rust crate thiserror to v2.0.20
        * [materialize-monitoring#229](https://github.com/MaterializeInc/materialize-monitoring/pull/229)
        * [`v2.0.20`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.20)
    * DEP-187 Scrape cadvisor from kubelet instead of via daemonset
        * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
    * Update Rust crate clap to v4.6.6
        * [materialize-monitoring#219](https://github.com/MaterializeInc/materialize-monitoring/pull/219)
        * [`v4.6.6`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#466---2026-08-06)
    * DEP-190 Provide separate sizing profiles for thanos
        * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)
        * [`v0.49.4`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0494---2026-08-04)
    * Convert raw blocks into structured configs
        * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)
    * DEP-187 Collect cAdvisor metrics with Alloy
        * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)
    * DEP-188 Add node-exporter; support priorityClasses
        * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)
    * Update Rust crate clap to v4.6.5
        * [materialize-monitoring#190](https://github.com/MaterializeInc/materialize-monitoring/pull/190)
        * [`v4.6.5`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.4...clap_complete-v4.6.5)
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
        * [`v0.12.0`](https://redirect.github.com/astral-sh/uv/blob/HEAD/CHANGELOG.md#0120)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
        * [`v0.49.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0492---2026-07-28)
        * [`v0.49.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0491---2026-07-25)
        * [`v0.49.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0490---2026-07-25)
        * [`v0.48.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0485---2026-07-22)
        * [`v0.48.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0482---2026-07-21)
        * [`v0.48.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0481---2026-07-17)
        * [`v0.48.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0480---2026-07-16)
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
        * [`v0.3.4`](https://redirect.github.com/rust-lang/glob/blob/HEAD/CHANGELOG.md#034---2026-07-21)
    * Update Rust crate tokio to v1.53.1
        * [materialize-monitoring#147](https://github.com/MaterializeInc/materialize-monitoring/pull/147)
        * [`v1.53.1`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.1): Tokio v1.53.1
    * Update Rust crate clap to v4.6.4
        * [materialize-monitoring#146](https://github.com/MaterializeInc/materialize-monitoring/pull/146)
        * [`v4.6.4`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#464---2026-07-21)
        * [`v4.6.3`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#463---2026-07-20)
    * CLO-152 Support splitting metrics into tiers
        * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
    * Update Rust crate thiserror to v2.0.19
        * [materialize-monitoring#143](https://github.com/MaterializeInc/materialize-monitoring/pull/143)
        * [`v2.0.19`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.19)
    * Update Rust crate tokio to v1.53.0
        * [materialize-monitoring#139](https://github.com/MaterializeInc/materialize-monitoring/pull/139)
        * [`v1.53.0`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.0): Tokio v1.53.0
    * Update Rust crate serde to v1.0.229
        * [materialize-monitoring#142](https://github.com/MaterializeInc/materialize-monitoring/pull/142)
        * [`v1.0.229`](https://redirect.github.com/serde-rs/serde/releases/tag/v1.0.229)
    * Update Rust crate anyhow to v1.0.104
        * [materialize-monitoring#141](https://github.com/MaterializeInc/materialize-monitoring/pull/141)
        * [`v1.0.104`](https://redirect.github.com/dtolnay/anyhow/releases/tag/1.0.104)
    * Update Rust crate serde_json to v1.0.151
        * [materialize-monitoring#144](https://github.com/MaterializeInc/materialize-monitoring/pull/144)
        * [`v1.0.151`](https://redirect.github.com/serde-rs/json/releases/tag/v1.0.151)
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
        * [`v1.13.1`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1131-2026-07-15)
        * [`v1.13.0`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1130-2026-07-09)
        * [`v1.12.4`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1124-2025-06-09)
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Implement a Query Registry for reducing total metric set
        * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)
        * [`v4.6.2`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#462---2026-07-15)

## materialize-monitoring Helm Chart v0.7.0

* CLO-153 Metric storage documentation; otlp fixes
    * [materialize-monitoring#155](https://github.com/MaterializeInc/materialize-monitoring/pull/155)
* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
* Drop renovate on legacy/; tune helm updates
    * [materialize-monitoring#135](https://github.com/MaterializeInc/materialize-monitoring/pull/135)
* Implement a Query Registry for reducing total metric set
    * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 Replace prometheus-style pipeline with otelcol for processing
    * [materialize-monitoring#115](https://github.com/MaterializeInc/materialize-monitoring/pull/115)
* CLO-152 add schema support for otelcol pipeline blocks
    * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
* Included Pipelines @ v0.8.0..v0.9.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)

## materialize-monitoring Helm Chart v0.6.0

* Allow configuring otlpExporter, googleCloudExporter, datadogExporter
    * [materialize-monitoring#106](https://github.com/MaterializeInc/materialize-monitoring/pull/106)

## materialize-monitoring Helm Chart v0.5.0

* CLO-112 Harden Long-Term storage in GCP
    * [materialize-monitoring#103](https://github.com/MaterializeInc/materialize-monitoring/pull/103)

## Pipelines v0.7.0

* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 Replace prometheus-style pipeline with otelcol for processing
    * [materialize-monitoring#115](https://github.com/MaterializeInc/materialize-monitoring/pull/115)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

## mzmon-lib (shared library) v0.9.0

* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)
* Update dependency uv_build to >=0.12,<0.13
    * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
* Update Rust crate jsonschema to 0.49.0
    * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
* Update Rust crate glob to v0.3.4
    * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
* Update Rust crate regex to v1.13.1
    * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
* CLO-152 Support importance axis for extracted metrics
    * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
* Update Rust crate tokio to v1.52.4
    * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
* Port metric registry to rust
    * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
* Implement a Query Registry for reducing total metric set
    * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
* Update Rust crate clap to v4.6.2
    * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)
* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 add schema support for otelcol pipeline blocks
    * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

## Pipelines v0.6.0

* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)

## Pipelines v0.5.0

* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
* Split loki.write out of main processing pipeline
    * [materialize-monitoring#81](https://github.com/MaterializeInc/materialize-monitoring/pull/81)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0

## Prometheus Scrapers v0.3.0

* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Enable alloy pipelines in materialize-monitoring
        * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)

## Container Images v0.2.0

* Update dependency grafana/alloy to v1.18.1
    * [materialize-monitoring#218](https://github.com/MaterializeInc/materialize-monitoring/pull/218)
* Update dependency grafana/alloy to v1.18.0
    * [materialize-monitoring#145](https://github.com/MaterializeInc/materialize-monitoring/pull/145)
* Update gcr.io/distroless/base-debian13 Docker digest to f4a335c
    * [materialize-monitoring#102](https://github.com/MaterializeInc/materialize-monitoring/pull/102)
* Update gcr.io/distroless/base-debian13 Docker digest to 7c4468d
    * [materialize-monitoring#85](https://github.com/MaterializeInc/materialize-monitoring/pull/85)
* Update dependency grafana/alloy to v1.17.1
    * [materialize-monitoring#77](https://github.com/MaterializeInc/materialize-monitoring/pull/77)

## mzmon-lib (shared library) v0.8.0

* Update Rust crate jsonschema to 0.47.0
    * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
* Update Rust crate jsonschema to v0.46.10
    * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
* Implement Gateway Pipeline for Logs
    * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
* Update dependency grafana-foundation-sdk to v0.0.18
    * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
* Update Rust crate reqwest to 0.13
    * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)

## materialize-monitoring Helm Chart v0.4.0

* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
* Pin ghcr.io/materializeinc/mzmon-alloy Docker tag to c47e937
    * [materialize-monitoring#90](https://github.com/MaterializeInc/materialize-monitoring/pull/90)
* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
* Included Pipelines @ v0.6.0..v0.7.0
    * Split loki.write out of main processing pipeline
        * [materialize-monitoring#81](https://github.com/MaterializeInc/materialize-monitoring/pull/81)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
* Included Prometheus Scrapers @ v0.2.0..v0.3.0
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)

## Dashboards v0.12.0

* DEP-222 Port Materialize Environment Overview to rust sdk
    * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
    * Dashboards have been rewritten under a different dashboard framework
        * Dashboard queries have adopted queries from our query registry
* DEP-222 Rust implementation of Grafana Dashboard framework
    * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
* Provide Datadog queries in documentation
    * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)
* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
* CLO-152 Support importance axis for extracted metrics
    * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
* Implement a Query Registry for reducing total metric set
    * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
* Update dependency grafana-foundation-sdk to v0.0.18
    * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * [`v0.0.18`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.17...v0.0.18)
    * [`v0.0.17`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.16...v0.0.17)
    * [`v0.0.16`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.15...v0.0.16)
    * [`v0.0.15`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.13...v0.0.15)
    * [`v0.0.13`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.12...v0.0.13)
* Implement Loki with Production Configuration
    * [materialize-monitoring#48](https://github.com/MaterializeInc/materialize-monitoring/pull/48)
* Add annotations to distinguish dashboards; roadmapping
    * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)

### Dependencies

* Included mzmon-lib (shared library) @ v0.10.0..v0.11.0
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
    * DEP-127 Policy for deprecations and breaking changes
        * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)
    * Update Rust crate rustls-pki-types to v1.15.1
        * [materialize-monitoring#256](https://github.com/MaterializeInc/materialize-monitoring/pull/256)
    * DEP-195 Implement TLS across stack
        * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
        * [`v0.49.9`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0499---2026-08-09)
        * [`v0.49.8`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0498---2026-08-08)
        * [`v0.49.7`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0497---2026-08-07)
        * [`v0.49.6`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0496---2026-08-06)
        * [`v0.49.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0495---2026-08-05)
    * DEP-185 Add an E2E test suite
        * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
    * Update Rust crate thiserror to v2.0.20
        * [materialize-monitoring#229](https://github.com/MaterializeInc/materialize-monitoring/pull/229)
        * [`v2.0.20`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.20)
    * DEP-187 Scrape cadvisor from kubelet instead of via daemonset
        * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
    * Update Rust crate clap to v4.6.6
        * [materialize-monitoring#219](https://github.com/MaterializeInc/materialize-monitoring/pull/219)
        * [`v4.6.6`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#466---2026-08-06)
    * DEP-190 Provide separate sizing profiles for thanos
        * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)
        * [`v0.49.4`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0494---2026-08-04)
    * Convert raw blocks into structured configs
        * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)
    * DEP-187 Collect cAdvisor metrics with Alloy
        * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)
    * Update Rust crate clap to v4.6.5
        * [materialize-monitoring#190](https://github.com/MaterializeInc/materialize-monitoring/pull/190)
        * [`v4.6.5`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.4...clap_complete-v4.6.5)
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
        * [`v0.12.0`](https://redirect.github.com/astral-sh/uv/blob/HEAD/CHANGELOG.md#0120)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
        * [`v0.49.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0492---2026-07-28)
        * [`v0.49.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0491---2026-07-25)
        * [`v0.49.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0490---2026-07-25)
        * [`v0.48.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0485---2026-07-22)
        * [`v0.48.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0482---2026-07-21)
        * [`v0.48.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0481---2026-07-17)
        * [`v0.48.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0480---2026-07-16)
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
        * [`v0.3.4`](https://redirect.github.com/rust-lang/glob/blob/HEAD/CHANGELOG.md#034---2026-07-21)
    * Update Rust crate tokio to v1.53.1
        * [materialize-monitoring#147](https://github.com/MaterializeInc/materialize-monitoring/pull/147)
        * [`v1.53.1`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.1): Tokio v1.53.1
    * Update Rust crate clap to v4.6.4
        * [materialize-monitoring#146](https://github.com/MaterializeInc/materialize-monitoring/pull/146)
        * [`v4.6.4`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#464---2026-07-21)
        * [`v4.6.3`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#463---2026-07-20)
    * Update Rust crate thiserror to v2.0.19
        * [materialize-monitoring#143](https://github.com/MaterializeInc/materialize-monitoring/pull/143)
        * [`v2.0.19`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.19)
    * Update Rust crate tokio to v1.53.0
        * [materialize-monitoring#139](https://github.com/MaterializeInc/materialize-monitoring/pull/139)
        * [`v1.53.0`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.0): Tokio v1.53.0
    * Update Rust crate serde to v1.0.229
        * [materialize-monitoring#142](https://github.com/MaterializeInc/materialize-monitoring/pull/142)
        * [`v1.0.229`](https://redirect.github.com/serde-rs/serde/releases/tag/v1.0.229)
    * Update Rust crate anyhow to v1.0.104
        * [materialize-monitoring#141](https://github.com/MaterializeInc/materialize-monitoring/pull/141)
        * [`v1.0.104`](https://redirect.github.com/dtolnay/anyhow/releases/tag/1.0.104)
    * Update Rust crate serde_json to v1.0.151
        * [materialize-monitoring#144](https://github.com/MaterializeInc/materialize-monitoring/pull/144)
        * [`v1.0.151`](https://redirect.github.com/serde-rs/json/releases/tag/v1.0.151)
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
        * [`v1.13.1`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1131-2026-07-15)
        * [`v1.13.0`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1130-2026-07-09)
        * [`v1.12.4`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1124-2025-06-09)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)
        * [`v4.6.2`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#462---2026-07-15)
    * Enable MZ podmonitors by default
        * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
        * [`v0.47.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0470---2026-07-08)
    * Implement alloy metrics pipelines
        * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Enable alloy pipelines in materialize-monitoring
        * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
        * [`v2.14.2`](https://redirect.github.com/pydantic/pydantic-settings/compare/v2.14.1...v2.14.2)
        * [`v2.14.1`](https://redirect.github.com/pydantic/pydantic-settings/releases/tag/v2.14.1)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)

## Dashboards v0.11.0

* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0

## Prometheus Scrapers v0.2.0

* MaterializeInc/jun/add-auth-to-compute-sql-endpoint
    * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)

## materialize-monitoring Optional CRDs v0.3.0

* Move grafana-operator CRDs into the materialize-monitoring-crds chart
    * [materialize-monitoring#161](https://github.com/MaterializeInc/materialize-monitoring/pull/161)

## Container Images v0.1.1

* Update debian Docker tag to trixie-20260623
    * [materialize-monitoring#73](https://github.com/MaterializeInc/materialize-monitoring/pull/73)
* Create distroless images for alloy
    * [materialize-monitoring#72](https://github.com/MaterializeInc/materialize-monitoring/pull/72)

## Container Images v0.1.0

* Bootstrapped

## Dashboards v0.10.0

* Improvements to better support GCP/GKE/GMP Dashboards/Datasources
    * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Expose classic scrapeconfigs
        * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)

## Prometheus Scrapers v0.1.1

* Attach explicit pod labels to scrapers in GCP
    * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Add PodMonitors for prometheus.operator
    * [materialize-monitoring#31](https://github.com/MaterializeInc/materialize-monitoring/pull/31)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
    * Monitoring Roadmap and Version/Changelog Management
        * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## Prometheus Scrapers v0.1.0

* Bootstrapped

## Dashboards v0.9.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)

## mzmon-lib (shared library) v0.7.0

* Update dependency pydantic-settings to v2.14.2 [SECURITY]
    * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
* Update python Docker tag to v3.14
    * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
* Update Rust crate jsonschema to v0.46.9
    * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
* Update Rust crate anyhow to v1.0.103
    * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
* Update Rust crate itertools to 0.15.0
    * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
* MaterializeInc/jun/add-auth-to-compute-sql-endpoint
    * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
* Add annotations to distinguish dashboards; roadmapping
    * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
* Release Dashboards v0.11.0
    * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
* Release Dashboards v0.10.0
    * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
* Update PR description on bump updates
    * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
* Improvements to better support GCP/GKE/GMP Dashboards/Datasources
    * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
* Attach explicit pod labels to scrapers in GCP
    * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
* Generate PodMonitoring resources for GCP
    * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
* Upgrade to rust 1.96
    * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Release Dashboards v0.9.0
    * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
* Only upload artifacts while in a draft state
    * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
* Release Dashboards v0.8.0
    * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)

## Pipelines v0.4.0

* Implement Gateway Pipeline for Logs
    * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Expose classic scrapeconfigs
        * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)

## materialize-monitoring Helm Chart v0.3.0

* Implement Loki with Production Configuration
    * [materialize-monitoring#48](https://github.com/MaterializeInc/materialize-monitoring/pull/48)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Release materialize-monitoring Helm Chart v0.3.0
    * [materialize-monitoring#17](https://github.com/MaterializeInc/materialize-monitoring/pull/17)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Use global_id to not run into errors on right join
        * [materialize-monitoring#24](https://github.com/MaterializeInc/materialize-monitoring/pull/24)
    * Coalesce object names into dashboards
        * [materialize-monitoring#23](https://github.com/MaterializeInc/materialize-monitoring/pull/23)
* Included Pipelines @ v0.3.0..v0.4.0
* Included Prometheus Scrapers @ v0.1.1..v0.2.0
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Add PodMonitors for prometheus.operator
        * [materialize-monitoring#31](https://github.com/MaterializeInc/materialize-monitoring/pull/31)
* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)

## materialize-monitoring Optional CRDs v0.2.0

* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)

## Dashboards v0.8.0

* Use global_id to not run into errors on right join
    * [materialize-monitoring#24](https://github.com/MaterializeInc/materialize-monitoring/pull/24)
* Coalesce object names into dashboards
    * [materialize-monitoring#23](https://github.com/MaterializeInc/materialize-monitoring/pull/23)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)

## Pipelines v0.3.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.5.0..v0.6.0
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
    * Monitoring Roadmap and Version/Changelog Management
        * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## mzmon-lib (shared library) v0.6.0

* Include artifacts when creating github releases
    * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
* Support generating a release when version bump PRs are merged
    * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
* Support auto-formatting based on labels
    * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
* Generated automated versioning PRs
    * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## materialize-monitoring Helm Chart v0.2.0

### Dependencies

* Included Dashboards @ v0.6.0..v0.7.0
    * Fix cloud compatibility with Environment Monitoring dashboards
        * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
    * Update for self-managed workloads
        * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)
* Included Pipelines @ v0.1.0..v0.2.0
    * Generate agent logging pipeline
        * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
    * Alloy Pipeline Generation
        * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)
* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)

## Dashboards v0.7.0

* Fix cloud compatibility with Environment Monitoring dashboards
    * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
* Update for self-managed workloads
    * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)

### Dependencies

* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Generate agent logging pipeline
        * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
    * Alloy Pipeline Generation
        * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## Pipelines v0.2.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

### Dependencies

* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)

## mzmon-lib (shared library) v0.5.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Implement capsules and targets for alloy pipelines
    * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## Dashboards v0.6.0

* Fix cloud compatibility with Environment Monitoring dashboards
    * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
* Update for self-managed workloads
    * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)

## Pipelines v0.1.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)

## mzmon-lib (shared library) v0.4.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Implement capsules and targets for alloy pipelines
    * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## materialize-monitoring Helm Chart v0.1.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)
* Provide helm reference documentation for materialize-monitoring
    * [materialize-monitoring#9](https://github.com/MaterializeInc/materialize-monitoring/pull/9)
* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)
* Update contributor documentation around dashboards
    * [materialize-monitoring#5](https://github.com/MaterializeInc/materialize-monitoring/pull/5)

## materialize-monitoring Optional CRDs v0.1.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)

## mzmon-lib (shared library) v0.3.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)
* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)

## Dashboards v0.5.0

* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)
* Update contributor documentation around dashboards
    * [materialize-monitoring#5](https://github.com/MaterializeInc/materialize-monitoring/pull/5)
