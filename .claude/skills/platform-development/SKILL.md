---
name: platform-development
description: |
  This skill should be used when changing the Terraform modules under
  `terraform/`, the E2E test tiers under `test/e2e/`, or the CI that gates them —
  and when a chart change has consequences for either. Covers how the module and
  the chart are coupled, what each test tier can and cannot prove, and the
  gotchas that produce silently-wrong-but-valid config.
---

# Platform Development

The Terraform module lives next to the chart whose value paths it encodes, so a
chart change and its Terraform consequence land in the same PR. That is the whole
reason it is in this repo; keep it true.

For chart internals (templates, `values.yaml`, subchart wrapping, helm-unittest)
use the [chart-development skill](../chart-development/SKILL.md). This skill is
about the layer above it and the tests around both.

## Read first

- [Terraform Modules design doc](../../../docs/content/reference/internal/design-docs/20260803-terraform-modules.md)
  — why the module is here, the values-composition order, and the test tiers.
- [`terraform/modules/materialize-monitoring/README.md`](../../../terraform/modules/materialize-monitoring/README.md)
  — the subchart fan-outs and why the Alloy pod-template hash exists.
- [`test/e2e/README.md`](../../../test/e2e/README.md) — what each tier covers and
  the traps in extending them.
- [Contributing](../../../docs/content/reference/internal/contributing.md) —
  prerequisites, `make` targets, and the pre-commit/pre-push split.

## The rule that generates most of the others

**Helm cannot template a subchart's values from the parent chart.** The parent can
compute anything — it sees all of `.Values` and `.Files` — but a subchart value is
static YAML, so there is nowhere to put a computed result that a subchart reads.

Everything the module does that looks like duplication follows from this: the
scheduling fan-out, the storage-class fan-out, the Azure identity labels, and the
config hash that rolls Alloy all exist because the chart cannot do them itself.
Before adding a consumer-side fan-out, confirm the chart genuinely cannot express
it — and when you add one, note which subchart keys it is coupled to.

## `terraform validate` proves almost nothing here

A value written to a path no subchart reads is still valid HCL. It renders
perfectly and is silently ignored.

So the gate is `make terraform-check`, which plans each example, extracts the
composed Helm values from the plan, and renders the chart against them. It needs
no cluster. **When you add a lever, add an assertion that it lands** — see the
`storageClass` count and the GCM filter check in
[`bin/terraform-render-check.sh`](../../../bin/terraform-render-check.sh).

Both an AWS and a GCP example are rendered, and that is not redundancy: the
chart's storage defaults are S3-shaped, so an AWS-only example agrees with every
default it fails to set.

## Gotchas that have each cost real debugging time

- **`yamlencode` quotes every key.** Anything grepping composed values for
  `key:` must tolerate `"key":`.
- **`yamldecode` promotes an unquoted YAML date to RFC 3339.** `2024-01-01`
  becomes `2024-01-01T00:00:00Z`, which Loki rejects. Quote dates in chart values
  that Terraform reads back.
- **HCL ternaries require both branches to share a type.** A conditional map of
  differing shapes will not compile; emit a conditional *list of documents*
  instead.
- **`source` cannot be a variable, and an absolute local path is copied** into
  `.terraform/modules/` without the chart directory beside it — which silently
  drops the sizing profiles. Use a git source or a `./`-relative path.
- **The Helm provider defaults `render_subchart_notes` to `true`**, opposite the
  CLI. Left on, subchart notes bury the validators' warnings.
- **The provider version constraint in this module caps what a downstream root can
  select.** Widening it is a prerequisite for a downstream provider major, not a
  consequence.
- **`count` cannot depend on a value a wrapper computes.** A wrapper that
  provisions a database in the same apply hands this module an endpoint and a
  generated password, and Terraform expands `count` before it knows either —
  `Invalid count argument`. Anything gating a resource's existence needs a
  plan-time-known input, which is what `grafana_database_enabled` and
  `grafana_database_manage_password_secret` are for. Infer from the value when it
  is null, so a caller with literals never has to think about it.
- **`coalesce` errors when *every* argument is null**, rather than returning null.
  On an optional credential that is the default path, so it fails the one case
  with nothing to decide. Use a conditional.
- **A stale pinned ref masks variable validation.** When a downstream wrapper
  passes an input the pinned tag does not have, the plan dies on
  `Unsupported argument` *before* any `validation` block runs — so a test of a
  validation rule reads as passing when it never executed. Confirm the failure
  case actually fails before trusting the success cases.

## Tests

| Tier | Command | Cluster | Proves |
|---|---|---|---|
| 0 | `make terraform-check` | none | values land in the paths the chart reads |
| 1 | `make e2e-tier1` + `make e2e-verify-tier1` | kind | the logging round trip actually works |
| 2 | `make e2e-tier2` + `make e2e-verify-tier2` | kind | real object storage, a real Postgres, and Thanos |
| 3 | — | real clouds | downstream, on released tags only |

Every E2E target names its cluster explicitly (`KIND_CONTEXT`) rather than
inheriting the current kubeconfig context. Keep it that way: these targets
install, restart, and delete.

**The tiers collide rather than coexist** — both name their CRDs release
`mzmon-crds`, so a tier-2 apply onto a tier-1 cluster fails on the first release
it creates. `make e2e-tier1-down` is the switch, and it uninstalls the main
release *before* the CRDs one: the main release's `pre-delete` hook is what
removes the Grafana custom resources, and without it their finalizers have no
remover and the CRDs wedge in Terminating.

Tier 2 composes two roots — `terraform/test/generic-cloud` (the substrate, still
applyable alone) and `terraform/test/tier2` (the composition, which reads the
substrate's state). It sets `chart_registry` to the repo's `charts/` directory,
which is load-bearing: the module's default is the published OCI registry, and a
tier installing a *released* chart would be testing the wrong artifact. It also
needs `min_zones = 0` — kind's nodes carry no zone label at all, so any spread
constraint over that key fails to schedule no matter how low `minDomains` is.

The assertions live in `packages/mz-monitoring-e2e`. One binary for every tier —
it reads the release's coalesced Helm values (`helm get values --all`) to decide
what applies, so there is no tier flag, and `make e2e-verify E2E_CONTEXT=<ctx>`
points it at any cluster including a real one. It asserts only; it never
installs.

Transport defaults to a port-forward (`src/forward.rs`, no local listener). The
API server's Service proxy is available with `--transport proxy` but is not the
portable choice: it **strips `Authorization`** (so Grafana cannot authenticate
over it, though custom headers like Loki's `X-Scope-OrgID` pass fine), and it
needs control-plane-to-pod reachability — on EKS a proxied request to Thanos on
9090 times out where a port-forward succeeds. Also use `encode_segment`, not
`encode`, for path segments: routers match the raw path, so `mzmon%2Dloki` is a
404 where `mzmon-loki` is not.

Two rules when extending it:

- **Values are intent, not description.** A component the values enable but the
  cluster lacks is a *failure*. Only a genuinely-disabled component skips, and a
  skip is an ignored test rather than an absent one — a suite whose list silently
  shrinks looks exactly like one that passed.
- **Assert query success everywhere, non-empty results only on self-monitoring
  series.** Materialize scrapers stay off in these tiers, so the only data
  guaranteed to exist is what the stack produces about itself. Backwards, this
  yields either a suite that passes while blind or one that flakes on empty
  Materialize series forever.

## Iterating without cutting a release

The default loop makes you release the chart, release the module, and bump the
downstream ref before you can test anything. Two edits in
`materialize-terraform-self-managed` remove that entirely, and they are what to
reach for whenever you need a real `plan` or a real cluster.

**1. Point the wrapper at this checkout** — in each
`{aws,gcp,azure}/modules/monitoring/main.tf`:

```hcl
module "monitoring" {
- source = "github.com/MaterializeInc/materialize-monitoring//terraform/modules/materialize-monitoring?ref=materialize-monitoring/<CURRENT_TAG>"
+ source = "../../../../materialize-monitoring/terraform/modules/materialize-monitoring"
```

Four `..` lands in the directory holding both repos. It **must** stay relative —
an absolute path is copied into `.terraform/modules/` without `charts/` beside
it. The swap itself needs one `terraform init`; edits to the module after that
take effect on the next `plan` with no re-init.

**2. Point the chart at this checkout** — in each
`{aws,gcp,azure}/examples/simple/main.tf` (and `enterprise`):

```hcl
module "monitoring" {
  ...
+ chart_registry = "/Users/you/workspace/materialize-monitoring/charts"
```

`chart_registry` is not required to be an OCI registry. A local directory
installs from your working tree, and the module reads its version from that
directory's `Chart.yaml`, so the pair cannot drift.

A third shortcut — installing one component at a time with a scratch release —
and the reasoning behind all of them are in
[Iterating against a live cluster](../../../docs/content/reference/internal/contributing.md#iterating-against-a-live-cluster).

Revert both before committing. A relative `source` that reaches main is a broken
module for every consumer.

### Planning without a cloud account

Most plan-time failures reproduce with no credentials, because Terraform
evaluates variable validation and expands `count` before it calls a provider.
Instantiate the module from a scratch directory with only its required inputs
(`prefix`, `project_id`, `region` on GCP) and dummy values, feed it the case you
want to test, and read the error. Anything that fails in the graph shows up
before the provider is ever contacted.

To reproduce a value that is *unknown at plan time* — the shape a wrapper
produces when it provisions a database in the same apply — drive the input from a
`random_password` result rather than a literal.

## Releasing

The module and the chart are **one release**, and the module reads its version
out of the chart's own `Chart.yaml` — so a ref names both. A downstream wrapper
pinning `?ref=materialize-monitoring/vX.Y.Z` cannot use a module feature until a
release contains it; check the tag before assuming a variable is reachable.

Bump level is chosen by editing the CHANGELOG placeholder heading. See
[Choosing the next version](../../../docs/content/reference/internal/releasing.md#choosing-the-next-version).
