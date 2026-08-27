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
The generated artifacts are checked into grafana-foundation-sdk, one document per package (`dashboardv2`, `timeseries`, `prometheus`, `common`, and so on).
Those documents are the authoritative shape of what Grafana accepts, so we vendor them rather than re-running cog.

Upstream publishes the same 55 documents twice — as OpenAPI 3.0 under `openapi/` and as JSON Schema draft-07 under `jsonschema/`.
We vendor the draft-07 set at `packages/mzmon-lib/schemas/grafana/`.
The type definitions are identical (verified name-for-name and property-for-property across all 55 pairs), draft-07 is what the Rust codegen consumes directly, and it expresses nullability as `anyOf [T, null]` rather than the OpenAPI 3.0 `nullable` keyword that draft-07 tooling ignores.

The one thing only the OpenAPI set carries is each document's `info` block: `x-schema-identifier`, `x-schema-kind`, `x-schema-variant`.
That is load-bearing — the identifier is the Grafana plugin id that a dashboard puts in `VizConfigKind.group` (panels) or `DataQueryKind.group` (dataqueries), and it is not always the document name: `annotationslist` publishes as `annolist`.
So `bin/fetch-grafana-schemas.sh` fetches both renderings, distills those three fields into `packages.json`, and keeps only that.

The vendored `.jsonschema.json` files are byte-identical to upstream, which is why the directory is excluded from the formatters (see `.pre-commit-config.yaml` and `.ecrc`).
Normalization that codegen needs happens in `bin/gen-grafana-models.sh` instead.

The current pin is the grafana-foundation-sdk release tag `v0.0.18` (June 12, 2026), generated there by cog `v0.1.20`.
Renovate maintains the `FSDK_REF` pin in the script via the `bin/*.sh` custom manager, and the `auto-format` workflow re-runs the script on PRs that touch it — Renovate can move the tag but cannot regenerate the vendored tree, so without that step a bump would land with stale schemas.
A `packageRules` entry caps the tag range below `0.1.0`: the upstream repo tags three families off one namespace (`v0.0.x` releases, a parallel `go/v0.0.x` set for the Go module, and `dashboard-converter-v1.0.0`), and `semver-coerced` reads that last one as a 1.0.0 major bump.

Upstream also publishes JSONSchema (draft 2020-12) renderings of the same documents under `jsonschema/`; we do not vendor those.

## Dashboard v1 schema

Dashboard v1 was the schema used in Grafana 10 and 11 (earlier versions did not have a particular schema). Grafana 12 supported v1 by default but had an experimental option to use v2beta1. Dashboard v1 schemas are automatically migrated to v2 in later Grafana versions.

The v1 schema is vendored at `packages/mzmon-lib/schemas/grafana/dashboard.jsonschema.json`.
At the current pin it is generated from Grafana `v11.6.0`'s `public/openapi3.json`, as are the panel and datasource packages (from kind registry `v11.6.x`).

## Dashboard v2 schema

Grafana 12 previewed v2 as `v2beta1` (not the default). Grafana 13 supports v2 by default and is the version we target.

Dashboard v2 cannot be automatically downgraded to v1 inside Grafana; we provide best-effort generation of close v1 dashboards as a second-class output.

Reference schemas, generated at the current pin from Grafana `v13.0.2`'s `apps/dashboard/kinds/` cue sources:

- v2beta1: `packages/mzmon-lib/schemas/grafana/dashboardv2beta1.jsonschema.json`
- v2: `packages/mzmon-lib/schemas/grafana/dashboardv2.jsonschema.json`

## Rust models

`bin/gen-grafana-models.sh` generates Rust types from the vendored schemas into `packages/mzmon-lib/src/grafana/generated/`, using [typify](https://github.com/oxidecomputer/typify) (pure Rust — no Java or Node in the build).
The module is named `generated` rather than `gen` because `gen` is a reserved keyword in Rust edition 2024, which this workspace uses.

Three properties of the upstream schemas shape everything built on top of them:

- **No cross-document `$ref`s.** Every document is self-contained, so shared types are duplicated into each document that uses them: 1285 definitions under 769 distinct names, and 44 of the duplicated names genuinely disagree (`Options` exists in 24 documents with 24 different shapes). A flat namespace is impossible — one Rust module per document is the only mapping that works, which is what the upstream Go and Python SDKs also do. It also means `common` is not worth generating: nothing can `$ref` it, and every panel document already carries its own copy.
- **Panel options are not typed in the dashboard document.** `VizConfigSpec.options` is a free-form map, so a typed `timeseries::Options` has to be serialized into it, paired with the plugin id from `packages.json`.
- **The `*Kind` unions carry no `discriminator`**, even though every variant has a `kind` field with a `const` value. The generated enums are therefore `#[serde(untagged)]` — exact on the write path, but a deserialization failure reports only "data did not match any variant". This is the one place the foundation SDK adds something the schemas do not: hand-written unmarshallers keyed on `kind`.

`PACKAGES` in the generation script is deliberately narrow — generating all 53 usable documents costs ~92k lines of mostly redundant copies. It currently tracks the six panel plugins `env-top.yaml` renders, plus the log panel and Loki dataquery for log dashboards.

Two documents cannot be generated at all: `alerting` (typify panics deriving variant names for the `MatchType` enum `["=", "!=", "=~", "!~"]`) and, without normalization, `table` (cog emits a `default` that violates its own schema — `Options.footer` defaults `reducer` to null where `TableFooterOptions` requires an array).

`packages/mzmon-lib/tests/grafana_golden.rs` parses the pre-rendered `env-top.yaml` into the generated models and checks that re-serializing loses nothing.
That test is what caught the one place the schemas are narrower than Grafana itself: `MatcherConfig.options` is typed `object`, but Grafana's `byName` matcher takes a bare field-name string, which the golden dashboard emits and Grafana accepts.
The generation script widens it, with the reasoning recorded inline.

## py-mzmon-lib and Grafana Foundation SDK

For Python dashboard implementations, we use **grafana-foundation-sdk** for most of the codegen surface, and **py-mzmon-lib** (lives at `packages/py-mzmon-lib`, included as a uv workspace) for shared utilities, best practices, and gap-filling patches.

When reaching for an SDK building block, first check what `py-mzmon-lib` already exposes — there are wrappers and helpers for common shapes that aren't covered well by the upstream SDK.

As of May 2026, grafana-foundation-sdk has not yet merged its v2 schema upstream, so some local tweaks may be necessary to get things working with the latest Grafana. Check `py-mzmon-lib`'s shims before adding new compatibility code.
