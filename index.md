# materialize-monitoring Documentation


# materialize-monitoring Documentation

`materialize-monitoring` is first-class observability for Materialize deployments — metrics, logs, events, dashboards, and alerts, packaged as a Helm chart and a Terraform module.
It is a one-stop shop for teams who want one, and every piece of it can be turned off for teams who already run their own.

If you install Materialize with the Terraform modules, the stack comes up with the cluster by default from `materialize-terraform-self-managed` v11 onward — set `enable_observability = false` to opt out.

Nothing here is required to run Materialize.
If you are looking for the main Materialize documentation, see [materialize.com/docs](https://materialize.com/docs/).

> [!WARNING]
>  **Pre-1.0.** Interfaces are still moving, and breaking changes can ride a minor release until 1.0 is stamped.
>  See the [Roadmap](/materialize-monitoring/reference/internal/roadmap/) for what is built and what is coming, and the [Changelog](/materialize-monitoring/reference/changelog/) for what has shipped.

## Start here

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
| **Dashboards** | [Importing the Grafana set](/materialize-monitoring/dashboards/grafana/importing/), the [Grafana Operator](/materialize-monitoring/dashboards/grafana/grafana-operator/) path that keeps it in sync, [how Grafana is wired](/materialize-monitoring/dashboards/grafana/architecture/), [authentication](/materialize-monitoring/dashboards/grafana/auth/), and [Datadog](/materialize-monitoring/dashboards/datadog/) |

Two areas are still stubs and are not linked above: **Alerting**, and Metrics → Rules.
They appear in the sidebar because the sections exist; the [Roadmap](/materialize-monitoring/reference/internal/roadmap/) tracks the work behind them.

## Operating the stack

* [Production Best Practices](/materialize-monitoring/operating/production-best-practices/) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing](/materialize-monitoring/operating/securing/) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading](/materialize-monitoring/operating/upgrading/) and [Uninstalling](/materialize-monitoring/operating/uninstalling/) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting](/materialize-monitoring/operating/o11y-troubleshooting/) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values](/materialize-monitoring/reference/helm/materialize-monitoring-values/) — the generated Helm values reference.
* [Terraform variables](/materialize-monitoring/reference/terraform/materialize-monitoring-variables/) — the generated module variable reference.
* [Reference Metrics](/materialize-monitoring/reference/stable-metrics/list-metrics/) — the metrics the dashboards depend on, plus [common queries](/materialize-monitoring/reference/stable-metrics/common-queries/) and [common alerts](/materialize-monitoring/reference/stable-metrics/common-alerts/).
* [Compatibility](/materialize-monitoring/reference/compatibility/) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Custom Resource Definitions](/materialize-monitoring/reference/crds/) — the custom resources the stack reads and relies on.
* [Changelog](/materialize-monitoring/reference/changelog/) — per-component release history.

## For contributors

* [Contributing](/materialize-monitoring/reference/internal/contributing/) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap](/materialize-monitoring/reference/internal/roadmap/) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout](/materialize-monitoring/reference/internal/repo-layout/) — where things live in the repo.
* [Versioning](/materialize-monitoring/reference/internal/versioning/) and [Releasing](/materialize-monitoring/reference/internal/releasing/) — the per-component version streams and the release automation.
* [Design Docs](/materialize-monitoring/reference/internal/design-docs/overview/) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).

