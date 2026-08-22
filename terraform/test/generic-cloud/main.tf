# The generic-cloud substrate for tier-2 E2E: a fourth cloud, in kind.
#
# Provisions what a cloud wrapper would provision — S3-compatible object storage
# with credentials, and a Postgres database — and stops there. It does not call
# the monitoring module. That separation is deliberate: the substrate has to be
# provable on its own, and it is what a tier-2 root composes with the module.
#
# rustfs over MinIO or LocalStack: it is a real S3 implementation, so "does
# Loki's compactor talk to this endpoint correctly" is answered by a real server
# rather than an emulator. See the design doc's LocalStack comparison.
#
# What this cannot cover, by construction: workload identity. rustfs takes static
# credentials and kind has no OIDC issuer an IAM provider trusts, so IRSA and
# GKE Workload Identity are only exercised at tier 3. The chart-side validators
# assert the *shape* of that config instead.

resource "kubernetes_namespace" "substrate" {
  count = var.create_namespace ? 1 : 0

  metadata {
    name = var.namespace
  }
}

locals {
  namespace = var.create_namespace ? kubernetes_namespace.substrate[0].metadata[0].name : var.namespace

  postgres_release_name = "mzmon-grafana-db"

  # The `cluster` chart suffixes the release name, so the Cluster and every
  # object it owns are `<release>-cluster*`. Read back rather than assumed —
  # getting this wrong is silent: the endpoint simply does not resolve.
  postgres_cluster_name = "${local.postgres_release_name}-cluster"

  # In-cluster, so plain HTTP. TLS to the object store is a tier-3 concern; here
  # it would only test cert plumbing we did not write.
  s3_endpoint = "http://${data.kubernetes_service.rustfs.metadata[0].name}.${local.namespace}.svc.cluster.local:9000"
}

# The rustfs chart names its Service `<release>-svc`, not `<release>`. Looked up
# instead of hardcoded so a chart bump that renames it fails here rather than
# leaving every consumer pointed at a name that does not resolve.
data "kubernetes_service" "rustfs" {
  metadata {
    name      = "${helm_release.rustfs.name}-svc"
    namespace = local.namespace
  }
}

# ==============================================================================
# Object storage
# ==============================================================================

resource "random_password" "s3_access_key" {
  length  = 20
  special = false
}

resource "random_password" "s3_secret_key" {
  length  = 40
  special = false
}

# ==============================================================================
# cert-manager
# ==============================================================================

# Cluster plumbing rather than substrate storage, and it lives here for the same
# reason rustfs does: it is what a cloud wrapper would already have installed. The
# monitoring module consumes an issuer; it does not install the thing that serves
# one.
#
# Only cert-manager itself is installed. The issuer is deliberately left to the
# chart's own `certificates.internal.selfSigned` path, so tier 2 exercises the
# code we ship rather than a `ClusterIssuer` written by hand here that no consumer
# would have.
resource "helm_release" "cert_manager" {
  count = var.install_cert_manager ? 1 : 0

  name             = "cert-manager"
  namespace        = "cert-manager"
  create_namespace = true
  repository       = "https://charts.jetstack.io"
  chart            = "cert-manager"
  version          = "v1.19.1"
  timeout          = 600

  # cert-manager ships its own CRDs and does not install them by default. Nothing
  # else in this repo installs them either — the monitoring CRDs chart carries
  # Prometheus Operator and Grafana CRDs and has no business carrying another
  # ecosystem's.
  set {
    name  = "crds.enabled"
    value = "true"
  }

  # kind runs the control plane on the same node, and cert-manager's webhook has
  # to be reachable from the API server before any Certificate can be admitted.
  # Waiting here is what stops the monitoring release racing it and failing on a
  # webhook that is not serving yet.
  wait = true
}

resource "helm_release" "rustfs" {
  name       = "rustfs"
  namespace  = local.namespace
  repository = "https://charts.rustfs.com"
  chart      = "rustfs"
  version    = "1.0.0-beta.12-preview.1"
  timeout    = 600

  # Generated rather than defaulted: the chart refuses to render with its
  # well-known `rustfsadmin` credentials unless explicitly opted in, and there is
  # no reason to opt in.
  set_sensitive {
    name  = "secret.rustfs.access_key"
    value = random_password.s3_access_key.result
  }

  set_sensitive {
    name  = "secret.rustfs.secret_key"
    value = random_password.s3_secret_key.result
  }

  # The chart defaults to a 4-replica distributed erasure-coded cluster, which
  # needs 4 schedulable nodes and tests rustfs rather than the stack. Standalone
  # is a mode switch, not just a replica count — distributed mode refuses to
  # render below 2 replicas.
  set {
    name  = "mode.distributed.enabled"
    value = "false"
  }

  set {
    name  = "mode.standalone.enabled"
    value = "true"
  }

  set {
    name  = "replicaCount"
    value = "1"
  }

  # The chart defaults to `local-path`, which is the k3s name. kind ships the same
  # provisioner as `standard`, so the PVCs never bind without this.
  set {
    name  = "storageclass.name"
    value = var.storage_class
  }

  set {
    name  = "storageclass.dataStorageSize"
    value = var.storage_size
  }

  wait = true

  depends_on = [kubernetes_namespace.substrate]
}

# rustfs does not create buckets, and both Loki and Thanos expect theirs to
# exist. A Job rather than an init container: it runs once per apply, and its
# logs are the diagnostic when a bucket is missing.
resource "kubernetes_job" "create_buckets" {
  metadata {
    name      = "rustfs-create-buckets"
    namespace = local.namespace
  }

  spec {
    backoff_limit = 6

    template {
      metadata {
        labels = { app = "rustfs-create-buckets" }
      }

      spec {
        restart_policy = "OnFailure"

        container {
          name  = "awscli"
          image = "amazon/aws-cli:2.31.19"

          env {
            name  = "AWS_ACCESS_KEY_ID"
            value = random_password.s3_access_key.result
          }
          env {
            name  = "AWS_SECRET_ACCESS_KEY"
            value = random_password.s3_secret_key.result
          }
          env {
            name = "AWS_DEFAULT_REGION"
            # rustfs ignores the region; the SDK requires one to sign.
            value = "us-east-1"
          }

          command = ["/bin/sh", "-c"]
          args = [
            # `mb` on an existing bucket is an error, not a no-op, so tolerate it
            # — this Job re-runs on every apply.
            <<-EOT
              set -eu
              for b in ${var.loki_bucket} ${var.thanos_bucket}; do
                aws --endpoint-url ${local.s3_endpoint} s3 mb "s3://$b" || \
                  aws --endpoint-url ${local.s3_endpoint} s3 ls "s3://$b" >/dev/null
                echo "bucket ready: $b"
              done
            EOT
          ]
        }
      }
    }
  }

  wait_for_completion = true

  timeouts {
    create = "5m"
    update = "5m"
  }

  depends_on = [helm_release.rustfs]
}

# The credentials, in the shape a consumer reads them: one Secret it can mount or
# reference, rather than values threaded through Terraform outputs into Helm.
resource "kubernetes_secret" "s3_credentials" {
  metadata {
    name      = "mzmon-objstore-credentials"
    namespace = local.namespace
  }

  data = {
    AWS_ACCESS_KEY_ID     = random_password.s3_access_key.result
    AWS_SECRET_ACCESS_KEY = random_password.s3_secret_key.result
  }

  type = "Opaque"

  depends_on = [kubernetes_namespace.substrate]
}

# ==============================================================================
# Postgres
# ==============================================================================
# Stands in for RDS or Cloud SQL. This is what exercises the production Grafana
# state shape — Grafana owning a database and running its own migrations against
# it — which the `grafana-postgres` profile exists to argue for and which
# SQLite-on-emptyDir never tests.

resource "helm_release" "cnpg_operator" {
  name             = "cnpg"
  namespace        = "cnpg-system"
  repository       = "https://cloudnative-pg.github.io/charts"
  chart            = "cloudnative-pg"
  version          = "0.29.0"
  create_namespace = true
  timeout          = 600

  wait = true
}

# The operator's own chart for the Cluster resource, rather than a raw manifest:
# `kubernetes_manifest` dry-runs against the API server at plan time and fails
# when the CRD does not exist yet, which is always on a fresh cluster.
resource "helm_release" "postgres" {
  name       = local.postgres_release_name
  namespace  = local.namespace
  repository = "https://cloudnative-pg.github.io/charts"
  chart      = "cluster"
  version    = "0.8.1"
  timeout    = 900

  set {
    name  = "cluster.instances"
    value = var.postgres_instances
  }

  set {
    name  = "cluster.storage.size"
    value = var.postgres_storage_size
  }

  set {
    name  = "cluster.storage.storageClass"
    value = var.storage_class
  }

  # No backup target in a cluster that lives for one job.
  set {
    name  = "backups.enabled"
    value = "false"
  }

  wait = true

  depends_on = [
    helm_release.cnpg_operator,
    kubernetes_namespace.substrate,
  ]
}
