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
For what you can rely on, see the [stability guarantees](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/stability/).
For what's planned, see the [Roadmap](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/roadmap/),
and for what has shipped, see the [Changelog](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/changelog/).

## Start here

* [Vision](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/vision/) — what the project is for, where it is going, and why.
* [Getting Started](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/getting-started/overview/) — the installation paths, and how to choose between them.
* [Installing via Terraform](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/getting-started/terraform/) — the recommended path: observability comes up with the cluster.
* [Installing via Helm](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/getting-started/helm/) — the full-fidelity surface, for when Terraform is not how you deploy.
* [Dependencies](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/getting-started/dependencies/) — what has to exist in the cluster before any of it installs.
* [Production Best Practices](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/operating/production-best-practices/) — the checklist before this runs anywhere that matters.

## How it works

* [Architecture](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/architecture/) — the umbrella chart, the components it bundles, and how telemetry moves between them.
* [o11y Glossary](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/o11y-glossary/) — the vocabulary the rest of these pages assume.

## By signal

| | |
|---|---|
| **Metrics** | [Collecting](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/metrics/collecting/overview/) — the four ways metrics get in — plus [scraping](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/metrics/scraping/), [storing](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/metrics/storing/) in Thanos, and [querying](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/metrics/querying/) them back out |
| **Logs & Events** | The [Alloy agent/gateway split](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/logs-and-events/architecture/), [collecting](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/logs-and-events/collecting/), [storing](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/logs-and-events/storing/) in Loki, [querying](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/logs-and-events/querying/), and [rules](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/logs-and-events/rules/) |
| **Alerting** | The [alert architecture](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/alerting/architecture/) from ruler to receiver, [configuring](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/alerting/configuring/) the rule evaluators, [alert channels](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/alerting/channels/), [maintenance windows](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/alerting/maintenance/), and [configuring alerting through Terraform](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/alerting/terraform/) |
| **Dashboards** | [Available dashboards](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/dashboards/all/) and [importing the Grafana set](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/dashboards/grafana/importing/), the [Grafana Operator](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/dashboards/grafana/grafana-operator/) path that keeps it in sync, [how Grafana is wired](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/dashboards/grafana/architecture/), [authentication](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/dashboards/grafana/auth/), and [Datadog](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/dashboards/datadog/) |

Metrics → Rules is a stub, so this page doesn't link to it.
It appears in the sidebar because the section exists, and the [Roadmap](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/roadmap/) tracks the work behind it.

## Operating the stack

* [Production Best Practices](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/operating/production-best-practices/) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/operating/securing/) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/operating/upgrading/) and [Uninstalling](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/operating/uninstalling/) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/operating/o11y-troubleshooting/) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/helm/materialize-monitoring-values/) — the generated Helm values reference.
* [Terraform variables](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/terraform/materialize-monitoring-variables/) — the generated module variable reference.
* [Compatibility](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/compatibility/) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Common Alerts](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/common-alerts/) — the bundled alerting rules, and the page each alert's `runbook_url` points at.
* [Common Queries](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/common-queries/) — the PromQL behind the dashboard panels.
* [List of Metrics](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/list-metrics/) — the metrics the dashboards depend on.
* [Custom Resource Definitions](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/crds/) — the custom resources the stack reads and relies on.
* [Changelog](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/changelog/) — per-component release history.

## For contributors

* [Contributing](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/contributing/) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/roadmap/) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/repo-layout/) — where things live in the repo.
* [Versioning](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/versioning/) and [Releasing](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/releasing/) — the per-component version streams and the release automation.
* [Design Docs](/materialize-monitoring/preview/jason-oomkilled-memory-limiter-exit/reference/development/design-docs/overview/) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).

