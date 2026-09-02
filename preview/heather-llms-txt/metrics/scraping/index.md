# Scraping




# Scraping Metrics with `materialize-monitoring`

By default, `materialize-monitoring` is configured to scrape metrics from any ServiceMonitor or PodMonitor resources in the cluster.
This allows you to easily add new metrics to your monitoring stack by simply creating a new ServiceMonitor resources.

## Scrape Architecture with Grafana Alloy

`materialize-monitoring` runs Grafana Alloy with `prometheus.operator` components on `alloy-gateway` instances (Deployment) which read ServiceMonitors and PodMonitors in order to determine what targets to scrape.

`alloy-gateway` runs in [clustering mode](https://grafana.com/docs/alloy/latest/get-started/clustering/) by default, which means that
scraping is distributed across all replicas of `alloy-gateway` and the scrape state is shared between them.

For what each monitor kind is and how they differ, see [Custom Resource Definitions](../../reference/crds/#servicemonitor-vs-podmonitor).

### Container metrics from the kubelet {#kubelet}

One target is not discovered from a monitor resource: `alloy-gateway` scrapes `/metrics/cadvisor` on **every kubelet** directly, on by default.

The kubelet already computes these statistics, so running an in-process cAdvisor on each agent would compute them twice — measured at roughly 750Mi per agent against a 200Mi logs-only envelope.
Scraping the kubelet removes that cost and puts the remaining scrape cost on the gateway, where it is shared across replicas rather than reserved on every node.

Coverage does not suffer for it: a GKE kubelet serves 69 distinct `container_*` metrics against the 70 an in-process cAdvisor produced, and every `container_*` metric this chart's queries reference is present.

| Value | Default | Notes |
|---|---|---|
| `pipeline.metrics.kubelet.scrapeInterval` | `60s` | The dominant cost lever — roughly 6.7k series per node per scrape |
| `pipeline.metrics.kubelet.tlsInsecureSkipVerify` | `false` | On GKE and EKS the kubelet certificate verifies against the in-cluster CA, which the chart passes as `ca_file` |

> [!WARNING]
>   A distribution that signs kubelet certificates with a CA the pods do not trust **fails this scrape quietly** — container metrics simply stop, and nothing errors at install.
>   `kind` is the known case. Check `up{job="cadvisor"}` when bringing up a new distribution, and set `tlsInsecureSkipVerify: true` only where you must.

## Manually Configured Scraping

If you are not using default `materialize-monitoring` setup, you can use the following
scrape configuration files as a starting point for your own Prometheus setup.

### Authenticating the SQL metrics endpoint

The `materialize-sql` scrapers collect SQL-derived metrics from the environmentd `/metrics/mz_compute`, `/metrics/mz_frontier`, `/metrics/mz_storage`, and `/metrics/mz_usage` endpoints. Scrape it as the built-in `mz_support` role.

The **Classic** and **Google Cloud Managed Prometheus** configs carry `username: mz_support` inline, so they need no extra setup.
Prometheus Operator `basicAuth` can only reference a Kubernetes Secret — it has no inline fields — so the `materialize-sql` `PodMonitor` reads its credentials from a Secret named `materialize-sql-monitor`.
It needs a `username` key (`mz_support`) and an **empty** `password` key.

The empty password is not a placeholder to fill in later.
**This endpoint does not validate passwords**: the Basic-auth username selects the role the underlying queries run as — which scopes what those queries can see — and the password field is never read.
It has to be present at all only because alloy's `prometheus.operator.*` scrapeconfig generation rejects an absent password reference with `resource name may not be empty`.
An empty value produces the same `mz_support:` Basic header the username-only configs already send.
See [Securing](../../operating/securing/#materialize-metrics-endpoint) for what that means for network access to the port.
Create it in the namespace the scrapers run in (for example, `materialize`):

```bash
kubectl create secret generic materialize-sql-monitor \
  --namespace materialize \
  --from-literal=username=mz_support \
  --from-literal=password=
```

### Which Prometheus Distribution Am I Using?

An easy way to check if you are using Prometheus Operator is to see
if you have `PodMonitor` or `ServiceMonitor` CRDs in your cluster:

```bash
# Get the CRD resource directly:
kubectl get crd podmonitors.monitoring.coreos.com servicemonitors.monitoring.coreos.com
# This may fail with NotFound if you do not have Prometheus Operator.
# If you get a permission error, try the api-resources command instead:
kubectl api-resources | grep monitoring.coreos.com
# This is empty if you do not have Prometheus Operator
```

> [!INFO]
>   `monitoring.coreos.com` is the group for Prometheus Operator CRDs.

Alternatively, you can check this table of common Prometheus Distributions if you remember how you installed it:

<details>
  <summary>List of Common Prometheus Distributions</summary>
  <table>
	<thead>
			<tr>
					<th>Distribution</th>
					<th>Monitor Format</th>
					<th>Install Methods</th>
					<th>Notes</th>
			</tr>
	</thead>
	<tbody>
			<tr>
					<td><a href="https://materializeinc.github.io/materialize-monitoring/" rel="external" class="external-link">materialize-monitoring</a></td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Helm, Terraform</td>
					<td>The default monitoring stack for Materialize. Uses Grafana Alloy with <code>prometheus.operator</code> components.</td>
			</tr>
			<tr>
					<td><a href="https://github.com/prometheus-community/helm-charts/tree/main/charts/kube-prometheus-stack" rel="external" class="external-link">kube-prometheus-stack</a></td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Helm</td>
					<td>prometheus-community Helm Chart of Prometheus Operator.</td>
			</tr>
			<tr>
					<td><a href="https://github.com/prometheus-operator/kube-prometheus" rel="external" class="external-link">kube-prometheus</a></td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Helm</td>
					<td>Helm distribution of Prometheus Operator.</td>
			</tr>
			<tr>
					<td><a href="https://prometheus-operator.dev/" rel="external" class="external-link">prometheus-operator</a> (<a href="https://github.com/prometheus-operator/prometheus-operator" rel="external" class="external-link">Github</a>)</td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Manual</td>
					<td>The upstream distribution of Prometheus Operator</td>
			</tr>
			<tr>
					<td><a href="https://github.com/bitnami/charts/tree/main/bitnami/kube-prometheus" rel="external" class="external-link">Bitnami kube-prometheus</a></td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Helm</td>
					<td>Bitnami&rsquo;s Helm distribution of Prometheus Operator.</td>
			</tr>
			<tr>
					<td><a href="https://github.com/bitnami/charts/tree/main/bitnami/prometheus" rel="external" class="external-link">Bitnami Prometheus chart</a></td>
					<td><a href="#classic">Classic</a></td>
					<td>Helm</td>
					<td>Bitnami&rsquo;s Helm distribution of Prometheus without Operator.</td>
			</tr>
			<tr>
					<td><a href="https://grafana.com/docs/grafana-cloud/monitor-infrastructure/kubernetes-monitoring/configuration/helm-chart-config/" rel="external" class="external-link">k8s-monitoring-helm</a> (<a href="https://github.com/grafana/k8s-monitoring-helm/tree/main/charts/k8s-monitoring" rel="external" class="external-link">Github</a>)</td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Helm</td>
					<td>Grafana&rsquo;s Kubernetes monitoring Helm chart.</td>
			</tr>
			<tr>
					<td><a href="https://prometheus.io/docs/prometheus/latest/installation/" rel="external" class="external-link">prometheus.io</a></td>
					<td><a href="#classic">Classic</a></td>
					<td>Source, Binary, Docker</td>
					<td>Download of binary prometheus (brew, apt, et al.) or Docker image</td>
			</tr>
			<tr>
					<td><a href="https://github.com/thanos-community/helm-charts/tree/master/charts/thanos" rel="external" class="external-link">thanos-community helm</a></td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Helm</td>
					<td>Only if kube-prometheus-stack.enabled=true, otherwise refer to another technology.</td>
			</tr>
			<tr>
					<td><a href="https://aws.amazon.com/prometheus/" rel="external" class="external-link">Amazon Managed Prometheus (AMP)</a></td>
					<td><a href="#classic">Classic</a></td>
					<td>AWS</td>
					<td>Amazon&rsquo;s managed service for Prometheus. Not enabled by default.</td>
			</tr>
			<tr>
					<td><a href="https://cloud.google.com/stackdriver/docs/managed-prometheus" rel="external" class="external-link">Google Cloud Managed Service for Prometheus (GMP)</a></td>
					<td><a href="#gmp">Google Cloud Monitoring</a></td>
					<td>Google Cloud (GCP/GKE)</td>
					<td>Google&rsquo;s managed service for Prometheus. Enabled by default with GKE.</td>
			</tr>
			<tr>
					<td><a href="https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.operator.servicemonitors/" rel="external" class="external-link">Grafana Alloy with prometheus.operator</a></td>
					<td><a href="#prometheus-operator">Prometheus Operator</a></td>
					<td>Grafana Alloy</td>
					<td>Grafana Alloy&rsquo;s implementation of Prometheus Operator.</td>
			</tr>
	</tbody>
</table>
<p>The following distributions are known to not work at this time:</p>
<ul>
<li>VictoriaMetrics</li>
<li>Grafana Mimir</li>
<li>Cortex</li>
</ul>
</details>


> [!INFO]
>   If your metrics backend is an OpenTelemetry database, these scrape configurations are not the path you want.
>   The Alloy gateway forwards over OTLP natively today — see [Metrics > Storing](../storing/#otlp) for a generic OTLP backend such as Honeycomb, or [Google Cloud Monitoring](../storing/#gcm) and [Datadog](../storing/#datadog) for those exporters.
>   The `otlp-metrics-honeycomb` and `otel-metrics-fanout` profiles assemble both shapes.

### Prometheus Operator Scrape Downloads {#prometheus-operator}

These individual files can be used with a manual `prometheus-operator`
(including `kube-prometheus-stack` and `kube-prometheus`)
setup or a less-common manual Grafana Alloy `prometheus.operator` setup.


<table class="prometheus-scrapers">
  <thead>
    <tr>
      <th>Scraper</th>
      <th>Kind</th>
      <th>Download</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>clusterd</td>
      <td>monitoring.coreos.com/v1/PodMonitor</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/prometheus-operator/podmonitor-clusterd.yaml?xxhash=d1343768d3865164" download="clusterd-d1343768d3865164.yaml"><code>podmonitor-clusterd.yaml</code></a></td>
    </tr>
    <tr>
      <td>environmentd</td>
      <td>monitoring.coreos.com/v1/PodMonitor</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/prometheus-operator/podmonitor-environmentd.yaml?xxhash=cb80961f5213b005" download="environmentd-cb80961f5213b005.yaml"><code>podmonitor-environmentd.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-operator</td>
      <td>monitoring.coreos.com/v1/PodMonitor</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/prometheus-operator/podmonitor-materialize-operator.yaml?xxhash=64335808148e4ef8" download="materialize-operator-64335808148e4ef8.yaml"><code>podmonitor-materialize-operator.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-sql</td>
      <td>monitoring.coreos.com/v1/PodMonitor</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/prometheus-operator/podmonitor-sql.yaml?xxhash=564076a90db12393" download="materialize-sql-564076a90db12393.yaml"><code>podmonitor-sql.yaml</code></a></td>
    </tr>
    <tr>
      <td>mz-kubelet-cadvisor</td>
      <td>monitoring.coreos.com/v1alpha1/ScrapeConfig</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/prometheus-operator/scrapeconfig-cadvisor.yaml?xxhash=9f7fd044ea2df9c7" download="mz-kubelet-cadvisor-9f7fd044ea2df9c7.yaml"><code>scrapeconfig-cadvisor.yaml</code></a></td>
    </tr>
  </tbody>
</table>


#### Installing Prometheus Operator Scrape Configurations

The above files are meant to be used as manifests that can be passed
to `kubectl apply` directly.
The namespace isn't generally too important, but you may elect to
put them alongside your materialize-operator resource.

If your `materialize-operator` is in the `materialize` namespace, you can download each into a directory and apply like:
```bash
kubectl apply -f scrapers/ -n materialize
```

### Classic ScrapeConfig Downloads {#classic}

These are classic scrape_configs for non-operator Prometheus setups.
These are placed into your Prometheus configuration (prometheus.yml) as a single scrape_config.

> [!WARNING]
>   These are provided as best-effort convenience. Prometheus Operator
>   Monitors are the preferred implementation.


<table class="prometheus-scrapers">
  <thead>
    <tr>
      <th>Scraper</th>
      <th>Kind</th>
      <th>Download</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>scrape_config</td>
      <td>Classic ScrapeConfig</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/classic/scrape_config.yaml?xxhash=9c95cae25d1b638c" download="scrape_config-9c95cae25d1b638c.yaml"><code>scrape_config.yaml</code></a></td>
    </tr>
  </tbody>
</table>


See [Prometheus Configuration](https://prometheus.io/docs/prometheus/latest/configuration/configuration/)
for information on how to configure scrape_configs in your Prometheus setup.

### Google Cloud Managed Service for Prometheus PodMonitoring {#gmp}

These are PodMonitoring resources specifically for [Google Cloud Managed Service for Prometheus (GMP)](https://docs.cloud.google.com/stackdriver/docs/managed-prometheus).

> [!WARNING]
>   These are provided as best-effort convenience.


<table class="prometheus-scrapers">
  <thead>
    <tr>
      <th>Scraper</th>
      <th>Kind</th>
      <th>Download</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>clusterd</td>
      <td>monitoring.googleapis.com/v1/ClusterPodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/clusterd.yaml?xxhash=94899a50af87d083" download="clusterd-94899a50af87d083.yaml"><code>clusterd.yaml</code></a></td>
    </tr>
    <tr>
      <td>environmentd</td>
      <td>monitoring.googleapis.com/v1/ClusterPodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/environmentd.yaml?xxhash=7e24b75a0d93e8bb" download="environmentd-7e24b75a0d93e8bb.yaml"><code>environmentd.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-operator</td>
      <td>monitoring.googleapis.com/v1/PodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/materialize-operator.yaml?xxhash=d216820a50b4ec53" download="materialize-operator-d216820a50b4ec53.yaml"><code>materialize-operator.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-sql-mz-compute</td>
      <td>monitoring.googleapis.com/v1/ClusterPodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/materialize-sql-mz-compute.yaml?xxhash=ada272b9fa0082c2" download="materialize-sql-mz-compute-ada272b9fa0082c2.yaml"><code>materialize-sql-mz-compute.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-sql-mz-frontier</td>
      <td>monitoring.googleapis.com/v1/ClusterPodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/materialize-sql-mz-frontier.yaml?xxhash=db50e8f025eeb840" download="materialize-sql-mz-frontier-db50e8f025eeb840.yaml"><code>materialize-sql-mz-frontier.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-sql-mz-storage</td>
      <td>monitoring.googleapis.com/v1/ClusterPodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/materialize-sql-mz-storage.yaml?xxhash=af6b93d644fb3d5c" download="materialize-sql-mz-storage-af6b93d644fb3d5c.yaml"><code>materialize-sql-mz-storage.yaml</code></a></td>
    </tr>
    <tr>
      <td>materialize-sql-mz-usage</td>
      <td>monitoring.googleapis.com/v1/ClusterPodMonitoring</td>
      <td><a href="/materialize-monitoring/preview/heather-llms-txt/prometheus-scrapers/gmp/materialize-sql-mz-usage.yaml?xxhash=7b4c9342ed2545d5" download="materialize-sql-mz-usage-7b4c9342ed2545d5.yaml"><code>materialize-sql-mz-usage.yaml</code></a></td>
    </tr>
  </tbody>
</table>


> [!INFO]
>   GMP collects a subset of cAdvisor metrics by default, but the chart does not depend on it.
>   `alloy-gateway` scrapes each kubelet's `/metrics/cadvisor` endpoint itself, which yields a fuller set than GMP's default collection — see [Container metrics from the kubelet](#kubelet).

