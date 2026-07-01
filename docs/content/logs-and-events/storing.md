---
title: "Storing"
weight: 20
---

# Storing Logs

Loki keeps all durable log data in a single [object storage](../#storage) backend, maintained by the [Loki Compactor](../#backend).
This page covers the storage layout, the index, retention, and disaster recovery.
See the [logging architecture](../) for how storage relates to the ingesters that write it and the queriers that read it.

## Object storage

Loki is a black box over an S3-style object store.
The supported backends are **S3-compatible** storage (AWS S3, MinIO, Ceph, R2, …), **Google Cloud Storage**, and **Azure Blob Storage**.
For integration testing, Loki instead uses a local **filesystem** store in single-binary mode — no object storage required.

A single bucket holds everything, separated by prefix:

| Prefix | Contents |
|---|---|
| `/loki/chunks` | compressed log chunks and the TSDB index |
| `/loki/ruler` | [Loki Ruler](../#ruler) rule definitions |

> [!INFO]
>   Prefer granting access through cloud **workload identity** (IRSA on AWS, Workload Identity on GKE, Azure Workload Identity) so no long-lived credentials live in the cluster.
>   Manually configured credentials are supported as a documented escape hatch for environments where workload identity is unavailable.

## Granting object-storage access (workload identity)

On every managed cloud the recommended way to give Loki access to its bucket is **workload identity** — no static keys in the cluster.
The shape is the same across providers: a Loki pod runs as a Kubernetes **ServiceAccount** annotated to reference a cloud identity → the platform projects a signed token into the pod → that token is exchanged for short-lived cloud credentials → Loki uses them against the object store.
Only the binding mechanism differs. Pick your provider:

{{< tabs >}}
{{% tab "AWS · EKS (IRSA)" %}}
**IRSA** (IAM Roles for Service Accounts). Chain: ServiceAccount annotated with a role ARN → EKS projects an OIDC token → the SDK calls **STS `AssumeRoleWithWebIdentity`** → temporary credentials → **S3**. Requires the cluster's **OIDC provider** registered in IAM (one-time).

*Trust policy* — scope `:sub` to the **exact namespace and ServiceAccount Loki runs as** (default namespace `loki`, ServiceAccount `<release>-loki`), not another workload's. For several component ServiceAccounts, use `StringLike` with a `*` suffix.

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": { "Federated": "arn:aws:iam::<account-id>:oidc-provider/oidc.eks.<region>.amazonaws.com/id/<oidc-id>" },
    "Action": "sts:AssumeRoleWithWebIdentity",
    "Condition": { "StringEquals": {
      "oidc.eks.<region>.amazonaws.com/id/<oidc-id>:aud": "sts.amazonaws.com",
      "oidc.eks.<region>.amazonaws.com/id/<oidc-id>:sub": "system:serviceaccount:loki:<release>-loki"
    }}
  }]
}
```

A trust policy scoped to the wrong namespace/ServiceAccount is what produces `STS: AssumeRoleWithWebIdentity … 403 AccessDenied`.

*Permissions policy* — least-privilege to the single bucket + `/loki/*`. `DeleteObject` is required (compactor retention/compaction and the delete-requests store).

```json
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow", "Action": ["s3:ListBucket"], "Resource": ["arn:aws:s3:::<bucket>"],
      "Condition": { "StringLike": { "s3:prefix": ["loki/*"] } } },
    { "Effect": "Allow", "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
      "Resource": ["arn:aws:s3:::<bucket>/loki/*"] }
  ]
}
```

*ServiceAccount* — annotate through chart values; the EKS webhook then injects `AWS_ROLE_ARN` / `AWS_WEB_IDENTITY_TOKEN_FILE`.

```yaml
loki:
  serviceAccount:
    annotations:
      eks.amazonaws.com/role-arn: arn:aws:iam::<account-id>:role/<loki-role>
```
{{% /tab %}}
{{% tab "GCP · GKE (Workload Identity)" %}}
**GKE Workload Identity.** Chain: ServiceAccount annotated with a Google service account (GSA) → GKE exchanges the pod's token for GSA credentials → **GCS**. Requires Workload Identity enabled on the cluster and node pool.

1. Grant the GSA object access on the bucket:

   ```bash
   gcloud storage buckets add-iam-policy-binding gs://<bucket> \
     --member="serviceAccount:<gsa>@<project>.iam.gserviceaccount.com" \
     --role="roles/storage.objectAdmin"
   ```

2. Let the Loki KSA impersonate the GSA — the member **must match Loki's namespace/ServiceAccount**:

   ```bash
   gcloud iam service-accounts add-iam-policy-binding <gsa>@<project>.iam.gserviceaccount.com \
     --role="roles/iam.workloadIdentityUser" \
     --member="serviceAccount:<project>.svc.id.goog[loki/<release>-loki]"
   ```

3. Annotate the ServiceAccount, and use the GCS backend (`loki.loki.object_store: gcs`):

   ```yaml
   loki:
     serviceAccount:
       annotations:
         iam.gke.io/gcp-service-account: <gsa>@<project>.iam.gserviceaccount.com
   ```
{{% /tab %}}
{{% tab "Azure · AKS (Workload ID)" %}}
**Microsoft Entra Workload ID.** Chain: ServiceAccount annotated with a managed-identity client ID → AKS projects a token → exchanged with Entra for the identity's credentials → **Azure Blob**. Requires the OIDC issuer + workload identity enabled on the cluster.

1. Grant the user-assigned managed identity **`Storage Blob Data Contributor`** on the storage account (or container scope).
2. Create a **federated identity credential** on that identity — subject **must match Loki's namespace/ServiceAccount**:
   - issuer = the AKS cluster's OIDC issuer URL
   - subject = `system:serviceaccount:loki:<release>-loki`
   - audience = `api://AzureADTokenExchange`
3. Annotate the ServiceAccount, label the pods so the webhook injects the token, and use the Azure backend (`loki.loki.object_store: azure`):

   ```yaml
   loki:
     serviceAccount:
       annotations:
         azure.workload.identity/client-id: <client-id>
     # The workload-identity webhook only acts on pods carrying this label;
     # apply it via the chart's pod-label values for the Loki components.
     podLabels:
       azure.workload.identity/use: "true"
   ```
{{% /tab %}}
{{< /tabs >}}

> [!INFO]
>   **The token exchange and the object store are both 443 hops** to your cloud's identity and storage endpoints (AWS: STS + S3; GCP: `sts.googleapis.com`/`oauth2.googleapis.com` + GCS; Azure: `login.microsoftonline.com` + Blob). If you enable the Loki NetworkPolicy you must allow that egress (`networkPolicy.externalStorage`), or the credential fetch fails — a *blocked* egress hangs the compactor at startup (a silent connect timeout), while an *allowed* egress with a bad binding returns a fast auth error. See [Operating > Production Best Practices](../../operating/production-best-practices/#11-security--credentials).

> [!NOTE]
>   **Alternatives.** **EKS Pod Identity** is a newer AWS alternative to IRSA (a pod-identity *association*, no `oidc-provider` to manage). **Static keys / connection strings** are the last-resort escape hatch on any cloud — supply them as a Secret consumed by reference, for environments without workload identity.

**Verifying.** Confirm the ServiceAccount carries the provider annotation and the pod has the injected identity env/token, then split the two failure modes: a failure during the **token exchange** (`403`/AccessDenied on `AssumeRoleWithWebIdentity`, or the equivalent GCP/Azure exchange error) is a **binding/trust-scope** problem — usually a namespace/ServiceAccount subject mismatch; an authorization error on the **storage operation itself**, *after* the exchange succeeds, is a **permissions** problem on the bucket/container.

## Chunks and the index

Two kinds of data live in the bucket:

- **Chunks** — the compressed log lines themselves, flushed from ingesters in batches.
- **The index** — the map from [stream labels](../../o11y-glossary/#logs-and-events) to the chunks that contain them, written in the [TSDB](https://grafana.com/docs/loki/latest/operations/storage/tsdb/) index format (schema **v13**).

Because Loki indexes only labels, the index stays small relative to the log volume — the chunks dominate storage.
[Structured metadata](../#storage) is stored with the chunks, queryable but not part of the label index.

> [!WARNING]
>   The schema is configured in append-only **periods** with a future start date; a period that is already in use can never be changed retroactively.
>   Plan schema changes (for example a future index-format bump) as a new period with a `from` date ahead of now — never by editing a past period.

## Compaction

The [Loki Compactor](../#backend) merges the many small index files produced by individual ingesters into a single compacted index per tenant per day.
This keeps reads efficient as volume grows.
The compactor is a **singleton** — exactly one instance coordinates against the shared bucket.

## Retention

Retention is enforced by the compactor, not by the object store's own lifecycle rules.

- A global **retention period** sets how long logs are kept before deletion.
- **Tiered (per-stream) retention** lets different log streams keep data for different lengths of time — for example, keeping `ERROR` and audit-relevant streams far longer than high-volume `INFO` chatter. This is a primary cost lever at fleet scale.
- The **deletion API** processes targeted deletes (for example, compliance "right to be forgotten" requests) outside the normal retention schedule.

## Scaling

- **Ingesters** run **ephemerally** (node-local `emptyDir`, no PVC); durability comes from the replication factor of 3, so you run at least three. Scale past three on memory / stream cardinality, not bytes.
- **Object storage** scales on its own — there is no capacity to provision, only cost and retention to manage.
- Scaling the read side (queriers, frontend) is independent and covered in [Querying](../querying/).

> [!WARNING]
>   Ingester rollouts happen one at a time (guarded by a PodDisruptionBudget). `flush-on-shutdown` is best-effort — a truncated flush is covered by the other replicas, so durability does not depend on a graceful stop.
>   See the [ingester](../#loki-ingester) notes and [Operating > Upgrading](../../operating/upgrading/).

## Disaster recovery

Loki has **no native snapshot** mechanism — recovery is a property of the object store, not a Loki feature:

- **Durability and versioning.** Chunks are immutable once written; enabling object versioning protects against accidental overwrite or deletion.
- **Cross-region replication.** Replicating the bucket to a second region gives you a recovery point if the primary region is lost.
- **Tamper evidence.** Object Lock / WORM (compliance mode) makes stored logs immutable for a fixed window — important when logs must serve as evidence in a security or audit event.
- **Restore.** Recovery is "repoint Loki at the bucket." The WAL is ephemeral and the index is rebuilt from object storage, so there is no separate database to restore.

> [!INFO]
>   During a security or audit event, the log store's guarantees come from these object-store features.
>   Freeze the relevant bucket (or a replicated copy) to preserve logs before retention or deletion can act on them.

## See more

- [Logging Architecture](../) — storage in the context of the full pipeline.
- [Collecting](../collecting/) — how logs arrive before they are stored.
- [Querying](../querying/) — reading stored logs back.
- [Loki storage](https://grafana.com/docs/loki/latest/operations/storage/) and [retention](https://grafana.com/docs/loki/latest/operations/storage/retention/) (official).
