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

- **Per-component versioning.** `packages/components.yaml` already gives each artifact its own SemVer stream with path-based attribution, and `propose-bumps` opens the bump PR automatically. The module joins the existing **`materialize-monitoring`** component rather than getting one of its own — see [One version, two artifacts](#one-version-two-artifacts).
- **Snapshot tests.** `charts/materialize-monitoring/tests/` already runs helm-unittest with a `__snapshot__` directory and a `loki_profiles_test.yaml`. The tests that pin the couplings the module depends on belong beside the tests that already pin everything else.
- **Profile access.** The module can read `charts/materialize-monitoring/profiles/*.values.yaml` with `file()` from a path in its own repo, at the same commit as the chart it pins. Cross-repo, it cannot.

What this costs, stated honestly:

- **No Terraform tooling here yet.** `lint.yaml` runs editorconfig, ruff/pyright, cargo, shellcheck/shfmt, yamllint — no `terraform fmt`, `tflint`, or `terraform-docs`. Those get added. It is a small, well-understood addition.
- **Two repos to bump.** The Terraform repo's tag stops solely determining the observability version; its wrapper pins `?ref=materialize-monitoring/vX.Y.Z` and Renovate proposes the bump. This is the real cost, and it is the same cost the repo already pays for every upstream chart it pins.
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

**The exception is the sizing set.** `loki-small` and `loki-large` are load-bearing in a way the cloud examples are not — they change resources, WAL, caches, limits, retention, and autoscaling ceilings, and the failure mode of re-expressing them wrong is a topology that renders fine and misbehaves under load.

Because the module lives in this repo it **reads those files directly**, via `file("${path.module}/../../../charts/materialize-monitoring/profiles/loki-small.values.yaml")`, at the same commit as the chart it pins. No re-expression, no drift window. That is the concrete payoff of the repo decision above.

Worth correcting an overstatement from an earlier draft: the sizing profiles are *not* foundational in the sense of carrying storage or schema wiring. `loki-small` states plainly that it defines only the deltas from the medium defaults and that topology stays production-shaped (replication factor 3, ≥3 ingesters) at every size. The genuinely foundational content — `schemaConfig`, `bucketNames`, object-store type, identity annotations — lives in the **cloud example** profiles, and Terraform computes all of that itself. So the drift risk is narrower than first described.

### Sizing profiles: medium is the chart defaults

The convention is already established and should be extended rather than reinvented: **the chart defaults target a medium install**, and profiles are deltas away from it in both directions. `loki-small` and `loki-large` both say so in their headers, and both carry a documented throughput envelope (sustained / 5-minute burst / regression ceiling) plus a "typical fit" line.

Three consequences for this workstream:

- **"Medium on main" means no sizing profile at all** — the tier-2 main-branch variant runs the bare chart defaults. That is a nice property: the default configuration is the one under continuous test.
- **Thanos needs the same treatment, and has none today.** The chart's `thanos` section configures which components are enabled and the compactor's retention policy, but sets **no resources or replica counts anywhere**. So `thanos-small` / `thanos-large` are net-new, and they should mirror Loki's shape: deltas from a medium default, with a documented envelope. Thanos's natural envelope axis is different from Loki's throughput — active series, ingested samples per second, block/retention volume, and query concurrency — so the envelope needs its own vocabulary rather than a copied one.
- **A `kind` profile** that sets only CI-appropriate resource sizes, with no feature management. Keeping it purely a sizing overlay is what makes it composable — `-f kind.values.yaml` layers cleanly over `loki-test` or over the defaults, and it never becomes a second place where features get turned on and off.

Two Thanos-specific things to resolve while sizing it, both of which fail quietly:

- **`queryFrontend` is disabled by default** and the values note it is "only required for production" — but the Thanos datasource URL points at `thanos-query`. Enabling the query frontend in a large profile without moving the datasource to it means caching is deployed and nothing routes through it.
- **Every size stays `receive.mode: standalone`,** and availability comes from the replication factor instead. `standalone` is *RouterIngestor* mode — it already shards across a ketama hashring, so `mode` is a topology choice, not an availability one. `split` is not usable: `receive.ingester` does not inherit from the top-level `receive.*` defaults (a hard swap in `thanos.receive.cfg`, which upstream has confirmed is intentional), so it means restating ~31 keys with eight schema-required sub-objects. What the profiles must set is an **odd** replication factor via `receive.extraArgs` — quorum is `(rf/2)+1`, so 2 tolerates no loss at all while 3 tolerates one. See the [Thanos production checklist](../../../../operating/production-best-practices/#metrics-thanos).

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

## Parity audit

The swap is a net upgrade on nearly every axis, but a handful of things the old stack did are not covered today.
Ordered by severity — and the first is a functional gap in the **chart's own default path**, not a Terraform concern. Terraform only makes it visible, by making the bundled path the default for everyone.

### 1. cAdvisor is not collected on the bundled path

The Kubernetes dashboards depend on cAdvisor directly — `packages/queries/materialize-kubernetes.yaml` builds on `container_cpu_usage_seconds_total`, `container_spec_cpu_quota`, and `container_memory_working_set_bytes`, and several queries are annotated "requires cAdvisor".

The chart does ship the scrape, twice: `pre-rendered/scrapers/prometheus-operator/scrapeconfig-cadvisor.yaml` and a `mz-kubelet-cadvisor` job in the `classic` flavor.
Both are consumed by **Prometheus**, not by Alloy.
The gateway metrics pipeline wires only `prometheus.operator.podmonitors` and `prometheus.operator.servicemonitors` — and Alloy has no `prometheus.operator.scrapeconfigs` equivalent, so a `ScrapeConfig` is inert wherever Alloy is the collector.

The old Terraform stack *did* scrape cAdvisor, because it ran real Prometheus with a raw scrape config.
So on the bundled Alloy → Thanos path, the Kubernetes panels are empty today.
This is the highest-priority item in this doc that is not itself Terraform work.

**Decision: run `prometheus.exporter.cadvisor` in the agent.** There are two ways to get these metrics and the tradeoff is genuine:

| Approach | Per-node cost | Completeness | Dependency |
|---|---|---|---|
| `prometheus.exporter.cadvisor` in the agent | **New** work per node; scales with container count | Full cAdvisor metric set | None — node-local |
| Scrape the kubelet's `/metrics/cadvisor` | Reuses what the kubelet already computes; ~zero marginal | Whatever the kubelet chooses to expose | API-server proxy, or node-level auth |

The cost argument favors the kubelet, and it favors it most in bin-packed environments — containers-per-node is exactly the axis the exporter scales on.
But **completeness favors the exporter, and we already have a documented case of it mattering**: [Compatibility](../../../compatibility/) records that GKE does not expose all cAdvisor and kube-state-metrics metrics, which is precisely why the GCP-optimized dashboards drop some percentage-based panels. Running cAdvisor ourselves makes the metric set ours rather than the platform's, and removes the API-server proxy from the collection path at the same time.

The exporter it is, with the bin-packing risk managed rather than dismissed: explicit resource limits, a footprint measured on the tier-2 medium run so the number is known rather than assumed, and the ability to disable it per node pool for the densest pools. The kubelet scrape stays documented as the fallback — `scrapeconfig-cadvisor.yaml` already has the relabeling written, so switching is cheap if the measured footprint turns out badly.

### 2. Node metrics: node-exporter's known footprint beats Alloy's exporter

The old stack enabled `prometheus-node-exporter`; the chart has no equivalent.
Alloy's `prometheus.exporter.unix` **is** node_exporter, and consolidating into the existing agent DaemonSet is attractive on paper — one fewer workload, and it is already stubbed as a commented placeholder at `packages/alloy-pipelines/agent.yaml:229-230`.

**Take node-exporter anyway, at least first.** The deciding argument is operational, not technical:

- **Known resource envelope.** node-exporter's footprint is well characterized; Alloy-with-exporters is not, and the agent runs on *every* node including the tightly-packed ones. In environments doing heavy bin packing, an uncharacterized per-node request is a real scheduling risk, not a theoretical one.
- **Shared envelope means shared fate.** Folding node metrics into the agent puts logs and metrics under one set of limits. A metrics regression then starves log collection — the signal you most need during the incident it caused. Separate DaemonSets have independent limits, independent eviction, and can be excluded from specific node pools (the Materialize pool carries do-not-disrupt pods; the generic pool does not).
- **Consolidation stays available later**, once the footprint is measured. The reverse — backing a fleet out of a starving agent — is worse.

Consequences worth accepting explicitly: one more DaemonSet, and node-exporter's own scrape needs a collector, which the gateway can do via a ServiceMonitor the subchart already ships.

Also note the schema has **no `prometheus.exporter.*` support**, and the agent pipeline is **logs-only** today (journal + node-local pod logs → `loki.write`), so the Alloy-native route was never the cheap option it looked like.

### 3. Generic annotation-based pod discovery, with exclusions

The old `prometheus.yml` had a `kubernetes-pods` job keyed on `prometheus.io/scrape`, so it collected **any** annotated pod — including a customer's own workloads. The chart's surface is explicit PodMonitors for Materialize components only, so anyone who annotated their own applications loses that collection silently.

Add it — but it cannot be a naive port, because **`prometheus.io/scrape` and exposing `/metrics` are coupled upstream.** Pods that expose metrics carry the annotation, and the chart already collects many of them explicitly. A naive annotation job therefore double-scrapes everything the PodMonitors already cover.

Double-scraping is not merely wasteful. The same series arrives twice under different `job` labels, which breaks `sum()` and `rate()` aggregations and double-counts in exactly the panels people trust. With the OTLP egress change below, both copies land in the same destination with nothing to catch it.

So the job needs a **"already scraped elsewhere" exclusion list**, and the mechanism matters more than the list:

- The shipped monitors select by **label**, not annotation (`app.kubernetes.io/name: environmentd`, and nothing in the scrapers references `prometheus.io/*` at all). So exclusions are expressible as `action: drop` relabel rules on those same label values.
- **Generate the exclusions from the same source as the monitors.** The scrapers are already generated by the `scrape` transpiler in `mzmon-lib`; a hand-maintained name list beside them would drift the first time a component is renamed. Deriving both from one definition makes the invariant structural.
- Cover our own components too, not just Materialize's — Loki, Thanos, Grafana, Alloy, Alertmanager, and kube-state-metrics all have deterministic names and `serviceMonitor.enabled: true`.
- Provide a per-pod opt-out for the case the list cannot know about, and default the whole job **off** so enabling it is a deliberate cardinality decision.

### 4. SQL-on-scrape metric families

The old stack scraped `mz_compute`, `mz_storage`, and `mz_usage`; the chart additionally ships `materialize-sql-mz-frontier`.
With the module defaulting the SQL scraper off (no `mz_support` role — see [Secrets](#secrets)), those families disappear relative to the old stack until upstream native metrics land.

This is deliberate and roadmap-aligned ([Metrics contract](../../roadmap/#metrics-contract-upstream-dependency)), but it must ship as a **named list of which panels degrade**. Unannounced, a blank panel reads as a bug in the new stack rather than a known upstream gap.

### 5. metrics-server is a Console dependency, so phasing it out is a handoff

Worth being precise about, because the failure is silent.
The operator module installs metrics-server with the comment that it is *required for the Materialize Console to display cluster metrics* — and the chart's `metrics-server` subchart sits **outside** the `default` tag.

So setting `install_metrics_server = false` downstream must flip `tags.metrics-server` on in the module's computed values **in the same change**. Do one without the other and the Console quietly loses cluster metrics.

### 6. NetworkPolicy covers only Loki

`networkPolicy` appears exactly once in the chart's values — under `loki`. Thanos, Grafana, Alloy, Alertmanager, and kube-state-metrics have none.

That matters because the Terraform repo sets `enable_network_policies = true`, and the new stack has far more flows than the single scraping pod it replaces: agent → gateway, gateway → Thanos receive, Thanos → object storage and STS, Grafana → Loki / Thanos / Postgres.
This needs an explicit audit before the cutover.

It also has a testing consequence: `loki-test` sets `networkPolicy.enabled: false`, so **tier 1 cannot catch a policy regression**. The tier-2 variant should run with policies enabled.

### 7. Capacity

The old stack asked for 500m / 512Mi (Prometheus) plus 100m / 128Mi (Grafana).
The new one is microservice Loki, Thanos, Grafana, Alertmanager, kube-state-metrics, and two Alloy roles.

The examples' `generic` node pool is sized for the old shape and will likely need to grow — downstream, in the same change as the cutover, or the first apply lands unschedulable pods.

### 8. Smaller deltas

- **Grafana reachability.** Old and new both give an in-cluster URL only — see [Reaching Grafana](#reaching-grafana).
- **Retention.** Old was 15 days on local disk. The new default needs setting deliberately, and agreeing with the bucket lifecycle policy the wrapper creates.
- **Prometheus API consumers.** Thanos Query is Prometheus-API-compatible, so anything pointing at the new URL keeps working; only host and port change. Nothing in the Terraform repo consumes `prometheus_url` beyond re-exporting it.

## Agent → gateway transport: OTLP with a WAL

Planned alongside this workstream. It changes where durability lives, so it belongs here.

```text
agent   loki.source.file (pod logs) ┐
        prometheus.exporter.cadvisor ┘→ OTLP/gRPC + persistent queue (WAL)
                                          ↓
gateway otelcol.receiver.otlp (stateless) → existing fan-outs
                                          ↓
        loki.write (logs)   ·   prometheus.remote_write (metrics → Thanos / AMP)
```

**The gateway stays stateless and the backend destinations do not change.** Thanos and AMP speak Prometheus remote-write, not OTLP, so `prometheus.remote_write` and `loki.write` stay exactly as they are. OTLP is the **agent-to-gateway transport**, and the WAL sits at the first hop. Reading via otelcol is out of scope.

### Why the first hop is the right place for it

This is a better factoring than putting the queue on the gateway, for a reason worth writing down:

- **The agent is the only tier holding data that exists nowhere else.** The gateway can afford to be stateless precisely *because* the agent retries — backpressure propagates upstream into the agent's queue instead of needing gateway disk.
- **The gateway is the scale-out tier.** Keeping it a stateless Deployment preserves horizontal scaling, clustering, and cheap rescheduling; a per-replica volume would force a StatefulSet on the component that most wants to be fungible.
- **It changes an existing recommendation's rationale.** The production checklist asks for ≥2 gateway replicas because the gateway holds in-memory buffers, making a single replica a delivery gap across restarts. With an agent-side WAL, a gateway restart becomes survivable — the replica guidance stays good practice but stops being the only thing standing between a restart and data loss. That is a doc update in `operating/production-best-practices.md`.
- **The module barely changes.** No StorageClass consumer, no StatefulSet, no PVC. I had this wrong in an earlier draft; the corrected design touches Terraform almost not at all.

### The delta is agent-side only

The receiving half already exists. The gateway runs `otelcol.receiver.otlp` with both gRPC and HTTP, its Service already exposes 4317 and 4318, and OTLP logs already bridge into the same `loki.process.inputProcessor` that `loki.source.api` feeds via the existing `otelcol.exporter.loki`. Metrics have the mirror-image bridge in `gateway-metrics.yaml`. So the gateway needs nothing.

What the agent needs:

- **An otelcol path at all.** The agent is `loki.*`-only today: journal and pod logs into `loki.process`, out through `loki.write` to the gateway's port 3100. It has no otelcol components.
- **Bridges in both signal directions** — logs from `loki.process` into OTLP, and cAdvisor metrics from the exporter into OTLP. The gateway already demonstrates both bridge patterns in reverse, so the shapes are known.
- **`otelcol.exporter.otlp` with a persistent sending queue.** Component and attribute names for the file-backed queue need confirming against the pinned Alloy version, along with whether to type these in the schema or use the `raw:` escape hatch the existing otelcol blocks all use.
- `loki.source.api` on the gateway **stays** — third parties send their own logs there, so this is not a replacement of that ingress.

### Two correctness details that are easy to miss

**The WAL and the positions file must share a lifetime.** `loki.source.file` tracks a read offset in its positions file; the OTLP queue holds read-but-unsent data. If the queue is lost while positions have advanced past it, those lines are gone — the positions file will not re-read them. Both must live on the same node-local persistence with the same durability, or the WAL provides less than it appears to.

Note the asymmetry this creates: logs are re-readable from the node as long as positions are intact, so a lost queue is recoverable in principle. **cAdvisor metrics have no re-readable source** — a lost queue is a permanent gap. The WAL matters more for metrics than for logs.

**Backing: `hostPath`, `/var/lib/mzmon-alloy` → `/var/lib/alloy/wal`.** The agent is a DaemonSet already mounting host paths to read `/var/log/pods`, so node-local disk is the natural home and no PVC or StorageClass enters the picture. `hostPath` also survives pod restarts, which is what makes the positions/WAL lifetime guarantee above hold.

Sizing is bounded by compaction — default threshold ~100 MiB, with compaction-on-start enabled so the on-disk file is reclaimed rather than growing monotonically across restarts. That keeps the footprint modest enough that node disk pressure is not a live concern; the one thing to keep in view is simply that this is *shared node disk*, so the bound should stay explicit in values rather than inherited silently.

**Round-trip fidelity.** The metric bridge already had to set `add_metric_suffixes: false` to keep names stable across the OTLP round trip. The log path has the analogous risk in label ↔ resource-attribute mapping, and `gateway.yaml` already carries a TODO for an `otelcol.processor.transform` before bridging to handle resource attributes. That TODO becomes load-bearing once the agent's logs arrive as OTLP rather than through `loki.source.api`.

### It gives tier 2 a real durability test

The assertion the E2E suite was missing a reason for: partition the gateway (or delete it) while data is flowing, restore it, and assert **no gap** in either the metric series or the log stream across the outage.

`hostPath` backing makes a second assertion available and worth having — restart the agent pod itself, not just the gateway, and assert the queued data still arrives. That is the case a PVC-less DaemonSet would normally lose, and it is the one that proves the host mount is actually doing its job.

Both work on kind and both are only meaningful against a real backend, so they belong on the rustfs + CNPG tier-2 variant. This is the test that would catch a regression in the guarantee the change exists to provide.

## In-cluster TLS and authentication

Three hops need authenticating, and none of them are today: **agent → gateway**, **gateway → Loki / Thanos**, and **Grafana → Loki / Thanos**.

### What already exists, and what does not

The **client half is modeled**. Every pipeline destination carries a TLS block with CA, client cert, and client key, and `minVersion: TLS13` — `AGENT_LOKI_DEST_TLS_*` for agent → gateway, `GATEWAY_LOKI_DEST_TLS_*` and the Prometheus-destination equivalents for gateway → backends. `authType` (`none` / `basicAuth` / `bearer` / `oauth2` / `sigv4`) is modeled alongside it.

What is missing is everything around that:

- **Nothing issues the certs.** There are no `Certificate` templates and no cert-manager integration anywhere in the chart. The values assume an operator supplies PEMs out of band.
- **The server halves are unwired.** The gateway's OTLP and `loki.source.api` receivers, Loki's own HTTP server, and Thanos receive are not configured to present certs or require client ones. A configured client with an unconfigured server is just TLS-off.
- **Grafana → backends has no cert path.** `connections.datasources.*.valuesFrom` can inject `secureJsonData`, so client certs are *expressible*, but nothing models or defaults them.
- **`authType: none` is the default on every destination**, and the production checklist already flags `loki.write` auth as not yet wired.

### cert-manager stays an optional dependency

cert-manager is an upstream dependency we want to **encourage** in production, not one the chart can require. Plenty of installs will not have it, and the CRDs chart has no business pulling in a separate ecosystem's CRDs.

So the split falls out cleanly along the [shared responsibility model](../../../../operating/production-best-practices/#shared-responsibility-model):

- **The chart** keeps certificates **opt-in and off by default**, renders no `Certificate` resources unless enabled, and continues to accept operator-supplied material for anyone bringing their own PKI.
- **The Terraform path turns them on by default**, because cert-manager and a ClusterIssuer are already in that stack. This is the consumer supplying what the chart consumes by name — the same shape as buckets and workload identity.

That also means the chart cannot treat missing cert-manager CRDs as an error, and the docs should frame cert-manager as the recommended production path rather than a prerequisite.

### Mount the material, keep inline as the escape hatch

The existing TLS values carry **PEM contents through environment variables**, which is the natural way to express a certificate inline in `values.yaml` — reasonable for the case it was written for.

It is the wrong carrier for cert-manager, though, and specifically for **renewal**: env vars are captured at process start, and cert-manager renews by rewriting the Secret in place, so an env-injected cert would keep working for exactly one certificate lifetime and then fail everywhere at once.

**Decision: file-mounted material for the cert-manager path** (`ca_file` / `cert_file` / `key_file` against a mounted Secret), with inline PEM retained as the escape hatch for values-only and bring-your-own-PKI users.

The chart already has the surface for this and an established pattern to copy — both Alloy roles expose `alloy.mounts.extra` and `controller.volumes.extra`, each already carrying a `tmp` `emptyDir` example. Mounting a cert Secret follows the existing convention rather than introducing a new one.

One thing file mounts do not settle by themselves: the kubelet updates mounted Secret contents on renewal, but the process still has to notice. Reload-on-change versus a config-checksum annotation that rolls the workload is still an open call.

### Mirror `materialize-instance`'s issuer variables exactly

The sibling module already solved the naming, and its split is the one this stack needs:

- `issuer_ref` — the browser-facing certs, i.e. the Grafana LB.
- `internal_issuer_ref` — cluster-internal mTLS with `*.cluster.local` SANs.

Its variable documentation spells out why the two cannot be one: a public ACME issuer such as Let's Encrypt **cannot sign `cluster.local`** and rejects single-label domains. That constraint applies here identically.

Using the same variable names and semantics means the examples pass one set of locals to both modules, and an operator who understands certs for their Materialize instance already understands them for their monitoring stack. The Terraform repo also already ships `kubernetes/modules/cert-manager` and `kubernetes/modules/self-signed-cluster-issuer` (which outputs `issuer_name`), so the wiring exists — nothing new is needed downstream beyond passing it through.

### Bootstrap and trust domains

cert-manager and the issuer have to exist before the chart. Under Terraform that is a `depends_on`; for Helm-only users it is the same "install this first" story the CRDs chart already has. Per the [ordering reality](../20260627-loki-production-infrastructure/#the-ordering-reality), the chart must still converge if certs are not ready yet — crashloop-and-retry, not a hard pre-flight failure.

Two trust domains coexist and should not be conflated: **in-cluster** material from the issuer, and **external destinations** (a remote Loki, AMP, an OTLP forward target) using a public or customer CA. The module wires the first and leaves the second to `additional_values`.

### Testing

Tier 2 should run with mTLS enabled end-to-end — otherwise the auth surface ships unqualified, the same argument as network policy.

The test that earns its keep is **rotation**: issue a deliberately short-lived certificate, force renewal, and assert the pipeline keeps delivering across it. That is precisely the failure the env-var shape produces, and it is invisible to any test that only checks a freshly-installed stack.

## Reaching Grafana

> [!INFO]
> **Shipped as designed.** This section is the original problem statement and is kept as the record; "today" below means August 2026, before the work landed. For current behaviour see [Reaching Grafana](../../../../getting-started/terraform/#reaching-grafana) on the Terraform path and [Reaching Grafana](../../../../dashboards/grafana/architecture/#reaching-grafana) for the chart.
>
> Two things came out differently. **Ingress is not preferred everywhere** — an Ingress and an annotated `LoadBalancer` Service are two ways of asking the same cloud for a load balancer, so the wrappers follow what each platform's controllers consume: Ingress on AWS, annotated Service on GCP and Azure. And **the allowlist is required even for an internal load balancer**, because the chart cannot see the internal-scheme annotation, which makes the CIDR list the only thing that renders the intent legible to it.

**Grafana is not reachable today.** The chart's `grafana` block surfaces `fullnameOverride`, `namespaceOverride`, `replicas`, `grafana.ini`, and `serviceMonitor` — no service type, no ingress. The upstream default is `ClusterIP`, so the only access path is `kubectl port-forward`.

This is parity-neutral — the module being replaced also emitted an in-cluster service DNS name as its `grafana_url` — but it stops being acceptable when Grafana is the primary interface to the whole stack rather than a bundled extra.

### Express it in the chart, wire the cloud specifics in Terraform

Surface `grafana.ingress` and `grafana.service` (the upstream chart supports both) so Helm-only users get the same capability, and let the module supply cloud-specific annotations, the hostname, and the certificate reference. The alternative — building a bespoke LB path in the Terraform module — would leave Helm users with nothing and put chart-shape knowledge somewhere new.

On AWS, prefer **Ingress through the load-balancer controller** over the repo's `nlb` module. Grafana is ordinary HTTP wanting host-based routing and a certificate, which is what Ingress is for. The Console uses the NLB module for reasons specific to it — OIDC redirect and port constraints that pushed it onto 443 with a target-group setup — and those do not apply here.

### Follow the house LB convention

The Terraform repo already has the right posture, and the monitoring module should copy it rather than invent one:

- **Internal by default.** `aws/modules/nlb` defaults `internal = true`, and `internal_load_balancer` defaults to `true` in the examples.
- **Public requires a CIDR allowlist**, and — worth copying specifically — the NLB module *enforces* it with a `validation` block: `ingress_cidr_blocks` must be present and contain valid CIDRs whenever `internal = false`. Copy the validation, not just the default. An unenforced default is a convention; an enforced one is a guardrail.
- Terminate TLS with `issuer_ref` (the external issuer, per the section above) or ACM on AWS.

**SSO is out of scope for now.** It is worth working out eventually — the enterprise example already runs Ory, so OIDC is available rather than hypothetical — but the shipped posture is internal-by-default plus an enforced allowlist for public, with the generated admin password. Document that combination as the boundary, so nobody reads "Grafana is exposable" as "Grafana is safe to expose broadly."

### Output behavior changes

`grafana_url` becomes the external URL when Grafana is exposed and stays the in-cluster service name otherwise. Worth calling out in the upgrade note, since the output keeps its name while its meaning becomes conditional.

## One version, two artifacts

**Decision: the Terraform module ships as part of the existing `materialize-monitoring` component, not as a separate one.**

The component stops meaning "the Helm chart" and starts meaning "the materialize-monitoring release", which has two artifacts: the chart and the Terraform module. Concretely, in `packages/components.yaml`:

- `content_paths` gains `terraform/`, so module changes attribute to this component.
- `version_paths` stays `charts/materialize-monitoring/Chart.yaml` — the chart version *is* the release version. Terraform modules carry no in-tree version file; they are versioned by Git tag, and `<component>/vX.Y.Z` already supplies one.
- `artifacts` is unchanged. The module is consumed by Git ref, so there is nothing to attach.
- `title` should be reworded off "Helm Chart".

### Why coupling beats a separate stream

A separate `terraform` component would model the two as independently evolving. They do not:

- **The ref becomes the answer.** `?ref=materialize-monitoring/v0.9.0` installs chart v0.9.0. Which chart a given module ref deploys is readable from the ref itself — no mapping table, no compatibility matrix between our own two artifacts.
- **No mismatch window.** With separate streams, a chart bump proposes a module bump as a *second* PR, and between the two merges the pair is inconsistent. Shared versioning removes the window rather than narrowing it.
- **It matches the actual dependency.** The module's whole job is encoding chart value paths; it is not independently useful. A separate SemVer stream would advertise an independence that does not exist.
- **One changelog entry** for a change that spans both, which is the common case.

### The costs, stated plainly

- **A Terraform-only change publishes a chart release.** `propose-bumps` bumps `Chart.yaml`, so a module fix ships a chart version that is byte-identical to its predecessor except for the version, plus an OCI push. Cheap, but real, and Helm-only users will see it. A `### Terraform` changelog subsection keeps it legible.
- **A breaking module change forces a major bump of the chart version.** This is the sharper cost: renaming a module input is not a breaking *chart* change, but shared versioning makes the chart go to a new major anyway. Mitigations: prefer additive changes with deprecation over renames, and make the changelog say which surface broke — the [customer-facing surface subsection](../../roadmap/#versioning-changelog-and-releases) already contemplated for the deprecation policy is the right home. The alternative is not obviously better: two version numbers to reconcile is its own confusion.
- **Every chart release re-pins the module downstream**, so Renovate fires on each one even when the module did not change. That is arguably correct — a chart release can move values the module sets — and downstream can group or schedule those PRs.

The `materialize-monitoring-crds` chart stays a separate component, and the module pins it separately via `crds_chart_version`. That is the one pin this coupling does not cover, which is right: the CRDs chart deliberately has a looser lifecycle.

## Compatibility

Three compatibility axes reconcile at pin time, and [Compatibility](../../../compatibility/) is where that gets recorded.
Coupled versioning collapses what would have been a two-number mapping into one: the row that page needs is `materialize-terraform-self-managed vX` ↔ `materialize-monitoring vY`, and `vY` covers the chart and the module together.

- **Chart ↔ Materialize.** Dashboards from v0.8.0 want `mz_object_info` (Materialize v26.29.0+, degrading gracefully without it); scrapers from v0.1.1 need the `app.kubernetes.io/name` labels added in v26.24.0. The Terraform repo pins the Materialize version too, so the pair must actually be compatible.
- **Chart ↔ Grafana.** Dashboards from v0.8.0 target dashboard schema v2 and want Grafana v13+; `dashboards.config.grafana.apiTarget` defaults to `dashboard.grafana.app/v2`. The bundled Grafana satisfies this; an `external` Grafana may not, so the module surfaces the knob.
- **Chart ↔ Kubernetes.** The chart declares `kubeVersion: ">=1.27.0-0"`, below every managed-cluster floor the Terraform repo targets.

## Migration

A major version bump in the Terraform repo with a README upgrade note, matching how v6/v7/v8 were handled.

`enable_observability` has never been load-bearing — it defaults to `false` in `simple`, has no CI coverage, and the Grafana module labels itself unstable — so the cutover is a straightforward replacement rather than a delicate migration. Say it plainly and move on:

- The `prometheus` and `grafana` releases and their PVCs are destroyed. Up to 15 days of local Prometheus data goes with them; there is no backfill.
- Grafana state moves off its PVC. The chart default is SQLite on an `emptyDir`, so anything hand-created in the old instance does not carry over and will not survive a pod restart until Postgres is wired (see [Secrets](#secrets)).
- `prometheus_url` is replaced by a Thanos Query endpoint output. Keeping the old name as an alias for one release is cheap courtesy; a clean break is also defensible given the semantics change.
- `grafana_admin_password` keeps its name and meaning. `grafana_url` keeps its name but its meaning becomes conditional — the external URL when Grafana is exposed, the in-cluster service name otherwise (see [Reaching Grafana](#reaching-grafana)).
- `enable_observability = true` now requires the wrapper module, which needs cluster OIDC inputs.

`enable_observability` keeps its name — it is the right switch, and renaming it would churn six example roots for nothing. **It flips to `true` by default once this is GA**, in `simple` as well as `enterprise`, which is the point at which observability stops being opt-in and the "arrives with your cluster" goal is actually met.

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
- **Thanos has no sizing profiles yet**, so "small on PRs, medium on main" currently says nothing about Thanos. Medium needs nothing built — it is the chart defaults, which is a nice property since it puts the default configuration under continuous test — but `thanos-small` is a prerequisite for the PR variant to mean anything, and a `kind` resource-sizing profile is a prerequisite for either to fit a CI runner. See [Sizing profiles](#sizing-profiles-medium-is-the-chart-defaults).
- **Runner sizing.** Medium-on-main means microservice Loki with real replicas plus Thanos plus Grafana plus CNPG plus rustfs on one kind node. That wants a larger runner and belongs on the post-merge or merge-queue path, not as a PR gate.
- **Network policy is invisible to tier 1.** `loki-test` sets `networkPolicy.enabled: false`, and the chart has no policies for the other components anyway ([parity item 6](#6-networkpolicy-covers-only-loki)) — while the Terraform repo enables policies by default. The tier-2 variant should run with them on, or the whole policy surface ships unqualified.
- **mTLS needs its own tier-2 coverage, including rotation.** Same argument as network policy: run tier 2 with certs enabled end-to-end. And add the rotation case — a short-lived cert, forced renewal, assert delivery continues — because that failure is invisible to any test that only exercises a freshly-installed stack. See [In-cluster TLS](#in-cluster-tls-and-authentication).
- **Node and container metrics are only assertable once they exist.** Until [parity items 1 and 2](#1-cadvisor-is-not-collected-on-the-bundled-path) land, the suite cannot assert on `container_*` or `node_*` series, and the Kubernetes dashboard assertions stay structural for a reason that is a bug rather than a design choice. Worth encoding as a test that is expected to fail until then, so it converts to coverage the moment collection lands.

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
| **`prometheus.exporter.cadvisor` in the agent**, plus the metrics path it needs | The Kubernetes dashboards need cAdvisor, the shipped `ScrapeConfig` is inert under Alloy, and the old stack collected it. See [parity item 1](#1-cadvisor-is-not-collected-on-the-bundled-path) | **Blocking** — a functional gap in the default path, independent of Terraform |
| ~~**node-exporter subchart** plus a collector for its metrics~~ **Shipped** | [Parity item 2](#2-node-metrics-node-exporters-known-footprint-beats-alloys-exporter), landed as designed: a separate DaemonSet on the `default` tag, scraped by the gateway through the ServiceMonitor the subchart ships. Beyond the design: a collector allowlist rather than upstream defaults, a NetworkPolicy, and the `monitoring-critical` priority class | Done |
| **Agent OTLP export with a bounded, node-local persistent queue** — otelcol path in the agent, bridges for both signals, and the positions/WAL lifetime guarantee | [Agent → gateway transport](#agent--gateway-transport-otlp-with-a-wal). Gateway ingress and the backend fan-outs already exist, so this is agent-side pipeline work | Independent of the module; the durability guarantee is the point |
| `otelcol.processor.transform` before the log bridge (existing TODO in `gateway.yaml`) | Resource-attribute handling becomes load-bearing once agent logs arrive as OTLP rather than through `loki.source.api` | Blocking for the OTLP transport change |
| **cert-manager integration, opt-in** — `Certificate` resources from `issuer_ref` / `internal_issuer_ref`, server-side TLS on the receiving halves, and **file-mounted material** via the existing `mounts.extra` / `volumes.extra` surface so renewal takes effect | [In-cluster TLS and authentication](#in-cluster-tls-and-authentication). The client half is modeled; issuance, the server halves, and rotation-safety are not. cert-manager stays optional in the chart and on-by-default only on the Terraform path | Blocking for the cutover — `authType: none` on every hop |
| ~~**Grafana `ingress` / `service` values**, internal by default with an enforced CIDR allowlist for public~~ **Shipped** | [Reaching Grafana](#reaching-grafana), landed as designed: chart values plus per-cloud wrapper inputs, and the `validation` block copied rather than just the default. Grafana state also gained a dedicated database on the Terraform path | Done |
| NetworkPolicy for Thanos, Grafana, Alloy, Alertmanager, kube-state-metrics | Only Loki has one today, and the Terraform repo enables network policies by default | Blocking for the cutover |
| Generic `prometheus.io/scrape` discovery, default off, with exclusions **generated from the same source as the monitors** | The old stack collected any annotated pod; a naive port double-scrapes everything already covered, and a hand-maintained exclusion list drifts | Not blocking; the exclusion mechanism is the part worth getting right |
| A published list of panels that degrade without the SQL-on-scrape families | Otherwise a known upstream gap reads as a regression in the new stack | Should land with the migration note |
| Scheduling profiles (nodeSelector / tolerations / priorityClassName) that fan out to subcharts | Every example passes node selectors today; without them the fan-out map lives unverified in Terraform | Not blocking, but it is the module's largest maintenance liability until it lands |
| Storage-class profile, same shape | Examples pass one `storage_class` today | Not blocking |
| Thanos and Alloy validators wired into `mzmon.validate.collect` | The two components with workload-identity bindings have no validation at all | Should land with the storage wiring |
| Snapshot tests pinning rendered service-account names and subject strings | The fail-fast mechanism for the Terraform path's hardest coupling | Should land with the module |
| Subject strings emitted in `NOTES.txt` and as module outputs | Makes the value that must match copy-pasteable rather than derived | Should land with the module |
| Migrate the foundational parts of the Loki profiles into chart defaults or a `sizing` selector | Leaves profiles documentation-only and removes the last drift risk | Longer-term; reading the files directly covers the interim |
| Terraform tooling in `lint.yaml` (`terraform fmt`, `tflint`, `terraform-docs`), and `terraform/` added to the existing `materialize-monitoring` component's `content_paths` | Prerequisite for hosting the module here at all. See [One version, two artifacts](#one-version-two-artifacts) | Blocking for the module landing here |
| `packages/mz-monitoring-e2e` crate — a fourth workspace member, with cluster-connectivity helpers and the Grafana / Loki / Thanos / Alloy assertions | The assertion engine for tiers 1 and 2 | Blocking for qualification, which downstream assumes |
| kind-based E2E workflow with path-filtered variants (chart vs Terraform, small on PRs / medium on main) | The venue for tiers 1 and 2; `test.yaml` runs cargo test and helm-unittest only today | Blocking alongside the crate |
| `thanos-small` / `thanos-large` sizing profiles, mirroring the Loki convention (deltas from the medium defaults, with a documented envelope) | Thanos has no resources or replica counts in values at all today, so the sizing vocabulary covers only half the stack | Blocking for the tier-2 PR variant |
| A `kind` profile setting CI-appropriate resource sizes only, no feature management | Lets any variant fit a CI runner while staying composable with `loki-test` and the sizing profiles | Blocking for tiers 1 and 2 |
| Retire the SQL-on-scrape default, or make it fail loud rather than quiet | Terraform provisions no `mz_support` role, so today the scraper comes up failing auth | Module defaults it off; chart cleanup follows |

## Documentation to update

The Terraform story is stale in several places here and should be corrected as the work lands:

- `getting-started/terraform.md` — ✅ done. Now the install guide, including a tfvars reference.
- `getting-started/_index.md` — ✅ done.
- `reference/compatibility.md` — ✅ done. Carries the Terraform-repo ↔ chart-version table.
- `reference/internal/roadmap.md` — the M4 "Terraform wrapper module" row, the "Downstream pinning" bullet, and a `terraform` entry wherever components are enumerated. Also two Testing / CI rows: the "Synthetic-data end-to-end smoke test" row **is** the E2E suite described here, and the "kind / ArgoCD / FluxCD CI matrix" row needs its kind half split out and promoted well above "very low priority" (ArgoCD and Flux stay).
- `reference/internal/repo-layout.md` — ✅ done. Refreshed against the tree and carries `terraform/` as a planned entry.
- `operating/production-best-practices.md` — the `[consumer]` items become concretely satisfied-by-Terraform on that path, which matters to a reader choosing an install method.

## Open questions

- [x] ~~Does hosting the module here create a chicken-and-egg problem for the Terraform repo's integration tests?~~ **No.** Downstream consumes released tags only, Renovate-pinned, and assumes qualification already happened here. The contract is one-directional.
- [x] ~~Does the integration harness run the object-storage path on every apply?~~ Superseded by the [tier structure](#tiers): real object storage is exercised at tier 2 on kind via rustfs, and at tier 3 on real clouds.
- [ ] Reload-on-change or a config-checksum-annotation rollout when a mounted certificate is renewed? The first avoids restarts, the second is simpler and matches how the chart already handles config revisions.
- [ ] Does Loki and Thanos server-side TLS change the datasource URLs to `https` in a way that interacts with the tenant-header wiring, and does the bundled Grafana trust the internal issuer's CA by default or need it mounted?
- [ ] Should the Grafana LB be Ingress-through-the-LB-controller on all three clouds, or does GCP/Azure want the `load_balancers` module instead? AWS is clear; the others are not.
- [ ] Verify Terraform resolves a `?ref=` containing a slash (`materialize-monitoring/v0.9.0`). Slashes are valid in refnames and the ref is passed through to git, so this should work — but the whole consumption path depends on it, so confirm before committing to the tag format.
- [ ] Does a `### Terraform` changelog subsection carry enough signal for Helm-only readers seeing a version bump that did not change the chart, or does the release note need to say so more loudly?
- [ ] Confirm the Alloy component and attribute names for a file-backed sending queue, and whether the pipeline schema should type the otelcol components or keep using the `raw:` escape hatch as the existing bridges do.
- [ ] Should logs and metrics share one agent-side OTLP exporter and queue (uniform behavior, shared backpressure, one node's blast radius) or separate queues over the same connection (independent failure)? Worth exposing as a value either way.
- [x] ~~`hostPath` or `emptyDir` for the agent queue, and how is it bounded?~~ **`hostPath`, `/var/lib/mzmon-alloy` → `/var/lib/alloy/wal`**, bounded by compaction at ~100 MiB with compaction-on-start. Survives pod restarts, needs no StorageClass.
- [ ] Measure `prometheus.exporter.cadvisor`'s per-node footprint on the tier-2 medium run, and set the limits from the measurement. If it lands badly on dense nodes, the kubelet-scrape fallback is already written in `scrapeconfig-cadvisor.yaml`.
- [ ] Measure the per-node cost of `prometheus.exporter.unix` before revisiting consolidation of node metrics into the agent. Keeping node-exporter separate is deliberate for now, not permanent.
- [ ] Where does the generated exclusion list for annotation discovery live — in the `scrape` transpiler alongside the monitor definitions, or as a separate derived artifact both consume?
- [x] ~~What does "medium" mean, and does the vocabulary cover Thanos?~~ **Medium is the chart defaults** — the Loki profiles already say so and define only deltas. Thanos sizing profiles are net-new work, tracked in the prerequisites table.
- [x] ~~Is the Alloy support bundle available to the E2E suite?~~ **Enabled by default**, so the suite can rely on it without a flag. The exact endpoint path still wants confirming against the pinned version.
- [ ] What envelope vocabulary do the Thanos profiles document? Loki's is throughput (sustained / burst / ceiling); Thanos wants active series, ingested samples per second, retention volume, and query concurrency instead.
- [ ] Does enabling `queryFrontend` in `thanos-large` also move the Thanos datasource URL onto it? Deploying the cache without routing through it is a silent no-op.
- [x] ~~Is `receive.mode: standalone` acceptable as the medium default for a tier-0 metrics store?~~ **Yes** — it is RouterIngestor mode and already shards across a hashring. Availability comes from an odd replication factor, which every profile must set via `extraArgs`; `split` is unusable because `receive.ingester` deliberately does not inherit the top-level defaults.
- [x] Thanos has no PodDisruptionBudgets or topology spread constraints on any component today, and Receive is PVC-backed and therefore AZ-pinned. Does zone spread land with the sizing profiles, or as its own piece of work?
  **Resolved, partly by removing the premise.** PDBs landed separately (`thanos.global.pdb`). Receive is no longer PVC-backed — it and the Compactor moved to `emptyDir` with explicit `ephemeral-storage` budgets, because a volume that cannot cross zones makes an AZ failure worse rather than safer; only the Store Gateway keeps a PVC. Zone spread is still outstanding and is its own piece of work, but it is now a scheduling change with nothing pinning it — see [Storage: ephemeral by default](../../../../operating/production-best-practices/#thanos-ephemeral-storage).
- [ ] Should tier 2 also gate chart changes that touch storage wiring, or only run on main? Path-filtering is more precise but more machinery to keep correct.
- [ ] Is a `kind-integration` (rustfs + CNPG) profile worth adding so the chart's own gate can reach the deeper storage shape, rather than that coverage living only on the Terraform path?
- [ ] One `scheduling` profile parameterized by values, or separate `node-selector` / `tolerations` profiles? The former is fewer files; the latter composes more cleanly with the existing one-concern-per-profile convention.
- [ ] How much of the Loki profiles is genuinely foundational versus illustrative? That answer sets the size of the "migrate into chart defaults" task.
- [ ] Should the wrapper modules provision the Grafana Postgres database from the existing per-cloud `database` module in the first version, or defer it behind a variable? Deferring ships sooner; not deferring means Grafana state survives a restart out of the box. Note tier 2 covers the CNPG-backed shape either way, so the chart-side path is qualified before the wrapper uses it.
- [ ] Retention defaults for the buckets: what lifecycle policy does the wrapper set, and does it agree with the chart's compactor/retention defaults?
- [ ] Is the `prometheus_url` output alias worth one release, or is a clean break clearer given the semantics change to Thanos Query?
- [x] ~~Should `enable_observability` default to `true` in `examples/simple`?~~ **Yes, at GA.** Recorded in [Migration](#migration).
- [ ] Alertmanager routing has no Terraform input here (no receivers, no upstream integration). Is `additional_values` sufficient for the first version? Note the E2E suite cannot assert delivery without a receiver, so routing stays effectively unqualified either way.
