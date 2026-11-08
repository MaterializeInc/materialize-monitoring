---
title: "Storing"
weight: 20
---

# Storing Metrics

`alloy-gateway` forwards the metrics it collects to one or more storage backends.
Out of the box that is an in-cluster **Thanos Receive**, but you can send metrics to any number of Prometheus remote-write stores (Thanos, Mimir, Amazon Managed Prometheus, Grafana Cloud, …), to an **OpenTelemetry (OTLP)** endpoint, or to several of both at once — each enabled destination receives its own copy, filtered to its own [importance tier](#the-importance-tiers).

This page walks through the three decisions that setup involves:

- **Where** metrics go — the [remote-write destinations](#the-remote-write-destinations) and the [other backends](#other-metric-storage-backends) (generic OTLP, Google Cloud Monitoring, Datadog).
- **How** the gateway authenticates — [authentication](#authentication) and the [gateway Secret](#supplying-credentials-the-gateway-secret) that holds the credentials.
- **What** each backend stores — the [importance tiers and denylist](#controlling-what-each-destination-stores) that decide which metrics reach each destination.

If you keep the bundled Thanos, it persists blocks to object storage — see [Thanos object storage](#thanos-object-storage) for the S3/GCS/Azure setup.

## The remote-write destinations {#the-remote-write-destinations}

A Prometheus remote-write store is an entry of type `prometheus` in `pipeline.destinations`, the map of everything the gateway writes to.
The chart ships one such entry, `thanos`, pointing at the bundled in-cluster Thanos Receive; adding an entry adds a destination.

A `prometheus` entry takes the following values:

| Value | Purpose |
|---|---|
| `type` | `prometheus`. Required on an entry you add; `thanos` already has it. |
| `enabled` | Write to this destination (default `true`). |
| `url` | Remote-write endpoint. Required, except on `thanos`, which defaults to the in-cluster Receive. |
| `metrics.minMetricImportance` | Which [tier](#the-importance-tiers) this destination receives (default `all`). |
| `authType` | `none` (default), `basicAuth`, `bearer`, or `sigv4`. |
| `sigv4` | AWS SigV4 signing config, for [Amazon Managed Prometheus](#amazon-managed-prometheus-sigv4--irsa). |
| `tls` | Client TLS for this hop. |
| `externalLabels` | Extra labels stamped on every sample this destination receives. |

For every field, see [`pipeline.destinations` in the values reference](../../reference/helm/materialize-monitoring-values/#destinations).

Point the bundled destination at an external store instead:

```yaml
pipeline:
  destinations:
    thanos:
      url: https://<your-remote-write-endpoint>/api/v1/write
```

Or write to two backends at once, each on its own tier:

```yaml
pipeline:
  destinations:
    # Everything, to storage you already pay for.
    thanos:
      metrics:
        minMetricImportance: all
    # Alerting metrics only, to a backend that bills per sample.
    amp:
      type: prometheus
      url: https://aps-workspaces.us-east-1.amazonaws.com/workspaces/ws-.../api/v1/remote_write
      metrics:
        minMetricImportance: essential
      authType: sigv4
      sigv4:
        region: us-east-1
```

The name is more than a label in `values`.
It becomes the **Alloy component label**, so it appears as `prometheus.remote_write.<name>` in the gateway's own metrics and in its UI.
It's also the fragment the [credential environment variables](#credential-environment-variables) are derived from.
It must match `[a-zA-Z_][a-zA-Z0-9_]*`, with no dashes or dots, and `egress` and `destination` are reserved.
Both rules fail at render rather than at gateway startup.

### One component per destination, and why it matters {#one-component-per-destination}

Each destination gets its own `prometheus.remote_write` component with its own write-ahead log, rather than sharing one component with several `endpoint` blocks.
That costs a WAL per destination and buys two things:

- **Tiers that are real.** The importance filter sits *upstream* of the component, so a destination on `essential` writes a WAL holding only essential series.
  Sharing one component would mean filtering with `write_relabel_config` on the way *out* of the WAL — every destination paying full-firehose disk regardless of the tier it asked for.
- **Independent failure domains.** A backend that stops accepting writes backs up its own WAL and nothing else.
  On a shared component a stuck endpoint holds back WAL truncation for every other endpoint too, so one unreachable SaaS backend would grow the disk the in-cluster path depends on.

Budget disk accordingly: the gateway's WAL volume has to hold the sum of the enabled destinations' buffers, not one.
The `essential` tier is a small fraction of `all`, so a filtered second destination costs far less than a doubling.

### External labels

Every sample carries a `cluster` label identifying its source cluster, set from the chart's `clusterName` (default `default`).
Set it per install so series from different clusters stay distinct once they land in a shared backend:

```yaml
clusterName: prod-us-east-1
```

The Terraform module sets it from its `cluster_name` input.
The same value labels log lines and alerts.

`cluster` is applied to every destination.
Add labels for one destination only with its `externalLabels` map, which is useful where one backend needs a tenant or account key the others do not:

```yaml
pipeline:
  destinations:
    amp:
      externalLabels:
        account: "012345678901"
```

Setting `cluster` in `externalLabels` replaces the environment-derived one for that destination.

### Upgrading from `prometheusRemoteWrite` {#upgrading-from-prometheusremotewrite}

Remote-write destinations used to be configured under `pipeline.metrics.gateway.destination.prometheusRemoteWrite`, a map of the same shape.
That map is deprecated.
Each entry you still set there applies as a `prometheus` entry of `pipeline.destinations`, and the render warns.

To move an entry, apply the following changes:

- Put it under `pipeline.destinations`, with `type: prometheus`.
- Move `minMetricImportance` and `unfilteredMetricsEnv` under `metrics`.
  Every other field keeps its name.
- Check the variable names.
  While an entry with no counterpart stays in the old map, it keeps the names that map derived,
  such as `GATEWAY_PROM_DEST_AMP` and `GATEWAY_PROMETHEUS_DEST_AMP_PASSWORD`.
  Once moved, it derives `GATEWAY_AMP_DEST` and `GATEWAY_AMP_DEST_PASSWORD`.
  If a Secret already uses the old names, keep them with `urlEnv` and the `*Env` fields.

The following example moves an entry:

```yaml
pipeline:
  destinations:
    amp:                                  # was: prometheusRemoteWrite.amp
      type: prometheus
      url: https://aps-workspaces.us-east-1.amazonaws.com/workspaces/ws-.../api/v1/remote_write
      metrics:
        minMetricImportance: essential    # was: minMetricImportance
      authType: sigv4
      sigv4:
        region: us-east-1
```

An entry named after a destination of another type, such as `loki`, fails at render rather than turning that destination into a remote write.

The map's older shape, with `url`, `authType`, and the rest directly under `prometheusRemoteWrite` rather than under a name,
**fails at render** with a message naming the keys to move.
Helm merges maps, so a leftover `prometheusRemoteWrite.url` would otherwise apply to nothing,
and the default endpoint would keep receiving with no error.
Move those keys onto `pipeline.destinations.thanos` to keep the bundled Receive.

## Authentication

Credentials are supplied through environment variables (not inline in values), so they can be sourced from a mounted Secret.
Set `authType` and fill in the matching block:

- **`none`** — no auth (the in-cluster Thanos default).
- **`basicAuth`** — username / password from env.
- **`bearer`** — bearer token from env.
- **`sigv4`** — AWS SigV4 signing, derived from IRSA (see [Amazon Managed Prometheus](#amazon-managed-prometheus-sigv4--irsa) below).

An `otlp` destination in `pipeline.destinations` takes `none`, `basicAuth`, `bearer`, `headers`, or `sigv4`; see [Generic OTLP](#otlp).
A `googleCloud` destination uses the gateway's Google identity, and a `datadog` destination its API key.

Whichever destination and method you choose, the gateway reads the actual secret from an environment variable at runtime (`sys.env(...)`).
There are two ways to populate that variable:

- **Inline in `values`** — the Prometheus remote-write and Loki destinations accept the credential directly (`basicAuth.password`, `bearer.token`, …). When set, the value is baked into the gateway ConfigMap in plaintext, so this is convenient for a quick start but weaker for production.
- **From a Secret** — leave the inline field blank and provide the same env var through a Secret. This is the preferred path for every destination.

Both are described next.

### Supplying credentials (the gateway Secret)

The gateway loads its environment from two objects that share one name — `mzmon-alloy-gateway-env` (that is `<fullnameOverride>-alloy-gateway-env`, `mzmon` by default) — in the release namespace:

- a **ConfigMap** the chart generates — non-secret env such as the per-destination allowlists and tenant map, plus any credentials you chose to set inline in `values`;
- a **Secret** *you* create — the credential values in the table below, kept out of the rendered manifests.

The chart does **not** create the Secret; it is mounted `optional: true`, so you populate it out-of-band with only the keys your enabled destinations need.
For example, a bearer-authenticated `otlp` destination named `collector` and a `datadog` destination named `datadog` need two keys:

```bash
kubectl create secret generic mzmon-alloy-gateway-env \
  --namespace monitoring \
  --from-literal=GATEWAY_COLLECTOR_DEST_BEARER_TOKEN='<token>' \
  --from-literal=GATEWAY_DATADOG_DEST_API_KEY='<api-key>'
```

The Secret's keys are injected as environment variables next to the ConfigMap's, so the key names **are** the env-var names in the table below.

> [!WARNING]
>   The Secret name must match the release: with the default `fullnameOverride: mzmon` it is `mzmon-alloy-gateway-env`, and it must live in the namespace the gateway runs in (`monitoring` above).
>   Because the mount is optional, a mismatched name or namespace is silently ignored — the destination then authenticates with empty credentials instead of failing loudly.

> [!INFO]
>   `kubectl create secret` is fine for a first install, but in production source the Secret from Sealed Secrets, External Secrets, or SOPS rather than committing raw credentials.

### Credential environment variables

Populate only the rows for the destinations and auth methods you enable:

`<NAME>` below is the destination's key in `pipeline.destinations`, uppercased with anything outside `A-Z0-9` folded to `_`.
A `prometheus` destination named `amp` reads `GATEWAY_AMP_DEST_PASSWORD`,
and a `datadog` destination named `datadog` reads `GATEWAY_DATADOG_DEST_API_KEY`.
Every row is per destination, so two destinations on `basicAuth` need two pairs of keys.

| Destination · method | `values` block | Secret keys (env vars) |
|---|---|---|
| `prometheus` or `otlp` · `basicAuth` | `pipeline.destinations.<name>.basicAuth` | `GATEWAY_<NAME>_DEST_USERNAME`, `GATEWAY_<NAME>_DEST_PASSWORD` |
| `prometheus` or `otlp` · `bearer` | `pipeline.destinations.<name>.bearer` | `GATEWAY_<NAME>_DEST_BEARER_TOKEN` |
| `otlp` · `headers` | `pipeline.destinations.<name>.headers` | whatever each header's `valueEnv` names; you choose |
| `datadog` | `pipeline.destinations.<name>.apiKey` | `GATEWAY_<NAME>_DEST_API_KEY` |
| `prometheus` or `otlp` · `sigv4` | `pipeline.destinations.<name>.sigv4` | none; uses the pod's IRSA identity |
| Deprecated `prometheusRemoteWrite` | `<name>.basicAuth`, `.bearer` | `GATEWAY_PROMETHEUS_DEST_<NAME>_USERNAME`, `…_PASSWORD`, `…_BEARER_TOKEN` |
| Deprecated `otel` block | `…otel.auth.*`, `…otel.datadogExporter` | `GATEWAY_OTEL_DEST_USERNAME`, `…_PASSWORD`, `…_BEARER_TOKEN`, `…_DATADOG_API_KEY` |

SigV4 and the cloud-native exporters (GCM via Workload Identity) carry no secret keys — they authenticate with the gateway pod's ambient cloud identity, so those rows stay out of the Secret entirely.

Each destination can also name its own variables, such as `basicAuth.usernameEnv`, `bearer.tokenEnv`, or `apiKey.keyEnv`, where the derived name is inconvenient or a Secret already uses another.
The Terraform module sets them explicitly for this reason; see [Through Terraform](#through-terraform).

## Controlling what each destination stores

Not every backend should receive every metric.
`alloy-gateway` gives you two independent controls: a global **denylist** that drops metrics before they reach any destination, and a per-destination **importance filter** that keeps only metrics at or above a chosen tier.

### The importance tiers

Every metric in the registry is classified by *importance* — how likely you are to want it — independent of which backend you use.
The levels, from most to least important, are **essential**, **recommended**, **extended**, and **diagnostic**.
A fifth value, **all**, is a firehose meaning "everything scraped, including metrics the registry has not classified."
The tier definitions and the full membership of each tier live in [the metric list](../../reference/list-metrics/).

<!-- The tier *definitions* live in reference/list-metrics.md; this page owns the config/operational angle only. Keep them from drifting. -->

Each destination picks a floor with `minMetricImportance`.
The filter is cumulative — a floor keeps that tier **and every tier more important than it**:

| `minMetricImportance` | Metrics kept |
|---|---|
| `essential` | essential |
| `recommended` | essential + recommended |
| `extended` | essential + recommended + extended |
| `diagnostic` | essential + recommended + extended + diagnostic |
| `all` | everything scraped, classified or not (`.*`) |

The defaults lean permissive for cheap local storage and frugal for metered SaaS backends:

| Destination | Default `minMetricImportance` |
|---|---|
| `pipeline.destinations.thanos` (bundled Thanos) | `all` |
| `pipeline.destinations` entry of type `prometheus` | `all` |
| `pipeline.destinations` entry of type `otlp` | `all` |
| `pipeline.destinations` entry of type `googleCloud` | `recommended` |
| `pipeline.destinations` entry of type `datadog` | `recommended` |

Set it per destination:

```yaml
pipeline:
  destinations:
    datadog:
      type: datadog
      metrics:
        minMetricImportance: essential   # ship only alerting metrics to Datadog
```

The series the chart's recording rules write, such as the `ext:*` series for the metadata database, have tiers too.
A destination on `recommended` receives them without the metrics they're recorded from.
The Thanos ruler reads those metrics from the bundled Thanos, so they have to reach the `thanos` destination.
The render warns when the `thanos` destination's tier drops a metric that an installed recording rule reads.
For more information, see [Recorded series](../../alerting/configuring/#recorded-series).

For a worked example that keeps full fidelity in Thanos while sending a smaller, cheaper slice to Google Cloud Monitoring and Datadog, see the annotated [`otel-metrics-fanout.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/otel-metrics-fanout.values.yaml) profile.
The remote-write equivalent — Thanos on `all`, Amazon Managed Prometheus on `essential` — is [`aws-amp-fanout.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/aws-amp-fanout.values.yaml).

On the remote-write side the tier does more than filter, because the filter runs before the write-ahead log: a destination on `essential` also buffers only essential series to disk.
See [One component per destination](#one-component-per-destination).

### How the allowlist is built

Tier membership is generated from the query registry into `charts/materialize-monitoring/pre-rendered/metrics/metric-tiers.yaml` (via `mz-monitoring-build gen-metric-tiers`, or `make metric-tiers`).
At render time the chart reads that file, unions the tiers at or above each destination's `minMetricImportance`, and hands the gateway the result as an allowlist regex — one environment variable per destination, such as `GATEWAY_AMP_DEST_METRICS_ALLOWLIST` for a destination named `amp`.
`all` skips the file entirely and uses `.*`.
Do not edit `metric-tiers.yaml` by hand: reclassify metrics in the registry and regenerate.

### The denylist

`denyMetrics` drops metrics for **every** destination, at the gateway input, before the per-destination fan-out:

```yaml
pipeline:
  metrics:
    gateway:
      denyMetrics:
        - some_noisy_metric
        - another_expensive_.*        # entries are regex fragments, OR-joined
```

Reach for the denylist to shed a metric everywhere (cost, cardinality, noise); reach for `minMetricImportance` to tune what an *individual* backend receives.

Entries in both the denylist and the tiers name Prometheus series, as Thanos shows them: `foo_total`, `foo_bucket`, `foo_count`.
Inside the gateway a typed histogram is one metric named for its family, and the match is made per series all the same.

| Entry names | Effect on a histogram `foo` |
|---|---|
| `foo_bucket` on the denylist | `foo_count` and `foo_sum` still reach every destination; the buckets do not |
| `foo_count` in a tier, `foo_bucket` in a lower one | A destination on the higher tier receives `foo_count` alone |
| `foo_bucket` in a tier | A destination on that tier receives the whole histogram |

### Through Terraform

The `materialize-monitoring` module exposes the OTLP-family destinations directly, each taking the same `min_importance` tier documented above:

```hcl
google_cloud_metrics = { min_importance = "recommended" }

datadog_metrics = { site = "datadoghq.com", min_importance = "essential" }
datadog_api_key = var.datadog_api_key

otlp_metrics = {
  url            = "api.honeycomb.io"
  min_importance = "recommended"
  auth_headers   = { "x-honeycomb-dataset" = "mzmon" }
}
otlp_auth_header_secrets = { "x-honeycomb-team" = var.honeycomb_api_key }
```

Remote-write destinations go through `prometheus_remote_write`, keyed by name; each key becomes a `pipeline.destinations` entry of type `prometheus`.
A key of `thanos` **retunes** the bundled destination rather than adding a second one, so dropping local fidelity and adding AMP is one input:

```hcl
prometheus_remote_write = {
  thanos = { min_importance = "all" }
  amp = {
    url            = "https://aps-workspaces.us-east-1.amazonaws.com/workspaces/ws-.../api/v1/remote_write"
    min_importance = "essential"
    auth_type      = "sigv4"
    sigv4_region   = "us-east-1"
  }
}

# SigV4 needs no credentials, only an identity to sign as.
gateway_service_account_annotations = {
  "eks.amazonaws.com/role-arn" = aws_iam_role.amp_writer.arn
}
```

`basicAuth` and `bearer` destinations take their credentials from `prometheus_remote_write_credentials`, keyed by the same destination name.
The module derives the environment variable names from that name, as `GATEWAY_PROMETHEUS_DEST_<NAME>_PASSWORD` and so on,
and writes them into the chart's values, so the Secret it creates and the config the gateway reads cannot disagree.

Credentials do **not** go through the Helm values — the module puts them in the gateway Secret instead, and rolls the gateway when one changes.
See the [module README](https://github.com/MaterializeInc/materialize-monitoring/blob/main/terraform/modules/materialize-monitoring/README.md#metric-destinations) for the environment variables each input becomes.

The per-cloud wrappers in `materialize-terraform-self-managed` surface Google Cloud Monitoring under flatter names (`enable_google_cloud_metrics`, `google_cloud_metrics_min_importance`), because it is the one destination needing cloud resources the chart cannot create — a service account and the Workload Identity binding it authenticates with.
Anything not modelled here is still reachable through `additional_values`.
See [Getting Started > Terraform](../../getting-started/terraform/#extra-metrics-destinations).

### Operational notes

> [!NOTE]
>   **The filter fails open.**
>   If a destination's allowlist environment variable is empty or unset, the gateway falls back to `.*`.
>   A misconfiguration therefore ships *everything* to that backend rather than nothing — safe for visibility, but watch cost on metered backends.

> [!WARNING]
>   **The gateway shards scrape targets across replicas.**
>   Scraping runs with clustering enabled, so targets are distributed over the gateway pods.
>   During a partial rollout a metric can look "missing" simply because its target is being scraped by a pod that has not yet picked up the new config — roll out **all** gateway replicas before concluding a metric is filtered out.

> [!INFO]
>   **Backend schema browsers are historical.**
>   A metric appearing in a backend's schema or column list (Honeycomb, Datadog, …) is not proof it is arriving *now* — those views are cumulative and can show columns from before a filter change.
>   Query for recent samples to confirm what is currently flowing.

## Thanos object storage

When you run the bundled **Thanos** (the default destination), it persists metric
blocks to object storage — the same durability model as Loki's chunks.
The supported backends are **S3-compatible** storage (AWS S3, MinIO, Ceph, R2, …), **Google Cloud Storage**, and **Azure Blob Storage**.
See the [Thanos storage reference](https://thanos.io/tip/thanos/storage.md/) for the full config schema.

### The objstore Secret

Thanos reads its object-store config from a Kubernetes Secret.
The chart does **not** create it by default (`thanos.global.objstore.createSecret: false`), so you supply it:

| Value | Default | Purpose |
|---|---|---|
| `thanos.global.objstore.secretName` | `thanos-objstore-config` | Secret holding the object-store config. |
| `thanos.global.objstore.secretKey` | `objstore.yml` | Key within that Secret. |

The Secret holds a Thanos `objstore.yml` — a `type:` plus a provider `config:`.
Create it in the namespace Thanos runs in:

```bash
kubectl create secret generic thanos-objstore-config \
  --namespace monitoring \
  --from-file=objstore.yml=./objstore.yml
```

> [!INFO]
>   Prefer cloud **workload identity** (IRSA on AWS, Workload Identity on GKE, Azure Workload ID) over long-lived keys in `objstore.yml`.
>   Omit the credential fields from the config and annotate the Thanos ServiceAccount instead — no static secrets in the cluster.

### Granting object-storage access (workload identity)

Annotate the Thanos ServiceAccount through `thanos.global.serviceAccount.annotations` (shared by receive, store gateway, and compactor), and leave the credential fields out of `objstore.yml` so the SDK uses the ambient identity.
The Thanos ServiceAccount is `thanos-thanos` (a deterministic `fullnameOverride`), in the release namespace (recommended `monitoring`); the split-namespace profile places it in a dedicated `thanos` namespace instead. Scope the binding to that exact namespace/ServiceAccount.

{{< tabs >}}
{{% tab "AWS · EKS (IRSA)" %}}
**IRSA** (IAM Roles for Service Accounts). Chain: the Thanos ServiceAccount is annotated with a role ARN → EKS projects an OIDC token → the SDK calls **STS `AssumeRoleWithWebIdentity`** → temporary credentials → **S3**. Requires the cluster's **OIDC provider** registered in IAM (one-time).

A ready-made starting point lives at `charts/materialize-monitoring/profiles/aws-example.values.yaml`.

*Trust policy* — scope `:sub` to the **Thanos namespace and ServiceAccount**, not another workload's:

```json
{
  "Effect": "Allow",
  "Principal": { "Federated": "arn:aws:iam::<account-id>:oidc-provider/oidc.eks.<region>.amazonaws.com/id/<oidc-id>" },
  "Action": "sts:AssumeRoleWithWebIdentity",
  "Condition": { "StringEquals": {
    "oidc.eks.<region>.amazonaws.com/id/<oidc-id>:aud": "sts.amazonaws.com",
    "oidc.eks.<region>.amazonaws.com/id/<oidc-id>:sub": "system:serviceaccount:monitoring:thanos-thanos"
  }}
}
```

> [!INFO]
>   The default assumes the release is installed into `monitoring`.
>   Under [split namespaces](../../operating/production-best-practices/stack-wide/#namespace-layout), the `:sub` is `system:serviceaccount:thanos:thanos-thanos` instead.

*Permissions policy* — least-privilege to the single bucket. `DeleteObject` is required (the compactor rewrites and deletes blocks during compaction/downsampling):

```json
{
  "Statement": [
    { "Effect": "Allow", "Action": ["s3:ListBucket", "s3:GetBucketLocation"], "Resource": "arn:aws:s3:::<bucket>" },
    { "Effect": "Allow", "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"], "Resource": "arn:aws:s3:::<bucket>/*" }
  ]
}
```

*ServiceAccount* — annotate via chart values; the EKS webhook injects `AWS_ROLE_ARN` / `AWS_WEB_IDENTITY_TOKEN_FILE`:

```yaml
thanos:
  global:
    serviceAccount:
      annotations:
        eks.amazonaws.com/role-arn: arn:aws:iam::<account-id>:role/<thanos-role>
```

*`objstore.yml`* — no `access_key`/`secret_key`, so the default chain uses the IRSA token:

```yaml
type: S3
config:
  bucket: <bucket>
  endpoint: s3.<region>.amazonaws.com
  region: <region>
```
{{% /tab %}}
{{% tab "GCP · GKE (Workload Identity)" %}}
**GKE Workload Identity.** Chain: the Thanos ServiceAccount is annotated with a Google service account (GSA) → GKE exchanges the pod token for that GSA's credentials → **GCS**. Requires Workload Identity enabled on the cluster and node pool. Below, `<gsa>` is the GSA; `[<namespace>/thanos-thanos]` is the Kubernetes ServiceAccount (KSA).

1. Grant the GSA object access on the bucket (`roles/storage.objectAdmin`).
2. Bind the GSA's IAM policy so the Thanos KSA may impersonate it — the KSA **must match Thanos's namespace/ServiceAccount**:

   ```bash
   gcloud iam service-accounts add-iam-policy-binding <gsa>@<project>.iam.gserviceaccount.com \
     --role="roles/iam.workloadIdentityUser" \
     --member="serviceAccount:<project>.svc.id.goog[monitoring/thanos-thanos]"
   ```

   > [!INFO]
   >   The default assumes the release is installed into `monitoring`.
   >   Under [split namespaces](../../operating/production-best-practices/stack-wide/#namespace-layout), use `--member="serviceAccount:<project>.svc.id.goog[thanos/thanos-thanos]"` here.

3. Annotate the ServiceAccount:

   ```yaml
   thanos:
     global:
       serviceAccount:
         annotations:
           iam.gke.io/gcp-service-account: <gsa>@<project>.iam.gserviceaccount.com
   ```

*`objstore.yml`* — no `service_account` key, so ambient Workload Identity credentials are used:

```yaml
type: GCS
config:
  bucket: <bucket>
```
{{% /tab %}}
{{% tab "Azure · AKS (Workload ID)" %}}
**Microsoft Entra Workload ID.** Chain: the Thanos ServiceAccount is annotated with a managed-identity client ID → AKS projects a token → exchanged with Entra for the identity's credentials → **Azure Blob**. Requires the OIDC issuer + workload identity enabled on the cluster.

1. Grant the user-assigned managed identity **`Storage Blob Data Contributor`** on the storage account (or container scope).
2. Create a **federated identity credential** on that identity — subject **must match Thanos's namespace/ServiceAccount** (`system:serviceaccount:monitoring:thanos-thanos`), audience `api://AzureADTokenExchange`.

   > [!INFO]
   >   The default assumes the release is installed into `monitoring`.
   >   Under [split namespaces](../../operating/production-best-practices/stack-wide/#namespace-layout), the subject is `system:serviceaccount:thanos:thanos-thanos` instead.

3. Annotate the ServiceAccount and label the pods so the webhook injects the token:

   ```yaml
   thanos:
     global:
       serviceAccount:
         annotations:
           azure.workload.identity/client-id: <client-id>
   ```

*`objstore.yml`* — see the [Thanos Azure config](https://thanos.io/tip/thanos/storage.md/#azure) for the exact keys (`storage_account`, `container`); omit the shared key so the workload identity is used.
{{% /tab %}}
{{< /tabs >}}

> [!INFO]
>   The token exchange and the object store are both 443 hops to your cloud's identity and storage endpoints. If a Thanos NetworkPolicy is enabled you must allow that egress, or the credential fetch hangs the component at startup.

> [!NOTE]
>   **Verifying.** Split the two failure modes: a `403`/AccessDenied during the **token exchange** (`AssumeRoleWithWebIdentity` or the GCP/Azure equivalent) is a **binding/trust-scope** problem — usually a namespace/ServiceAccount subject mismatch; an authorization error on the **bucket operation itself**, after the exchange succeeds, is a **permissions** problem on the bucket. These are the same mechanics as [Loki's object store](../../logs-and-events/storing/#granting-object-storage-access-workload-identity).

### Retention and downsampling

The Thanos **Compactor** compacts raw blocks and produces downsampled resolutions, each with independent retention (`thanos.compactor.retention`):

| Resolution | Default retention |
|---|---|
| raw | `30d` |
| 5m | `90d` |
| 1h | `365d` |

Downsampling keeps long-range queries cheap: a year-wide query reads 1h blocks, not raw samples. Tune these to trade storage cost against how far back high-resolution data stays available.

Each Receive replica uploads its own copy of every block it holds, labelled with `receive_replica`.
The Compactor deduplicates on that label (`--deduplication.replica-label=receive_replica`), so compacted blocks hold one copy of each series rather than one per replica.
The same flag enables vertical compaction, which merges the short overlaps Receive writes across a restart instead of halting on them.
See [Retention & compaction](../../operating/production-best-practices/thanos/#thanos-retention-compaction) for the upgrade caveats.

### Components

The bundled Thanos runs as a small set of roles over the shared bucket:

- **Receive** — the remote-write endpoint `alloy-gateway` writes to; buffers recent data and uploads TSDB blocks to object storage.
- **Store Gateway** — serves historical blocks *from* object storage for queries.
- **Compactor** — a **singleton** that compacts and downsamples blocks in the bucket (owns retention).
- **Query** — federates recent data (Receive) and historical data (Store Gateway) behind one PromQL endpoint.

The **Ruler** is on by default and evaluates PromQL rules.
`queryFrontend` is off by default and enabled by the `thanos-large` profile.
See [Metrics Architecture](../architecture/) for how the roles connect.

## Other Metric Storage Backends

Two families of backend sit alongside the bundled Thanos, and both fan out *in addition to* it rather than replacing it.

**More remote-write stores** — Amazon Managed Prometheus, Mimir, Grafana Cloud, another Thanos — are added as further [`prometheus` entries](#the-remote-write-destinations).
Nothing on this page is an either/or with the in-cluster Thanos: keeping full fidelity locally while shipping a filtered slice to a metered backend is the shape the map exists for.

**OpenTelemetry (OTLP) backends** are entries in `pipeline.destinations`, the map that also holds the gateway's log destinations.
Each entry is one connection with its own credentials, metric tier, and queue, and it can carry logs as well as metrics.
The following types send metrics:

`otlp`
: Any OTLP endpoint, such as Honeycomb, Grafana Cloud, or your own OpenTelemetry Collector.

`googleCloud`
: Google Cloud Managed Service for Prometheus, through the Telemetry API.

`datadog`
: Datadog, through its own exporter.

A metrics-carrying entry gets its own tier filter, set by `metrics.minMetricImportance`, so each backend receives only the tiers you choose.
For the full list of fields, see [`pipeline.destinations` in the values reference](../../reference/helm/materialize-monitoring-values/#destinations).

> [!NOTE]
>   The exporters under `pipeline.metrics.gateway.destination.otel` are deprecated.
>   They still render as before, and the render warns while any is enabled.
>   See [Move off the deprecated `otel` block](#otel-migration).

### Generic OTLP (Honeycomb, Grafana Cloud, collectors) {#otlp}

An `otlp` entry sends to any OTLP-compatible endpoint:

```yaml
pipeline:
  destinations:
    collector:
      type: otlp
      url: collector.example.com:4317
      protocol: grpc
      tls:
        enabled: true
      metrics:
        minMetricImportance: recommended
```

Over gRPC, `url` is a `host:port`.
Over `protocol: http`, the default, `url` is a base URL such as `https://otlp.example.com`, and the exporter appends `/v1/metrics`.
Turn `tls.enabled` on for any endpoint that serves TLS; with it off, the gRPC exporter connects in plaintext.

Set `authType` to authenticate:

| `authType` | Credentials |
|---|---|
| `basicAuth` | `basicAuth.username` and `basicAuth.password`, or the variables `basicAuth.usernameEnv` and `basicAuth.passwordEnv` name |
| `bearer` | `bearer.token`, or the variable `bearer.tokenEnv` names |
| `headers` | `headers`, a list of request headers |
| `sigv4` | The gateway's AWS identity, with `sigv4.region`, `sigv4.service`, and an optional `sigv4.roleArn` to assume |

The credential values come from the gateway Secret.
For the variable names, see [Supplying credentials](#supplying-credentials-the-gateway-secret).

#### API-key headers {#otlp-headers}

Several OTLP vendors authenticate with a custom request header rather than a bearer token, such as Honeycomb's `x-honeycomb-team`.
`authType: headers` covers that case:

```yaml
pipeline:
  destinations:
    honeycomb:
      type: otlp
      url: api.honeycomb.io:443
      protocol: grpc
      tls:
        enabled: true
      metrics: {}
      authType: headers
      headers:
        - key: x-honeycomb-team
          valueEnv: GATEWAY_HONEYCOMB_API_KEY
        - key: x-honeycomb-dataset
          value: mzmon
```

Each header sets exactly one of `value` or `valueEnv`.
A `value` renders into the gateway's pipeline ConfigMap in plaintext, so keep it for non-secret routing headers such as a dataset or tenant name.
A `valueEnv` names an environment variable the gateway reads at startup, which is where a credential belongs.
The variable name is yours to pick, so name it after the backend.

The chart checks the shape at render time.
An empty header list, a header with no `key`, a header that sets both `value` and `valueEnv` or neither, and a `valueEnv` that no `extraEnv` or `envFrom` source could supply all fail the install.

For a ready-made starting point, copy the annotated [`otlp-metrics-honeycomb.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/otlp-metrics-honeycomb.values.yaml) profile.

### Google Cloud Monitoring (GCM) {#gcm}

A `googleCloud` entry writes OTLP to Google's [Telemetry API](https://docs.cloud.google.com/stackdriver/docs/otlp/overview) (`telemetry.googleapis.com`).
The Telemetry API stores the series in Cloud Monitoring as Google Cloud Managed Service for Prometheus metrics, under `prometheus.googleapis.com/`, where PromQL reads them.

```yaml
pipeline:
  destinations:
    googleCloud:
      type: googleCloud
      metrics:
        minMetricImportance: recommended
```

The project needs the `telemetry.googleapis.com` API enabled.
Authentication uses **Workload Identity**: the `alloy-gateway` ServiceAccount is annotated with a Google service account, and the gateway takes that ambient identity with no credentials in the config.
The token exchange works the same way as for [Thanos on GCS](#granting-object-storage-access-workload-identity), but on the gateway's ServiceAccount rather than Thanos's.
That identity needs `roles/monitoring.metricWriter` on the project.

#### What it costs

Samples ingested into `prometheus.googleapis.com/` are billed per sample.
The `workload.googleapis.com/` custom metrics it wrote before v0.31.0 are billed per byte, at 8 bytes a point.

| Domain | Billed by | List price | Measured at `recommended` |
|---|---|---|---|
| `workload.googleapis.com/` (`otelcol.exporter.googlecloud` before v0.31.0) | MiB ingested | $0.26 per MiB, about $2 per million points | 307 MiB a day across 212 metric types, about $2,400 a month |
| `prometheus.googleapis.com/` (OTLP to the Telemetry API) | Samples ingested | $0.06 per million samples | The same points as samples, about $75 a month |

A histogram point is billed as two samples plus one per non-empty bucket, so a histogram costs no more than its buckets did as separate points.
An untyped series is written twice, as `/unknown` and `/unknown:counter`, and each scrape target adds a `target_info` series.
Both are small next to the total.

<!--
Agent note: the measured row is from the heather-mzmon GCP test install (internal), read from
monitoring.googleapis.com/billing/bytes_ingested in September 2026 (DEP-331). The $75 a month prices
the same point count as samples; re-measure from billing/samples_ingested once the OTLP export has run
there for a day.
-->

#### What arrives

The metric type carries the kind of point: `prometheus.googleapis.com/<name>/gauge`, `/counter`, `/histogram` or `/summary`.
PromQL takes the bare name, as it does against Thanos, and a histogram answers to `<name>_bucket`, `<name>_count` and `<name>_sum`.
The kind comes from each target's `# TYPE` line, because every gateway scrape sets `honor_metadata`.
Pushed remote-write carries no types, so it arrives untyped.

Cloud Monitoring files every series under a `prometheus_target` resource.

| Label | Value |
|---|---|
| `project_id` | The project the gateway runs in, or the destination's `project` |
| `location` | The GKE cluster's region or zone, or the destination's `location` |
| `cluster` | The series' own `cluster` label, or else `CLUSTER_NAME`, which is the `cluster` every other destination carries |
| `namespace` | The series' own `namespace` label, as in Thanos |
| `job`, `instance` | The scrape target's |

A series label named `location`, `project_id`, `job` or `instance` keeps its value as `exported_<label>`.
Every series also carries `collected_by="materialize-monitoring"`.
That label separates these series from other Prometheus data in the project: GKE's managed kube-state-metrics writes the same metric types under the same `job`.

Counters and histograms are cumulative, and Cloud Monitoring requires a start time on each point, which a Prometheus scrape does not supply.
The gateway takes each series' first point as its baseline and does not send it.
A counter's raw value in Cloud Monitoring therefore differs from Thanos, while `rate()` and `increase()` agree.

Off GKE there is no metadata server to supply the project and the location, and the Telemetry API refuses a point without a location.
The destination's `project` and `location` supply them.

Because the destination is metered, it defaults to `minMetricImportance: recommended`.
`essential` writes less; a lower floor keeps more there.

### Datadog {#datadog}

A `datadog` entry writes metrics, logs, or both to Datadog:

```yaml
pipeline:
  destinations:
    datadog:
      type: datadog
      site: datadoghq.eu
      metrics:
        minMetricImportance: recommended
```

Set `site` to your Datadog site, such as `datadoghq.com` or `datadoghq.eu`.
The exporter derives its intake endpoints from the site, and `metrics.url` or `logs.url` overrides one for a proxy or PrivateLink.
The API key is read from `GATEWAY_<NAME>_DEST_API_KEY`, `GATEWAY_DATADOG_DEST_API_KEY` for the entry above.
Supply it from the gateway Secret, or name another variable in `apiKey.keyEnv`.
Like Google Cloud, it defaults to `minMetricImportance: recommended`.
To send logs too, add a `logs` block.

The [`otel-metrics-fanout.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/otel-metrics-fanout.values.yaml) profile shows Datadog and Google Cloud together, each on its own tier.

### Move off the deprecated `otel` block {#otel-migration}

The exporters under `pipeline.metrics.gateway.destination.otel` still render as before.
To move each one to `pipeline.destinations`, use the following table:

| Deprecated | Replacement |
|---|---|
| `otlpExporter` | An entry of `type: otlp`, with a `metrics` block. Its `url` was a bare `host:port`; over `protocol: http`, add the scheme. Set `tls.enabled: true` for a TLS endpoint |
| `googleCloudExporter` | An entry of `type: googleCloud`. `endpoint` becomes `url` |
| `datadogExporter` | An entry of `type: datadog`. `url` becomes `site`, and `metricEndpoint` and `logsEndpoint` become `metrics.url` and `logs.url`. Set `apiKey.keyEnv: GATEWAY_OTEL_DEST_DATADOG_API_KEY` to keep reading the same variable |
| `otel.enabled` | A `metrics` block on each entry |
| `pipeline.logging.gateway.destination.otel.enabled` | A `logs` block on each `otlp` or `datadog` entry |
| `otel.auth` | The entry's own `authType`. `awsSigv4` becomes `sigv4`, and the `headers.headers` list becomes `headers` |

For the deprecated `prometheusRemoteWrite` map, see [Upgrading from `prometheusRemoteWrite`](#upgrading-from-prometheusremotewrite).

The raw `config`, `handlers`, and `auth.custom` overrides have no equivalent in `pipeline.destinations`.
A destination that relies on one can stay in the `otel` block, which is removed only in a major version.

### Amazon Managed Prometheus (SigV4 + IRSA)

To push to Amazon Managed Prometheus (AMP), sign requests with SigV4 and let the gateway pod assume an IAM role via **IRSA** — no static keys in the cluster.

AMP is a remote-write destination like any other, so this **adds** to the bundled Thanos rather than replacing it.
Running both is the usual choice: AMP bills per sample ingested and per active series, so the pairing below keeps everything in local storage you already pay for and sends only the alerting metrics to AMP.
To send metrics *only* to AMP, set `enabled: false` on the `thanos` destination and on the bundled backend.

1. Add a destination pointing at your workspace's remote-write URL, with `sigv4`:

   ```yaml
   pipeline:
     destinations:
       thanos:
         metrics:
           minMetricImportance: all
       amp:
         type: prometheus
         url: https://aps-workspaces.<region>.amazonaws.com/workspaces/<workspace-id>/api/v1/remote_write
         metrics:
           minMetricImportance: essential
         authType: sigv4
         sigv4:
           region: <region>
           # roleArn: optional — only to assume a *different* role than IRSA grants
   ```

2. Grant an IAM role `aps:RemoteWrite` on the workspace, and bind it to the gateway with IRSA by annotating the `alloy-gateway` ServiceAccount:

   ```yaml
   alloy-gateway:
     serviceAccount:
       annotations:
         eks.amazonaws.com/role-arn: arn:aws:iam::<account-id>:role/<gateway-role>
   ```

With `sigv4` set (region only), the AWS SDK's default credential chain picks up the IRSA web-identity token the EKS webhook injects (`AWS_ROLE_ARN` / `AWS_WEB_IDENTITY_TOKEN_FILE`) — you never set access keys.
`roleArn` is only for chaining to a *different* role (STS `AssumeRole`) beyond what IRSA already grants.

> [!INFO]
>   IRSA requires the cluster's OIDC provider registered in IAM and the role's trust policy scoped to the gateway's namespace/ServiceAccount.
>   Those mechanics — and the failure modes (a `403 AccessDenied` on `AssumeRoleWithWebIdentity` is a trust-scope problem; an authz error on the write itself is a permissions problem) — are the same as the Loki object-store setup: see [Logs &amp; Events &gt; Storing](../../logs-and-events/storing/#granting-object-storage-access-workload-identity).

> [!NOTE]
>   If the gateway NetworkPolicy is enabled, allow egress (443) to the AMP endpoint and to AWS STS, or the credential fetch and the write will fail.

[`aws-amp-fanout.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/aws-amp-fanout.values.yaml) is this whole setup as one annotated profile.

## See more

- [Logs &amp; Events &gt; Storing](../../logs-and-events/storing/) — the Loki object-storage analog, with more depth on workload identity, retention, and disaster recovery.
- [Collecting](../collecting/) — how metrics arrive before they are stored.
- [Scraping](../scraping/) — configuring ServiceMonitors / PodMonitors on the scrape side.
- [Thanos storage](https://thanos.io/tip/thanos/storage.md/) and [AMP: ingest metrics](https://docs.aws.amazon.com/prometheus/latest/userguide/AMP-onboard-ingest-metrics.html) (official).
