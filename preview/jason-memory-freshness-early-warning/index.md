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
For what you can rely on, see the [stability guarantees](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/stability/).
For what's planned, see the [Roadmap](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/roadmap/),
and for what has shipped, see the [Changelog](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/changelog/).

## Start here

* [Vision](/materialize-monitoring/preview/jason-memory-freshness-early-warning/vision/) — what the project is for, where it is going, and why.
* [Getting Started](/materialize-monitoring/preview/jason-memory-freshness-early-warning/getting-started/overview/) — the installation paths, and how to choose between them.
* [Installing via Terraform](/materialize-monitoring/preview/jason-memory-freshness-early-warning/getting-started/terraform/) — the recommended path: observability comes up with the cluster.
* [Installing via Helm](/materialize-monitoring/preview/jason-memory-freshness-early-warning/getting-started/helm/) — the full-fidelity surface, for when Terraform is not how you deploy.
* [Dependencies](/materialize-monitoring/preview/jason-memory-freshness-early-warning/getting-started/dependencies/) — what has to exist in the cluster before any of it installs.
* [Production Best Practices](/materialize-monitoring/preview/jason-memory-freshness-early-warning/operating/production-best-practices/) — the checklist before this runs anywhere that matters.

## How it works

* [Architecture](/materialize-monitoring/preview/jason-memory-freshness-early-warning/architecture/) — the umbrella chart, the components it bundles, and how telemetry moves between them.
* [o11y Glossary](/materialize-monitoring/preview/jason-memory-freshness-early-warning/o11y-glossary/) — the vocabulary the rest of these pages assume.

## By signal

| | |
|---|---|
| **Metrics** | [Collecting](/materialize-monitoring/preview/jason-memory-freshness-early-warning/metrics/collecting/overview/) — the four ways metrics get in — plus [scraping](/materialize-monitoring/preview/jason-memory-freshness-early-warning/metrics/scraping/), [storing](/materialize-monitoring/preview/jason-memory-freshness-early-warning/metrics/storing/) in Thanos, and [querying](/materialize-monitoring/preview/jason-memory-freshness-early-warning/metrics/querying/) them back out |
| **Logs & Events** | The [Alloy agent/gateway split](/materialize-monitoring/preview/jason-memory-freshness-early-warning/logs-and-events/architecture/), [collecting](/materialize-monitoring/preview/jason-memory-freshness-early-warning/logs-and-events/collecting/), [storing](/materialize-monitoring/preview/jason-memory-freshness-early-warning/logs-and-events/storing/) in Loki, [querying](/materialize-monitoring/preview/jason-memory-freshness-early-warning/logs-and-events/querying/), and [rules](/materialize-monitoring/preview/jason-memory-freshness-early-warning/logs-and-events/rules/) |
| **Alerting** | The [alert architecture](/materialize-monitoring/preview/jason-memory-freshness-early-warning/alerting/architecture/) from ruler to receiver, [configuring](/materialize-monitoring/preview/jason-memory-freshness-early-warning/alerting/configuring/) the rule evaluators, [alert channels](/materialize-monitoring/preview/jason-memory-freshness-early-warning/alerting/channels/), [maintenance windows](/materialize-monitoring/preview/jason-memory-freshness-early-warning/alerting/maintenance/), and [configuring alerting through Terraform](/materialize-monitoring/preview/jason-memory-freshness-early-warning/alerting/terraform/) |
| **Dashboards** | [Available dashboards](/materialize-monitoring/preview/jason-memory-freshness-early-warning/dashboards/all/) and [importing the Grafana set](/materialize-monitoring/preview/jason-memory-freshness-early-warning/dashboards/grafana/importing/), the [Grafana Operator](/materialize-monitoring/preview/jason-memory-freshness-early-warning/dashboards/grafana/grafana-operator/) path that keeps it in sync, [how Grafana is wired](/materialize-monitoring/preview/jason-memory-freshness-early-warning/dashboards/grafana/architecture/), [authentication](/materialize-monitoring/preview/jason-memory-freshness-early-warning/dashboards/grafana/auth/), and [Datadog](/materialize-monitoring/preview/jason-memory-freshness-early-warning/dashboards/datadog/) |

Metrics → Rules is a stub, so this page doesn't link to it.
It appears in the sidebar because the section exists, and the [Roadmap](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/roadmap/) tracks the work behind it.

## Operating the stack

* [Production Best Practices](/materialize-monitoring/preview/jason-memory-freshness-early-warning/operating/production-best-practices/) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing](/materialize-monitoring/preview/jason-memory-freshness-early-warning/operating/securing/) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading](/materialize-monitoring/preview/jason-memory-freshness-early-warning/operating/upgrading/) and [Uninstalling](/materialize-monitoring/preview/jason-memory-freshness-early-warning/operating/uninstalling/) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting](/materialize-monitoring/preview/jason-memory-freshness-early-warning/operating/o11y-troubleshooting/) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/helm/materialize-monitoring-values/) — the generated Helm values reference.
* [Terraform variables](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/terraform/materialize-monitoring-variables/) — the generated module variable reference.
* [Compatibility](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/compatibility/) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Common Alerts](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/common-alerts/) — the bundled alerting rules, and the page each alert's `runbook_url` points at.
* [Common Queries](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/common-queries/) — the PromQL behind the dashboard panels.
* [List of Metrics](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/list-metrics/) — the metrics the dashboards depend on.
* [Custom Resource Definitions](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/crds/) — the custom resources the stack reads and relies on.
* [Changelog](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/changelog/) — per-component release history.

## For contributors

* [Contributing](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/contributing/) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/roadmap/) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/repo-layout/) — where things live in the repo.
* [Versioning](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/versioning/) and [Releasing](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/releasing/) — the per-component version streams and the release automation.
* [Design Docs](/materialize-monitoring/preview/jason-memory-freshness-early-warning/reference/development/design-docs/overview/) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).

