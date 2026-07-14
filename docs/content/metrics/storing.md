---
title: "Storing"
weight: 20
---

# Storing Metrics

Metrics collected by `alloy-gateway` are forwarded to an **OTLP** or **Prometheus remote-write** backend.
Out of the box that is an in-cluster **Thanos Receive**; you can point it at any remote-write-compatible store (Thanos, Mimir, Amazon Managed Prometheus, Grafana Cloud, …) through chart values.
The destination values below live under `pipeline.metrics.gateway.destination.prometheusRemoteWrite`.
If you keep the bundled Thanos, it in turn persists blocks to object storage — see [Thanos object storage](#thanos-object-storage) for the S3/GCS/Azure setup.

## The remote-write destination

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
- **`sigv4`** — AWS SigV4 signing (below).

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

### Google Cloud Monitoring (GCM) {#gcm}

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
