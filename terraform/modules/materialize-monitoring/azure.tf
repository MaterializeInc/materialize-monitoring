# Azure Workload Identity.
#
# The Entra Workload ID webhook injects the projected token and the `AZURE_*`
# variables, but only into pods carrying the label
# `azure.workload.identity/use: "true"`. Each subchart needs a different lever to
# get that label onto its pods:
#
#   * Loki — `loki.podLabels`, which `_pod.tpl` merges into every pod.
#   * Thanos — `global.commonLabels`, which feeds the `thanos.labels` helper that
#     the pod templates render. There is no `podLabels` in this chart, so the
#     label has to travel with the common set.
#
# `commonLabels` also lands on object metadata, which is harmless, and — checked,
# because it would otherwise be a breaking change — *not* in any workload
# selector. Every Thanos selector is a hardcoded two-key match on `component` and
# `instance`, so adding this to a running install does not touch an immutable
# field.
#
# Everything else is the webhook's job. Do not set the token volume or the
# `AZURE_*` variables by hand: `AZURE_AUTHORITY_HOST` differs on Azure Government
# and Azure China, and the webhook resolves it from the cluster's environment
# while a hardcoded value silently breaks on a sovereign cloud.

locals {
  azure = local.storage != null && local.storage.cloud == "azure" ? local.storage : null

  azure_identity_document = local.azure == null ? [] : [yamlencode({
    loki = {
      loki = {
        podLabels = { "azure.workload.identity/use" = "true" }
      }
    }

    thanos = {
      global = {
        commonLabels = { "azure.workload.identity/use" = "true" }
      }
    }
  })]
}
