# Providers pinned to a named context.
#
# This root creates namespaces, StatefulSets, and a Postgres cluster. Left
# implicit, both providers fall back to the current kubeconfig context — so
# `make e2e-generic-cloud` would provision into whatever cluster you last used.
# Naming the context makes a wrong target an error instead of an outcome.
#
# A tier-2 root that composes this with the monitoring module configures its own
# providers; these exist so the substrate is applyable on its own, which is the
# whole point of keeping it separate.

provider "kubernetes" {
  config_path    = var.kubeconfig_path
  config_context = var.kube_context
}

provider "helm" {
  kubernetes {
    config_path    = var.kubeconfig_path
    config_context = var.kube_context
  }
}
