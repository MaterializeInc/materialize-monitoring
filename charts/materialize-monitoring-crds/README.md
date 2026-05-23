# materialize-monitoring-crds

![Version: 0.0.1](https://img.shields.io/badge/Version-0.0.1-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 0.0.0](https://img.shields.io/badge/AppVersion-0.0.0-informational?style=flat-square)

CRDs for Materialize Monitoring components.
This does not install any additional resources other than CRDs.

**Homepage:** <https://github.com/MaterializeInc/materialize-monitoring>

## Maintainers

| Name | Email | Url |
| ---- | ------ | --- |
| Materialize | <support@materialize.com> | <https://materialize.com> |

## Source Code

* <https://github.com/MaterializeInc/materialize-monitoring>

## Requirements

Kubernetes: `>=1.27.0-0`

| Repository | Name | Version |
|------------|------|---------|
| oci://ghcr.io/prometheus-community/charts | prometheus-operator-crds | ^29.0.0 |

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| prometheus-operator-crds.crds.annotations."helm.sh/resource-policy" | string | `"keep"` |  |

