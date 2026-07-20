---
title: "SDKs and Schemas"
weight: 10
---

# SDKs and Schemas

The Grafana ecosystem has been undergoing major transitions in how dashboard configurations are managed circa 2025; web searches frequently turn up inconsistent or outdated documentation. This page pins down what we target, what we generate, and which SDKs we use to do it.

## Targets

Currently supported:

- **Grafana 13** (Dashboard v2 schema) — latest as of April 2026, primary target
- **Grafana 12** (Dashboard v2beta1 schema)

Planned (stubs are acceptable for now):

- **BEST-EFFORT** Grafana 11 (Dashboard v1 schema)
- **UNSUPPORTED** Datadog

## Ecosystem state

What you need to know to navigate the Grafana SDK landscape:

- **grafonnet** (jsonnet) was the canonical way to do dashboards-as-code through Grafana 11.
- **grafana-foundation-sdk** was introduced for Grafana 12 with backwards-compatible support to Grafana 10. Repository: <https://github.com/grafana/grafana-foundation-sdk/>.
- grafana-foundation-sdk is built on Grafana's [cog codegen framework](https://github.com/grafana/cog/) using cue-based or openapi schemas.
- As of May 2026, grafana-foundation-sdk is not yet fully mature but is usable and ergonomic. Documentation and versioning are messy; **always double-check work against the openapi schemas**.

## Dashboard v1 schema

Dashboard v1 was the schema used in Grafana 10 and 11 (earlier versions did not have a particular schema). Grafana 12 supported v1 by default but had an experimental option to use v2beta1. Dashboard v1 schemas are automatically migrated to v2 in later Grafana versions.

A copy of the v1 openapi schema (generated from cog `61ff0a6055fa48f0c7b105fe4a37af637191314f`, April 9, 2026) is bundled with the dashboards-as-code skill at `.claude/skills/dashboards-as-code/references/dashboard.openapi.json`.

## Dashboard v2 schema

Grafana 12 previewed v2 as `v2beta1` (not the default). Grafana 13 supports v2 by default and is the version we target.

Dashboard v2 cannot be automatically downgraded to v1 inside Grafana; we provide best-effort generation of close v1 dashboards as a second-class output.

Reference schemas (cog `61ff0a6055fa48f0c7b105fe4a37af637191314f`, April 9, 2026):

- v2beta1: `.claude/skills/dashboards-as-code/references/dashboardv2beta1.openapi.json`
- v2: `.claude/skills/dashboards-as-code/references/dashboardv2.openapi.json`

## py-mzmon-lib and Grafana Foundation SDK

For Python dashboard implementations, we use **grafana-foundation-sdk** for most of the codegen surface, and **py-mzmon-lib** (lives at `packages/py-mzmon-lib`, included as a uv workspace) for shared utilities, best practices, and gap-filling patches.

When reaching for an SDK building block, first check what `py-mzmon-lib` already exposes — there are wrappers and helpers for common shapes that aren't covered well by the upstream SDK.

As of May 2026, grafana-foundation-sdk has not yet merged its v2 schema upstream, so some local tweaks may be necessary to get things working with the latest Grafana. Check `py-mzmon-lib`'s shims before adding new compatibility code.
