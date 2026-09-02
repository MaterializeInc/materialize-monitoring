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
  * `.pre-commit-config.yaml`: contributor-experience hooks (see [Internal Development](../contributing/))
  * `.gitattributes`: Git LFS for the subchart tarballs, plus `linguist-generated` on the docsite's duplicate artifacts (see [Two copies, one review](../dashboard/generating/#two-copies-one-review))
  * `bin/`: bash dev/CI entrypoints (flat; no subdirectories)
    * `check-lfs.sh`: verify/repair Git LFS state
    * `extract-crd-schemas.sh`: pull CRD schemas out of upstream charts
    * `fetch-grafana-schemas.sh`: vendor the cog-generated Grafana JSON Schema documents from grafana-foundation-sdk
    * `gen-grafana-models.sh`: generate the Rust Grafana models from those vendored schemas (typify)
    * `extract-grafana-operator-crds.sh`: deflate the Grafana Operator CRDs into the CRDs chart (`make grafana-operator-crds`)
    * `helm-deps.sh`: vendor each chart's locked subchart tarballs into `charts/*/charts/` (`make helm-deps`)
    * `mz-monitoring-build` / `mz-monitoring-check`: thin wrappers over the Rust binaries
  * `packages/`: hand-authored, contributor-facing inputs
    * `components.yaml`: the component manifest driving per-component versioning, changelog attribution, and release artifacts (see [Versioning](../versioning/))
    * `dashboards/`: dashboards-as-code in Rust (`mz-dashboards`) — one module per backend under `src/`, currently `grafana/`; each dashboard owns a `theme.rs` (per-tab colours) and a `selector.rs` (PromQL fragments that name dashboard variables), with `grafana/transform.rs` shared between them; `grafana/render.rs` serializes deterministically, `grafana/mod.rs` is the dashboard registry, and `grafana/queries.rs` is the handle through which every panel takes its expression *and* its description from the query registry — panels write no PromQL and no prose; all driven by `mz-monitoring-build gen-dashboards`
    * `queries/`: query-registry YAML inputs (`materialize-*.yaml` for the deployment, `infra-*.yaml` for the platform, `node-*.yaml` for node-exporter) — the metric/log/alert query definitions, validated against `mzmon-lib/schemas/query/mzmon-query.schema.yaml`
    * `alloy-pipelines/`: Alloy pipeline YAML inputs (`agent.yaml`, `gateway.yaml`, `gateway-metrics.yaml`, `gateway-dest-stub.yaml`)
    * `prometheus-scrapers/`: hand-authored scrape sources (`podmonitor-*.yaml`, `scrapeconfig-cadvisor.yaml`) that the `scrape` transpiler renders into the per-flavor outputs under `pre-rendered/scrapers/`
    * `alloy/`: build context for the distroless Alloy image (`Dockerfile`, `example-config.alloy`)
    * `mzmon-lib/`: Rust library — typed Alloy model, the `scrape` transpiler, and the `query` registry (model + rendering + metric extraction); embedded JSONSchemas under `schemas/{alloy,scrape,query}/`; vendored upstream Grafana JSON Schema documents plus the `packages.json` plugin-id manifest under `schemas/grafana/`, and the Rust models generated from them under `src/grafana/generated/`; `tests/fixtures/env-top.python-baseline.yaml` is the frozen final Python render, shared by the model round-trip and dashboard parity suites; not consumed by customers
    * `mz-monitoring-build/`: Rust CLI for artifact generation (`gen_pipelines.rs`, `gen_scrape_configs.rs`, `extract_metrics.rs`, `main.rs`) and for the release machinery (`versioning.rs`, `propose.rs`, `publish.rs`, `release_notes.rs`, `github.rs`)
    * `mz-monitoring-check/`: Rust schema/consistency checks
    * `mz-monitoring-e2e/`: Rust assertion suite for a *running* stack — one binary for every kind and cloud tier, reading the release's coalesced Helm values to decide what applies (`make e2e-verify`). Asserts only; installs nothing
  * `charts/`
    * `materialize-monitoring/`: umbrella chart
      * `Chart.yaml` / `Chart.lock`: chart metadata; lock pins subchart versions
      * `values.yaml` / `README.md` / `README.md.gotmpl`: profile-driven defaults; README generated from the template via `helm-docs`
      * `charts/`: vendored subchart tarballs (LFS) — `alloy`, `loki`, `thanos`, `alertmanager`, `grafana`, `grafana-operator`, `kube-state-metrics`, `prometheus-node-exporter`, `metrics-server`
      * `pre-rendered/`: generated artifacts loaded via `{{ .Files.Get }}`; never hand-edited
        * `dashboards/`: `grafana/` and `datadog/`
        * `pipelines/`: rendered Alloy (`agent.alloy`, `gateway.alloy`, `gateway-metrics.alloy`, `gateway-dest-stub.alloy`)
        * `scrapers/`: `classic/` (raw scrape config), `prometheus-operator/` (PodMonitors + ScrapeConfig), `gmp/` (GCP `PodMonitoring`)
        * `rules/`: `prometheus/`, `loki/`, `thanos/`
        * `metrics/`: `metric-tiers.yaml`
      * `templates/`: provided resources — `alerts/`, `dashboards/`, `pipelines/`, `scrapers/`, plus `grafana-grafana.yaml` (the `Grafana` instance), `grafana-datasources.yaml` (the Thanos / Loki `GrafanaDatasource`s), `validate.yaml` and `NOTES.txt` (the render-time validation surface), and the `_*.tpl` helper files (`_helpers.tpl`, `_grafana_helpers.tpl`, `_loki_helpers.tpl`, `_thanos_helpers.tpl`, `_alloy_helpers.tpl`)
      * `profiles/`: composable values overlays — sizing (`loki-small`, `loki-large`, `loki-test`), cloud examples (`aws-example`, `gcp-example`, `azure-example`, `aws-amp-fanout`), and shape overlays (`existing-grafana`, `grafana-postgres`, `grafana-pvc`, `grafana-ingress`, `split-namespace`, `otel-metrics-fanout`, `otlp-metrics-honeycomb`). The chart defaults target a **medium** install, so sizing profiles are deltas in either direction
        * `registry/`: hardened-image and private-registry overlays — `pull-secret` (the pull secret alone, composed first) `mirror` (same images, private host — anchors per upstream registry), and one per hardened-image vendor (`chainguard`, `docker-hardened-images`). Split that way because the pull secret is the half they all share, and because no single `global` key reaches every subchart. Only vendors that keep the upstream entrypoint and layout get a profile — Bitnami rebuilds around its own charts, so it is a port rather than a retag
      * `tests/`: `helm-unittest` suites plus `__snapshot__/` (`make helm-tests`)
    * `materialize-monitoring-crds/`: CRDs chart (`Chart.yaml`, `Chart.lock`, `values.yaml`, `README.md`, `README.md.gotmpl`, `tests/`)
      * `charts/`: the vendored `prometheus-operator-crds` tarball, plus `grafana-operator-crds/` deflated from the upstream operator chart, which publishes no CRDs chart of its own — generated by `bin/extract-grafana-operator-crds.sh` (`make grafana-operator-crds`)
  * `terraform/` (see [Terraform Modules](../design-docs/20260803-terraform-modules/)): the cloud-agnostic module that installs the released charts. Versioned as part of the `materialize-monitoring` component rather than on a stream of its own; per-cloud wrappers live in `materialize-terraform-self-managed`
    * `modules/materialize-monitoring/`: the module. Concern per file — `values.tf` (composition order), `scheduling.tf` and `storage_class.tf` (subchart fan-outs), `destinations.tf` (extra metric destinations), `config_hash.tf` (the pod-template hash that rolls Alloy)
      * `examples/{aws,gcp}/`: not deployable roots — plan targets for the tier-0 render check. Both clouds, because the chart's storage defaults are S3-shaped and an AWS-only example agrees with every default it fails to set
    * `test/generic-cloud/`: the tier-2 substrate — rustfs standing in for S3, CNPG for a managed Postgres. Provisions storage and credentials and stops there; it does not call the module
  * `test/e2e/`: kind cluster config and the failure-diagnostics collector (`make e2e-*`). The assertions themselves live in `packages/mz-monitoring-e2e`
  * `docs/`: Hugo docsite (the source of this page)
    * `hugo.toml`: site config; `go.mod` / `go.sum` pin the theme
    * `content/`: authored Markdown
      * `_index.md` files carry **frontmatter only** — no prose. A section's landing content lives in a regular page inside it at `weight: 1` (e.g. `logs-and-events/architecture.md`, `reference/internal/contributing.md`), so every directory in the sidebar is a container and every clickable title is a real page. The site home `content/_index.md` is the one exception
      * top-level sections: `getting-started/`, `metrics/` (incl. `collecting/`), `logs-and-events/`, `dashboards/` (incl. `grafana/`), `alerting/`, `operating/`, plus `architecture.md` and `o11y-glossary.md`
      * `reference/`: `helm/`, `terraform/` (generated variable reference), `stable-metrics/`, `crds.md`, `changelog.md`, and `internal/` (this section — `dashboard/`, `pipelines/`, `design-docs/`, plus `repo-layout.md`, `roadmap.md`, `releasing.md`, `versioning.md`, `skills.md`, `helm.md`)
    * `layouts/`, `static/`, `assets/`, `data/`, `i18n/`, `archetypes/`, `themes/`: Hugo machinery
    * `public/`, `resources/`: generated output (not checked in)
  * `legacy/`: preserved field-engineering assets — `sql_exporter/`, `prometheus/`, `grafana/`, `datadog/`, `tests/`, `docker-compose.yml`, `scrape_config.yaml`
  * `tools/`: ancillary ecosystems kept out of `bin/`
    * `chartlib/`: helm-docs templates
    * `shlib/`: shared bash helpers
  * `.claude/skills/`: authoring conventions consumed by both contributors and AI agents
  * `.github/`: GitHub Actions workflows — `test.yaml` (cargo, helm-unittest, terraform), `e2e.yaml` (the kind tiers), `lint.yaml`, `pipelines.yaml`, `docs.yaml`, `auto-format.yaml`, `propose-bumps.yaml`, `publish-*.yaml`; also `pull_request_template.md`, whose `### Release Notes` section is harvested into `CHANGELOG.md`. Each workflow that path-filters does so in a `changes` job rather than on the trigger, so its `*-gate` rollup still reports on unrelated PRs — a required check skipped at the trigger level stays pending forever
