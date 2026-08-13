# materialize-monitoring Terraform module

Installs the `materialize-monitoring` Helm charts at a pinned version.

This is the **cloud-agnostic** half of the Terraform story. It knows chart shape — value paths, subchart names, deterministic service-account names — and nothing about clouds.
Buckets, IAM, and workload identity belong to a per-cloud wrapper in [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/materialize-terraform-self-managed), which creates those resources and passes their identifiers in through `object_storage`.

It lives in this repository, next to the chart it encodes, so a chart change and its Terraform consequence land in the same pull request.
It ships as part of the `materialize-monitoring` component rather than on a version stream of its own, which means **the module's Git tag is the chart version**: `?ref=materialize-monitoring/v0.9.0` installs chart `0.9.0`.

## Usage

```hcl
module "monitoring" {
  source = "github.com/MaterializeInc/materialize-monitoring//terraform/modules/materialize-monitoring?ref=materialize-monitoring/v0.8.0"

  namespace = "monitoring"
  sizing    = "small"

  object_storage = {
    cloud         = "aws"
    loki_bucket   = aws_s3_bucket.loki.id
    thanos_bucket = aws_s3_bucket.thanos.id
    region        = var.region

    loki_service_account_annotations   = { "eks.amazonaws.com/role-arn" = aws_iam_role.loki.arn }
    thanos_service_account_annotations = { "eks.amazonaws.com/role-arn" = aws_iam_role.thanos.arn }
  }
}
```

The trust policies for those roles need the Kubernetes service-account subjects, which the module emits rather than leaving you to derive:

```hcl
output "subjects" {
  value = module.monitoring.workload_identity_subjects
  # => { loki = "system:serviceaccount:monitoring:loki", thanos = "system:serviceaccount:monitoring:thanos-thanos", ... }
}
```

## How values compose

Helm merges `values` in order, later documents winning. The module builds four groups:

1. **Wiring** — namespaces, the Grafana admin Secret reference, Loki's NetworkPolicy selectors, subchart tags.
2. **Sizing** — the chart's own profile files, read from `charts/materialize-monitoring/profiles/` in this repository at the pinned commit.
3. **Storage** — buckets, objstore config, and workload-identity annotations derived from `object_storage`.
4. **`additional_values`** — your raw YAML, last, so it always wins.

That ordering is what makes the escape hatch real: every chart setting is reachable without waiting on a module release.

### `source` must not be an absolute local path

The sizing profiles are read out of the chart directory beside the module, which works for the two supported source forms — a Git source clones the whole repository, and a `./`-relative path is used in place.

An **absolute** local path is different: Terraform copies just the module directory into `.terraform/modules/` and leaves `charts/` behind. The module raises a precondition rather than silently dropping the profile.

## Grafana persistence and exposure

This module takes the connection details for Grafana's own state database (`grafana_database_host` and friends) and creates the Secret holding its password. It does not provision the database — that is the per-cloud wrapper's job, since the instance is a cloud resource.

It does not model an ingress or service type either. Those are plain chart values, so a wrapper computes the cloud-specific annotations and passes them through `additional_values`, which lands ahead of the caller's own documents and so keeps every chart-side validator in play.

> **Computed hosts need the gates set explicitly.** `grafana_database_enabled` and `grafana_database_manage_password_secret` default to inferring from `grafana_database_host` and `grafana_database_password`, which is right for literals. A wrapper that provisions the instance in the same apply must set both, because Terraform decides resource counts before it can know an endpoint or a generated password — leaving them null there fails the plan with `Invalid count argument`.

## Known gaps

- **mTLS between components is not wired.** The chart's TLS surface exists but is not yet driven by cert-manager.
- **No identity provider is configured for Grafana.** Reachable is not the same as protected; the admin password is the whole of the access control until one is set through `additional_values`.

Each is tracked; see the [design doc](https://materializeinc.github.io/materialize-monitoring/reference/internal/design-docs/20260803-terraform-modules/).

## Scheduling

`node_selector` and `tolerations` take one value each and fan out across the subcharts. Helm cannot template subchart values from a parent chart, so this has to happen in the consumer — but it is smaller than it looks: Loki resolves every workload through `coalesce $component.nodeSelector .Values.defaults.nodeSelector`, and Thanos has a real `global`, so eight keys cover the stack.

The asymmetry is deliberate:

| | `node_selector` | `tolerations` |
|---|---|---|
| Deployments and StatefulSets | applied | applied |
| `alloy-agent` (DaemonSet) | **not** applied | applied |

A node selector on the agent would confine it to one workload pool and silently stop collecting logs and node metrics from every other node. Tolerations do the opposite — they widen where it may run — which is exactly what a DaemonSet wants.

Two Loki memcached StatefulSets (`chunksCache`, `resultsCache`) do not render through `_pod.tpl` and so are named explicitly. That is the drift risk in this map, and the render check below is what catches it.

## Rolling Alloy on a config change

The module stamps `mzmon.materialize.cloud/values-hash` onto both Alloy pod templates, so a config change rolls them and an unchanged apply does not.

This exists because the chart cannot do it. The alloy subchart hashes `configMap.content` into its pod template only when it creates that ConfigMap itself, and this chart points it at ConfigMaps the umbrella renders — so the guard never fires and the pod template never changes when the pipeline or the metric filters do. The parent chart can compute the right hash but has nowhere to put it: subchart values are static YAML. A consumer holds the values before Helm renders them, so it can.

It has to be a **restart**, not a reload. The `-env` ConfigMaps are consumed with `envFrom`, and environment variables are fixed at container start, so neither the config reloader nor Alloy's `/-/reload` can pick up a filter change — they would silently no-op on half the config surface.

The hash covers every value document plus the chart version, rather than only the pipeline paths. Narrowing it would mean encoding chart internals here, which is the coupling that goes stale; the cost is that an unrelated change also rolls Alloy. Documents are decoded and re-encoded first, so reformatting `additional_values` rolls nothing.

## Testing

`examples/aws` and `examples/gcp` are not deployable roots — their buckets and roles are placeholders. They exist as plan targets, so the values this module composes can be rendered against the chart with no cluster involved:

```bash
make terraform-render
```

Each example is planned against a kubeconfig that does not exist (every resource is a create, so nothing refreshes and the providers never connect), the `helm_release` values are read back out of the plan, and the chart is rendered against them. Rendering is the only step that proves a value reached the setting it was aimed at — `terraform validate` accepts every wrong value path, because they are all still valid HCL.

Both clouds are rendered, and that is not redundancy. The chart's storage defaults are S3-shaped, so an AWS-only example agrees with every default it fails to set. The GCP example is what catches a backend key the module forgot to override.

The check has earned its place four times over: a missing Loki NetworkPolicy selector, two memcached StatefulSets the scheduling fan-out was missing, and the Loki schema period that pointed the chunk client at S3 on a GCS install — the last one only after a live cluster found it first, which is why the GCP example now exists.

<!-- BEGIN_TF_DOCS -->
## Requirements

| Name | Version |
| ---- | ------- |
| <a name="requirement_terraform"></a> [terraform](#requirement\_terraform) | >= 1.3.0 |
| <a name="requirement_helm"></a> [helm](#requirement\_helm) | >= 2.5.0, < 2.18.0 |
| <a name="requirement_kubernetes"></a> [kubernetes](#requirement\_kubernetes) | >= 2.10.0, < 2.39.0 |
| <a name="requirement_random"></a> [random](#requirement\_random) | >= 3.0.0, < 3.10.0 |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_additional_values"></a> [additional\_values](#input\_additional\_values) | Raw YAML documents appended to the Helm values, in order, after everything this module computes.<br/>Later documents win, so anything here overrides the module's opinion.<br/><br/>This is the supported way to reach chart settings the module does not model — including<br/>scheduling (node selectors, tolerations) and Grafana ingress, neither of which the module<br/>surfaces yet. See the README. | `list(string)` | `[]` | no |
| <a name="input_chart_registry"></a> [chart\_registry](#input\_chart\_registry) | OCI registry holding the materialize-monitoring charts. Override for a mirrored or air-gapped registry. | `string` | `"oci://ghcr.io/materializeinc/helm-charts"` | no |
| <a name="input_chart_version"></a> [chart\_version](#input\_chart\_version) | Version of the materialize-monitoring chart.<br/><br/>Leave null, which is the supported path: the module reads the version out of the chart's own<br/>`Chart.yaml` in this repository, so a module ref always installs the chart it shipped with and<br/>the two cannot drift. Set it only to pin a chart version different from the module's. | `string` | `null` | no |
| <a name="input_crds_chart_version"></a> [crds\_chart\_version](#input\_crds\_chart\_version) | Version of the materialize-monitoring-crds chart. Read from its `Chart.yaml` when null, like `chart_version`. Tracked separately because the CRDs chart has a deliberately looser lifecycle. | `string` | `null` | no |
| <a name="input_create_namespace"></a> [create\_namespace](#input\_create\_namespace) | Whether this module creates the namespace. Defaults to false because the Materialize operator module already creates `monitoring` in the supported topology. | `bool` | `false` | no |
| <a name="input_enable_monitoring_crds"></a> [enable\_monitoring\_crds](#input\_enable\_monitoring\_crds) | Install the materialize-monitoring-crds chart (prometheus-operator and grafana-operator CRDs).<br/><br/>Set false when the cluster already has them from elsewhere — kube-prometheus-stack, or a<br/>platform team that owns CRDs centrally — since Terraform would otherwise fail trying to<br/>create objects it does not own.<br/><br/>Note the teardown blast radius: destroying this release deletes the CRDs, which cascades to<br/>every GrafanaDashboard, GrafanaDatasource, PrometheusRule, and PodMonitor in the cluster,<br/>including ones this stack did not create. It is a separate `helm_release` so it can be<br/>targeted independently (`-target=module.monitoring.helm_release.crds`).<br/><br/>Teardown also needs the Grafana custom resources deleted before grafana-operator goes, or<br/>their finalizers have no remover and the CRDs wedge in Terminating. See the "Uninstalling"<br/>page in the docs. | `bool` | `true` | no |
| <a name="input_enable_sql_scraper"></a> [enable\_sql\_scraper](#input\_enable\_sql\_scraper) | Enable the SQL-on-scrape collector against environmentd.<br/><br/>Off by default. The chart enables it with an empty password, and no `mz_support` role is<br/>provisioned by the Materialize Terraform modules, so it would come up failing authentication.<br/>It also targets the legacy metric surface that native endpoints are replacing.<br/><br/>Supply `sql_scraper_password` when enabling it. | `bool` | `false` | no |
| <a name="input_google_cloud_metrics"></a> [google\_cloud\_metrics](#input\_google\_cloud\_metrics) | Also export metrics to Google Cloud Monitoring from the Alloy gateway. Null disables it; Thanos<br/>is unaffected either way.<br/><br/>`min_importance` picks a metric tier — `essential`, `recommended`, `extended`, `diagnostic`, or<br/>`all` — and each tier includes the ones below it. This is a cost control: GCM bills per custom<br/>metric and `all` sends the entire surface.<br/><br/>Authentication is ADC only. Bind the gateway ServiceAccount to a Google service account holding<br/>`roles/monitoring.metricWriter` through `object_storage.gateway_service_account_annotations`;<br/>failing that it falls back to the node's service account, which works only if that account has<br/>the role. | <pre>object({<br/>    min_importance = optional(string, "recommended")<br/>    prefix         = optional(string)<br/>  })</pre> | `null` | no |
| <a name="input_grafana_admin_password"></a> [grafana\_admin\_password](#input\_grafana\_admin\_password) | Grafana admin password. Generated when null. Supplied to Grafana as a Secret this module owns, rather than letting the bundled chart mint one — the chart's own generation does not survive upgrades. | `string` | `null` | no |
| <a name="input_grafana_admin_user"></a> [grafana\_admin\_user](#input\_grafana\_admin\_user) | Grafana admin username. | `string` | `"admin"` | no |
| <a name="input_grafana_database_enabled"></a> [grafana\_database\_enabled](#input\_grafana\_database\_enabled) | Whether to point Grafana at PostgreSQL at all.<br/><br/>Null infers it from `grafana_database_host`, which is right whenever that host is a literal. Set<br/>it explicitly when the host is computed from a resource created in the same apply. | `bool` | `null` | no |
| <a name="input_grafana_database_host"></a> [grafana\_database\_host](#input\_grafana\_database\_host) | Hostname of the PostgreSQL database backing Grafana's own state. Null (the default) leaves Grafana on SQLite, where everything created through the UI is lost on every restart. Host only — the port is `grafana_database_port`. | `string` | `null` | no |
| <a name="input_grafana_database_manage_password_secret"></a> [grafana\_database\_manage\_password\_secret](#input\_grafana\_database\_manage\_password\_secret) | Whether this module creates the Secret holding the database password, and references it from<br/>`grafana.ini` with `$__file{}`.<br/><br/>Null infers it from `grafana_database_password`. Set it explicitly when that password is<br/>generated in the same apply.<br/><br/>False is not a way to supply the password by another route: it means no Secret and no<br/>`$__file{}` reference at all, so a connection that needs one has to get both from<br/>`additional_values`. That is also the shape for a genuinely passwordless connection — a Cloud<br/>SQL Auth Proxy sidecar, or peer authentication. | `bool` | `null` | no |
| <a name="input_grafana_database_name"></a> [grafana\_database\_name](#input\_grafana\_database\_name) | Name of the database Grafana owns. | `string` | `"grafana"` | no |
| <a name="input_grafana_database_password"></a> [grafana\_database\_password](#input\_grafana\_database\_password) | Password for `grafana_database_user`, supplied to Grafana as a Secret this module owns and read<br/>from a mounted file rather than the environment.<br/><br/>Never inlined into `grafana.ini`, which renders into a ConfigMap.<br/><br/>Null when the connection needs no password — a Cloud SQL Auth Proxy sidecar with<br/>`--auto-iam-authn`, or a `trust`/peer-authenticated database. Note that IAM database<br/>authentication *without* a proxy does not work: Grafana reads its password once at startup and<br/>has no refresh hook, so the first reconnect after the token expires fails. | `string` | `null` | no |
| <a name="input_grafana_database_port"></a> [grafana\_database\_port](#input\_grafana\_database\_port) | Port for `grafana_database_host`. | `number` | `5432` | no |
| <a name="input_grafana_database_ssl_mode"></a> [grafana\_database\_ssl\_mode](#input\_grafana\_database\_ssl\_mode) | libpq SSL mode for the Grafana database connection.<br/><br/>`require` encrypts but does not authenticate the server. `verify-full` also authenticates it<br/>and is the better choice — but it needs a CA bundle on disk, which this module does not mount:<br/>supply `grafana.ini.database.ca_cert_path` and the matching `grafana.extraSecretMounts` through<br/>`additional_values` when you use it. | `string` | `"require"` | no |
| <a name="input_grafana_database_user"></a> [grafana\_database\_user](#input\_grafana\_database\_user) | Database user Grafana connects as. Must own `grafana_database_name`, because Grafana runs schema migrations at startup. | `string` | `"grafana"` | no |
| <a name="input_install_metrics_server"></a> [install\_metrics\_server](#input\_install\_metrics\_server) | Install metrics-server as part of this stack.<br/><br/>Leave false when the Materialize operator module installs it (the default topology), and set<br/>it true when that module has `install_metrics_server = false` — otherwise nothing provides the<br/>metrics API and the Materialize Console silently loses cluster metrics. | `bool` | `false` | no |
| <a name="input_install_node_exporter"></a> [install\_node\_exporter](#input\_install\_node\_exporter) | Install node-exporter as part of this stack.<br/><br/>On by default: node-level metrics are part of the stack's baseline, and nothing else in it<br/>collects them. Set false when the cluster already runs its own node-exporter DaemonSet — a<br/>second one wastes a per-node slot and produces the same series twice under two `job` labels,<br/>which double-counts in any `sum()` over them.<br/><br/>This writes the chart's `node-exporter.enabled` circuit breaker rather than a tag. Tags are<br/>OR'd, so `tags.node-exporter = false` would not turn it off while `tags.default` is true. | `bool` | `true` | no |
| <a name="input_install_timeout"></a> [install\_timeout](#input\_install\_timeout) | Timeout for each Helm release, in seconds. Well above Helm's 300s default: a first install brings up Loki, Thanos, Grafana, and both Alloy roles together. | `number` | `900` | no |
| <a name="input_materialize_instance_namespace"></a> [materialize\_instance\_namespace](#input\_materialize\_instance\_namespace) | Namespace the Materialize instance runs in. Used to scope scrape targets. | `string` | `"materialize-environment"` | no |
| <a name="input_materialize_operator_namespace"></a> [materialize\_operator\_namespace](#input\_materialize\_operator\_namespace) | Namespace the Materialize operator runs in. | `string` | `"materialize"` | no |
| <a name="input_min_zones"></a> [min\_zones](#input\_min\_zones) | Number of availability zones the node pool can actually launch in, used to adjust the hard zone<br/>spread on Thanos Receive and Loki's ingesters. Null leaves the chart's defaults alone, which<br/>assume two or more zones and is correct for every managed cloud default.<br/><br/>Set this when that assumption does not hold, because the chart's constraints fail closed rather<br/>than degrading:<br/><br/>  * `0` — no node carries a `topology.kubernetes.io/zone` label. Common on `kind` and on many<br/>    on-premises distributions. The hard constraints are dropped; the soft host spread stays.<br/>  * `1` — a single zone. `minDomains` becomes 1, which is satisfiable now and turns into real<br/>    protection the day a second zone appears.<br/>  * `2` or more — `minDomains` is set to the real count, which is stricter than the chart's<br/>    floor of 2 whenever you have more zones than that.<br/><br/>Leaving this null on a cluster with fewer than two zones leaves those pods **Pending forever**<br/>rather than unbalanced: below `minDomains` Kubernetes treats the global minimum as 0, so one<br/>zone holding every replica computes a skew equal to the replica count. One zone is exactly as<br/>broken as none. | `number` | `null` | no |
| <a name="input_namespace"></a> [namespace](#input\_namespace) | Namespace to install the monitoring stack into. | `string` | `"monitoring"` | no |
| <a name="input_node_selector"></a> [node\_selector](#input\_node\_selector) | Node selector for the centralized monitoring workloads.<br/><br/>Not applied to the Alloy agent: it is a DaemonSet that must reach every node to collect logs<br/>and node metrics, so constraining it to a workload pool would silently stop collection<br/>everywhere else. | `map(string)` | `{}` | no |
| <a name="input_object_storage"></a> [object\_storage](#input\_object\_storage) | Buckets and workload identity for the logging and metrics backends, supplied by the per-cloud<br/>wrapper module. Leave null to configure storage yourself through `additional_values`.<br/><br/>`cloud` selects the objstore dialect. The `*_service_account_annotations` maps carry the<br/>workload-identity annotation for each component's ServiceAccount — the chart validates that the<br/>annotation's cloud matches the objstore backend, so a mismatched pair fails at render time<br/>rather than at pod start.<br/><br/>`azure_storage_account` is required when `cloud` is `azure`, and only then: both Loki and Thanos<br/>name the account separately from the container. Azure needs nothing else — the annotation plus<br/>the pod label the module applies are all the Entra webhook requires. | <pre>object({<br/>    cloud                               = string<br/>    loki_bucket                         = string<br/>    thanos_bucket                       = string<br/>    region                              = optional(string)<br/>    endpoint                            = optional(string)<br/>    azure_storage_account               = optional(string)<br/>    loki_service_account_annotations    = optional(map(string), {})<br/>    thanos_service_account_annotations  = optional(map(string), {})<br/>    gateway_service_account_annotations = optional(map(string), {})<br/>  })</pre> | `null` | no |
| <a name="input_sizing"></a> [sizing](#input\_sizing) | Deployment size. The chart's defaults target `medium`, and the small/large profiles are deltas<br/>from it, so `medium` intentionally applies no profile at all.<br/><br/>Profiles are read from the chart directory in this repository at the same commit as the pinned<br/>chart version, so they cannot drift from it. A profile that does not exist yet is skipped, which<br/>is how Thanos sizing will start applying once those profiles land. | `string` | `"medium"` | no |
| <a name="input_sql_scraper_password"></a> [sql\_scraper\_password](#input\_sql\_scraper\_password) | Password for the SQL scraper's database user. Required when `enable_sql_scraper` is true. | `string` | `null` | no |
| <a name="input_storage_class"></a> [storage\_class](#input\_storage\_class) | StorageClass for the PVC-backed workloads. Null uses the cluster default.<br/><br/>Four are PVC-backed by default: Alertmanager, the Loki ruler, and the Thanos Store Gateway and<br/>Compactor. Loki's ingesters and Thanos Receive use node-local `emptyDir` by design — durability<br/>is the replication factor there, and a volume would pin them to one availability zone. The<br/>class is still fanned out to Receive so that re-enabling its persistence picks the class up<br/>rather than silently missing it.<br/><br/>Required where the default class cannot serve the nodes: GCP's C4 and N4 families take only<br/>Hyperdisk, and every Persistent Disk class fails to attach with `pd-balanced disk type cannot<br/>be used by <machine-type>`.<br/><br/>Changing it on an existing install does not move the volumes. `volumeClaimTemplates` are<br/>immutable, so the old PVCs must be deleted first — discarding their contents. | `string` | `null` | no |
| <a name="input_tolerations"></a> [tolerations](#input\_tolerations) | Tolerations for the monitoring workloads, including the Alloy agent DaemonSet — tolerations widen where a pod may run, which is what a DaemonSet wants. | <pre>list(object({<br/>    key      = optional(string)<br/>    operator = optional(string, "Equal")<br/>    value    = optional(string)<br/>    effect   = optional(string)<br/>  }))</pre> | `[]` | no |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_chart_version"></a> [chart\_version](#output\_chart\_version) | Chart version this release is pinned to. |
| <a name="output_grafana_admin_password"></a> [grafana\_admin\_password](#output\_grafana\_admin\_password) | Grafana admin password. |
| <a name="output_grafana_admin_secret_name"></a> [grafana\_admin\_secret\_name](#output\_grafana\_admin\_secret\_name) | Name of the Secret holding the Grafana admin credentials. |
| <a name="output_grafana_admin_user"></a> [grafana\_admin\_user](#output\_grafana\_admin\_user) | Grafana admin username. |
| <a name="output_grafana_url"></a> [grafana\_url](#output\_grafana\_url) | In-cluster URL for Grafana, which is what the module deploys by default. The chart can expose Grafana through `grafana.ingress` or `grafana.service` supplied via `additional_values`; this output does not follow that yet, so use the external hostname you configured there instead. |
| <a name="output_logs_url"></a> [logs\_url](#output\_logs\_url) | Loki read endpoint (query frontend). Reads carry a tenant header; see the chart's datasource configuration. |
| <a name="output_metrics_url"></a> [metrics\_url](#output\_metrics\_url) | Thanos Query endpoint. Prometheus-API-compatible, so consumers of a Prometheus URL keep working against it. |
| <a name="output_namespace"></a> [namespace](#output\_namespace) | Namespace the monitoring stack is installed into. |
| <a name="output_release_name"></a> [release\_name](#output\_release\_name) | Name of the materialize-monitoring Helm release. |
| <a name="output_remote_write_url"></a> [remote\_write\_url](#output\_remote\_write\_url) | Thanos Receive remote-write endpoint, for writers outside this stack. |
| <a name="output_service_account_names"></a> [service\_account\_names](#output\_service\_account\_names) | ServiceAccount names the chart renders for storage-bound components. |
| <a name="output_workload_identity_subjects"></a> [workload\_identity\_subjects](#output\_workload\_identity\_subjects) | `system:serviceaccount:<namespace>:<sa>` subjects for the components that bind to cloud object storage. Use these when building IRSA / Workload Identity trust policies. |
<!-- END_TF_DOCS -->
