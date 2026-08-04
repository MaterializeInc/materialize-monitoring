# GCP example: what the per-cloud wrapper module passes in.
#
# Same role as the `aws` example beside it — a plan-and-render target, not a
# deployable root. This one carries the weight, because the chart's storage
# defaults are S3-shaped: every backend key the module has to override is
# already correct on AWS and wrong here. A missing one does not degrade, it
# crash-loops the component that reads it, so this is the example that has to
# stay in the render check.
#
# Uses `large` so the other sizing tier gets rendered too (`medium` is the
# chart's own defaults and applies no profile at all).

terraform {
  required_version = ">= 1.3.0"

  required_providers {
    helm = {
      source  = "hashicorp/helm"
      version = ">= 2.5.0, < 2.18.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = ">= 2.10.0, < 2.39.0"
    }
  }
}

provider "kubernetes" {
  config_path = var.kubeconfig_path
}

provider "helm" {
  kubernetes {
    config_path = var.kubeconfig_path
  }
}

variable "kubeconfig_path" {
  description = "Path to a kubeconfig. Irrelevant for `terraform plan`, which is all this example is used for."
  type        = string
  default     = "~/.kube/config"
}

module "monitoring" {
  source = "../../"

  namespace = "monitoring"
  sizing    = "large"

  materialize_instance_namespace = "materialize-environment"
  materialize_operator_namespace = "materialize"

  # No `region` or `endpoint`: those are S3-only fields, and Thanos rejects them
  # in a GCS objstore config.
  object_storage = {
    cloud         = "gcp"
    loki_bucket   = "example-mzmon-loki"
    thanos_bucket = "example-mzmon-thanos"

    loki_service_account_annotations = {
      "iam.gke.io/gcp-service-account" = "example-mzmon-loki@example-project.iam.gserviceaccount.com"
    }
    thanos_service_account_annotations = {
      "iam.gke.io/gcp-service-account" = "example-mzmon-thanos@example-project.iam.gserviceaccount.com"
    }
  }

  node_selector = { workload = "generic" }

  # A recognizable sentinel rather than a realistic class name. The render check
  # counts `volumeClaimTemplates` in the output and requires every one of them to
  # carry this value, which is what proves the five-key fan-out in
  # storage_class.tf is complete. A sixth PVC-backed workload appearing in a
  # subchart fails that count instead of silently landing on the cluster default.
  #
  # GCP is the right example to carry it: C4 and N4 nodes accept only Hyperdisk,
  # so there the default class does not work at all.
  storage_class = "render-check-storage-class"
}

output "grafana_url" {
  value = module.monitoring.grafana_url
}

output "metrics_url" {
  value = module.monitoring.metrics_url
}

output "workload_identity_subjects" {
  value = module.monitoring.workload_identity_subjects
}
