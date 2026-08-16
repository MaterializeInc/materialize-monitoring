# Pinned to a named context, for the same reason the substrate is: this root
# installs the whole monitoring stack, and an implicit provider would put it in
# whatever cluster you last used.

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
