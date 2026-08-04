# Values composition.
#
# The Helm provider merges `values` in order, later documents winning. The order
# here is deliberate:
#
#   1. wiring      — what only Terraform knows (namespaces, buckets, identity)
#   2. sizing      — the chart's own profile files, read from this repository
#   3. additional  — the caller's raw YAML, last, so it always wins
#
# Keeping the caller last is what makes the escape hatch real: any chart setting
# is reachable without a module release.

locals {
  # ----------------------------------------------------------------------------
  # Sizing profiles
  # ----------------------------------------------------------------------------
  # Read straight from the chart directory, so a profile and the chart version
  # that ships it can never disagree. `medium` is the chart's own defaults and
  # therefore has no files. Profiles that do not exist yet are skipped rather
  # than failing, which is how the Thanos profiles start applying the moment
  # they land without a change here.
  #
  # This depends on the whole repository being present alongside the module.
  # It is with the two supported source forms — a git source clones the repo,
  # and a `./`-relative local path is used in place. It is NOT with an
  # *absolute* local path, which Terraform copies into `.terraform/modules/`
  # on its own, leaving `charts/` behind. `helm_release.monitoring` carries a
  # precondition for that case, because silently dropping the sizing profile
  # is a far worse failure than refusing to plan.
  profile_dir = "${path.module}/../../../charts/materialize-monitoring/profiles"

  # The tier's Loki profile is the sentinel: it ships with every version of the
  # chart, so its absence means the chart directory is unreachable rather than
  # that the profile has not been written yet.
  required_profile = var.sizing == "medium" ? null : "${local.profile_dir}/loki-${var.sizing}.values.yaml"

  candidate_profiles = var.sizing == "medium" ? [] : [
    "${local.profile_dir}/loki-${var.sizing}.values.yaml",
    "${local.profile_dir}/thanos-${var.sizing}.values.yaml",
  ]

  sizing_profiles = [
    for p in local.candidate_profiles : file(p) if fileexists(p)
  ]

  # ----------------------------------------------------------------------------
  # Object storage
  # ----------------------------------------------------------------------------
  storage = var.object_storage

  # Loki names its backend with a lowercase object_store type; Thanos uses its
  # own uppercase dialect and a different config shape per cloud.
  loki_object_store = local.storage == null ? null : lookup({
    aws   = "s3"
    gcp   = "gcs"
    azure = "azure"
  }, local.storage.cloud, null)

  thanos_objstore_config = local.storage == null ? null : (
    local.storage.cloud == "aws" ? yamlencode({
      type = "S3"
      config = merge(
        { bucket = local.storage.thanos_bucket },
        local.storage.region == null ? {} : { region = local.storage.region },
        local.storage.endpoint == null ? {} : { endpoint = local.storage.endpoint },
      )
      }) : local.storage.cloud == "gcp" ? yamlencode({
      type   = "GCS"
      config = { bucket = local.storage.thanos_bucket }
      }) : yamlencode({
      type   = "AZURE"
      config = { container = local.storage.thanos_bucket }
    })
  )

  # Emitted as its own document rather than merged into `wiring_values`: a
  # conditional map would force both ternary branches to share a type, and a
  # separate document is closer to how the values list actually composes.
  storage_documents = local.storage == null ? [] : [yamlencode({
    loki = {
      loki = {
        storage = {
          bucketNames = {
            chunks = local.storage.loki_bucket
            ruler  = local.storage.loki_bucket
          }
          object_store = { type = local.loki_object_store }
        }
        # Must match storage.object_store.type or the compactor fails at startup.
        compactor = { delete_request_store = local.loki_object_store }
      }
      serviceAccount = { annotations = local.storage.loki_service_account_annotations }

      # With NetworkPolicy on, Loki has no egress to object storage or STS
      # unless it is granted. Broad by necessity: the endpoints are outside the
      # cluster and their addresses are not known here. Narrow it to a VPC
      # endpoint's CIDR through additional_values where you can.
      networkPolicy = local.storage.cloud != "aws" ? {} : {
        externalStorage = {
          ports = [443]
          cidrs = ["0.0.0.0/0"]
        }
      }
    }

    thanos = {
      global = {
        objstore = {
          createSecret = true
          config       = local.thanos_objstore_config
        }
        serviceAccount = { annotations = local.storage.thanos_service_account_annotations }
      }
    }

    alloy-gateway = {
      serviceAccount = { annotations = local.storage.gateway_service_account_annotations }
    }
  })]

  # ----------------------------------------------------------------------------
  # Wiring
  # ----------------------------------------------------------------------------
  wiring_values = {
    materialize-system = {
      namespace = var.materialize_instance_namespace
    }
    materialize-operator = {
      namespace = var.materialize_operator_namespace
    }

    materialize = {
      environmentdSQL = {
        serviceMonitor = { enabled = var.enable_sql_scraper }
        secret = {
          create   = var.enable_sql_scraper
          password = var.enable_sql_scraper ? var.sql_scraper_password : ""
        }
      }
    }

    # The operator module installs metrics-server in the default topology;
    # this only turns ours on when the caller says it does not.
    tags = {
      metrics-server = var.install_metrics_server
    }

    grafana = {
      admin = {
        existingSecret = kubernetes_secret.grafana_admin.metadata[0].name
        userKey        = "admin-user"
        passwordKey    = "admin-password"
      }
    }

    # Loki's NetworkPolicy is on by default and the chart requires both
    # namespace selectors when it is — it refuses to render otherwise. They are
    # namespace-derived, so this is wiring only the caller can supply.
    loki = {
      networkPolicy = {
        metrics = {
          namespaceSelector = {
            matchLabels = { "kubernetes.io/metadata.name" = local.namespace }
          }
        }
        ingress = {
          namespaceSelector = {
            matchLabels = { "kubernetes.io/metadata.name" = local.namespace }
          }
        }
      }
    }
  }

  # ----------------------------------------------------------------------------
  # Final ordered list
  # ----------------------------------------------------------------------------
  # Sizing sits before storage so the cloud-specific facts Terraform computes
  # always win over anything a profile happens to set. The caller is last.
  values = concat(
    [yamlencode(local.wiring_values)],
    local.sizing_profiles,
    local.storage_documents,
    local.scheduling_document,
    var.additional_values,
  )
}
