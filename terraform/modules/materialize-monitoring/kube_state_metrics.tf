# kube-state-metrics pod-label allowlist.
#
# The chart names the Materialize labels in `kube-state-metrics
# .metricLabelsAllowlist`, one entry per resource (`nodes=[...]`, `pods=[...]`).
# Helm overwrites lists rather than merging them, so a caller adding one pod
# label through `additional_values` has to restate the whole list, and leaving
# out the `nodes` entry empties every per-pool panel with nothing failing.
#
# So the chart's list is read and its `pods` entry extended, the way the agent's
# tolerations are in scheduling.tf: a label added to the chart is carried along
# for free, and callers only ever add. Nothing is written when the caller adds
# nothing, which leaves the chart's own list in charge.
#
# Coupled to the entry syntax kube-state-metrics parses, `<resource>=[<k>,...]`.
# `make terraform-render` asserts the composed flag on the rendered Deployment.

locals {
  ksm_allowlist     = try(local.chart_values["kube-state-metrics"].metricLabelsAllowlist, [])
  ksm_pods_entry_re = "^pods=\\[(.*)\\]$"

  ksm_chart_pod_labels = flatten([
    for entry in local.ksm_allowlist :
    split(",", regex(local.ksm_pods_entry_re, entry)[0])
    if can(regex(local.ksm_pods_entry_re, entry))
  ])

  ksm_pod_labels = distinct(concat(local.ksm_chart_pod_labels, var.kube_state_metrics_pod_labels))

  ksm_pod_labels_document = length(var.kube_state_metrics_pod_labels) == 0 ? [] : [yamlencode({
    kube-state-metrics = {
      metricLabelsAllowlist = concat(
        [for entry in local.ksm_allowlist : entry if !can(regex(local.ksm_pods_entry_re, entry))],
        ["pods=[${join(",", local.ksm_pod_labels)}]"],
      )
    }
  })]
}
