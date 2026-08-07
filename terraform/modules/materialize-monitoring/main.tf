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
  }

  depends_on = [
    helm_release.crds,
    kubernetes_secret.grafana_admin,
  ]
}
