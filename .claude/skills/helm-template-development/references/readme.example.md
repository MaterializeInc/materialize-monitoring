# An Example README for a Helm Chart

This is an example README for a helm chart.

This needs to define a Parameters section so that `@bitnami/readme-generator-for-helm`
can generate the README.md for this chart based on parameters in `values.yaml`.

## Parameters Section

### Global Configuration

| Name                      | Description                                          | Value |
| ------------------------- | ---------------------------------------------------- | ----- |
| `global.imageRegistry`    | An override for all image registries                 | `""`  |
| `global.imagePullSecrets` | An array of image pull secrets to use                | `[]`  |
| `fullnameOverride`        | Override for the full name of resources              | `""`  |
| `commonLabels`            | Common labels to apply to all resources              | `{}`  |
| `commonAnnotations`       | Common annotations to apply to all resources         | `{}`  |
| `extraDeploy`             | An array of arbitrary Kubernetes resources to deploy | `[]`  |

### Example Component

| Name                                                          | Description                                                                             | Value                        |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ---------------------------- |
| `myComponent.enabled`                                         | Whether to enable this component                                                        | `true`                       |
| `myComponent.replicaCount`                                    | Number of replicas for this component. This is ignored if a auto-scaling is configured. | `1`                          |
| `myComponent.image.repository`                                | Container image repository for this component                                           | `my-image-repo/my-component` |
| `myComponent.image.tag`                                       | Container image tag for this component. Defaults to AppVersion unless digest is set.    | `""`                         |
| `myComponent.image.digest`                                    | Container image digest for this component. This takes precedence over tag.              | `""`                         |
| `myComponent.image.pullPolicy`                                | Container image pull policy for this component                                          | `IfNotPresent`               |
| `myComponent.startupProbe.enabled`                            | Whether to enable a startup probe for this component                                    | `true`                       |
| `myComponent.startupProbe.failureThreshold`                   | Failure threshold for the probe                                                         | `10`                         |
| `myComponent.startupProbe.periodSeconds`                      | Wait time between probes                                                                | `10`                         |
| `myComponent.livenessProbe.enabled`                           | Whether to enable a liveness probe for this component                                   | `true`                       |
| `myComponent.livenessProbe.failureThreshold`                  | Failure threshold for the probe                                                         | `10`                         |
| `myComponent.livenessProbe.periodSeconds`                     | Wait time between probes                                                                | `10`                         |
| `myComponent.readinessProbe.enabled`                          | Whether to enable a readiness probe for this component                                  | `true`                       |
| `myComponent.readinessProbe.failureThreshold`                 | Failure threshold for the probe                                                         | `10`                         |
| `myComponent.readinessProbe.periodSeconds`                    | Wait time between probes                                                                | `10`                         |
| `myComponent.resources.limits`                                | Resource limits for this component                                                      | `{}`                         |
| `myComponent.resources.requests`                              | Resource requests for this component                                                    | `{}`                         |
| `myComponent.podSecurityContext.enabled`                      | Whether to enable a pod security context for this component                             | `true`                       |
| `myComponent.podSecurityContext.fsGroupChangePolicy`          | Whether to change fsGroup                                                               | `OnRootMismatch`             |
| `myComponent.podSecurityContext.fsGroup`                      | fsGroup to set on the pod.                                                              | `1234`                       |
| `myComponent.containerSecurityContext.enabled`                | Whether to enable a container security context for this component                       | `true`                       |
| `myComponent.containerSecurityContext.privileged`             | Whether to run the container in privileged mode                                         | `false`                      |
| `myComponent.containerSecurityContext.runAsUser`              | User ID to run the container as                                                         | `1234`                       |
| `myComponent.containerSecurityContext.runAsGroup`             | Group ID to run the container as                                                        | `1234`                       |
| `myComponent.containerSecurityContext.readOnlyRootFilesystem` | Whether to set the root filesystem as read-only                                         | `true`                       |
