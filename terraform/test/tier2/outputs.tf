output "namespace" {
  description = "Namespace the monitoring stack is installed in. This is what `make e2e-verify` targets."
  value       = var.namespace
}

output "grafana_url" {
  description = "In-cluster Grafana URL."
  value       = module.monitoring.grafana_url
}

output "metrics_url" {
  description = "In-cluster Thanos Query URL. Present only at this tier — tier 1 has no Thanos."
  value       = module.monitoring.metrics_url
}

output "s3_endpoint" {
  description = "The rustfs endpoint both backends were pointed at, echoed so a failing run does not need the substrate's state to interpret."
  value       = local.substrate.s3_endpoint
}
