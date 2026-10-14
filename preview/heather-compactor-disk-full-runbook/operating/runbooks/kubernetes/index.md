# Kubernetes




# Kubernetes runbook

This page is for you if you operate the Kubernetes cluster that `materialize-monitoring` runs on.
It holds the procedures that apply to more than one component.
The component runbooks link here for those steps.

## Storage

These procedures change the PersistentVolumeClaims (PVCs) that the stack's StatefulSets create from their `volumeClaimTemplates`.

### Temporarily increase storage of working PVCs

A working PVC holds only a cache or a scratch copy of data that lives elsewhere, usually the bucket.
Losing one costs the component time to rebuild it, and loses no data.
A one-off job, such as the Thanos Compactor's first run after a long halt, can need more space than the component needs in steady state.
You can grow a working PVC for that job, and later return it to the chart's size.

The following table lists the stack's PVCs and whether each is a working PVC:

| Component | PVC | Holds | Working PVC |
|---|---|---|---|
| Thanos Compactor | `data-thanos-compactor-0` | Blocks downloaded for the compaction in progress, and the block written from them | Yes |
| Thanos Store Gateway | `data-thanos-storegateway-N` | Index headers built from the bucket's blocks | Yes |
| Alertmanager | `storage-alertmanager-N` | Silences and the notification log | No |
| Loki ruler | `data-loki-ruler-N` | The write-ahead log of recording-rule samples not yet sent | No |

Kubernetes can grow a PVC in place, but never shrinks one.
Nothing reconciles an existing PVC against its StatefulSet's `volumeClaimTemplates`, so a `helm upgrade` leaves a grown PVC at its new size.

#### Grow the PVC

To grow a working PVC, follow these steps.
In each snippet, set `PVC_NAME` to the PVC from the table earlier on this page, and `NEW_SIZE` to the size you want.

1. Check that the PVC's StorageClass allows expansion:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster
   PVC_NAME=data-thanos-compactor-0

   STORAGE_CLASS=$(kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get pvc "$PVC_NAME" \
     -o jsonpath='{.spec.storageClassName}')
   kubectl --context "$KUBECONTEXT" get storageclass "$STORAGE_CLASS" \
     -o jsonpath='{.allowVolumeExpansion}{"\n"}'
   ```

   If the command prints nothing or `false`, you can't grow the PVC in place.
   Raise the component's `persistence.size` in the chart values and upgrade the release instead.
   Then [recreate the PVC](#return-the-pvc-to-the-charts-size), which creates it at the new size.

1. Request the new size:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster
   PVC_NAME=data-thanos-compactor-0
   NEW_SIZE=50Gi

   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" patch pvc "$PVC_NAME" \
     -p "$(printf '{"spec":{"resources":{"requests":{"storage":"%s"}}}}' "$NEW_SIZE")"
   ```

1. Wait until the PVC reports the new size:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster
   PVC_NAME=data-thanos-compactor-0

   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get pvc "$PVC_NAME" \
     -o jsonpath='{.status.capacity.storage} {.status.conditions[*].type}{"\n"}'
   ```

   While the volume grows, the conditions show `Resizing` and then `FileSystemResizePending`.
   Most CSI drivers grow the filesystem while the pod runs.
   If the condition stays at `FileSystemResizePending`, delete the pod, and the filesystem grows when the new pod mounts the volume.

Some cloud providers limit how often you can resize a volume.
For example, Amazon EBS allows one modification of a volume every six hours.
Choose a size with room to spare, so that you don't need a second resize.

#### Return the PVC to the chart's size

After the job that needed the space finishes, you can return a working PVC to the size its StatefulSet requests by recreating it.
Recreate only a working PVC.

> [!WARNING]
> Deleting a PVC discards everything on it, and you can't undo it.
> With the default `reclaimPolicy` of `Delete`, the StorageClass also deletes the disk behind the PVC.
>
> - **Alertmanager:** the replica loses its silences and notification log.
>   Replicas share both by gossip, so a new replica recovers them from its peers.
>   If every replica loses its PVC, silenced alerts notify again, and recent notifications can repeat.
> - **Loki ruler:** the replica loses the recording-rule samples it hasn't sent yet.
>   Those samples are missing from the metrics store for good.

To recreate a working PVC, follow these steps.
In each snippet, set `PVC_NAME` to the PVC, and `POD_NAME` to the pod that mounts it.
The pod's name is the PVC's name without its prefix, such as `thanos-compactor-0` for `data-thanos-compactor-0`.

1. Delete the PVC without waiting.
   The PVC stays `Terminating` until no pod mounts it:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster
   PVC_NAME=data-thanos-compactor-0

   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" delete pvc "$PVC_NAME" --wait=false
   ```

1. Delete the pod that mounts it:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster
   POD_NAME=thanos-compactor-0

   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" delete pod "$POD_NAME"
   ```

1. Check that the StatefulSet recreated the pod and a new PVC at the requested size:

   ```bash
   MONITORING_NAMESPACE=monitoring
   KUBECONTEXT=my-cluster
   PVC_NAME=data-thanos-compactor-0
   POD_NAME=thanos-compactor-0

   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get pod "$POD_NAME"
   kubectl --context "$KUBECONTEXT" -n "$MONITORING_NAMESPACE" get pvc "$PVC_NAME" \
     -o jsonpath='{.status.capacity.storage}{"\n"}'
   ```

   If the new pod stays `Pending` with an event that says its PVC is being deleted, it started before the old PVC was gone.
   Delete the pod again.

For a StatefulSet with more than one replica, such as the Store Gateway, recreate one PVC at a time.
Wait for each pod to be `Ready` before you start the next.

> [!NOTE]
> A recreated working PVC starts empty, so the component pays to rebuild it.
> A Store Gateway replica rebuilds its index headers from the bucket, and serves no historical query until it finishes.
> On a large bucket, that takes minutes.
> The Compactor downloads the blocks of a compaction in progress again.

