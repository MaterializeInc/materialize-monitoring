# Tier 2: the monitoring module against the generic-cloud substrate.
#
# What this proves that tier 1 cannot: the object-storage code paths. Tier 1 runs
# Loki on a local filesystem and no Thanos at all, because Thanos needs a real
# bucket in every deployment shape it supports. Here both backends talk S3 to
# rustfs and Grafana keeps its state in a real Postgres, so the storage wiring,
# the static-credential path, and the Thanos assertions in the E2E suite all
# become live.
#
# What it still cannot prove is workload identity — rustfs takes static keys and
# kind has no OIDC issuer an IAM provider trusts. The substrate says so in its
# `workload_identity_available` output; that gap belongs to tier 3.

data "terraform_remote_state" "substrate" {
  backend = "local"

  config = {
    path = var.substrate_state_path
  }
}

locals {
  substrate = data.terraform_remote_state.substrate.outputs
}

# CNPG generates the application role's password and publishes it in a Secret, so
# it is read here rather than passed in. A data source resolves at plan time,
# which keeps `grafana_database_enabled` plan-time-known — the module expands
# `count` on it, and a value it cannot know yet is an `Invalid count argument`.
data "kubernetes_secret" "postgres" {
  metadata {
    name      = local.substrate.postgres_secret_name
    namespace = local.substrate.namespace
  }
}

module "monitoring" {
  source = "../../modules/materialize-monitoring"

  # `abspath`, because helm resolves this itself rather than relative to the
  # Terraform working directory.
  chart_registry = coalesce(var.chart_registry, abspath("${path.module}/../../../charts"))

  namespace = var.namespace
  sizing    = var.sizing

  materialize_instance_namespace = "materialize-environment"
  materialize_operator_namespace = "materialize"

  object_storage = {
    cloud         = "aws"
    loki_bucket   = local.substrate.loki_bucket
    thanos_bucket = local.substrate.thanos_bucket
    endpoint      = local.substrate.s3_endpoint

    # No service-account annotations: there is no workload identity to bind to.
    # That is the case these static credentials exist for, and the case a
    # self-hosted store hits in production.
  }

  object_storage_access_key_id     = local.substrate.s3_access_key_id
  object_storage_secret_access_key = local.substrate.s3_secret_access_key

  # Grafana on the substrate's Postgres, which is the other half of what a cloud
  # wrapper provisions.
  grafana_database_enabled  = true
  grafana_database_host     = local.substrate.postgres_host
  grafana_database_port     = 5432
  grafana_database_name     = data.kubernetes_secret.postgres.data["dbname"]
  grafana_database_user     = data.kubernetes_secret.postgres.data["username"]
  grafana_database_password = data.kubernetes_secret.postgres.data["password"]
  # CNPG issues a server certificate from its own CA, which nothing in the
  # cluster trusts by default. Verification belongs at tier 3 against a managed
  # database that presents a public chain.
  grafana_database_ssl_mode = "disable"

  # kind's local-path provisioner. The chart's defaults name cloud classes that
  # do not exist here.
  storage_class = "standard"

  # A single-zone kind cluster cannot satisfy a zone-spread constraint.
  min_zones = 0

  # metrics-server is not part of what tier 2 is testing, and a second one fights
  # the cluster's own over the same APIService.
  install_metrics_server = false

  # kind signs kubelet serving certificates with a CA the pods do not trust, so
  # the gateway's cAdvisor scrape fails TLS verification and every `container_*`
  # series is silently absent — `up{job="cadvisor"} == 0` with nothing erroring
  # at install. The chart documents this as the case for a new distribution.
  #
  # Scoped to this test root on purpose: it is a property of kind, not advice for
  # a real cluster, where leaving verification on is correct (EKS and GKE both
  # scrape cAdvisor fine as shipped).
  additional_values = concat(
    [
      yamlencode({
        pipeline = {
          metrics = {
            kubelet = {
              tlsInsecureSkipVerify = true
            }
          }
        }
      })
    ],
    # Certificates, when the substrate installed cert-manager.
    #
    # Through `additional_values` rather than module variables because the module
    # does not model certificates yet — that lands with the load-balancer work,
    # which is what needs the external issuer. The chart surface is what tier 2
    # is qualifying here.
    #
    # The chart bootstraps its own self-signed root, which is the path a Helm-only
    # consumer takes and the one worth having under test. A real deployment points
    # `internal.issuerRef` at its own PKI instead, and that path renders the same
    # Certificates against a different issuer.
    #
    # Short lifetimes deliberately: renewal is the failure that a freshly-installed
    # test cluster cannot see, so the certificate has to age out inside the run
    # rather than a quarter later. cert-manager renews at `renewBefore`, so a 1h
    # certificate with 55m of headroom renews ~5 minutes in.
    !local.substrate.cert_manager_available ? [] : [
      yamlencode({
        certificates = {
          enabled = true
          internal = {
            selfSigned = {
              enabled = true
            }
          }
          duration    = "1h"
          renewBefore = "55m"
        }
      })
    ]
  )

  depends_on = [data.terraform_remote_state.substrate]
}
