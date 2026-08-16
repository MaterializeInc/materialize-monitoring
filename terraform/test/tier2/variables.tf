variable "kube_context" {
  description = "kubeconfig context to install into. Defaults to the cluster `make e2e-cluster` creates; there is no default-to-current-context, because this root installs a whole stack and a wrong target should be an error."
  type        = string
  default     = "kind-mzmon-e2e"
  nullable    = false
}

variable "kubeconfig_path" {
  description = "Path to the kubeconfig holding `kube_context`."
  type        = string
  default     = "~/.kube/config"
  nullable    = false
}

variable "substrate_state_path" {
  description = <<-EOT
    Path to the generic-cloud root's state file, read for the endpoint, buckets and credentials it
    provisioned.

    Composed through state rather than by instantiating the substrate as a child module: the
    substrate configures its own providers so it stays applyable on its own, and a child module
    carrying provider blocks cannot be cleanly removed. Two applies, one direction of dependency.
  EOT
  type        = string
  default     = "../generic-cloud/terraform.tfstate"
  nullable    = false
}

variable "namespace" {
  description = "Namespace for the monitoring stack. Separate from the substrate's, so the stack reaches object storage across a namespace boundary the way it would a real cloud endpoint."
  type        = string
  default     = "monitoring"
  nullable    = false
}

variable "sizing" {
  description = "Sizing profile. `small` is what fits a kind node; `medium` is the chart defaults and wants a larger runner."
  type        = string
  default     = "small"
  nullable    = false
}

variable "chart_registry" {
  description = <<-EOT
    Where to install the chart from. Defaults to this repository's `charts/` directory.

    Load-bearing, not a convenience: the module's default is the published OCI registry, and a tier
    that installed a *released* chart would be testing the wrong artifact — the point of the tier is
    the working tree. `chart_registry` is not required to be a registry; a local directory installs
    from disk, and the module reads its version out of that directory's `Chart.yaml`, so the chart
    and the version cannot drift apart.
  EOT
  type        = string
  default     = null
  nullable    = true
}
