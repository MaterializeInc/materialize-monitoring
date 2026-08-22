variable "kube_context" {
  description = "kubeconfig context to provision into. Defaults to the cluster `make e2e-cluster` creates; there is no default-to-current-context, because this root creates workloads and a wrong target should be an error."
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

variable "namespace" {
  description = "Namespace for the substrate. Deliberately not `monitoring` — the stack under test should have to reach across a namespace boundary the way it would to a real cloud endpoint."
  type        = string
  default     = "mzmon-cloud"
  nullable    = false
}

variable "create_namespace" {
  description = "Create the namespace. False when a caller already made it."
  type        = bool
  default     = true
  nullable    = false
}

variable "loki_bucket" {
  description = "Bucket name for Loki chunks and rules."
  type        = string
  default     = "mzmon-loki"
  nullable    = false
}

variable "thanos_bucket" {
  description = "Bucket name for Thanos blocks."
  type        = string
  default     = "mzmon-thanos"
  nullable    = false
}

variable "storage_class" {
  description = "StorageClass for the substrate's volumes. Defaults to kind's, which is `standard` — the same local-path provisioner k3s calls `local-path`, which is what the rustfs chart assumes."
  type        = string
  default     = "standard"
  nullable    = false
}

variable "storage_size" {
  description = "Volume size for the object store. Sized for a CI job's worth of telemetry, not for retention."
  type        = string
  default     = "4Gi"
  nullable    = false
}

variable "postgres_instances" {
  description = "Postgres replica count. One is correct here: this stands in for a managed database, and CNPG's HA behaviour is not what the monitoring stack is being tested for."
  type        = number
  default     = 1
  nullable    = false
}

variable "postgres_storage_size" {
  description = "Volume size for the Postgres cluster."
  type        = string
  default     = "2Gi"
  nullable    = false
}

variable "cert_manager_available" {
  description = <<-EOT
    Whether cert-manager is usable in this cluster, independent of whether this
    root installs it.

    Null follows `install_cert_manager`, which is right for the default path.
    Set it true alongside `install_cert_manager = false` on a cluster that
    already has cert-manager, or tier 2 reads the substrate as having none and
    quietly runs without certificates or any TLS hop.
  EOT
  type        = bool
  default     = null
}

variable "install_cert_manager" {
  type        = bool
  default     = true
  description = <<-EOT
    Install cert-manager into the substrate.

    On by default because the monitoring module's certificate support is off
    unless an issuer exists, and tier 2 is where that path is meant to be
    exercised. Turn it off for a run that only cares about storage, or on a
    cluster that already has cert-manager.
  EOT
}
