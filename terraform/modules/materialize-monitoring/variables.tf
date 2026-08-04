# ==============================================================================
# Chart selection
# ==============================================================================

variable "chart_registry" {
  description = "OCI registry holding the materialize-monitoring charts. Override for a mirrored or air-gapped registry."
  type        = string
  default     = "oci://ghcr.io/materializeinc/helm-charts"
  nullable    = false
}

variable "chart_version" {
  description = "Version of the materialize-monitoring chart. Pinned deliberately — bump it with the module, not independently."
  type        = string
  default     = "0.8.0"
  nullable    = false
}

variable "crds_chart_version" {
  description = "Version of the materialize-monitoring-crds chart. Tracked separately because the CRDs chart has a deliberately looser lifecycle."
  type        = string
  default     = "0.3.0"
  nullable    = false
}

variable "install_timeout" {
  description = "Timeout for each Helm release, in seconds. Well above Helm's 300s default: a first install brings up Loki, Thanos, Grafana, and both Alloy roles together."
  type        = number
  default     = 900
  nullable    = false
}

# ==============================================================================
# Placement
# ==============================================================================

variable "namespace" {
  description = "Namespace to install the monitoring stack into."
  type        = string
  default     = "monitoring"
  nullable    = false
}

variable "create_namespace" {
  description = "Whether this module creates the namespace. Defaults to false because the Materialize operator module already creates `monitoring` in the supported topology."
  type        = bool
  default     = false
  nullable    = false
}

variable "enable_monitoring_crds" {
  description = <<-EOT
    Install the materialize-monitoring-crds chart (prometheus-operator and grafana-operator CRDs).

    Set false when the cluster already has them from elsewhere — kube-prometheus-stack, or a
    platform team that owns CRDs centrally — since Terraform would otherwise fail trying to
    create objects it does not own.

    Note the teardown blast radius: destroying this release deletes the CRDs, which cascades to
    every GrafanaDashboard, GrafanaDatasource, PrometheusRule, and PodMonitor in the cluster,
    including ones this stack did not create. It is a separate `helm_release` so it can be
    targeted independently (`-target=module.monitoring.helm_release.crds`).
  EOT
  type        = bool
  default     = true
  nullable    = false
}

# ==============================================================================
# Materialize integration
# ==============================================================================

variable "materialize_instance_namespace" {
  description = "Namespace the Materialize instance runs in. Used to scope scrape targets."
  type        = string
  default     = "materialize-environment"
  nullable    = false
}

variable "materialize_operator_namespace" {
  description = "Namespace the Materialize operator runs in."
  type        = string
  default     = "materialize"
  nullable    = false
}

variable "enable_sql_scraper" {
  description = <<-EOT
    Enable the SQL-on-scrape collector against environmentd.

    Off by default. The chart enables it with an empty password, and no `mz_support` role is
    provisioned by the Materialize Terraform modules, so it would come up failing authentication.
    It also targets the legacy metric surface that native endpoints are replacing.

    Supply `sql_scraper_password` when enabling it.
  EOT
  type        = bool
  default     = false
  nullable    = false
}

variable "sql_scraper_password" {
  description = "Password for the SQL scraper's database user. Required when `enable_sql_scraper` is true."
  type        = string
  default     = null
  sensitive   = true
}

variable "install_metrics_server" {
  description = <<-EOT
    Install metrics-server as part of this stack.

    Leave false when the Materialize operator module installs it (the default topology), and set
    it true when that module has `install_metrics_server = false` — otherwise nothing provides the
    metrics API and the Materialize Console silently loses cluster metrics.
  EOT
  type        = bool
  default     = false
  nullable    = false
}

# ==============================================================================
# Sizing
# ==============================================================================

variable "sizing" {
  description = <<-EOT
    Deployment size. The chart's defaults target `medium`, and the small/large profiles are deltas
    from it, so `medium` intentionally applies no profile at all.

    Profiles are read from the chart directory in this repository at the same commit as the pinned
    chart version, so they cannot drift from it. A profile that does not exist yet is skipped, which
    is how Thanos sizing will start applying once those profiles land.
  EOT
  type        = string
  default     = "medium"
  nullable    = false

  validation {
    condition     = contains(["small", "medium", "large"], var.sizing)
    error_message = "sizing must be one of: small, medium, large."
  }
}

# ==============================================================================
# Object storage and workload identity
# ==============================================================================

# Keep this type expression free of blank lines and comments: terraform-docs
# publishes it verbatim, and the docsite renders it inside a raw HTML block that
# a blank line would terminate.
variable "object_storage" {
  description = <<-EOT
    Buckets and workload identity for the logging and metrics backends, supplied by the per-cloud
    wrapper module. Leave null to configure storage yourself through `additional_values`.

    `cloud` selects the objstore dialect. The `*_service_account_annotations` maps carry the
    workload-identity annotation for each component's ServiceAccount — the chart validates that the
    annotation's cloud matches the objstore backend, so a mismatched pair fails at render time
    rather than at pod start.
  EOT
  type = object({
    cloud                               = string
    loki_bucket                         = string
    thanos_bucket                       = string
    region                              = optional(string)
    endpoint                            = optional(string)
    loki_service_account_annotations    = optional(map(string), {})
    thanos_service_account_annotations  = optional(map(string), {})
    gateway_service_account_annotations = optional(map(string), {})
  })
  default = null

  validation {
    condition     = var.object_storage == null || contains(["aws", "gcp", "azure"], try(var.object_storage.cloud, ""))
    error_message = "object_storage.cloud must be one of: aws, gcp, azure."
  }
}

# ==============================================================================
# Grafana
# ==============================================================================

variable "grafana_admin_password" {
  description = "Grafana admin password. Generated when null. Supplied to Grafana as a Secret this module owns, rather than letting the bundled chart mint one — the chart's own generation does not survive upgrades."
  type        = string
  default     = null
  sensitive   = true
}

variable "grafana_admin_user" {
  description = "Grafana admin username."
  type        = string
  default     = "admin"
  nullable    = false
}

# ==============================================================================
# Escape hatch
# ==============================================================================

variable "node_selector" {
  description = <<-EOT
    Node selector for the centralized monitoring workloads.

    Not applied to the Alloy agent: it is a DaemonSet that must reach every node to collect logs
    and node metrics, so constraining it to a workload pool would silently stop collection
    everywhere else.
  EOT
  type        = map(string)
  default     = {}
  nullable    = false
}

variable "tolerations" {
  description = "Tolerations for the monitoring workloads, including the Alloy agent DaemonSet — tolerations widen where a pod may run, which is what a DaemonSet wants."
  type = list(object({
    key      = optional(string)
    operator = optional(string, "Equal")
    value    = optional(string)
    effect   = optional(string)
  }))
  default  = []
  nullable = false
}

variable "additional_values" {
  description = <<-EOT
    Raw YAML documents appended to the Helm values, in order, after everything this module computes.
    Later documents win, so anything here overrides the module's opinion.

    This is the supported way to reach chart settings the module does not model — including
    scheduling (node selectors, tolerations) and Grafana ingress, neither of which the module
    surfaces yet. See the README.
  EOT
  type        = list(string)
  default     = []
  nullable    = false
}
