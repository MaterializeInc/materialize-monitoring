---
title: "Storing"
weight: 20
---

# Storing Metrics

`alloy-gateway` forwards the metrics it collects to one or more storage backends.
Out of the box that is an in-cluster **Thanos Receive**, but you can send metrics to any Prometheus remote-write store (Thanos, Mimir, Amazon Managed Prometheus, Grafana Cloud, …), to an **OpenTelemetry (OTLP)** endpoint, or to several at once — each enabled destination receives its own copy.

This page walks through the three decisions that setup involves:

- **Where** metrics go — the default [remote-write destination](#the-remote-write-destination) and the [other backends](#other-metric-storage-backends) (generic OTLP, Google Cloud Monitoring, Datadog, Amazon Managed Prometheus).
- **How** the gateway authenticates — [authentication](#authentication) and the [gateway Secret](#supplying-credentials-the-gateway-secret) that holds the credentials.
- **What** each backend stores — the [importance tiers and denylist](#controlling-what-each-destination-stores) that decide which metrics reach each destination.

If you keep the bundled Thanos, it persists blocks to object storage — see [Thanos object storage](#thanos-object-storage) for the S3/GCS/Azure setup.

## The remote-write destination

These values live under `pipeline.metrics.gateway.destination.prometheusRemoteWrite`:

| Value | Purpose |
|---|---|
| `enabled` | Toggle the metrics remote-write sink (default `true`). |
| `url` | Remote-write endpoint (default: in-cluster Thanos Receive). |
| `authType` | `none` (default), `basicAuth`, `bearer`, or `sigv4`. |
| `sigv4` | AWS SigV4 signing config — for Amazon Managed Prometheus (see below). |

Point it at an external store:

```yaml
pipeline:
  metrics:
    gateway:
      destination:
        prometheusRemoteWrite:
          enabled: true
          url: https://<your-remote-write-endpoint>/api/v1/write
```

Every sample carries a `cluster` label identifying its source cluster, set from the `CLUSTER_NAME` environment variable (default `default`).
Set it per install so series from different clusters stay distinct once they land in a shared backend:

```yaml
env:
  CLUSTER_NAME: prod-us-east-1
```

## Authentication

Credentials are supplied through environment variables (not inline in values), so they can be sourced from a mounted Secret.
Set `authType` and fill in the matching block:

- **`none`** — no auth (the in-cluster Thanos default).
- **`basicAuth`** — username / password from env.
- **`bearer`** — bearer token from env.
- **`oauth2`** — OAuth2 client credentials from env.
- **`sigv4`** — AWS SigV4 signing, derived from IRSA (see [Amazon Managed Prometheus](#amazon-managed-prometheus-sigv4--irsa) below).

The OpenTelemetry destinations have their own auth block, `destination.otel.auth`, whose `authType` is one of `none`, `basic`, `bearer`, `headers`, `awsSigv4`, or `custom`.

Whichever destination and method you choose, the gateway reads the actual secret from an environment variable at runtime (`sys.env(...)`).
There are two ways to populate that variable:

- **Inline in `values`** — the Prometheus remote-write and Loki destinations accept the credential directly (`basicAuth.password`, `bearer.token`, …). When set, the value is baked into the gateway ConfigMap in plaintext, so this is convenient for a quick start but weaker for production.
- **From a Secret** — leave the inline field blank and provide the same env var through a Secret. This is the preferred path, and the only option for the OpenTelemetry and Datadog destinations, which have no inline field.

Both are described next.

### Supplying credentials (the gateway Secret)

The gateway loads its environment from two objects that share one name — `mzmon-alloy-gateway-env` (that is `<fullnameOverride>-alloy-gateway-env`, `mzmon` by default) — in the release namespace:

- a **ConfigMap** the chart generates — non-secret env such as the per-destination allowlists and tenant map, plus any credentials you chose to set inline in `values`;
- a **Secret** *you* create — the credential values in the table below, kept out of the rendered manifests.

The chart does **not** create the Secret; it is mounted `optional: true`, so you populate it out-of-band with only the keys your enabled destinations need.
A bearer-authenticated OTLP endpoint plus a Datadog API key — the pairing in the reference `mzmon-gcp` install — is just two keys:

```bash
kubectl create secret generic mzmon-alloy-gateway-env \
  --namespace monitoring \
  --from-literal=GATEWAY_OTEL_DEST_BEARER_TOKEN='<token>' \
  --from-literal=GATEWAY_OTEL_DEST_DATADOG_API_KEY='<api-key>'
```

The Secret's keys are injected as environment variables next to the ConfigMap's, so the key names **are** the env-var names in the table below.

> [!WARNING]
>   The Secret name must match the release: with the default `fullnameOverride: mzmon` it is `mzmon-alloy-gateway-env`, and it must live in the namespace the gateway runs in (`monitoring` above).
>   Because the mount is optional, a mismatched name or namespace is silently ignored — the destination then authenticates with empty credentials instead of failing loudly.

> [!INFO]
>   `kubectl create secret` is fine for a first install, but in production source the Secret from Sealed Secrets, External Secrets, or SOPS rather than committing raw credentials.

### Credential environment variables

Populate only the rows for the destinations and auth methods you enable:

| Destination · method | `values` block | Secret keys (env vars) |
|---|---|---|
| Prometheus remote-write · `basicAuth` | `…destination.prometheusRemoteWrite.basicAuth` | `GATEWAY_PROMETHEUS_DEST_USERNAME`, `GATEWAY_PROMETHEUS_DEST_PASSWORD` |
| Prometheus remote-write · `bearer` | `…prometheusRemoteWrite.bearer` | `GATEWAY_PROMETHEUS_DEST_BEARER_TOKEN` |
| Prometheus remote-write · `oauth2` | `…prometheusRemoteWrite.oauth2` | `GATEWAY_PROMETHEUS_DEST_OAUTH2_CLIENT_ID`, `…_CLIENT_SECRET`, `…_TOKEN_URL` |
| Prometheus remote-write · client TLS | `…prometheusRemoteWrite.tls` | `GATEWAY_PROMETHEUS_DEST_TLS_CA`, `…_TLS_CERT`, `…_TLS_KEY` |
| OTLP · `basic` | `…otel.auth.basic` | `GATEWAY_OTEL_DEST_USERNAME`, `GATEWAY_OTEL_DEST_PASSWORD` |
| OTLP · `bearer` | `…otel.auth.bearer` | `GATEWAY_OTEL_DEST_BEARER_TOKEN` |
| OTLP · `headers` | `…otel.auth.headers` | whatever each header's `valueEnv` names — you choose |
| Datadog | `…otel.datadogExporter` | `GATEWAY_OTEL_DEST_DATADOG_API_KEY` |
| SigV4 — AMP or OTLP `awsSigv4` | `…prometheusRemoteWrite.sigv4` / `…otel.auth.awsSigv4` | — none; uses the pod's IRSA identity |

SigV4 and the cloud-native exporters (GCM via Workload Identity) carry no secret keys — they authenticate with the gateway pod's ambient cloud identity, so those rows stay out of the Secret entirely.

## Controlling what each destination stores

Not every backend should receive every metric.
`alloy-gateway` gives you two independent controls: a global **denylist** that drops metrics before they reach any destination, and a per-destination **importance filter** that keeps only metrics at or above a chosen tier.

### The importance tiers

Every metric in the registry is classified by *importance* — how likely you are to want it — independent of which backend you use.
The levels, from most to least important, are **essential**, **recommended**, **extended**, and **diagnostic**.
A fifth value, **all**, is a firehose meaning "everything scraped, including metrics the registry has not classified."
The tier definitions and the full membership of each tier live in [the metric list](../../reference/stable-metrics/list-metrics/).

<!-- The tier *definitions* live in reference/stable-metrics/list-metrics.md; this page owns the config/operational angle only. Keep them from drifting. -->

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
| `prometheusRemoteWrite` (bundled Thanos) | `all` |
| `otlpExporter` (generic OTLP) | `all` |
| `googleCloudExporter` (GCM) | `recommended` |
| `datadogExporter` | `recommended` |

Set it per destination:

```yaml
pipeline:
  metrics:
    gateway:
      destination:
        otel:
          datadogExporter:
            minMetricImportance: essential   # ship only alerting metrics to Datadog
```

> [!NOTE]
>   The **extended** and **diagnostic** tiers are still being populated — today they are empty.
>   Until they fill in, `extended` and `diagnostic` resolve to the same set as `recommended`.
>   If you want *everything* that is scraped, use `all`, not `diagnostic`.

For a worked example that keeps full fidelity in Thanos while sending a smaller, cheaper slice to Google Cloud Monitoring and Datadog, see the annotated [`otel-metrics-fanout.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/otel-metrics-fanout.values.yaml) profile.

### How the allowlist is built

Tier membership is generated from the query registry into `charts/materialize-monitoring/pre-rendered/metrics/metric-tiers.yaml` (via `mz-monitoring-build gen-metric-tiers`, or `make metric-tiers`).
At render time the chart reads that file, unions the tiers at or above each destination's `minMetricImportance`, and hands the gateway the result as an allowlist regex — one environment variable per destination, such as `GATEWAY_UNFILTERED_PROM_METRICS`.
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

### Through Terraform

The `materialize-monitoring` module exposes three destinations directly, each taking the same `min_importance` tier documented above:

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
>   Under [split namespaces](../../operating/production-best-practices/#namespace-layout), the `:sub` is `system:serviceaccount:thanos:thanos-thanos` instead.

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
   >   Under [split namespaces](../../operating/production-best-practices/#namespace-layout), use `--member="serviceAccount:<project>.svc.id.goog[thanos/thanos-thanos]"` here.

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
   >   Under [split namespaces](../../operating/production-best-practices/#namespace-layout), the subject is `system:serviceaccount:thanos:thanos-thanos` instead.

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

### Components

The bundled Thanos runs as a small set of roles over the shared bucket:

- **Receive** — the remote-write endpoint `alloy-gateway` writes to; buffers recent data and uploads TSDB blocks to object storage.
- **Store Gateway** — serves historical blocks *from* object storage for queries.
- **Compactor** — a **singleton** that compacts and downsamples blocks in the bucket (owns retention).
- **Query** — federates recent data (Receive) and historical data (Store Gateway) behind one PromQL endpoint.

`queryFrontend` and `ruler` are available but off by default (`thanos.queryFrontend` / `thanos.ruler`).

## Other Metric Storage Backends

Two families of backend sit alongside the default Thanos remote-write sink: **OpenTelemetry (OTLP)** destinations, configured under the `otel` block, and remote-write variants such as **Amazon Managed Prometheus**, which reuse `prometheusRemoteWrite` with SigV4 auth.

Enable the OTLP path with `pipeline.metrics.gateway.destination.otel.enabled: true`, then turn on one or more exporters — `otlpExporter` (generic), `googleCloudExporter`, and `datadogExporter` can all run at once, each with its own `minMetricImportance`.

> [!WARNING]
>   The `otel` block is shared with the logs pipeline.
>   If you enable the OTLP *logs* destination, this block is still used for exporter configuration even when `otel.enabled` is `false` for metrics.

### Generic OTLP (Honeycomb, Grafana Cloud, collectors) {#otlp}

`otlpExporter` is the generic OTLP push exporter — point it at any OTLP-compatible endpoint, whether a vendor (Honeycomb, Grafana Cloud) or your own OpenTelemetry Collector:

```yaml
pipeline:
  metrics:
    gateway:
      destination:
        otel:
          enabled: true
          otlpExporter:
            enabled: true
            url: <host>:4317        # host[:port], no http:// or https:// prefix
            protocol: grpc          # grpc → otlp, http → otlphttp
            compression: gzip       # gzip for compatibility, snappy for speed
          auth:
            authType: bearer        # none | basic | bearer | headers | awsSigv4 | custom
```

`url` takes a `host[:port]` with no scheme; `protocol: grpc` selects the OTLP/gRPC exporter and `protocol: http` selects OTLP/HTTP.
Authentication is configured once under `otel.auth` (shared by the OTLP exporter): pick `authType` and fill the matching block.
The credential values themselves come from the gateway Secret — see [Supplying credentials](#supplying-credentials-the-gateway-secret) for the env-var keys.

#### API-key headers {#otlp-headers}

Several OTLP vendors authenticate with a custom request header rather than a bearer token — Honeycomb's `x-honeycomb-team`, for one.
`authType: headers` covers that case without dropping to raw Alloy config:

```yaml
          auth:
            authType: headers
            headers:
              headers:
                - key: x-honeycomb-team
                  valueEnv: GATEWAY_OTEL_DEST_HONEYCOMB_API_KEY
                - key: x-honeycomb-dataset
                  value: mzmon
```

Each header sets exactly one of `value` or `valueEnv`.
`value` renders into the gateway's pipeline ConfigMap in plaintext, so keep it for non-secret routing headers such as a dataset or tenant name; `valueEnv` names an environment variable the gateway reads at startup, which is where a credential belongs.
The variable name is yours to pick — nothing else in the chart depends on it — so name it after the backend rather than reusing another destination's key.

The chart checks the shape at render time: an empty header list, a header missing its `key`, a header setting both `value` and `valueEnv` or neither, and a `valueEnv` no `extraEnv` or `envFrom` source could supply all fail the install rather than authenticating with an empty header at run time.

For a ready-made starting point — generic OTLP to Honeycomb, including the header auth above — copy the annotated [`otlp-metrics-honeycomb.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/otlp-metrics-honeycomb.values.yaml) profile.

### Google Cloud Monitoring (GCM) {#gcm}

`googleCloudExporter` writes to Google Cloud Monitoring under the metric prefix `workload.googleapis.com/mzmon`:

```yaml
pipeline:
  metrics:
    gateway:
      destination:
        otel:
          enabled: true
          googleCloudExporter:
            enabled: true
            minMetricImportance: recommended
```

Authentication uses **Workload Identity** — annotate the `alloy-gateway` ServiceAccount with the target Google service account and leave credentials out of the config, so the SDK's default chain uses the ambient identity.
The token-exchange mechanics are the same as [Thanos on GCS](#granting-object-storage-access-workload-identity) above, but on the gateway's ServiceAccount rather than Thanos's.
Grant that identity `roles/monitoring.metricWriter` on the project.
GCM supports only `gzip` compression.

Because GCM is metered, it defaults to `minMetricImportance: recommended`; raise it to `essential` to write even less, or lower the floor if you want more history there.

### Datadog {#datadog}

`datadogExporter` writes metrics (and logs) to Datadog:

```yaml
pipeline:
  metrics:
    gateway:
      destination:
        otel:
          enabled: true
          datadogExporter:
            enabled: true
            url: datadoghq.com          # your Datadog site
            minMetricImportance: recommended
```

The API key is read from the `GATEWAY_OTEL_DEST_DATADOG_API_KEY` environment variable — source it from a Secret; never inline it in values.
Set `url` to your Datadog site (for example `datadoghq.com` or `datadoghq.eu`); `metricEndpoint` and `logsEndpoint` default to the matching intake URLs.
Like GCM, it defaults to `minMetricImportance: recommended`.
The [`otel-metrics-fanout.values.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/otel-metrics-fanout.values.yaml) profile shows Datadog and GCM enabled together, each on its own tier.

### Amazon Managed Prometheus (SigV4 + IRSA)

To push to Amazon Managed Prometheus (AMP), sign requests with SigV4 and let the gateway pod assume an IAM role via **IRSA** — no static keys in the cluster.

1. Point the destination at your workspace's remote-write URL and enable `sigv4`:

   ```yaml
   pipeline:
     metrics:
       gateway:
         destination:
           prometheusRemoteWrite:
             url: https://aps-workspaces.<region>.amazonaws.com/workspaces/<workspace-id>/api/v1/remote_write
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

## See more

- [Logs &amp; Events &gt; Storing](../../logs-and-events/storing/) — the Loki object-storage analog, with more depth on workload identity, retention, and disaster recovery.
- [Collecting](../collecting/) — how metrics arrive before they are stored.
- [Scraping](../scraping/) — configuring ServiceMonitors / PodMonitors on the scrape side.
- [Thanos storage](https://thanos.io/tip/thanos/storage.md/) and [AMP: ingest metrics](https://docs.aws.amazon.com/prometheus/latest/userguide/AMP-onboard-ingest-metrics.html) (official).
