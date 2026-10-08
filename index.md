# materialize-monitoring Documentation


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
For what you can rely on, see the [stability guarantees](/materialize-monitoring/reference/stability/).
For what's planned, see the [Roadmap](/materialize-monitoring/reference/development/roadmap/),
and for what has shipped, see the [Changelog](/materialize-monitoring/reference/changelog/).

## Start here

* [Vision](/materialize-monitoring/vision/) — what the project is for, where it is going, and why.
* [Getting Started](/materialize-monitoring/getting-started/overview/) — the installation paths, and how to choose between them.
* [Installing via Terraform](/materialize-monitoring/getting-started/terraform/) — the recommended path: observability comes up with the cluster.
* [Installing via Helm](/materialize-monitoring/getting-started/helm/) — the full-fidelity surface, for when Terraform is not how you deploy.
* [Dependencies](/materialize-monitoring/getting-started/dependencies/) — what has to exist in the cluster before any of it installs.
* [Production Best Practices](/materialize-monitoring/operating/production-best-practices/) — the checklist before this runs anywhere that matters.

## How it works

* [Architecture](/materialize-monitoring/architecture/) — the umbrella chart, the components it bundles, and how telemetry moves between them.
* [o11y Glossary](/materialize-monitoring/o11y-glossary/) — the vocabulary the rest of these pages assume.

## By signal

| | |
|---|---|
| **Metrics** | [Collecting](/materialize-monitoring/metrics/collecting/overview/) — the four ways metrics get in — plus [scraping](/materialize-monitoring/metrics/scraping/), [storing](/materialize-monitoring/metrics/storing/) in Thanos, and [querying](/materialize-monitoring/metrics/querying/) them back out |
| **Logs & Events** | The [Alloy agent/gateway split](/materialize-monitoring/logs-and-events/architecture/), [collecting](/materialize-monitoring/logs-and-events/collecting/), [storing](/materialize-monitoring/logs-and-events/storing/) in Loki, [querying](/materialize-monitoring/logs-and-events/querying/), and [rules](/materialize-monitoring/logs-and-events/rules/) |
| **Alerting** | The [alert architecture](/materialize-monitoring/alerting/architecture/) from ruler to receiver, [configuring](/materialize-monitoring/alerting/configuring/) the rule evaluators, [alert channels](/materialize-monitoring/alerting/channels/), [maintenance windows](/materialize-monitoring/alerting/maintenance/), and [configuring alerting through Terraform](/materialize-monitoring/alerting/terraform/) |
| **Dashboards** | [Available dashboards](/materialize-monitoring/dashboards/all/) and [importing the Grafana set](/materialize-monitoring/dashboards/grafana/importing/), the [Grafana Operator](/materialize-monitoring/dashboards/grafana/grafana-operator/) path that keeps it in sync, [how Grafana is wired](/materialize-monitoring/dashboards/grafana/architecture/), [authentication](/materialize-monitoring/dashboards/grafana/auth/), and [Datadog](/materialize-monitoring/dashboards/datadog/) |

Metrics → Rules is a stub, so this page doesn't link to it.
It appears in the sidebar because the section exists, and the [Roadmap](/materialize-monitoring/reference/development/roadmap/) tracks the work behind it.

## Operating the stack

* [Production Best Practices](/materialize-monitoring/operating/production-best-practices/) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing](/materialize-monitoring/operating/securing/) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading](/materialize-monitoring/operating/upgrading/) and [Uninstalling](/materialize-monitoring/operating/uninstalling/) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting](/materialize-monitoring/operating/o11y-troubleshooting/) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values](/materialize-monitoring/reference/helm/materialize-monitoring-values/) — the generated Helm values reference.
* [Terraform variables](/materialize-monitoring/reference/terraform/materialize-monitoring-variables/) — the generated module variable reference.
* [Compatibility](/materialize-monitoring/reference/compatibility/) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Common Alerts](/materialize-monitoring/reference/common-alerts/) — the bundled alerting rules, and the page each alert's `runbook_url` points at.
* [Common Queries](/materialize-monitoring/reference/common-queries/) — the PromQL behind the dashboard panels.
* [List of Metrics](/materialize-monitoring/reference/list-metrics/) — the metrics the dashboards depend on.
* [Custom Resource Definitions](/materialize-monitoring/reference/crds/) — the custom resources the stack reads and relies on.
* [Changelog](/materialize-monitoring/reference/changelog/) — per-component release history.

## For contributors

* [Contributing](/materialize-monitoring/reference/development/contributing/) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap](/materialize-monitoring/reference/development/roadmap/) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout](/materialize-monitoring/reference/development/repo-layout/) — where things live in the repo.
* [Versioning](/materialize-monitoring/reference/development/versioning/) and [Releasing](/materialize-monitoring/reference/development/releasing/) — the per-component version streams and the release automation.
* [Design Docs](/materialize-monitoring/reference/development/design-docs/overview/) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).

