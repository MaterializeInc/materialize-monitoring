---
title: "materialize-monitoring Documentation"
htmltitle: "Home"
disable_toc: false
disable_h1: true
weight: 1
---
# materialize-monitoring Documentation

`materialize-monitoring` is first-class observability for Materialize deployments:
metrics, logs, events, dashboards, and alerts, packaged as a Helm chart and a Terraform module.
It's a one-stop shop if you want one, and you can turn off any piece of it if you already run your own.

If you install Materialize with the Terraform modules,
the stack comes up with the cluster by default from `materialize-terraform-self-managed` v11 onward.
To opt out, set `enable_observability = false`.

Nothing here is required to run Materialize.
For Materialize itself, see the [Materialize documentation](https://materialize.com/docs/).

From v1.0.0, a breaking change to the committed surface goes through a deprecation cycle and lands only in a major release.
For what you can rely on, see the [stability guarantees]({{< relref "reference/stability.md" >}}).
For what's planned, see the [Roadmap]({{< relref "reference/development/roadmap.md" >}}),
and for what has shipped, see the [Changelog]({{< relref "reference/changelog.md" >}}).

## Start here

* [Vision]({{< relref "vision.md" >}}) — what the project is for, where it is going, and why.
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
| **Alerting** | The [alert architecture]({{< relref "alerting/architecture.md" >}}) from ruler to receiver, [configuring]({{< relref "alerting/configuring.md" >}}) the rule evaluators, [alert channels]({{< relref "alerting/channels.md" >}}), [maintenance windows]({{< relref "alerting/maintenance.md" >}}), and [configuring alerting through Terraform]({{< relref "alerting/terraform.md" >}}) |
| **Dashboards** | [Available dashboards]({{< relref "dashboards/all.md" >}}) and [importing the Grafana set]({{< relref "dashboards/grafana/importing.md" >}}), the [Grafana Operator]({{< relref "dashboards/grafana/grafana-operator.md" >}}) path that keeps it in sync, [how Grafana is wired]({{< relref "dashboards/grafana/architecture.md" >}}), [authentication]({{< relref "dashboards/grafana/auth.md" >}}), and [Datadog]({{< relref "dashboards/datadog.md" >}}) |

Metrics → Rules is a stub, so this page doesn't link to it.
It appears in the sidebar because the section exists, and the [Roadmap]({{< relref "reference/development/roadmap.md" >}}) tracks the work behind it.

## Operating the stack

* [Production Best Practices]({{< relref "operating/production-best-practices.md" >}}) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing]({{< relref "operating/securing.md" >}}) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading]({{< relref "operating/upgrading.md" >}}) and [Uninstalling]({{< relref "operating/uninstalling.md" >}}) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting]({{< relref "operating/o11y-troubleshooting.md" >}}) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values]({{< relref "reference/helm/materialize-monitoring-values.md" >}}) — the generated Helm values reference.
* [Terraform variables]({{< relref "reference/terraform/materialize-monitoring-variables.md" >}}) — the generated module variable reference.
* [Compatibility]({{< relref "reference/compatibility.md" >}}) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Common Alerts]({{< relref "reference/common-alerts.md" >}}) — the bundled alerting rules, and the page each alert's `runbook_url` points at.
* [Common Queries]({{< relref "reference/common-queries.md" >}}) — the PromQL behind the dashboard panels.
* [List of Metrics]({{< relref "reference/list-metrics.md" >}}) — the metrics the dashboards depend on.
* [Custom Resource Definitions]({{< relref "reference/crds.md" >}}) — the custom resources the stack reads and relies on.
* [Changelog]({{< relref "reference/changelog.md" >}}) — per-component release history.

## For contributors

* [Contributing]({{< relref "reference/development/contributing.md" >}}) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap]({{< relref "reference/development/roadmap.md" >}}) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout]({{< relref "reference/development/repo-layout.md" >}}) — where things live in the repo.
* [Versioning]({{< relref "reference/development/versioning.md" >}}) and [Releasing]({{< relref "reference/development/releasing.md" >}}) — the per-component version streams and the release automation.
* [Design Docs]({{< relref "reference/development/design-docs/overview.md" >}}) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).
