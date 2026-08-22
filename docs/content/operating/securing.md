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

## Certificates {#certificates}

`certificates.enabled` renders cert-manager `Certificate` resources for the stack.
It is **off by default and cert-manager is never a hard dependency** — with it off, nothing in this section renders at all.

It is gated on a value rather than on an API-server capability probe, deliberately.
A probe would make the same chart render differently under `helm template`, the Terraform render check, and an ArgoCD diff than it does under a live install, which is exactly the class of bug those checks exist to catch.
A missing CRD with the flag on is an apply-time failure with a resource name in it, which is a better error than silently rendering nothing.

### Two issuers, because they cannot be one

| | `certificates.internal` | `certificates.external` |
|---|---|---|
| Names | `$svc`, `$svc.$ns`, `$svc.$ns.svc`, `$svc.$ns.svc.$clusterDomain`, `localhost` | the public DNS name the load balancer answers on |
| Typical issuer | a self-signed root, or your private CA | ACME, or a private CA that signs your public names |
| Verified by | the stack's own components | a browser |

A public ACME issuer cannot sign `loki-distributor.monitoring.svc`, and a self-signed root means nothing to a browser.
Collapsing these into one key would make one of the two unusable.

**The external certificate is only needed behind an L4 load balancer**, which passes TCP through and leaves TLS to terminate at the pod — so the material has to exist in the cluster.
An L7 load balancer terminating with a cloud-managed certificate (ACM, Google Certificate Manager, Azure Key Vault) attaches it by ARN or resource ID and the private key never enters the cluster; for that shape leave `external` unset and pass the annotation through `grafana.service.annotations`.
The render warns if you set `external.dnsNames` without an issuer, since that combination looks configured and issues nothing.

### Where the root comes from

Either you supply one, or the chart makes one:

```yaml
# Consume your own PKI. The production path.
certificates:
  enabled: true
  internal:
    issuerRef:
      name: my-ca-issuer
      kind: ClusterIssuer
```

```yaml
# Bootstrap a self-signed root. Renders a selfSigned issuer, a CA Certificate
# signed by it, and a CA issuer every component certificate then references.
certificates:
  enabled: true
  internal:
    selfSigned:
      enabled: true
```

Setting both is an error — component certificates can reference only one issuer, so one of the two things you asked for would silently not happen.

- [ ] `[operator]` **Prefer an issuer scoped to this stack over the cluster's general-purpose one.** None of the receiving components implement per-client authorization, so the whole authorization decision is "is this signed by the CA we trust". Reusing a `ClusterIssuer` that signs for every workload in the cluster reduces mTLS to "has any certificate" — real, and much narrower than it sounds. See [What a certificate means](#rbac) for why the trust domain is the security property.
- [ ] `[operator]` **Use `kind: ClusterIssuer` if you run [split-namespace](../production-best-practices/#namespace-layout).** A namespaced `Issuer` signs only for `Certificate` resources in its own namespace, so components elsewhere sit `Pending` forever. The render refuses that combination rather than letting you find out.
- [ ] `[operator]` **A bootstrapped `ClusterIssuer`'s CA Secret lands in cert-manager's namespace, not yours.** cert-manager reads a `ca` issuer's Secret from its *cluster resource namespace* (`cert-manager` by default), so the chart renders the CA `Certificate` there. Override with `certificates.internal.selfSigned.caSecretNamespace` if your cert-manager uses a different one; get it wrong and the issuer sits `False` with `secret not found` while the Secret is one namespace over.

### The SAN ladder, and why `clusterDomain` matters

Every internal certificate carries four rungs per Service — `$svc`, `$svc.$ns`, `$svc.$ns.svc`, and `$svc.$ns.svc.$clusterDomain` — plus `localhost` and `127.0.0.1`.

All four, because **the chart's own URLs disagree about which form to use**: every in-cluster destination it writes stops at `$svc.$ns.svc`, while the Terraform test substrate writes `…svc.cluster.local`.
A certificate carrying only the fully-qualified name therefore fails verification against endpoints the chart itself ships, and the error reads as a broken certificate rather than as a mismatch in name form.

`cluster.local` is a default, not a fact.
Set `global.clusterDomain` if yours differs — it propagates into Loki and Thanos, which build their own internal addresses from it, so one value covers all three.

- [ ] `[operator]` **Set `metrics-server.tls.clusterDomain` too** if you change the global. metrics-server reads its own key, and the render warns when the two disagree.
- [x] `[chart]` A render check asserts that every in-cluster destination URL the chart writes matches a SAN on the corresponding certificate. A wrong `services` list is valid YAML and installs clean, so this is the cheapest guard against the failure the ladder exists to prevent.

### Turning a backend onto TLS

`profiles/mtls.values.yaml` moves two hops off plaintext — gateway → Loki and gateway → Thanos Receive — and is the supported way to do it:

```bash
helm upgrade --install mzmon charts/materialize-monitoring -n monitoring \
  -f charts/materialize-monitoring/profiles/aws-example.values.yaml \
  -f charts/materialize-monitoring/profiles/mtls.values.yaml \
  --set certificates.enabled=true \
  --set certificates.internal.selfSigned.enabled=true
```

**It is a profile rather than a switch because the change is not one setting.**
Turning on Loki's listener is one key; keeping the deployment working is six, spread across three subcharts and two of this chart's own trees — the writer, the reader, the kubelet probes, the metrics scrape, and the canary all dial the port that just moved.
Every one of them fails quietly, and none of the symptoms names TLS:

| Left behind | What you see |
|---|---|
| The gateway's destination | Writes fail with a protocol error that reads like Loki is broken |
| `loki.defaults.readinessProbe` scheme | Every Loki pod fails readiness at once — presents as a crashloop |
| `loki.monitoring.serviceMonitor` scheme | Loki's own metrics vanish, and `up` goes *absent* rather than 0, so an alert on `up == 0` does not fire either |
| The Grafana datasource URL | Every log panel renders empty, with no error on the dashboard |
| The canary's flags | The end-to-end check reports the log store as broken when it is not |

The render refuses each of those rather than letting you find out, which is most of what the chart contributes here.

Thanos Receive is narrower by construction: its TLS flags scope to the remote-write listener, so probes, metrics and the ServiceMonitor are untouched and Thanos Query stays plaintext.

#### Through Terraform

The Terraform module composes the same profiles from one input, because a consumer of the module has no copy of the chart directory to point `-f` at:

```hcl
certificates_enabled = true
internal_tls         = "authenticate"   # off | encrypt | present | authenticate
```

The stages map to the profiles in order — `encrypt` is `mtls.values.yaml`, `present` adds `mtls-phase2`, `authenticate` adds `mtls-phase3` — so the table in the next section describes both paths.
`internal_tls` needs `certificates_enabled`, and the module refuses the combination without it rather than installing a stack that mounts Secrets nothing created.

In `materialize-terraform-self-managed` both are on by default, since every example there installs cert-manager.

### The phases, and where each hop can actually end up

Three profiles, composed in order. **The two hops do not reach the same place, and that is a property of Kubernetes rather than of the backends** — all of this was measured on a live cluster, not read off documentation.

| Phase | Profile | Gateway ingress | Loki | Thanos Receive |
|---|---|---|---|---|
| 1 | `mtls.values.yaml` | TLS, no client CA | TLS, `NoClientCert` | TLS, no client CA |
| 2 | `+ mtls-phase2.values.yaml` | client CA set; clients present | `VerifyClientCertIfGiven`, client presents | client presents, server still ignores it |
| 3 | `+ mtls-phase3.values.yaml` | `RequireAndVerifyClientCert` — **authenticated** | **unreachable** | client CA set — **authenticated** |

The gateway's own ingress reaches phase 3 because its listeners are not the ports the kubelet probes — readiness is on `12345`. That is the difference between it and Loki.

**Loki's HTTP port cannot require client certificates, ever.** The kubelet's readiness and liveness probes dial the same port 3100 that the gateway does, and a Kubernetes `httpGet` probe has no field for a client certificate. Setting `RequireAndVerifyClientCert` fails every probe with `remote error: tls: certificate required`, and every Loki pod goes unready and then restarts. The render refuses it. **Phase 2 is the ceiling for that hop**: a certificate from the wrong CA is refused, an anonymous client is still served. Real authentication there needs an authenticating proxy in front of Loki, or a listener the kubelet does not touch.

**Thanos Receive does reach phase 3**, because its probes are on the HTTP port while the TLS flags scope to the separate remote-write listener. Verified: a client presenting no certificate is refused at the TLS handshake; one presenting a certificate from the trusted CA is served.

**`min_version` has three vocabularies in one binary, and two of them fail differently.** Client blocks (`tls_config`) take `TLS13`; the dskit-flavoured listeners (`loki.source.api`, `prometheus.receive_http`) take `VersionTLS13` and reject anything else at load, crashlooping the pod; `otelcol.receiver.otlp` takes OpenTelemetry's `1.3` and rejects the others **silently** — the component goes unhealthy, its port never binds, and the process stays up reporting Ready. `alloy validate` catches none of the three. Values use one vocabulary and the chart translates per listener; that silent case is why `kubectl get pods` is not enough to confirm this feature is working.

Two more measured constraints the profiles encode, both of which crashloop the stack if you get them wrong:

- **Loki's `client_ca_file` and `client_auth_type` must arrive together.** dskit refuses a client CA with no policy — Loki exits at startup with `client CA's have been configured without a Client Auth Policy`, buried in a Go stack trace, on every microservice at once. That is why phase 1 ships neither.
- **Both probes need the scheme, not just readiness.** Liveness hits a different path on the same port; left plaintext it returns 400 and the kubelet restarts the container *after* readiness has gone green, which reads as an unrelated flap.

- [ ] `[operator]` **Phase 1 is encryption, not authentication**, and phase 2 only rejects the wrong CA. Phase 3 is where a client presenting nothing is refused — on Thanos Receive's remote-write listener and all four gateway ingress ports. Loki's HTTP port stops at phase 2 and cannot go further, because the kubelet probes it.
- [ ] `[operator]` **Roll the server and its clients in either order at phase 1 and 2, never at phase 3.** Kubernetes does not order them, so a server that starts requiring certificates before its clients present them stops ingesting until they catch up. Phase 2 exists to make phase 3 order-independent; the render refuses phase 3 applied without it.
- [ ] `[operator]` **Grafana's datasource TLS does not renew like the rest.** It reads from `secureJsonData`, which is provisioned config rather than a file mount, so a new CA means re-provisioning the datasource.

### Renewal is the failure that matters

Certificate material is **mounted from a Secret**, not injected through environment variables.
Env vars are captured once at process start and cert-manager renews by rewriting the Secret in place, so an env-carried PEM works for exactly one certificate lifetime and then fails on every hop simultaneously — months after the change that caused it.
Prefer the `tls.*File` carriers on every destination over the inline `ca`/`cert`/`key`, which remain supported for bring-your-own-PKI.

The mount is unconditional and marked `optional: true`, so the same values work before, during and after issuance: a Secret that does not exist yet mounts empty rather than blocking the pod.

- [ ] `[operator]` **Do not set `renewBefore` near `duration`.** It looks like a way to exercise renewal and it is a way to break cert-manager: at 92% of duration (a 1h certificate with 55m of headroom) renewal fires every few minutes, and on a small cluster with six certificates the controller livelocked in an optimistic-locking re-queue loop, stopped renewing, and then reported `Ready=True: "Certificate is up to date and has not expired"` on certificates that had expired 45 minutes earlier — every TLS hop failing with `certificate has expired` while the Certificate resource looked healthy. Keep `renewBefore` to a third of `duration` or less, and force renewal explicitly (delete the Secret, or `cmctl renew`) if you want to test it.
- [ ] `[operator]` **A mounted file is not a reloaded file.** The kubelet refreshes Secret contents atomically, but the process still has to notice, and reload support differs per component. That is why no hop turns on by default, and why enabling one is a decision to make per component rather than per stack.

## What is not there yet {#gaps}

Stated plainly, because the values surface implies more than the deployment has — `minVersion: TLS13` sitting next to `enabled: false` reads like a switch rather than a project.

| Gap | Status |
|---|---|
| **Certificate issuance** | ✅ Shipped, off by default. `certificates.enabled` renders cert-manager `Certificate` resources with the full SAN ladder — see [Certificates](#certificates) |
| **In-cluster TLS, gateway → Thanos Receive** | ✅ Shipped and **authenticated** at phase 3, off by default. A client with no certificate is refused at the handshake |
| **In-cluster TLS, gateway → Loki** | 🔨 Encrypted at phase 2, and that is its ceiling — the kubelet probes the same port and cannot present a certificate |
| **In-cluster TLS, every gateway ingress port** | ✅ Shipped and **authenticated** at phase 3 — `3100`, `4317`, `4318` and `9090`. All four listeners render from Helm and take TLS from values; a client presenting no certificate is refused at the handshake on each |
| **In-cluster TLS, agent → gateway** | ✅ Shipped and **authenticated** at phase 3. The listener renders from Helm and the agent's destination presents a certificate; moving `prometheus.receive_http` out of the pre-rendered pipeline was the last blocker |
| **Mutual TLS between components** | ✅ At phase 3, five listeners require and verify a client certificate: Thanos Receive's remote-write port and the gateway's 3100, 4317, 4318 and 9090. Loki's HTTP port is the exception and stays at verify-if-given. Authentication, not authorization — none of these can express "this identity may write and that one may not", so the size of the trust domain is the security property |
| **Authenticated scrapes of node-exporter** | Available and deliberately parked. `kubeRBACProxy` would authenticate via TokenReview/SubjectAccessReview over HTTPS, at the cost of a second container on every node to protect an endpoint that exposes no secrets |
| **A trust bundle for a private CA** | ❌ Not shipped ([DEP-236](https://linear.app/materializeinc/issue/DEP-236)). Needed for an S3-compatible store behind a private CA, and for images that ship no CA bundle at all |
| **Intra-Loki and intra-Thanos TLS** | ❌ Not shipped. Distributor→ingester gRPC, the memberlist ring, query→store — all real hops inside a single subchart's trust boundary |
| **Redaction in the pipeline** | ❌ Not shipped ([DEP-220](https://linear.app/materializeinc/issue/DEP-220)) |

**Issuance and use are separate switches on purpose**, and a default install turns on neither. A hop only leaves plaintext once that component's renewal behaviour has been proven, because a component that does not reload a renewed certificate works for exactly one certificate lifetime and then fails with no deploy nearby to blame — which is why `tls::survives_renewal` forces a reissue and asserts delivery across it rather than trusting a freshly-installed stack.

Where the phases land is therefore your choice, not the chart's. A stack sitting at phase 1 or 2 is **encrypted and not authenticated**, and phase 2 is the state most likely to be mistaken for mTLS: every values file carries a `certFile`, the servers name a client CA, and a client presenting nothing is still served. Only phase 3 refuses it.

The design for the first three is written up in the [TLS and authentication design doc](../../reference/internal/design-docs/20260816-tls-authentication/) (internal), including the two-phase rollout that gets there without an outage.

## See also

- [Production Best Practices](../production-best-practices/) — the per-component checklists these items read across.
- [Network policies](../production-best-practices/#network-policies) — the full per-component policy table.
- [Grafana > Authentication](../../dashboards/grafana/auth/) — identity providers, role mapping, and the break-glass path.
- [Grafana > Reaching Grafana](../../dashboards/grafana/architecture/#reaching-grafana) — exposure options and the allowlist convention.
- [Logs & Events > Storing](../../logs-and-events/storing/) — object storage, workload identity, and retention for logs.
- [Metrics > Storing](../../metrics/storing/) — the same for metrics.
- [Helm values reference](../../reference/helm/materialize-monitoring-values/) — every key named on this page, with the reasoning next to it.
