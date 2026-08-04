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
    # The gateway needs this for Google Cloud Monitoring below, not for storage.
    gateway_service_account_annotations = {
      "iam.gke.io/gcp-service-account" = "example-mzmon-gateway@example-project.iam.gserviceaccount.com"
    }
  }

  google_cloud_metrics = {
    min_importance = "recommended"
  }

  node_selector = { workload = "generic" }

  # A sentinel, not a realistic class name: the render check requires every
  # volumeClaimTemplate to carry it, which is what proves storage_class.tf's
  # fan-out is complete. GCP carries it because C4/N4 take only Hyperdisk, so the
  # default class does not work there at all.
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
