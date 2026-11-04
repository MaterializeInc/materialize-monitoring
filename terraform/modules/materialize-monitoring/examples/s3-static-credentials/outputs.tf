output "grafana_url" {
  description = "The module's `grafana_url`: Grafana's in-cluster URL."
  value       = module.monitoring.grafana_url
}
