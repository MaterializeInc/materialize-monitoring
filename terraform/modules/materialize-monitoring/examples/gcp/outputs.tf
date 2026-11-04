output "grafana_url" {
  description = "The module's `grafana_url`: Grafana's in-cluster URL."
  value       = module.monitoring.grafana_url
}

output "metrics_url" {
  description = "The module's `metrics_url`: the Thanos Query endpoint."
  value       = module.monitoring.metrics_url
}

output "workload_identity_subjects" {
  description = "The module's `workload_identity_subjects`: the service-account subjects a cloud identity binding trusts."
  value       = module.monitoring.workload_identity_subjects
}
