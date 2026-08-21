---
title: "In-cluster TLS and Certificate Authentication"
weight: 20260816
# draft=false makes it render as a page
# params.status=Draft is to indicate that the design is not final
draft: false
publishdate: 2026-08-16
lastmod: 2026-08-16
# custom parameters
params:
  author: Heather Lapointe
  status: "Draft"
---

# In-cluster TLS and Certificate Authentication

{{< param-table >}}

This doc is the design for **in-cluster mTLS via cert-manager** on the [Roadmap](../../roadmap/#charts--helm) ([DEP-195](https://linear.app/materializeinc/issue/DEP-195), OO-M1).
It expands the [In-cluster TLS and authentication](../20260803-terraform-modules/#in-cluster-tls-and-authentication) section of the Terraform modules doc into a full proposal and supersedes it; that section stays as the Terraform-side summary and should point here.

The central claim is that this work is about **authentication, not encryption**.
Every hop inside the stack is plaintext HTTP with `authType: none` today, and turning on one-way TLS would fix the smaller half of that.
Inside a cluster the realistic threat is not a passive network tap, it is *a workload that can reach the Service and write whatever it likes into Loki and Thanos*.
Only the client certificate answers that, which is why the deliverable is mTLS on both halves of each hop rather than `https://` in the destination URLs.

<!--
Agent note: this doc records decisions and their *why*. When a decision lands in code, update the section
and check the matching row in "Work in this repo". The two-phase rollout table and the "what a certificate
means" section are the parts most likely to be contradicted by implementation — revise them rather than
leaving them aspirational.

The Terraform doc's "In-cluster TLS and authentication" section is the older, shorter version of this.
Do not let the two drift; that one is a pointer plus the Terraform-variable decisions.
-->

## Goals

Functional requirements, framed as value-first user stories.
Priority tags (**Must** / **Should** / **Could**) are relative to the first shipped version.

- **[Must] As a security reviewer,** I want every hop between stack components to be mutually authenticated, so that reaching a Service is not the same thing as being allowed to use it.
- **[Must] As an operator,** I want certificates issued and renewed for me, so that enabling authentication does not commit me to a rotation runbook I will forget to run.
- **[Must] As an operator,** I want a certificate renewal to be invisible, so that turning this on does not schedule an outage for one certificate lifetime from now.
- **[Must] As an operator without cert-manager,** I want the chart to install and run exactly as it does today, so that a hardening feature is not a new hard dependency.
- **[Must] As an operator with my own PKI,** I want to supply certificate material myself, so that a stack that assumes cert-manager is not a stack I have to fork.
- **[Must] As an on-prem operator with an S3-compatible store behind my own CA,** I want to tell the stack which roots to trust, so that using my object storage does not mean disabling verification or rebuilding images.
- **[Must] As an operator on a cluster whose domain is not `cluster.local`,** I want certificates that match the names the chart actually dials, so that enabling this does not produce hostname mismatches on endpoints the chart itself wrote.
- **[Should] As an operator on a managed cloud,** I want my existing certificate store (ACM, Google Certificate Manager, Azure Key Vault) to work for the public endpoint, so that hardening does not require adopting ACME.
- **[Should] As an operator mid-rollout,** I want to enable this hop by hop without dropping telemetry, so that hardening is not an all-at-once cutover on a live pipeline.
- **[Should] As a Terraform consumer,** I want this on by default, so that the opinionated path is the secure one.
- **[Should] As a maintainer,** I want the same primitive to serve the [BYOC control-plane ingress](../20260813-byoc-observability/#ingress-otlp-into-the-control-plane), so that we operate one certificate story rather than two.
- **[Could] As an operator,** I want authenticated scrapes of node-level exporters, so that the one endpoint no NetworkPolicy can protect is not the one endpoint left open.

## Technical BLUF

- **The client half is already modeled; the server half is the work.** Every destination carries `tls.{ca,cert,key}` and `minVersion: TLS13`. A configured client against an unconfigured server is TLS-off, so nothing in that surface is load-bearing yet.
- **Issuance is cert-manager, opt-in in the chart, on by default in Terraform.** Same shared-responsibility split as buckets and workload identity: the chart consumes an issuer by name, the consumer supplies it.
- **Gate rendering on a values flag, never on `.Capabilities`.** A capability probe makes `helm template`, tier-0 renders, and GitOps diffs produce different output than a live install, which is exactly the class of bug the render tests exist to catch.
- **File-mounted material, inline PEM as the escape hatch.** Env-injected PEMs are captured at process start; cert-manager renews by rewriting the Secret. That combination works for one certificate lifetime and then fails everywhere at once.
- **No component ships with mTLS on until its rotation test passes.** Reload-on-renewal is a per-component property, not a stack property, and it is the only failure mode that a freshly-installed test cluster cannot see.
- **Three trust domains, deliberately not merged.** Internal mesh (private issuer), browser-facing (the Grafana LB, often a cloud-managed certificate that never enters the cluster), and outbound destinations including object storage (frequently a customer's own private CA). A component can need all three at once, which is why the surface is a **trust bundle**, not "the CA".
- **Never hardcode `cluster.local`, and never assume one SAN form.** The chart's own URLs use `$svc.$ns.svc`, so a certificate carrying only `*.cluster.local` fails against the very endpoints this chart ships. SANs must cover the whole ladder plus `localhost` and any ingress hosts.
- **The scheme is derived, not hand-edited.** `tls.enabled: true` must change `http://` to `https://` in the rendered destination URL, or the first symptom of a correct config is a plaintext-to-TLS-port error.
- **Rollout is two-phase per hop.** Servers accept-but-do-not-require client certificates first, clients start presenting, then verification flips to required. A one-shot cutover drops telemetry on any hop where the two halves land out of order.
- **mTLS authenticates peers; it does not authorize tenants.** Loki tenancy stays `X-Scope-OrgID`, asserted by the client. In-cluster there is no load balancer to assign identity the way the [BYOC design](../20260813-byoc-observability/#identity-is-assigned-by-the-load-balancer-never-asserted-by-the-client) does, so "signed by our CA" is the whole authorization decision — which is why the size of the trust domain is the security property that matters most here.
- **NetworkPolicy and mTLS are complements.** One answers "who can open a socket", the other "who is on the other end". Neither substitutes for the other, and node-exporter is the case that proves it.

## Non-goals

- **A service mesh.** If an operator runs Istio or Linkerd, mTLS is already handled and this feature should be left off. The chart should not attempt to detect or integrate with one.
- **Intra-subchart TLS.** Loki's distributor↔ingester gRPC, Thanos's query↔store gRPC and receive hashring, and Alloy's own clustering traffic are all real hops with real upstream flags, and all of them are internal to a single subchart's trust boundary. They are named in [Hops in scope](#hops-in-scope) and deferred; the inter-component hops are where the authentication gap actually is.
- **Inventing a PKI or a CA hierarchy.** This consumes an issuer. cert-manager's `Issuer` / `ClusterIssuer` is the interface; where the root comes from is the consumer's decision.
- **Issuing certificates *for* object storage.** Trusting one is in scope (see [Object storage](#object-storage-is-in-scope-in-one-direction)); standing one up is the operator's.
- **End-user authentication to Grafana.** SSO through `grafana.ini` is a separate surface, already documented, and unrelated to workload identity.
- **Per-identity authorization rules.** None of the receiving components support meaningful per-client policy, and pretending otherwise would misrepresent what a certificate buys. See [What a certificate actually means](#what-a-certificate-actually-means).

## What exists today

| Capability | State | Where |
|---|---|---|
| Client-side TLS on every pipeline destination (`ca` / `cert` / `key` + `*Env`, `serverName`, `minVersion: TLS13`) | ✅ Modeled | `pipeline.{logging,metrics}.{agent,gateway}.destination.*.tls` |
| Destination auth types (`none` / `basicAuth` / `bearer` / `oauth2` / `sigv4`) | ✅ Shipped | `authType` on every destination |
| Mounted-secret convention on both Alloy roles | ✅ Exists | `alloy.mounts.extra`, `controller.volumes.extra` |
| Loki multi-tenancy | ✅ Shipped | `auth_enabled: true`; `X-Scope-OrgID` from `pipeline.logging.tenancy.tenantMap` and from the Grafana datasource header |
| Grafana ingress TLS | ✅ Modeled | `grafana.ingress.tls`, with the render refusing a `LoadBalancer` with no allowlist unless `allowPublicAccess` |
| **Default posture on every in-cluster hop** | ⚠️ **Plaintext** | `http://alloy-gateway…:3100`, `http://loki-distributor…:3100`, `http://thanos-receive…:10908`, `http://thanos-query…:9090`, `http://loki-query-frontend…:3100` |
| **Certificate issuance** | ✅ Shipped | `templates/certificates.yaml`, gated on `certificates.enabled` (default false). Per-component `Certificate`s with the full SAN ladder, an opt-in self-signed root chain, and a separate external issuer for a Grafana behind an L4 LB |
| **Server-side TLS on all three gateway listeners** | ⚠️ `raw` only | `loki.source.api` `http`, `otelcol.receiver.otlp` `grpc` / `http`, and `prometheus.receive_http` `http` — every one reaches `tls` only through the `raw` escape hatch. The `receive_http` schema says so outright: *"`http` configures the server; a `tls` block uses the `raw:` escape"* |
| **A configurable cluster domain** | ✅ Shipped | `global.clusterDomain`. Placed under `global` rather than top-level because Loki and Thanos already read `global.clusterDomain` and build real addresses from it, so Helm's propagation makes one value cover all three — this settles [open question 9](#open-questions). `metrics-server` reads its own `tls.clusterDomain` and is covered by a validator instead |
| **A trust-bundle surface for non-public CAs** | 🔨 Modeled | `certificates.trustBundle` names a Secret and/or ConfigMap. Mounting it into Loki and Thanos is outstanding |
| **Server-side TLS on Loki / Thanos / Grafana / Alertmanager** | ⬜ Not wired | Each subchart exposes it through its own config passthrough; the umbrella models none of it |
| **Grafana → backends client certificates** | ⚠️ Expressible, unmodeled | `connections.datasources.*.valuesFrom` can inject `secureJsonData`; nothing defaults or documents the cert keys |
| **Authenticated node-exporter scrape** | ⚠️ Parked | `nodeExporter.kubeRBACProxy` off by default — a sidecar on every node to protect an endpoint that exposes no secrets |
| **NetworkPolicy coverage** | ✅ Shipped | Every workload, on by default ([DEP-192](https://linear.app/materializeinc/issue/DEP-192)). Ingress is narrowed to known peers; egress is narrowed only where the chart knows the destination set, which is node-exporter, kube-state-metrics and Loki |
| **External client-cert auth (BYOC)** | 📄 Designed | [BYOC design doc](../20260813-byoc-observability/#two-stage-verification-with-one-revocation-checkpoint) |

The pattern in that table is worth stating directly: **the values surface implies more security than the deployment has.**
An operator reading `minVersion: TLS13` next to `enabled: false` can reasonably come away thinking TLS is a switch rather than a project.
Part of the deliverable is making the shipped default and the modeled capability agree.

## Hops in scope

| Hop | Transport | Server side | Phase |
|---|---|---|---|
| `alloy-agent` → `alloy-gateway` | Loki push, HTTP :3100 | `loki.source.api` | 1 |
| Application / third-party → `alloy-gateway` | remote-write, HTTP :9090 | `prometheus.receive_http` | 1 |
| Application / third-party → `alloy-gateway` | OTLP :4317 / :4318 | `otelcol.receiver.otlp` | 1 |
| `alloy-gateway` → Loki distributor | Loki push, HTTP :3100 | Loki server | 1 |
| `alloy-gateway` → Thanos receive | remote-write, HTTP :10908 | Thanos receive | 1 |
| Loki / Thanos → object storage | HTTPS, operator-supplied endpoint | the store | 1 (trust only) |
| Grafana → Thanos query | PromQL, HTTP :9090 | Thanos query | 2 |
| Grafana → Loki query frontend | LogQL, HTTP :3100 | Loki server | 2 |
| `alloy-gateway` → scrape targets (kube-state-metrics, node-exporter) | scrape, HTTP | target | 3 |
| Loki ruler / Thanos ruler → Alertmanager | HTTP | Alertmanager | 3 |
| Loki ruler → `alloy-gateway` (recording-rule samples) | remote-write, HTTP :9090 | `prometheus.receive_http` | 3 |
| `alloy-gateway` → control-plane gateway (BYOC) | OTLP | control-plane LB | designed elsewhere |
| Browser → Grafana | HTTPS | Ingress / LB | already modeled |
| Intra-Loki, intra-Thanos, Alloy clustering | gRPC / HTTP | upstream | deferred |

**The gateway has three listeners, not two.**
`loki.source.api` on :3100, `otelcol.receiver.otlp` on :4317 / :4318, and `prometheus.receive_http` on :9090 are all unauthenticated ingress.
`receive_http` is the easiest of the three to forget, because it is defined in `gateway-metrics.yaml` rather than alongside the other two in `gateway.yaml` — and forgetting it leaves the metrics write path wide open behind a logs path that looks secured.
Any statement about this work that says "both receivers" is wrong; there are three, and all three need the same treatment.

Phase 1 is the ingest path, where the authentication gap is worst: anything that can reach `alloy-gateway:3100` can write arbitrary logs attributed to arbitrary tenants, anything that can reach `alloy-gateway:9090` can write arbitrary series, and anything that can reach `thanos-receive:10908` can write them straight past the gateway.
Phase 2 is the read path, where the exposure is disclosure rather than injection.
Phase 3 is the long tail.

Scrape targets sit apart from the rest because the direction is inverted: the *gateway* is the client, and its certificate has to reach the scrape configuration in the `ServiceMonitor` / `PodMonitor` CRs (`packages/prometheus-scrapers/`) rather than the pipeline values.
That is a different code path from every push destination and should be scoped as such.

## What a certificate actually means

This is the section to read if you read only one.

The receiving components in this stack do not implement per-client authorization.
Loki, Thanos receive, and Alloy's receivers can be told to require a client certificate signed by a given CA; none of them can be told that *this* identity may write and *that* one may not.
So the entire authorization decision collapses to one bit: **is this certificate signed by the CA we trust?**

Two consequences follow, and both are load-bearing.

**The trust domain is the security boundary.** If the monitoring stack trusts a cluster-wide CA that also signs certificates for every other workload in the cluster, then mTLS raises the bar from "can reach the Service" to "has any certificate from the cluster CA" — which, in a cluster where cert-manager issues to everything, is nearly the same set. The protection that survives is against unauthenticated and off-cluster clients, which is real but much narrower than it sounds.

**Tenancy is still asserted, not assigned.** Loki runs `auth_enabled: true` and takes the tenant from `X-Scope-OrgID`, set by the client. A certificate does not constrain that header. The BYOC design solves the equivalent problem by having the load balancer derive a verified tenant header from the client certificate and having the gateway overwrite anything the client sent; in-cluster there is no such checkpoint, and adding one would mean putting a proxy in front of Loki that we do not otherwise need.

**Recommendation: issue from an issuer scoped to the monitoring stack** rather than reusing the cluster's general-purpose `ClusterIssuer`, and say plainly in the docs what the shared-issuer configuration does and does not buy.
This is in tension with mirroring `materialize-instance`'s `internal_issuer_ref` exactly, which is what makes the examples pass one set of locals to both modules.
The proposal is to keep the **variable names and semantics** identical — an operator who understands certs for their Materialize instance understands them here — while documenting a dedicated issuer as the recommended value, and defaulting the Terraform path to a monitoring-scoped issuer it provisions itself.
See the [open questions](#open-questions); this is the one most worth arguing about before implementation.

## Three trust domains, and where each certificate comes from

Calling this "internal versus external" is the shape most designs start with and it does not survive contact with the deployments we actually have.
There are three, they use different issuance mechanisms, and **a single component can participate in all three at once** — the Loki ingester presents an internal certificate, verifies the internal CA on inbound connections, and trusts a completely unrelated CA when it writes to object storage.

| Domain | Names | Issued by | Material lives |
|---|---|---|---|
| Internal mesh | `$svc`, `$svc.$ns`, `$svc.$ns.svc`, `$svc.$ns.svc.$clusterDomain`, `localhost` | cert-manager against a private issuer — self-signed, a CA `Issuer`, or an external issuer fronting ACM Private CA / Google CAS / Azure | Mounted Secret in the cluster |
| Browser-facing | The Grafana ingress or LB hostname | ACME **or** a cloud-managed certificate store (ACM, Google Certificate Manager, Azure Key Vault), each of which has public **and** private variants | Often **never in the cluster** — attached to the LB by ARN or resource ID |
| Outbound destinations | Whatever the operator's object store, remote Loki, AMP, or OTLP target presents | Not ours. Frequently a customer's own private CA | We hold only the CA to trust |

Three consequences shape the values surface.

**The browser-facing certificate may not be a `Certificate` at all.**
ACM, Google Certificate Manager, and Azure Key Vault attach at the load balancer by reference; the private key never enters the cluster and there is nothing to mount, renew, or reload.
For those, the chart's entire job is to pass an annotation through to the Service or Ingress — which `grafana.ingress` and the [LB convention](../20260803-terraform-modules/#follow-the-house-lb-convention) already do — and to *not* try to issue anything.
A design that models browser-facing TLS as "a cert-manager `Certificate` with an ACME issuer" quietly excludes every consumer using their cloud's certificate store, which is most of them.

**Each of those stores has a private variant, and they are not interchangeable with the public one.**
ACM Private CA, Google CAS, and Azure's private CA issue for internal names, and cert-manager has external issuers for the first two.
So `internal_issuer_ref` must not assume a self-signed or in-cluster CA issuer: it is an `issuerRef`, and whatever satisfies it is the operator's business.
The only hard constraint is the one below — a public ACME issuer cannot sign internal names, so the two variables cannot collapse into one.

**Trust is plural.** See [Object storage](#object-storage-is-in-scope-in-one-direction).

## SANs and the cluster domain

`cluster.local` is a default, not a fact.
Clusters get built with `--cluster-domain=cluster.internal` or a site-specific domain often enough that hardcoding it is a reliable way to ship a feature that works everywhere we test and nowhere a customer runs it.
The chart has **no `clusterDomain` value at all** today, so this work introduces one.

The sharper trap is that a correct cluster domain is not sufficient, because **the chart's own URLs stop short of it**.
Every in-cluster destination is written as `$svc.$ns.svc` — `http://loki-distributor.{{ ns }}.svc:3100` — while `terraform/test/generic-cloud` writes the object-storage endpoint as `…svc.cluster.local:9000`.
A certificate carrying only `*.svc.cluster.local` SANs therefore fails verification against the exact endpoints this chart ships, and the error surfaces as a hostname mismatch that reads like a bug in the certificate rather than a mismatch in name form.

**Decision: every internal `Certificate` carries the full SAN ladder**, rendered from the same namespace helpers the URLs use:

- `$svc`
- `$svc.$ns`
- `$svc.$ns.svc`
- `$svc.$ns.svc.$clusterDomain`
- `localhost` and the `127.0.0.1` IP SAN, for self-probes, health checks, and anything a component reaches through its own loopback
- `$ingressHost[s]` for any component with an ingress, when the internal issuer is also serving that hostname

Two follow-ons.
The ladder has to be derived from the same `mzmon.*.namespace` helpers as the URLs, or `profiles/split-namespace.values.yaml` produces certificates that are valid for the wrong namespace — and that failure only appears in the split configuration, which is not the default anyone tests first.
And if intra-subchart TLS is ever picked up, StatefulSet members need their per-pod headless names (`$pod.$svc.$ns.svc.$clusterDomain`) as well; that is one more reason it is deferred rather than assumed cheap.

The cheapest guard is a render assertion: for each `Certificate`, every destination URL that resolves to that Service must match at least one SAN. A wrong SAN list is valid YAML and installs clean.

## Object storage is in scope, in one direction

An earlier draft listed object storage as a non-goal on the reasoning that S3, GCS, and Azure Blob already use TLS with a public CA.
That is wrong in two ways that both show up in real deployments.

**S3-compatible is not S3.** We have a customer running MinIO on Rancher on-prem, serving TLS with their own non-public certificates. The endpoint is already configurable — `loki.storage.object_store.s3.endpoint` documents naming an S3-compatible host directly — so the stack fully supports pointing at such a store and has no way to trust it. The failure is a certificate-verification error at startup on every component that touches storage, and the only workarounds available today are disabling verification or rebuilding images.

**A public CA in the image is not guaranteed.** Minimal and distroless images may ship no CA bundle at all, so even a genuinely public endpoint can fail verification depending on what the component runs on. Relying on the system trust store is an assumption about the image, and we ship images.

**Decision: the chart exposes a trust-bundle surface, and trust is plural.**
`caFile` singular is the wrong model — a Loki ingester may simultaneously need the internal issuer's CA (to verify the gateway and the query frontend) and the object store's CA (to write chunks), and those are unrelated roots.
The surface should let an operator supply additional CAs as a Secret or ConfigMap that is mounted and concatenated with the internal CA into the bundle each component is pointed at, rather than replacing it.

Scope stays narrow and one-directional: **we trust the store, we do not authenticate to it with a certificate.** Object-storage auth remains the credential paths already built — workload identity, or the static credentials from [DEP-203](https://linear.app/materializeinc/issue/DEP-203).

Two smaller notes. `tier 2`'s rustfs substrate is plaintext `http://`, so a private-CA object store is not covered by any test today and should get one — it is the configuration a real customer is running. And whatever bundle mechanism lands should be reachable by Alloy too, since a `loki.write` or `prometheus.remote_write` aimed at an operator's own backend has exactly the same problem.

## Issuance

### cert-manager stays optional in the chart

cert-manager is a dependency we **encourage** in production and cannot **require**.
Plenty of installs do not have it, and the CRDs chart has no business pulling in another ecosystem's CRDs.
The split follows the [shared responsibility model](../../../../operating/production-best-practices/#shared-responsibility-model): the chart renders `Certificate` resources only when told to, and the Terraform path turns them on because cert-manager and a `ClusterIssuer` are already in that stack.

### Gate on values, not on capabilities

The obvious implementation is `.Capabilities.APIVersions.Has "cert-manager.io/v1"`.
It should not be used.

A capability probe makes the same chart render differently depending on what is talking to the cluster.
`helm template` with no cluster, the tier-0 Terraform render check, and an ArgoCD server-side diff would each produce output that a live `helm install` does not — and tier 0 exists precisely to assert that values land where they are supposed to, which requires the render to be a pure function of the values.

**Decision: `certificates.enabled` gates every cert-manager resource**, defaulting false, and the chart never probes the API server for CRDs.
A missing CRD with the flag on is a clear apply-time failure with a name in it, which is a better error than silently rendering nothing.

### Convergence, not pre-flight

cert-manager and the issuer must exist before the chart.
Under Terraform that is a `depends_on`; for Helm users it is the same "install this first" story the CRDs chart already carries.
Per the [ordering reality](../20260627-loki-production-infrastructure/#the-ordering-reality), the chart must still converge when certificates are not ready yet: pods that mount a not-yet-created Secret crashloop and recover, and the validation hook must not turn "the issuer has not signed yet" into a failed release.

### Proposed values shape

```yaml
certificates:
  # -- Render cert-manager Certificate resources for in-cluster mTLS.
  enabled: false
  issuerRef:
    name: ""
    kind: ClusterIssuer
    group: cert-manager.io
  duration: 2160h    # 90d
  renewBefore: 720h  # 30d
  # Where the material is mounted in every consuming pod.
  mountPath: /etc/mzmon/tls
  # Additional roots to trust, merged with the internal CA rather than
  # replacing it — an on-prem object store's private CA, a customer PKI, a
  # public bundle for images that ship none. Mounted and concatenated.
  trustBundle:
    secretName: ""
    configMapName: ""
  # Per-component overrides; each renders one Certificate with the full SAN
  # ladder for that component's Service, in whichever namespace it runs in.
  components:
    alloyAgent: {}
    alloyGateway: {}
    loki: {}
    thanos: {}
    grafana: {}
    alertmanager: {}

# -- The cluster's DNS domain. `cluster.local` is a default, not a fact.
# Load-bearing for SANs; see "SANs and the cluster domain".
clusterDomain: cluster.local
```

One `Certificate` per component role rather than one shared certificate, because the SANs differ, the namespaces can differ under `profiles/split-namespace.values.yaml`, and a shared key across every workload makes any single compromise total.

On the client side, the existing `tls` block gains file paths alongside the inline PEMs:

```yaml
tls:
  enabled: true
  # Existing inline/env carriers stay as the bring-your-own-PKI escape hatch.
  ca: ""
  cert: ""
  key: ""
  # New: preferred for anything cert-manager renews.
  caFile: /etc/mzmon/tls/ca.crt
  certFile: /etc/mzmon/tls/tls.crt
  keyFile: /etc/mzmon/tls/tls.key
  serverName: ""
  minVersion: TLS13
```

And each of the gateway's **three** listeners gains a server block it does not have today.
They are split across the two pipeline trees the same way the listeners themselves are, so neither can be added without the other:

```yaml
pipeline:
  logging:
    gateway:
      server:
        # loki.source.api :3100 and otelcol.receiver.otlp :4317/:4318
        tls: &serverTls
          enabled: false
          certFile: /etc/mzmon/tls/tls.crt
          keyFile: /etc/mzmon/tls/tls.key
          clientCAFile: /etc/mzmon/tls/ca.crt
          # none | request | require | verifyIfGiven | requireAndVerify
          clientAuth: requireAndVerify
  metrics:
    gateway:
      server:
        # prometheus.receive_http :9090 — the listener most likely to be
        # left plaintext behind a secured logs path.
        tls: *serverTls
```

`clientAuth` is exposed rather than implied because `verifyIfGiven` is the state the [two-phase rollout](#rollout-is-two-phase-per-hop) needs, and an operator who cannot name that state cannot perform the rollout safely.

A `profiles/mtls.values.yaml` should assemble the whole thing, in keeping with [profiles as documentation](../20260803-terraform-modules/#profiles-are-documentation-with-one-exception).

## Renewal is the failure that matters

The existing TLS values carry PEM **contents through environment variables**.
That is a reasonable way to write a certificate inline in `values.yaml`, and it is the wrong carrier for anything cert-manager renews: environment variables are captured at process start, and renewal rewrites the Secret in place.
The result works for exactly one certificate lifetime and then fails on every hop simultaneously, months after the change that caused it, with no deploy nearby to blame.

**Decision: file-mounted material for the cert-manager path**, inline PEM retained for values-only and bring-your-own-PKI users.
Both Alloy roles already expose `alloy.mounts.extra` and `controller.volumes.extra`, so mounting a cert Secret follows an existing convention rather than introducing one.

File mounts move the problem rather than solving it.
The kubelet does refresh mounted Secret contents — atomically, on its sync period, so a reader never sees a half-written pair — but the process still has to notice.
Reload behavior differs per component:

- `otelcol.receiver.otlp` supports a `reload_interval` on its TLS block and can reload the client CA independently, which is the best case in the stack.
- Alloy's Prometheus and Loki client paths reload certificate files on new connections rather than on a timer, which is usually enough and is not the same guarantee.
- Loki's and Thanos's server-side TLS reload behavior varies by component and version.
- Grafana reads its datasource TLS material from `secureJsonData`, which is provisioned config rather than a file, so renewal there means re-provisioning the datasource — a genuinely different mechanism.

The honest position is that per-component reload support is the gating unknown for this whole feature, and it cannot be settled by reading values files.

**Decision: no component's mTLS default flips on until a rotation test passes for that component.**
Not "until mTLS works", which a freshly-installed cluster will always show.
Where a component cannot reload, the fallback options in preference order are: a longer `duration` with a generous `renewBefore` and documented manual restart; a checksum annotation that rolls the workload when the Secret changes; and a reloader sidecar — which is a new dependency and should be the last resort, not the first design.

## Rollout is two-phase per hop

Enabling both halves of a hop in one release requires the server to start requiring client certificates at the same moment the clients start presenting them.
Kubernetes does not order those, so on any hop where the server rolls first, ingestion stops until the clients catch up.

Each hop therefore moves through four states:

| Phase | Server | Client | Effect |
|---|---|---|---|
| 0 | plaintext | plaintext | Today. |
| 1 | TLS on, `clientAuth: none` | TLS on, verifying the server | Traffic is encrypted; either order of rollout works. |
| 2 | `clientAuth: verifyIfGiven` | presenting a client certificate | Certificates that are presented are verified; clients that have not rolled yet still work. |
| 3 | `clientAuth: requireAndVerify` | presenting a client certificate | Authenticated. Unauthenticated clients are rejected. |

> [!WARNING]
>   **Measured on a live cluster: this table is the model, not what every hop can do.** Three corrections, all found by running the rollout rather than by reading configuration references.
>
>   **Phase 3 is unreachable on any port the kubelet probes.** Loki serves `/ready` and `/loki/api/v1/status/buildinfo` on the same 3100 the gateway writes to, and a Kubernetes `httpGet` probe has no field for a client certificate. `RequireAndVerifyClientCert` fails every probe with `remote error: tls: certificate required`; the pods go unready and restart. There is no values-side fix, so **phase 2 is the terminal state for the Loki HTTP hop** and the same applies to any component whose probe and data ports coincide. Getting real authentication there needs an authenticating proxy or a listener the kubelet does not touch — neither is in scope here, and the design should stop implying phase 3 is universally reachable.
>
>   **Not every server has a phase 2.** Thanos Receive's `--remote-write.server-tls-client-ca` is require-and-verify the moment it is set — there is no verify-if-given, and the flag's help text does not say so. On that hop phase 2 is a *client-only* step: the writer starts presenting while the server still ignores it, which is what makes phase 3 order-independent. Receive does reach phase 3, because its probes are on a different port from the TLS'd listener.
>
>   **The two halves of Loki's client-auth config are not independent.** dskit refuses a `client_ca_file` with no `client_auth_type` policy: Loki exits at startup with `client CA's have been configured without a Client Auth Policy`, every microservice at once, with the reason inside a Go stack trace. The CA and the policy have to arrive in the same step, so phase 1 ships neither.
>
>   Shipped as `profiles/mtls.values.yaml` plus `mtls-phase2` / `mtls-phase3`, with a validator refusing each of the combinations above.

Phase 2 is the one that makes this safe, and it is also the one an operator will be tempted to skip.
The documentation should state that a stack sitting in phase 2 is **not** authenticated — it is a migration state, not a destination — because `verifyIfGiven` produces a config that looks like mTLS in every values file and rejects nothing.

The other thing that breaks a naive rollout is the URL scheme.
Every in-cluster destination default is `http://`, and `tls.enabled: true` against an `http://` URL is either ignored or fails confusingly depending on the component.
**The rendered scheme must be derived from `tls.enabled`**, with an explicit URL still winning if the operator sets one.
A tier-0 render assertion should pin this, since a wrong scheme is valid YAML.

## Per-component notes

**`alloy-gateway`'s three listeners.**

> [!WARNING]
>   **Found during implementation: typing the `tls` block is not the blocker — *conditionality* is.**
>   All three listeners live in `pre-rendered/pipelines/gateway.alloy`, which the chart emits **verbatim** into the ConfigMap. Alloy config has no conditional-block construct, and a `tls` block with an empty `cert_file` is a load error rather than a no-op, so there is no way to make these listeners serve TLS *only when a Helm value says so* while they stay pre-rendered. Typing the schema does not change that.
>
>   Two ways out, and it should be a deliberate decision:
>
>   1. **Render the ingress from Helm**, the way `mzmon.alloyGateway.pipeline.destination` already is. That is the existing precedent for a values-driven part of the pipeline, and the `alloy validate` pre-install job is the safety net that replaces the JSONSchema. Cost: three more components leave schema validation.
>   2. **Render variants at build time** and have Helm pick the file. Keeps schema coverage; doubles the maintenance surface for every future ingress change.
>
>   Until then the chart **refuses** `pipeline.logging.agent.destination.loki.tls.enabled` with a message saying why, rather than enabling a client half that would talk TLS at a plaintext port. The gateway's *own* destinations — gateway → Loki and gateway → Thanos — have no such problem, because both are Helm-rendered, and those are the phase-1 hops that shipped first.

`loki.source.api`'s `http` block, `otelcol.receiver.otlp`'s `grpc` / `http` blocks, and `prometheus.receive_http`'s `http` block all reach `tls` only through the `raw` escape hatch.
Shipping mTLS on `raw` would work and should not be the end state: the [pipelines-as-code](../../pipelines/) convention is that anything we configure routinely gets a typed block.
Typing `tls` on all three is part of this work, and `receive_http` is the one to check for last, since it lives in `gateway-metrics.yaml` and is easy to leave behind.

**The gateway is both server and client, in different trust domains.**
It requires certificates from agents (internal CA) while presenting one to a control-plane gateway (BYOC PKI, from the license) and another to Loki and Thanos.
The values shape must let each listener and each destination name its own CA and material — a single global `tls` block for the process would make the BYOC path unrepresentable.

**Loki and Thanos.**
Both expose server TLS and a client CA through their own configuration — Loki via its server block's HTTP and gRPC TLS config, Thanos receive via its remote-write server TLS flags.

> [!NOTE]
>   **Measured during implementation: the two are not the same size of job.**
>
>   **Thanos Receive is narrow.** `--remote-write.server-tls-*` scopes to the remote-write listener, so the HTTP port — probes, metrics, the ServiceMonitor — is untouched. Two keys (`extraArgs`, `extraVolumes`) and it is done. The one trap is that Helm overwrites lists, so `extraArgs` has to restate `--receive.replication-factor=3` or write quorum silently drops to 1.
>
>   **Loki is wide, and every part of it fails quietly.** `server.http_tls_config` moves port 3100, which is also the readiness probe, the metrics scrape, the Grafana datasource, and the canary's target. Six settings across three subcharts move together, and none of the symptoms names TLS: a plaintext probe reads as a crashloop, a plaintext scrape makes `up` go *absent* rather than 0, and a plaintext datasource renders empty panels with no error.
>
>   Two things made it tractable. `loki.defaults` is coalesced ahead of the per-component values in the subchart's `_pod.tpl`, so one `extraVolumeMounts` and one `readinessProbe` reach every microservice. And the canary is the exception — it renders from its own template, so nothing in `defaults` reaches it, the same asymmetry that already forces an explicit `priorityClassName` on it.
>
>   The conclusion for the remaining hops: **the validator is the deliverable, not the values.** A profile can assemble six coupled settings; only a render-time check stops someone applying four of them.
These go through the existing subchart passthrough, and the exact keys should be pinned by snapshot tests, because a value written to a path the subchart does not read is still valid YAML and still renders green.
Both are also the components that need the [object-store trust bundle](#object-storage-is-in-scope-in-one-direction), so their mounts carry two unrelated roots and should be built as a bundle from the start rather than retrofitted from a single `caFile`.

**Grafana → backends.**
Client certificates are expressible today through `connections.datasources.*.valuesFrom` injecting `secureJsonData`, and nothing defaults or documents them.
This should become modeled configuration on `connections.datasources.*.tls` that renders the right `jsonData.tlsAuth` / `secureJsonData` pair, so an operator is not reverse-engineering Grafana's datasource schema.
Note the renewal caveat above: this path is provisioned config, not a file mount.

**node-exporter.**
`kubeRBACProxy` is off by default, and the reason stands: it is a second container on every node, with a request comparable to node_exporter's, to protect an endpoint that exposes no secrets.
It is also the one component where mTLS is the *only* available control — the pods run `hostNetwork: true`, and most CNIs do not apply pod NetworkPolicy to host-networked pods, so port 9100 is guarded by the node firewall and nothing Kubernetes enforces.
**Recommendation: leave it off, wire it so it can be turned on in one flag once certificates exist, and state the trade in the production checklist** rather than quietly flipping a per-node cost on every install.

**Alertmanager.**
Lowest-value hop in the set — the traffic is alert notifications, and the exposure is injecting false alerts rather than reading or corrupting telemetry.
Phase 3, and acceptable to defer past 1.0 if it is the last thing standing.

## Relationship to NetworkPolicy

These are complements and the docs should refuse to let them be read as alternatives.
NetworkPolicy answers *who can open a socket*; mTLS answers *who is on the other end*.
A NetworkPolicy that permits the gateway's pod label does not distinguish the gateway from anything else wearing that label, and mTLS does not stop an attacker from exhausting a listener it cannot authenticate to.

Two asymmetries are worth writing down:

- **node-exporter has NetworkPolicy that most CNIs will not enforce**, so mTLS (via `kubeRBACProxy`) is the only in-cluster control available there.
- **NetworkPolicy now covers every component** ([DEP-192](https://linear.app/materializeinc/issue/DEP-192)), which makes mTLS the remaining half rather than the first half. It also sharpens what mTLS is for here: the shipped policies answer *which pods* may open a socket, and several of them answer it with "any pod in the cluster" — the Alloy gateway's ingest ports and every Thanos service port. Identity on those hops is exactly what a NetworkPolicy cannot supply.

## Relationship to BYOC

The [BYOC design](../20260813-byoc-observability/) uses the same primitive at the cluster boundary: a client certificate shipped in the customer's license, verified at an L7 load balancer that holds the verifier CA and does revocation, and again at `otelcol.receiver.otlp`.

Three things carry over, and one does not.

Carrying over: typed server-side TLS on the OTLP receiver is a shared prerequisite; the mounted-material and reload work is the same work; and the file-mount convention is what makes revocation-adjacent operations (re-issue, re-mount) tractable in both.

Not carrying over: **identity assignment**. BYOC gets a verified tenant header from the load balancer and has the gateway overwrite whatever the client sent. In-cluster there is no equivalent, so tenancy remains client-asserted. Do not let the BYOC section's stronger guarantee be read as describing the in-cluster path.

Also worth remembering from that doc: `otelcol.receiver.otlp` has **no CRL support**.
In BYOC the load balancer is the only revocation checkpoint.
In-cluster there is no load balancer, so revocation is effectively "re-issue the CA" — a real limitation, and an argument for shorter certificate durations rather than for a revocation mechanism we do not have.

## Related work outside this repo

This is not the only TLS story in flight, and the pieces should land on one set of conventions rather than three.

**`materialize-terraform-self-managed` has its own load balancers to finish.**
The Materialize console is the clear one; pgwire is an open question — whether it terminates TLS at the LB, passes through, or is exposed at all differs by deployment, and it is worth settling before the monitoring stack's LB conventions harden into precedent.
The convergence point is the [issuer variable split](#three-trust-domains-and-where-each-certificate-comes-from): `issuer_ref` for browser-facing names, `internal_issuer_ref` for internal ones, with the same semantics in both repos, so an operator configures certificates once.

**Certificate sources are broader than ACME on both sides.**
ACM, Google Certificate Manager, and Azure Key Vault all need to be first-class for the public endpoints, in both their public and private variants — and for the LB-attached ones the wiring is an annotation carrying an ARN or resource ID, not a `Certificate` resource.
Whichever repo builds that passthrough first should build it as the shared shape.

**The trust-bundle surface is not monitoring-specific either.**
A private-CA object store is a problem for anything in a self-managed deployment that writes to it.
If `materialize-instance` grows an equivalent, the two should agree on how additional roots are supplied rather than inventing a Secret layout each.

## Testing

The test that proves the least is the one that will get written first: install with certificates on, assert data still flows.
A stack with TLS misconfigured into a no-op passes it.

Three tests actually qualify this feature.

**Negative authentication.** A client presenting no certificate, and a client presenting one signed by a different CA, must both be rejected on every phase-3 hop. Without this, `verifyIfGiven` and `requireAndVerify` are indistinguishable from the outside, and a stack can ship believing it is authenticated when it is only encrypted.

**Rotation.** Issue a deliberately short-lived certificate, force renewal, and assert delivery continues across it. This is the failure the env-var carrier produces, it is invisible to any test against a freshly installed stack, and per the decision above it is what gates each component's default. It belongs in tier 2, with the certificate `duration` and `renewBefore` set low enough that the renewal happens inside the test's runtime rather than being simulated.

**Transport assertion.** Confirm that the hop is genuinely TLS — a plaintext connection to the port is refused, or the peer certificate is the expected one — rather than inferring it from a successful query.

Tier 1 should run the chart-only path with a self-signed issuer, which is enough for the render, mount, and negative-auth assertions.
Tier 2 should run the full Terraform path with mTLS on by default, since that is the configuration Terraform consumers will actually get, and it is the tier that can afford the rotation test's runtime.
The [Rust E2E suite](../../roadmap/#testing--ci--devex) already assigns NetworkPolicy and mTLS to tier 2; this fills in what those assertions have to be.

## Work in this repo

| Item | Depends on | Phase |
|---|---|---|
| ~~`clusterDomain` value, threaded through the namespace helpers~~ **Shipped** as `global.clusterDomain` | — | 1 |
| ~~`certificates.*` values block and `Certificate` templates, gated on `certificates.enabled`, carrying the full SAN ladder~~ **Shipped**, plus an opt-in self-signed root and an external issuer for the L4-LB case | `clusterDomain` | 1 |
| ~~Render assertion that every destination URL matches a SAN on the corresponding `Certificate`~~ **Shipped** (`mzmon.certificates.validate.sans`) | above | 1 |
| `certificates.trustBundle` — additional roots mounted and concatenated with the internal CA, reaching Loki, Thanos, and both Alloy roles | — | 1 |
| ~~Mount rendering for each component, on the existing `mounts.extra` / `volumes.extra` convention~~ **Shipped for both Alloy roles**, as an unconditional `optional: true` secret volume — so the same values work before, during and after issuance. Loki and Thanos outstanding | above | 1 |
| ~~`*File` carriers on every destination `tls` block~~ **Shipped** | — | 1 |
| ~~Scheme derivation from `tls.enabled`~~ **Shipped** (`mzmon.alloy.destUrl`), with unit coverage; a tier-0 assertion is still worth adding | — | 1 |
| Typed `tls` blocks on `loki.source.api`, `otelcol.receiver.otlp`, **and `prometheus.receive_http`** in the Alloy schema, replacing `raw` | — | 1 |
| `pipeline.logging.gateway.server.tls` **and `pipeline.metrics.gateway.server.tls`** values and rendering | above | 1 |
| ~~Loki and Thanos server TLS through subchart passthrough~~ **Shipped** via `profiles/mtls.values.yaml`. Loki: `loki.server.http_tls_config` + `defaults.extraVolumes`/`extraVolumeMounts`/`readinessProbe` + `monitoring.serviceMonitor.scheme` + canary flags. Thanos: `receive.extraArgs` + volumes. A validator refuses every half-applied combination rather than pinning keys with snapshots — the coupling, not the key names, is what breaks | — | 1–2 |
| ~~Modeled `connections.datasources.*.tls` for Grafana → backends~~ **Shipped** and verified against a live Grafana: `tls.caPem` (inline; a CA is public material) or `tls.caSecret` (a `valuesFrom` reference). Neither is defaulted, because an https datasource with no CA fails as an empty dashboard — the render refuses it. **Caveat found on the cluster:** grafana-operator's `valuesFrom` substitutes into a `${...}` placeholder that must already exist at the target path, and on 5.24 the substituted CA arrives but Grafana reports `failed to parse TLS CA PEM certificate`; the same CA inlined connects. Grafana → Thanos Query is not shipped: Query has no `--http.tls-*` flags, only an experimental `--http.config` that would also TLS its probe endpoints | — | 2 |
| ~~`profiles/mtls.values.yaml`~~ **Shipped** | most of the above | 2 |
| Terraform `issuer_ref` / `internal_issuer_ref` variables and default-on wiring | chart side | 2 |
| Rotation, negative-auth, and transport E2E assertions | per hop | with each hop |
| A tier-2 variant with a private-CA object store, replacing plaintext rustfs | trust bundle | 2 |
| LB-attached certificate passthrough (ACM / GCM / Azure KV references) for the Grafana Service and Ingress | — | 2 |
| `ServiceMonitor` / `PodMonitor` `tlsConfig` for scrape targets | — | 3 |
| `kubeRBACProxy` wiring, left off by default | certificates | 3 |

## Documentation to update

- [Securing](../../../../operating/securing/) — **currently a title and nothing else.** This is the user-facing home for the whole feature: the three trust domains, the two-phase rollout, what to supply for a private-CA object store, and what mTLS does and does not authorize. Everything else on this list is a cross-reference; this one is the page.
- [Architecture](../../../../architecture/) — the component diagram and hop descriptions currently describe plaintext endpoints without saying so, and list two gateway listeners where there are three.
- [Production best practices](../../../../operating/production-best-practices/) — the `kubeRBACProxy` checklist item says the cert-manager integration is where issuance lands; it should point here, and the checklist needs the phase-2-is-not-authenticated warning.
- `terraform/modules/materialize-monitoring/README.md` — "mTLS between components is not wired" is accurate today and needs to change with the feature, not after it.
- [Values reference](../../../helm/materialize-monitoring-values/) — regenerated by `helm-docs`, but the `@raw` prose on the TLS blocks should stop implying that `minVersion: TLS13` is doing something while `enabled` is false.
- [Terraform modules design doc](../20260803-terraform-modules/#in-cluster-tls-and-authentication) — reduce to the Terraform-variable decisions plus a pointer here.
- [Roadmap](../../roadmap/) — the DEP-195 row gains a link to this doc.

## Open questions

1. **Dedicated issuer or shared with `materialize-instance`?** *(Partly settled: the chart now offers `internal.selfSigned` as an opt-in dedicated root, default off, with consume-by-reference as the production path. The Terraform default is still open.)* Sharing keeps the variable story simple and makes any Materialize workload's certificate valid against our receivers. A dedicated issuer makes "signed by our CA" mean something. The proposal above recommends dedicated-by-default in Terraform with identical variable names; this deserves an explicit decision rather than inheriting one.
2. **Which components actually reload certificate files?** *(Partly answered, measured: with `duration: 1h` / `renewBefore: 55m` on a live cluster, cert-manager took every certificate to revision 6 — five renewals — while Loki served TLS and the pipeline kept delivering, with no restarts attributable to renewal. That covers Loki's server side and Alloy's client side. Thanos Receive, Grafana and the remaining hops are still unmeasured, and Grafana is known not to reload: it stores datasource material in its own database, so a new CA takes effect on grafana-operator's `resyncPeriod` rather than on renewal.)*
3. **Should Loki tenancy derive from certificate identity in-cluster?** It would close the assert-vs-assign gap, and the only mechanism available is a proxy in front of Loki that the stack does not otherwise need. Probably not worth it — but the alternative is documenting that in-cluster tenancy is a convention, not a control.
4. **Is `split-namespace` fully covered?** Cross-namespace SANs, `serverName` / SNI, and NetworkPolicy selectors that reference pod labels without a namespace selector all get harder when components are split. The `Certificate` SAN list has to be derived from the same namespace helpers the URLs use.
5. **Does the pre-install validation hook need certificates?** It runs `alloy validate` on the assembled ConfigMap. If a rendered config references `cert_file` paths that do not exist in the validator pod, validation may fail for a reason that has nothing to do with the config's correctness.
6. **Certificate duration.** Short durations reduce the cost of having no revocation mechanism and increase the blast radius of a reload bug. The 90d/30d default above is a starting point, not a researched one.
7. **Does anything here change if the operator runs a service mesh?** The intended answer is "leave it off", but a mesh that transparently re-encrypts a connection the pipeline also encrypts is a double-TLS configuration someone will file a bug about.
8. **One trust bundle or one per direction?** A single concatenated bundle per component is simpler and means the object store's CA is also trusted for peer verification — harmless in practice, sloppy in principle. Separate bundles are more precise and more values. Leaning single, but it should be a decision rather than an accident of implementation.
9. ~~**Does `clusterDomain` belong at the top level or under `global`?**~~ **Settled: `global`.** Not on taste — Loki and Thanos already read `global.clusterDomain` and build memberlist, cache and endpoint addresses from it, so anything else would leave three keys that can disagree. Verified by rendering: setting it changes Loki's own config, not just our SANs. `metrics-server` reads `tls.clusterDomain` and does not participate, so it gets a validator rather than a silent second write.
10. **How far does pgwire go?** Named here because settling it downstream changes what "the LB conventions" means, and the monitoring stack should not harden a precedent that the console and pgwire work then has to fight.
