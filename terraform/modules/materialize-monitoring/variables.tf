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

    The Grafana custom resources still have to go before grafana-operator does, or their
    finalizers have no remover and the CRDs wedge in Terminating. The chart's `pre-delete` hook
    handles that ordering now — but it lives in the *main* release, so destroying this one first
    takes the resource types out from under it. Destroy in the module's own order, and see the
    "Uninstalling" page in the docs.
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

    `endpoint` applies to `aws` only, and both backends need one — their shared objstore client
    rejects an empty endpoint rather than resolving the AWS SDK's regional default. It is optional
    because the module derives `s3.<region>.amazonaws.com` from `region`, falling back to the global
    host when neither is given; name it to point at a VPC endpoint or an S3-compatible store.
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

variable "object_storage_access_key_id" {
  description = <<-EOT
    Static access key for an S3-compatible object store, for deployments that have no workload
    identity to bind to — an on-prem or self-hosted store (MinIO, rustfs, Ceph), or a cluster whose
    IAM provider does not trust its OIDC issuer.

    Prefer workload identity wherever it exists: it rotates, and the
    `object_storage.*_service_account_annotations` maps are how it is configured. These two
    variables are the fallback, not the default path.

    Set both or neither. `aws` only — GCS takes a service-account key and Azure a storage-account
    key, neither of which is an access-key pair; configure those through `additional_values`.
  EOT
  type        = string
  default     = null
  sensitive   = true
}

variable "object_storage_secret_access_key" {
  description = <<-EOT
    Secret key paired with `object_storage_access_key_id`.

    Reaches the backends as a Secret in both cases, never a ConfigMap: Thanos already renders its
    objstore config into one, and the module switches Loki's `configStorageType` to `Secret` when
    these are set — the chart's default puts the rendered config in a ConfigMap, which would publish
    this value to anyone with read access to the namespace.
  EOT
  type        = string
  default     = null
  sensitive   = true
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

# Keep this type expression free of blank lines and comments: terraform-docs
# publishes it verbatim, and the docsite renders it inside a raw HTML block that
# a blank line would terminate.
variable "datadog_metrics" {
  description = <<-EOT
    Also export metrics to Datadog from the Alloy gateway. Null disables it; Thanos is unaffected
    either way. Pair it with `datadog_api_key`, which is what actually authenticates.

    `site` is your Datadog site — `datadoghq.com`, `datadoghq.eu`, `us3.datadoghq.com`, and so on.
    Getting it wrong is a 403 from the intake, not a routing error.

    `min_importance` picks a metric tier — `essential`, `recommended`, `extended`, `diagnostic`, or
    `all` — and each tier includes the ones below it. This is a cost control, and a sharper one than
    for most backends: Datadog bills per custom metric, so `all` is rarely what you want.

    `metric_endpoint` and `logs_endpoint` override the intake URLs the exporter derives from `site`.
    Leave them null unless you are routing through a proxy or PrivateLink — a hand-written endpoint
    that disagrees with `site` fails at the intake rather than at plan time.
  EOT
  type = object({
    site            = optional(string, "datadoghq.com")
    min_importance  = optional(string, "essential")
    metric_endpoint = optional(string)
    logs_endpoint   = optional(string)
  })
  default = null

  validation {
    condition = var.datadog_metrics == null || contains(
      ["essential", "recommended", "extended", "diagnostic", "all"],
      try(var.datadog_metrics.min_importance, ""),
    )
    error_message = "datadog_metrics.min_importance must be one of: essential, recommended, extended, diagnostic, all."
  }
}

variable "datadog_api_key" {
  description = <<-EOT
    Datadog API key for `datadog_metrics`.

    Delivered as a Secret this module creates (`mzmon-alloy-gateway-env`), never through the Helm
    values — anything in `values` is readable with `helm get values` by anyone who can read the
    release Secret. The gateway reads it from the environment at startup.

    An app key is not needed and is not accepted here; the metrics intake authenticates with the API
    key alone.
  EOT
  type        = string
  default     = null
  sensitive   = true
}

# Keep this type expression free of blank lines and comments: terraform-docs
# publishes it verbatim, and the docsite renders it inside a raw HTML block that
# a blank line would terminate.
variable "otlp_metrics" {
  description = <<-EOT
    Also export metrics to a generic OTLP endpoint from the Alloy gateway — Honeycomb, Grafana
    Cloud, or your own OpenTelemetry Collector. Null disables it; Thanos is unaffected either way.

    `url` is a `host[:port]` with **no** scheme; `https://` in it fails at gateway start. `protocol`
    selects the exporter: `grpc` for OTLP/gRPC, `http` for OTLP/HTTP.

    `min_importance` picks a metric tier — `essential`, `recommended`, `extended`, `diagnostic`, or
    `all` — and each tier includes the ones below it. Metered backends are the reason it defaults
    below `all` here.

    `auth_headers` sets **non-secret** request headers, such as Honeycomb's `x-honeycomb-dataset`.
    They render into the gateway's pipeline ConfigMap as literals, so put credentials in
    `otlp_auth_header_secrets` or `otlp_auth_bearer_token` instead — those reach the gateway through
    a Secret.
  EOT
  type = object({
    url            = string
    protocol       = optional(string, "grpc")
    compression    = optional(string)
    min_importance = optional(string, "recommended")
    auth_headers   = optional(map(string), {})
  })
  default = null

  validation {
    condition = var.otlp_metrics == null || contains(
      ["essential", "recommended", "extended", "diagnostic", "all"],
      try(var.otlp_metrics.min_importance, ""),
    )
    error_message = "otlp_metrics.min_importance must be one of: essential, recommended, extended, diagnostic, all."
  }

  validation {
    condition     = var.otlp_metrics == null || contains(["grpc", "http"], try(var.otlp_metrics.protocol, ""))
    error_message = "otlp_metrics.protocol must be grpc or http."
  }

  validation {
    condition     = var.otlp_metrics == null || !can(regex("://", try(var.otlp_metrics.url, "")))
    error_message = <<-EOT
      otlp_metrics.url must be a host[:port] with no scheme — "api.honeycomb.io", not
      "https://api.honeycomb.io".

      The exporter takes a bare endpoint and selects its transport from `protocol`. A scheme here is
      not rejected at render time; the gateway fails to dial the destination at startup.
    EOT
  }
}

variable "otlp_auth_header_secrets" {
  description = <<-EOT
    Secret request headers for `otlp_metrics`, as header name to value — Honeycomb's
    `x-honeycomb-team`, for instance. This is the API-key-header case, which is how most OTLP
    vendors authenticate.

    Each value is delivered as a Secret this module creates (`mzmon-alloy-gateway-env`) rather than
    through the Helm values, and the gateway reads it from the environment at startup. The module
    derives the variable name from the header (`x-honeycomb-team` becomes
    `GATEWAY_OTEL_DEST_HEADER_X_HONEYCOMB_TEAM`); nothing else depends on it.

    Non-secret headers belong in `otlp_metrics.auth_headers`, which renders them inline. The two
    compose into one header set. Cannot be combined with `otlp_auth_bearer_token`: the chart has one
    auth slot per OTLP destination.
  EOT
  type        = map(string)
  default     = {}
  nullable    = false
  sensitive   = true
}

variable "otlp_auth_bearer_token" {
  description = <<-EOT
    Bearer token for `otlp_metrics`, for endpoints that take an `Authorization: Bearer` header
    rather than a vendor-specific one.

    Delivered as a Secret this module creates (`mzmon-alloy-gateway-env`) rather than through the
    Helm values. Cannot be combined with `otlp_auth_header_secrets` or `otlp_metrics.auth_headers`:
    the chart has one auth slot per OTLP destination.
  EOT
  type        = string
  default     = null
  sensitive   = true
}

# Keep this type expression free of blank lines and comments: terraform-docs
# publishes it verbatim, and the docsite renders it inside a raw HTML block that
# a blank line would terminate.
variable "prometheus_remote_write" {
  description = <<-EOT
    Prometheus remote-write destinations for the Alloy gateway, keyed by name — Amazon Managed
    Prometheus, Grafana Cloud, Mimir, another Thanos. Empty leaves the chart's single bundled
    Thanos destination exactly as it is.

    The key names the destination and becomes its Alloy component label, so it must match
    `[a-zA-Z_][a-zA-Z0-9_]*` and appears in the gateway's own metrics. Two keys are special only by
    convention: `thanos` is the chart's built-in destination, so setting it here **retunes** that
    one rather than adding a second — which is how you drop the bundled backend to a cheaper tier,
    or turn it off with `enabled = false` while keeping another destination.

    Each destination gets its own remote-write component and its own upstream tier filter, so
    `min_importance` — `essential`, `recommended`, `extended`, `diagnostic`, or `all`, each
    including the ones below it — is genuinely per destination: a metered backend on `essential`
    never buffers the metrics it would only discard. Amazon Managed Prometheus bills per sample
    ingested and per active series, which is what that is for.

    `auth_type` is `none`, `sigv4`, `basicAuth`, or `bearer`. `sigv4` needs no credentials at
    all — the gateway signs with the IRSA identity from `gateway_service_account_annotations`. The
    other two take their credentials from `prometheus_remote_write_credentials`, which delivers
    them through a Secret rather than the Helm values.
  EOT
  type = map(object({
    url             = optional(string)
    enabled         = optional(bool, true)
    min_importance  = optional(string, "all")
    auth_type       = optional(string, "none")
    sigv4_region    = optional(string)
    sigv4_role_arn  = optional(string)
    external_labels = optional(map(string), {})
  }))
  default  = {}
  nullable = false

  validation {
    condition = alltrue([
      for name, _ in var.prometheus_remote_write : can(regex("^[a-zA-Z_][a-zA-Z0-9_]*$", name))
    ])
    error_message = <<-EOT
      Every prometheus_remote_write key must match [a-zA-Z_][a-zA-Z0-9_]* — no dashes, dots, or
      leading digits.

      The key becomes an Alloy component label. A name Alloy cannot parse is not caught by
      `terraform validate`; the chart refuses to render, and a hand-written config would fail at
      gateway startup instead.
    EOT
  }

  validation {
    condition = alltrue([
      for name, _ in var.prometheus_remote_write : !contains(
        # `egress` is the fan-out seam's own component label. The rest are the
        # keys the pre-map single-destination shape used, which the chart reads
        # as a leftover override rather than as a destination.
        ["egress", "enabled", "url", "urlEnv", "minMetricImportance", "unfilteredMetricsEnv",
        "externalLabels", "authType", "basicAuth", "bearer", "oauth2", "sigv4", "tls"],
        name,
      )
    ])
    error_message = <<-EOT
      A prometheus_remote_write key may not be `egress`, nor any of the chart's pre-map
      single-destination keys (`enabled`, `url`, `authType`, `basicAuth`, `bearer`, `oauth2`,
      `sigv4`, `tls`, `externalLabels`, `urlEnv`, `minMetricImportance`, `unfilteredMetricsEnv`).

      Both are render-time failures the chart raises and `terraform validate` cannot: `egress`
      collides with the fan-out seam's component label, and the rest are read as a leftover
      override of the old singular shape.
    EOT
  }

  validation {
    condition = alltrue([
      for _, dest in var.prometheus_remote_write : contains(
        ["essential", "recommended", "extended", "diagnostic", "all"], dest.min_importance,
      )
    ])
    error_message = "prometheus_remote_write min_importance must be one of: essential, recommended, extended, diagnostic, all."
  }

  validation {
    condition = alltrue([
      for _, dest in var.prometheus_remote_write : contains(
        ["none", "sigv4", "basicAuth", "bearer"], dest.auth_type,
      )
    ])
    error_message = "prometheus_remote_write auth_type must be one of: none, sigv4, basicAuth, bearer."
  }

  validation {
    condition = alltrue([
      for name, dest in var.prometheus_remote_write :
      dest.url != null || name == "thanos" || dest.enabled == false
    ])
    error_message = <<-EOT
      Every enabled prometheus_remote_write destination needs a url, except `thanos`, which
      inherits the chart's in-cluster endpoint.

      A destination with no url renders a remote_write component whose endpoint resolves to the
      empty string. Alloy accepts that at load, so it surfaces only as writes failing once the
      gateway is running.
    EOT
  }

  validation {
    condition = alltrue([
      for _, dest in var.prometheus_remote_write :
      dest.url == null || can(regex("^https?://", dest.url))
    ])
    error_message = <<-EOT
      Every prometheus_remote_write url must carry an http:// or https:// scheme, and be the full
      remote-write path — for AMP that is
      "https://aps-workspaces.<region>.amazonaws.com/workspaces/<id>/api/v1/remote_write".

      Unlike otlp_metrics.url, which is a bare host, this one is a URL.
    EOT
  }
}

variable "prometheus_remote_write_credentials" {
  description = <<-EOT
    Credentials for `prometheus_remote_write` destinations whose `auth_type` is `basicAuth` or
    `bearer`, keyed by the same destination name.

    Delivered as a Secret this module creates (`mzmon-alloy-gateway-env`) rather than through the
    Helm values, which are readable with `helm get values` and land in Terraform state. The module
    derives the variable names from the destination name — `amp` becomes
    `GATEWAY_PROMETHEUS_DEST_AMP_USERNAME` and friends — and writes those same names into the
    chart's values, so the two cannot disagree.

    `sigv4` destinations need no entry here: they sign with the gateway pod's IRSA identity.
  EOT
  type = map(object({
    username     = optional(string)
    password     = optional(string)
    bearer_token = optional(string)
  }))
  default   = {}
  nullable  = false
  sensitive = true
}

variable "gateway_service_account_annotations" {
  description = <<-EOT
    Annotations for the Alloy gateway's ServiceAccount, for binding it to a cloud identity —
    `eks.amazonaws.com/role-arn` for IRSA, `iam.gke.io/gcp-service-account` for Workload Identity.

    Required by a `sigv4` remote-write destination, which has no other source of credentials.
    Merged with any annotations `object_storage` contributes, so both can be present.
  EOT
  type        = map(string)
  default     = {}
  nullable    = false
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
    StorageClass for the PVC-backed workloads. Null uses the cluster default.

    Four are PVC-backed by default: Alertmanager, the Loki ruler, and the Thanos Store Gateway and
    Compactor. Loki's ingesters and Thanos Receive use node-local `emptyDir` by design — durability
    is the replication factor there, and a volume would pin them to one availability zone. The
    class is still fanned out to Receive so that re-enabling its persistence picks the class up
    rather than silently missing it.

    Required where the default class cannot serve the nodes: GCP's C4 and N4 families take only
    Hyperdisk, and every Persistent Disk class fails to attach with `pd-balanced disk type cannot
    be used by <machine-type>`.

    Changing it on an existing install does not move the volumes. `volumeClaimTemplates` are
    immutable, so the old PVCs must be deleted first — discarding their contents.
  EOT
  type        = string
  default     = null
}

variable "min_zones" {
  description = <<-EOT
    Number of availability zones the node pool can actually launch in, used to adjust the hard zone
    spread on Thanos Receive and Loki's ingesters. Null leaves the chart's defaults alone, which
    assume two or more zones and is correct for every managed cloud default.

    Set this when that assumption does not hold, because the chart's constraints fail closed rather
    than degrading:

      * `0` — no node carries a `topology.kubernetes.io/zone` label. Common on `kind` and on many
        on-premises distributions. The hard constraints are dropped; the soft host spread stays.
      * `1` — a single zone. `minDomains` becomes 1, which is satisfiable now and turns into real
        protection the day a second zone appears.
      * `2` or more — `minDomains` is set to the real count, which is stricter than the chart's
        floor of 2 whenever you have more zones than that.

    Leaving this null on a cluster with fewer than two zones leaves those pods **Pending forever**
    rather than unbalanced: below `minDomains` Kubernetes treats the global minimum as 0, so one
    zone holding every replica computes a skew equal to the replica count. One zone is exactly as
    broken as none.
  EOT
  type        = number
  default     = null

  validation {
    condition     = var.min_zones == null || var.min_zones >= 0
    error_message = "min_zones must be null or a non-negative number."
  }
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

    Each element is one YAML document. `.tfvars` is HCL, so an indented heredoc is the readable way
    to write one — `<<-YAML` strips the common leading indent, and `$${...}` escapes a literal `$`
    ahead of a brace. The marker is arbitrary; `YAML` rather than the usual `EOT` only because this
    description is itself a heredoc:

    ```hcl
    additional_values = [
      <<-YAML
        grafana:
          ingress:
            enabled: true
            hosts: ["grafana.example.com"]
      YAML
    ]
    ```
  EOT
  type        = list(string)
  default     = []
  nullable    = false

  # Caught here rather than at `helm install`, which reports a parse failure
  # against the merged result and names neither the document nor the caller. The
  # module already parses these in config_hash.tf, where a malformed one falls
  # back to hashing the raw string — so without this check a mis-indented
  # document is accepted all the way to the cluster.
  validation {
    condition = alltrue([for doc in var.additional_values : can(yamldecode(doc))])
    error_message = format(
      "additional_values must contain valid YAML. Could not parse element(s): %s.",
      join(", ", [
        for i, doc in var.additional_values : tostring(i) if !can(yamldecode(doc))
      ]),
    )
  }
}

# ==============================================================================
# Certificates
# ==============================================================================
# The variable names and types mirror `materialize-instance` in
# materialize-terraform-self-managed deliberately: an operator who has configured
# certificates for their Materialize instance should not have to learn a second
# vocabulary to configure them for the monitoring stack, and the two are usually
# pointed at the same issuers by the same wrapper.
#
# `object({ name, kind })` with no `group` matches that module. cert-manager's
# `issuerRef.group` is always `cert-manager.io` for `Issuer` / `ClusterIssuer`,
# and an external issuer is still referenced through one of those kinds — so the
# field buys nothing and would be one more thing to keep in step. The chart
# accepts it; this module does not surface it.

variable "certificates_enabled" {
  description = <<-EOT
    Render cert-manager `Certificate` resources for in-cluster TLS.

    **Requires cert-manager to already be installed**, with its CRDs present.
    This module does not install it — the same shared-responsibility split as
    buckets and workload identity — so with the flag on and cert-manager absent
    the apply fails on an unknown `cert-manager.io/v1` kind.

    Off by default rather than on. The design calls for the Terraform path to be
    secure by default, and that becomes safe once a wrapper that installs
    cert-manager owns the default; flipping it here today would break every
    existing consumer's next apply.

    Issuing certificates does not turn TLS on anywhere — `var.internal_tls` is
    what moves the hops off plaintext, and it needs this. The two are separate
    because a component only leaves plaintext once its renewal behaviour is
    proven, so the material exists before every hop is ready to use it.
  EOT
  type        = bool
  default     = false
  nullable    = false
}

variable "internal_tls" {
  description = <<-EOT
    How far the in-cluster hops move off plaintext. Requires
    `certificates_enabled`, which is what issues the material these settings
    point at.

    The stages are the chart's own `profiles/mtls*.values.yaml`, composed in
    order, and they exist because Kubernetes does not order a server's rollout
    against its clients'. On a **fresh install** there is nothing to strand, so
    go straight to `authenticate`. On a **running stack** step through
    `present` first, or on any hop where the server pod happens to roll before
    the writers, ingestion stops until they catch up — which on a busy pipeline
    is data loss rather than latency.

    | Value | Profiles | State |
    |---|---|---|
    | `off` | none | plaintext everywhere |
    | `encrypt` | `mtls` | servers serve TLS, clients verify them; no client certificates anywhere |
    | `present` | `+ mtls-phase2` | clients also present a certificate; servers still serve anyone. **A way-station, not a destination** — nothing is rejected on the Thanos hop, and only a wrong-CA certificate is on the Loki one |
    | `authenticate` | `+ mtls-phase3` | servers require a client certificate from their CA |

    `authenticate` is authentication, not authorization: none of these components
    can express "this identity may write and that one may not", so the size of
    the trust domain is the security property. That is the argument for leaving
    `issuer_ref` null and letting the chart bootstrap a root scoped to this
    release.

    Two hops stop short of `authenticate` by nature rather than by choice, and
    the chart refuses to configure them otherwise: Loki's HTTP port is probed by
    the kubelet, and a `httpGet` probe has no field for a client certificate, so
    that hop's terminal state is `present`. Grafana's datasource verifies the
    backend but presents nothing.
  EOT
  type        = string
  default     = "off"
  nullable    = false

  validation {
    condition     = contains(["off", "encrypt", "present", "authenticate"], var.internal_tls)
    error_message = "internal_tls must be one of: off, encrypt, present, authenticate."
  }

  validation {
    # Caught here rather than at render because the failure is otherwise a
    # crash loop: the profiles mount `/etc/mzmon/tls` from Secrets that only
    # `certificates_enabled` creates, and Loki exits at startup when the cert
    # file it was told to serve is not there.
    condition     = var.internal_tls == "off" || var.certificates_enabled
    error_message = "internal_tls is set but certificates_enabled is false, so nothing issues the material the TLS settings point at. Turn certificates on, or supply the Secrets yourself through additional_values and leave this off."
  }
}

variable "issuer_ref" {
  description = <<-EOT
    Default cert-manager (Cluster)Issuer used for the monitoring stack's TLS
    certificates. Used for both the external (browser-facing) certificate and
    the internal ones unless overridden by `var.internal_issuer_ref`.

    Leave null with `certificates_enabled` on and the chart bootstraps a
    self-signed root of its own, scoped to this release — which is the
    recommended shape rather than a fallback. None of the receiving components
    here implement per-client authorization, so "signed by the CA we trust" is
    the entire authorization decision; an issuer shared with every workload in
    the cluster reduces that to "has any certificate".
  EOT
  type = object({
    name = string
    kind = string
    # Defaulted rather than required, because every in-tree issuer is
    # `cert-manager.io` and making callers restate it would be noise. It has to
    # exist, though: an external issuer — AWS Private CA
    # (`awspca.cert-manager.io`), Google CAS (`cas-issuer.jetstack.io`) — lives
    # in its own API group, and cert-manager resolves `issuerRef` by group as
    # well as kind. Hardcoding the default group made those unreachable through
    # Terraform even though the chart has always taken the field.
    group = optional(string, "cert-manager.io")
  })
  default = null
}

variable "internal_issuer_ref" {
  description = <<-EOT
    Optional override for the issuer signing cluster-internal certificates
    (those carrying `*.svc.<cluster domain>` SANs).

    Required when `var.issuer_ref` points at a public ACME issuer such as Let's
    Encrypt, since a public CA cannot sign in-cluster names. When null,
    `var.issuer_ref` is used for both.

    Under the chart's `split-namespace` layout this must be a `ClusterIssuer`: a
    namespaced `Issuer` signs only for its own namespace, and the components are
    spread across several. The chart refuses that combination at render time.
  EOT
  type = object({
    name = string
    kind = string
    # Defaulted rather than required, because every in-tree issuer is
    # `cert-manager.io` and making callers restate it would be noise. It has to
    # exist, though: an external issuer — AWS Private CA
    # (`awspca.cert-manager.io`), Google CAS (`cas-issuer.jetstack.io`) — lives
    # in its own API group, and cert-manager resolves `issuerRef` by group as
    # well as kind. Hardcoding the default group made those unreachable through
    # Terraform even though the chart has always taken the field.
    group = optional(string, "cert-manager.io")
  })
  default = null
}

variable "grafana_external_dns_names" {
  description = <<-EOT
    Public DNS names to put on a browser-facing certificate for Grafana, issued
    from `var.issuer_ref`.

    Only needed behind an **L4** load balancer, which passes TCP through and
    leaves TLS to terminate at the pod, so the material has to exist in the
    cluster. An L7 load balancer terminating with a cloud-managed certificate
    (ACM, Google Certificate Manager, Azure Key Vault) attaches it by ARN or
    resource ID and the key never enters the cluster — for that shape leave this
    empty and pass the annotation through `grafana.service.annotations` in
    `additional_values`.

    Setting this with no `var.issuer_ref` issues nothing, and the render says so.
  EOT
  type        = list(string)
  default     = []
  nullable    = false
}

variable "certificate_duration" {
  description = <<-EOT
    Lifetime of each issued certificate, as a Go duration (e.g. `2160h`). Null
    keeps the chart's default of 90 days.

    Keep `certificate_renew_before` well under this — a third or less.
    cert-manager renews at `duration - renewBefore`, so a value close to
    `duration` renews continuously; on a small cluster that has been observed to
    livelock the controller, after which it stops renewing and reports
    certificates as healthy while they expire.
  EOT
  type        = string
  default     = null
}

variable "certificate_renew_before" {
  description = "How long before expiry cert-manager renews, as a Go duration (e.g. `720h`). Null keeps the chart's default of 30 days. See the warning on `certificate_duration`."
  type        = string
  default     = null
}
