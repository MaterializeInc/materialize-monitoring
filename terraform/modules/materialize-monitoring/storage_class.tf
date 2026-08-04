# StorageClass fan-out.
#
# Five PVC-backed workloads, five keys. Unlike scheduling there is no lever that
# covers several at once: Thanos has a `global` for scheduling but not for
# persistence, and Loki's `_pod.tpl` coalescing does not extend to storage. So
# the document is written out literally rather than generated — at this size an
# explicit map is easier to check against the subcharts than a generator, and it
# sidesteps the mixed nesting depths.
#
# Loki's ingesters are deliberately absent. They run on node-local `emptyDir`,
# with durability coming from the replication factor rather than from a volume.
#
# This is coupled to the pinned chart version. `make terraform-render` renders
# against the vendored subcharts, so a key that moves shows up as a value that
# no longer lands.

locals {
  storage_class_document = var.storage_class == null ? [] : [yamlencode({
    alertmanager = { persistence = { storageClass = var.storage_class } }

    loki = {
      ruler = { persistence = { storageClass = var.storage_class } }
    }

    # `receive.persistence` is the standalone (RouterIngestor) path, which is
    # what this chart runs. Split mode would move it to `receive.ingester`.
    thanos = {
      receive      = { persistence = { storageClass = var.storage_class } }
      compactor    = { persistence = { storageClass = var.storage_class } }
      storegateway = { persistence = { storageClass = var.storage_class } }
    }
  })]
}
