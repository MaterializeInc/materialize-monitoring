# Destination credentials for the Alloy gateway.
#
# The chart mounts two objects that share one name — `mzmon-alloy-gateway-env` —
# into the gateway with `envFrom`: a ConfigMap it renders itself (allowlists,
# tenant map, non-secret destination config) and a Secret it deliberately does
# **not** create, mounted `optional: true`. That Secret is this file. Sharing the
# name is not a collision: they are different kinds, and the chart lists the
# Secret second, so it wins on any key set by both.
#
# Terraform is the delivery target where creating it actually makes sense — the
# operator already has the credential in a variable, and the alternative is a
# `kubectl create secret` step that nothing verifies. Keeping the values out of
# the Helm release is the point: `values` is readable through `helm get values`
# by anyone who can read the release Secret.
#
# The mount being optional is what makes a mistake here quiet. A Secret in the
# wrong namespace, or under the wrong name, is not an error — the gateway starts,
# `sys.env(...)` resolves to the empty string, and the destination rejects every
# request. Both are pinned to the same values the chart uses.

locals {
  # The names the chart's own configs read. Two are fixed by the chart (its
  # datadog and bearer blocks name them literally); the header variables are
  # derived in destinations.tf and written into the values beside this, so the
  # two cannot drift.
  gateway_env_secret_data = merge(
    var.datadog_api_key == null ? {} : {
      GATEWAY_OTEL_DEST_DATADOG_API_KEY = var.datadog_api_key
    },
    var.otlp_auth_bearer_token == null ? {} : {
      GATEWAY_OTEL_DEST_BEARER_TOKEN = var.otlp_auth_bearer_token
    },
    {
      for name in local.otlp_auth_header_names :
      local.otlp_auth_header_env[name] => var.otlp_auth_header_secrets[name]
    },
  )

  # `count` cannot take a sensitive value, and this map is sensitive by
  # construction. Its *size* is not a secret — it is already implied by which
  # credential variables the caller set — so the mark is dropped from the count
  # alone. `try` covers the case where no sensitive credential is set at all, on
  # Terraform versions where `nonsensitive` rejects an unmarked value.
  gateway_env_secret_enabled = try(
    nonsensitive(length(local.gateway_env_secret_data)),
    length(local.gateway_env_secret_data),
  ) > 0

  # Rolls the gateway when a credential changes. `envFrom` variables are fixed at
  # container start, so rotating a key updates the Secret and leaves the running
  # pod authenticating with the old one — indefinitely, since nothing else about
  # the release changed. See config_hash.tf for the same problem on the values
  # side.
  #
  # A truncated SHA-256 of the credentials is not the credentials, so the mark is
  # dropped here too; carrying it would spread to `config_hash`, to the values
  # document built from it, and from there to the whole release.
  gateway_env_credential_hash = try(
    nonsensitive(sha256(jsonencode(local.gateway_env_secret_data))),
    sha256(jsonencode(local.gateway_env_secret_data)),
  )
}

resource "kubernetes_secret" "alloy_gateway_env" {
  count = local.gateway_env_secret_enabled ? 1 : 0

  metadata {
    # Fixed, like the Grafana Secrets above: the chart's `envFrom` names it
    # literally rather than templating it from the release.
    name      = "mzmon-alloy-gateway-env"
    namespace = local.namespace
  }

  data = local.gateway_env_secret_data

  type = "Opaque"

  depends_on = [kubernetes_namespace.monitoring]
}
