---
title: "Securing"
weight: 50
---

# Securing materialize-monitoring

This page reads across the whole stack: what its trust boundaries are, what it is allowed to do in your cluster, where credentials live, and what is not built yet.
Per-component configuration lives in [Production Best Practices](../production-best-practices/), which is organized by backend; this page is organized by the question you are actually asking.

Everything here describes the chart's defaults.
The [Terraform modules](../../getting-started/terraform/) are one `[consumer]` implementation of them and close a few of the open items on their own; those are marked where they apply.

## What you are securing

The stack's value and its risk are the same property: **it aggregates everything**.
Every log line from every pod, every metric series from every workload and node, and the credentials needed to reach the object storage holding both, all land in one namespace behind one Grafana.

That shapes the threat model more than any individual component does:

- **Read access to Grafana is read access to the cluster's telemetry**, across every namespace, through datasources that are already authenticated to the backends. There is no per-user scoping below the datasource.
- **Logs and metrics are not sanitized.** An application that logs a token logs it into Loki, where it is searchable and retained for as long as the tenant's retention says.
- **The collection tier is privileged by nature.** It reads pods, nodes, and the API server cluster-wide, because that is the job.

None of that is unusual for an observability stack.
It is worth stating because the useful mitigations follow from it — restrict who reaches Grafana, keep secrets out of telemetry, and know what the collectors are allowed to read — rather than from hardening any single pod.

## Trust boundaries

Three boundaries, in decreasing order of how much attention they usually deserve.

| Boundary | What crosses it | What controls it |
|---|---|---|
| **Outside → the stack** | Humans reaching Grafana; anything you deliberately expose | Grafana authentication, ingress/LB allowlists, the render-time exposure check |
| **The cluster → the stack** | The Alloy gateway's ingest ports, every backend's query and write API | NetworkPolicy, and nothing else — see below |
| **The stack → outside** | Object storage, notification providers, external metric destinations, an identity provider | Egress NetworkPolicy, workload identity, your egress firewall |

### Outside → the stack

Grafana is the only component in the stack meant for a human, and the only one this chart will help you expose.
The render enforces the allowlist convention rather than trusting it:

| Shape | What the render does |
|---|---|
| `LoadBalancer` with no `loadBalancerSourceRanges` | **Error**, unless `connections.grafana.allowPublicAccess` is set — which suppresses the error, not the exposure, and still warns |
| `NodePort` | **Error** unless `allowPublicAccess` is set; a NodePort has no allowlist mechanism of its own |
| Ingress with no `hosts`, or the upstream placeholder host | **Error** — it would carry no rules and route nothing |
| Ingress with no `tls` block | **Warning**. Grafana authenticates with a session cookie, so without TLS that cookie and the admin password cross the network in the clear |
| Exposed with no `grafana.ini.server.root_url` | **Warning**. Share links, alert notification links, and OAuth redirect URIs are all built from it, and all three break silently when it does not match the URL users reach |
| Exposed with no authentication provider enabled | **Warning** — the admin password is the only thing in front of every log and metric in the cluster |
| Exposed with `auth.anonymous` enabled | **Error** unless `allowPublicAccess` is set. Anyone who can reach it reads every dashboard and every datasource behind it without signing in |

- [ ] `[operator]` **Configure an identity provider before Grafana is reachable by anyone.** A fresh install has one account — the admin, with a generated password — and that is fine behind `kubectl port-forward` and not fine otherwise. See [Grafana > Authentication](../../dashboards/grafana/auth/).
- [ ] `[operator]` **Nothing else in the stack should be exposed.** Loki, Thanos, Alertmanager and the Alloy gateway have no authentication of their own (see below), so an ingress in front of any of them is an unauthenticated one.
- [ ] `[operator]` Reach a backend directly through `kubectl port-forward` when you need to, rather than by giving it an ingress. [Troubleshooting](../o11y-troubleshooting/) uses that shape throughout.

### The cluster → the stack

**Inside the cluster, NetworkPolicy is the only access control the stack has.**
No component authenticates its callers:

| Endpoint | What an arbitrary pod could do without a policy |
|---|---|
| Alloy gateway ingest (`3100`, `4317`, `4318`, `9090`) | Write logs and metrics into your backends |
| Loki distributor / query frontend (`3100`) | Read and write any tenant's logs |
| Thanos Receive (`10908`) / Query (`9090`) | Write and read any metric series |
| Alertmanager (`9093`) | Read alerts, create silences |
| Every `/metrics` endpoint | Read the stack's own telemetry |

Loki runs with `auth_enabled: true`, which is worth reading correctly: it makes the `X-Scope-OrgID` header **required**, so reads and writes are scoped to a tenant rather than pooled into an implicit one.
It does not authenticate anything — the header is a string any client can send.
Tenancy here is a data-partitioning boundary, not a security one, and the hard isolation boundary remains the install.

Every workload now ships a NetworkPolicy, on by default.
[Production Best Practices > Network policies](../production-best-practices/#network-policies) has the per-component table, the reasoning behind the ingress/egress asymmetry, and the three limits worth knowing — chiefly that a NetworkPolicy does nothing unless your CNI enforces it, and that `kindnet` does not.

- [ ] `[operator]` **Confirm your CNI enforces NetworkPolicy.** Cilium and Calico do. If yours does not, everything in the table above is reachable from any pod in the cluster and the policies are documentation.
- [ ] `[operator]` Narrow the ingest ports if your cluster has no workloads that legitimately push telemetry. They are open to the whole cluster by default precisely because usually it does.

### The stack → outside

- [ ] `[consumer]` **(Terraform: automatic on AWS and GCP)** Use **workload identity** for object storage — IRSA, GKE Workload Identity, or Azure Workload Identity — rather than static keys. See [Logs & Events > Storing](../../logs-and-events/storing/#granting-object-storage-access-workload-identity) and [Metrics > Storing](../../metrics/storing/#thanos-object-storage).
- [ ] `[operator]` Narrow the `0.0.0.0/0` egress rules where your infrastructure gives you something tighter — a VPC endpoint's CIDR, or Cilium's `toFQDNs`. The chart cannot derive the API server's or the object store's address, so it ships the broad form and says so.
- [ ] `[operator]` Remember the credential endpoint, not just the bucket. Workload identity fetches a token from STS (or the GCP/Azure equivalent) on `443`, and a policy that covers the bucket but not the token endpoint hangs the component at startup rather than failing it. This is the single most common NetworkPolicy mistake in this stack; [Loki's checklist](../production-best-practices/#11-security--credentials) covers the symptom in detail.

## Cluster permissions the stack holds {#rbac}

Two grants are broader than people expect, and both are load-bearing rather than incidental.

| Subject | Scope | Notable permissions |
|---|---|---|
| `alloy-agent`, `alloy-gateway` | cluster-wide | `get`/`list`/`watch` on **`secrets`** and `configmaps`; `pods`, `pods/log`, `nodes`, `nodes/metrics`, `events`; the Prometheus Operator CRDs |
| `kube-state-metrics` | cluster-wide | `list`/`watch` on **`secrets`** and every other built-in resource kind |
| `grafana-operator` | cluster-wide | full CRUD on `grafana.integreatly.org/*`, plus `deployments`, `services`, `secrets`, `configmaps`, `ingresses` |
| `metrics-server` | cluster-wide | `nodes/metrics`, and `get`/`list`/`watch` on `pods`, `nodes`, `namespaces` |
| `loki` | cluster-wide | `get`/`watch`/`list` on `configmaps` and `secrets` |

**Both Alloy roles can read every Secret in the cluster.**
This comes from the upstream Alloy chart and it is required by `prometheus.operator.servicemonitors` and `podmonitors`: a `ServiceMonitor` may reference a Secret for bearer-token or TLS scrape credentials, and Alloy resolves those references itself, so it cannot know in advance which Secrets it will need.

**kube-state-metrics can list every Secret**, for `kube_secret_*` metrics.
It reads metadata and never the values — but the RBAC grant does not distinguish those, so the ServiceAccount is as powerful as the grant.

- [ ] `[operator]` **Treat these ServiceAccounts as high-value.** Anyone who can create a pod with the `alloy-gateway` ServiceAccount, or exec into its pods, can read every Secret in the cluster. That is a stronger reason to restrict `exec` in the monitoring namespace than anything about the telemetry itself.
- [ ] `[operator]` **Do not simply drop the Secret grant — a default install already uses it.** The shipped `mzmon-materialize-environmentd-sql` PodMonitor authenticates with `basicAuth` against the `materialize-sql-monitor` Secret, and Alloy resolves that reference itself. What makes the grant *cluster-wide* rather than namespaced is `namespaceSelector: any: true` on the monitors plus the operator convention that each resolves its credentials from its own namespace. If you know which namespaces host monitors, a Role in each of those is a real reduction; getting it wrong fails visibly, with an RBAC error in the gateway's logs and a target that stops being scraped.
- [ ] `[operator]` `grafana-operator` watches **cluster-wide** by default, so two releases in different namespaces both reconcile every `Grafana` in the cluster. Scope `WATCH_NAMESPACE`, or narrow `connections.grafana.labels` per release.

## Credentials and secrets

The chart **consumes secrets by name and mints almost none**.
A default install renders three: the Grafana admin credentials, the Thanos objstore config, and the Materialize SQL-monitor credentials.
Everything else — an object-storage key, a Grafana database password, an OIDC client secret, an external destination's token — is yours to provision, with External Secrets Operator, Vault Agent, SOPS, or your cloud's CSI driver.

Two rendering traps are worth knowing, because both put a plaintext credential somewhere durable rather than failing:

- **`grafana.ini` renders into a ConfigMap.** A secret written there is plaintext in the release manifest, in `helm get values`, and in whatever Git repo holds your values. Use `$__file{/path}` against a mounted Secret, or `$__env{VAR}`. The subchart's `assertNoLeakedSecrets` check fails the render on a known-sensitive key set to a literal — leave it on. See [Grafana > Authentication](../../dashboards/grafana/auth/#client-secrets-never-go-in-grafanaini).
- **Loki's config defaults to a ConfigMap too.** With static S3 credentials the rendered config carries `secret_access_key` verbatim, so `loki.loki.configStorageType: Secret` is load-bearing whenever you are not using workload identity. The Terraform module sets it automatically on that path; a hand-written values file has to. Thanos needs no equivalent — its objstore document already renders into a Secret.

- [ ] `[consumer]` Provision every Secret in the namespace the **pod** runs in, which under [`split-namespace`](../production-best-practices/#namespace-layout) is not the release namespace.
- [ ] `[operator]` **Rotate the Grafana admin password** after an identity provider is configured, or disable the account. It is a generated password in a Secret, and it bypasses SSO.
- [ ] `[operator]` Prefer file-mounted credential material over environment variables for anything that renews. An env-var PEM is read once at process start, so renewal does not take effect until every pod restarts.

## Workload hardening

Most of the stack runs non-root with a read-only root filesystem and all capabilities dropped.
Two workloads cannot, and both are deliberate:

| Workload | Deviation | Why |
|---|---|---|
| `alloy-agent` | runs as **root**; three `hostPath` mounts (`/var/log`, `/run/log/journal`, `/etc/machine-id`) | It reads container logs and the systemd journal off the node. No capabilities are added — everything it reads is reachable by uid 0 under ordinary file permissions |
| `node-exporter` | **`hostNetwork: true`**; `hostPath` mounts of `/proc`, `/sys` and `/` | The network collectors read namespaced files under `/proc/net`; in a pod network namespace they would report the pod's traffic rather than the node's |

`node-exporter` is otherwise the most locked-down workload in the stack — distroless, non-root, read-only root filesystem, `automountServiceAccountToken: false` — because a shell on a container that reads the host's `/proc`, `/sys` and `/` is a materially better foothold than most.

Two consequences follow from those deviations, and the second is the one that surprises people:

1. **`hostNetwork` puts node-exporter outside pod NetworkPolicy** on most CNIs, so port `9100` is guarded by the node firewall and nothing Kubernetes enforces. See [Exposure](../production-best-practices/#exposure).
2. **The release namespace cannot run under Pod Security Admission `baseline` or `restricted` as shipped.** Baseline forbids `hostPath` volumes and host namespaces, and the two DaemonSets need both, so the namespace has to be labelled `privileged`. Nothing else in the stack needs it.

- [ ] `[operator]` **Give the DaemonSets their own namespace if you want PSA above `privileged` for the rest.** [`split-namespace`](../production-best-practices/#namespace-layout) is the mechanism; the backends, Grafana and the gateway are all `baseline`-clean today. Note that support for that layout is best-effort, and that it changes the workload-identity subject and the NetworkPolicy selectors along with it.
- [ ] `[operator]` **`seccompProfile` is not set on every workload** — Alloy, Thanos, Alertmanager and grafana-operator leave it unset, so they inherit the container runtime's default rather than declaring `RuntimeDefault`. Set it through each subchart's `podSecurityContext` if you are targeting `restricted`.
- [ ] `[operator]` Restrict `pods/exec` and `pods/portforward` in the monitoring namespace. Given the ServiceAccount permissions above, exec into the gateway is a cluster-wide Secret read.

## Supply chain

Every image is pinned by registry, repository and tag — in this chart's `values.yaml` where the cadence matters enough to own, in a subchart's defaults otherwise.
Repointing at a mirror or a hardened rebuild is a values change: four overlays under `profiles/registry/` do it for the whole stack at once, including the pull-secret wiring.
See [Images and registries](../production-best-practices/#images-and-registries) for the vendors and the UID hazard that makes a careless swap crash-loop.

- [ ] `[operator]` **Pin Grafana plugin versions (`name@version`), or bake them into an image.** `grafana.plugins` downloads from `grafana.com` at every pod start — a startup dependency on a third party, and a way for a plugin to change underneath a pinned Grafana. A validator warns on an unpinned entry.
- [ ] `[operator]` **The Grafana Image Renderer stays off.** It is a headless Chromium that fetches URLs on Grafana's behalf: a large attack surface and a server-side request forgery pivot into the cluster network. A validator warns when it is enabled.

## The telemetry itself

Data that reaches the stack is stored as it arrives.

- [ ] `[operator]` **Keep secrets out of logs at the source.** The pipeline ships no redaction stage today ([DEP-220](https://linear.app/materializeinc/issue/DEP-220)), so a token an application logs is a token in Loki, searchable for the tenant's full retention.
- [ ] `[operator]` **Set retention deliberately.** It is the only bound on how long anything that did leak stays queryable. Retention and compaction are covered in [Logs & Events > Storing](../../logs-and-events/storing/) and [Metrics > Storing](../../metrics/storing/).
- [ ] `[consumer]` Enable server-side encryption and access logging on the buckets. The Terraform modules enable versioning; encryption policy is yours, and bucket versioning means a deleted object is not necessarily gone.
- [ ] `[operator]` Remember that **isolation within a Loki tenant is label-based**, not enforced. Per-environment separation via `environment_id` is a query convention; the hard boundary is a separate install. See [Tenancy & auth](../production-best-practices/#9-tenancy--auth).

## What is not there yet {#gaps}

Stated plainly, because the values surface implies more than the deployment has — `minVersion: TLS13` sitting next to `enabled: false` reads like a switch rather than a project.

| Gap | Status |
|---|---|
| **In-cluster TLS on every hop** | Not shipped. Every internal URL is `http://` — agent → gateway, gateway → Loki and Thanos, Grafana → both. Traffic between components is plaintext on the pod network |
| **Certificate issuance** | Not shipped. No `Certificate` templates and no cert-manager integration ([DEP-195](https://linear.app/materializeinc/issue/DEP-195)) |
| **Mutual TLS between components** | Not shipped. NetworkPolicy answers *which pods* may connect; nothing answers *who is on the other end*, and several policies answer the first question with "any pod in the cluster" |
| **Authenticated scrapes of node-exporter** | Available and deliberately parked. `kubeRBACProxy` would authenticate via TokenReview/SubjectAccessReview over HTTPS, at the cost of a second container on every node to protect an endpoint that exposes no secrets |
| **A trust bundle for a private CA** | Not shipped. Nothing lets you add a CA to Loki, Thanos or Alloy for an S3-compatible endpoint with a private certificate |
| **Redaction in the pipeline** | Not shipped ([DEP-220](https://linear.app/materializeinc/issue/DEP-220)) |

The design for the first three is written up in the [TLS and authentication design doc](../../reference/internal/design-docs/20260816-tls-authentication/) (internal), including the two-phase rollout that gets there without an outage.

## See also

- [Production Best Practices](../production-best-practices/) — the per-component checklists these items read across.
- [Network policies](../production-best-practices/#network-policies) — the full per-component policy table.
- [Grafana > Authentication](../../dashboards/grafana/auth/) — identity providers, role mapping, and the break-glass path.
- [Grafana > Reaching Grafana](../../dashboards/grafana/architecture/#reaching-grafana) — exposure options and the allowlist convention.
- [Logs & Events > Storing](../../logs-and-events/storing/) — object storage, workload identity, and retention for logs.
- [Metrics > Storing](../../metrics/storing/) — the same for metrics.
- [Helm values reference](../../reference/helm/materialize-monitoring-values/) — every key named on this page, with the reasoning next to it.
