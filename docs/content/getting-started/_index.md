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

{{% hint warning %}}
TODO: integrate `materialize-monitoring` into `materialize-terraform-self-managed`.
{{% /hint %}}

_`materialize-monitoring` is supported in `materialize-terraform-self-managed` as of version: TODO_

See [Terraform Installation](./terraform) for users who are using `materialize-terraform-self-managed` to set up their Materialize cluster and want to use Terraform to set up their monitoring infrastructure as well.

## `materialize-monitoring` Helm Chart

If you have a cluster with `materialize-operator` installed,
you can install the `materialize-monitoring` Helm chart in the same cluster to set up your monitoring infrastructure.

The helm chart provides a greater level of customization than the terraform module.

See [Helm Installation](./helm) for instructions on how to install `materialize-monitoring` via Helm.

## Getting Help

Please [reach out for Support](https://materialize.com/docs/support/).
