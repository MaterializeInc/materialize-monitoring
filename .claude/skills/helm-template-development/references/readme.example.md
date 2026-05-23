# An Example README for a Helm Chart

This is an example of the README.md that [helm-docs](https://github.com/norwoodj/helm-docs)
generates from a chart's `values.yaml` plus a `README.md.gotmpl` template.

The README is fully generated — do not edit it by hand. Edit the
template (`README.md.gotmpl`) for static prose and badges, or
`values.yaml` for the parameter table content, then re-run
`make charts/<chart>/README.md`.

Sections in the Values table come from per-key `# @section -- <Name>`
comments in `values.yaml` (see [values.example.yaml](values.example.yaml)).

## Example template

A minimal `README.md.gotmpl`:

```gotemplate
{{ template "chart.header" . }}
{{ template "chart.description" . }}

{{ template "chart.versionBadge" . }}{{ template "chart.typeBadge" . }}{{ template "chart.appVersionBadge" . }}

## TL;DR

```bash
helm install my-release oci://example.com/my-chart
```

{{ template "chart.requirementsSection" . }}

{{ template "chart.valuesSection" . }}
```

## Example output (Values section)

### Global Configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| global.imageRegistry | string | `""` | An override for all image registries |
| global.imagePullSecrets | list | `[]` | An array of image pull secrets to use |
| fullnameOverride | string | `""` | Override for the full name of resources |
| commonLabels | object | `{}` | Common labels to apply to all resources |
| commonAnnotations | object | `{}` | Common annotations to apply to all resources |
| extraDeploy | list | `[]` | An array of arbitrary Kubernetes resources to deploy |

### Example Component

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| myComponent.enabled | bool | `true` | Whether to enable this component |
| myComponent.replicaCount | int | `1` | Number of replicas for this component. Ignored when auto-scaling is configured. |
| myComponent.image.repository | string | `"my-image-repo/my-component"` | Container image repository for this component |
| myComponent.image.tag | string | `""` | Container image tag for this component. Defaults to AppVersion unless digest is set. |
| myComponent.image.digest | string | `""` | Container image digest for this component. Takes precedence over tag. |
| myComponent.image.pullPolicy | string | `"IfNotPresent"` | Container image pull policy for this component |
| myComponent.startupProbe.enabled | bool | `true` | Whether to enable a startup probe for this component |
| myComponent.startupProbe.failureThreshold | int | `10` | Failure threshold for the probe |
| myComponent.startupProbe.periodSeconds | int | `10` | Wait time between probes |
| myComponent.livenessProbe.enabled | bool | `true` | Whether to enable a liveness probe for this component |
| myComponent.livenessProbe.failureThreshold | int | `10` | Failure threshold for the probe |
| myComponent.livenessProbe.periodSeconds | int | `10` | Wait time between probes |
| myComponent.readinessProbe.enabled | bool | `true` | Whether to enable a readiness probe for this component |
| myComponent.readinessProbe.failureThreshold | int | `10` | Failure threshold for the probe |
| myComponent.readinessProbe.periodSeconds | int | `10` | Wait time between probes |
| myComponent.resources.limits | object | see `values.yaml` | Resource limits for this component |
| myComponent.resources.requests | object | see `values.yaml` | Resource requests for this component |
| myComponent.podSecurityContext.enabled | bool | `true` | Whether to enable a pod security context for this component |
| myComponent.podSecurityContext.fsGroupChangePolicy | string | `"OnRootMismatch"` | Whether to change fsGroup |
| myComponent.podSecurityContext.fsGroup | int | `1234` | fsGroup to set on the pod. |
| myComponent.containerSecurityContext.enabled | bool | `true` | Whether to enable a container security context for this component |
| myComponent.containerSecurityContext.privileged | bool | `false` | Whether to run the container in privileged mode |
| myComponent.containerSecurityContext.runAsUser | int | `1234` | User ID to run the container as |
| myComponent.containerSecurityContext.runAsGroup | int | `1234` | Group ID to run the container as |
| myComponent.containerSecurityContext.readOnlyRootFilesystem | bool | `true` | Whether to set the root filesystem as read-only |
