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

  # `abspath`, because helm resolves this itself rather than relative to the
  # Terraform working directory. Hoisted so the mTLS profiles below are read out
  # of the same chart tree that is being installed — a profile from one version
  # against a chart from another renders values onto paths that moved.
  chart_registry = coalesce(var.chart_registry, abspath("${path.module}/../../../charts"))

  # The mTLS rollout, in order. Composed rather than restated: tier 2 qualifies
  # the profiles the chart actually ships, so a change to one of them is caught
  # here instead of drifting away from a copy.
  #
  # All three, so the tier lands on **phase 3** — the servers require a client
  # certificate. The phases exist to make a *rollout* order-independent, which is
  # a live-cluster concern; a fresh install has no running writers to strand, so
  # applying them together is safe and is the only state worth asserting. Phase 1
  # and phase 2 alone are both states where an anonymous client is still served.
  #
  # `additional_values` is last in the module's value list, so these override the
  # sizing profile's `thanos.receive.extraArgs` rather than the reverse. That
  # matters more than it looks: Helm overwrites lists, and the losing side of
  # that merge is silent — either the TLS flags vanish or
  # `--receive.replication-factor=3` does, and the second one degrades write
  # quorum to 1 without erroring.
  mtls_profiles = [
    for name in ["mtls", "mtls-phase2", "mtls-phase3"] :
    file("${local.chart_registry}/materialize-monitoring/profiles/${name}.values.yaml")
  ]
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
  chart_registry = local.chart_registry

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

  # Certificates, when the substrate installed cert-manager.
  #
  # No issuer named, so the chart bootstraps its own self-signed root. That is
  # the path a consumer without existing PKI takes, and the one worth having
  # under test; a real deployment points `internal_issuer_ref` at its own CA
  # instead, and that path renders the same Certificates against a different
  # issuer. The `issuer_ref` branch is covered at tier 0 by
  # `bin/terraform-render-check.sh`, which has no cluster to need one on.
  #
  # Chart-default lifetimes, deliberately — `certificate_duration` and
  # `certificate_renew_before` are left unset. An earlier version of this used
  # `1h`/`55m` to make renewal happen inside the run, and that turned out to be
  # pathological. renewBefore at 92% of duration means cert-manager renews every
  # ~5 minutes; with six certificates on one small cluster the controller
  # livelocked in an optimistic-locking re-queue loop, stopped renewing entirely,
  # and then reported `Ready=True: "Certificate is up to date and has not
  # expired"` on certificates that had expired 45 minutes earlier. Every TLS hop
  # failed with `certificate has expired` while cert-manager insisted it was fine.
  #
  # Keep renewBefore well under duration (cert-manager's own guidance is a third
  # or less). `tls::survives_renewal` **forces** renewal by deleting the Secret
  # rather than provoking it with a short lifetime, which is both slower and, as
  # above, capable of breaking the thing it is meant to test.
  certificates_enabled = local.substrate.cert_manager_available

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
    # The mTLS rollout itself, when the substrate installed cert-manager. Issuance
    # is a module variable — see `certificates_enabled` above — but the per-hop
    # server and client settings are chart surface with no module equivalent, and
    # deliberately so: which hops are encrypted is a property of the workload, not
    # of the cloud a wrapper is provisioning.
    local.substrate.cert_manager_available ? local.mtls_profiles : []
  )

  depends_on = [data.terraform_remote_state.substrate]
}
