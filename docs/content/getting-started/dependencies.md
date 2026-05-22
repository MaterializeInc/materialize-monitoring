---
title: "Dependencies"
weight: 20
---

# materialize-monitoring Dependencies

## Installing CRDs

The following CRDs are required for `materialize-monitoring`
to function properly.

A second `materialize-monitoring-crds` Helm chart is provided to install these CRDs separately from the main `materialize-monitoring` chart, which is recommended to manage the lifecycle of these CRDs separately from the main chart.

## Configuring Storage

Before you start, you need to be able to store your metrics and logs
somewhere.

#### >> I am running in a cloud environment with a managed Kubernetes service (EKS, GKE, AKS, etc.)

TODO: setup bucket with IRSA

#### >> I am running in an on-premises Kubernetes cluster with access to cloud object storage (S3, GCS, Azure Blob Storage, etc.)

TODO: setup bucket with service account credentials
