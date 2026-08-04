---
title: "Terraform"
weight: 30
---

# Installing via Terraform

For teams standing up self-managed Materialize with [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/materialize-terraform-self-managed).
The observability stack comes up with the cluster, from the same modules, with no separate install step.

> [!WARNING]
>  **Preview.** The Terraform modules are built and validated, but have not yet shipped in a tagged release of `materialize-terraform-self-managed`.
>  Until they do, the module `source` cannot resolve — see [Before the first release](#before-the-first-release).
>  AWS and GCP are wired; Azure still uses the previous Prometheus + Grafana modules.

## What you get

One module per cloud creates the storage and identity the stack needs, then installs the [Helm charts](../helm/) at a pinned version:

| | |
|---|---|
| **Metrics** | Thanos, backed by object storage, with a Prometheus-compatible query API |
| **Logs** | Loki, backed by object storage |
| **Collection** | Alloy — an agent DaemonSet on every node, and a gateway for shaping and egress |
| **Dashboards** | The released Grafana dashboard set, via grafana-operator |
| **Alerting** | Alertmanager with the bundled rules |

Cloud-side, per backend: one bucket, and one IAM role (AWS) or Google service account with a Workload Identity binding (GCP).

## Usage

Observability is a module block in each example root, gated on one variable:

```hcl
module "monitoring" {
  count  = var.enable_observability ? 1 : 0
  source = "../../modules/monitoring"

  prefix     = var.name_prefix
  project_id = var.project_id
  region     = var.region

  namespace        = "monitoring"
  create_namespace = false # the operator module creates it

  node_selector = local.generic_node_labels

  materialize_instance_namespace = local.materialize_instance_namespace
  materialize_operator_namespace = local.materialize_operator_namespace

  depends_on = [module.operator, module.gke, module.generic_nodepool, module.coredns]
}
```

If you start from an example root, that block is already there. Turn it on in your `terraform.tfvars`:

```hcl
enable_observability = true
```

See the [tfvars reference](#tfvars-reference) below for the rest.

## tfvars reference

Variables you would realistically set in a `terraform.tfvars` at an example root.
Everything else has a default that suits the supported topology.

### Turning it on

| Variable | Default | Notes |
|---|---|---|
| `enable_observability` | `false` in `simple`, `true` in `enterprise` | The switch. Everything below is inert without it |

### Sizing and placement

These are set on the `monitoring` module block rather than as root variables, so change them there or add a matching root variable.

| Variable | Default | Notes |
|---|---|---|
| `sizing` | `"medium"` | `small`, `medium`, or `large`. The chart's defaults *are* medium, so that tier applies no overlay. Start at `small` for dev or a constrained node pool |
| `node_selector` | `{}` | Reaches every centralized workload. **Not** the Alloy agent DaemonSet, which must run on every node — see [Scheduling](#scheduling) |
| `tolerations` | `[]` | Reaches the agent too, since tolerations widen rather than narrow where a pod may run |
| `namespace` | `"monitoring"` | Also the namespace half of every workload-identity subject |
| `create_namespace` | `false` | The operator module already creates `monitoring` |

### Storage and retention

| Variable | Default | Notes |
|---|---|---|
| `storage_class` | `null` (cluster default) | Reaches the five PVC-backed workloads. **Required on GCP C4/N4 node pools** — see below. Loki's ingesters are unaffected; they use node-local `emptyDir` by design |
| `bucket_force_destroy` | `false` | Allows `terraform destroy` to delete non-empty buckets. Leave false outside throwaway environments |
| `enable_bucket_versioning` | `true` | Versioning is the disaster-recovery primitive — neither Loki nor Thanos has a native snapshot |
| `logs_retention_days` | `null` | Bucket-level expiry for logs. Off by default; Loki's compactor already enforces retention |
| `metrics_retention_days` | `null` | Off by default, and **leave it off** unless you have a reason. Thanos keeps blocks per downsampling resolution (raw 30d / 5m 90d / 1h 365d), and a bucket rule expiring sooner deletes blocks the compactor still references |

#### StorageClass on GCP C4 and N4 node pools

The **C4** and **N4** machine families accept only Hyperdisk.
They cannot attach Persistent Disk of any type, and GKE's default `standard-rwo` class is `pd-balanced`, so every PVC-backed workload hangs:

```
AttachVolume.Attach failed for volume "pvc-...":
  pd-balanced disk type cannot be used by c4-standard-8 machine type, badRequest
```

The other classes GKE creates by default — `premium-rwo` (`pd-ssd`) and `standard` (`pd-standard`) — are Persistent Disk too, so none of them work either.
GKE does not create a Hyperdisk class for you:

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: hyperdisk-balanced
provisioner: pd.csi.storage.gke.io
parameters:
  type: hyperdisk-balanced
volumeBindingMode: WaitForFirstConsumer
allowVolumeExpansion: true
```

Then `storage_class = "hyperdisk-balanced"`.

> [!WARNING]
>   Switching the class on an existing install does not migrate the volumes.
>   A StatefulSet's `volumeClaimTemplates` are immutable, so the old PVCs have to be deleted before the new class takes effect — and deleting them discards their contents.
>   That is cheap for these five (Thanos re-downloads blocks from object storage, and the Alertmanager and ruler volumes hold local state, not the rules themselves), but it is not a no-op.

### Extra metrics destinations

Thanos is always the primary metric store. These fan out in addition to it, from the Alloy gateway.

| Variable | Default | Notes |
|---|---|---|
| `enable_google_cloud_metrics` | `false` | GCP only. Export to Google Cloud Monitoring. Creates a service account with `roles/monitoring.metricWriter` and binds the gateway to it |
| `google_cloud_metrics_min_importance` | `"recommended"` | `essential`, `recommended`, `extended`, `diagnostic`, or `all`. Each tier includes the ones below it |
| `google_cloud_metrics_prefix` | `null` | Metric name prefix. Defaults to `workload.googleapis.com/mzmon` |

`min_importance` is a **cost control**, not a filter for convenience.
Cloud Monitoring bills per custom metric and per sample, so the tier you pick sets the bill.
`recommended` covers the dashboards; `all` sends the full surface and is a diagnostic setting, not a steady state.

The tiers come from the same `metric-tiers.yaml` the chart uses, so a Terraform-selected tier and a Helm-selected one always mean the same set.
See [Metrics > Storing](../../metrics/storing/) for what each tier contains.

> [!INFO]
>   Authentication is ADC only — there is no key-file path.
>   Without the Workload Identity binding the module creates, the exporter falls back to the node's service account, which works only if that account happens to hold `roles/monitoring.metricWriter`.

### Integration

| Variable | Default | Notes |
|---|---|---|
| `install_metrics_server` | `false` | The operator module installs metrics-server. If you set `install_metrics_server = false` **there**, set this `true` in the same change — the Materialize Console depends on the metrics API |
| `materialize_instance_namespace` | `"materialize-environment"` | Scopes scrape targets |
| `materialize_operator_namespace` | `"materialize"` | |
| `grafana_admin_password` | `null` | Generated when unset. Read it with `terraform output -raw grafana_admin_password` |

### Anything else

| Variable | Default | Notes |
|---|---|---|
| `additional_values` | `[]` | Raw Helm values YAML, appended last so it wins over everything the modules compute. The supported way to reach any chart setting the modules do not model |

```hcl
additional_values = [
  yamlencode({
    thanos = {
      receive = { replicaCount = 5 }
    }
  }),
]
```

The full input and output list is in the [Terraform variable reference](../../reference/terraform/materialize-monitoring-variables/).
Every chart value is reachable through `additional_values`; see the [Helm values reference](../../reference/helm/materialize-monitoring-values/).

## Outputs

```console
$ terraform output grafana_url
"http://grafana.monitoring.svc.cluster.local"

$ terraform output -raw grafana_admin_password
```

| Output | |
|---|---|
| `grafana_url` | In-cluster address (see [Reaching Grafana](#reaching-grafana)) |
| `grafana_admin_password` | Sensitive |
| `metrics_url` | Thanos Query. Prometheus-API-compatible, so anything that spoke to the old `prometheus_url` works against it |
| `logs_url` | Loki read endpoint |

## Reaching Grafana

Grafana is `ClusterIP` today — the chart exposes no ingress or service-type values yet — so access is a port-forward:

```bash
kubectl -n monitoring port-forward svc/grafana 3000:80
```

Log in as `admin` with the password from `terraform output -raw grafana_admin_password`.

## Scheduling

`node_selector` reaches every centralized workload — Loki, Thanos, Grafana, Alertmanager, kube-state-metrics, and the Alloy gateway — but deliberately **not** the Alloy agent.

The agent is a DaemonSet whose job is to collect from every node.
A node selector would confine it to one workload pool and silently stop collecting logs and node metrics from everywhere else.
`tolerations` do reach it, because tolerations widen where a pod may run, which is exactly what a DaemonSet wants.

## Capacity

The stack is meaningfully larger than the Prometheus + Grafana pair it replaces: microservice Loki, Thanos, Grafana, Alertmanager, kube-state-metrics, and two Alloy roles.

If your node pool is sized for the old shape, the first apply lands unschedulable pods.
Either grow the pool or start at `sizing = "small"`.

## Going to production

The modules cover the cloud-resource half of a production deployment — buckets, workload identity, version pinning — but not the sizing, retention, and capacity decisions that only you can make.

[Production Best Practices](../../operating/production-best-practices/) is the checklist, tagged by owner. Start with [what the Terraform path already handles](../../operating/production-best-practices/#terraform-consumer), then work the items still tagged `[operator]`.

Two that catch people out on a first production install:

- **A default StorageClass must exist.** Several components are PVC-backed and the modules do not create one.
- **Retention is enforced in-cluster, not by the bucket.** `metrics_retention_days` defaults to off for a reason — see the [Thanos checklist](../../operating/production-best-practices/#metrics-thanos).

## Before the first release

The wrapper modules pin the common module to a released tag, and that tag does not yet contain it.
Until the first release, point `source` at a branch or a local checkout:

```hcl
# In {aws,gcp}/modules/monitoring/main.tf
source = "github.com/MaterializeInc/materialize-monitoring//terraform/modules/materialize-monitoring?ref=my-branch"
```

The ref is the only version you set. The module reads its chart version from the `Chart.yaml` shipped beside it, so the two cannot disagree — `chart_version` exists only to pin something different deliberately.

A local path works too, but it **must be relative**.
Terraform copies a module referenced by an absolute path into `.terraform/modules/` without the chart directory beside it, and the sizing profiles stop resolving.
The module raises a precondition rather than failing quietly.

## Migrating from the previous stack

The `prometheus` and `grafana` modules are replaced. Applying the change:

- **Destroys** the `prometheus` and `grafana` releases and their PersistentVolumeClaims. Up to 15 days of local Prometheus data goes with them — there is no backfill, and collection restarts at install. Anything hand-created in the old Grafana does not carry over.
- Replaces the `prometheus_url` output with `metrics_url` and `logs_url`.
- Creates new cloud resources: buckets and identities per backend.

`enable_observability` keeps its name and defaults.

## Tearing it down

`terraform destroy` deadlocks unless the Grafana custom resources are deleted first, while grafana-operator is still running to remove its finalizers:

```bash
kubectl -n monitoring delete grafanadatasources,grafanamanifests,grafanas --all
```

The module orders its two releases correctly, but ordering *within* a release is not something Terraform controls.
See [Operating > Uninstalling](../../operating/uninstalling/) for the mechanism and for recovering a teardown that is already stuck.

## If you are not using Terraform

The [Helm charts](../helm/) are the full-fidelity surface, and everything above is a thin layer over them.
Terraform's job is the part Helm cannot do: creating the buckets, granting workload identity, and pinning a version.
