---
title: "Pipelines"
weight: 40
bookCollapseSection: true
---

# Pipelines as Code

Instead of hand-maintained alloy config files, we author pipeline definitions as YAML, validate them against a project-owned JSONSchema, and render them to `config.alloy` via the `mz-monitoring-build gen-pipelines` subcommand. Sources live under `packages/alloy-pipelines/`; schemas under `packages/mzmon-lib/schemas/alloy/`; the Rust renderer and validator live under `packages/mzmon-lib/`.

The audience for this section is **repo contributors** — SRE, Field Engineering, CloudOps, Database Engineers — and AI agents reading the corresponding `pipelines-as-code` skill. The pipelines themselves target machines (alloy), not end users, so the conventions below favor *contributor* ergonomics: IDE autocomplete, schema-driven docs, escape hatches when reality outpaces the typed model.

## In this section

- **[Authoring]({{< relref "authoring.md" >}})** — schema model, strict-attributes policy, when to use `raw:` vs. extend the schema, how to add a new typed component or stage.
- **[Logging]({{< relref "logging.md" >}})** — log pipeline conventions, label families, retention.
- **[Metrics]({{< relref "metrics.md" >}})** — metrics pipeline conventions.

The `pipelines-as-code` Claude skill at `.claude/skills/pipelines-as-code/SKILL.md` is the live state snapshot — current pipeline inventory, in-flight stubs, schema gaps — and links back into the pages above for stable reference.
