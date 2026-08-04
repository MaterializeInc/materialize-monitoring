---
title: "Skills"
weight: 20
---

# Skills

`.claude/skills/` holds authoring conventions that are consumed by **both** contributors and AI agents.
They exist so a convention lives in one place rather than being re-explained per reviewer, and so an agent picks up the same opinions a reviewer would apply.

Each skill is a `SKILL.md` with front matter naming when it applies, plus optional `references/`, `assets/`, and `scripts/` beside it.

| Skill | Applies when |
|---|---|
| [`yaml-development`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/yaml-development/SKILL.md) | editing any `.yaml` / `.kyaml` — formatting conventions and the deliberate yamllint relaxations |
| [`chart-development`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/chart-development/SKILL.md) | changing anything under `charts/*` — templates, `values.yaml`, subchart wrapping, profiles, helm-unittest |
| [`platform-development`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/platform-development/SKILL.md) | changing `terraform/`, `test/e2e/`, or the CI gating them — and chart changes with consequences for either |
| [`deployment-operations`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/deployment-operations/SKILL.md) | standing up the stack against a real or local cluster, or diagnosing an unhealthy one |
| [`dashboards-as-code`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/dashboards-as-code/SKILL.md) | authoring Grafana dashboards in `packages/grafana-dashboards` |
| [`pipelines-as-code`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/pipelines-as-code/SKILL.md) | authoring Alloy pipelines in `packages/alloy-pipelines` |

## Related skills outside this repo

The [`agent-skills`](https://github.com/MaterializeInc/agent-skills) repository carries org-wide skills, including one for [`materialize-terraform-self-managed`](https://github.com/MaterializeInc/agent-skills/blob/main/skills/materialize-terraform-self-managed/SKILL.md) — the repo the per-cloud monitoring wrappers live in.

The two are **not synced**, and deliberately so for now: this repo's `platform-development` covers the common module and the chart it installs, while that one covers the deployment repo as a whole. The seam worth watching is the wrapper contract — the `object_storage` shape, the ServiceAccount names its trust policies must match, and the fact that a wrapper pins the module by released tag. If that contract changes here, the downstream skill is the thing most likely to go quietly stale.

## Skills are thin on purpose

A skill routes; it does not duplicate.
Substantive content belongs in these docs — operators hit the same problems contributors do, and a troubleshooting entry only readable inside a skill file helps nobody with a broken cluster.

The two newest follow that split deliberately:

- **`deployment-operations`** is mostly pointers into [o11y Troubleshooting](../../../operating/o11y-troubleshooting/), [Uninstalling](../../../operating/uninstalling/), and [Production Best Practices](../../../operating/production-best-practices/). What it keeps for itself is the *habits* — name the cluster explicitly, assert on recent data rather than any data, treat a green `helm upgrade` as no evidence a change took effect.
- **`platform-development`** keeps the reasoning that has no natural operator-facing home: why the module fans values out at all (Helm cannot template subchart values from a parent), why `terraform validate` proves almost nothing, and the handful of HCL and `yamlencode`/`yamldecode` behaviours that produce valid-but-wrong config.

When a skill starts accumulating prose, that is the signal to move it into a doc and leave a link.
