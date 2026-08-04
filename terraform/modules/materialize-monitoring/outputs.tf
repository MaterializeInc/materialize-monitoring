output "namespace" {
  description = "Namespace the monitoring stack is installed into."
  value       = local.namespace
}

output "release_name" {
  description = "Name of the materialize-monitoring Helm release."
  value       = helm_release.monitoring.name
}

output "chart_version" {
  description = "Chart version this release is pinned to."
  value       = helm_release.monitoring.version
}

output "grafana_url" {
  description = "In-cluster URL for Grafana. Grafana is ClusterIP-only today, so reaching it from outside the cluster needs a port-forward."
  value       = "http://grafana.${local.namespace}.svc.cluster.local"
}

output "grafana_admin_user" {
  description = "Grafana admin username."
  value       = var.grafana_admin_user
}

output "grafana_admin_password" {
  description = "Grafana admin password."
  value       = local.grafana_admin_password
  sensitive   = true
}

output "grafana_admin_secret_name" {
  description = "Name of the Secret holding the Grafana admin credentials."
  value       = kubernetes_secret.grafana_admin.metadata[0].name
}

output "metrics_url" {
  description = "Thanos Query endpoint. Prometheus-API-compatible, so consumers of a Prometheus URL keep working against it."
  value       = "http://thanos-query.${local.namespace}.svc.cluster.local:9090"
}

output "logs_url" {
  description = "Loki read endpoint (query frontend). Reads carry a tenant header; see the chart's datasource configuration."
  value       = "http://loki-query-frontend.${local.namespace}.svc.cluster.local:3100"
}

output "remote_write_url" {
  description = "Thanos Receive remote-write endpoint, for writers outside this stack."
  value       = "http://thanos-receive.${local.namespace}.svc.cluster.local:10908/api/v1/receive"
}

# The value a cloud wrapper has to match in a trust policy. Emitted rather than
# left to be derived, so it is copy-pasteable and visibly changes if it changes.
output "workload_identity_subjects" {
  description = "`system:serviceaccount:<namespace>:<sa>` subjects for the components that bind to cloud object storage. Use these when building IRSA / Workload Identity trust policies."
  value = {
    loki          = "system:serviceaccount:${local.namespace}:loki"
    thanos        = "system:serviceaccount:${local.namespace}:thanos-thanos"
    alloy_gateway = "system:serviceaccount:${local.namespace}:alloy-gateway"
  }
}

output "service_account_names" {
  description = "ServiceAccount names the chart renders for storage-bound components."
  value = {
    loki          = "loki"
    thanos        = "thanos-thanos"
    alloy_gateway = "alloy-gateway"
  }
}
