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
