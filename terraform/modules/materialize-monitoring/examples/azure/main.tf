# Azure example: what the per-cloud wrapper module passes in.
#
# Same role as the `aws` and `gcp` examples — a plan-and-render target, not a
# deployable root. Azure is the third shape the module has to express, and it is
# the one that needed new fields: both Loki and Thanos name the storage account
# separately from the container, and neither can derive it.
#
# It also exercises the Workload Identity labelling, which reaches Thanos through
# `global.commonLabels` rather than a `podLabels` the chart does not have. See
# azure.tf.

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
  sizing    = "small"

  materialize_instance_namespace = "materialize-environment"
  materialize_operator_namespace = "materialize"

  object_storage = {
    cloud = "azure"
    # Containers, not buckets — one storage account holds both, and role
    # assignments scope per container so the two backends stay isolated.
    loki_bucket   = "mzmon-loki"
    thanos_bucket = "mzmon-thanos"

    azure_storage_account = "examplemzmonstg"

    # One identity per backend, each scoped to its own container. The webhook
    # reads the client ID from the annotation; nothing else is needed.
    loki_service_account_annotations = {
      "azure.workload.identity/client-id" = "00000000-0000-0000-0000-000000000001"
    }
    thanos_service_account_annotations = {
      "azure.workload.identity/client-id" = "00000000-0000-0000-0000-000000000002"
    }
  }

  node_selector = { workload = "generic" }
}

output "grafana_url" {
  value = module.monitoring.grafana_url
}

output "workload_identity_subjects" {
  value = module.monitoring.workload_identity_subjects
}
