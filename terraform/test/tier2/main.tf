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
}
