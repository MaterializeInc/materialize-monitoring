# Azure Workload Identity, wired by hand.
#
# On AKS the projected token and the four `AZURE_*` variables are normally
# injected by the workload-identity webhook, which only mutates pods carrying
# the label `azure.workload.identity/use: "true"`.
#
# Loki can be labelled — `_pod.tpl` merges `loki.podLabels` into every pod. The
# bundled Thanos chart cannot: its pod template's labels are `thanos.labels` plus
# the component label, with no extension point. Per-component `labels` land on
# the StatefulSet, and `global.podAnnotations` exists with no `podLabels` beside
# it. So the webhook can never see a Thanos pod.
#
# Rather than fall back to a storage-account key (a long-lived credential) or the
# node pool's identity (node-scoped, so every pod on the node inherits it), this
# injects what the webhook would: Thanos's Azure provider tries workload identity
# as part of its credential chain, and that path needs only the env vars and the
# token file. Same posture as Loki, no static secret, no chart patch.
#
# Upstream fix worth filing: `global.podLabels` on the Thanos chart, which would
# make the webhook work and let this file shrink to the Loki label.

locals {
  azure = local.storage != null && local.storage.cloud == "azure" ? local.storage : null

  # Where the projected token is mounted. The path is arbitrary but has to agree
  # between the mount and AZURE_FEDERATED_TOKEN_FILE.
  azure_token_dir  = "/var/run/secrets/azure/tokens"
  azure_token_file = "azure-identity-token"

  azure_identity_document = local.azure == null ? [] : [yamlencode({
    # Loki takes the label, so the webhook handles it the supported way.
    loki = {
      loki = {
        podLabels = { "azure.workload.identity/use" = "true" }
      }
    }

    thanos = {
      global = {
        # The audience is fixed by Entra's token-exchange endpoint; the federated
        # credential on the Azure side must be created with the same value.
        extraVolumes = [{
          name = "azure-identity-token"
          projected = {
            defaultMode = 420
            sources = [{
              serviceAccountToken = {
                path              = local.azure_token_file
                expirationSeconds = 3600
                audience          = "api://AzureADTokenExchange"
              }
            }]
          }
        }]

        extraVolumeMounts = [{
          name      = "azure-identity-token"
          mountPath = local.azure_token_dir
          readOnly  = true
        }]

        extraEnv = [
          {
            name  = "AZURE_CLIENT_ID"
            value = local.azure.azure_client_id
          },
          {
            name  = "AZURE_TENANT_ID"
            value = local.azure.azure_tenant_id
          },
          {
            name  = "AZURE_FEDERATED_TOKEN_FILE"
            value = "${local.azure_token_dir}/${local.azure_token_file}"
          },
          {
            name  = "AZURE_AUTHORITY_HOST"
            value = "https://login.microsoftonline.com/"
          },
        ]
      }
    }
  })]
}
