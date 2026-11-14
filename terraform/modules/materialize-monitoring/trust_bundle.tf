# The trust bundle: CAs the stack trusts for outbound TLS beyond its images' own.
#
# The chart mounts a ConfigMap named `mzmon-trust-bundle`, optionally, into Loki,
# Thanos, both Alloy roles and Alertmanager, and adds the mount to the
# directories Go loads its roots from. It never creates the ConfigMap, because the
# usual source is a trust-manager `Bundle` that writes it. This creates it from
# `trust_bundle_ca_pem` instead, for a caller that holds the CA as a string.
#
# Two things make the order and the restart load-bearing. A pod that starts
# before the ConfigMap exists mounts an empty directory. And a Go process reads
# its roots once, at its first TLS connection, so the kubelet refreshing the
# mounted file later changes nothing in a running pod. Hence the release waits
# on the ConfigMap, and the hash below rolls every pod that reads it.

locals {
  # See `trust_bundle_enabled` in variables.tf: inferring from the PEM fails the
  # plan when the PEM is computed in the same apply.
  trust_bundle_enabled = var.trust_bundle_enabled != null ? var.trust_bundle_enabled : var.trust_bundle_ca_pem != null

  # A volume reads a ConfigMap from its own pod's namespace only, so the bundle
  # goes wherever a consumer runs. Keyed by input variables rather than
  # `local.namespace`, so `for_each` stays known at plan.
  trust_bundle_namespaces = !local.trust_bundle_enabled ? toset([]) : toset(
    var.trust_bundle_namespaces != null
    ? var.trust_bundle_namespaces
    : [var.namespace, coalesce(var.alertmanager_namespace, var.namespace)]
  )
}

resource "kubernetes_config_map" "trust_bundle" {
  for_each = local.trust_bundle_namespaces

  metadata {
    # Fixed: the chart's volumes name it literally, because subchart values
    # cannot be templated from the release.
    name      = "mzmon-trust-bundle"
    namespace = each.key
  }

  data = {
    "ca.crt" = var.trust_bundle_ca_pem
  }

  depends_on = [kubernetes_namespace.monitoring]

  lifecycle {
    precondition {
      condition     = var.trust_bundle_ca_pem != null
      error_message = "trust_bundle_enabled is true but trust_bundle_ca_pem is null, so the trust bundle would hold no CAs."
    }
  }
}

locals {
  # Truncated, like `config_hash`: a change detector, not an integrity check.
  trust_bundle_hash = var.trust_bundle_ca_pem == null ? null : substr(sha256(var.trust_bundle_ca_pem), 0, 16)

  trust_bundle_annotations = { "mzmon.materialize.cloud/trust-bundle-hash" = local.trust_bundle_hash }

  # One annotation per consumer, each on the map that subchart merges into every
  # pod it renders. Maps deep-merge across values documents, so these sit beside
  # `config_hash_document`'s annotation on the Alloy roles rather than replacing
  # it. With no bundle there is nothing to roll on, and no document is added.
  trust_bundle_document = !local.trust_bundle_enabled ? [] : [yamlencode({
    loki = {
      defaults = { podAnnotations = local.trust_bundle_annotations }
    }
    thanos = {
      global = { podAnnotations = local.trust_bundle_annotations }
    }
    alertmanager = {
      podAnnotations = local.trust_bundle_annotations
    }
    "alloy-agent" = {
      controller = { podAnnotations = local.trust_bundle_annotations }
    }
    "alloy-gateway" = {
      controller = { podAnnotations = local.trust_bundle_annotations }
    }
  })]
}
