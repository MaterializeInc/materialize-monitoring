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

  # Loki and Thanos both reach S3 through the same Thanos objstore client, and
  # that client validates the endpoint itself — an empty one is rejected up
  # front ("no s3 endpoint in config file") rather than falling through to the
  # AWS SDK's regional default. So it is required on the AWS path, for both, and
  # is derived here when the caller named only a region. The global host is the
  # last resort: it works in any region, since the client resolves the bucket's
  # own region from it.
  s3_endpoint_given = local.storage == null || local.storage.cloud != "aws" ? null : coalesce(
    local.storage.endpoint,
    local.storage.region == null ? null : "s3.${local.storage.region}.amazonaws.com",
    "s3.amazonaws.com",
  )

  # An S3-compatible store is normally named by URL, and the objstore client
  # wants a bare `host:port` — given a scheme it fails at startup with
  # "Endpoint url cannot have fully qualified paths", which names neither the
  # offending value nor the component. So the scheme is stripped here rather than
  # made the caller's problem.
  #
  # It also *selects the transport*: `http://` means plain HTTP, which the client
  # will not do unless told. Deriving it from the scheme keeps one fact in one
  # place; as two inputs they could disagree, and the failure for that is a TLS
  # handshake error against a plaintext port.
  s3_endpoint = local.s3_endpoint_given == null ? null : replace(local.s3_endpoint_given, "/^https?:///", "")
  s3_insecure = local.s3_endpoint_given != null && can(regex("^http://", local.s3_endpoint_given))

  # The port object storage is actually on, for Loki's egress NetworkPolicy.
  #
  # Hardcoding 443 here is what broke tier 2: a self-hosted store answers on its
  # own port (rustfs on 9000), the policy blocked the dial, and Loki's index
  # gateway failed with a bare `i/o timeout` — which surfaces to a user as every
  # query hanging until the frontend returns 504, with nothing naming the policy.
  # Thanos hid it further by working fine, since the chart writes no policy for it.
  s3_port = local.s3_endpoint == null ? 443 : try(
    tonumber(regex(":(\\d+)$", local.s3_endpoint)[0]),
    local.s3_insecure ? 80 : 443,
  )

  # 443 stays in the list unconditionally, because this same rule is what grants
  # egress to **STS** for workload identity — always 443, and unrelated to
  # wherever the bucket lives. Swapping it for the store's port rather than
  # adding to it would break IRSA on any deployment that puts S3 somewhere else,
  # and the failure is a credential fetch hanging at pod start, not a config
  # error. A superset also means this can only widen what the rule allowed
  # before, so no existing deployment loses access.
  s3_egress_ports = distinct(concat([local.s3_port], [443]))

  # Static credentials, when the deployment has no workload identity to bind to.
  # Both backends reach S3 through the same Thanos objstore client, but they name
  # the keys differently — `access_key`/`secret_key` in the objstore config,
  # `access_key_id`/`secret_access_key` in Loki's own s3 block — so the pair is
  # rendered twice rather than shared.
  static_s3_credentials = var.object_storage_access_key_id != null

  thanos_objstore_config = local.storage == null ? null : (
    local.storage.cloud == "aws" ? yamlencode({
      type = "S3"
      config = merge(
        {
          bucket   = local.storage.thanos_bucket
          endpoint = local.s3_endpoint
        },
        local.storage.region == null ? {} : { region = local.storage.region },
        # The chart renders this whole document into a Secret
        # (`global.objstore.createSecret`), so the key is not exposed by putting
        # it here.
        !local.static_s3_credentials ? {} : {
          access_key = var.object_storage_access_key_id
          secret_key = var.object_storage_secret_access_key
        },
        !local.s3_insecure ? {} : { insecure = true },
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
      loki = merge({
        storage = {
          bucketNames = {
            chunks = local.storage.loki_bucket
            ruler  = local.storage.loki_bucket
          }
          # Only GCS carries everything it needs in the bucket name. S3 needs an
          # endpoint — Loki does not default one, and every component that
          # touches storage crash-loops without it — and Azure needs the account,
          # which is not derivable from the container name.
          object_store = merge(
            { type = local.loki_object_store },
            local.storage.cloud != "aws" ? {} : {
              s3 = merge(
                { endpoint = local.s3_endpoint },
                # Named rather than left to be parsed back out of the endpoint
                # host, so request signing does not depend on that inference.
                local.storage.region == null ? {} : { region = local.storage.region },
                !local.static_s3_credentials ? {} : {
                  access_key_id     = var.object_storage_access_key_id
                  secret_access_key = var.object_storage_secret_access_key
                },
                !local.s3_insecure ? {} : { insecure = true },
              )
            },
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
        },
        !local.static_s3_credentials ? {} : {
          # **Load-bearing.** The Loki chart defaults `configStorageType` to
          # ConfigMap, and the rendered config carries `secret_access_key`
          # verbatim — so leaving the default publishes the key to anyone who can
          # read ConfigMaps in the namespace. Thanos needs no equivalent: its
          # objstore document already renders into a Secret.
          configStorageType = "Secret"
        }
      )
      serviceAccount = { annotations = local.storage.loki_service_account_annotations }

      # With NetworkPolicy on, Loki has no egress to object storage or STS
      # unless it is granted. Broad by necessity: the endpoints are outside the
      # cluster and their addresses are not known here. Narrow it to a VPC
      # endpoint's CIDR through additional_values where you can.
      networkPolicy = local.storage.cloud != "aws" ? {} : {
        externalStorage = {
          ports = local.s3_egress_ports
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
  # Gated on the resolved locals rather than on the values themselves, so a
  # wrapper computing host and password inside the same apply still gets a plan.
  # See `grafana_database_enabled` in variables.tf.
  #
  # Its own document rather than a map merged into `wiring_values`, for the same
  # reason as `storage_documents`: a ternary has to unify the types of both
  # branches, and `{}` against this object cannot be unified — `terraform plan`
  # fails with "Inconsistent conditional result types". A conditionally empty
  # *list* of documents has no such constraint. The keys here do not collide with
  # the `grafana` block in `wiring_values`, so the two documents compose.
  grafana_database_documents = !local.grafana_database_enabled ? [] : [yamlencode({
    grafana = {
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
          # ConfigMap. Omitted entirely when there is no Secret, which is the
          # Cloud SQL Auth Proxy / peer-authentication shape. Both branches are
          # maps of string, so this ternary does unify.
          local.grafana_database_password_secret ? {
            password = "$__file{/etc/secrets/grafana-db/password}"
          } : {},
        )
      }

      # Always emitted, empty when there is no Secret to mount. An empty list is
      # the subchart's own default, so it contributes nothing — and it keeps the
      # conditional inside a list, where the types unify.
      extraSecretMounts = local.grafana_database_password_secret ? [{
        name       = "grafana-db"
        secretName = one(kubernetes_secret.grafana_database[*].metadata[0].name)
        mountPath  = "/etc/secrets/grafana-db"
        readOnly   = true
      }] : []
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

    # node-exporter is in the chart's `default` tag, so opting out needs the
    # circuit breaker: tags are OR'd, and `tags.node-exporter = false` would lose
    # to `tags.default = true`.
    node-exporter = {
      enabled = var.install_node_exporter
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
  # always win over anything a profile happens to set.
  module_documents = concat(
    [yamlencode(local.wiring_values)],
    local.sizing_profiles,
    local.grafana_database_documents,
    local.storage_documents,
    local.azure_identity_document,
    local.storage_class_document,
    local.google_cloud_metrics_document,
    local.datadog_metrics_document,
    local.otlp_metrics_document,
    local.scheduling_document,
    local.zone_spread_document,
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
