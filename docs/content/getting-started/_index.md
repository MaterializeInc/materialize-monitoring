---
title: "Getting Started"
weight: 10
bookCollapseSection: true
---

# Getting Started with materialize-monitoring

There are a lot of ways to get started with `materialize-monitoring`.
The contents of this chapter describe how to install and configure
`materialize-monitoring` in various environments.

## `materialize-terraform-self-managed` Terraform Module

If you stand up Materialize with the Terraform modules, observability comes up with the cluster — the modules create the buckets and workload identity, then install these charts at a pinned version.

> [!WARNING]
>  **Preview.** The modules are built and validated but not yet in a tagged release of `materialize-terraform-self-managed`. AWS and GCP are wired; Azure still uses the previous Prometheus + Grafana modules.

See [Terraform Installation](./terraform) — including the [tfvars reference](./terraform#tfvars-reference).

## `materialize-monitoring` Helm Chart

If you have a cluster with `materialize-operator` installed,
you can install the `materialize-monitoring` Helm chart in the same cluster to set up your monitoring infrastructure.

The Helm chart is the full-fidelity surface: the Terraform modules are a thin layer over it that adds the cloud resources Helm cannot create, and every chart value stays reachable from Terraform through `additional_values`.

See [Helm Installation](./helm) for instructions on how to install `materialize-monitoring` via Helm.

## Going to Production

Whichever path you take, [Production Best Practices](../operating/production-best-practices/) is the checklist for a real deployment — sizing, retention, replication, disruption budgets, and object-storage durability — with each item tagged by who owns it.

## Getting Help

Please [reach out for Support](https://materialize.com/docs/support/).
