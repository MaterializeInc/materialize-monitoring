# materialize-monitoring Documentation


# materialize-monitoring Documentation

`materialize-monitoring` is first-class observability for Materialize deployments — metrics, logs, events, dashboards, and alerts, packaged as a Helm chart and a Terraform module.
It is a one-stop shop for teams who want one, and every piece of it can be turned off for teams who already run their own.

If you install Materialize with the Terraform modules, the stack comes up with the cluster by default from `materialize-terraform-self-managed` v11 onward — set `enable_observability = false` to opt out.

Nothing here is required to run Materialize.
If you are looking for the main Materialize documentation, see [materialize.com/docs](https://materialize.com/docs/).

> [!WARNING]
>  **Pre-1.0.** Interfaces are still moving, and breaking changes can ride a minor release until 1.0 is stamped.
>  See the [Roadmap](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/roadmap/) for what is built and what is coming, and the [Changelog](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/changelog/) for what has shipped.

## Start here

* [Getting Started](/materialize-monitoring/preview/heather-troubleshoot-skill/getting-started/overview/) — the installation paths, and how to choose between them.
* [Installing via Terraform](/materialize-monitoring/preview/heather-troubleshoot-skill/getting-started/terraform/) — the recommended path: observability comes up with the cluster.
* [Installing via Helm](/materialize-monitoring/preview/heather-troubleshoot-skill/getting-started/helm/) — the full-fidelity surface, for when Terraform is not how you deploy.
* [Dependencies](/materialize-monitoring/preview/heather-troubleshoot-skill/getting-started/dependencies/) — what has to exist in the cluster before any of it installs.
* [Production Best Practices](/materialize-monitoring/preview/heather-troubleshoot-skill/operating/production-best-practices/) — the checklist before this runs anywhere that matters.

## How it works

* [Architecture](/materialize-monitoring/preview/heather-troubleshoot-skill/architecture/) — the umbrella chart, the components it bundles, and how telemetry moves between them.
* [o11y Glossary](/materialize-monitoring/preview/heather-troubleshoot-skill/o11y-glossary/) — the vocabulary the rest of these pages assume.

## By signal

| | |
|---|---|
| **Metrics** | [Collecting](/materialize-monitoring/preview/heather-troubleshoot-skill/metrics/collecting/overview/) — the four ways metrics get in — plus [scraping](/materialize-monitoring/preview/heather-troubleshoot-skill/metrics/scraping/), [storing](/materialize-monitoring/preview/heather-troubleshoot-skill/metrics/storing/) in Thanos, and [querying](/materialize-monitoring/preview/heather-troubleshoot-skill/metrics/querying/) them back out |
| **Logs & Events** | The [Alloy agent/gateway split](/materialize-monitoring/preview/heather-troubleshoot-skill/logs-and-events/architecture/), [collecting](/materialize-monitoring/preview/heather-troubleshoot-skill/logs-and-events/collecting/), [storing](/materialize-monitoring/preview/heather-troubleshoot-skill/logs-and-events/storing/) in Loki, [querying](/materialize-monitoring/preview/heather-troubleshoot-skill/logs-and-events/querying/), and [rules](/materialize-monitoring/preview/heather-troubleshoot-skill/logs-and-events/rules/) |
| **Dashboards** | [Importing the Grafana set](/materialize-monitoring/preview/heather-troubleshoot-skill/dashboards/grafana/importing/), the [Grafana Operator](/materialize-monitoring/preview/heather-troubleshoot-skill/dashboards/grafana/grafana-operator/) path that keeps it in sync, [how Grafana is wired](/materialize-monitoring/preview/heather-troubleshoot-skill/dashboards/grafana/architecture/), [authentication](/materialize-monitoring/preview/heather-troubleshoot-skill/dashboards/grafana/auth/), and [Datadog](/materialize-monitoring/preview/heather-troubleshoot-skill/dashboards/datadog/) |

Two areas are still stubs and are not linked above: **Alerting**, and Metrics → Rules.
They appear in the sidebar because the sections exist; the [Roadmap](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/roadmap/) tracks the work behind them.

## Operating the stack

* [Production Best Practices](/materialize-monitoring/preview/heather-troubleshoot-skill/operating/production-best-practices/) — sizing, retention, replication, disruption budgets, and durability, each tagged by who owns it.
* [Securing](/materialize-monitoring/preview/heather-troubleshoot-skill/operating/securing/) — network policy, in-cluster TLS, and exposing Grafana.
* [Upgrading](/materialize-monitoring/preview/heather-troubleshoot-skill/operating/upgrading/) and [Uninstalling](/materialize-monitoring/preview/heather-troubleshoot-skill/operating/uninstalling/) — including the teardown ordering that avoids a finalizer deadlock.
* [o11y Troubleshooting](/materialize-monitoring/preview/heather-troubleshoot-skill/operating/o11y-troubleshooting/) — when the monitoring itself is the thing that is broken.

## Reference

* [materialize-monitoring values](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/helm/materialize-monitoring-values/) — the generated Helm values reference.
* [Terraform variables](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/terraform/materialize-monitoring-variables/) — the generated module variable reference.
* [Reference Metrics](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/stable-metrics/list-metrics/) — the metrics the dashboards depend on, plus [common queries](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/stable-metrics/common-queries/) and [common alerts](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/stable-metrics/common-alerts/).
* [Compatibility](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/compatibility/) — supported versions of Materialize, Grafana, GKE, and the Terraform modules.
* [Custom Resource Definitions](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/crds/) — the custom resources the stack reads and relies on.
* [Changelog](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/changelog/) — per-component release history.

## For contributors

* [Contributing](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/contributing/) — the contributor guide, conventions, and the pre-commit wiring.
* [Roadmap](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/roadmap/) — the current source of truth for what is built, in flight, and planned next.
* [Repository Layout](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/repo-layout/) — where things live in the repo.
* [Versioning](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/versioning/) and [Releasing](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/releasing/) — the per-component version streams and the release automation.
* [Design Docs](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/design-docs/overview/) — the decisions behind the larger pieces.

## Getting help

Please [reach out for Support](https://materialize.com/docs/support/), or open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues).

