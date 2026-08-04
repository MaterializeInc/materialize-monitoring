# Complete example: what a per-cloud wrapper module passes in.
#
# This is not a deployable root on its own — the buckets and IAM roles it names
# are placeholders that a wrapper in materialize-terraform-self-managed would
# create. It exists so the module has a plan target: CI plans this, extracts the
# composed values, and renders the chart against them, which catches every value
# path typo without needing a cluster.
#
# Note the relative `source`. An absolute path would make Terraform copy the
# module into .terraform/modules/ without the chart directory beside it, and the
# sizing profiles would not resolve.

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
    cloud         = "aws"
    loki_bucket   = "example-mzmon-loki"
    thanos_bucket = "example-mzmon-thanos"
    region        = "us-east-1"
    endpoint      = "s3.amazonaws.com"

    loki_service_account_annotations = {
      "eks.amazonaws.com/role-arn" = "arn:aws:iam::012345678901:role/ExampleLoki"
    }
    thanos_service_account_annotations = {
      "eks.amazonaws.com/role-arn" = "arn:aws:iam::012345678901:role/ExampleThanos"
    }
  }

  node_selector = { workload = "generic" }

  tolerations = [
    {
      key      = "dedicated"
      operator = "Equal"
      value    = "monitoring"
      effect   = "NoSchedule"
    },
  ]
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
