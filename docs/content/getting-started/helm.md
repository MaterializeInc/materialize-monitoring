---
title: "Helm"
weight: 40
---

# Installing via Helm

{{< wip >}}

If you are not using the terraform module to install `materialize-monitoring`, you can use the provided Helm charts to install the monitoring stack in your Kubernetes cluster.

> [!SUCCESS]
>  During early development (June 2026), the helm charts are the only way to install `materialize-monitoring`.

> [!INFO]
>  The terraform module is the recommended way to install `materialize-monitoring`.
>
>  However, the Helm charts allow for access to the full configuration.

## Dependency: Installing CRDs

`materialize-monitoring` relies on several Custom Resource Definitions (CRDs) to function properly.

A second `materialize-monitoring-crds` Helm chart is provided to install these CRDs separately from the main `materialize-monitoring` chart, which is recommended to manage the lifecycle of these CRDs separately from the main chart.

## Dependency: Setting Up Storage

You will likely need to set up storage for your metrics and logs before you can start using `materialize-monitoring`.
The specific steps for setting up storage will depend on your environment and the storage solution you choose.

If you are using both external metric storage and external log storage,
you will not need an object storage bucket.

### Cloud Managed Kubernetes Service (AWS EKS, Google Cloud GKE, Azure AKS, etc.)

TODO: setup bucket with IRSA

### On-Premises Kubernetes Cluster with Access to Cloud Object Storage (S3, GCS, Azure Blob Storage, etc.)

TODO: setup bucket with service account credentials

## Customizing your Helm Installation

The `materialize-monitoring` Helm chart is designed to be highly customizable, so you can easily integrate with your existing observability infrastructure.

Typically, you would want to create a values.yaml file that has your
specific configurations.
You may start fresh or you can copy an example from
the `charts/materialize-monitoring/examples/` directory in this repository.

> [!WARNING]
>  Be aware that when merging examples together that you do not have
>  multiple of the same key on the same level since they do not automatically merge.
>  YAML is whitespace sensitive.

You must specify `-f YOUR_VALUES.yaml` in your `helm install`/`helm upgrade` command to apply these customizations.
These are automatically overlaid on top of the default values of the
chart, so you only need to specify the values that are different from the default.

> [!INFO]
>  This documentation may refer to values in dotted notation (e.g., `component.subcomponent.key=value`)
>  which corresponds roughly to this YAML structure:
>
>  ```yaml
>  component:
>    subcomponent:
>      key: value
>  ```

### Configuring a Profile via Tags

The `materialize-monitoring` Helm chart provides several pre-configured profiles that you can use to quickly set up your monitoring stack with a specific set of components.

TODO: document profiles and tags

### Disabling a Component

If you want further control of the managed components, you can selectively disable components in the `materialize-monitoring` Helm chart by setting the `enabled` field for that component to `false` in your `YOUR_VALUES.yaml` file or via `--set` in your `helm install`/`helm upgrade` command.

## Initial Installation

TODO: helm install commands
