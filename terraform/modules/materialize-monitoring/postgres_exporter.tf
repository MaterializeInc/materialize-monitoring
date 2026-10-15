# The postgres-exporter's targets and credentials, and the metadata databases
# the recording rules read.
#
# The exporter reads every target's credentials from one file,
# `postgres_exporter.yml`, and from nowhere else: in multi-target mode it has no
# indirection to a file or an environment variable per credential. So the whole
# file is the Secret, built here from the caller's passwords, and the chart
# mounts it by the name it defaults to. The values carry the targets and never a
# password.
#
# `externalDependencies.consensus` is composed here, in one document, because it
# is a list: Helm replaces a list wholesale rather than merging it, so two
# documents each writing part of it would leave only the later one's entries.

locals {
  # Grafana's database joins the targets whenever this module configures it with
  # a password it manages. Both are plan-known, which `count` below needs: the
  # host and password themselves may be computed in the same apply.
  postgres_exporter_watches_grafana = var.enable_postgres_exporter && local.grafana_database_password_secret

  postgres_exporter_grafana_target = local.postgres_exporter_watches_grafana ? [{
    name = "grafana"
    host = var.grafana_database_host
    port = var.grafana_database_port
    # Not `grafana_database_name`: per-table statistics come from the database
    # the connection opens, and Grafana's ~90 tables would add ~2,600 series no
    # panel reads. Every instance-wide family is the same from `postgres`.
    database          = "postgres"
    user              = var.grafana_database_user
    ssl_mode          = var.grafana_database_ssl_mode
    metadata_database = false
  }] : []

  postgres_exporter_targets = var.enable_postgres_exporter ? concat(
    local.postgres_exporter_grafana_target,
    var.postgres_exporter_targets,
  ) : []

  # A target with no password is not an error the exporter reports clearly: it
  # answers that target's probe with HTTP 400, so `up` reads 0 and nothing
  # names the cause. Grafana's is checked by the gate above.
  postgres_exporter_unpassworded = [
    for t in var.postgres_exporter_targets : t.name
    if !contains(try(nonsensitive(keys(var.postgres_exporter_passwords)), keys(var.postgres_exporter_passwords)), t.name)
  ]

  postgres_exporter_passwords = merge(
    local.postgres_exporter_watches_grafana ? { grafana = var.grafana_database_password } : {},
    var.postgres_exporter_passwords,
  )

  postgres_exporter_config = yamlencode({
    auth_modules = {
      for t in local.postgres_exporter_targets : t.name => {
        type = "userpass"
        userpass = {
          username = t.user
          password = lookup(local.postgres_exporter_passwords, t.name, "")
        }
        options = { sslmode = t.ssl_mode }
      }
    }
  })

  # Rolls the exporter when a credential changes. It reads the file once at
  # start, and the chart mounts it with `subPath`, which a Secret update never
  # reaches, so nothing else would make it pick up a rotated password. A
  # truncated hash is not the credentials, so the sensitive mark is dropped, as
  # it is for the gateway's in gateway_credentials.tf.
  postgres_exporter_credential_hash = substr(try(
    nonsensitive(sha256(local.postgres_exporter_config)),
    sha256(local.postgres_exporter_config),
  ), 0, 16)

  postgres_exporter_document = var.enable_postgres_exporter ? [yamlencode({
    tags = { "postgres-exporter" = true }
    "postgres-exporter" = {
      targets = [
        for t in local.postgres_exporter_targets : {
          name     = t.name
          host     = t.host
          port     = t.port
          database = t.database
        }
      ]
      annotations = { "checksum/mzmon-credentials" = local.postgres_exporter_credential_hash }
    }
  })] : []

  consensus_entries = concat(
    [for d in var.metadata_databases : { flavor = d.flavor, resourceId = d.resource_id }],
    [for t in local.postgres_exporter_targets : { flavor = "postgres", resourceId = t.name } if t.metadata_database],
  )

  consensus_document = length(local.consensus_entries) == 0 ? [] : [yamlencode({
    externalDependencies = { consensus = local.consensus_entries }
  })]
}

resource "kubernetes_secret" "postgres_exporter" {
  count = var.enable_postgres_exporter ? 1 : 0

  metadata {
    # The chart's `postgres-exporter.config.existingSecret.name` default.
    name      = "postgres-exporter"
    namespace = local.namespace
  }

  data = {
    "postgres_exporter.yml" = local.postgres_exporter_config
  }

  type = "Opaque"

  lifecycle {
    precondition {
      condition     = length(local.postgres_exporter_unpassworded) == 0
      error_message = "postgres_exporter_passwords has no password for: ${join(", ", local.postgres_exporter_unpassworded)}. The exporter refuses a target without one."
    }
    precondition {
      condition     = !local.postgres_exporter_watches_grafana || !contains([for t in var.postgres_exporter_targets : lower(t.name)], "grafana")
      error_message = "postgres_exporter_targets names a target \"grafana\", which this module already adds for Grafana's own database. Rename it."
    }
  }

  depends_on = [kubernetes_namespace.monitoring]
}
