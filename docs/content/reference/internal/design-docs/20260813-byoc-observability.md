---
title: "Observability for Bring-Your-Own-Cloud"
weight: 20260813
# draft=false makes it render as a page
# params.status=Draft is to indicate that the design is not final
draft: false
publishdate: 2026-08-13
lastmod: 2026-08-13
# custom parameters
params:
  author: Heather Lapointe
  status: "Draft"
---

# Observability for Bring-Your-Own-Cloud

{{< param-table >}}

This doc is the **BYOC gateway-to-gateway architecture and proposal** owed by the [Roadmap](../../roadmap/#byoc) ([DEP-219](https://linear.app/materializeinc/issue/DEP-219)), and it covers the two rows next to it: the [dual-destination pipeline pattern](https://linear.app/materializeinc/issue/DEP-124) and [sanitizing telemetry before control-plane egress](https://linear.app/materializeinc/issue/DEP-220).

The shape is centralized monitoring in our management/control plane, fed by an OTLP ingress that customer environments reach with a client certificate shipped in their license.
Inside the customer environment, `materialize-monitoring` deploys as it does for any self-managed install — the full stack, their Grafana, their retention — and a *second*, reduced copy of telemetry crosses the boundary to us.

The central claim of this doc is that **the customer-local stack is the primary deployment and the control-plane copy is the derivative**, not the other way around.
Every decision below follows from that: the customer keeps more than we do, and no failure of our control plane is allowed to degrade their monitoring.

<!--
Agent note: this doc records decisions and their *why*. Work spans three repos — pipeline/chart/module work
lands here, the stack deployer and cloud wrappers land in the BYOC and terraform repos, and the license
service is control-plane work. When a decision lands in code, update the section and check the matching open
question. The "Chart-side prerequisites" table is the work-in-this-repo list; keep it in sync with the Roadmap.

Note this doc revises a Roadmap claim — see "A roadmap position this changes". Do not let the two drift.
-->

## Goals

Functional requirements, framed as value-first user stories.
Each describes *what* a user needs and *why it matters*; the [Technical BLUF](#technical-bluf) and the sections below describe *how* we deliver it.
Priority tags (**Must** / **Should** / **Could**) are relative to the first shipped version.

Four stakeholder classes consume this:

- **BYOC customers**, who bought a managed experience and expect us to notice problems before they do, without handing us their data wholesale.
- **Materialize SRE and support**, who need to answer "how is this environment doing" for an environment running in infrastructure they cannot log into.
- **Self-managed customers**, who run the stack themselves and may opt in to the same channel to get the same support benefit.
- **Maintainers of this repo**, who need one pipeline codebase to serve both the customer-local and control-plane roles rather than a fork per deployment shape.

- **[Must] As a BYOC customer,** I want Materialize to monitor my environment and act on problems, so that "bring your own cloud" does not also mean "bring your own on-call".
- **[Must] As a BYOC customer,** I want to know exactly what telemetry leaves my account and to see the rules that decide it, so that I can satisfy my own compliance review without taking our word for it.
- **[Must] As a BYOC customer,** I want my own full-fidelity logs and metrics to stay in my account with my retention, so that sending a reduced copy to Materialize costs me nothing in local observability.
- **[Must] As a BYOC customer,** I want a working Grafana at a real hostname with real TLS on day one, so that the monitoring stack is usable rather than a `kubectl port-forward` exercise.
- **[Must] As Materialize support on an escalation,** I want metrics and the logs that explain them for the environment in question, so that triage does not begin with a screen-share request.
- **[Must] As a security reviewer,** I want each environment to authenticate as itself with a credential we can revoke unilaterally, so that a compromised or churned customer loses access without a redeploy on their side.
- **[Must] As a maintainer,** I want a control-plane outage to be invisible to the customer's own monitoring, so that our availability is never a dependency of theirs.
- **[Should] As a self-managed customer,** I want to opt in to the same channel with the same license, so that getting Materialize eyes on my deployment is a flag rather than a project.
- **[Should] As Materialize SRE,** I want one place to see the health of every environment across every customer, so that fleet-wide regressions are visible as fleet-wide rather than as a series of unrelated tickets.
- **[Should] As a cost owner (ours),** I want what crosses the boundary to be tier-selected rather than everything, so that control-plane storage scales with environment count on a known slope.
- **[Could] As Materialize support,** I want a one-shot deep capture from an environment when streaming telemetry is not enough, so that a hard escalation does not require a synchronous customer session.

## Technical BLUF

- **Gateway-to-gateway.** The customer environment's `alloy-gateway` is the client; a control-plane `alloy-gateway` is the server. No new component type — the same pipeline codebase in two roles.
- **The [`egress` seam](#the-egress-seam-is-already-the-extension-point) already exists** for exactly this. `gateway-dest-stub.yaml` defines `loki.process.egress` and `otelcol.processor.filter.egress` as swappable destination tails, and the chart renders the real tail. Multi-destination is a change to that tail, not to the pipeline.
- **Metrics multi-destination is already built and shipping.** `pipeline.metrics.gateway.destination.otel` fans out to several exporters at once, each with its own `minMetricImportance`, alongside `prometheusRemoteWrite`. The `otel-metrics-fanout` profile is the working shape. **The control plane is one more exporter.**
- **The importance tiers are the reduction lever for metrics, and they already exist.** `metric-tiers.yaml` (generated by `mz-monitoring-build gen-metric-tiers`) grades every registry metric `essential` / `recommended` / `extended` / `diagnostic`. The control-plane destination is `essential`; local stays `all`. No new mechanism.
- **Logs are the actual gap.** There is exactly one `loki.write "destination"` and one `egress` seam. Logs need what metrics already have: a destination *list* with per-destination processing. This is the bulk of the pipeline work.
- **Redaction attaches to the destination, not to the pipeline.** A global redaction stage would destroy the customer's own copy to protect ours — the exact inversion of the goal. Per-destination chains are what make "the customer keeps more" true.
- **Redaction is fail-closed and allowlist-shaped for log bodies.** A denylist of patterns is a promise we cannot keep against arbitrary log content; structured fields we recognize cross, and unrecognized bodies do not, unless explicitly opted in.
- **Client certificates come from the license**, verified **twice**: at an L7 load balancer that holds the verifier CA public key and does revocation, and again at `otelcol.receiver.otlp`. The receiver has **no CRL support**, which makes the load balancer the **only** revocation checkpoint — a load-bearing fact, not a detail.
- **Certificate identity is assigned, never asserted.** The LB injects a verified tenant header derived from the client certificate, and the gateway **overwrites** any client-supplied tenant metadata unconditionally. Without that, any valid cert holder can write as any other tenant.
- **Per-destination failure isolation is a hard requirement.** The control-plane branch gets its own queue and its own drop policy, so our unavailability produces a gap in *our* copy and nothing else.
- **Delivery is the stack deployer**, using the same self-managed Terraform modules plus a BYOC profile. The BYOC-specific part is credential material and destination wiring, not a different stack.
- **Self-managed opt-in is the same mechanism with different provisioning** — the license already travels to those clusters, so the channel is a flag plus a certificate.

## Non-goals

- **Replacing the customer-local stack with the control-plane one.** BYOC customers get the full stack in their environment. The control-plane copy is additive and reduced.
- **Shipping raw customer log bodies to the control plane by default.** The default posture is structured, redacted, and reduced. Anything else is opt-in per customer and contractual.
- **A second pipeline codebase for the control plane.** The control-plane gateway runs this repo's pipelines with a different values overlay, or it is a fork we will fail to maintain.
- **Customer access to the control-plane Grafana.** Customers use their own Grafana in their own environment. Ours is an internal surface, and mixing the two turns a tenancy bug into a customer-visible data leak.
- **Replacing the license service or inventing a PKI.** This consumes an issuance path; it does not design the CA hierarchy beyond the requirements it places on it.
- **Real-time or interactive query into customer environments.** This is telemetry egress, not a tunnel. Nothing here opens an inbound path into a customer account.
- **Datadog / GCM / Honeycomb as control-plane backends.** Those stay customer-facing destinations. The control plane is Loki + Thanos, the stack we already operate.

## What exists today

More than a first read suggests.
The pieces below are shipped and in use, and the design deliberately builds on them rather than beside them.

| Capability | State | Where |
|---|---|---|
| Swappable destination tail (`egress` seam) | ✅ Shipped | `packages/alloy-pipelines/gateway-dest-stub.yaml`; real tail rendered by `_alloy_helpers.tpl` |
| Metrics fan-out to N destinations at once | ✅ Shipped | `pipeline.metrics.gateway.destination.otel.*`, `profiles/otel-metrics-fanout.values.yaml` |
| Per-destination metric reduction by importance | ✅ Shipped | `minMetricImportance`, `pre-rendered/metrics/metric-tiers.yaml` |
| Metric denylist | ✅ Shipped | `pipeline.metrics.gateway.denyMetrics` (regex-alternated) |
| OTLP ingress on the gateway | ✅ Shipped | `otelcol.receiver.otlp` — gRPC 4317, HTTP 4318, bridged to both signal paths |
| Loki push ingress on the gateway | ✅ Shipped | `loki.source.api` on 3100 |
| Client-side TLS on destinations | ✅ Modeled | `tls.{ca,cert,key}` + `*Env` on every destination, `minVersion: TLS13` |
| Auth types on destinations | ✅ Shipped | `none` / `basicAuth` / `bearer` / `oauth2` / `sigv4` |
| Multi-tenancy for logs | ✅ Shipped | `pipeline.logging.tenancy.tenantMap` — `static` / `byEnvironment` / `byNamespace` / `byLabel` / `none` |
| Redaction primitives | ✅ In schema, unused | `stage.replace`, `stage.drop`, `stage.limit`, `stage.sampling`, `stage.label_drop`, `stage.structured_metadata_drop`, `stage.match` |
| **Logs to more than one destination** | ❌ **Missing** | One `loki.write "destination"`, one `egress` seam |
| **Per-destination log processing** | ❌ **Missing** | `inputProcessor` is global; everything downstream of it is identical |
| **Receiver-side (server) TLS / mTLS** | ⚠️ `raw` only | `grpc` / `http` nested blocks accept only the `raw` escape hatch in the schema |
| **Certificate issuance** | ⬜ Planned | cert-manager integration, [DEP-195](https://linear.app/materializeinc/issue/DEP-195), OO-M1 |

### The `egress` seam is already the extension point

`gateway.yaml` forwards processed logs to `loki.process.egress.receiver` and processed metrics to `otelcol.processor.filter.egress.receiver` — labels it references but does not define.
`gateway-dest-stub.yaml` supplies them at build time so `alloy validate` can run on a complete graph; at deploy time `_alloy_helpers.tpl` renders the real tail from `pipeline.{logging,metrics}.gateway.destination.*`.

The stub states the contract directly: a deployment may render a different tail as long as it keeps the `egress` labels.
That is precisely the extension point this design needs, and it means **the main pipeline does not change** to gain a second destination.
This is worth stating plainly because it sets the size of the work: dual-destination is a destination-tail feature, not a pipeline rewrite.

### The metrics half is done

This is the most important finding for scoping, and it is easy to miss because the feature was built for SaaS backends rather than for us.

`pipeline.metrics.gateway.destination.otel` already supports **multiple exporters enabled simultaneously**, each with its own `minMetricImportance` and its own handler list, running *alongside* `prometheusRemoteWrite`.
`otel-metrics-fanout.values.yaml` is a working example: Thanos at `all`, Google Cloud Monitoring at `recommended`, Datadog at `essential`.

`metric-tiers.yaml` is generated from the query registry and grades every metric into `essential` / `recommended` / `extended` / `diagnostic`, matched as anchored-regex fragments with each destination unioning "this tier and everything more important".

**So "essential metrics to the control plane" is a values change against a shipped feature.** The control plane is one more OTLP exporter pointed at our gateway, at `essential`, with client-certificate TLS.
What it needs beyond today is the credential wiring and the isolation guarantees below — not a fan-out mechanism.

That reframes the roadmap's [dual-destination row](https://linear.app/materializeinc/issue/DEP-124) ([DEP-124](https://linear.app/materializeinc/issue/DEP-124)): it is a **logs** feature that inherits an already-solved metrics pattern.

## Architecture

```text
CUSTOMER ENVIRONMENT (their cloud account)          │  MATERIALIZE CONTROL PLANE
                                                    │
  alloy-agent (DaemonSet, per node)                 │
    pod logs · journal                              │
        │ OTLP/gRPC (+ node-local WAL, DEP-189)     │
        ▼                                           │
  alloy-gateway  ──────────────────────┐            │
    receivers: loki.source.api :3100   │            │
               otelcol.receiver.otlp   │            │
    kubelet cAdvisor scrape            │            │
    loki.process.inputProcessor        │            │
        │                              │            │
        ├── egress:local ──────────────┼──▶ Loki / Thanos (theirs, full fidelity)
        │     no redaction             │            │        ▲
        │     metrics: all             │            │        │
        │                              │            │   their Grafana
        │                              │            │   (DNS + TLS, configured)
        │                              │            │
        └── egress:controlPlane        │            │
              redact + reduce          │            │
              metrics: essential       │            │
              own queue, own drops     │            │
                    │                  │            │
                    │  OTLP/gRPC 4317, mTLS         │
                    │  client cert from license     │
                    └───────────────────────────────┼──▶ L7 LB  ── verify chain
                                                    │      │      check revocation
                                                    │      │      inject tenant header
                                                    │      ▼
                                                    │   alloy-gateway (control plane)
                                                    │      otelcol.receiver.otlp
                                                    │      verify client cert again
                                                    │      OVERWRITE tenant metadata
                                                    │      per-tenant limits
                                                    │         │
                                                    │         ▼
                                                    │   Loki / Thanos (ours, multi-tenant)
                                                    │         │
                                                    │         ▼
                                                    │   Grafana + Alertmanager (internal)
```

Two properties of this diagram carry the design:

**The customer's path does not traverse ours.** `egress:local` reaches their Loki and Thanos without any dependency on our control plane being up, reachable, or correct.

**The control-plane branch is downstream of a fork, not inline.** Redaction and reduction happen on that branch only, which is what makes "the customer keeps more of their logs" structurally true instead of a policy we promise to apply carefully.

### The control-plane gateway is this repo's gateway

It runs the same rendered `gateway.alloy` with a different values overlay: OTLP ingress with mTLS on, kubelet scraping off, Kubernetes-events collection off, and the egress tail pointed at our own Loki and Thanos.
It is the *receiving* half of a pattern the pipeline already implements — `otelcol.receiver.otlp` bridging into `loki.process.inputProcessor` via `otelcol.exporter.loki` is exactly the path an agent's OTLP already takes today.

Keeping it the same artifact is a deliberate constraint. A control-plane fork would drift within a release or two, and the failure mode is that a pipeline fix ships to customers and not to the system watching them.

## Ingress: OTLP into the control plane

### Two-stage verification, with one revocation checkpoint

The client certificate is verified at both hops, and the split is not redundant — the two hops can check different things.

| | **L7 load balancer** | **`otelcol.receiver.otlp`** |
|---|---|---|
| Needs | Verifier CA public key only | Client CA bundle |
| Chain verification | ✅ | ✅ |
| **Revocation (CRL / OCSP)** | ✅ | ❌ **not supported** |
| Identity extraction → header | ✅ | — (consumes the header) |
| Terminates TLS | ✅ | Re-terminates (LB → gateway is a separate session) |
| Protects against | Unauthenticated and revoked clients, at the edge | An attacker who reaches the pod directly |

The load balancer needs **only the verifier CA public key** — no private material, no per-customer configuration, no state that grows with the fleet. Adding a customer is issuing a certificate, not touching the ingress.

**The receiver cannot check revocation, so the load balancer is the only place revocation is enforced.**
That has a consequence worth writing in the operational runbook rather than discovering later: *any* network path that reaches the gateway pod while bypassing the LB also bypasses revocation.
The mitigations are ordinary and must all be present — the gateway's Service is internal, a NetworkPolicy admits only the LB's source, and no second ingress object targets it.
This is one of the strongest arguments for the NetworkPolicy work already queued for the gateway in OO-M1.

Certificate verification at the receiver is expressible today only through the schema's `raw` escape hatch, since the `grpc` and `http` blocks accept nested blocks as raw text. Typing the `tls` block is listed in the [prerequisites](#chart-side-prerequisites).

### Identity is assigned by the load balancer, never asserted by the client

The load balancer extracts the environment identity from the verified certificate subject or SAN and injects it as a request header.
`otelcol.receiver.otlp` can propagate request metadata downstream with `include_metadata`, which is how the value reaches the pipeline and becomes the Loki tenant.

**The gateway must overwrite that metadata unconditionally, not merely read it.**
If the pipeline trusts a client-supplied header when the LB header is absent — or worse, when both are present — then any customer holding a valid certificate can write into any other customer's tenant.
That is a confused-deputy bug with a customer-data blast radius, and the only safe construction is: strip everything tenant-shaped that arrived from the client, then apply what the LB asserted, then fail the request if the LB asserted nothing.

Fail-closed is the important half. A missing identity header means the request did not come through the verifying LB, and the correct response is rejection rather than a default tenant.

### Why OTLP rather than the Loki push API

Both ingresses exist on the gateway, so this is a real choice.
OTLP wins on three grounds: it is one transport for both signals across the boundary rather than two protocols with two auth configurations; it is the transport the agent→gateway hop is already moving to ([DEP-189](https://linear.app/materializeinc/issue/DEP-189)), so the durability work is shared; and gRPC over a single long-lived mTLS connection is a better fit for a cross-internet hop than repeated HTTP pushes.

The known cost is round-trip fidelity. The metrics bridge already needed `add_metric_suffixes: false` to keep names stable across an OTLP round trip, and the log path has the analogous risk in label ↔ resource-attribute mapping — which is why `gateway.yaml` already carries a TODO for an `otelcol.processor.transform` before the log bridge ([DEP-223](https://linear.app/materializeinc/issue/DEP-223)).
**That TODO becomes load-bearing here**, exactly as it does for the agent OTLP transport. Labels that survive the customer's Loki but not ours produce a control-plane copy that silently fails to join against the same queries — the worst kind of failure, because the data is present and wrong rather than absent.

## The license as credential carrier

The license already travels to every self-managed and BYOC cluster, which makes it the natural carrier for the client certificate, the key, and the control-plane CA bundle.
It also means the observability channel is provisioned by the process that already provisions entitlement, with no second distribution mechanism.

Two constraints fall out of that choice, and one of them is a genuine tension.

**Certificate lifetime should not equal license lifetime.**
Licenses are long-lived — often multi-year. A client certificate with that validity is a long-lived bearer credential sitting in a customer's cluster, and its compromise window is the same length.
Three options, in the order I would rank them:

1. **Short-lived certificates with automated renewal**, where the license carries a bootstrap credential exchanged against a control-plane issuance endpoint. Best security posture; requires an issuance service and an online dependency at renewal time.
2. **Medium-lived certificates (weeks to months) reissued with the license**, accepting a bounded window and leaning on revocation for anything urgent. Simplest; makes CRL freshness at the LB genuinely load-bearing.
3. **Long-lived certificates matching the license.** Simplest to ship and the weakest — a stolen cert is valid for years, and revocation becomes the only control, permanently.

Option 1 is right if the issuance service exists; option 2 is the honest interim. This is flagged as an [open question](#open-questions) because it depends on control-plane capability outside this repo, not on anything here.

**Material must be file-mounted, not carried in environment variables.**
The existing TLS values pass PEM contents through env vars (`GATEWAY_LOKI_DEST_TLS_CERT` and siblings), which is reasonable for inline values but wrong for anything that renews: env vars are captured at process start, so a renewed certificate takes effect only on restart — and under any renewal scheme the whole fleet would fail at once, one certificate lifetime after install.
The [Terraform design doc reached this same conclusion](../20260803-terraform-modules/#mount-the-material-keep-inline-as-the-escape-hatch) for cert-manager and settled on file-mounted material with inline PEM as the escape hatch.
**BYOC should use file mounts from the start**, via the `alloy.mounts.extra` / `controller.volumes.extra` surface both Alloy roles already expose.

Whether a renewed file is picked up by reload-on-change or by a checksum annotation that rolls the workload is [still open in that doc](../20260803-terraform-modules/#mount-the-material-keep-inline-as-the-escape-hatch), and BYOC inherits the answer rather than deciding it separately.

## Multi-destination for logs

### What has to change

Today: `loki.process.inputProcessor` → `loki.process.egress` → `loki.write "destination"`. One chain, one writer, one policy.

Proposed: `inputProcessor` forwards to **N per-destination chains**, each with its own processing and its own writer.

```text
loki.process.inputProcessor
  (global drops, rate limit, level normalization, tenancy — unchanged)
    │
    ├──▶ loki.process.egress.local          ──▶ loki.write.local          (no redaction)
    │
    └──▶ loki.process.egress.controlPlane   ──▶ loki.write.controlPlane   (redact + reduce)
```

`loki.process` already accepts a list in `forward_to`, so the fan-out itself is free.
The work is in the values surface and the helper that renders it: `pipeline.logging.gateway.destination.loki` is a single object today and becomes a **map of named destinations**, each carrying the existing `url` / `retries` / `authType` / `tls` surface plus a new processing block.

Following the metrics precedent matters here.
`destination.otel` is already a map-shaped surface with per-exporter settings, so making logs map-shaped brings the two signals into the same idiom rather than inventing a third.

**Backward compatibility:** the existing single-destination path is the one-entry case. The rendered output for a default install should be byte-identical to today's, which is exactly what the existing snapshot tests are for — and is the cheapest possible proof that a structural values change did not move anything for existing users.

### Per-destination isolation is a hard requirement, not a nicety

Fan-out creates a coupling that does not exist today: **a slow or unavailable destination can back-pressure the shared upstream**, and the control-plane destination is across the internet while the local one is in-cluster.
Left alone, the natural failure is that our control plane having a bad day degrades log delivery to the customer's own Loki. That inverts the entire premise of the design.

Three requirements follow:

- **Independent buffering per destination.** `loki.write` supports a `wal` block (available through the schema's `raw` escape hatch today), so each destination gets its own queue rather than sharing one.
- **Asymmetric drop policy.** The control-plane branch is **droppable**; the local branch is not. When the control-plane queue fills, it sheds and increments a counter. It never blocks.
- **A bounded queue with an explicit ceiling in values.** An unbounded queue converts our outage into the customer's disk-pressure incident, which is a worse failure than the gap it avoids.

The asymmetry is the design statement: **a control-plane outage produces a gap in our copy, and nothing else.**
This should be asserted in tests, not just documented — see [Testing](#testing).

### Reduction for logs needs a vocabulary metrics already has

Metrics reduce by importance tier. Logs have no equivalent, and inventing an unrelated one would be a mistake.

The closest existing structure is the tenancy map, which already classifies logs into `default` / `infra` / `audit` / `environment`.
That classification exists for routing, but it is close to the axis that matters for egress — *what kind of log is this* — and extending it is cheaper and more legible than a parallel taxonomy.

The proposed selection axes for the control-plane branch, most to least valuable:

1. **Class** — infra and Materialize component logs cross; customer query and application logs do not, by default.
2. **Level** — `WARN` and above by default. The gateway already normalizes levels into a canonical set via `stage.replace`, so this selector is reliable rather than pattern-matching per component.
3. **Sampling** — `stage.sampling` for high-volume classes that are useful in aggregate but not line-by-line.
4. **Rate limiting** — `stage.limit` with its own ceiling on the control-plane branch, independent of the global limit in `inputProcessor`.

All four are existing schema stages. None of this needs new pipeline primitives — it needs them composed per-destination and exposed in values.

## Redaction

This is [DEP-220](https://linear.app/materializeinc/issue/DEP-220), and it is the part of the design most likely to be wrong in a way that matters, because its failure mode is silent and irreversible.

### Allowlist, not denylist

A denylist of sensitive patterns is the intuitive design and the wrong one.
It requires enumerating every way a secret can appear in arbitrary log output produced by components we do not all control, and its failure mode is a leak that nobody notices because nothing errored.

**The default posture should be that structured fields we recognize cross, and unrecognized free-text bodies do not.**
Concretely, for the control-plane branch:

- **Labels and structured metadata**: allowlisted keys, since these are the dashboards' join keys and they are enumerable.
- **Log bodies**: dropped by default for customer-workload classes; crossing for Materialize component classes where we control the format, with pattern redaction applied on top as defense in depth rather than as the primary control.
- **`stage.replace`** for known-shaped secrets (connection strings, bearer tokens, keys) applied to everything that does cross — belt and braces, never the only mechanism.

The roadmap already names the sharp example for metrics: the `_info` metric family that made dashboards legible is precisely the family carrying object and cluster names.
The same tension applies to log labels, and the resolution is the same — decide per field, deliberately, with the default being exclusion.

### Redaction must be fail-closed

If a redaction stage cannot be applied — a malformed line, a parse failure, an unrecognized shape — the entry must not cross.
The alternative is a pipeline that ships raw content exactly when its assumptions were violated, which is the moment it is most likely to matter.

This has a direct consequence for the ordering: **redaction runs on the control-plane branch before the writer, and never as a filter the writer can skip.**
Composed as a `loki.process` chain terminating in that branch's `loki.write`, there is no path from `inputProcessor` to the control-plane writer that does not traverse it. That structural guarantee is worth more than any amount of configuration care.

### Customers must be able to read the rules

A customer-facing document that says "we send essential metrics and reduced logs" is not sufficient for a compliance review.
Two concrete deliverables:

- **The rendered pipeline is the artifact.** The Alloy config is already inspectable in the cluster, and the gateway's support-bundle endpoint returns the rendered config. That is a stronger answer than prose: the customer can read what actually runs.
- **A published egress schedule** in the docsite: the metric tier list is already generated (`metric-tiers.yaml`), and the log class/field allowlist should be generated the same way rather than hand-maintained, for the same reason the scraper exclusions are generated — a hand-maintained list drifts on the first rename.

### An opt-out is required, and it should be blunt

Some customers will not permit log egress at any level of redaction.
That must be a supported configuration — metrics-only to the control plane — rather than a negotiation, and it should be a single switch rather than a set of fields to get right individually.
Support quality degrades accordingly, and saying so plainly up front is better than discovering it during an escalation.

## Delivery in BYOC

The stack deployer applies the self-managed Terraform modules into the customer's account, so `materialize-monitoring` arrives the way it does for any Terraform-path install: the common module in `terraform/modules/materialize-monitoring` pinned to a chart version, wrapped by the per-cloud module in `materialize-terraform-self-managed`.

What BYOC adds is a **profile plus credentials**, not a different stack:

| Concern | BYOC handling |
|---|---|
| Control-plane destination wiring | A `byoc` profile in `charts/materialize-monitoring/profiles/`, composed with the cloud and sizing profiles as the assembled shape |
| Certificate material | Delivered by the deployer as a Secret, file-mounted into the gateway |
| Terraform surface | Extends the existing `destinations.tf` pattern — it already models "extra destinations alongside the default" for `google_cloud_metrics` |
| Grafana reachability | The `grafana-ingress` profile plus the per-cloud LB wiring, which [shipped in OO-M1](../20260803-terraform-modules/#reaching-grafana) |
| Grafana persistence | `grafana-postgres`, from the database the deployment already provisions |
| Certificate issuance for in-cluster TLS | cert-manager, on by default on the Terraform path ([DEP-195](https://linear.app/materializeinc/issue/DEP-195)) |

The Grafana rows are worth calling out as **already-solved dependencies rather than new work**.
"BYOC environments all have their Grafana properly configured with DNS and TLS" is a requirement that the OO-M1 reachability and persistence work already satisfies on the Terraform path; BYOC's contribution is that the deployer always sets those values rather than leaving them to the operator.
That is a defaults decision in a profile, not a feature.

**The genuinely new Terraform surface is small:** a control-plane endpoint, a secret reference for the certificate material, the reduction tiers for each signal, and the log-egress opt-out. Everything else composes from what exists.

### Two BYOC-specific risks

**Egress cost and network path.** Cross-account, cross-region telemetry egress is a line item on the customer's bill, and it is our reduction decisions that set it. The tier selection should be presented with an egress-volume estimate rather than only as a fidelity choice, and a private path (PrivateLink or equivalent) is worth evaluating where the customer's account and ours are in the same region.

**Environment identity must be stable and non-guessable.** The tenant key derived from the certificate is what separates customers in our Loki and Thanos. It should be an opaque identifier minted with the license, not a customer-chosen name — both because names collide and because a name is a weaker thing to bind a security boundary to. It must also survive environment recreation, or historical telemetry detaches from the environment it describes.

## Self-managed opt-in

This is the same channel with different provisioning, and it should be deliberately unexciting: the license already reaches these clusters, so the certificate can travel the same way, and the customer flips a flag.

Three differences from BYOC worth designing for:

- **Consent is explicit and per-install.** In BYOC the customer bought a managed service; in self-managed they did not. The flag defaults off and the docs state exactly what crosses when it is on.
- **Not everyone installs via Terraform.** The values surface must be reachable from raw Helm, which it is, provided the profile is a normal composable profile and not something the module synthesizes.
- **Version skew is wider.** Self-managed customers upgrade on their own schedule, so the control-plane ingress must accept older pipeline versions for as long as the deprecation policy ([DEP-127](https://linear.app/materializeinc/issue/DEP-127)) promises. This is the strongest argument for keeping the wire format plain OTLP with the existing label contract, and for the control-plane ingress to have no version-coupled expectations of its clients.

Fleet-wide, this is also the highest-leverage source of signal we do not have today: self-managed deployments are currently invisible to us between escalations.

## Control-plane scale

The control plane aggregates N customers × M environments into one Loki and one Thanos, which changes the sizing question from the single-environment shape the chart's profiles are written for.

- **Tenancy is the primary control.** `tenantMap` already supports `byEnvironment`; the control-plane overlay uses the LB-asserted identity as the tenant, which gives per-tenant limits, per-tenant retention, and a query-time boundary that is enforced rather than conventional.
- **Per-tenant ingestion limits are mandatory.** A single misbehaving environment — a crash loop emitting at full rate — must not degrade ingestion for every other customer. Loki's per-tenant limits are the mechanism; the values need to be set deliberately rather than inherited.
- **Retention is shorter than the customer's.** We hold a reduced copy for triage, not an archive. This is a cost lever and a data-minimization argument at once, and it should be stated in the customer-facing document.
- **Thanos sizing needs its own envelope.** The [`thanos-small` / `thanos-large` profiles are net-new work](../20260803-terraform-modules/#sizing-profiles-medium-is-the-chart-defaults), and the control plane is the deployment most likely to need `thanos-large` plus a documented per-environment series budget. `essential` tier × environment count is a computable number, and it should be computed before this ships rather than discovered.
- **Alerting is fleet-shaped.** Alert rules written for one environment need a per-tenant dimension and grouping that does not page once per environment for a fleet-wide cause. This depends on [Alertmanager adoption](https://linear.app/materializeinc/issue/DEP-216) landing first — without routing, the control plane collects signal nobody is paged on, which is half the value.

## Support bundles

**Open question, with a recommendation.**

There are two distinct things called a support bundle, and conflating them is the main risk.

**The Alloy support bundle already exists** — the gateway's endpoint returns rendered config, component health, and discovered target counts, [enabled by default](../20260803-terraform-modules/#open-questions). It is a monitoring-stack diagnostic: it answers "is collection configured and working", and it is already planned as an assertion surface for the Rust E2E suite.

**A Materialize support bundle** — a one-shot deep capture of an environment's state for a hard escalation — is a different artifact with a different owner, and it is closer to the `materialize` and orchestratord repos than to this one.

The recommendation is to treat them separately and to scope only the first here, with one addition: **the streaming channel should carry enough signal that a bundle is rarely the first resort.**
If every escalation begins with a bundle request, the reduction tiers are set too aggressively, and that is a tuning signal worth watching after launch.

Where this repo plausibly contributes to the second is a **triggered, higher-fidelity window**: on a support request, temporarily raise the tier and lift redaction on the control-plane branch for a bounded period, with the customer's consent, rather than shipping a static artifact. That reuses the entire mechanism above and adds only a time-boxed values change.
It also inherits the mechanism's guarantees, which a separate bundle path would have to re-earn.

Two constraints if we build it: consent must be explicit and auditable, and the elevation must expire on its own rather than requiring someone to remember to turn it off.

## A roadmap position this changes

The [Roadmap's BYOC section](../../roadmap/#byoc) currently states: *"Logs stay inside the customer network; metrics may cross."*

**This proposal revises that.** Limited, redacted, reduced logs crossing the boundary is a deliberate change of position, and the reason is that metrics alone do not make an escalation tractable — the roadmap's own [Upgrades dashboard rationale](../../roadmap/#dashboards) is a worked example of a question ("is it stuck, and what do I do") that metrics describe and logs answer.

The change should be made explicitly in the roadmap rather than left to be inferred from this doc, and the customer-facing framing should be the one this design earns: not "logs cross" but "a redacted, reduced, allowlisted subset crosses, and here is the generated schedule of exactly what".

<!-- Agent note: offer to update roadmap.md's BYOC section when this doc is accepted. The two must not disagree. -->

## Chart-side prerequisites

Work in **this** repo. Ordered roughly by dependency.

| Item | Why it is needed | Blocking? |
|---|---|---|
| **Map-shaped log destinations** — `pipeline.logging.gateway.destination.loki` becomes a named map, rendered as N `loki.process` → `loki.write` pairs off the `egress` seam | The core gap: logs have one destination, metrics have many. Everything else in this doc depends on it | **Blocking** — nothing dual-destination ships without it |
| **Per-destination log processing block** (class, level, sampling, rate limit, redaction) | Makes "the customer keeps more" structural rather than a promise. Global processing would degrade both copies | **Blocking** |
| **Per-destination queue and drop policy** — `wal` per `loki.write`, bounded, droppable on the control-plane branch only | Prevents our outage from degrading the customer's own log delivery. The single highest-severity failure this design can have | **Blocking** |
| **Typed `tls` block on `otelcol.receiver.otlp`'s `grpc` / `http`** in the pipeline schema | Receiver-side mTLS is `raw`-only today, so the control-plane ingress config would be unvalidated at build time | **Blocking** for the control-plane gateway |
| **Tenant metadata overwrite, fail-closed**, on the control-plane receiver path | Without it a valid certificate can write as any tenant. Security-blocking, not merely correctness | **Blocking** |
| **`otelcol.processor.transform` before the log bridge** ([DEP-223](https://linear.app/materializeinc/issue/DEP-223)) | Label ↔ resource-attribute fidelity across the OTLP round trip. Already an open TODO in `gateway.yaml`; shared with the agent OTLP work | **Blocking** — silent label loss is worse than delivery failure |
| **Redaction stage set** composed from existing primitives, allowlist-shaped, fail-closed | [DEP-220](https://linear.app/materializeinc/issue/DEP-220). Primitives exist; the composition and the values surface do not | **Blocking** for log egress |
| **Generated log-class / field allowlist artifact**, alongside `metric-tiers.yaml` | The customer-readable egress schedule, and the thing that stops the allowlist drifting on a rename | Should land with redaction |
| **File-mounted certificate material** for gateway destinations, inline PEM retained as escape hatch | Env-var PEMs break renewal fleet-wide at one certificate lifetime. Shared with [cert-manager work](https://linear.app/materializeinc/issue/DEP-195) | **Blocking** for any renewing credential |
| **`byoc` profile** composing control-plane destination, reduction tiers, redaction defaults, Grafana ingress and persistence | The assembled shape the stack deployer applies | Blocking for delivery |
| **Control-plane values overlay** — OTLP mTLS ingress on, kubelet and events collection off, egress to our backends | Lets the control-plane gateway be this repo's artifact rather than a fork | Blocking for the control plane |
| **NetworkPolicy for the gateway** ([DEP-192](https://linear.app/materializeinc/issue/DEP-192)) | The receiver cannot check revocation, so "the LB is the only path" must be enforced rather than assumed | **Blocking** — it is the revocation guarantee |
| **Per-tenant Loki limits** in the control-plane overlay | One environment's crash loop must not degrade ingestion for the fleet | Blocking for the control plane |
| `thanos-large` sizing plus a per-environment series budget for the `essential` tier | Control-plane sizing is a fleet-count function nobody has computed | Blocking for capacity planning, not for the first environment |
| Alertmanager routing ([DEP-216](https://linear.app/materializeinc/issue/DEP-216)) with a per-tenant dimension | Without routing the control plane collects signal nobody is paged on | Blocking for the value, not the pipeline |
| Snapshot test proving the default single-destination render is unchanged | The cheapest possible proof that a structural values change moved nothing for existing users | Should land with the map change |

## Testing

The kind tiers extend to cover this, and two of the assertions are genuinely new in kind:

- **Tier 1 (chart):** the map-shaped destination renders N writers; the default one-entry case is byte-identical to today's output.
- **Tier 2 (substrate):** a second gateway in the same kind cluster stands in for the control plane. Assert mTLS handshake success, rejection of an untrusted client certificate, and rejection when the identity header is absent.
- **Redaction, positively and negatively.** Feed known-sensitive synthetic lines and assert they are absent from the control-plane destination *and present* in the local one. The second half is the one that catches a global redaction stage that was supposed to be per-destination — a test that only checks the control-plane copy passes happily while the customer's own logs are being destroyed.
- **Tenant spoofing.** Send a client-supplied tenant header alongside a valid certificate for a different tenant and assert the LB-asserted identity wins. This is the confused-deputy case, and it is exactly the kind of bug that is invisible until someone looks for it.
- **Isolation under control-plane failure.** Partition the control-plane gateway while data flows and assert the local destination sees **no gap** and no back-pressure, and that the control-plane branch sheds rather than blocks. This is the design's central promise, so it deserves the same treatment the [WAL durability test](../20260803-terraform-modules/#it-gives-tier-2-a-real-durability-test) gets.
- **Certificate rotation.** Issue a deliberately short-lived certificate, force renewal, and assert delivery continues across it — the failure the env-var carriage produces, invisible to any test that only checks a fresh install.

## Documentation to update

- A **customer-facing egress document**: what crosses, why, how it is reduced and redacted, how to read the generated schedule, and how to turn log egress off. This is the compliance-review artifact and the most important item in this list.
- `reference/internal/roadmap.md` — the BYOC section's three rows, and the "logs stay inside the customer network" line, per [A roadmap position this changes](#a-roadmap-position-this-changes).
- `logs-and-events/storing.md` — a multi-destination section mirroring `metrics/storing.md`'s "Controlling what each destination stores", which is the model to copy.
- `metrics/storing.md` — the control plane as a destination in the fan-out list.
- `operating/production-best-practices.md` — the shared responsibility model gains a BYOC column, since the `[consumer]` items are ours in that deployment rather than the customer's.
- `reference/internal/pipelines/logging.md` — per-destination processing, once the map lands.

## Open questions

- [ ] **Certificate lifetime and renewal path.** Short-lived with a bootstrap exchange (option 1), medium-lived reissued with the license (option 2), or license-lifetime (option 3)? This depends on control-plane issuance capability outside this repo, and it is the single largest security decision in the doc.
- [ ] What exactly is the tenant identity in the certificate — subject CN, a SAN URI, or a custom OID — and does it carry both customer and environment, or only environment with the mapping held control-plane side?
- [ ] Which L7 load balancer, and does the chosen one support CRL *and* OCSP, with a documented refresh interval? "Supports revocation" is not sufficient; a CRL refreshed daily is a one-day revocation window.
- [ ] Does the LB terminate and re-originate TLS to the gateway, and if so what authenticates that second hop? An internal issuer certificate is the obvious answer and should be stated rather than assumed.
- [ ] Should the control-plane gateway accept `loki.source.api` at all, or OTLP exclusively? Exclusively is a smaller attack surface; accepting both eases migration for anything already pushing Loki-native.
- [ ] Log class taxonomy: extend the existing `tenantMap` classes (`default` / `infra` / `audit` / `environment`), or introduce an explicit egress-class label? Extending is cheaper and reuses a shipped concept; a separate axis is more honest if the two classifications diverge.
- [ ] Is `WARN` and above the right default level for control-plane log egress? It is a defensible starting point, but the tuning signal after launch is how often support asks for a bundle — measure before hardening it.
- [ ] Does redaction belong at the customer gateway (before the wire) or the control-plane gateway (after)? **Strong lean: the customer gateway**, so unredacted data never crosses the boundary at all — but the control-plane side is the one *we* can fix without a customer upgrade, and version skew makes that argument non-trivial. Possibly both, with the control-plane side as a backstop.
- [ ] How does the control plane handle a pipeline version older than its own expectations, given self-managed customers upgrade on their own schedule? Plain OTLP plus the existing label contract should make this a non-event, but it needs an explicit compatibility statement.
- [ ] Per-environment series budget at the `essential` tier — what is the actual number, and what fleet size does the first control-plane deployment size for?
- [ ] PrivateLink (or per-cloud equivalent) versus public internet with mTLS: worth it for same-region customers, or premature? Cost, latency, and the customer's own egress bill all point one way; operational complexity points the other.
- [ ] Does the control-plane Loki tenant map to customer or to environment? Per-environment gives tighter limits and cleaner retention; per-customer makes cross-environment queries possible, which is what a support engineer actually wants during an escalation.
- [ ] Support-bundle scope: is the triggered high-fidelity window described above worth building, or does a separate one-shot capture artifact owned outside this repo serve better?
- [ ] Should the customer-facing egress schedule be published in the public docsite, or delivered per-customer with their license? Public is a stronger trust signal and constrains us more.
- [ ] Does the customer get visibility into *their own* control-plane copy — a read-only view of what we hold about them? Attractive for trust, and it reopens the tenancy boundary this doc otherwise keeps closed.
