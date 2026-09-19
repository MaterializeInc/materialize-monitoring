---
title: "Headlamp for Field-Engineering Access to Customer Clusters"
weight: 20260917
# draft=false makes it render as a page
draft: false
publishdate: 2026-09-17
lastmod: 2026-09-17
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5
  status: "Draft"
---

# Headlamp for Field-Engineering Access to Customer Clusters

{{< param-table >}}

This doc proposes that a self-managed Materialize install **ship an optional in-cluster Kubernetes UI — [Headlamp](https://headlamp.dev/) — authenticated through the Ory stack the install already runs for Console SSO**, so that a Materialize field engineer can read and operate a customer's Materialize deployment **without a cloud identity in the customer's account and without a kubeconfig**.
It further proposes a **Materialize-specific Headlamp plugin** that turns the rollout procedure — today a sequence of hand-edits to a custom resource — into an operation with a preflight, a confirmation, and a progress view.

The request comes from Field Engineering, and the stated problem is that reaching a customer's cluster takes too many hoops.
Those hoops are specific, and naming them decides what this proposal is allowed to claim.

| Hoop today | What it is |
|---|---|
| **Cloud IAM credentials** | A principal in the customer's AWS, GCP, or Azure account that can mint a kubeconfig — an IAM role, an EKS access entry, a role assignment |
| **Approval process** | The review that precedes issuing that principal, which is bespoke per customer and re-run per engagement |
| **Customer-side grant** | Work only the customer can do, on their schedule |

Notably absent from that list is the **network path**.
The Ory stack already publishes Hydra, Kratos, the selfservice UI, and Console on reachable, TLS-terminated, source-restricted LoadBalancers.
A field engineer can already reach a customer's Console over the network.
**The front door exists; what is missing is a door into Kubernetes behind it.**

The central claim is that **cluster access for a vendor should be an identity grant, not a credential grant.**
An IAM role is a credential handed to an outside party, scoped by a policy language the customer must reason about separately, and revoked by remembering to revoke it.
A group membership in an identity provider the customer already operates is a grant they can enumerate, scope with Kubernetes RBAC, audit per action, and withdraw in one place.

<!--
Agent note: this doc records decisions and their *why*. When a decision lands in code, update the section and
check the matching row in "Where this work lands".

Four things here are easy to get wrong and are stated deliberately: the honest accounting in "What this
actually removes" (one of the three hoops does not go away), the distinction between authenticating a human to
Headlamp and authorizing that human at the API server in "Four login flows", why flow D is *worse* for vendor
access rather than better, and the `authenticatorKind` reset in "What the plugin does". Revise those rather
than softening them.

Work spans repos. The chart-side deep links and the query contract land here; the Headlamp deployment, the Ory
wiring, and the RBAC land in materialize-terraform-self-managed; the plugin is its own artifact.
-->

## What this actually removes

The proposal eliminates one hoop outright, shrinks a second, and **moves rather than removes the third**.
Stating that plainly is the difference between a proposal that survives its first review and one that does not.

| Hoop | Outcome | Why |
|---|---|---|
| **Cloud IAM credentials** | **Removed** | Headlamp needs no cloud principal. A field engineer never holds an identity in the customer's cloud account, so the class of grant disappears rather than shrinking |
| **Approval process** | **Shrunk** | The approval moves from issuing a cloud credential to adding a group membership. It is reviewed once at install time against a named RBAC role, not re-argued per engagement |
| **Customer-side grant** | **Moved** | The customer still grants access. What changes is that the grant is a one-time configuration with a standing, revocable, enumerable result, rather than a per-case credential handoff |

The honest summary is that **this replaces a recurring, bespoke, high-privilege grant with a single, reviewable, low-privilege one**.
It does not make vendor access self-service, and a design that tried to would deserve the objection it would get.

Two further gains do not appear in the hoop list and are worth as much.

* **The access is legible.** A customer can answer "which Materialize employees can reach this cluster, and what may they do" by reading a group membership and a `ClusterRole`. There is no equivalent answer for a set of IAM policies attached to a role assumed by an outside party.
* **The audit record names a person.** Today the cluster records a cloud principal. Under this design the Kubernetes audit log records the field engineer's own identity on every action.

## Goals

Functional requirements, framed as value-first user stories.
Priority tags (**Must** / **Should** / **Could**) are relative to the first shipped version.
The actors are a **field engineer**, employed by Materialize, and a **customer operator**, who owns the cluster.

- **[Must] As a field engineer,** I want to read pod status, events, and workload state for a customer's Materialize deployment without a cloud identity in their account, so that the largest hoop is gone rather than smaller.
- **[Must] As a field engineer,** I want to trigger and watch a rollout from one place, so that an upgrade is an operation rather than a transcription exercise conducted over a screen share.
- **[Must] As a customer operator,** I want to enumerate which Materialize identities can reach my cluster and what each may do, so that vendor access is a fact I can read rather than a policy I must reconstruct.
- **[Must] As a customer operator,** I want to revoke all vendor access in one action, so that withdrawing it does not depend on finding every place it was granted.
- **[Must] As a customer operator,** I want every action a field engineer takes attributed to that person in my own audit log, so that the record is mine and not a report I am given.
- **[Must] As a customer operator,** I want reading my log lines to be a grant I make separately and knowingly, so that operational access is not silently data access.
- **[Must] As a customer operator,** I want vendor access to be unable to read Secrets in the Materialize namespace, so that a support grant is never a database credential grant.
- **[Must] As a security reviewer,** I want the UI's authorization to be Kubernetes RBAC rather than a second permission model, so that there is one place to read what a field engineer may do.
- **[Should] As a customer operator,** I want to reuse the identity provider the install already runs for Console, so that this adds an application rather than an identity system.
- **[Should] As a customer operator,** I want vendor access to be time-bound, so that it lapses instead of accumulating.
- **[Should] As a field engineer,** I want a preflight before a rollout that names what will change and what it needs, so that a rollout that cannot succeed fails before it starts.
- **[Should] As a customer operator,** I want a rollout never to be able to drop `authenticatorKind`, so that an upgrade cannot silently disable authentication.
- **[Should] As a field engineer,** I want rollout progress expressed as hydration and generation state rather than as pod restarts, so that "is it done" is answerable.
- **[Should] As a maintainer,** I want the plugin to take its queries from the query registry rather than write its own, so that a third surface cannot disagree with the dashboards about what a number means.
- **[Could] As a field engineer,** I want a one-click support bundle scoped to an environment, so that collecting evidence is not a shell script run over a screen share.

## Technical BLUF

- **The identity chain already exists end to end, and this proposal adds two links to it.** The `ory-kratos` module takes an `upstream_identity_providers` list that federates an external OIDC provider as a sign-in method, and its claim mapper already lifts an upstream `groups` claim into `identity.traits.groups`, checking both the flat and `raw_claims` positions. The selfservice UI's consent handler then copies that into `session.id_token.groups`. Federating Materialize's IdP into a customer's Kratos is **one list entry**, not an integration.
- **The missing links are the last two: token to Kubernetes group, and Kubernetes group to RBAC.** Everything before them ships today and is exercised by Console SSO.
- **Authenticating a human to Headlamp and authorizing that human at the Kubernetes API server are separate problems, and the easy solutions only solve the first.** Putting an identity-aware proxy in front of Headlamp produces a login screen and no per-user authorization: every request still reaches the API server as one ServiceAccount, and the audit log records that ServiceAccount. For vendor access that is disqualifying, because the audit record is most of the value.
- **Nothing today configures a Kubernetes API server to trust Hydra as a user identity provider.** The `oidc_issuer_url` variables throughout the cloud modules are workload-identity issuers — IRSA, AKS workload identity — which authenticate pods to the cloud, not humans to the cluster.
- **Impersonation is the flow that works without reconfiguring the API server, and it is what should ship.** A proxy verifies the Hydra-issued token and forwards to the API server with `Impersonate-User` and `Impersonate-Group` headers from the token's claims. Kubernetes evaluates RBAC against the impersonated user and records both identities in the audit event.
- **End-to-end OIDC is *worse* here, which inverts the usual recommendation.** It is the shorter trust path, and it requires the customer to change their API server's authentication configuration. That is a larger, more invasive, more reviewed ask than the IAM role this proposal exists to eliminate. Trading a bounded credential grant for a cluster-wide authentication change is not a trade a customer should be asked to make.
- **The network path is already solved and is not a cost of this proposal.** Ory already publishes Hydra, Kratos, and the selfservice UI on source-restricted LoadBalancers, with `lb_overrides.source_ranges` and `lb_source_cidrs` as the existing controls. Headlamp inherits that pattern and must not default to internet-reachable.
- **`pods/log` is a data-access decision, not a convenience one, and for a vendor it is a contractual one.** An `environmentd` log line can contain query text, object names, and identifiers belonging to the customer. It belongs in a grant the customer makes knowingly and separately.
- **Reading a Secret in the Materialize namespace is reading the backend database credential.** `spec.backendSecretName` names the Secret holding the metadata and persist DSNs. RBAC has no field-level granularity, so `get` on that Secret is the whole of it, and Headlamp will render it. For vendor access this is a boundary, not a default.
- **Break-glass does not belong to the vendor.** `pods/exec` into `environmentd` reaches a SQL session. A field engineer who needs it should be asking the customer, and the absence of a standing grant is what makes that ask visible.
- **The rollout procedure has exactly one silent failure mode, and a plugin removes it.** Materialize's documentation warns that leaving `authenticatorKind` undefined on a subsequent rollout resets it to `None`. A plugin that reads the live spec and patches one field cannot produce that outcome; a human editing YAML can.
- **`ManuallyPromote` is the rollout strategy that most wants a UI.** The strategy set is `WaitUntilReady`, `ImmediatelyPromoteCausingDowntime`, and `ManuallyPromote`, and the last is a rollout that deliberately waits for a human decision. A button is the right interface for a decision.
- **The plugin should not write PromQL or LogQL.** The rollout signals it needs are already defined in the query registry and already plotted by `env-upgrade`. The plugin becomes the registry's third consumer after the dashboards and Console, which is the same argument the [tenant query API doc](../20260916-tenant-query-api/#technical-bluf) makes.
- **Most of this does not land in this repository.** The deployment, the Ory wiring, and the RBAC belong beside `ory-stack`. What lands here is the dashboard deep-link contract and, if the plugin reads telemetry, a dependency on the tenant query API.

## Non-goals

- **Making vendor access self-service.** The customer grants it. This makes the grant smaller, standing, and revocable; it does not remove the customer from the decision.
- **Replacing `kubectl`.** A customer operator keeps theirs. This narrows who needs one routinely, and specifically removes the case where the person needing one works for a different company.
- **Standing elevated access for Materialize staff.** Secret reads and `pods/exec` stay outside every tier defined here.
- **Replacing the Materialize Console.** Console is the interface to a Materialize environment. Headlamp is the interface to the Kubernetes objects underneath it.
- **Replacing the Grafana dashboards.** `env-upgrade` and `env-logs` remain where a rollout or an incident is investigated. The plugin links to them and does not reimplement them.
- **A cross-customer fleet console.** One Headlamp per customer cluster, reached individually. Aggregating across customers is a different product with a different trust model.
- **Authorizing inside Materialize.** Nothing here grants SQL privileges.

## What exists today

| Capability | State | Where |
|---|---|---|
| OIDC issuer in-cluster | ✅ Shipped | Ory Hydra, `ory-stack` (internal); issuer is `https://<hydra_fqdn>` or `https://<single_domain_fqdn>/hydra` |
| Identity store and login flows | ✅ Shipped | Ory Kratos plus the selfservice UI |
| **Federating an external IdP into Kratos** | ✅ **Shipped** | `upstream_identity_providers` on the `ory-kratos` module — one list entry per provider, rendered as a sign-in button |
| **Upstream `groups` claim reaching identity traits** | ✅ **Shipped** | The module's claim mapper reads `groups` from both `claims` and `claims.raw_claims` and writes `identity.traits.groups` |
| `groups` reaching the ID token | ✅ Shipped | The selfservice UI's consent handler copies `identity.traits.groups` into `session.id_token.groups` |
| SAML federation | ✅ Shipped, optional | Ory Polis via `saml_providers`, sharing the same claim mapper |
| OAuth2 client provisioning | ✅ Shipped | `OAuth2Client` custom resources reconciled by Hydra Maester; credentials written to a Secret |
| Reachable, source-restricted front door | ✅ Shipped | LoadBalancers for Hydra, Kratos, and the UI, with `lb_source_cidrs` and `lb_overrides.source_ranges` |
| Console authenticated by Hydra | ✅ Shipped | `authenticatorKind: Oidc`; client audience defaults to `["materialize"]` |
| **ID token becoming a Kubernetes identity** | ❌ **Missing** | Nothing translates a Hydra token into a Kubernetes user |
| **Kubernetes RBAC for a support role** | ❌ **Missing** | No `ClusterRole` describes what a field engineer may do |
| API server trusting Hydra for **users** | ❌ Absent, and deliberately not required | No cloud module sets an API-server OIDC flag or an EKS identity-provider association |
| A Kubernetes UI | ❌ **Missing** | Access is `kubectl` with a kubeconfig |
| Rollout as an operation | ❌ **Missing** | A documented sequence of edits to the `Materialize` resource |
| Rollout observability | ✅ Shipped | `env-upgrade` (`mz-mon-env-upgrade`) — Events, Generations, Reconciliation |

Two asymmetries are the shape of this proposal.

**The identity plumbing is nearly complete and the Kubernetes half is empty.**
An upstream group membership already travels all the way to an ID token claim, because Console SSO needed it.
Nothing consumes that claim as a Kubernetes identity.

**A rollout is already observable here to a level of detail the CLI procedure does not approach, and is operable nowhere except by hand-edited YAML.**
The Generations tab splits a blue/green rollout and counts hydrating collections down to zero.
There is no corresponding way to *start* one except by editing a resource.

## Four login flows

The four differ in one dimension: **who the Kubernetes API server thinks is making the request.**
Everything else — the login screen, the session, the branding — is the same in all four.

| Flow | API server sees | Per-user RBAC | Audit names a person | Asks the customer for |
|---|---|---|---|---|
| **A.** ServiceAccount token, pasted | the ServiceAccount | No | No | Nothing |
| **B.** Ory in front, ServiceAccount behind | the ServiceAccount | No | No | An application and an ingress |
| **C.** Ory plus impersonation | the user, impersonated | **Yes** | **Yes** | An application, an ingress, and an impersonation grant |
| **D.** Ory end to end | the user | **Yes** | **Yes** | **A change to the cluster's authentication configuration** |

### A — ServiceAccount token

Headlamp's default in-cluster posture: a user obtains a ServiceAccount token and pastes it into the UI.

This is a shared, long-lived bearer credential with no notion of who holds it.
Handing one to a vendor is strictly worse than the IAM role it would replace, because at least the IAM role names a principal.
It is listed because it is what an unconfigured install offers, and because leaving it reachable defeats every other flow.

### B — Identity-aware proxy, single ServiceAccount

Headlamp supports [running behind an identity-aware proxy](https://headlamp.dev/docs/latest/installation/in-cluster/identity-aware-proxy/), trusting user information from proxy headers, and supports `--service-account-token-path` so that it authenticates to the API server as its own pod's ServiceAccount.

The result authenticates the human and authorizes nobody.
Every request reaches the API server as `system:serviceaccount:<ns>:headlamp`, so RBAC can only describe what *Headlamp* may do, and the permission set becomes the union of what any user needs, granted to all of them.

For vendor access this fails the requirement that carries most of the value: the customer's audit log would record a ServiceAccount, not the field engineer.
It is worth building only as a **read-only viewer** for a customer's own staff, and is not the flow through which a rollout is triggered.

### C — Impersonation (recommended)

A component holds a ServiceAccount with impersonation rights, verifies the Hydra-issued token on each request, and forwards to the API server with `Impersonate-User` and `Impersonate-Group` headers taken from the token's `sub`/`email` and `groups` claims.

Kubernetes then evaluates RBAC against the impersonated user and records **both** identities in the audit event — the impersonator and the impersonated.
Per-user authorization and a truthful audit trail, on a stock managed cluster, with no change to how the cluster authenticates anyone.

Two implementations, and the choice matters less than the flow.

| Implementation | Status | Trade-off |
|---|---|---|
| Headlamp's own impersonation in in-cluster OIDC mode | Exists; reported broken ([#4198](https://github.com/kubernetes-sigs/headlamp/issues/4198)), with a consolidated opt-in mode proposed rather than shipped ([#5402](https://github.com/kubernetes-sigs/headlamp/issues/5402)) | No extra component; couples a customer's vendor-access path to Headlamp's release cadence |
| A dedicated impersonating proxy between Headlamp and the API server | Established pattern, several implementations | One more component; Headlamp is configured with a kubeconfig whose `server` is the proxy and is otherwise unaware |

The proxy variant is a drop-in because of how Headlamp's OIDC mode already behaves: it sends the ID token as the bearer token to the configured cluster server.
Pointing that server at a verifying proxy rather than at the API server changes nothing in Headlamp.

**The impersonation grant is the sensitive object in this design.**
A ServiceAccount that may impersonate any user and any group is cluster-admin by construction.
It must be restricted with `resourceNames` to the specific groups the support roles bind to, so that the proxy can become `mz-field-observer` and cannot become `system:masters`.
That restriction is also what a customer's security reviewer should be pointed at first.

### D — End-to-end OIDC

The API server is configured to trust Hydra directly, Headlamp performs the authorization code flow, and the user's ID token is the credential the API server validates.
No impersonation, no intermediary, the shortest trust path — and **the wrong recommendation for this use case**.

The reason is the ask, not the mechanism.
Configuring API-server OIDC is a cluster-wide authentication change: an EKS identity-provider association, a set of API server flags or a structured `AuthenticationConfiguration`, or Identity Service on GKE, with AKS effectively out of reach for a third-party issuer.
It touches how *every* principal authenticates to the cluster, it is reviewed accordingly, and on a managed cluster it is frequently a platform-team change with its own change window.

This proposal exists to shrink what a customer is asked to approve.
Trading a scoped, revocable IAM role for a change to cluster-wide authentication does the opposite.

It remains worth supporting where a customer has already configured API-server OIDC for their own staff, because then the ask is zero and flow D is strictly better.
Three details decide whether it works in that case.

* **The issuer must be reachable from the API server and present a certificate it trusts.**
* **The issuer string must match exactly.** The `ory-stack` module builds `hydra_external_url` with no trailing slash and notes that downstream comparison is exact-match. In single-domain mode the issuer carries a path (`https://<host>/hydra`), where OIDC Discovery 1.0 appends `/.well-known/openid-configuration` and RFC 8414 inserts the path. Both the API server and Headlamp use the appended form.
* **The audience must be the client ID the API server expects.** The Materialize client's audience defaults to `["materialize"]`; Headlamp needs its own.

### Recommendation

| Flow | Disposition |
|---|---|
| **C** | The default the deployment configures. Works everywhere, per-user RBAC, correct audit, smallest ask |
| **D** | Supported where the customer has already configured API-server OIDC. Better when it is free |
| **B** | Available as a read-only viewer for the customer's own staff. Not a vendor-access flow |
| **A** | Disabled whenever any other flow is configured. It is a bypass of all of them |

## Who a field engineer is to the customer's cluster

The chain from a Materialize employee's identity to a Kubernetes RBAC decision has six links, and **four of them ship today**.

| Link | Mechanism | State |
|---|---|---|
| Materialize identity carries a group | Materialize's own IdP | Exists |
| That group reaches Kratos | `upstream_identity_providers` entry, `provider = "generic"` | ✅ Shipped |
| The claim becomes an identity trait | The module's claim mapper, reading `groups` from `claims` or `claims.raw_claims` | ✅ Shipped |
| The trait becomes a token claim | The consent handler's `session.id_token.groups` patch | ✅ Shipped |
| The claim becomes a Kubernetes group | `Impersonate-Group` from the verified token | ❌ **New** |
| The group becomes a permission | `ClusterRoleBinding` to a support `ClusterRole` | ❌ **New** |

Two configuration details on the shipped links are easy to miss and break the chain silently.

* **The `scope` default does not include `groups`.** `upstream_identity_providers` defaults to `["openid", "email", "profile"]`, so the Materialize provider entry must request whatever scope its IdP emits groups under. Without it the mapper runs, finds no `groups`, and writes an empty list — a login that succeeds and authorizes nothing.
* **The redirect URI is per-provider.** It is `<kratos public URL>/self-service/methods/oidc/callback/<id>`, registered at Materialize's IdP against the `id` the customer chose.

### The customer chooses whether to federate at all

Federation is the better experience and it is not the only option, and some security teams will decline a dependency on a vendor's IdP.

| Option | Field engineer signs in with | Customer controls | Cost |
|---|---|---|---|
| **Federate Materialize's IdP** | Their Materialize identity | The group-to-role mapping, and whether the provider is configured at all | Trusts Materialize's IdP to assert who is an employee |
| **Per-engineer Kratos identity** | A credential the customer issued | The identity itself, directly | Account lifecycle becomes the customer's problem |

Federation is the recommendation and the second option must remain available, because the RBAC and the flows above are identical either way.
Only the source of the identity differs.

## What a field engineer may do

Headlamp renders what RBAC permits and nothing else, so the roles *are* the design.
Three tiers, bound to groups, and a deliberate fourth that does not exist.

| Tier | Group | Admits | Excludes |
|---|---|---|---|
| **Observer** | `mz-field-observer` | `get`/`list`/`watch` on workloads, pods, events, nodes, PVCs, and the `Materialize` resource | `pods/log`, Secrets, `pods/exec`, every write |
| **Log reader** | `mz-field-logs` | Observer, plus `pods/log` in the Materialize namespaces | Secrets, `pods/exec`, every write |
| **Operator** | `mz-field-operator` | Log reader, plus `patch` on `materializes.materialize.cloud` and `delete` on pods | Secrets, `pods/exec`, writes to arbitrary workloads |
| **Break-glass** | — | **Does not exist** | — |

Six decisions this table is making.

* **`pods/log` is separated from `get pods` on purpose.** An `environmentd` log line can contain query text, object names, and customer identifiers. For a vendor this is a disclosure, and the customer should grant it as one.
* **Secrets are excluded at every tier, as a boundary rather than a default.** `spec.backendSecretName` names the Secret holding the metadata and persist DSNs, with `external_login_password_mz_system` beside them. RBAC cannot grant a field, so `get` on that Secret is the whole of it, and Headlamp will display it.
* **There is no break-glass tier, and that is the design.** `pods/exec` on `environmentd` reaches a SQL session with the container's privileges. A field engineer who needs it asks the customer, and the absence of a standing grant is exactly what makes the ask visible and time-bound by nature.
* **`delete pod` on `environmentd` is a failover.** It belongs in the operator tier, behind a confirmation dialog the plugin can supply and a `kubectl` invocation cannot.
* **Broad `edit` anywhere is an escalation.** Patching a Deployment that mounts a privileged ServiceAccount is a path to that ServiceAccount. The operator tier is scoped to the `Materialize` resource for that reason.
* **Which tier a customer grants is theirs to choose.** Observer is the default the documentation should recommend, because an install that starts at operator has no smaller state to fall back to.

## What the customer controls

Vendor access is only acceptable if the customer can see it, scope it, audit it, and end it.
Four requirements, each naming the object that satisfies it.

| Requirement | Satisfied by | State |
|---|---|---|
| **Enumerate** who can reach the cluster | The `ClusterRoleBinding` subjects, plus the provider entry | Falls out of the design |
| **Scope** what they may do | The three `ClusterRole`s, which the customer may narrow further | Falls out of the design |
| **Audit** what they did | Kubernetes audit events carrying the impersonated user | Requires audit logging to be on, which is not universal |
| **Revoke** in one action | Removing the binding, or the provider entry | Falls out of the design |

Revocation deserves a note.
Removing a `ClusterRoleBinding` takes effect on the next authorization decision, which is immediate.
Removing the Kratos provider entry stops new logins and does not invalidate an existing session, so a complete revocation is both, and the documentation should say so rather than leaving a customer to discover it.

**Time-bounding is a requirement without a mechanism yet.**
Kratos group membership has no native expiry, and a short token lifetime bounds a session rather than a grant.
The requirement stands; see [open questions](#open-questions).

## The Materialize plugin

Headlamp's plugin system is React and TypeScript against `@kinvolk/headlamp-plugin`, with registration points for sidebar entries, routes, detail-view sections and header actions, table column processors, and project views.
Plugins are delivered in-cluster either baked into a static plugin directory or fetched by the chart's plugin manager.

A generic Kubernetes UI shows a `Materialize` resource as a YAML blob with a status block.
The plugin's job is to make it a thing with a state and a set of legal transitions.

### What the plugin does

**A rollout view.**
A `Materialize` details section stating what a generic view cannot: which generation is old and which is new, the `environmentd` version each is running, how far the new one has hydrated, and whether the operator's reconciliation loop is succeeding.
These are the questions `env-upgrade` was built to answer, and the plugin should answer them from the same definitions rather than from pod counts.

**Rollout actions, as detail-view header actions.**

| Action | Effect | Guard |
|---|---|---|
| **Upgrade** | Set `environmentdImageRef` and mint a fresh rollout UUID in one patch | Preflight before patching |
| **Request rollout** | Mint a fresh UUID into `requestRollout` (v1alpha1) or `forceRollout` (v1) | Names which field it is setting and why |
| **Promote** | Complete a `ManuallyPromote` rollout | Offered only when the new generation is ready |
| **Set strategy** | Choose among `WaitUntilReady`, `ImmediatelyPromoteCausingDowntime`, `ManuallyPromote` | States that the middle one causes downtime, in the dialog |

**A preflight that refuses rather than warns.**
A rollout runs both generations at once, so it needs headroom for two.
The preflight reads what the cluster can schedule, the version skew between the operator and the requested `environmentd`, and whether the CRD version in use supports the field the action is about to set — and blocks on a failure instead of rendering a caution that gets scrolled past.

**A guard on `authenticatorKind`.**
Materialize's documentation warns that an undefined `authenticatorKind` on a later rollout resets authentication to `None`.
The plugin reads the live spec and patches one field, so it cannot produce that outcome, and the preflight refuses any patch that would leave the field unset.

**Confirmation that states whose environment this is.**
A field engineer operating a customer's production deployment is the case these dialogs exist for.
Every write action names the environment and the effect before it proceeds, and `ImmediatelyPromoteCausingDowntime` says the word downtime.

**Deep links into the dashboards.**
Every rollout view links to `mz-mon-env-upgrade` scoped to the environment and the rollout's time range; every pod view links to `mz-mon-env-logs` scoped to that pod.
This is the contract that lands in *this* repository: stable dashboard UIDs and a documented variable set a plugin can construct a URL from.

**A support bundle (Could).**
One action collecting the resource, its events, the operator's recent reconciliation log, and the current rollout state into a downloadable archive.
Last, because it is the item most likely to grow into a second product, and because what it collects is a disclosure question of its own.

### Where the plugin gets its numbers

The rollout signals already exist as definitions in `packages/queries/` and are already plotted by `env-upgrade`.
The plugin should consume those definitions rather than restate them, for the same reason panels take both expression and description from the registry.

| Option | Mechanism | Cost |
|---|---|---|
| **Query API** | Read PromQL and LogQL through the authenticated per-tenant endpoint | Depends on the [tenant query API](../20260916-tenant-query-api/), which is itself a proposal |
| **Embedded panels** | Render Grafana panels in the details view | Requires a reachable Grafana and a session in it; weakest interaction |
| **Kubernetes only** | Derive state from the resource, its status, and events | Available today; cannot show hydration progress, which is the number that matters |

The honest sequencing is that **the plugin ships useful on the third option and becomes good on the first**.
Generation state, image refs, reconciliation events, and every action above are Kubernetes-only.
Hydration progress — the number that answers *is it done* — is a metric, and reaching it from a browser is exactly the gap the query API exists to close.

## Where this work lands

| Work | Repository | Status |
|---|---|---|
| Headlamp deployment, as a Terraform module beside `ory-stack` | `materialize-terraform-self-managed` (internal) | ⬜ Proposed |
| Headlamp `OAuth2Client`, and the Materialize provider entry on `upstream_identity_providers` | `materialize-terraform-self-managed` (internal) | ⬜ Proposed |
| The impersonating proxy, and its `resourceNames`-restricted impersonation grant | `materialize-terraform-self-managed` (internal) | ⬜ Proposed |
| The three support `ClusterRole`s and their bindings | `materialize-terraform-self-managed` (internal) | ⬜ Proposed |
| Customer-facing documentation of what vendor access admits and how to revoke it | `materialize-terraform-self-managed` (internal) or Materialize docs | ⬜ Proposed |
| The `Materialize` plugin | Its own artifact | ⬜ Proposed |
| Stable dashboard UIDs and a documented deep-link variable contract | **This repository** | ⬜ Proposed |
| The query-API dependency, if the plugin reads telemetry | **This repository** | ⛓️ Blocked on the [tenant query API](../20260916-tenant-query-api/) |
| Documentation of the rollout operation as an operation | Materialize docs | ⬜ Proposed |

This doc is filed here because the observability half of a rollout already lives here, and because the deep-link contract is a commitment this repository would be making.
The majority of the implementation is not this repository's work.

## Open questions

**How is access time-bounded?**
The requirement is a Must and the mechanism is undecided.
Kratos group membership has no expiry, a short token lifetime bounds a session rather than a grant, and a per-case enablement flow is a feature rather than a configuration.
The nearest cheap answer is that the customer enables the provider entry for an engagement and removes it after, which is a runbook rather than a mechanism.

**Does this require Ory, or merely work best with it?**
An install that has not configured Ory has no OIDC issuer, and flows B, C, and D all need one.
Requiring Ory for Headlamp is defensible and narrows who can adopt it; the alternative is supporting an external issuer directly, which is more configuration surface for the same result.

**Which tier does a customer grant by default, and who decides?**
Observer is the recommendation. Whether an engagement starts there and escalates, or starts at operator, is a commercial and contractual question this doc does not settle.

**Is one Headlamp per customer cluster workable at Field Engineering's scale?**
Each is a separate URL, a separate login, and a separate grant.
That is correct for the trust model and may be poor ergonomics across many customers, and the answer is probably a bookmark list rather than an aggregating console.

**Does the deep-link contract promise anything?**
Dashboard UIDs and variable names are internal under the [deprecation policy](../20260823-deprecation-policy/#committed-surface).
A plugin constructing URLs from them either accepts breakage or promotes them, and promoting them is a cost that should be paid deliberately — the same trade the query API doc identifies for query IDs.

## See also

* [A Tenant-Scoped Query API](../20260916-tenant-query-api/) — the read path this plugin's best version depends on.
* [Deprecation Policy](../20260823-deprecation-policy/) — what promoting dashboard UIDs to a committed surface would cost.
* [Grafana authentication](../../../../dashboards/grafana/auth/) — the same Ory stack, configured for the other web UI in this stack.
* [Headlamp in-cluster installation](https://headlamp.dev/docs/latest/installation/in-cluster/) and [OIDC](https://headlamp.dev/docs/latest/installation/in-cluster/oidc/) (official).
* [Kubernetes user impersonation](https://kubernetes.io/docs/reference/access-authn-authz/user-impersonation/) (official) — the mechanism flow C rests on.
* [Upgrading on kind](https://materialize.com/docs/self-managed-deployments/upgrading/upgrade-on-kind/) (official) — the procedure the plugin replaces.
