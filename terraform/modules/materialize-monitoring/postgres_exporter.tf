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

  # The targets with every field but the credentials unmarked. A target object
  # arrives marked sensitive as a whole when any field came from a sensitive
  # value, which a wrapper passing a database module's username does (the AWS
  # one marks it). Left marked, the names would mark the Helm values, the
  # consensus declarations and the preconditions' messages, none of which hold
  # anything secret. Only `user` and the passwords belong to the credentials, and
  # those stay marked, in the Secret.
  #
  # Each field is unmarked, and then the list itself: a `for` over a marked list
  # returns a marked list, whatever its elements are.
  postgres_exporter_public_targets_marked = [
    for t in local.postgres_exporter_targets : {
      name              = try(nonsensitive(t.name), t.name)
      host              = try(nonsensitive(t.host), t.host)
      port              = try(nonsensitive(t.port), t.port)
      database          = try(nonsensitive(t.database), t.database)
      ssl_mode          = try(nonsensitive(t.ssl_mode), t.ssl_mode)
      metadata_database = try(nonsensitive(t.metadata_database), t.metadata_database)
    }
  ]
  postgres_exporter_public_targets = try(
    nonsensitive(local.postgres_exporter_public_targets_marked),
    local.postgres_exporter_public_targets_marked,
  )

  # The caller's target names, unmarked for the same reason.
  postgres_exporter_caller_names_marked = [
    for t in var.postgres_exporter_targets : try(nonsensitive(t.name), t.name)
  ]
  postgres_exporter_caller_names = try(
    nonsensitive(local.postgres_exporter_caller_names_marked),
    local.postgres_exporter_caller_names_marked,
  )

  # A target with no password is not an error the exporter reports clearly: it
  # answers that target's probe with HTTP 400, so `up` reads 0 and nothing
  # names the cause. Grafana's is checked by the gate above.
  postgres_exporter_password_names = try(
    nonsensitive(keys(var.postgres_exporter_passwords)),
    keys(var.postgres_exporter_passwords),
  )
  postgres_exporter_unpassworded = [
    for name in local.postgres_exporter_caller_names : name
    if !contains(local.postgres_exporter_password_names, name)
  ]

  postgres_exporter_passwords = merge(
    local.postgres_exporter_watches_grafana ? { grafana = var.grafana_database_password } : {},
    var.postgres_exporter_passwords,
  )

  postgres_exporter_config = yamlencode({
    auth_modules = {
      for i, t in local.postgres_exporter_public_targets : t.name => {
        type = "userpass"
        userpass = {
          username = local.postgres_exporter_targets[i].user
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
        for t in local.postgres_exporter_public_targets : {
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
    [for t in local.postgres_exporter_public_targets : { flavor = "postgres", resourceId = t.name } if t.metadata_database],
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
      condition     = !local.postgres_exporter_watches_grafana || !contains([for name in local.postgres_exporter_caller_names : lower(name)], "grafana")
      error_message = "postgres_exporter_targets names a target \"grafana\", which this module already adds for Grafana's own database. Rename it."
    }
  }

  depends_on = [kubernetes_namespace.monitoring]
}
