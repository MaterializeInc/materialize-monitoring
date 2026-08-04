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
  version   = var.crds_chart_version
  timeout   = var.install_timeout

  # CRDs are cluster-scoped; the namespace only holds the release metadata.
  create_namespace = false

  depends_on = [kubernetes_namespace.monitoring]
}

resource "helm_release" "monitoring" {
  name      = "mzmon"
  namespace = local.namespace
  chart     = "${var.chart_registry}/materialize-monitoring"
  version   = var.chart_version
  timeout   = var.install_timeout

  skip_crds = true

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
  }

  depends_on = [
    helm_release.crds,
    kubernetes_secret.grafana_admin,
  ]
}
