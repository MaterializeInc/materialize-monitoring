# Pod-template hash, so a config change actually rolls the Alloy workloads.
#
# The alloy subchart hashes `configMap.content` into its pod template, but only
# when it creates that ConfigMap itself. This chart points it at ConfigMaps the
# umbrella renders instead — `mzmon-alloy-{agent,gateway}` and their `-env`
# pair — so the guard never fires and nothing in the pod template changes when
# the pipeline or the metric filters do. The workloads keep serving the config
# they started with until someone restarts them by hand.
#
# Helm cannot close this from the parent chart. The parent can compute the exact
# hash — it can read all of `.Values` and `.Files` — but a subchart value is
# static YAML, so there is nowhere to put it that reaches the pod template. A
# consumer can, because it holds the values before Helm ever renders them.
#
# A restart is the requirement, not a reload: the `-env` ConfigMaps are consumed
# with `envFrom`, and environment variables are fixed at container start. Neither
# the config reloader nor Alloy's `/-/reload` can pick up a filter change, so
# they would silently no-op on half the config surface.

locals {
  # Every document, not only the pipeline ones. Narrowing this to the value paths
  # that feed those four ConfigMaps would encode chart internals here, which is
  # the coupling that goes stale. The cost is that a Loki-only change also rolls
  # Alloy; the guarantee is that no change is missed and an unchanged apply
  # changes nothing — which is the actual requirement.
  config_hash_inputs = {
    # The pipeline templates and the metric-tier artifacts ship inside the chart,
    # so a chart bump can change the rendered config with no values change.
    chart_version = local.chart_version

    # Decoded and re-encoded so the hash ignores formatting — reindenting a
    # document in `additional_values` rolls nothing. `try` falls back to the raw
    # string for anything yamldecode rejects but Helm would accept.
    documents = [
      for doc in concat(local.module_documents, var.additional_values) :
      try(yamldecode(doc), doc)
    ]
  }

  # Truncated: this is a change detector, not a integrity check, and a 16-char
  # annotation value stays readable in `kubectl describe`.
  config_hash = substr(sha256(jsonencode(local.config_hash_inputs)), 0, 16)

  config_hash_document = yamlencode({
    "alloy-gateway" = {
      controller = {
        podAnnotations = { "mzmon.materialize.cloud/values-hash" = local.config_hash }
      }
    }
    "alloy-agent" = {
      controller = {
        podAnnotations = { "mzmon.materialize.cloud/values-hash" = local.config_hash }
      }
    }
  })
}
