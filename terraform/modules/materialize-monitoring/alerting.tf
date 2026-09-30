# Alerting: which bundled rules install, where their alerts go, and the Secret
# receivers read their credentials from.
#
# Both values documents carry only what the caller set. Every attribute of
# `alert_rules` and `alerting` is optional, and an unset one is left out rather
# than written as null, so the chart's own default holds. That matters most for
# maps the chart already fills — `alerting.presets`, `rules.infraWorkloads` — where
# Helm deep-merges a caller's keys into the defaults instead of replacing them.
#
# Filtering is done with `for` comprehensions rather than `cond ? {} : {...}`:
# the two branches of a conditional must unify to one type, and `{}` against an
# object carrying a nested attribute does not. See values.tf.

locals {
  # ----------------------------------------------------------------------------
  # rules.*
  # ----------------------------------------------------------------------------
  alert_rules_overrides = var.alert_rules.overrides == null ? {} : var.alert_rules.overrides

  alert_rules_values = merge(
    { for k, v in {
      enabled      = var.alert_rules.enabled
      capabilities = var.alert_rules.capabilities
      selected     = var.alert_rules.selected
      disabled     = var.alert_rules.disabled
    } : k => v if v != null },

    # `for_duration` becomes the chart's `for`. It is not called `for` here
    # because `{ for = "6h" }` is a `for` expression to the HCL parser, so a
    # caller could only write it quoted.
    { for k, v in {
      overrides = {
        for name, o in local.alert_rules_overrides :
        name => { for k2, v2 in { "for" = o.for_duration, labels = o.labels } : k2 => v2 if v2 != null }
      }
    } : k => v if length(local.alert_rules_overrides) > 0 },

    { for k, v in {
      namespaces = { for k2, v2 in {
        environment = var.alert_rules.environment_namespaces
        exclude     = var.alert_rules.excluded_namespaces
      } : k2 => v2 if v2 != null }
    } : k => v if length(v) > 0 },

    # A fixed list of tiers rather than the object itself, so an unset
    # `infra_workloads` and an unset tier within it take the same path.
    { for k, v in {
      infraWorkloads = {
        for tier in ["core", "important", "nonessential", "daemonset"] :
        tier => var.alert_rules.infra_workloads[tier]
        if var.alert_rules.infra_workloads != null && try(var.alert_rules.infra_workloads[tier], null) != null
      }
    } : k => v if length(v) > 0 },
  )

  alert_rules_document = length(local.alert_rules_values) == 0 ? [] : [yamlencode({
    rules = local.alert_rules_values
  })]

  # ----------------------------------------------------------------------------
  # alerting.*
  # ----------------------------------------------------------------------------
  # The Alertmanager-native parts — receivers, routes, inhibit rules, time
  # intervals, `global` — are passed through as written. The chart does not model
  # receiver types, and neither does this module, so an integration Alertmanager
  # adds needs no module release.
  alerting_values = merge(
    { for k, v in {
      preset          = var.alerting.preset
      presets         = var.alerting.presets
      unknownSeverity = var.alerting.unknown_severity
      receivers       = var.alerting.receivers
      inhibitRules    = var.alerting.inhibit_rules
      timeIntervals   = var.alerting.time_intervals
      templates       = var.alerting.templates
      global          = var.alerting.global
    } : k => v if v != null },

    { for k, v in {
      routes = { for k2, v2 in {
        root  = try(var.alerting.routes.root, null)
        extra = try(var.alerting.routes.extra, null)
      } : k2 => v2 if v2 != null }
    } : k => v if length(v) > 0 },
  )

  alerting_document = length(local.alerting_values) == 0 ? [] : [yamlencode({
    alerting = local.alerting_values
  })]

  # ----------------------------------------------------------------------------
  # Receiver credentials
  # ----------------------------------------------------------------------------
  # The chart mounts `alertmanager-receivers` by default, optionally, at this
  # path, and names both literally, so they are fixed here too.
  alertmanager_namespace           = var.alertmanager_namespace == null ? local.namespace : var.alertmanager_namespace
  alerting_receiver_secret_name    = "alertmanager-receivers"
  alerting_receiver_secret_mount   = "/etc/alertmanager/secrets/${local.alerting_receiver_secret_name}"
  alerting_receiver_secret_keys    = try(nonsensitive(keys(var.alerting_receiver_secrets)), keys(var.alerting_receiver_secrets))
  alerting_receiver_secret_enabled = length(local.alerting_receiver_secret_keys) > 0

  # Every key the receivers read under the default mount. Found in the encoded
  # configuration rather than by walking it, since receivers are arbitrary
  # Alertmanager structures and HCL has no recursion. Nothing in `alerting` is
  # sensitive, so neither is this.
  alerting_referenced_secret_keys = distinct([
    for m in regexall("${local.alerting_receiver_secret_mount}/([^\"\\\\]+)", jsonencode(var.alerting)) : m[0]
  ])

  alerting_missing_secret_keys = sort(setsubtract(local.alerting_referenced_secret_keys, local.alerting_receiver_secret_keys))
}

# Created only when the caller supplies credentials. Left alone otherwise, so
# External Secrets Operator, Vault Agent or a CSI driver can own the same name.
resource "kubernetes_secret" "alertmanager_receivers" {
  count = local.alerting_receiver_secret_enabled ? 1 : 0

  metadata {
    name      = local.alerting_receiver_secret_name
    namespace = local.alertmanager_namespace
  }

  data = var.alerting_receiver_secrets

  type = "Opaque"

  depends_on = [kubernetes_namespace.monitoring]

  lifecycle {
    # A receiver reading a key this Secret lacks is the silent case: the mount
    # exists, the file does not, and Alertmanager fails that notification at the
    # moment it is needed. Checked only when this module owns the Secret; one
    # owned elsewhere is invisible from here.
    precondition {
      condition     = length(local.alerting_missing_secret_keys) == 0
      error_message = "alerting receivers read ${join(", ", local.alerting_missing_secret_keys)} from ${local.alerting_receiver_secret_mount}/, but alerting_receiver_secrets does not set them. Add each as a key there, or point the receiver at a Secret you mount yourself."
    }
  }
}
