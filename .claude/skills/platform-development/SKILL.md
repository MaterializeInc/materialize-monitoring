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

## Tests

| Tier | Command | Cluster | Proves |
|---|---|---|---|
| 0 | `make terraform-check` | none | values land in the paths the chart reads |
| 1 | `make e2e-tier1` + `make e2e-verify-tier1` | kind | the logging round trip actually works |
| 2 | `make e2e-generic-cloud` | kind | real object storage and a real Postgres |
| 3 | — | real clouds | downstream, on released tags only |

Every E2E target names its cluster explicitly (`KIND_CONTEXT`) rather than
inheriting the current kubeconfig context. Keep it that way: these targets
install, restart, and delete.

## Iterating without cutting a release

The default loop makes you release the chart, release the module, and bump the
downstream ref before you can test anything. Three shortcuts remove that, all
temporary and all documented in
[Iterating against a live cluster](../../../docs/content/reference/internal/contributing.md#iterating-against-a-live-cluster):
point a wrapper's `source` at a relative path to this repo, point
`chart_registry` at a local chart directory, and install a second release with
everything disabled except the component you are working on.

Revert all three before committing. A relative `source` that reaches main is a
broken module for every consumer.

## Releasing

The module and the chart are **one release**, and the module reads its version
out of the chart's own `Chart.yaml` — so a ref names both. A downstream wrapper
pinning `?ref=materialize-monitoring/vX.Y.Z` cannot use a module feature until a
release contains it; check the tag before assuming a variable is reachable.

Bump level is chosen by editing the CHANGELOG placeholder heading. See
[Choosing the next version](../../../docs/content/reference/internal/releasing.md#choosing-the-next-version).
