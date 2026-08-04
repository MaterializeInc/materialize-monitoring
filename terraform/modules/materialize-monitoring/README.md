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

## Known gaps

- **Grafana ingress is not modelled.** The chart exposes no ingress or service-type values yet.
- **Grafana is not reachable from outside the cluster.** The chart exposes no ingress or service-type values yet, so `grafana_url` is an in-cluster address and access means a port-forward.
- **mTLS between components is not wired.** The chart's TLS surface exists but is not yet driven by cert-manager.

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
| <a name="input_grafana_admin_password"></a> [grafana\_admin\_password](#input\_grafana\_admin\_password) | Grafana admin password. Generated when null. Supplied to Grafana as a Secret this module owns, rather than letting the bundled chart mint one — the chart's own generation does not survive upgrades. | `string` | `null` | no |
| <a name="input_grafana_admin_user"></a> [grafana\_admin\_user](#input\_grafana\_admin\_user) | Grafana admin username. | `string` | `"admin"` | no |
| <a name="input_install_metrics_server"></a> [install\_metrics\_server](#input\_install\_metrics\_server) | Install metrics-server as part of this stack.<br/><br/>Leave false when the Materialize operator module installs it (the default topology), and set<br/>it true when that module has `install_metrics_server = false` — otherwise nothing provides the<br/>metrics API and the Materialize Console silently loses cluster metrics. | `bool` | `false` | no |
| <a name="input_install_timeout"></a> [install\_timeout](#input\_install\_timeout) | Timeout for each Helm release, in seconds. Well above Helm's 300s default: a first install brings up Loki, Thanos, Grafana, and both Alloy roles together. | `number` | `900` | no |
| <a name="input_materialize_instance_namespace"></a> [materialize\_instance\_namespace](#input\_materialize\_instance\_namespace) | Namespace the Materialize instance runs in. Used to scope scrape targets. | `string` | `"materialize-environment"` | no |
| <a name="input_materialize_operator_namespace"></a> [materialize\_operator\_namespace](#input\_materialize\_operator\_namespace) | Namespace the Materialize operator runs in. | `string` | `"materialize"` | no |
| <a name="input_namespace"></a> [namespace](#input\_namespace) | Namespace to install the monitoring stack into. | `string` | `"monitoring"` | no |
| <a name="input_node_selector"></a> [node\_selector](#input\_node\_selector) | Node selector for the centralized monitoring workloads.<br/><br/>Not applied to the Alloy agent: it is a DaemonSet that must reach every node to collect logs<br/>and node metrics, so constraining it to a workload pool would silently stop collection<br/>everywhere else. | `map(string)` | `{}` | no |
| <a name="input_object_storage"></a> [object\_storage](#input\_object\_storage) | Buckets and workload identity for the logging and metrics backends, supplied by the per-cloud<br/>wrapper module. Leave null to configure storage yourself through `additional_values`.<br/><br/>`cloud` selects the objstore dialect. The `*_service_account_annotations` maps carry the<br/>workload-identity annotation for each component's ServiceAccount — the chart validates that the<br/>annotation's cloud matches the objstore backend, so a mismatched pair fails at render time<br/>rather than at pod start. | <pre>object({<br/>    cloud                               = string<br/>    loki_bucket                         = string<br/>    thanos_bucket                       = string<br/>    region                              = optional(string)<br/>    endpoint                            = optional(string)<br/>    loki_service_account_annotations    = optional(map(string), {})<br/>    thanos_service_account_annotations  = optional(map(string), {})<br/>    gateway_service_account_annotations = optional(map(string), {})<br/>  })</pre> | `null` | no |
| <a name="input_sizing"></a> [sizing](#input\_sizing) | Deployment size. The chart's defaults target `medium`, and the small/large profiles are deltas<br/>from it, so `medium` intentionally applies no profile at all.<br/><br/>Profiles are read from the chart directory in this repository at the same commit as the pinned<br/>chart version, so they cannot drift from it. A profile that does not exist yet is skipped, which<br/>is how Thanos sizing will start applying once those profiles land. | `string` | `"medium"` | no |
| <a name="input_sql_scraper_password"></a> [sql\_scraper\_password](#input\_sql\_scraper\_password) | Password for the SQL scraper's database user. Required when `enable_sql_scraper` is true. | `string` | `null` | no |
| <a name="input_storage_class"></a> [storage\_class](#input\_storage\_class) | StorageClass for the five PVC-backed workloads (Alertmanager, the Loki ruler, and Thanos<br/>receive/compactor/store-gateway). Null uses the cluster default. Loki's ingesters are<br/>unaffected — node-local `emptyDir` by design.<br/><br/>Required where the default class cannot serve the nodes: GCP's C4 and N4 families take only<br/>Hyperdisk, and every Persistent Disk class fails to attach with `pd-balanced disk type cannot<br/>be used by <machine-type>`.<br/><br/>Changing it on an existing install does not move the volumes. `volumeClaimTemplates` are<br/>immutable, so the old PVCs must be deleted first — discarding their contents. | `string` | `null` | no |
| <a name="input_tolerations"></a> [tolerations](#input\_tolerations) | Tolerations for the monitoring workloads, including the Alloy agent DaemonSet — tolerations widen where a pod may run, which is what a DaemonSet wants. | <pre>list(object({<br/>    key      = optional(string)<br/>    operator = optional(string, "Equal")<br/>    value    = optional(string)<br/>    effect   = optional(string)<br/>  }))</pre> | `[]` | no |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_chart_version"></a> [chart\_version](#output\_chart\_version) | Chart version this release is pinned to. |
| <a name="output_grafana_admin_password"></a> [grafana\_admin\_password](#output\_grafana\_admin\_password) | Grafana admin password. |
| <a name="output_grafana_admin_secret_name"></a> [grafana\_admin\_secret\_name](#output\_grafana\_admin\_secret\_name) | Name of the Secret holding the Grafana admin credentials. |
| <a name="output_grafana_admin_user"></a> [grafana\_admin\_user](#output\_grafana\_admin\_user) | Grafana admin username. |
| <a name="output_grafana_url"></a> [grafana\_url](#output\_grafana\_url) | In-cluster URL for Grafana. Grafana is ClusterIP-only today, so reaching it from outside the cluster needs a port-forward. |
| <a name="output_logs_url"></a> [logs\_url](#output\_logs\_url) | Loki read endpoint (query frontend). Reads carry a tenant header; see the chart's datasource configuration. |
| <a name="output_metrics_url"></a> [metrics\_url](#output\_metrics\_url) | Thanos Query endpoint. Prometheus-API-compatible, so consumers of a Prometheus URL keep working against it. |
| <a name="output_namespace"></a> [namespace](#output\_namespace) | Namespace the monitoring stack is installed into. |
| <a name="output_release_name"></a> [release\_name](#output\_release\_name) | Name of the materialize-monitoring Helm release. |
| <a name="output_remote_write_url"></a> [remote\_write\_url](#output\_remote\_write\_url) | Thanos Receive remote-write endpoint, for writers outside this stack. |
| <a name="output_service_account_names"></a> [service\_account\_names](#output\_service\_account\_names) | ServiceAccount names the chart renders for storage-bound components. |
| <a name="output_workload_identity_subjects"></a> [workload\_identity\_subjects](#output\_workload\_identity\_subjects) | `system:serviceaccount:<namespace>:<sa>` subjects for the components that bind to cloud object storage. Use these when building IRSA / Workload Identity trust policies. |
<!-- END_TF_DOCS -->
