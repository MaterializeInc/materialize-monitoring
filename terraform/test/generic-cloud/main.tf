# The generic-cloud substrate for tier-2 E2E: a fourth cloud, in kind.
#
# Provisions what a cloud wrapper would provision — S3-compatible object storage
# with credentials, and a Postgres database — and stops there. It does not call
# the monitoring module. That separation is deliberate: the substrate has to be
# provable on its own, and it is what a tier-2 root composes with the module.
#
# Garage over MinIO or LocalStack: it is a real S3 implementation, so "does
# Loki's compactor talk to this endpoint correctly" is answered by a real server
# rather than an emulator. See the design doc's LocalStack comparison.
#
# What this cannot cover, by construction: workload identity. Garage authenticates
# with static access keys and kind has no OIDC issuer an IAM provider trusts, so
# IRSA and GKE Workload Identity are only exercised at tier 3. The chart-side
# validators assert the *shape* of that config instead.

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

  # The Service exposes four ports — S3, RPC, web, admin — and only the S3 one is
  # the endpoint. Selected by port name rather than by index or a literal 3900,
  # because the module turns this port into Loki's egress NetworkPolicy rule and a
  # wrong one there fails as a bare `i/o timeout` that names nothing.
  s3_api_port = one([
    for p in data.kubernetes_service.garage.spec[0].port : p.port if p.name == "s3-api"
  ])

  # In-cluster, so plain HTTP. TLS to the object store is a tier-3 concern; here
  # it would only test cert plumbing we did not write.
  s3_endpoint = "http://${data.kubernetes_service.garage.metadata[0].name}.${local.namespace}.svc.cluster.local:${local.s3_api_port}"
}

# The chart names its Service after the release. Looked up instead of hardcoded so
# a chart bump that renames it — or moves the S3 API off its port — fails here
# rather than leaving every consumer pointed at something that does not resolve.
data "kubernetes_service" "garage" {
  metadata {
    name      = helm_release.garage.name
    namespace = local.namespace
  }
}

# ==============================================================================
# Object storage
# ==============================================================================

# Garage fixes the shape of its credentials: an access key is `GK` followed by 24
# hex characters and a secret key is 64, and the chart's configure job checks both
# against that regex before it will import them. `random_id` rather than
# `random_password`, because `.hex` is the generator that produces exactly that
# alphabet — `random_password` has no way to say "hex", and a rejected key fails
# inside a Helm hook where the error is a Job log rather than a plan error.
resource "random_id" "s3_access_key" {
  byte_length = 12
  prefix      = "GK"
}

resource "random_id" "s3_secret_key" {
  byte_length = 32
}

# Garage's own cluster secrets, pinned rather than left to the chart. The chart
# generates them with `randHex` at render time, which is re-rolled on every
# render: each `terraform apply` would rotate the RPC secret and restart the
# StatefulSet under a stack that is mid-test.
resource "random_id" "garage_rpc_secret" {
  byte_length = 32
}

resource "random_id" "garage_admin_token" {
  byte_length = 32
}

# ==============================================================================
# cert-manager
# ==============================================================================

# Cluster plumbing rather than substrate storage, and it lives here for the same
# reason Garage does: it is what a cloud wrapper would already have installed. The
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

# The upstream project publishes no chart to any Helm repository, and its own docs
# install from a clone of the source tree. So this is a community mirror, pinned
# by version.
# It is the better chart regardless of provenance: `clusterConfig` is what makes
# Garage usable from Terraform at all, running the cluster layout, the key import,
# and the bucket grants as a post-install hook. Without it the substrate would owe
# a hand-written bootstrap against the admin API, since a fresh Garage node serves
# no S3 until a layout is applied and holds no credentials until a key is imported.
resource "helm_release" "garage" {
  name       = "garage"
  namespace  = local.namespace
  repository = "https://datahub-local.github.io/garage-helm"
  chart      = "garage"
  version    = "0.8.0"

  # The post-install hook sleeps before and after it configures the cluster, so a
  # first install spends three minutes in the hook alone. 600 is not enough margin
  # on a loaded runner once image pulls are counted.
  timeout = 900

  # ---------------------------------------------------------------------------
  # Cluster shape
  # ---------------------------------------------------------------------------

  # The chart defaults to three replicas at replication factor three, which needs
  # three schedulable nodes and tests Garage rather than the stack. Both have to
  # move together: a factor higher than the node count leaves every write refused
  # for lack of quorum, which surfaces as Loki failing to flush rather than as
  # anything naming the object store.
  set {
    name  = "deployment.replicaCount"
    value = "1"
  }

  set {
    name  = "garage.replicationFactor"
    value = "1"
  }

  # Garage validates the region a request was signed for; it does not ignore it
  # the way some S3-compatible stores do. This has to be the same region tier 2
  # hands the module, or every signed request comes back 403 with a signature
  # complaint that reads like a wrong secret key.
  set {
    name  = "garage.s3.api.region"
    value = var.s3_region
  }

  set_sensitive {
    name  = "garage.secret.rpcSecret"
    value = random_id.garage_rpc_secret.hex
  }

  set_sensitive {
    name  = "garage.secret.adminToken"
    value = random_id.garage_admin_token.hex
  }

  # No probes, deliberately, and the chart ships none. Garage's `/health` reports
  # unhealthy until a layout is applied, and the layout is applied by the
  # post-install hook — which Helm does not run until the workload is ready. A
  # readinessProbe on the obvious endpoint deadlocks the install.

  # ---------------------------------------------------------------------------
  # Storage
  # ---------------------------------------------------------------------------

  # The chart leaves both classes null, which means the cluster default. kind has
  # one, so this is belt-and-braces there, but it is load-bearing on any cluster
  # whose default class is not the one the rest of the substrate uses.
  set {
    name  = "persistence.data.storageClass"
    value = var.storage_class
  }

  set {
    name  = "persistence.data.size"
    value = var.storage_size
  }

  set {
    name  = "persistence.meta.storageClass"
    value = var.storage_class
  }

  set {
    name  = "persistence.meta.size"
    value = var.metadata_storage_size
  }

  # ---------------------------------------------------------------------------
  # Cluster configuration: layout, buckets, credentials
  # ---------------------------------------------------------------------------

  set {
    name  = "clusterConfig.enabled"
    value = "true"
  }

  # Layout capacity, named rather than left to default. The chart would otherwise
  # hand `layout assign -c` the Kubernetes quantity verbatim, and Garage parses
  # that field with `bytesize`, which knows `G` but not the `Gi` a quantity is
  # written in. It is a relative placement weight on a one-node cluster, so the
  # binary-to-decimal slip the strip introduces costs nothing; a value that does
  # not parse costs everything, because the configure hook runs under `|| true`
  # and an unassigned node means no layout, which means no S3 service at all.
  set {
    name  = "clusterConfig.layout.capacity"
    value = replace(var.storage_size, "i", "")
  }

  # Buckets, because both Loki and Thanos expect theirs to exist and neither
  # creates one. The rest of `clusterConfig.layout` stays at its defaults: a
  # single zone, which is the only shape one node can be in.
  set {
    name  = "clusterConfig.buckets[0].name"
    value = var.loki_bucket
  }

  set {
    name  = "clusterConfig.buckets[1].name"
    value = var.thanos_bucket
  }

  # One key with read and write on both buckets. Two keys would model a cloud more
  # closely, but the module takes a single credential pair for both backends, so a
  # second key here could not be wired to anything.
  set {
    name  = "clusterConfig.keys.mzmon.keyId"
    value = random_id.s3_access_key.hex
  }

  set_sensitive {
    name  = "clusterConfig.keys.mzmon.secretKey"
    value = random_id.s3_secret_key.hex
  }

  set {
    name  = "clusterConfig.keys.mzmon.buckets[0]"
    value = var.loki_bucket
  }

  set {
    name  = "clusterConfig.keys.mzmon.buckets[1]"
    value = var.thanos_bucket
  }

  wait = true

  depends_on = [kubernetes_namespace.substrate]
}

# The configure hook runs every one of its commands under `|| true`, so a bucket
# that was never created and a key that was never imported both leave the release
# green. This Job is what turns that back into a failure, and it asserts something
# the hook could not anyway: that the exact credentials tier 2 is about to hand
# Loki and Thanos can round-trip an object through the exact buckets it is about
# to name. A Job rather than an init container — it runs once per apply, and its
# logs are the diagnostic.
resource "kubernetes_job" "verify_buckets" {
  metadata {
    name      = "garage-verify-buckets"
    namespace = local.namespace
  }

  spec {
    backoff_limit = 6

    template {
      metadata {
        labels = { app = "garage-verify-buckets" }
      }

      spec {
        restart_policy = "OnFailure"

        container {
          name  = "awscli"
          image = "amazon/aws-cli:2.31.19"

          env {
            name  = "AWS_ACCESS_KEY_ID"
            value = random_id.s3_access_key.hex
          }
          env {
            name  = "AWS_SECRET_ACCESS_KEY"
            value = random_id.s3_secret_key.hex
          }
          env {
            name  = "AWS_DEFAULT_REGION"
            value = var.s3_region
          }

          command = ["/bin/sh", "-c"]
          args = [
            # A write and a read, not just `ls`: a bucket can exist and still
            # refuse the key, which is the failure a permissions regression in the
            # configure hook produces, and it is invisible to a listing.
            <<-EOT
              set -eu
              for b in ${var.loki_bucket} ${var.thanos_bucket}; do
                echo ok | aws --endpoint-url ${local.s3_endpoint} s3 cp - "s3://$b/.mzmon-substrate-check"
                aws --endpoint-url ${local.s3_endpoint} s3 cp "s3://$b/.mzmon-substrate-check" - >/dev/null
                aws --endpoint-url ${local.s3_endpoint} s3 rm "s3://$b/.mzmon-substrate-check"
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

  depends_on = [helm_release.garage]
}

# The credentials, in the shape a consumer reads them: one Secret it can mount or
# reference, rather than values threaded through Terraform outputs into Helm.
resource "kubernetes_secret" "s3_credentials" {
  metadata {
    name      = "mzmon-objstore-credentials"
    namespace = local.namespace
  }

  data = {
    AWS_ACCESS_KEY_ID     = random_id.s3_access_key.hex
    AWS_SECRET_ACCESS_KEY = random_id.s3_secret_key.hex
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
