---
title: "Dashboard"
weight: 30
bookCollapseSection: true
---

# Dashboards as Code

Instead of more common ClickOps strategies (manually configuring dashboards in the UI), we manage dashboards as reproducible source code. Sources live under `packages/grafana-dashboards/`.

The audience for this section is **repo contributors** — SRE, Field Engineering, CloudOps, Database Engineers — and AI agents reading the corresponding `dashboards-as-code` skill. The audience for the dashboards themselves (panel descriptions, naming, visual choices) is the Materialize end user; see [Style Guidelines]({{< relref "style-guidelines.md" >}}) for that voice.

## In this section

- **[SDKs and Schemas]({{< relref "sdks.md" >}})** — Grafana versions we target, Dashboard v1 vs v2 schema state, and the `grafana-foundation-sdk` / `py-mzmon-lib` toolchain.
- **[Style Guidelines]({{< relref "style-guidelines.md" >}})** — palettes, layouts, panel visualization conventions, description voice, PromQL conventions, Materialize metric label families, known gotchas, and PromQL recipes.
- **[Generating and Pushing Dashboards]({{< relref "generating.md" >}})** — code structure, UID determinism, the `__main__` entry point, the production push path (`gcx dashboards update`), and the ad-hoc Grafana v2 API path for iteration.
- **[Testing]({{< relref "testing.md" >}})** — testing conventions (currently sparse).

The `dashboards-as-code` Claude skill at `.claude/skills/dashboards-as-code/SKILL.md` is the live state snapshot — current dashboard inventory, in-flight stubs, and cleanup candidates — and links back into the pages above for stable reference.
