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
#   * The Alloy gateway — `controller.podLabels`, which reaches the pod template
#     and never the Deployment's selector. Keyed on the gateway having an
#     `azure.workload.identity/client-id` annotation rather than on
#     `object_storage.cloud`: the gateway needs an Azure identity only for what
#     it reads from Azure Monitor, which is independent of where the buckets are.
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

  # Both sources of gateway annotations; see destinations.tf and values.tf.
  # Only the keys are read, and those are known at plan even when the client ID
  # is not.
  gateway_azure_identity = contains(keys(merge(
    try(local.storage.gateway_service_account_annotations, {}),
    var.gateway_service_account_annotations,
  )), "azure.workload.identity/client-id")

  azure_gateway_identity_document = local.gateway_azure_identity ? [yamlencode({
    alloy-gateway = {
      controller = {
        podLabels = { "azure.workload.identity/use" = "true" }
      }
    }
  })] : []
}
