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
  description = <<-EOT
    Version of the materialize-monitoring chart.

    Leave null, which is the supported path: the module reads the version out of the chart's own
    `Chart.yaml` in this repository, so a module ref always installs the chart it shipped with and
    the two cannot drift. Set it only to pin a chart version different from the module's.
  EOT
  type        = string
  default     = null
}

variable "crds_chart_version" {
  description = "Version of the materialize-monitoring-crds chart. Read from its `Chart.yaml` when null, like `chart_version`. Tracked separately because the CRDs chart has a deliberately looser lifecycle."
  type        = string
  default     = null
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

    Teardown also needs the Grafana custom resources deleted before grafana-operator goes, or
    their finalizers have no remover and the CRDs wedge in Terminating. See the "Uninstalling"
    page in the docs.
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

variable "install_node_exporter" {
  description = <<-EOT
    Install node-exporter as part of this stack.

    On by default: node-level metrics are part of the stack's baseline, and nothing else in it
    collects them. Set false when the cluster already runs its own node-exporter DaemonSet — a
    second one wastes a per-node slot and produces the same series twice under two `job` labels,
    which double-counts in any `sum()` over them.

    This writes the chart's `node-exporter.enabled` circuit breaker rather than a tag. Tags are
    OR'd, so `tags.node-exporter = false` would not turn it off while `tags.default` is true.
  EOT
  type        = bool
  default     = true
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

    `azure_storage_account` is required when `cloud` is `azure`, and only then: both Loki and Thanos
    name the account separately from the container. Azure needs nothing else — the annotation plus
    the pod label the module applies are all the Entra webhook requires.
  EOT
  type = object({
    cloud                               = string
    loki_bucket                         = string
    thanos_bucket                       = string
    region                              = optional(string)
    endpoint                            = optional(string)
    azure_storage_account               = optional(string)
    loki_service_account_annotations    = optional(map(string), {})
    thanos_service_account_annotations  = optional(map(string), {})
    gateway_service_account_annotations = optional(map(string), {})
  })
  default = null

  validation {
    condition = (
      var.object_storage == null ||
      try(var.object_storage.cloud, "") != "azure" ||
      try(var.object_storage.azure_storage_account, null) != null
    )
    error_message = "object_storage.azure_storage_account is required when cloud is \"azure\": both Loki and Thanos name the account separately from the container, and neither can derive it."
  }

  validation {
    condition     = var.object_storage == null || contains(["aws", "gcp", "azure"], try(var.object_storage.cloud, ""))
    error_message = "object_storage.cloud must be one of: aws, gcp, azure."
  }
}

# ==============================================================================
# Extra metrics destinations
# ==============================================================================

variable "google_cloud_metrics" {
  description = <<-EOT
    Also export metrics to Google Cloud Monitoring from the Alloy gateway. Null disables it; Thanos
    is unaffected either way.

    `min_importance` picks a metric tier — `essential`, `recommended`, `extended`, `diagnostic`, or
    `all` — and each tier includes the ones below it. This is a cost control: GCM bills per custom
    metric and `all` sends the entire surface.

    Authentication is ADC only. Bind the gateway ServiceAccount to a Google service account holding
    `roles/monitoring.metricWriter` through `object_storage.gateway_service_account_annotations`;
    failing that it falls back to the node's service account, which works only if that account has
    the role.
  EOT
  type = object({
    min_importance = optional(string, "recommended")
    prefix         = optional(string)
  })
  default = null

  validation {
    condition = var.google_cloud_metrics == null || contains(
      ["essential", "recommended", "extended", "diagnostic", "all"],
      try(var.google_cloud_metrics.min_importance, ""),
    )
    error_message = "google_cloud_metrics.min_importance must be one of: essential, recommended, extended, diagnostic, all."
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

# Grafana keeps its own state — users, service accounts and tokens, annotations,
# dashboard versions and permissions, preferences, and alert-rule state — in a
# database separate from the observability data in Thanos and Loki. The chart
# default is SQLite on an `emptyDir`, which loses all of it on every restart.
#
# These variables point it at PostgreSQL instead. They are the Terraform
# equivalent of the chart's `grafana-postgres` profile, minus the replica count:
# raising `grafana.replicas` is a cost decision, so it stays with the caller
# through `additional_values`. Durability across restarts does not need it.
#
# The database and an owning user are the caller's to provision. Grafana runs
# its own schema migrations at startup, so a read/write-only grant fails the
# migration.

variable "grafana_database_host" {
  description = "Hostname of the PostgreSQL database backing Grafana's own state. Null (the default) leaves Grafana on SQLite, where everything created through the UI is lost on every restart. Host only — the port is `grafana_database_port`."
  type        = string
  default     = null
}

variable "grafana_database_port" {
  description = "Port for `grafana_database_host`."
  type        = number
  default     = 5432
  nullable    = false
}

variable "grafana_database_name" {
  description = "Name of the database Grafana owns."
  type        = string
  default     = "grafana"
  nullable    = false
}

variable "grafana_database_user" {
  description = "Database user Grafana connects as. Must own `grafana_database_name`, because Grafana runs schema migrations at startup."
  type        = string
  default     = "grafana"
  nullable    = false
}

variable "grafana_database_ssl_mode" {
  description = <<-EOT
    libpq SSL mode for the Grafana database connection.

    `require` encrypts but does not authenticate the server. `verify-full` also authenticates it
    and is the better choice — but it needs a CA bundle on disk, which this module does not mount:
    supply `grafana.ini.database.ca_cert_path` and the matching `grafana.extraSecretMounts` through
    `additional_values` when you use it.
  EOT
  type        = string
  default     = "require"
  nullable    = false

  validation {
    condition     = contains(["disable", "require", "verify-ca", "verify-full"], var.grafana_database_ssl_mode)
    error_message = "grafana_database_ssl_mode must be one of: disable, require, verify-ca, verify-full."
  }
}

variable "grafana_database_password" {
  description = <<-EOT
    Password for `grafana_database_user`, supplied to Grafana as a Secret this module owns and read
    from a mounted file rather than the environment.

    Never inlined into `grafana.ini`, which renders into a ConfigMap.

    Null when the connection needs no password — a Cloud SQL Auth Proxy sidecar with
    `--auto-iam-authn`, or a `trust`/peer-authenticated database. Note that IAM database
    authentication *without* a proxy does not work: Grafana reads its password once at startup and
    has no refresh hook, so the first reconnect after the token expires fails.
  EOT
  type        = string
  default     = null
  sensitive   = true
}

# The two below exist because Terraform decides how many of a resource to create
# *before* it knows any value that resource's creation depends on. A wrapper
# module that provisions the database in the same apply hands `..._host` an RDS
# endpoint and `..._password` a generated secret, and neither is knowable at plan
# time — so inferring "is there a database" from them fails the plan outright
# with "Invalid count argument", rather than degrading.
#
# Leave both null when the values are literals; the inference is correct there and
# a direct caller never has to think about it. A wrapper sets them explicitly from
# its own plan-time-known intent.

variable "grafana_database_enabled" {
  description = <<-EOT
    Whether to point Grafana at PostgreSQL at all.

    Null infers it from `grafana_database_host`, which is right whenever that host is a literal. Set
    it explicitly when the host is computed from a resource created in the same apply.
  EOT
  type        = bool
  default     = null
}

variable "grafana_database_manage_password_secret" {
  description = <<-EOT
    Whether this module creates the Secret holding the database password, and references it from
    `grafana.ini` with `$__file{}`.

    Null infers it from `grafana_database_password`. Set it explicitly when that password is
    generated in the same apply.

    False is not a way to supply the password by another route: it means no Secret and no
    `$__file{}` reference at all, so a connection that needs one has to get both from
    `additional_values`. That is also the shape for a genuinely passwordless connection — a Cloud
    SQL Auth Proxy sidecar, or peer authentication.
  EOT
  type        = bool
  default     = null
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

variable "storage_class" {
  description = <<-EOT
    StorageClass for the five PVC-backed workloads (Alertmanager, the Loki ruler, and Thanos
    receive/compactor/store-gateway). Null uses the cluster default. Loki's ingesters are
    unaffected — node-local `emptyDir` by design.

    Required where the default class cannot serve the nodes: GCP's C4 and N4 families take only
    Hyperdisk, and every Persistent Disk class fails to attach with `pd-balanced disk type cannot
    be used by <machine-type>`.

    Changing it on an existing install does not move the volumes. `volumeClaimTemplates` are
    immutable, so the old PVCs must be deleted first — discarding their contents.
  EOT
  type        = string
  default     = null
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
