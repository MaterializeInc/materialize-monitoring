# AWS example: what the per-cloud wrapper module passes in.
#
# This is not a deployable root on its own — the buckets and IAM roles it names
# are placeholders that a wrapper in materialize-terraform-self-managed would
# create. It exists so the module has a plan target: `make terraform-render`
# plans it, extracts the composed values, and renders the chart against them,
# which catches every value-path typo without needing a cluster.
#
# There is a `gcp` example beside this one, and both are rendered. That is not
# redundancy: the chart's defaults are S3-shaped, so an AWS-only example agrees
# with every default it fails to set, and cannot catch a missing backend key.
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
    # The regional host, not the global `s3.amazonaws.com`. Both work, but the
    # global one is also the chart's default for Loki — naming it here would
    # make the rendered config look correct whether or not the module actually
    # wrote an endpoint, which is the failure this example is here to catch.
    # Omitting it entirely is supported too: the module derives this same host
    # from `region`.
    endpoint = "s3.us-east-1.amazonaws.com"

    loki_service_account_annotations = {
      "eks.amazonaws.com/role-arn" = "arn:aws:iam::012345678901:role/ExampleLoki"
    }
    thanos_service_account_annotations = {
      "eks.amazonaws.com/role-arn" = "arn:aws:iam::012345678901:role/ExampleThanos"
    }
  }

  # Both SaaS metric destinations, set here so the render check has something to
  # assert against. Each fans out *in addition to* Thanos, and each proves a
  # different half of the credential wiring: Datadog's key reaches an env var the
  # chart names itself, while the OTLP header's variable name is derived by the
  # module and has to match on both sides — the values say `valueEnv`, the Secret
  # supplies it, and a mismatch is silent.
  #
  # The keys are placeholders. They are never sent anywhere: the example is only
  # ever planned, never applied.
  datadog_metrics = {
    site           = "datadoghq.com"
    min_importance = "essential"
  }
  datadog_api_key = "example-not-a-real-datadog-key"

  otlp_metrics = {
    url            = "api.honeycomb.io"
    protocol       = "grpc"
    min_importance = "recommended"

    # Non-secret, so it renders inline rather than through the Secret.
    auth_headers = {
      "x-honeycomb-dataset" = "mzmon"
    }
  }

  otlp_auth_header_secrets = {
    "x-honeycomb-team" = "example-not-a-real-honeycomb-key"
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
