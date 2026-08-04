---
title: "Repo Layout"
weight: 10
---

This page is a convenience cache of where things live in the repo.
The repository is under active development, so this layout goes stale easily.
If it disagrees with what is actually in the tree, trust the repo and update this page.

The repo is a polyglot monorepo: a Rust workspace, a `uv`-managed Python workspace, and a Go-tooled Hugo docsite all share the root.

<!--
Agent note: only document tracked paths. Local build output and scratch directories are gitignored and do
not belong here — `git ls-files | awk -F/ '{print $1}' | sort -u` is the quickest way to check what is real.
-->

* `materialize-monitoring/`
  * `Cargo.toml` / `Cargo.lock`: top-level Rust workspace (members under `packages/`)
  * `pyproject.toml` / `uv.lock` / `.python-version`: Python workspace, managed by `uv`
  * `go.mod` / `go.sum`: Go module that pins `hugo` and `helm-docs` via tool directives
  * `Makefile`: top-level entrypoint (`make all`, `charts`, `dashboards`, `helm-docs`, `serve-docs`)
  * `CHANGELOG.md`: source of truth for released changes, maintained by the release tooling (see [Releasing](../releasing/))
  * `CONTRIBUTING.md` / `README.md` / `LICENSE`
  * `renovate.json`: automated dependency bumps
  * `.terraform-docs.yml` / `.terraform-docs.docsite.yml`: Terraform doc generation — the first injects into module READMEs, the second writes the docsite variable reference
  * `.pre-commit-config.yaml`: contributor-experience hooks (see [Internal Development](../))
  * `bin/`: bash dev/CI entrypoints (flat; no subdirectories)
    * `check-lfs.sh`: verify/repair Git LFS state
    * `extract-crd-schemas.sh`: pull CRD schemas out of upstream charts
    * `extract-grafana-operator-crds.sh`: deflate the Grafana Operator CRDs into the CRDs chart (`make grafana-operator-crds`)
    * `grafonnet-render`: grafonnet rendering helper
    * `mz-monitoring-build` / `mz-monitoring-check`: thin wrappers over the Rust binaries
  * `packages/`: hand-authored, contributor-facing inputs
    * `components.yaml`: the component manifest driving per-component versioning, changelog attribution, and release artifacts (see [Versioning](../versioning/))
    * `grafana-dashboards/`: Grafana dashboards-as-code (Python + `grafana-foundation-sdk`); sources under `dashboards/` (e.g. `mz_environment/`, `render.py`, `palette.py`)
    * `py-mzmon-lib/`: Python helper library imported by the dashboard packages; not consumed by customers. Also home to the original query registry (`registry/`), now ported to `mzmon-lib`'s Rust `query` module
    * `queries/`: query-registry YAML inputs (`materialize-*.yaml`) — the metric/log/alert query definitions, validated against `mzmon-lib/schemas/query/mzmon-query.schema.yaml`
    * `alloy-pipelines/`: Alloy pipeline YAML inputs (`agent.yaml`, `gateway.yaml`, `gateway-metrics.yaml`, `gateway-dest-stub.yaml`)
    * `prometheus-scrapers/`: hand-authored scrape sources (`podmonitor-*.yaml`, `scrapeconfig-cadvisor.yaml`) that the `scrape` transpiler renders into the per-flavor outputs under `pre-rendered/scrapers/`
    * `alloy/`: build context for the distroless Alloy image (`Dockerfile`, `example-config.alloy`)
    * `mzmon-lib/`: Rust library — typed Alloy model, the `scrape` transpiler, and the `query` registry (model + rendering + metric extraction); embedded JSONSchemas under `schemas/{alloy,scrape,query}/`; not consumed by customers
    * `mz-monitoring-build/`: Rust CLI for artifact generation (`gen_pipelines.rs`, `gen_scrape_configs.rs`, `extract_metrics.rs`, `main.rs`)
    * `mz-monitoring-check/`: Rust schema/consistency checks
  * `charts/`
    * `materialize-monitoring/`: umbrella chart
      * `Chart.yaml` / `Chart.lock`: chart metadata; lock pins subchart versions
      * `values.yaml` / `README.md` / `README.md.gotmpl`: profile-driven defaults; README generated from the template via `helm-docs`
      * `charts/`: vendored subchart tarballs (LFS) — `alloy`, `loki`, `thanos`, `alertmanager`, `grafana`, `grafana-operator`, `kube-state-metrics`, `metrics-server`
      * `pre-rendered/`: generated artifacts loaded via `{{ .Files.Get }}`; never hand-edited
        * `dashboards/`: `grafana/` and `datadog/`
        * `pipelines/`: rendered Alloy (`agent.alloy`, `gateway.alloy`, `gateway-metrics.alloy`, `gateway-dest-stub.alloy`)
        * `scrapers/`: `classic/` (raw scrape config), `prometheus-operator/` (PodMonitors + ScrapeConfig), `gmp/` (GCP `PodMonitoring`)
        * `rules/`: `prometheus/`, `loki/`, `thanos/`
        * `metrics/`: `metric-tiers.yaml`
      * `templates/`: provided resources — `alerts/`, `dashboards/`, `pipelines/`, `scrapers/`, plus `grafana-grafana.yaml` (the `Grafana` instance), `grafana-datasources.yaml` (the Thanos / Loki `GrafanaDatasource`s), `validate.yaml` and `NOTES.txt` (the render-time validation surface), and the `_*.tpl` helper files (`_helpers.tpl`, `_grafana_helpers.tpl`, `_loki_helpers.tpl`, `_thanos_helpers.tpl`, `_alloy_helpers.tpl`)
      * `profiles/`: composable values overlays — sizing (`loki-small`, `loki-large`, `loki-test`), cloud examples (`aws-example`, `gcp-example`, `aws-amp-example`), and shape overlays (`existing-grafana`, `grafana-postgres`, `split-namespace`, `otel-metrics-fanout`, `otlp-metrics-honeycomb`). The chart defaults target a **medium** install, so sizing profiles are deltas in either direction
      * `tests/`: `helm-unittest` suites plus `__snapshot__/` (`make helm-tests`)
    * `materialize-monitoring-crds/`: CRDs chart (`Chart.yaml`, `Chart.lock`, `values.yaml`, `README.md`, `README.md.gotmpl`, `tests/`)
      * `charts/`: the vendored `prometheus-operator-crds` tarball, plus `grafana-operator-crds/` deflated from the upstream operator chart, which publishes no CRDs chart of its own — generated by `bin/extract-grafana-operator-crds.sh` (`make grafana-operator-crds`)
  * `terraform/` *(planned — see [Terraform Modules](../design-docs/20260803-terraform-modules/))*: the cloud-agnostic module that installs the released charts. Versioned as part of the `materialize-monitoring` component rather than on a stream of its own; per-cloud wrappers live in `materialize-terraform-self-managed`
  * `docs/`: Hugo docsite (the source of this page)
    * `hugo.toml`: site config; `go.mod` / `go.sum` pin the theme
    * `content/`: authored Markdown
      * top-level sections: `getting-started/`, `metrics/` (incl. `collecting/`), `logs-and-events/`, `dashboards/` (incl. `grafana/`), `alerting/`, `operating/`, plus `architecture.md` and `o11y-glossary.md`
      * `reference/`: `helm/`, `terraform/` (generated variable reference), `stable-metrics/`, `crds.md`, `changelog.md`, and `internal/` (this section — `dashboard/`, `pipelines/`, `design-docs/`, plus `repo-layout.md`, `roadmap.md`, `releasing.md`, `versioning.md`, `skills.md`, `helm.md`)
    * `layouts/`, `static/`, `assets/`, `data/`, `i18n/`, `archetypes/`, `themes/`: Hugo machinery
    * `public/`, `resources/`: generated output (not checked in)
  * `legacy/`: preserved field-engineering assets — `sql_exporter/`, `prometheus/`, `grafana/`, `datadog/`, `tests/`, `docker-compose.yml`, `scrape_config.yaml`
  * `tools/`: ancillary ecosystems kept out of `bin/`
    * `chartlib/`: helm-docs templates
    * `grafonnet/`: grafonnet/jsonnet vendoring (`jsonnetfile.json` + lock)
    * `shlib/`: shared bash helpers
  * `.claude/skills/`: authoring conventions consumed by both contributors and AI agents
  * `.github/`: GitHub Actions workflows
