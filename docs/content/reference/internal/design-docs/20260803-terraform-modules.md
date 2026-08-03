---
title: "Terraform Modules for materialize-monitoring"
weight: 20260803
# draft=false makes it render as a page
# params.status=Draft is to indicate that the design is not final
draft: false
publishdate: 2026-08-03
lastmod: 2026-08-03
# custom parameters
params:
  author: Heather Lapointe
  status: "Draft"
---

# Terraform Modules for materialize-monitoring

{{< param-table >}}

This doc captures the design for the **Terraform wrapper** on the [Roadmap](../../roadmap/) (M4) and for retiring the hand-rolled observability stack that the [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/materialize-terraform-self-managed) module ships today.
Terraform is deliberately downstream of Helm: the chart stays the full-fidelity surface, and Terraform pins a chart version and supplies the cloud resources around it.

<!--
Agent note: this doc records decisions and their *why*. Work is split across two repos — the common module
and all chart work land here, the cloud wrapper modules land in `materialize-terraform-self-managed`. When
a decision lands in code, update the section and check the matching open question. The "Chart-side
prerequisites" table is the work-in-this-repo list; keep it in sync with the Roadmap.
-->

## Goals

Functional requirements, framed as value-first user stories.
Each describes *what* a user needs and *why it matters*; the [Technical BLUF](#technical-bluf) and the sections below describe *how* we deliver it.
Priority tags (**Must** / **Should** / **Could**) are relative to the first shipped version of the module.

Three stakeholder classes consume this:

- **Teams standing up self-managed Materialize with Terraform**, who follow the recommended module and expect observability to arrive with it rather than as a separate project.
- **Materialize field engineering and support**, who need the deployed stack to be a known, versioned artifact they can reason about during an escalation.
- **Maintainers of this repo**, who need Terraform to consume released artifacts instead of re-implementing them, so a dashboard fix ships once.

- **[Must] As a platform team following the recommended Terraform module,** I want a supported observability stack to come up with my cluster, so that the day the deployment goes live I already have dashboards, metrics, and logs rather than a follow-on project.
- **[Must] As an operator,** I want the dashboards and scrape configuration I get from Terraform to be the same released artifacts the chart ships, so that a fix upstream reaches me by bumping a version rather than by someone hand-copying a JSON file.
- **[Must] As a support engineer on an escalation,** I want to identify exactly which chart version a Terraform-deployed environment is running, so that I can reason about known issues without auditing the cluster.
- **[Must] As an operator,** I want logs, metrics, alerts, and rules — not metrics alone — because the stack Terraform installs today cannot answer a log question at all.
- **[Must] As an operator,** I want telemetry to land in object storage I control with access granted by cloud workload identity, so that retention is durable and no long-lived credentials live in the cluster.
- **[Must] As a maintainer,** I want a chart-shape change that would break the Terraform path to fail in *this* repo's CI, so that the breakage is caught next to the change that caused it rather than in a customer's apply.
- **[Should] As a platform team,** I want to reach the full chart value surface from Terraform without waiting on a module release for every knob, so that an unanticipated requirement does not become a blocker.
- **[Should] As a team with an existing Grafana or Prometheus,** I want to install collection and dashboards without the bundled backends, so that adopting this does not mean running duplicate infrastructure.
- **[Should] As a cost owner,** I want retention and sizing selected from a small set of named shapes, so that I do not have to become a Loki or Thanos topology expert to control spend.
- **[Should] As a maintainer,** I want the observability path exercised end to end against a real cluster *in this repo*, so that a release tag means qualified rather than merely built — downstream integration tests assume we already did this, and today nothing does.
- **[Could] As an operator in a restricted or air-gapped environment,** I want to point the module at a mirrored chart registry, so that the install does not require reaching GHCR.

## Technical BLUF

- **The common module lives in this repo** (`terraform/modules/materialize-monitoring`), next to the chart whose value paths it encodes. **Per-cloud wrapper modules live in `materialize-terraform-self-managed`** and wrap it.
- Terraform **does** support modules calling modules, at any depth, and that repo already does it — `kubernetes/modules/ory-stack` wraps four sibling modules. So a cloud wrapper is idiomatic there, not novel.
- **Delete** `kubernetes/modules/prometheus` and `kubernetes/modules/grafana`. The 330 KB vendored `env-top.json` and the copied `prometheus.yml` go with them.
- **CRDs are a flag, not a module** — `enable_monitoring_crds`, default `true`, driving a second `helm_release` inside the same module.
- **Cloud wrappers stay chart-agnostic.** They create buckets and identities and forward identifiers; the common module owns every chart value path.
- **Terraform owns the `[consumer]` column** of the [shared responsibility model](../../../../operating/production-best-practices/#shared-responsibility-model): buckets, IAM, StorageClass, secrets, version pinning, sizing selection. Nothing else.
- **Values compose as an ordered list** — module-computed wiring first, then the user's raw-YAML overlay last — so the escape hatch always wins and never requires a module release.
- **Chart-shape drift fails fast in this repo**, via the existing `mzmon.validate.collect` error/warning accumulator plus snapshot tests that pin the rendered service-account names.
- **Qualification is ours, not downstream's.** Tiered E2E against kind lives here — chart variant on the `loki-test` profile, Terraform variant against in-cluster rustfs and CNPG — asserted by a new Rust suite. The Terraform repo's cloud tests consume released tags only.
- **One major version bump** in the Terraform repo. The change is acceptable to make bluntly, because `enable_observability` has never been load-bearing.

## Non-goals

- **A Terraform-native reimplementation of the chart.** No `kubernetes_manifest` for dashboards, no Terraform-rendered Alloy config. Terraform installs charts; the chart renders resources.
- **Feature parity between Terraform and Helm.** Helm keeps the full surface by design ([Roadmap](../../roadmap/): "Helm is prioritized over Terraform"). Terraform exposes an opinionated subset plus a raw-values escape hatch.
- **Publishing to the Terraform Registry.** Modules stay consumed by Git ref, as the rest of that repo already is.
- **Managing Grafana content through the `grafana` Terraform provider.** Dashboards and datasources are chart-owned via grafana-operator; a second writer would fight the operator's resync.
- **Migrating existing Prometheus data into Thanos.** The old stack is local-PVC and short-retention; the migration is a cutover, not a backfill.

## What exists today

Two Kubernetes modules, wired into six example roots (`{aws,gcp,azure}/examples/{simple,enterprise}`) behind `var.enable_observability`.

| | `kubernetes/modules/prometheus` | `kubernetes/modules/grafana` |
|---|---|---|
| Upstream chart | `prometheus-community/prometheus` 28.0.0 | `grafana-community/grafana` 12.4.2 |
| Storage | 50 Gi PVC, `ReadWriteOnce`, 15-day retention | 10 Gi PVC (SQLite state) |
| Also installs | kube-state-metrics, node-exporter | — |
| Off | Alertmanager, pushgateway | — |
| Materialize wiring | vendored `prometheus.yml` (annotation-based pod discovery + `mz_compute`/`mz_storage`/`mz_usage` SQL-on-scrape jobs + kubelet cAdvisor) | vendored `env-top.json` (330 KB), delivered as a ConfigMap read by the dashboard sidecar |
| Secrets | — | `random_password` admin, exported as a sensitive output |
| Self-description | — | `# WARNING: unstable as of June 2026 (major changes incoming soon!)` |

Surrounding wiring that the replacement has to keep working:

- The per-cloud `operator` module creates the `monitoring` namespace **and** installs `metrics-server` into it; both monitoring modules are therefore called with `create_namespace = false`, and one example already carries a comment about racing that namespace.
- `operator` and `materialize-instance` each render an `allow-monitoring-ingress` NetworkPolicy selecting the monitoring namespace by `kubernetes.io/metadata.name`, so scraping works under network policy.
- Outputs consumed by the examples: `prometheus_url`, `grafana_url`, `grafana_admin_password`.
- `enable_observability` defaults **false** in `simple` and **true** in `enterprise`. The integration harness applies `examples/simple`, so **the observability path has no CI coverage at all today**.

### Why replace it

1. **Vendored artifacts are already stale and structurally cannot keep up.** `env-top.json` is a point-in-time copy with no version linkage to the released dashboards component, and `prometheus.yml` is a copy of the legacy annotation-based path while the released **Prometheus Scrapers** component is the tested surface. The `mz_compute` / `mz_storage` / `mz_usage` jobs target the SQL-on-scrape endpoints slated for deletion (see [Metrics contract](../../roadmap/#metrics-contract-upstream-dependency)).
2. **It is metrics-only.** No logs, no events, no alerts, no rules, no OTLP export. A log question cannot be answered by this stack.
3. **It is not productionalizable.** Single-replica Prometheus on a `ReadWriteOnce` PVC with 15-day retention and no object storage is a demo shape, not a tier-0 dependency.
4. **It duplicates components the chart bundles.** kube-state-metrics arrives via the Prometheus subchart; metrics-server arrives from the `operator` module.
5. **Dashboard delivery is capped.** A single 330 KB dashboard already sits inside a ConfigMap; the sidecar path runs into the 1 MiB object ceiling well before the shipped dashboard set does.
6. **The wiring is duplicated six times** across example roots, with `depends_on` sets that have already drifted apart.

## Where the module lives

**Decision: the common module ships from this repo; the cloud wrappers ship from the Terraform repo.**

The common module's entire job is to know chart shape — value paths, subchart names, deterministic service-account names, the scheduling fan-out map, which tags enable what.
All of that is a projection of `charts/materialize-monitoring/values.yaml`, and it goes stale the instant the chart moves.
Putting it in this repo makes a chart change and its Terraform consequence **the same pull request**, reviewed together and released together.

Supporting facts, all of which already exist here:

- **Per-component versioning.** `packages/components.yaml` already gives each artifact its own SemVer stream with path-based attribution, and `propose-bumps` opens the bump PR automatically. A `terraform` component slots into that machinery with a `dependencies:` edge on `materialize-monitoring`, so a chart bump proposes a module bump.
- **Snapshot tests.** `charts/materialize-monitoring/tests/` already runs helm-unittest with a `__snapshot__` directory and a `loki_profiles_test.yaml`. The tests that pin the couplings the module depends on belong beside the tests that already pin everything else.
- **Profile access.** The module can read `charts/materialize-monitoring/profiles/*.values.yaml` with `file()` from a path in its own repo, at the same commit as the chart it pins. Cross-repo, it cannot.

What this costs, stated honestly:

- **No Terraform tooling here yet.** `lint.yaml` runs editorconfig, ruff/pyright, cargo, shellcheck/shfmt, yamllint — no `terraform fmt`, `tflint`, or `terraform-docs`. Those get added. It is a small, well-understood addition.
- **Two repos to bump.** The Terraform repo's tag stops solely determining the observability version; its wrapper pins `?ref=terraform/vX.Y.Z` and Renovate proposes the bump. This is the real cost, and it is the same cost the repo already pays for every upstream chart it pins.
- **"We want all our Terraform in one place."** Partially preserved: everything a user writes provider config for — VPC, cluster, buckets, IAM — still lives in the Terraform repo. What moves is the chart-shape translation layer, which is not really Terraform-about-infrastructure so much as Terraform-about-this-chart.

## Proposed module layout

```text
materialize-monitoring/
  terraform/modules/materialize-monitoring/   # NEW — chart + CRDs + secrets + values composition

materialize-terraform-self-managed/
  aws/modules/monitoring/                     # NEW — S3 + IRSA, wraps the common module
  gcp/modules/monitoring/                     # NEW — GCS + workload identity, wraps the common module
  azure/modules/monitoring/                   # NEW — Blob + workload identity, wraps the common module
  kubernetes/modules/prometheus/              # DELETE
  kubernetes/modules/grafana/                 # DELETE
```

Each example root goes from two gated module blocks to one:

```hcl
module "monitoring" {
  count  = var.enable_observability ? 1 : 0
  source = "../../modules/monitoring"

  name_prefix              = var.name_prefix
  namespace                = local.monitoring_namespace
  cluster_oidc_issuer_url  = module.eks.cluster_oidc_issuer_url
  oidc_provider_arn        = module.eks.oidc_provider_arn
  storage_class            = module.ebs_csi_driver.storage_class_name
  node_selector            = local.generic_node_labels

  materialize_instance_namespace = local.materialize_instance_namespace
  materialize_operator_namespace = local.operator_namespace

  additional_values = var.monitoring_additional_values

  depends_on = [module.operator, module.nodepool_generic, module.coredns]
}
```

### CRDs are a flag

Folded into the common module as `enable_monitoring_crds` (default `true`), driving a second `helm_release` installed ahead of the main one, which sets `skip_crds = true`.
A separate module bought nothing: a resource inside a module is still addressable as `module.monitoring.helm_release.crds`, so the teardown blast radius stays targetable without a module boundary.

The flag exists for the one case that matters — a cluster where prometheus-operator or grafana-operator CRDs are already managed by someone else (kube-prometheus-stack, a platform team), where Terraform would otherwise fail trying to create objects it does not own.
Per-group toggles pass through to the CRDs chart's own tags for the partial case.

The teardown caveat still needs stating in the module README: destroying the CRDs release cascades to every `GrafanaDashboard`, `GrafanaDatasource`, `PrometheusRule`, and `PodMonitor` in the cluster, including customer-authored ones.

## Cloud wrapper modules

Terraform supports module composition at any depth, and the Terraform repo already relies on it: `kubernetes/modules/ory-stack` wraps `ory-kratos`, `ory-hydra`, `ory-selfservice-ui`, and `ory-polis` by relative source path.
`count` and `depends_on` both work on module blocks, so `count = var.enable_observability ? 1 : 0` on the wrapper behaves as the examples already expect.

Each wrapper does three things:

1. Creates the cloud resources — one bucket per backend, one identity per backend.
2. Calls the common module with the resulting identifiers plus the properties forwarded from the example.
3. Re-exports the common module's outputs so callers see one surface.

**One bucket per backend, not a shared bucket with prefixes.** Loki and Thanos want different lifecycle and retention policies, IAM scoping is tighter, and Loki's `bucketNames` are bucket names rather than prefixes — a shared bucket pushes the separation somewhere upstream does not model it well. Bring-your-own-bucket stays available.

| Cloud | Buckets | Identity | Notes |
|---|---|---|---|
| AWS | S3 per backend | IAM role per backend, trust policy scoped to `system:serviceaccount:<ns>:<sa>` via the cluster OIDC provider | Follows the existing `aws/modules/storage` IRSA pattern exactly |
| GCP | GCS per backend | Google service account per backend, bound to the KSA with `roles/iam.workloadIdentityUser` | Mirrors `profiles/gcp-example.values.yaml` |
| Azure | Blob containers per backend | Managed identity + federated credential | Lowest-priority cloud, same shape |

Reusing the existing per-cloud `storage` module was considered and rejected: it is Materialize-specific (its outputs are named for the instance's own role), and observability buckets have different retention needs.

### The passthrough tax

Terraform has no variable splat — a wrapper's variable surface is hand-written duplication of the inner module's, and every new input means editing four files.
Mitigation is to keep the forwarded surface deliberately small:

- **Scalars only where they are genuinely cloud-shaped or example-shaped**: namespace, storage class, node selector, tolerations, the Materialize namespaces.
- **One `chart` object variable** for version pinning and registry override, forwarded whole.
- **`additional_values` forwarded verbatim** as the escape hatch, so an unanticipated knob never requires touching the wrapper at all.

That last point is what keeps the tax bounded: growth in chart configurability does not grow the wrapper.

## Values composition

`helm_release.values` takes an ordered list of YAML documents; later documents win.
The module composes:

1. **Wiring** the module computes and the chart cannot know — namespaces, bucket names, identity annotations, the Grafana admin secret name, subchart enablement tags.
2. **Sizing**, from a named shape variable.
3. **`var.additional_values`** — `list(string)` of raw YAML, passed straight through.

Putting the raw-YAML list last is what makes the "reach the full chart surface without a module release" goal true: any value the chart accepts is reachable, and the user's override always wins over the module's opinion.

### Profiles are documentation, with one exception

Profiles under `charts/materialize-monitoring/profiles/` exist to **document** deployment shapes — the AWS and GCP examples are annotated walkthroughs of the IAM wiring, and `grafana-postgres.values.yaml` is mostly prose explaining why SQLite on an `emptyDir` is not a production Grafana state store.
They are not a consumption API, so Terraform not loading them is not a defect. The module computes the cloud overlay anyway, because bucket names and role identifiers *are* Terraform outputs.

**The exception is Loki.** `loki-small`, `loki-large`, and `loki-test` carry foundational overlay content — microservice topology, `schemaConfig`, network policy, storage wiring — that is load-bearing rather than illustrative.
If the Terraform module expresses its own equivalent of those, the two drift, and the failure mode is a topology that renders but misbehaves under load.

Two mitigations, and we should do both:

- **Because the module lives in this repo, it reads those files directly** via `file("${path.module}/../../../charts/materialize-monitoring/profiles/loki-small.values.yaml")` at the same commit as the chart it pins. No re-expression, no drift window. This is the concrete payoff of the repo decision above.
- **Long-term, the foundational parts of the Loki profiles should migrate into chart defaults or a values-level sizing selector**, leaving the profiles genuinely documentation-only. A `sizing: small|large` key is also the only form that works identically under Helm, Terraform, Pulumi, ArgoCD, and Flux, and `tests/loki_profiles_test.yaml` already gives it a home to be pinned in.

### Scheduling and storage class want profiles

Every example passes `node_selector = local.generic_node_labels` and a `storage_class`, and the current modules apply them to their single release.
The umbrella chart surfaces `nodeSelector` / `tolerations` only for the two Alloy releases; there is no global equivalent, and Loki in microservice mode plus Thanos spread across many components with their own key paths.

**Decision: add scheduling profiles to the chart.**
A `profiles/node-selector.values.yaml` / `profiles/tolerations.values.yaml` pair (or one `scheduling` profile parameterized by the values it sets) puts the fan-out map in the chart, where it can be snapshot-tested against the pinned subchart versions, rather than in the Terraform module, where it would be an unverified projection.
Terraform then supplies the *values* — the label map, the toleration list — and the chart owns *where they go*.
This is a better factoring than a `global.scheduling` block for the same reason profiles beat flags generally: the mapping is inspectable and testable as data.

Same shape for the storage class.

## Fail-fast on chart-shape drift

Workload-identity bindings depend on the service-account names the chart renders (`loki`, `thanos-thanos`, `alloy-gateway`).
Those names are deterministic, and keeping them that way is correct — the alternative, having Terraform pick the names, was considered and dropped: it spreads naming across two repos to solve a problem that is really about *detection*.

The real requirement is that a change to a rendered name, or to a value path the module targets, **fails in this repo's CI** rather than in a customer's `terraform apply`.
The machinery for this already exists and is under-used:

- **The validation accumulator.** `mzmon.validate.collect` in `_helpers.tpl` gathers errors and warnings from per-component validators and fails the render through `validate.yaml` (and prints through `NOTES.txt`). Today it aggregates **only** `mzmon.loki.validate` and `mzmon.grafana.validate`. Thanos and Alloy have no validator at all, and `_thanos_helpers.tpl` has no service-account or objstore checks — so the two components with workload-identity bindings are the two with no validation.
- **What the validators should assert.** The chart cannot see an IAM trust policy, so it validates the half it can: object storage enabled implies an identity annotation or explicit credentials is present; the annotation's shape matches the cloud implied by the objstore type (an `eks.amazonaws.com/role-arn` next to `type: GCS` is a configuration error worth catching at render time); the compactor's delete-request store matches the object-store type, which `_loki_helpers.tpl` already does and is the model to copy.
- **What the snapshot tests should pin.** The rendered service-account names, and the resolved subject string `system:serviceaccount:<namespace>:<sa>` for every component with a binding. An upstream subchart renaming its service account then fails a snapshot here, in the pull request that bumps the subchart, with a diff that says exactly what changed. That is the fail-fast the Terraform path needs.
- **Emit the subject, do not make people derive it.** `NOTES.txt` and a module output should print the exact subject strings the consumer needs for their trust policies, so the value that has to match is copy-pasteable and visibly changes when it changes.

## Secrets

The chart consumes secrets by name and does not mint them — the deliberate outcome of the [Loki design doc's secrets decision](../20260627-loki-production-infrastructure/#secrets-strategy).
Terraform is one of the two targets where generation actually works, so it can be the provider.

- **Grafana admin.** The module creates a `kubernetes_secret` from `random_password` (or a caller-supplied value) and points `grafana.admin.existingSecret` at it. Preserves the existing `grafana_admin_password` output and avoids the bundled chart's regenerate-on-upgrade behavior.
- **Object storage.** Workload identity by default, so no secret. Static credentials remain reachable through `additional_values`.
- **Grafana state database.** The chart's default is SQLite on an `emptyDir`, which the `grafana-postgres` profile exists to talk people out of. Terraform is unusually well-placed here: the repo already provisions Postgres per cloud, and GCP's `database` module already takes a `databases` list for additional databases. AWS's takes a single `database_name` and would need extending. Wiring the production Grafana shape from the database the deployment already has is the natural follow-up, and it should be an explicit variable rather than a default in the first version.
- **SQL scrape.** The chart defaults `materialize.environmentdSQL.serviceMonitor.enabled: true` with an empty password, and the Terraform repo provisions no `mz_support` role — that scraper would come up failing auth. The module **defaults it off**, enabling it only when a password is supplied. This matches the direction of travel: the SQL-on-scrape surface is being retired for native endpoints.

## Helm provider semantics

The Terraform repo pins `hashicorp/helm` `>= 2.5.0, < 2.18.0` (provider v2), and installs OCI charts by putting the full reference in `chart` — precedent at `kubernetes/modules/ory-polis/main.tf:218`.
So: `chart = "oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring"` with `version`, no `repository`.
The published chart vendors its subchart tarballs, so apply time needs no access to upstream chart repositories.

Terraform is the **best-behaved** of the delivery targets, worth recording next to the [ordering table in the Loki design doc](../20260627-loki-production-infrastructure/#the-ordering-reality):

| Mechanism | `helm_release` (provider v2) |
|---|---|
| Helm hooks (`pre-install` / `pre-upgrade`) | ✅ full lifecycle — the pipeline pre-validate Job is a real gate here |
| `helm.sh/hook-weight` | ✅ |
| `--skip-crds` | ✅ `skip_crds = true` |
| Ordering between releases | native `depends_on` |
| `lookup`-based templates | ❌ still unavailable — consume-by-name remains correct |

Provider settings that matter:

- `timeout` well above the 300 s default. The existing modules already use 600 s for far less. Start at 900 s and revisit with CI timings.
- `wait_for_jobs = true`, so the pre-validate Job's verdict is actually observed.
- `atomic` deliberately **off**: an atomic rollback on a partial first install hides which component failed, and the diagnostic is worth more than the cleanup on a stack with this many parts.

## Collisions with what the Terraform repo already installs

| Collision | Resolution |
|---|---|
| `operator` module installs `metrics-server` into `monitoring` | Leave the chart's `metrics-server` subchart disabled (already outside the `default` tag). If the caller sets `install_metrics_server = false` on the operator module, the monitoring module flips the `metrics-server` tag instead. One installer either way. |
| Old Prometheus module installed kube-state-metrics | Goes away with the module; the chart's kube-state-metrics subchart is in `default` and takes over. |
| `operator` module owns the `monitoring` namespace | Keep it. The module defaults `create_namespace = false` and takes the namespace as an input, as today. Namespace ownership is a separate change with its own blast radius. |
| `allow-monitoring-ingress` NetworkPolicies select the namespace by name | Unaffected — the name does not change. Worth a CI assertion that scrapes still succeed under network policy, since collection moves from one pod to a DaemonSet plus a gateway. |
| Grafana dashboards previously delivered as sidecar ConfigMaps | Terraform manages those ConfigMaps, so they are deleted on apply. No orphans. |

## Version pinning and compatibility

`chart_version` and `crds_chart_version` default to a pinned pair, bumped as part of a release of the `terraform` component here; the wrapper's `?ref=` is bumped in the Terraform repo.
Renovate runs in both repos and is the mechanism for proposing both.

Three compatibility axes reconcile at pin time, and [Compatibility](../../../compatibility/) is where that gets recorded — including the row it currently lacks, mapping Terraform-repo versions to a chart version:

- **Chart ↔ Materialize.** Dashboards from v0.8.0 want `mz_object_info` (Materialize v26.29.0+, degrading gracefully without it); scrapers from v0.1.1 need the `app.kubernetes.io/name` labels added in v26.24.0. The Terraform repo pins the Materialize version too, so the pair must actually be compatible.
- **Chart ↔ Grafana.** Dashboards from v0.8.0 target dashboard schema v2 and want Grafana v13+; `dashboards.config.grafana.apiTarget` defaults to `dashboard.grafana.app/v2`. The bundled Grafana satisfies this; an `external` Grafana may not, so the module surfaces the knob.
- **Chart ↔ Kubernetes.** The chart declares `kubeVersion: ">=1.27.0-0"`, below every managed-cluster floor the Terraform repo targets.

## Migration

A major version bump in the Terraform repo with a README upgrade note, matching how v6/v7/v8 were handled.

`enable_observability` has never been load-bearing — it defaults to `false` in `simple`, has no CI coverage, and the Grafana module labels itself unstable — so the cutover is a straightforward replacement rather than a delicate migration. Say it plainly and move on:

- The `prometheus` and `grafana` releases and their PVCs are destroyed. Up to 15 days of local Prometheus data goes with them; there is no backfill.
- Grafana state moves off its PVC. The chart default is SQLite on an `emptyDir`, so anything hand-created in the old instance does not carry over and will not survive a pod restart until Postgres is wired (see [Secrets](#secrets)).
- `prometheus_url` is replaced by a Thanos Query endpoint output. Keeping the old name as an alias for one release is cheap courtesy; a clean break is also defensible given the semantics change.
- `grafana_url` and `grafana_admin_password` keep their names and meaning.
- `enable_observability = true` now requires the wrapper module, which needs cluster OIDC inputs.

`enable_observability` keeps its name. It is the right switch, and renaming it would churn six example roots for nothing.

No compatibility shim: the two paths install different charts, and a wrapper pretending otherwise would misrepresent what is running.

## Testing

Qualification happens **here**; the Terraform repo's integration tests are entirely downstream.
They consume a published release tag (Renovate-pinned) and assume our changes are already fully qualified — they are a deployment smoke test against real clouds, not our test suite.

This is a one-directional, tag-gated contract, and it resolves the chicken-and-egg concern from an earlier draft of this doc: downstream never consumes an untagged ref, so there is no cycle.
The corollary is sharper than it looks — **anything the tiers below do not cover ships unqualified.** The tier-2 coverage list is therefore load-bearing, not aspirational.

### Tiers

| Tier | Trigger | Where | Cluster | Storage |
|---|---|---|---|---|
| **0 — static** | every PR | this repo | none | none |
| **1 — chart E2E** | chart changes | this repo | kind | `loki-test` (SingleBinary + filesystem) |
| **2 — Terraform E2E** | Terraform changes | this repo | kind | rustfs (S3-compatible) + CNPG (Postgres) |
| **3 — cloud integration** | released tags | Terraform repo | real EKS / GKE / AKS | real S3 / GCS / Blob |

**Tier 0 — static, no cluster, seconds.**
`terraform fmt`, `terraform validate`, `tflint`, `terraform-docs` (all new to `lint.yaml`), plus two assertions that catch most of what this design can get wrong without ever starting a cluster:

- **Composed-values assertion.** `terraform plan` the test wrapper, pull the composed values out with `terraform show -json`, and `helm template` the pinned chart against them. Every value-path typo, every scheduling fan-out key that no longer exists, fails here in seconds rather than in a 20-minute cluster job.
- **Snapshot tests** in `charts/materialize-monitoring/tests/`, which already runs helm-unittest with a `__snapshot__` directory. Pin the rendered service-account names and the resolved `system:serviceaccount:<ns>:<sa>` subject strings — see [Fail-fast on chart-shape drift](#fail-fast-on-chart-shape-drift).

**Tier 1 — chart E2E on kind**, on chart changes, using the `loki-test` profile: SingleBinary Loki, local filesystem, no network policy, replication factor 1. This is the fast gate.

**Tier 2 — Terraform E2E on kind**, on Terraform changes. A test wrapper provisions **rustfs** for S3-compatible object storage and **CNPG** for Postgres in the kind cluster, then calls the common module exactly as a cloud wrapper would. `loki-small` on PRs, medium on main.

**Tier 3 — cloud integration**, downstream, on released tags only.

### The test wrapper is a fourth cloud

This is the part of the plan that does the most work beyond catching regressions.
The test wrapper plays the same role as the AWS, GCP, and Azure wrappers — provision storage, provision credentials, call the common module — with rustfs standing in for S3 and CNPG standing in for RDS or Cloud SQL.
If kind plus rustfs can satisfy the common module's interface without special-casing, the cloud-agnostic abstraction is real.
If it cannot, the abstraction was wrong and we find out in CI rather than when Azure lands.

That makes tier 2 a design check, not just scaffolding.

### Tier 2 tests the chart harder than tier 1 does

Worth stating explicitly because it inverts the usual assumption.
rustfs exercises the **real object-storage code paths** in both Loki and Thanos — chunk writes, the compactor's delete-request store, Thanos block upload — where `loki-test` runs filesystem mode and Thanos not at all in any storage-meaningful way.
CNPG exercises the **production Grafana state shape** that `grafana-postgres` exists to argue for, including Grafana running its own schema migrations against a database it owns.

Two consequences:

1. **A chart-only change can pass tier 1 and break tier 2.** A change to Loki's or Thanos's storage wiring is exactly the kind that clears a filesystem-mode gate. Tier 2 should therefore be **path-triggered on chart changes too** — anything touching `loki.*`/`thanos.*` storage values or the objstore helpers — and run on main regardless of trigger.
2. **The object-storage variant is worth promoting into a chart-level profile.** If rustfs and CNPG are good enough to qualify the Terraform path, they are good enough for the chart's own gate, and a `kind-integration` profile would let tier 1 opt into the deeper shape when a change warrants it.

### Assertions without a Materialize instance

Materialize scrapers stay disabled — they are integration-tested downstream, and this matches the module defaulting the SQL scraper off (see [Secrets](#secrets)).
That has a precise consequence for what the suite may assert:

- **Materialize-bearing panels have no series.** Assertions against `env-top` must be **structural** — the dashboard exists, its UID and folder are stable, its panels resolve a datasource, its queries return HTTP 200 — not data-bearing.
- **The stack's own telemetry is real data.** Self-observability is on by default (see the [Loki design doc's meta-monitoring](../20260627-loki-production-infrastructure/#meta-monitoring)): Loki, Thanos, Alloy, and kube-state-metrics all emit series into Thanos, and every pod's logs land in Loki. So the suite gets genuine non-empty assertions without Materialize.

The rule to encode: **assert query success everywhere; assert non-empty results only on self-monitoring series.** Getting this backwards produces either a suite that passes while blind or one that flakes on empty Materialize series forever.

### The Rust E2E suite

A fourth workspace member (`packages/mz-monitoring-e2e`), runnable against any variant with flags selecting target behavior.
The Terraform repo's harness is also Rust, so the shape will be familiar, but the two should not share code initially — that one drives cloud lifecycle, this one asserts against a running stack.

| Capability | Mechanism | What it actually proves |
|---|---|---|
| Cluster connectivity helpers | kubeconfig + port-forward, shared across variants | Reusable setup; port-forward is the pragmatic choice on kind |
| See dashboards | Grafana `/api/search`, `/api/dashboards/uid/<uid>` | grafana-operator really pushed them; UIDs and folder placement are stable |
| Explore Thanos and Loki *through Grafana* | Grafana query API against datasource UIDs `mzmon-thanos` / `mzmon-loki` | Datasource provisioning **and** the tenant-header wiring — the `no org id` failure the chart already warns about surfaces exactly here |
| Loki directly | `/ready`, `/loki/api/v1/labels`, `/loki/api/v1/query_range` | Full ingest → index → query round trip |
| Thanos directly | `/-/ready`, `/api/v1/stores`, instant queries for `up` and `scrape_samples_scraped` | Store fanout healthy; targets genuinely being scraped, not merely running |
| Alloy gateway support bundle | The gateway's support-bundle endpoint, downloaded and unpacked | Rendered config matches what the chart intended, component health, discovered target counts — config correctness and liveness in one artifact |

The support bundle deserves the emphasis it gets in the plan.
It collapses "is the rendered config what we meant" and "are the components healthy" into a single fetch, and it is the same artifact support would ask a customer for during an escalation — so exercising it in CI keeps that path honest instead of discovering it is broken mid-incident.
Two things to confirm against the pinned Alloy version: the exact endpoint, and whether it sits behind a stability-level flag.

Two operational notes carried over from the Terraform repo's harness, which already gets both right: every assertion needs **retry with a deadline** rather than a bare poll, and failures should **dump diagnostics** the way `dump_materialize_diagnostics` does — a red E2E with no artifacts costs more than the test saves.

### Gaps this plan does not close

- **Workload identity is untestable on kind.** rustfs takes static credentials; there is no OIDC issuer an IAM provider trusts. IRSA, GKE Workload Identity, and Azure Workload Identity are covered **only** at tier 3, on real clouds — and tier 3 runs after we have already tagged. This is the one place where "fully qualified before release" cannot be literally true. Mitigation is the chart-side validators asserting the *shape* of the identity config (annotation present, and consistent with the objstore type), so a misconfiguration fails at render time even though the binding itself is unexercised locally.
- **There is no `medium` profile.** The profile set is `loki-small`, `loki-large`, `loki-test` — so "medium on main" needs a decision: add `loki-medium`, define medium as small-plus-HA, or read it as `loki-large`. Related and larger: profiles are **Loki-only** today, so "small" and "medium" currently say nothing about Thanos sizing. The plan's vocabulary needs the profile set to catch up.
- **Runner sizing.** Medium-on-main means microservice Loki with real replicas plus Thanos plus Grafana plus CNPG plus rustfs on one kind node. That wants a larger runner and belongs on the post-merge or merge-queue path, not as a PR gate.

This suite is the roadmap's **synthetic-data end-to-end smoke test**, and it promotes the kind half of the **kind / ArgoCD / FluxCD CI matrix** item well above its current "very low priority" — the ArgoCD and Flux halves stay where they are.

### Does LocalStack make more sense than kind?

**No — and rustfs is the better choice over LocalStack's S3 even for the storage half.**

1. **kind is not optional.** `helm_release` needs a real API server, and the thing under test is a Helm chart. LocalStack would be additive, never a substitute.
2. **We are testing an S3 *protocol* client, not AWS behavior.** rustfs is a real S3 implementation; LocalStack is an emulator. For "does Loki's compactor talk to this endpoint correctly," a real implementation is strictly better evidence.
3. **What LocalStack could uniquely add does not give the coverage that matters.** End-to-end IRSA needs a cluster whose OIDC issuer the IAM provider trusts, which LocalStack does not supply. Worse, its IAM policy enforcement is off by default, so a green LocalStack test would give false confidence about least-privilege policies — the exact property we most want evidence for.
4. **The IAM logic lives downstream anyway.** The cloud wrappers are in the Terraform repo, so credential-free plan coverage of trust policies belongs there — where real-cloud tests already run.

The narrow case worth revisiting: if the wrappers accumulate enough bucket-policy and IAM logic to justify plan-level assertions without cloud credentials, LocalStack **in the Terraform repo, for `plan` only** is defensible. Not here, and not as a kind substitute.

## Chart-side prerequisites

Work in **this** repo. None of it is Terraform-specific — each item is a gap that also shows up under Pulumi and ArgoCD.

| Item | Why the module needs it | Blocking? |
|---|---|---|
| Scheduling profiles (nodeSelector / tolerations / priorityClassName) that fan out to subcharts | Every example passes node selectors today; without them the fan-out map lives unverified in Terraform | Not blocking, but it is the module's largest maintenance liability until it lands |
| Storage-class profile, same shape | Examples pass one `storage_class` today | Not blocking |
| Thanos and Alloy validators wired into `mzmon.validate.collect` | The two components with workload-identity bindings have no validation at all | Should land with the storage wiring |
| Snapshot tests pinning rendered service-account names and subject strings | The fail-fast mechanism for the Terraform path's hardest coupling | Should land with the module |
| Subject strings emitted in `NOTES.txt` and as module outputs | Makes the value that must match copy-pasteable rather than derived | Should land with the module |
| Migrate the foundational parts of the Loki profiles into chart defaults or a `sizing` selector | Leaves profiles documentation-only and removes the last drift risk | Longer-term; reading the files directly covers the interim |
| Terraform tooling in `lint.yaml` (`terraform fmt`, `tflint`, `terraform-docs`) and a `terraform` component in `components.yaml` | Prerequisite for hosting the module here at all | Blocking for the module landing here |
| `packages/mz-monitoring-e2e` crate — a fourth workspace member, with cluster-connectivity helpers and the Grafana / Loki / Thanos / Alloy assertions | The assertion engine for tiers 1 and 2 | Blocking for qualification, which downstream assumes |
| kind-based E2E workflow with path-filtered variants (chart vs Terraform, small on PRs / medium on main) | The venue for tiers 1 and 2; `test.yaml` runs cargo test and helm-unittest only today | Blocking alongside the crate |
| A `loki-medium` profile, or a decision that medium means small-plus-HA — plus Thanos sizing in the profile vocabulary | "Medium on main" has nothing to point at; profiles are Loki-only today | Blocking for the tier-2 main-branch variant |
| A `kind-integration` profile (rustfs + CNPG) usable by the chart's own gate | Lets tier 1 opt into the deeper storage shape; today only the Terraform path exercises real object storage | Not blocking, but it closes the tier-1/tier-2 coverage gap |
| Retire the SQL-on-scrape default, or make it fail loud rather than quiet | Terraform provisions no `mz_support` role, so today the scraper comes up failing auth | Module defaults it off; chart cleanup follows |

## Documentation to update

The Terraform story is stale in several places here and should be corrected as the work lands:

- `getting-started/terraform.md` — a work-in-progress stub with a "not available yet (as of May 2026)" warning and two TODOs. Becomes the real install guide.
- `getting-started/_index.md` — carries a TODO and an "as of version: TODO" placeholder.
- `reference/compatibility.md` — states that no version includes materialize-monitoring except its dashboard. Gains the Terraform-repo ↔ chart-version row.
- `reference/internal/roadmap.md` — the M4 "Terraform wrapper module" row, the "Downstream pinning" bullet, and a `terraform` entry wherever components are enumerated. Also two Testing / CI rows: the "Synthetic-data end-to-end smoke test" row **is** the E2E suite described here, and the "kind / ArgoCD / FluxCD CI matrix" row needs its kind half split out and promoted well above "very low priority" (ArgoCD and Flux stay).
- `reference/internal/repo-layout.md` — gains `terraform/`.
- `operating/production-best-practices.md` — the `[consumer]` items become concretely satisfied-by-Terraform on that path, which matters to a reader choosing an install method.

## Open questions

- [x] ~~Does hosting the module here create a chicken-and-egg problem for the Terraform repo's integration tests?~~ **No.** Downstream consumes released tags only, Renovate-pinned, and assumes qualification already happened here. The contract is one-directional.
- [x] ~~Does the integration harness run the object-storage path on every apply?~~ Superseded by the [tier structure](#tiers): real object storage is exercised at tier 2 on kind via rustfs, and at tier 3 on real clouds.
- [ ] What does "medium" mean? Add a `loki-medium` profile, define it as small-plus-HA, or read it as `loki-large` — and does the profile vocabulary grow to cover Thanos sizing at the same time?
- [ ] Confirm the Alloy support-bundle endpoint on the pinned version, and whether it sits behind a stability-level flag.
- [ ] Should tier 2 also gate chart changes that touch storage wiring, or only run on main? Path-filtering is more precise but more machinery to keep correct.
- [ ] Is a `kind-integration` (rustfs + CNPG) profile worth adding so the chart's own gate can reach the deeper storage shape, rather than that coverage living only on the Terraform path?
- [ ] One `scheduling` profile parameterized by values, or separate `node-selector` / `tolerations` profiles? The former is fewer files; the latter composes more cleanly with the existing one-concern-per-profile convention.
- [ ] How much of the Loki profiles is genuinely foundational versus illustrative? That answer sets the size of the "migrate into chart defaults" task.
- [ ] Should the wrapper modules provision the Grafana Postgres database from the existing per-cloud `database` module in the first version, or defer it behind a variable? Deferring ships sooner; not deferring means Grafana state survives a restart out of the box. Note tier 2 covers the CNPG-backed shape either way, so the chart-side path is qualified before the wrapper uses it.
- [ ] Retention defaults for the buckets: what lifecycle policy does the wrapper set, and does it agree with the chart's compactor/retention defaults?
- [ ] Is the `prometheus_url` output alias worth one release, or is a clean break clearer given the semantics change to Thanos Query?
- [ ] Should `enable_observability` default to `true` in `examples/simple` once this lands, matching `enterprise`? This is now purely a product decision — it no longer carries any CI-coverage argument, since qualification moved here.
- [ ] Alertmanager routing has no Terraform input here (no receivers, no upstream integration). Is `additional_values` sufficient for the first version? Note the E2E suite cannot assert delivery without a receiver, so routing stays effectively unqualified either way.
