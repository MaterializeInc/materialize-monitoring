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
  chart_dir      = "${path.module}/../../../charts/materialize-monitoring"
  crds_chart_dir = "${path.module}/../../../charts/materialize-monitoring-crds"
  profile_dir    = "${local.chart_dir}/profiles"

  # Version comes from the chart itself unless the caller pins one. This is what
  # makes "the module ref names the chart version" structurally true rather than
  # a convention someone has to remember on every bump.
  chart_version      = coalesce(var.chart_version, yamldecode(file("${local.chart_dir}/Chart.yaml")).version)
  crds_chart_version = coalesce(var.crds_chart_version, yamldecode(file("${local.crds_chart_dir}/Chart.yaml")).version)

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

  # Loki names its backend in four places and the chart's defaults name `s3` in
  # all of them. Three matter: `storage.object_store.type`, the schema period,
  # and the compactor's delete-request store. Missing one of those does not
  # degrade — it crash-loops the component that reads it, because the client is
  # chosen by name and then validated against a config that was never populated
  # ("no s3 endpoint in config file"). The fourth, `storage.type`, is inert here.
  #
  # The schema period bites hardest: it selects the *chunk* client, so every
  # ingester fails at startup. Its list is append-only across backend
  # migrations, so it is read from the chart rather than restated here — only the
  # backend name is rewritten, and a period added to the chart is carried along
  # for free.
  chart_values = yamldecode(file("${local.chart_dir}/values.yaml"))

  loki_schema_configs = [
    for c in local.chart_values.loki.loki.schemaConfig.configs : merge(c, {
      object_store = local.loki_object_store
      # An unquoted YAML date decodes to an RFC 3339 timestamp, which Loki
      # rejects. Round it back if that is what happened.
      from = try(formatdate("YYYY-MM-DD", c.from), c.from)
    })
  ]

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
      type = "AZURE"
      # `storage_account` is as required as `container` — the account is not
      # derivable from the container name, and omitting it fails at startup.
      config = {
        storage_account = local.storage.azure_storage_account
        container       = local.storage.thanos_bucket
      }
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
          # Azure needs the account alongside the type; the other two backends
          # carry everything they need in the bucket name.
          object_store = merge(
            { type = local.loki_object_store },
            local.storage.cloud != "azure" ? {} : {
              azure = { account_name = local.storage.azure_storage_account }
            },
          )
          # The legacy selector. Loki ignores it while `use_thanos_objstore` is
          # on, but the chart still renders a `ruler.storage` block from it, so
          # leaving it at the default puts a contradictory s3 store in the config
          # and makes Loki log that it is ignoring it on every start.
          type = local.loki_object_store
        }
        # Selects the chunk client. Left at the chart's default, every ingester
        # crash-loops on a non-AWS backend.
        schemaConfig = { configs = local.loki_schema_configs }
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
  # Grafana state database
  # ----------------------------------------------------------------------------
  # Empty unless `grafana_database_host` is set, so the default install keeps the
  # chart's SQLite behavior rather than emitting a half-written `[database]`
  # block. `host` carries the port because Grafana does not default one and a
  # bare host fails to connect.
  grafana_database_values = var.grafana_database_host == null ? {} : merge(
    {
      "grafana.ini" = {
        database = merge(
          {
            type     = "postgres"
            host     = "${var.grafana_database_host}:${var.grafana_database_port}"
            name     = var.grafana_database_name
            user     = var.grafana_database_user
            ssl_mode = var.grafana_database_ssl_mode
          },
          # `$__file{}` rather than the literal: `grafana.ini` renders into a
          # ConfigMap. Omitted entirely when there is no password, which is the
          # Cloud SQL Auth Proxy / peer-authentication shape.
          var.grafana_database_password == null ? {} : {
            password = "$__file{/etc/secrets/grafana-db/password}"
          },
        )
      }
    },
    var.grafana_database_password == null ? {} : {
      extraSecretMounts = [{
        name       = "grafana-db"
        secretName = one(kubernetes_secret.grafana_database[*].metadata[0].name)
        mountPath  = "/etc/secrets/grafana-db"
        readOnly   = true
      }]
    },
  )

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

    grafana = merge(
      {
        admin = {
          existingSecret = kubernetes_secret.grafana_admin.metadata[0].name
          userKey        = "admin-user"
          passwordKey    = "admin-password"
        }
      },
      local.grafana_database_values,
    )

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
  # always win over anything a profile happens to set.
  module_documents = concat(
    [yamlencode(local.wiring_values)],
    local.sizing_profiles,
    local.storage_documents,
    local.azure_identity_document,
    local.storage_class_document,
    local.google_cloud_metrics_document,
    local.scheduling_document,
  )

  # The caller stays last, so `additional_values` still overrides everything —
  # including the pod-template hash, which is derived from it rather than
  # competing with it. See config_hash.tf.
  values = concat(
    local.module_documents,
    [local.config_hash_document],
    var.additional_values,
  )
}
