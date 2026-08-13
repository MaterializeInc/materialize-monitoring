# StorageClass fan-out.
#
# Five keys, of which **four are live by default** — there is no lever covering
# more than one (Thanos has a `global` for scheduling but not persistence).
# Written out literally rather than generated: the nesting depths differ, and at
# this size an explicit map is easier to check against the subcharts.
#
# Live: Alertmanager, the Loki ruler, and the Thanos Store Gateway and Compactor.
#
# Inert but retained: `thanos.receive` defaults to `persistence.enabled: false`
# (node-local `emptyDir` with an explicit `ephemeral-storage` budget; durability
# is the replication factor, and a volume would pin it to one AZ). The key stays
# because re-enabling persistence is a documented escape hatch, and a re-enabled
# volume that silently missed the class would be a worse trap than one no-op key.
#
# Loki's ingesters are absent deliberately: node-local `emptyDir`, durability
# from the replication factor, and no escape hatch worth wiring.
#
# `make terraform-render` asserts every volumeClaimTemplate that *is* rendered
# carries the class, so a live workload missing here fails there rather than in a
# cluster.

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
