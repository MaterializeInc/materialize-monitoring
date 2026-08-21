# What a tier-2 root feeds into the monitoring module. Shaped to line up with the
# module's `object_storage` object so the composition is a copy, not a mapping.

output "namespace" {
  description = "Namespace the substrate runs in."
  value       = local.namespace
}

output "s3_endpoint" {
  description = "S3 endpoint for the object store. Goes to `object_storage.endpoint`."
  value       = local.s3_endpoint
}

output "s3_credentials_secret_name" {
  description = "Secret holding `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`, in the substrate namespace."
  value       = kubernetes_secret.s3_credentials.metadata[0].name
}

output "s3_access_key_id" {
  description = "Access key for the object store. Prefer the Secret; this is here for the components that only take an inline value."
  value       = random_password.s3_access_key.result
  sensitive   = true
}

output "s3_secret_access_key" {
  description = "Secret key for the object store."
  value       = random_password.s3_secret_key.result
  sensitive   = true
}

output "loki_bucket" {
  description = "Bucket for Loki. Goes to `object_storage.loki_bucket`."
  value       = var.loki_bucket
}

output "thanos_bucket" {
  description = "Bucket for Thanos. Goes to `object_storage.thanos_bucket`."
  value       = var.thanos_bucket
}

# CNPG generates the application credentials and publishes them as
# `<cluster>-app`, keyed `username`, `password`, `dbname`, `host`, `port`, and a
# ready-made `uri`. Passed by name so the password never transits an output.
output "postgres_secret_name" {
  description = "CNPG-generated Secret with the application role's credentials. Keys: `username`, `password`, `dbname`, `host`, `port`, `uri`, `jdbc-uri`."
  value       = "${local.postgres_cluster_name}-app"
}

output "postgres_host" {
  description = "Read-write Service for the Postgres cluster. Also available as the `host` key of the Secret above."
  value       = "${local.postgres_cluster_name}-rw.${local.namespace}.svc.cluster.local"
}

# The gap tier 2 cannot close, stated as an output so a caller cannot miss it:
# there is no workload identity here, and the module's `*_service_account_annotations`
# have nothing to point at.
output "workload_identity_available" {
  description = "Always false. rustfs takes static credentials and kind has no OIDC issuer an IAM provider trusts, so IRSA and Workload Identity are only covered at tier 3."
  value       = false
}

output "cert_manager_available" {
  description = "Whether cert-manager is installed, and therefore whether the monitoring module can be asked to render Certificates."
  value       = var.install_cert_manager
}
