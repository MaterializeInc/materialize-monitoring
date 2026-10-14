---
title: "Thanos"
weight: 20
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Thanos runbook

This page is for you if you operate the Thanos metrics store that `materialize-monitoring` installs, and one of its components has stopped working.
Each section starts from an alert or an error you can see, and ends when the component is healthy again.
For a symptom that no section here matches, see [o11y Troubleshooting](../../o11y-troubleshooting/).

## Thanos Compactor

The Thanos Compactor merges the blocks that Receive uploads into larger ones, downsamples them, and deletes blocks that are past retention.
It runs as a single replica, `thanos-compactor-0`, and works from the bucket.
Nothing on its volume is authoritative, so a Compactor that loses its volume redoes the work and loses no data.

### Thanos Compactor halted

The `ThanosCompactHalted` alert fires, and `thanos_compact_halted` is `1`.
The Compactor pod is still `Running` and `Ready`.

The Compactor stops all work after an error it treats as critical, and it doesn't retry until its process restarts.
While it's halted, the bucket gets no compaction, no downsampling, and no retention.
The bucket grows, and the Store Gateway serves long-range queries from many small raw blocks.

#### Find the error

The Compactor logs the error that halted it once, when it halts:

```bash
MONITORING_NAMESPACE=monitoring
KUBECONTEXT=my-cluster

kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" logs thanos-compactor-0 \
  | grep 'critical error detected; halting'
```

If the pod has restarted since the halt, the line isn't in its log until the Compactor reaches the same failure again.
That can take several minutes, because the Compactor first downloads the blocks it plans to compact.

Match the error to its cause in the following table:

| The error contains | Cause | Fix |
|---|---|---|
| `overlaps found while gathering blocks` | Receive wrote overlapping blocks across a restart, and vertical compaction is off | [Overlapping blocks from Receive restarts](#overlapping-blocks-from-receive-restarts) |
| `no space left on device` | The Compactor's volume is too small for the group it's compacting | [Compactor out of space](#compactor-out-of-space) |

#### Find how long it has been halted

The oldest raw block in the bucket shows when compaction last ran.
Receive uploads raw blocks at compaction level `1`, and a running Compactor merges them within a day.
The following command prints the start time of the oldest one:

```bash
MONITORING_NAMESPACE=monitoring
KUBECONTEXT=my-cluster

kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" exec thanos-compactor-0 -- \
  thanos tools bucket inspect --objstore.config-file=/etc/thanos/objstore.yml --output=tsv \
  | awk -F'\t' '$9 == "1" {print $2}' | sort | head -1
```

The command runs the `thanos` binary without a shell.
It works with the shell-less images that the Chainguard and Docker Hardened Images profiles select.
The more days of raw blocks there are, the longer the Compactor takes to catch up, and the larger its first compaction groups are.

#### Overlapping blocks from Receive restarts

The error looks like the following:

```text
level=error msg="critical error detected; halting" err="compaction: group 0@...:
pre compaction overlap check: overlaps found while gathering blocks.
[mint: ..., maxt: ..., range: 8s, blocks: 2]: <ulid: ...>, <ulid: ...>"
```

Two blocks from the same Receive replica overlap by a few seconds.
Receive writes such a pair across every restart, because it accepts replicated and retried samples older than the block it flushed on shutdown.
Without vertical compaction, the Compactor treats any overlap as corruption.

Run the Compactor with `--compact.enable-vertical-compaction` and `--deduplication.replica-label=receive_replica`.
Both are in the chart default `thanos.compactor.extraArgs`.
If the Compactor halted with them missing, either an `extraArgs` override dropped them, or the release predates them.
The Compactor merges the overlapping blocks on its next start, and the blocks need no manual repair.

The first run with deduplication on needs more disk than any later run.
If it halts on `no space left on device`, continue with [Compactor out of space](#compactor-out-of-space).

#### Restart the Compactor

After you fix the cause, restart the Compactor:

```bash
MONITORING_NAMESPACE=monitoring
KUBECONTEXT=my-cluster

kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" delete pod thanos-compactor-0
```

If the fix changed the Compactor's arguments, check that the pod started after the change before you assume that it picked the change up:

```bash
MONITORING_NAMESPACE=monitoring
KUBECONTEXT=my-cluster

kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get pod thanos-compactor-0 \
  -o jsonpath='{.status.startTime}{"\n"}'
```

#### Confirm the recovery

`thanos_compact_halted` returns to `0`, and `thanos_compact_todo_compactions` falls as the backlog drains:

```bash
MONITORING_NAMESPACE=monitoring
KUBECONTEXT=my-cluster

kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" port-forward pod/thanos-compactor-0 10902:10902 &
curl -s localhost:10902/metrics | grep -E '^thanos_compact_(halted|iterations_total|todo_compactions) '
```

After a long halt, the Compactor works through the whole backlog before downsampling catches up, which can take hours.

### Compactor out of space

The Compactor halts, and its log holds an error from partway through a compaction:

```text
level=error msg="critical error detected; halting" err="compaction: group 0@...:
compact blocks [...]: 3 errors: preallocate: no space left on device; ..."
```

The Compactor keeps a compaction group's source blocks and the block it writes from them on its volume at the same time.
The volume is too small for the group the Compactor is working on.

The largest group is usually the first one after the Compactor starts deduplicating Receive replicas.
Replicas that the Compactor compacted separately before then share that group, so it holds every replica's copy of the data.
The first run after an [overlap halt](#overlapping-blocks-from-receive-restarts) is such a case.

The volume can also be smaller than the chart value `thanos.compactor.persistence.size` says.
The size renders into the StatefulSet's `volumeClaimTemplates`, which Kubernetes applies only when it creates a PVC.
Raising the value later leaves the existing PVC at the size it was created with.

To resolve it, follow these steps:

1. Compare the volume's size with the size the StatefulSet requests:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster

   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get pvc data-thanos-compactor-0 \
     -o jsonpath='{.status.capacity.storage}{"\n"}'
   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get statefulset thanos-compactor \
     -o jsonpath='{.spec.volumeClaimTemplates[0].spec.resources.requests.storage}{"\n"}'
   ```

1. Grow `data-thanos-compactor-0`.
   If the volume is smaller than the StatefulSet's request, grow it to the request.
   If it already matches, grow it past the request.
   For the procedure, see [Temporarily increase storage of working PVCs](../kubernetes/#temporarily-increase-storage-of-working-pvcs).

1. [Restart the Compactor](#restart-the-compactor).

1. [Confirm the recovery](#confirm-the-recovery).

After the backlog drains, decide whether to keep the larger volume.
If you keep it, raise `thanos.compactor.persistence.size` to match, so that a recreated PVC has the same size.
Otherwise, [return the PVC to the chart's size](../kubernetes/#return-the-pvc-to-the-charts-size).
