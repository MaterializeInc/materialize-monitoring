# Extra metrics destinations.
#
# The gateway always remote-writes to Thanos; these fan out in addition to it.
# Each one turns on `destination.otel`, which is the chart's switch for the whole
# OTLP path — shared by every OTLP exporter, so enabling a second one later does
# not change this.
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
