# Extra metrics destinations.
#
# The gateway remote-writes to the bundled Thanos out of the box; everything here
# fans out in addition to it. Two shapes, and the difference is the chart's, not
# this module's: the three OTLP-family destinations each turn on
# `destination.otel`, a single switch shared by every OTLP exporter, while
# `prometheus_remote_write` writes into a map keyed by destination name — where a
# key of `thanos` retunes the built-in one rather than adding a second.
#
# Credentials are deliberately absent from every document here. They reach the
# gateway through the Secret in gateway_credentials.tf instead, so they stay out
# of the release payload — anything in `values` is readable with `helm get
# values` by anyone who can read the release Secret, and lands in Terraform state
# besides. What the values carry is the *name* of the environment variable the
# rendered Alloy config reads; the Secret supplies what is behind it.

locals {
  google_cloud_metrics_document = var.google_cloud_metrics == null ? [] : [yamlencode({
    pipeline = {
      metrics = {
        gateway = {
          destination = {
            otel = {
              enabled = true
              googleCloudExporter = merge(
                {
                  enabled             = true
                  minMetricImportance = var.google_cloud_metrics.min_importance
                },
                var.google_cloud_metrics.prefix == null ? {} : {
                  prefix = var.google_cloud_metrics.prefix
                },
              )
            }
          }
        }
      }
    }
  })]

  # ----------------------------------------------------------------------------
  # Datadog
  # ----------------------------------------------------------------------------
  # `site` is the only field most deployments set; the two endpoints are derived
  # from it by Datadog's own exporter and are overridable here only for the
  # proxy/private-link case. They are passed through rather than derived from
  # `site` in the module, because the chart already defaults them and a derived
  # value that disagrees with Datadog's own default is worse than no value.
  datadog_metrics_document = var.datadog_metrics == null ? [] : [yamlencode({
    pipeline = {
      metrics = {
        gateway = {
          destination = {
            otel = {
              enabled = true
              datadogExporter = merge(
                {
                  enabled             = true
                  url                 = var.datadog_metrics.site
                  minMetricImportance = var.datadog_metrics.min_importance
                },
                var.datadog_metrics.metric_endpoint == null ? {} : {
                  metricEndpoint = var.datadog_metrics.metric_endpoint
                },
                var.datadog_metrics.logs_endpoint == null ? {} : {
                  logsEndpoint = var.datadog_metrics.logs_endpoint
                },
              )
            }
          }
        }
      }
    }
  })]

  # ----------------------------------------------------------------------------
  # Generic OTLP
  # ----------------------------------------------------------------------------
  # Header names are not secret — only the values behind them are — so the key
  # set is unwrapped with `nonsensitive` to build the env-var names and the
  # values document. The values themselves are never touched here; they go
  # straight into the Secret. Without this the whole document inherits the mark
  # and the module's entire `values` list becomes sensitive, which would take
  # every unrelated setting with it.
  otlp_auth_header_names = nonsensitive(keys(var.otlp_auth_header_secrets))

  # Derived rather than asked for. The chart lets the caller name the variable
  # precisely so a deployment can pick per-backend names, but there is nothing
  # for a *module* caller to decide here: the name only has to be unique among
  # the gateway's env and stable across applies, and the header name already is
  # both.
  otlp_auth_header_env = {
    for name in local.otlp_auth_header_names :
    name => "GATEWAY_OTEL_DEST_HEADER_${upper(replace(name, "/[^A-Za-z0-9]/", "_"))}"
  }

  otlp_auth_headers_inline = try(var.otlp_metrics.auth_headers, null) == null ? {} : var.otlp_metrics.auth_headers

  # Inline headers render into the gateway's pipeline ConfigMap as literals,
  # which is correct for a dataset or tenant name and wrong for a credential —
  # hence the two inputs. Both land in the same list, since the chart takes one.
  otlp_auth_header_entries = concat(
    [for name, value in local.otlp_auth_headers_inline : { key = name, value = value }],
    [for name in local.otlp_auth_header_names : { key = name, valueEnv = local.otlp_auth_header_env[name] }],
  )

  # The chart's `otel.auth` is one stanza shared by every OTLP exporter and
  # selected by a single `authType`, so this is derived rather than asked for:
  # with two auth inputs and one slot, an explicit `auth_type` could only
  # disagree with them. `bearer` and `headers` together is the one combination
  # that cannot be expressed, and main.tf rejects it rather than silently
  # dropping one.
  otlp_auth_type = (
    var.otlp_auth_bearer_token != null ? "bearer" :
    length(local.otlp_auth_header_entries) > 0 ? "headers" :
    "none"
  )

  otlp_metrics_document = var.otlp_metrics == null ? [] : [yamlencode({
    pipeline = {
      metrics = {
        gateway = {
          destination = {
            otel = {
              enabled = true
              otlpExporter = merge(
                {
                  enabled             = true
                  url                 = var.otlp_metrics.url
                  protocol            = var.otlp_metrics.protocol
                  minMetricImportance = var.otlp_metrics.min_importance
                },
                var.otlp_metrics.compression == null ? {} : {
                  compression = var.otlp_metrics.compression
                },
              )
              # `bearer` needs nothing here: the chart's own bearer config
              # already reads GATEWAY_OTEL_DEST_BEARER_TOKEN, which the Secret
              # supplies. Only `headers` is caller-built.
              auth = merge(
                { authType = local.otlp_auth_type },
                local.otlp_auth_type != "headers" ? {} : {
                  headers = { headers = local.otlp_auth_header_entries }
                },
              )
            }
          }
        }
      }
    }
  })]
}

# ------------------------------------------------------------------------------
# Prometheus remote-write
# ------------------------------------------------------------------------------
# Unlike the three above, this one is not a *new* destination — the chart already
# remote-writes to the bundled Thanos, and this fans out beside it. The chart's
# value is a map keyed by name, so a key of `thanos` deep-merges over the chart's
# own default destination and any other key adds one. That is what makes
# "retune Thanos" and "add AMP" the same input rather than two.
#
# Each destination becomes its own `prometheus.remote_write` component with its
# own importance filter upstream of it, so `min_importance` is genuinely per
# destination: an AMP workspace on `essential` never buffers the metrics it would
# only discard. That matters more here than on the OTLP side, because AMP bills
# per sample ingested and per active series.
locals {
  # Matches the chart's own derivation (`regexReplaceAll "[^A-Za-z0-9]" "_" | upper`),
  # so the variable this module puts in the Secret is the variable the rendered
  # pipeline reads. Named explicitly in the values below rather than left to both
  # sides deriving it, which is the DEP-204 pattern: one of them changing its rule
  # would otherwise be a silent auth failure at run time.
  prom_dest_slug = {
    for name, _ in var.prometheus_remote_write :
    name => upper(replace(name, "/[^A-Za-z0-9]/", "_"))
  }

  # Credential *names* are not secret, and unwrapping them here keeps the
  # sensitivity mark off the whole values list. See `otlp_auth_header_names`.
  prom_dest_credential_names = nonsensitive(keys(var.prometheus_remote_write_credentials))

  prometheus_remote_write_document = length(var.prometheus_remote_write) == 0 ? [] : [yamlencode({
    pipeline = {
      metrics = {
        gateway = {
          destination = {
            prometheusRemoteWrite = {
              for name, dest in var.prometheus_remote_write :
              name => merge(
                {
                  enabled             = dest.enabled
                  minMetricImportance = dest.min_importance
                  authType            = dest.auth_type
                },
                # Omitted when null so the chart's own default survives. This is
                # what lets `thanos = { min_importance = "recommended" }` retune
                # the bundled destination without restating its URL.
                dest.url == null ? {} : { url = dest.url },
                length(dest.external_labels) == 0 ? {} : { externalLabels = dest.external_labels },
                # SigV4 carries no credentials at all: the AWS SDK signs from
                # the pod's IRSA identity. `role_arn` is the cross-account case,
                # where the gateway's own role assumes another one.
                dest.auth_type != "sigv4" ? {} : {
                  sigv4 = merge(
                    dest.sigv4_region == null ? {} : { region = dest.sigv4_region },
                    dest.sigv4_role_arn == null ? {} : { roleArn = dest.sigv4_role_arn },
                  )
                },
                # Only the env-var *names*; the values are in the Secret. The
                # chart derives the same names by default, but naming them is
                # what makes the pair unable to drift if either rule changes.
                dest.auth_type != "basicAuth" ? {} : {
                  basicAuth = {
                    usernameEnv = "GATEWAY_PROMETHEUS_DEST_${local.prom_dest_slug[name]}_USERNAME"
                    passwordEnv = "GATEWAY_PROMETHEUS_DEST_${local.prom_dest_slug[name]}_PASSWORD"
                  }
                },
                dest.auth_type != "bearer" ? {} : {
                  bearer = {
                    tokenEnv = "GATEWAY_PROMETHEUS_DEST_${local.prom_dest_slug[name]}_BEARER_TOKEN"
                  }
                },
              )
            }
          }
        }
      }
    }
  })]

  # IRSA and Workload Identity both reach the gateway the same way — an
  # annotation on its ServiceAccount — and a SigV4 destination has nowhere else
  # to get credentials from. Its own document rather than a merge into
  # `storage_documents`, because that one only exists when `object_storage` is
  # set and AMP does not imply a bucket. Helm deep-merges maps across documents,
  # so the two compose when both are present.
  gateway_service_account_document = length(var.gateway_service_account_annotations) == 0 ? [] : [yamlencode({
    alloy-gateway = {
      serviceAccount = { annotations = var.gateway_service_account_annotations }
    }
  })]
}
