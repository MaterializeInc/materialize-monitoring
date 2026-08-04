---
name: deployment-operations
description: |
  This skill should be used when standing up the monitoring stack against a real
  or local cluster, or when diagnosing one that is not healthy — pods that will
  not start, storage that will not attach, config changes that appear to do
  nothing, or a teardown that hangs. Applies to Helm and Terraform installs and to
  the kind E2E tiers.
---

# Deployment Operations

Most of what belongs here is in the docs, because operators hit the same problems.
Go there first; this file exists to route you and to record the habits that keep a
diagnosis honest.

## Read first

- [o11y Troubleshooting](../../../docs/content/operating/o11y-troubleshooting.md)
  — **indexed by symptom**, which is what you have when something is wrong. Error
  strings, pods that will not start, changes that appear to do nothing.
- [Uninstalling](../../../docs/content/operating/uninstalling.md) — read *before*
  tearing anything down. Teardown deadlocks by default.
- [Production Best Practices](../../../docs/content/operating/production-best-practices.md)
  — the checklist, tagged with who owns each item.
- [Installing via Helm](../../../docs/content/getting-started/helm.md) and
  [Terraform](../../../docs/content/getting-started/terraform.md) — the two paths.
- [`test/e2e/README.md`](../../../test/e2e/README.md) — the local kind tiers.

## Name the cluster, every time

`kubectl` and `helm` both take the current kubeconfig context when you do not say
otherwise, and these operations install, restart, and delete. A context left
pointing at production is the failure mode.

- The `make e2e-*` targets pin `KIND_CONTEXT`; the Terraform test substrate pins
  its providers. Do not add a command that inherits the ambient context.
- `kubectl config current-context` **ignores `--context`** — it reports the
  configured context, not the one a command used. Do not use it to prove which
  cluster you touched. Capture `kubectl cluster-info` instead; the server address
  is unambiguous.

## Three habits that prevent wrong conclusions

**Assert on recent data, not on any data.** Loki's filesystem store survives a pod
restart and WAL-replayed streams count toward `streams_created_total`, so an
unbounded query and that counter both pass against a stack that stopped ingesting
an hour ago. Bound the window, and keep it meaningfully shorter than how long a
break would have lasted.

**A green `helm upgrade` is not evidence the change took effect.** Alloy's config
arrives partly through `envFrom`, and environment variables are fixed at container
start. Check the ConfigMap *and* whether the pod restarted.

**Read the render output.** The chart's validators print to `NOTES.txt`, and a
warning there is usually the earliest signal. On the Terraform path they surface
in the apply log — which is why the module disables subchart notes, so ours are
not buried.

## When you cannot reproduce it later

The cluster dies with the CI runner, and a red E2E with no artifacts costs more
than the test saves. [`test/e2e/dump-diagnostics.sh`](../../../test/e2e/dump-diagnostics.sh)
collects pods, events, storage, describes for unhealthy pods, all container logs
including `--previous`, the four Alloy ConfigMaps, and the cluster's identity.
Run it before deleting anything, and extend it rather than gathering by hand.

## Things that are the cloud, not us

Worth recognising quickly, because they look like misconfiguration:

- **`RESOURCE_POOL_EXHAUSTED` / `GCE out of resources`** — a capacity stockout.
  Narrow machine families (Arm, bundled local SSD) hit it more often.
- **`pd-balanced disk type cannot be used by <machine-type>`** — GCP C4/N4 take
  only Hyperdisk, and no default GKE StorageClass qualifies.
- **Workload identity cannot be tested on kind.** There is no OIDC issuer an IAM
  provider trusts, so IRSA, GKE Workload Identity, and Entra Workload ID are only
  exercised against real clouds. The chart validators assert the *shape* of that
  config instead, which is why a mismatched annotation fails at render time.

## Storage changes are not free

`volumeClaimTemplates` are immutable, and Kubernetes does not garbage-collect
PVCs created from them — `persistentVolumeClaimRetentionPolicy` defaults to
`Retain`. So reinstalling does not reset a volume's class or size; the new
StatefulSet binds the old PVC. Delete PVCs deliberately, and know what is on them
before you do (see the per-workload notes in the troubleshooting page).
