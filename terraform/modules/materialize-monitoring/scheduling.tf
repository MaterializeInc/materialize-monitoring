# Scheduling fan-out.
#
# One selector for centralized infrastructure, spread across the subcharts that
# accept one. Smaller than it looks: Loki's `_pod.tpl` resolves every component
# through `coalesce $component.nodeSelector .Values.defaults.nodeSelector ...`
# (and the gateway Deployment coalesces against `defaults` too), so
# `loki.defaults` covers all 23 of its workloads with one key. Thanos has a real
# `global`. That leaves eight keys rather than thirty.
#
# The DaemonSet asymmetry is the part worth being deliberate about:
#
#   * `node_selector` is NOT applied to `alloy-agent`. It is a DaemonSet whose
#     job is to collect from every node, so constraining it to a workload pool
#     would silently stop collecting logs and node metrics from everywhere else.
#   * `tolerations` ARE applied to it. Tolerations widen where a pod may run, so
#     a DaemonSet wants them in order to reach tainted nodes.
#
# This map is coupled to the pinned chart version. `make terraform-check`
# re-derives it from the vendored subcharts and fails on drift.

locals {
  has_node_selector = length(var.node_selector) > 0
  has_tolerations   = length(var.tolerations) > 0

  # Subcharts taking the settings at their top level.
  scheduling_targets_flat = [
    "grafana",
    "grafana-operator",
    "alertmanager",
    "kube-state-metrics",
    "metrics-server",
  ]

  # Subcharts taking them one key down. `thanos.global` covers every Thanos
  # workload; `loki.defaults` covers every Loki workload that renders through
  # `_pod.tpl` — but the two memcached StatefulSets (chunks and results cache)
  # do not, so they need naming explicitly. Rendering the chart is what catches
  # a component that slips out of `defaults`; see the README's testing note.
  scheduling_targets_nested = {
    "thanos"        = ["global"]
    "alloy-gateway" = ["controller"]
    "loki"          = ["defaults", "chunksCache", "resultsCache"]
  }

  # DaemonSets: tolerations only, never a node selector.
  scheduling_targets_daemonset = {
    "alloy-agent" = "controller"
  }

  scheduling_leaf = merge(
    local.has_node_selector ? { nodeSelector = var.node_selector } : {},
    local.has_tolerations ? { tolerations = var.tolerations } : {},
  )

  scheduling_document = (local.has_node_selector || local.has_tolerations) ? [
    yamlencode(merge(
      { for chart in local.scheduling_targets_flat : chart => local.scheduling_leaf },
      { for chart, keys in local.scheduling_targets_nested : chart => { for k in keys : k => local.scheduling_leaf } },
      local.has_tolerations ? {
        for chart, key in local.scheduling_targets_daemonset :
        chart => { (key) = { tolerations = var.tolerations } }
      } : {},
    ))
  ] : []
}
