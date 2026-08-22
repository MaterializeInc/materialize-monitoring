---
title: "materialize-monitoring Documentation"
htmltitle: "Home"
disable_toc: false
disable_h1: true
weight: 1
---
# materialize-monitoring Documentation

`materialize-monitoring` is first-class observability for Materialize deployments — metrics, logs, events, dashboards, and alerts, packaged as a Helm chart and a Terraform module.
It is a one-stop shop for teams who want one, and every piece of it can be turned off for teams who already run their own.

If you install Materialize with the Terraform modules, the stack comes up with the cluster by default from `materialize-terraform-self-managed` v11 onward — set `enable_observability = false` to opt out.

Nothing here is required to run Materialize.
If you are looking for the main Materialize documentation, see [materialize.com/docs](https://materialize.com/docs/).

> [!WARNING]
>  **Pre-1.0.** Interfaces are still moving, and breaking changes can ride a minor release until 1.0 is stamped.
>  See the [Roadmap]({{< relref "reference/internal/roadmap.md" >}}) for what is built and what is coming, and the [Changelog]({{< relref "reference/changelog.md" >}}) for what has shipped.

## Start here

* [Getting Started]({{< relref "getting-started/overview.md" >}}) — the installation paths, and how to choose between them.
* [Installing via Terraform]({{< relref "getting-started/terraform.md" >}}) — the recommended path: observability comes up with the cluster.
* [Installing via Helm]({{< relref "getting-started/helm.md" >}}) — the full-fidelity surface, for when Terraform is not how you deploy.
* [Dependencies]({{< relref "getting-started/dependencies.md" >}}) — what has to exist in the cluster before any of it installs.
* [Production Best Practices]({{< relref "operating/production-best-practices.md" >}}) — the checklist before this runs anywhere that matters.

## How it works

* [Architecture]({{< relref "architecture.md" >}}) — the umbrella chart, the components it bundles, and how telemetry moves between them.
* [o11y Glossary]({{< relref "o11y-glossary.md" >}}) — the vocabulary the rest of these pages assume.

## By signal

| | |
|---|---|
| **Metrics** | [Collecting]({{< relref "metrics/collecting/overview.md" >}}) — the four ways metrics get in — plus [scraping]({{< relref "metrics/scraping.md" >}}), [storing]({{< relref "metrics/storing.md" >}}) in Thanos, and [querying]({{< relref "metrics/querying.md" >}}) them back out |
| **Logs & Events** | The [Alloy agent/gateway split]({{< relref "logs-and-events/architecture.md" >}}), [collecting]({{< relref "logs-and-events/collecting.md" >}}), [storing]({{< relref "logs-and-events/storing.md" >}}) in Loki, [querying]({{< relref "logs-and-events/querying.md" >}}), and [rules]({{< relref "logs-and-events/rules.md" >}}) |
| **Dashboards** | [Importing the Grafana set]({{< relref "dashboards/grafana/importing.md" >}}), the [Grafana Operator]({{< relref "dashboards/grafana/grafana-operator.md" >}}) path that keeps it in sync, [how Grafana is wired]({{< relref "dashboards/grafana/architecture.md" >}}), [authentication]({{< relref "dashboards/grafana/auth.md" >}}), and [Datadog]({{< relref "dashboards/datadog.md" >}}) |

Two areas are still stubs and are not linked above: **Alerting**, and Metrics → Rules.
They appear in the sidebar because the sections exist; the [Roadmap]({{< relref "reference/internal/roadmap.md" >}}) tracks the work behind them.

## Operating the stack

* [Production Best Practices]({{< relref "operating/production-best-practices.md" >}}) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing]({{< relref "operating/securing.md" >}}) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading]({{< relref "operating/upgrading.md" >}}) and [Uninstalling]({{< relref "operating/uninstalling.md" >}}) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting]({{< relref "operating/o11y-troubleshooting.md" >}}) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values]({{< relref "reference/helm/materialize-monitoring-values.md" >}}) — the generated Helm values reference.
* [Terraform variables]({{< relref "reference/terraform/materialize-monitoring-variables.md" >}}) — the generated module variable reference.
* [Reference Metrics]({{< relref "reference/stable-metrics/list-metrics.md" >}}) — the metrics the dashboards depend on, plus [common queries]({{< relref "reference/stable-metrics/common-queries.md" >}}) and [common alerts]({{< relref "reference/stable-metrics/common-alerts.md" >}}).
* [Compatibility]({{< relref "reference/compatibility.md" >}}) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Custom Resource Definitions]({{< relref "reference/crds.md" >}}) — the custom resources the stack reads and relies on.
* [Changelog]({{< relref "reference/changelog.md" >}}) — per-component release history.

## For contributors

* [Contributing]({{< relref "reference/internal/contributing.md" >}}) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap]({{< relref "reference/internal/roadmap.md" >}}) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout]({{< relref "reference/internal/repo-layout.md" >}}) — where things live in the repo.
* [Versioning]({{< relref "reference/internal/versioning.md" >}}) and [Releasing]({{< relref "reference/internal/releasing.md" >}}) — the per-component version streams and the release automation.
* [Design Docs]({{< relref "reference/internal/design-docs/overview.md" >}}) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).
