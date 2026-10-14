---
title: "Kubernetes"
weight: 90
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Kubernetes runbook

This page is for you if you operate the Kubernetes cluster that `materialize-monitoring` runs on.
It holds the procedures that apply to more than one component.
The component runbooks link here for those steps.

The commands on this page assume the stack runs in the `monitoring` namespace.

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

To grow a working PVC, follow these steps:

1. Check that the PVC's StorageClass allows expansion:

   ```bash
   kubectl get storageclass \
     "$(kubectl -n monitoring get pvc PVC -o jsonpath='{.spec.storageClassName}')" \
     -o jsonpath='{.allowVolumeExpansion}{"\n"}'
   ```

   Replace `PVC` with the PVC's name.

   If the command prints nothing or `false`, you can't grow the PVC in place.
   Raise the component's `persistence.size` in the chart values and upgrade the release instead.
   Then [recreate the PVC](#return-the-pvc-to-the-charts-size), which creates it at the new size.

1. Request the new size:

   ```bash
   kubectl -n monitoring patch pvc PVC -p '{"spec":{"resources":{"requests":{"storage":"SIZE"}}}}'
   ```

   Replace the following:

   - `PVC`: the PVC's name.
   - `SIZE`: the new size, such as `50Gi`.

1. Wait until the PVC reports the new size:

   ```bash
   kubectl -n monitoring get pvc PVC -o jsonpath='{.status.capacity.storage} {.status.conditions[*].type}{"\n"}'
   ```

   While the volume grows, the conditions show `Resizing` and then `FileSystemResizePending`.
   Most CSI drivers grow the filesystem while the pod runs.
   If the condition stays at `FileSystemResizePending`, delete the pod, and the filesystem grows when the new pod mounts the volume.

Some cloud providers limit how often you can resize a volume.
For example, Amazon EBS allows one modification of a volume every six hours.
Choose a size with room to spare, so that you don't need a second resize.

#### Return the PVC to the chart's size

After the job that needed the space finishes, you can return a working PVC to the size its StatefulSet requests by recreating it.
Don't recreate a PVC that isn't a working PVC, because deleting it discards its contents.

To recreate a working PVC, follow these steps:

1. Delete the PVC without waiting.
   The PVC stays `Terminating` until no pod mounts it:

   ```bash
   kubectl -n monitoring delete pvc PVC --wait=false
   ```

1. Delete the pod that mounts it:

   ```bash
   kubectl -n monitoring delete pod POD
   ```

   Replace `POD` with the pod's name, which is the PVC's name without its prefix, such as `thanos-compactor-0` for `data-thanos-compactor-0`.

1. Check that the StatefulSet recreated the pod and a new PVC at the requested size:

   ```bash
   kubectl -n monitoring get pod POD
   kubectl -n monitoring get pvc PVC -o jsonpath='{.status.capacity.storage}{"\n"}'
   ```

   If the new pod stays `Pending` with an event that says its PVC is being deleted, it started before the old PVC was gone.
   Delete the pod again.

For a StatefulSet with more than one replica, such as the Store Gateway, recreate one PVC at a time.
Wait for each pod to be `Ready` before you start the next.
