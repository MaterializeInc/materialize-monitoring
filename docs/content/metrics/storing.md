---
title: "Storing"
weight: 20
---

# Storing Metrics

{{< wip >}}

Metrics collected by `alloy-gateway` are forwarded to a Prometheus **remote-write** backend.
Out of the box that is an in-cluster **Thanos Receive**; you can point it at any remote-write-compatible store (Thanos, Mimir, Amazon Managed Prometheus, Grafana Cloud, …) through chart values.
Everything below lives under `pipeline.metrics.gateway.destination.prometheusRemoteWrite`.

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

## Amazon Managed Prometheus (SigV4 + IRSA)

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

- [Collecting](../collecting/) — how metrics arrive before they are stored.
- [Scraping](../scraping/) — configuring ServiceMonitors / PodMonitors on the scrape side.
- [AMP: ingest metrics](https://docs.aws.amazon.com/prometheus/latest/userguide/AMP-onboard-ingest-metrics.html) (official).
