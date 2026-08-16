# S3-compatible storage with static credentials: what a deployment with no
# workload identity passes in.
#
# Not a deployable root — the endpoint and keys are placeholders. It exists as a
# plan target so `make terraform-render` renders the chart against the composed
# values and proves the credential path lands, which is the half `terraform
# validate` cannot see.
#
# This is the shape the tier-2 E2E root uses against rustfs, and the shape an
# on-prem MinIO or Ceph deployment uses in production. It is deliberately a
# separate example from `aws`: that one carries IRSA annotations and no keys, so
# it cannot catch a regression in the static path, and this one carries keys and
# no annotations so it cannot hide behind workload identity.

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

    # Named by URL, scheme included, which is how an S3-compatible store is
    # normally addressed. The module strips the scheme — the objstore client
    # rejects one outright — and reads `http://` as "use plain HTTP", so this one
    # value also selects the transport. No region: the store is not AWS, so there
    # is no regional host to derive and nothing to sign against a region.
    endpoint = "http://objectstore.example.internal:9000"

    # No service-account annotations. That is the point of this example — the
    # deployment has no workload identity to bind to.
  }

  object_storage_access_key_id     = "EXAMPLEACCESSKEYID"
  object_storage_secret_access_key = "EXAMPLEsecretaccesskeyEXAMPLEsecret12345"
}

output "grafana_url" {
  value = module.monitoring.grafana_url
}
