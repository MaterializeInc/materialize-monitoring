# Overview




# Dashboards as Code

Instead of more common ClickOps strategies (manually configuring dashboards in the UI), we manage dashboards as reproducible source code.
Sources live under `packages/dashboards/`.

The audience for this section is **repo contributors** — SRE, Field Engineering, CloudOps, Database Engineers — and AI
agents reading the corresponding `dashboards-as-code` skill.
The audience for the dashboards themselves (panel descriptions, naming, visual choices) is the Materialize end user; see
[Style Guidelines](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/style-guidelines/) for that voice.

## In this section

- **[SDKs and Schemas](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/sdks/)** — Grafana versions we target, Dashboard v1 vs v2 schema state, the
  vendored schemas, and the generated Rust models.
- **[Style Guidelines](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/style-guidelines/)** — palettes, layouts, panel visualization conventions,
  description voice, PromQL conventions, Materialize metric label families, known gotchas, and PromQL recipes.
- **[Generating and Pushing Dashboards](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/generating/)** — code structure, UID determinism, the
  `__main__` entry point, the production push path (`gcx dashboards update`), and the ad-hoc Grafana v2 API path for
  iteration.
- **[Testing](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/testing/)** — testing conventions (currently sparse).

The `dashboards-as-code` Claude skill at `.claude/skills/dashboards-as-code/SKILL.md` is the live state snapshot —
current dashboard inventory, in-flight stubs, and cleanup candidates — and links back into the pages above for stable
reference.

