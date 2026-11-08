# Extra metrics destinations.
#
# The gateway remote-writes to the bundled Thanos out of the box; everything here
# fans out in addition to it. Two shapes, and the difference is the chart's, not
# this module's: the three OTLP-family destinations each become an entry in the
# chart's `pipeline.destinations` map, named `googleCloud`, `datadog`, and
# `otlp`, while `prometheus_remote_write` writes into a map keyed by destination
# name — where a key of `thanos` retunes the built-in one rather than adding a
# second.
#
# The OTLP-family entries name every credential variable explicitly, as the
# names the Secret in gateway_credentials.tf has always used, rather than letting
# the chart derive them from the entry name. The Secret and the values then
# change in the same apply only when this file says so.
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
      destinations = {
        googleCloud = {
          type = "googleCloud"
          metrics = {
            minMetricImportance = var.google_cloud_metrics.min_importance
          }
        }
      }
    }
  })]

  # ----------------------------------------------------------------------------
  # Datadog
  # ----------------------------------------------------------------------------
  # `site` is the only field most deployments set; Datadog's exporter derives
  # the metrics endpoint from it, and `metric_endpoint` overrides that only for
  # the proxy/PrivateLink case. The module sends metrics alone, so
  # `logs_endpoint` has nothing to apply to and is not written.
  datadog_metrics_document = var.datadog_metrics == null ? [] : [yamlencode({
    pipeline = {
      destinations = {
        datadog = {
          type   = "datadog"
          site   = var.datadog_metrics.site
          apiKey = { keyEnv = "GATEWAY_OTEL_DEST_DATADOG_API_KEY" }
          metrics = merge(
            { minMetricImportance = var.datadog_metrics.min_importance },
            var.datadog_metrics.metric_endpoint == null ? {} : {
              url = var.datadog_metrics.metric_endpoint
            },
          )
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

  # A destination has one `authType`, so this is derived rather than asked for:
  # with two auth inputs and one slot, an explicit `auth_type` could only
  # disagree with them. `bearer` and `headers` together is the one combination
  # that cannot be expressed, and main.tf rejects it rather than silently
  # dropping one.
  otlp_auth_type = (
    var.otlp_auth_bearer_token != null ? "bearer" :
    length(local.otlp_auth_header_entries) > 0 ? "headers" :
    "none"
  )

  # `url` is a bare host[:port], which is what the gRPC exporter dials. The
  # HTTP exporter needs a URL, so `https://` is added for it. TLS is on with a
  # 1.2 floor, which is what the exporter did before the chart had a `tls`
  # block for it to read.
  otlp_metrics_document = var.otlp_metrics == null ? [] : [yamlencode({
    pipeline = {
      destinations = {
        otlp = merge(
          {
            type     = "otlp"
            url      = var.otlp_metrics.protocol == "http" ? "https://${var.otlp_metrics.url}" : var.otlp_metrics.url
            protocol = var.otlp_metrics.protocol
            tls      = { enabled = true, minVersion = "TLS12" }
            metrics  = { minMetricImportance = var.otlp_metrics.min_importance }
            authType = local.otlp_auth_type
          },
          var.otlp_metrics.compression == null ? {} : {
            compression = var.otlp_metrics.compression
          },
          local.otlp_auth_type != "bearer" ? {} : {
            bearer = { tokenEnv = "GATEWAY_OTEL_DEST_BEARER_TOKEN" }
          },
          local.otlp_auth_type != "headers" ? {} : {
            headers = local.otlp_auth_header_entries
          },
        )
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
  # Built over the union of the destination and credential key sets, not just the
  # destinations. gateway_credentials.tf indexes this by credential key, and a
  # credential naming a destination that does not exist is a caller mistake the
  # precondition in main.tf reports properly — indexing a map that lacks the key
  # would pre-empt it with Terraform's own "Invalid index" instead.
  #
  # `replace` with a slash-wrapped pattern IS a regex in Terraform, matching the
  # chart's `regexReplaceAll "[^A-Za-z0-9]" "_" | upper`. (There is no
  # `regexreplace` function; the slashes are the opt-in.) Same construction as
  # `otlp_auth_header_env` above.
  prom_dest_slug = {
    for name in distinct(concat(
      keys(var.prometheus_remote_write),
      local.prom_dest_credential_names,
    )) :
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

# `google_cloud_metrics.prefix` is accepted and ignored for the deprecation
# window, so a caller still passing it keeps planning. A warning rather than an
# error, because nothing breaks: the export works, it just cannot honor a prefix.
check "google_cloud_metrics_prefix" {
  assert {
    condition     = try(var.google_cloud_metrics.prefix, null) == null
    error_message = "google_cloud_metrics.prefix is deprecated and ignored. The export writes OTLP to Google's Telemetry API, which names every metric prometheus.googleapis.com/<name>/<kind>. Remove it."
  }
}
