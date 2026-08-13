---
title: "o11y Troubleshooting"
weight: 20
---

# o11y Troubleshooting

Indexed by **what you actually see** — an error string, a pod that will not start, a change that appears to do nothing — because that is what you have when something is wrong.

Each entry names the cause and links to the page that explains it properly. Nothing here is unique to one install path; where a path already handles something for you, that is called out.

## Start here: a timeout is not a duration problem

> [!WARNING]
>  **"`helm upgrade` timed out" almost never means something was slow. Assume something is broken.**
>
>  This is the single most misleading error in the stack, and raising the timeout is almost always the wrong response. The client stopped waiting because a pod never became Ready — and a pod that is not Ready after five minutes is usually not going to be Ready after fifteen.

It applies identically whether you ran `helm` yourself or Terraform ran it for you. `helm_release` failing with a timeout means the same thing, and Terraform additionally taints the release, so the next apply replaces it (see [below](#helm_release-is-destroyed-and-recreated-on-every-apply)).

Diagnose in this order. It takes under a minute and almost always lands on the answer:

```bash
# 1. Events first — scheduling, image pulls, volume attach, and probe failures
#    all surface here, and this is where the actual cause usually is.
kubectl --namespace monitoring get events --sort-by=.lastTimestamp | tail -30

# 2. Then which pods are not Running/Completed, and why.
kubectl --namespace monitoring get pods
kubectl --namespace monitoring describe pod <the-unhappy-one>

# 3. Then that pod's logs — including the previous instance if it crash-looped,
#    which is where the real error is when a container restarts.
kubectl --namespace monitoring logs <pod> --all-containers --previous
```

The genuine "it really is just slow" case exists but is narrow: a **known** ingester rollout, which is ordered and readiness-gated at roughly a minute per ingester. If you did not just change the ingester pod spec, that is not what you are looking at. See [Upgrading](upgrading/).

## Install and storage

### `create bucket: no s3 endpoint in config file`

Loki's ingesters crash-loop with this, or the compactor does.

**Cause.** The backend is named in more than one place and they disagree. Loki's clients are chosen *by name* and then validated against a config that was never populated — so a value left at the chart's S3-shaped default produces an S3 client with no endpoint.

The schema period is the usual culprit, because it selects the **chunk** client and therefore takes down every ingester:

```yaml
schema_config:
  configs:
    - object_store: s3    # ← left at the default on a GCS/Azure install
```

**Fix.** Name the backend in all three load-bearing places. See [Selecting the backend](../../logs-and-events/storing/#selecting-the-backend) for the full set and the render-time check that catches a mismatch.

The chart refuses to install a mismatched set, so this only reaches a cluster on an install that predates that check — or on the pre-Thanos-objstore path below.

### `failed to create delete request store object client: at least one bucket name must be specified`

The compactor crash-loops.

**Cause.** The same disagreement, on the pre-Thanos-objstore path. When `use_thanos_objstore` is off, `loki.loki.storage.type` is the live selector, and `compactor.delete_request_store` still has to match it. A profile that switches the backend and forgets this one gets a client for a backend that was never configured.

**Fix.** Set `compactor.delete_request_store` to the same value as `storage.type`. The chart validates this pair on both paths.

### `AttachVolume.Attach failed ... pd-balanced disk type cannot be used by <machine-type>`

Every PVC-backed pod stays `Pending` or `ContainerCreating`; the rest of the stack is healthy.

**Cause.** GCP's **C4 and N4** machine families accept only Hyperdisk. They cannot attach Persistent Disk of any type, and *every* StorageClass GKE creates by default — `standard-rwo` (`pd-balanced`), `premium-rwo` (`pd-ssd`), `standard` (`pd-standard`) — is Persistent Disk. GKE does not create a Hyperdisk class for you.

**Fix.** Create a `hyperdisk-balanced` StorageClass and point the four PVC-backed workloads at it. The Terraform modules take `storage_class`; see [Getting Started > Terraform](../../getting-started/terraform/#storageclass-on-gcp-c4-and-n4-node-pools) for the manifest and the migration caveat.

> [!WARNING]
>  Changing the class does **not** move existing volumes, and this is easy to miss: a StatefulSet's `volumeClaimTemplates` are immutable, and Kubernetes does not garbage-collect PVCs created from them — the default `persistentVolumeClaimRetentionPolicy` is `Retain`.
>
>  So `helm uninstall` and reinstall leaves the old PVCs in place, the new StatefulSet binds them, and the wrong class persists through any number of reinstalls. Delete the PVCs explicitly.

### A PVC fails to provision below a few GiB

**Cause.** Cloud disk minimums, not the chart. GCP Hyperdisk and Azure managed disks floor at 4 GiB, and the CSI driver may round to 1 GiB rather than to the disk type's minimum.

**Fix.** The chart sizes Alertmanager's volume at 4Gi for exactly this reason — sized by the cloud floor, not by Alertmanager, which needs kilobytes. If you have lowered it, raise it back.

### Pods `Pending` with `RESOURCE_POOL_EXHAUSTED` or `GCE out of resources`

**Cause.** Not a configuration problem. The cloud has no capacity for that machine type in those zones, and the autoscaler has backed off. Narrow machine families — Arm, or anything with bundled local SSD — hit this more often.

**Fix.** Check the node pool's target versus its actual count, and the autoscaler's backoff state:

```bash
kubectl get configmap cluster-autoscaler-status -n kube-system -o yaml
```

Then wait it out, add zones, or change the machine type. A pod requiring a node pool that cannot scale looks identical to a scheduling misconfiguration, so confirm which one it is before changing selectors.

### Everything suddenly fails, and it worked an hour ago

Terraform cannot reach the cluster, `kubectl` returns an auth error, or a plan that succeeded this morning now fails on the provider rather than on anything you changed.

**Cause.** Your cloud credentials expired. AWS and GCP sessions commonly last **12 hours**, so if you authenticated at the start of the day they lapse in the late afternoon — right when you are mid-task and least likely to suspect the environment rather than the change you just made.

**Fix.** Re-authenticate before you debug anything else:

```bash
gcloud auth login && gcloud auth application-default login   # GCP
aws sso login                                                # AWS (or your usual flow)
```

Then re-fetch cluster credentials, because a kubeconfig entry with an exec plugin will keep failing until the underlying session is refreshed.

The tell is breadth: an expired session breaks *everything at once*, including things you did not touch. A real misconfiguration is almost always narrower. When a failure looks impossibly broad, check the clock before you check your work.

## Configuration that appears to do nothing

### An Alloy pipeline or metric-filter change has no effect

`helm upgrade` reported success. The ConfigMap holds the new value. Alloy is still doing the old thing.

**Cause.** Alloy's config arrives partly through `envFrom` ConfigMaps, and **environment variables are fixed at container start**. Nothing propagates them to a running process — not a config-reloader sidecar, not Alloy's `/-/reload`, which re-reads config *files* while `sys.env()` still returns the value the container started with.

This is the one place the chart cannot own its own rollout, and it is the reverse of what a chart normally guarantees.

**Fix.**

```bash
kubectl --namespace monitoring rollout restart deployment/alloy-gateway daemonset/alloy-agent
```

The Terraform modules stamp a values hash onto both pod templates so this happens automatically. On the Helm path it is yours — see [Collection (Alloy)](production-best-practices/#collection-alloy).

### Grafana's admin password changes on every deploy

**Cause.** The chart generates the password and reuses it by looking up the existing Secret. That lookup returns nothing during `helm template` and `--dry-run`, so any render-only pipeline — most GitOps setups — regenerates it on every sync.

**Fix.** Supply your own Secret and set `grafana.grafana.admin.existingSecret`. See [Installing via Helm](../../getting-started/helm/#verify-it-came-up).

### Logs are collected but never arrive in Loki

Labels and queries return `success` with no data. Nothing in Loki's own logs explains it.

**Cause.** Usually the gateway's write endpoint does not resolve. Loki's Service names depend on `deploymentMode`: microservice mode renders `loki-distributor` (writes) and `loki-query-frontend` (reads), while SingleBinary renders exactly one `loki` Service. The gateway retries DNS indefinitely and Loki, having received nothing, has nothing to report.

**Fix.** Check the gateway for the failing target, which is the only place it surfaces:

```bash
kubectl --namespace monitoring logs -l app.kubernetes.io/name=alloy-gateway --tail=200 | grep "final error sending batch"
```

Then confirm the write and read URLs match the deployment mode you are running.

> [!TIP]
>  When checking whether ingest is working, query a **bounded recent window**. Loki's filesystem store survives a pod restart and WAL-replayed streams count toward `loki_ingester_streams_created_total`, so an unbounded query and that counter both pass against a stack that stopped ingesting an hour ago.

## Teardown

### `helm uninstall` hangs, or the namespace stays `Terminating`

**Cause.** grafana-operator finalizers with no remover. See [Uninstalling](uninstalling/) for the mechanism, the ordered teardown, and how to recover a wedged one — including the worse variant, where a stuck CRD blocks *re-installing* the CRDs chart.

## Terraform-specific

### `helm_release` is destroyed and recreated on every apply

The plan says `replace_because_tainted`.

**Cause.** Terraform taints a resource whose *create* failed partway. The Helm install ran, `wait` is on by default, the pods never went Ready, and the create errored after the timeout — so Terraform cannot know what state the release is in.

**Fix.** Nothing; it clears itself once one create succeeds, after which applies become in-place upgrades. `terraform untaint` does not help — Helm cannot `upgrade` a release whose only revision is a failed install (`has no deployed releases`), so replacement is the correct path.

Note that a replace does **not** reset PVCs, per the storage-class warning above.

### `terraform apply` prints pages of unrelated Helm notes

**Cause.** The Helm provider defaults `render_subchart_notes` to `true`, the opposite of `helm install`, where `--render-subchart-notes` is opt-in. Eight subcharts' notes then bury this chart's own — which is where the validators' warnings are printed.

**Fix.** `render_subchart_notes = false`. The module sets this already.

## See more

- [Production Best Practices](production-best-practices/) — the deployment checklist, tagged by owner
- [Uninstalling](uninstalling/) and [Upgrading](upgrading/) — the two asymmetric lifecycle operations
- [Meta Observability](meta-observability/) — monitoring the monitoring stack
