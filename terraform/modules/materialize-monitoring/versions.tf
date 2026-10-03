terraform {
  # `optional()` in object type constraints requires 1.3.
  required_version = ">= 1.3.0"

  required_providers {
    helm = {
      source = "hashicorp/helm"
      # Matches the constraint used across materialize-terraform-self-managed,
      # so a wrapper module there can compose with this one without a conflict.
      version = "< 3.4.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = ">= 2.10.0, < 2.39.0"
    }
    random = {
      source  = "hashicorp/random"
      version = ">= 3.0.0, < 3.10.0"
    }
  }
}
