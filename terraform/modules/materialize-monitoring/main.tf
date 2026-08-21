resource "kubernetes_namespace" "monitoring" {
  count = var.create_namespace ? 1 : 0

  metadata {
    name = var.namespace
  }
}

locals {
  namespace = var.create_namespace ? kubernetes_namespace.monitoring[0].metadata[0].name : var.namespace
}

# ==============================================================================
# Grafana admin credentials
# ==============================================================================
# The chart consumes secrets by name and does not mint them. Terraform is one of
# the few delivery targets where generation actually works, so it provides the
# Secret rather than letting the bundled Grafana chart generate one — that
# generation does not survive an upgrade.

resource "random_password" "grafana_admin" {
  count = var.grafana_admin_password == null ? 1 : 0

  length  = 32
  special = true
  # Avoid characters that need escaping in Grafana's own config and in shells
  # operators will paste this into.
  override_special = "-_=+"
}

resource "kubernetes_secret" "grafana_admin" {
  metadata {
    name      = "mzmon-grafana-admin"
    namespace = local.namespace
  }

  data = {
    "admin-user"     = var.grafana_admin_user
    "admin-password" = local.grafana_admin_password
  }

  type = "Opaque"

  depends_on = [kubernetes_namespace.monitoring]
}

locals {
  grafana_admin_password = coalesce(
    var.grafana_admin_password,
    one(random_password.grafana_admin[*].result),
  )
}

locals {
  # Resolved once here rather than at each use, because `count` and the values
  # block have to agree: a Secret that exists with no mount referencing it is as
  # broken as a mount referencing a Secret that was never created.
  #
  # Both fall back to inferring from the value itself, which is correct for a
  # literal and fails the plan for a computed one — see the variables.
  grafana_database_enabled = coalesce(
    var.grafana_database_enabled,
    var.grafana_database_host != null,
  )

  grafana_database_password_secret = local.grafana_database_enabled && coalesce(
    var.grafana_database_manage_password_secret,
    var.grafana_database_password != null,
  )
}

# Grafana's database password, mounted as a file rather than passed through the
# environment or inlined into `grafana.ini` — that block renders into a
# ConfigMap, so a password there would be plaintext in the release manifest and
# in `helm get values`.
resource "kubernetes_secret" "grafana_database" {
  count = local.grafana_database_password_secret ? 1 : 0

  metadata {
    name      = "mzmon-grafana-db"
    namespace = local.namespace
  }

  data = {
    "password" = var.grafana_database_password
  }

  type = "Opaque"

  depends_on = [kubernetes_namespace.monitoring]
}

# ==============================================================================
# Charts
# ==============================================================================
# OCI charts are referenced in full through `chart`, matching how the Materialize
# Terraform modules already install OCI charts on the v2 Helm provider.

# CRDs first, so the main chart's custom resources have something to validate
# against. The main release then installs with `skip_crds`, because the bundled
# grafana-operator ships its CRDs unconditionally and would otherwise create them
# behind this release's back.
resource "helm_release" "crds" {
  count = var.enable_monitoring_crds ? 1 : 0

  name      = "mzmon-crds"
  namespace = local.namespace
  chart     = "${var.chart_registry}/materialize-monitoring-crds"
  version   = local.crds_chart_version
  timeout   = var.install_timeout

  # CRDs are cluster-scoped; the namespace only holds the release metadata.
  create_namespace = false

  # Same reason as the main release below.
  render_subchart_notes = false

  depends_on = [kubernetes_namespace.monitoring]
}

resource "helm_release" "monitoring" {
  name      = "mzmon"
  namespace = local.namespace
  chart     = "${var.chart_registry}/materialize-monitoring"
  version   = local.chart_version
  timeout   = var.install_timeout

  skip_crds = true

  # The provider defaults this to true, unlike `helm install`, where
  # --render-subchart-notes is opt-in. Left on, eight subcharts' notes bury this
  # chart's own — which is where the validators' warnings are printed — and most
  # of what they say is wrong here anyway, since the umbrella wires Grafana,
  # credentials, and endpoints differently than the subcharts assume.
  render_subchart_notes = false

  # The chart runs a pre-install/pre-upgrade validation Job; without this its
  # verdict is never observed and a bad config rolls anyway.
  wait_for_jobs = true

  # Deliberately not atomic. A rollback on a partial first install destroys the
  # evidence of which component failed, and this stack has enough moving parts
  # that the diagnostic is worth more than the automatic cleanup.
  atomic = false

  values = local.values

  lifecycle {
    precondition {
      condition     = local.required_profile == null || fileexists(local.required_profile)
      error_message = <<-EOT
        sizing is "${var.sizing}" but the chart's profile directory is not readable from the module
        (looked for ${coalesce(local.required_profile, "n/a")}).

        The module reads sizing profiles out of the chart directory in the same repository, so this
        means the repository is not present alongside the module. The usual cause is sourcing the
        module by an *absolute* local path: Terraform copies just that directory into
        .terraform/modules/ and leaves charts/ behind.

        Use a git source, or a "./"-relative path. Alternatively set sizing = "medium" (the chart's
        own defaults, which need no profile) and supply sizing through additional_values.
      EOT
    }

    precondition {
      condition     = can(yamldecode(file("${local.chart_dir}/Chart.yaml")).version)
      error_message = <<-EOT
        The chart's Chart.yaml is not readable from the module, so its version cannot be resolved.

        Same cause as above: the repository is not present alongside the module, usually from an
        absolute local path `source`. Use a git source or a "./"-relative path, or pin
        chart_version and crds_chart_version explicitly.
      EOT
    }

    # A `validation` block cannot see another variable on the module's minimum
    # Terraform (cross-variable validation landed in 1.9), so the paired checks
    # for the static credentials live here.
    precondition {
      condition = (
        (var.object_storage_access_key_id == null) ==
        (var.object_storage_secret_access_key == null)
      )
      error_message = <<-EOT
        object_storage_access_key_id and object_storage_secret_access_key must be set together.

        With only one, the objstore client falls back to its default credential chain for the other
        half and fails to authenticate at pod start rather than at plan time.
      EOT
    }

    precondition {
      condition = !(
        var.otlp_auth_bearer_token != null &&
        length(local.otlp_auth_header_entries) > 0
      )
      error_message = <<-EOT
        otlp_auth_bearer_token cannot be combined with otlp_auth_header_secrets or
        otlp_metrics.auth_headers.

        The chart has one auth slot per OTLP destination (`otel.auth.authType`), so only one of
        these can be rendered. Failing here is deliberate: the alternative is silently dropping the
        other, which reaches the destination as an authentication failure at run time.
      EOT
    }

    precondition {
      condition = (
        var.otlp_metrics != null ||
        (var.otlp_auth_bearer_token == null && length(local.otlp_auth_header_entries) == 0)
      )
      error_message = <<-EOT
        OTLP credentials are set but otlp_metrics is null, so no OTLP exporter is enabled and
        nothing would read them.

        Set otlp_metrics (at minimum its `url`), or drop the credentials.
      EOT
    }

    precondition {
      condition     = var.datadog_metrics == null || var.datadog_api_key != null
      error_message = <<-EOT
        datadog_metrics is set but datadog_api_key is null.

        The Datadog exporter authenticates with the API key alone, and the gateway reads it from an
        environment variable the module only writes when the key is given. Without it the exporter
        starts, sends, and is rejected by the intake — `fail_on_invalid_key` reports it in the
        gateway's logs and nowhere else.
      EOT
    }

    precondition {
      condition = (
        var.object_storage_access_key_id == null ||
        try(var.object_storage.cloud, null) == "aws"
      )
      error_message = <<-EOT
        object_storage_access_key_id is set but object_storage.cloud is
        "${try(var.object_storage.cloud, "null")}".

        Static access keys are an S3 concept. GCS takes a service-account key and Azure a
        storage-account key, and neither backend reads an access-key pair — set them through
        additional_values instead.
      EOT
    }
  }

  depends_on = [
    helm_release.crds,
    kubernetes_secret.grafana_admin,
    # The gateway's `envFrom` mount is optional, so a Secret that lands after the
    # pod does is not an error anywhere — the gateway simply starts with empty
    # credentials and every export is rejected until something restarts it.
    kubernetes_secret.alloy_gateway_env,
  ]
}

# Certificate wiring the chart can only complain about at render time, surfaced
# at plan time instead. `check` blocks warn rather than fail, which is the right
# severity here: every one of these is a configuration that installs cleanly and
# then does nothing, so the operator needs to see it — but a hard failure would
# be worse than the chart's own error, which is more specific.
check "certificates" {
  assert {
    condition     = var.certificates_enabled || (var.issuer_ref == null && var.internal_issuer_ref == null && length(var.grafana_external_dns_names) == 0)
    error_message = "issuer_ref / internal_issuer_ref / grafana_external_dns_names are set but certificates_enabled is false, so no Certificate resources are rendered and none of them do anything. Set certificates_enabled = true, or clear them."
  }

  assert {
    condition     = length(var.grafana_external_dns_names) == 0 || var.issuer_ref != null
    error_message = "grafana_external_dns_names is set with no issuer_ref, so no browser-facing certificate is issued. If your load balancer terminates TLS with a cloud-managed certificate that is correct — pass its ARN or resource ID through grafana.service.annotations in additional_values, and clear this list."
  }

  # The namespaced-Issuer-under-split-namespace case is deliberately not checked
  # here: split-namespace is a chart profile a caller composes through
  # `additional_values`, which this module cannot inspect. The chart refuses that
  # combination at render time with a message naming the components affected,
  # which is both more specific and closer to the mistake.
}
