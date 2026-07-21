# Unified Monitoring for Materialize

This customer-facing repository provides common observability (o11y)
infrastructure and configurations for Materialize Deployments, including
Self-Managed and Materialize Cloud.

## WORK IN PROGRESS

**This repository is being actively developed!**

This readme will try to track changes with dates, but expect
some amount of flux as this repository stabilizes.

## Roadmap and Layout

* [Roadmap](docs/content/reference/internal/roadmap.md) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout](docs/content/reference/internal/repo-layout.md) — where things live in the repo.

## Getting Started

This will be automatically (TODO: implement) deployed by our
[recommended terraform module](https://github.com/MaterializeInc/materialize-terraform-self-managed)
for deploying Materialize.

For more bespoke configurations, please see our documentation: TODO LINK.

## About This Repository

The primary Artifacts are:

* (TODO) A published, versioned helm umbrella chart for deploying an observability stack
    alongside Materialize.
* (WIP) Documentation about the intricacies of the observability stack and how to use it.
* (WIP) Dashboards that customers can use to manage their own Deployments and that we
    use for our own internal monitoring of Materialize Cloud.

## Supported Dashboards

### Materialize Overview

Day 2 Operations for Materialize environments.

Supported Materialize Versions:
* v26.0 - v26.24 (new-promsql v2_mz / unstable mz_)

Targets:
* (Alpha) Grafana 13
    * generated: `docs/static/dashboards/grafana/` (TODO: docsite link)
    * grafana-operator template: TODO
    * source (Python SDK): `packages/grafana-dashboards/dashboards/`
* (TODO) Grafana 12
* (TODO) Grafana 10-11
* (TODO) Datadog
* (TODO) Google Cloud Operations

### Materialize Fleet View

Day 2 Operations across many Materialize environments.

TODO

### Materialize Troubleshooting

Guided troubleshooting dashboards for common issues.

TODO

### Materialize Infrastructure

Infrastructure-level monitoring for Materialize dependencies.

TODO

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
