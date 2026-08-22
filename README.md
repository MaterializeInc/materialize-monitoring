# Unified Monitoring for Materialize

First-class observability for Materialize deployments — metrics, logs, events, dashboards, and alerts.
This repository packages the whole stack as a Helm chart and a Terraform module, so an operator gets a working observability deployment instead of a pile of parts to assemble.

**📖 [Documentation](https://materializeinc.github.io/materialize-monitoring/)** — installation, architecture, operations, and reference.

Nothing here is required to run Materialize.
The goal is a one-stop shop for teams who want one, without forcing this stack on teams who already run their own.

## Status

**Pre-1.0 and under active development.**
Interfaces are still moving: until 1.0 is stamped, breaking changes can ride a minor release — see [Choosing the next version](docs/content/reference/internal/releasing.md#choosing-the-next-version).

* [CHANGELOG.md](CHANGELOG.md) — the source of truth for what has shipped.
* [Roadmap](docs/content/reference/internal/roadmap.md) — the source of truth for what is built, in flight, and planned next.
* [Repository Layout](docs/content/reference/internal/repo-layout.md) — where things live in the repo.

## Getting started

### Terraform (recommended)

If you stand up Materialize with [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/materialize-terraform-self-managed), observability comes up with the cluster — **on by default since v11**, on AWS, GCP, and Azure alike.
The per-cloud wrapper modules create the buckets and workload identity, then install these charts at a pinned version.
The cloud-agnostic module they wrap lives in this repo at [`terraform/modules/materialize-monitoring`](terraform/modules/materialize-monitoring).

Set `enable_observability = false` to opt out.
See [Installing via Terraform](https://materializeinc.github.io/materialize-monitoring/getting-started/terraform/).

### Helm

The charts are the full-fidelity surface — everything Terraform does is a layer over them — and are published to GHCR as OCI artifacts:

```bash
helm install mzmon-crds oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-crds --namespace monitoring
helm install mzmon oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring --namespace monitoring --skip-crds
```

That is the shape, not a complete install: object storage and credentials come first, and the backend has to be named in your values.
See [Installing via Helm](https://materializeinc.github.io/materialize-monitoring/getting-started/helm/) for the real procedure, and [Production Best Practices](https://materializeinc.github.io/materialize-monitoring/operating/production-best-practices/) before running it anywhere that matters.

### Dashboards only

The Grafana dashboards are usable against an observability stack you already run.
Download them from [Importing Dashboards](https://materializeinc.github.io/materialize-monitoring/dashboards/grafana/importing/), or point your own collector at the shipped [scrape configs](https://materializeinc.github.io/materialize-monitoring/metrics/scraping/).

## What this repository provides

Each artifact carries its own SemVer stream, declared in [`packages/components.yaml`](packages/components.yaml) and released on a monthly cadence.

| Artifact | What it is |
|---|---|
| `materialize-monitoring` | Helm umbrella chart, plus the Terraform module that installs it |
| `materialize-monitoring-crds` | Optional CRDs chart, so CRD lifecycle is managed separately |
| Dashboards | Grafana dashboards-as-code, generated from Python (`grafana-foundation-sdk`) |
| Pipelines | Alloy agent and gateway pipelines, generated from a typed Rust model |
| Scrapers | ScrapeConfigs, PodMonitors/ServiceMonitors, and GCP `PodMonitoring` |
| Container images | A distroless, multi-arch, non-root Alloy image (FIPS boringcrypto) |
| Documentation | The [docsite](https://materializeinc.github.io/materialize-monitoring/), covering the stack and how to operate it |

The umbrella chart bundles the stack itself — Alloy (an agent DaemonSet and a gateway), Loki for logs, Thanos for metrics, Grafana with grafana-operator, Alertmanager, kube-state-metrics, node-exporter, and metrics-server — and layers the generated dashboards, pipelines, scrape configs, and rules on top.
Each component can be disabled to integrate with infrastructure you already run.
See [Architecture](https://materializeinc.github.io/materialize-monitoring/architecture/) for how the pieces fit together.

## Dashboards

**Materialize Environment Overview (`env-top`)** ships today: Day 2 operations for a Materialize environment, across Summary, Kubernetes, Cluster, Connections, Compute, and Storage tabs — including Hydration, Freshness, Sources, and Sinks summaries.
The same sources render cloud and self-managed variants, and a GCP/GMP-optimized variant.

* Source: [`packages/grafana-dashboards/dashboards/`](packages/grafana-dashboards/dashboards/)
* Generated: [`charts/materialize-monitoring/pre-rendered/dashboards/grafana/`](charts/materialize-monitoring/pre-rendered/dashboards/grafana/), and downloadable from the [docsite](https://materializeinc.github.io/materialize-monitoring/dashboards/grafana/importing/)
* Installed by the chart through grafana-operator, which keeps them in sync rather than importing a point-in-time copy

Troubleshooting, Logs & Events, Upgrades, Networking, the per-subsystem drilldowns, and native Datadog / Google Cloud Monitoring / Honeycomb dashboard sets are planned — see the [Dashboards workstream](docs/content/reference/internal/roadmap.md#dashboards) for status.
Until those land, non-Grafana backends are served by forwarding over OTLP rather than by native dashboards.

## Compatibility

Grafana v13+ is required for dashboard schema v2; v12 generally works.
Dashboards degrade gracefully without the `mz_object_info` metric introduced in Materialize v26.29.0, and the scrapers need the `environmentd` labels introduced in v26.24.0.

[Compatibility](https://materializeinc.github.io/materialize-monitoring/reference/compatibility/) carries the full matrix, including `materialize-terraform-self-managed` and GKE.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the quickstart, and [Internal Development](https://materializeinc.github.io/materialize-monitoring/reference/internal/contributing/) for the full contributor guide.

Bug reports and feature requests belong in [GitHub issues](https://github.com/MaterializeInc/materialize-monitoring/issues).
For help running Materialize, [reach out for support](https://materialize.com/docs/support/).

## License

This repository is [licensed](LICENSE) under the Business Source License 1.1, converting to the Apache 2.0 license on April 21, 2030.

Materialize itself is available as a self-managed product and as a fully managed cloud service with [credit-based pricing](https://materialize.com/pricing/).
