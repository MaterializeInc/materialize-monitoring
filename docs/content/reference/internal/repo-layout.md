---
title: "Repo Layout"
weight: 10
---

This page is a convenience cache of where things live in the repo.
The repository is under active development, so this layout goes stale easily.
If it disagrees with what is actually in the tree, trust the repo and update this page.

The repo is a polyglot monorepo: a Rust workspace, a `uv`-managed Python workspace, and a Go-tooled Hugo docsite all share the root.

* `materialize-monitoring/`
  * `Cargo.toml` / `Cargo.lock`: top-level Rust workspace (members under `packages/`)
  * `pyproject.toml` / `uv.lock` / `.python-version`: Python workspace, managed by `uv`
  * `go.mod` / `go.sum`: Go module that pins `hugo` and `helm-docs` via tool directives
  * `Makefile`: top-level entrypoint (`make all`, `charts`, `dashboards`, `helm-docs`, `serve-docs`)
  * `.pre-commit-config.yaml`: contributor-experience hooks (see [Internal Development](../))
  * `bin/`: bash dev/CI entrypoints (flat; no subdirectories)
    * `check-lfs.sh`: verify/repair Git LFS state
    * `dashboard-sync.sh`: regenerate and sync dashboards
    * `grafonnet-render`: grafonnet rendering helper
    * `mz-monitoring-build` / `mz-monitoring-check`: thin wrappers over the Rust binaries
  * `packages/`: hand-authored, contributor-facing inputs
    * `grafana-dashboards/`: Grafana dashboards-as-code (Python + `grafana-foundation-sdk`); sources under `dashboards/` (e.g. `mz_environment/`, `render.py`, `palette.py`)
    * `py-mzmon-lib/`: Python helper library imported by the dashboard packages; not consumed by customers. Also home to the original query registry (`registry/`), now ported to `mzmon-lib`'s Rust `query` module
    * `queries/`: query-registry YAML inputs (`materialize-*.yaml`) — the metric/log/alert query definitions, validated against `mzmon-lib/schemas/query/mzmon-query.schema.yaml`
    * `alloy-pipelines/`: Alloy pipeline YAML inputs (`agent.yaml`, `gateway.yaml`)
    * `ref-alloy-pipelines/`: Python reference pipelines being ported into the typed Rust path (`agent_config.py`, `gateway_config.py`, `processor.alloy`, `alloy/`)
    * `mzmon-lib/`: Rust library — typed Alloy model, the `scrape` transpiler, and the `query` registry (model + rendering + metric extraction); embedded JSONSchemas under `schemas/{alloy,scrape,query}/`; not consumed by customers
    * `mz-monitoring-build/`: Rust CLI for artifact generation (`gen_pipelines.rs`, `gen_scrape_configs.rs`, `extract_metrics.rs`, `main.rs`)
    * `mz-monitoring-check/`: Rust schema/consistency checks
  * `charts/`
    * `materialize-monitoring/`: umbrella chart
      * `Chart.yaml` / `Chart.lock`: chart metadata; lock pins subchart versions
      * `values.yaml` / `README.md` / `README.md.gotmpl`: profile-driven defaults; README generated from the template via `helm-docs`
      * `charts/`: vendored subchart tarballs (LFS) — `alloy`, `loki`, `thanos`, `alertmanager`, `grafana`, `grafana-operator`, `kube-state-metrics`, `metrics-server`
      * `pre-rendered/`: generated artifacts loaded via `{{ .Files.Get }}`; never hand-edited
        * `dashboards/`: `grafana/` and `datadog/` JSON
        * `pipelines/`: rendered Alloy (`agent.alloy`, `gateway.alloy`)
        * `rules/`: `prometheus/`, `loki/`, `thanos/`
      * `templates/`: provided resources — `alerts/`, `dashboards/`, `pipelines/`, `scrapers/`, plus `grafana-grafana.yaml` and `_helpers.tpl`
      * `examples/`: example values overlays (e.g. existing-grafana, IRSA/S3)
      * `profiles/`: profile value sets (still being defined)
    * `materialize-monitoring-crds/`: CRDs chart (`Chart.yaml`, `values.yaml`, `README.md`)
  * `docs/`: Hugo docsite (the source of this page)
    * `hugo.toml`: site config
    * `content/`: authored Markdown
      * top-level sections: `getting-started/`, `metrics/` (incl. `collecting/`), `logs-and-events/`, `dashboards/` (incl. `grafana/`), `alerting/`, `operating/`
      * `reference/`: `helm/`, `stable-metrics/`, and `internal/` (this section — `dashboard/`, `pipelines/`, `design-docs/`, plus `repo-layout.md`, `roadmap.md`, `releasing.md`, `skills.md`, `helm.md`)
    * `layouts/`, `static/`, `assets/`, `data/`, `i18n/`, `archetypes/`, `themes/`: Hugo machinery
    * `public/`, `resources/`: generated output (not checked in)
  * `legacy/`: preserved field-engineering assets — `sql_exporter/`, `prometheus/`, `grafana/`, `datadog/`, `tests/`, `docker-compose.yml`
  * `tools/`: ancillary ecosystems kept out of `bin/`
    * `chartlib/`: helm-docs templates
    * `grafonnet/`: grafonnet/jsonnet vendoring (`jsonnetfile.json` + lock)
    * `shlib/`: shared bash helpers
  * `.claude/skills/`: authoring conventions consumed by both contributors and AI agents
  * `.github/`: GitHub Actions workflows
