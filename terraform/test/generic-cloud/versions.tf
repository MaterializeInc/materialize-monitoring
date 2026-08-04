terraform {
  required_version = ">= 1.3.0"

  required_providers {
    helm = {
      source = "hashicorp/helm"
      # Matches the common module, so a root that composes both resolves a single
      # provider version.
      version = ">= 2.5.0, < 2.18.0"
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
