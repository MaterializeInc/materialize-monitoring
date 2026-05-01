# Unified Monitoring for Materialize

This customer-facing repository provides common observability (o11y)
infrastructure and configurations for Materialize Deployments, including
Self-Managed and Materialize Cloud.

## Getting Started

This is automatically deployed by our
[recommended terraform module](https://github.com/MaterializeInc/materialize-terraform-self-managed)
for deploying Materialize.

For more bespoke configurations, please see our documentation: TODO LINK.

## About This Repository

The primary Artifacts are:

* A published, versioned helm umbrella chart for deploying an observability stack
    alongside Materialize.
* Documentation about the intricacies of the observability stack and how to use it.
* Dashboards that customers can use to manage their own Deployments and that we
    use for our own internal monitoring of Materialize Cloud.

## License

Materialize is provided as a self-managed product and a fully managed cloud service with
[credit-based pricing](https://materialize.com/pricing/). Included in the price
are proprietary cloud-native features like horizontal scalability, high
availability, and a web management console.

We're big believers in advancing the frontier of human knowledge. To
that end, the source code of the standalone database engine is publicly
available, in this repository, and [licensed](LICENSE) under the BSL 1.1,
converting to the open-source Apache 2.0 license after 4 years. As stated in the
BSL, use of the standalone database engine on a single node is free forever.
