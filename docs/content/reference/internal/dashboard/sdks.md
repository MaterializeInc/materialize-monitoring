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

### Render context

`mzmon_lib::grafana::context::dashboard_context` is the Grafana-flavored [`TemplateContext`].
The two contexts in `query::render` are extraction-flavored — `doc_context` and `tier_context` fill parameters with recognizable sentinels (`[42m]`, `your-env-name`) because their job is to be parsed, not displayed.
This one resolves parameters to Grafana built-ins (`$__rate_interval`, `$__range`) and dashboard variables, and its enrichment functions do the real catalog joins where the extraction contexts use the identity.

`DashboardScope` carries the deployment-specific values that are *not* dashboard variables: the SQL metric prefix (`mz_` self-managed, `v2_mz_` Cloud — `DashboardScope::cloud()`), the operator and system namespace selectors, and the optional environment-exclusion fragment.

**Any `$variable` a rendered query references must be defined by the dashboard.**
An undefined Grafana variable interpolates to nothing and the selector silently matches no series — a panel that looks correct and is empty.
`REQUIRED_VARIABLES` is the list to assert a dashboard's variable set against.
That is also why the operator and system namespaces are plain values rather than `$…` references: no dashboard defines a variable for either, so referencing one would match nothing.

`NODE_VARIABLES` is separate. The node-exporter families write `instance=~"$nodeList"` literally in their templates (220 occurrences across `node-health.yaml` and `node-debug.yaml`, a convention inherited from the Node Exporter Full dashboard), so no parameter can supply it and a node dashboard has to define it. The pre-rendered `env-top` defines the four in `REQUIRED_VARIABLES` plus `includeSystemClusters`, `metricAdhoc` and the datasource — not `nodeList`.

Those two families need vetting before a dashboard uses them: they were authored separately from the dashboards, so their conventions were not held to the same standard as the Materialize families. Treat their content as unreviewed rather than as a baseline.

### Variable naming

Two conventions, both load-bearing:

- **`*List` marks the interpolation contract**, not multi-select. A `*List` variable is always interpolated as a pattern — write `instance=~"$nodeList"`, never `instance="$nodeList"` — because a single-select variable still renders as an alternation when "All" is chosen, and `=` against `a|b` matches nothing. The failure is silent, which is why the marker is worth the noise.
- **`Name` vs `Id` records which identifier is stable.** `mzClusterList` carries the cluster *id* as its value with the name as display text, because cluster names are neither stable nor unique. `environmentNameList` is the reverse: self-managed Materialize has no cloud organization id at all, so the name is the only stable identifier.

The Rust implementation renamed two variables the Python still spells differently — `environmentIdList` -> `environmentNameList` (the old name was simply wrong: it holds `materialize_cloud_organization_name` values) and `node` -> `nodeList` (the sole pattern variable missing the marker).
Until the Rust takes over rendering, a dashboard rendered by each is not permalink-compatible with the other: the variable name appears in Grafana URLs as `?var-environmentNameList=…`.

Catalog enrichment lives in `mzmon_lib::query::enrich`, beside the renderer rather than under `grafana`, because the joins are plain PromQL shared between the registry's template functions and the dashboards — the same split the Python makes, for the same reason.
Output is byte-identical to `py_mzmon_lib.enrich`, pinned by tests against captured Python output, so the two implementations do not churn against each other.

One gap: **`mzEnvironmentName` is the identity.** The registry declares it and 25 queries use it, but no implementation exists on either side — there is no verified info metric mapping a namespace to an environment name, and the Python never exercised it because its dashboards hand-write their PromQL. Those queries render, and their legends show the namespace rather than a friendly name. This is a stub, not intended behavior.

Note that dashboard-flavored PromQL is deliberately *not* parseable: `[$__rate_interval]` is not a duration. That is precisely why extraction substitutes `[42m]`. So `every_query_renders_through_the_dashboard_context` checks what is checkable instead — that all 207 PromQL queries render, that nothing is left unsubstituted, and that every `$name` in the output is a Grafana built-in or a declared variable.

### Panel API

`mzmon_lib::grafana::panel` builds panels. Its shape comes from an analysis of `env-top.yaml` rather than from porting the Python: of 4667 written fields there, 47% carried no panel-specific information.

Three findings drive it:

- **`kind` discriminants are never authored.** They accounted for ~600 field-writes of pure noise; `Panel::build` sets them.
- **A panel's `options` block belongs to the plugin, not the panel.** `timeseries`, `table`, `gauge` and `barchart` each used exactly one options block across all 69 panels; `piechart` used two and `stat` five, differing only in display knobs. Each plugin gets a preset whose `Default` is the block the baseline used, and only the knobs that varied are exposed. `Panel<O>` is generic over its plugin's options, so `log_scale` on a stat panel does not compile.
- **`fieldConfig` holds the real variation**, and it is a short list — `unit`, `min`, `noValue`, `custom`, `thresholds`, `color`, plus three one-offs. Those are the shared builder methods. `NoValue` names the recurring messages, four of which describe a missing scrape target rather than a filter miss.

Two deviations from the baseline are deliberate, both taken from the load-and-save round trip:

- `reduceOptions` emits `values` as well as `calcs`. The baseline emitted only `calcs`, and Grafana responded by filling the rest in *and* stamping `vizConfig.version` — a migration pass on ten panels every load.
- `gauge`'s `showThresholdMarkers` defaults to `false`. The baseline asked for `true` and Grafana rewrote it to `false` on save, so `true` was never what rendered.

`vizConfig.version` stays `""`: Grafana stamps the running plugin version on load and it is not predictable, so a guess would only be a value the server overwrites.

`panel_presets_reproduce_the_golden_options_blocks` in `grafana_golden.rs` rebuilds all 69 baseline panels through the API and compares options blocks, allowing only the two deviations above. A new divergence fails the test.

### Layout

`mzmon_lib::grafana::layout` owns both halves of a dashboard's panel arrangement.
A v2 dashboard keeps panels in a flat `elements` map and describes their arrangement in a separate `layout` tree that references them by name, so the two have to agree exactly — a reference with no element silently drops a panel, an element nobody references is invisible.
`Layout::assemble` builds both together, which is the only place that can guarantee they line up; it rejects duplicate element names and an empty layout.

`AutoGridLayout` is why there is no partition arithmetic: a `GridLayout` needs an x/y/w/h per panel, while an auto-grid takes a column cap and flows panels into it. Every row of the baseline uses one.

Nesting follows the baseline — tabs of rows of auto-grids, or `Layout::rows` for a dashboard with no tabs. The schema also allows `GridLayout` anywhere and arbitrary re-nesting of rows and tabs; none of that is modelled, since nothing uses it and the generated types remain available.

Two things the layout owns because they are generated rather than authored:

- **Panel ids.** Sequential from `FIRST_PANEL_ID` (1000, matching the baseline's `1000..1068`). `Panel::build` takes an id for direct use, and the assembly overwrites it — only the assembly knows the full set.
- **Nothing else.** Element *names* are authored, not derived: only 14 of the baseline's 69 are slugs of their titles, and the rest are deliberately short stable ids (`availability-percent` for "Environment Availability (Select Time Range)"). Deriving them from titles would couple a permalink-visible identifier to display-text churn.

`the_layout_api_reproduces_the_golden_tree` rebuilds the baseline's whole tree — 6 tabs, 22 rows, 69 placements — through the API and compares it field for field, allowing only `collapse` and `hideHeader`, which are emitted deliberately (Grafana writes `collapse` on save; `hideHeader` has no schema default, so absent and `false` behave identically).

### Query bridge

`mzmon_lib::grafana::query` turns a registry query into the two things a panel needs.
`panel_query(registry, id, ctx)` looks an id up, renders it for the engine the `TemplateContext` names, and returns a `QueryGroupKind` plus a markdown description.

The context's engine picks the datasource: `PromQl` builds Prometheus dataqueries against `${metricsDatasource}`, `LogQl` builds Loki ones against `${logsDatasource}`.
Datadog and Honeycomb render fine but have no Grafana datasource here, so the bridge rejects them rather than emitting an expression that would silently return nothing.

Two details worth knowing:

- **Prometheus compat fields.** Grafana's Prometheus datasource reads `query` and `qryType`, while the schema cog generates calls them `expr` and `queryType`. Sending only the schema spelling loses data on push, so the bridge emits both — matching `py_mzmon_lib.query_v2.CompatPrometheusDataQuery`.
- **Description formatting.** The registry's `Description` is structured (summary / nominal / degraded / unhealthy / notes); the hand-written panels inline "Nominal: …" in flowing prose. The bridge emits the summary in bold followed by the behavioral fields as labeled paragraphs, and unwraps the YAML hard-wrapping. That is one function (`format_description`) if the convention should change.

The registry-to-dashboard path is new: the Python dashboards hand-write their PromQL inline with f-strings rather than going through the registry, so the pre-rendered dashboards are not a byte-level baseline for it.
The context that resolves parameters to Grafana built-ins and dashboard variables is `dashboard_context`, above.

`packages/mzmon-lib/tests/grafana_query_bridge.rs` runs the bridge over every PromQL query in the registry (207 at the time of writing) plus the named availability query in detail.
The registry has no LogQL queries yet, so the Loki path is covered only by unit tests.

`packages/mzmon-lib/tests/grafana_golden.rs` parses the pre-rendered `env-top.yaml` into the generated models and checks that re-serializing loses nothing.
That test is what caught the one place the schemas are narrower than Grafana itself: `MatcherConfig.options` is typed `object`, but Grafana's `byName` matcher takes a bare field-name string, which the golden dashboard emits and Grafana accepts.
The generation script widens it, with the reasoning recorded inline.

## py-mzmon-lib and Grafana Foundation SDK

For Python dashboard implementations, we use **grafana-foundation-sdk** for most of the codegen surface, and **py-mzmon-lib** (lives at `packages/py-mzmon-lib`, included as a uv workspace) for shared utilities, best practices, and gap-filling patches.

When reaching for an SDK building block, first check what `py-mzmon-lib` already exposes — there are wrappers and helpers for common shapes that aren't covered well by the upstream SDK.

As of May 2026, grafana-foundation-sdk has not yet merged its v2 schema upstream, so some local tweaks may be necessary to get things working with the latest Grafana. Check `py-mzmon-lib`'s shims before adding new compatibility code.
