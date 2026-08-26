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

## Vendored schemas

cog is the codegen framework; it does not itself publish the Grafana resource schemas.
The generated artifacts are checked into grafana-foundation-sdk under `openapi/`, one document per package (`dashboardv2`, `timeseries`, `prometheus`, `common`, and so on).
Those documents are the authoritative shape of what Grafana accepts, so we vendor them rather than re-running cog.

The full set of 55 documents lives at `packages/mzmon-lib/schemas/grafana/`, with a `PROVENANCE.md` recording the pin.
`bin/fetch-grafana-schemas.sh` re-vendors them; that tree is the single copy, and the dashboards-as-code skill points at it rather than bundling its own.

The current pin is the grafana-foundation-sdk release tag `v0.0.18` (June 12, 2026), generated there by cog `v0.1.20`.
Renovate maintains the `FSDK_REF` pin in the script via the `bin/*.sh` custom manager, and the `auto-format` workflow re-runs the script on PRs that touch it — Renovate can move the tag but cannot regenerate the vendored tree, so without that step a bump would land with stale schemas.
A `packageRules` entry caps the tag range below `0.1.0`: the upstream repo tags three families off one namespace (`v0.0.x` releases, a parallel `go/v0.0.x` set for the Go module, and `dashboard-converter-v1.0.0`), and `semver-coerced` reads that last one as a 1.0.0 major bump.

Upstream also publishes JSONSchema (draft 2020-12) renderings of the same documents under `jsonschema/`; we do not vendor those.

## Dashboard v1 schema

Dashboard v1 was the schema used in Grafana 10 and 11 (earlier versions did not have a particular schema). Grafana 12 supported v1 by default but had an experimental option to use v2beta1. Dashboard v1 schemas are automatically migrated to v2 in later Grafana versions.

The v1 openapi schema is vendored at `packages/mzmon-lib/schemas/grafana/dashboard.openapi.json`.
At the current pin it is generated from Grafana `v11.6.0`'s `public/openapi3.json`, as are the panel and datasource packages (from kind registry `v11.6.x`).

## Dashboard v2 schema

Grafana 12 previewed v2 as `v2beta1` (not the default). Grafana 13 supports v2 by default and is the version we target.

Dashboard v2 cannot be automatically downgraded to v1 inside Grafana; we provide best-effort generation of close v1 dashboards as a second-class output.

Reference schemas, generated at the current pin from Grafana `v13.0.2`'s `apps/dashboard/kinds/` cue sources:

- v2beta1: `packages/mzmon-lib/schemas/grafana/dashboardv2beta1.openapi.json`
- v2: `packages/mzmon-lib/schemas/grafana/dashboardv2.openapi.json`

## py-mzmon-lib and Grafana Foundation SDK

For Python dashboard implementations, we use **grafana-foundation-sdk** for most of the codegen surface, and **py-mzmon-lib** (lives at `packages/py-mzmon-lib`, included as a uv workspace) for shared utilities, best practices, and gap-filling patches.

When reaching for an SDK building block, first check what `py-mzmon-lib` already exposes — there are wrappers and helpers for common shapes that aren't covered well by the upstream SDK.

As of May 2026, grafana-foundation-sdk has not yet merged its v2 schema upstream, so some local tweaks may be necessary to get things working with the latest Grafana. Check `py-mzmon-lib`'s shims before adding new compatibility code.
