# StorageClass fan-out.
#
# Five PVC-backed workloads, five keys — there is no lever covering more than one
# (Thanos has a `global` for scheduling but not persistence). Written out
# literally rather than generated: the nesting depths differ, and at this size an
# explicit map is easier to check against the subcharts.
#
# Loki's ingesters are absent deliberately: node-local `emptyDir`, durability
# from the replication factor.
#
# `make terraform-render` asserts every volumeClaimTemplate carries the class, so
# a workload missing here fails there rather than in a cluster.

locals {
  storage_class_document = var.storage_class == null ? [] : [yamlencode({
    alertmanager = { persistence = { storageClass = var.storage_class } }

    loki = {
      ruler = { persistence = { storageClass = var.storage_class } }
    }

    # `receive.persistence` is the standalone path this chart runs; split mode
    # would move it to `receive.ingester`.
    thanos = {
      receive      = { persistence = { storageClass = var.storage_class } }
      compactor    = { persistence = { storageClass = var.storage_class } }
      storegateway = { persistence = { storageClass = var.storage_class } }
    }
  })]
}
