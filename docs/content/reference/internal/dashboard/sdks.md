---
title: "SDKs and Schemas"
weight: 10
---

# SDKs and Schemas

The Grafana ecosystem has been undergoing major transitions in how dashboard configurations are managed circa 2025;
web searches frequently turn up inconsistent or outdated documentation.
This page pins down what we target, what we generate, and which SDKs we use to do it.

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
  The original plan here used it, and a vendored `tools/grafonnet/` tree existed for a while, but no dashboard was
  ever authored against it — both are gone.
- **grafana-foundation-sdk** was introduced for Grafana 12 with backwards-compatible support to Grafana 10.
  Repository: <https://github.com/grafana/grafana-foundation-sdk/>.
- grafana-foundation-sdk is built on Grafana's [cog codegen framework](https://github.com/grafana/cog/) using cue-based or openapi schemas.
- As of May 2026, grafana-foundation-sdk is not yet fully mature but is usable and ergonomic.
  Documentation and versioning are messy; **always double-check work against the openapi schemas**.

## Vendored schemas

cog is the codegen framework; it does not itself publish the Grafana resource schemas.
The generated artifacts are checked into grafana-foundation-sdk, one document per package (`dashboardv2`,
`timeseries`, `prometheus`, `common`, and so on).
Those documents are the authoritative shape of what Grafana accepts, so we vendor them rather than re-running cog.

Upstream publishes the same 55 documents twice — as OpenAPI 3.0 under `openapi/` and as JSON Schema draft-07 under `jsonschema/`.
We vendor the draft-07 set at `packages/mzmon-lib/schemas/grafana/`.
The type definitions are identical (verified name-for-name and property-for-property across all 55 pairs), draft-07 is
what the Rust codegen consumes directly, and it expresses nullability as `anyOf [T, null]` rather than the OpenAPI 3.0
`nullable` keyword that draft-07 tooling ignores.

The one thing only the OpenAPI set carries is each document's `info` block: `x-schema-identifier`, `x-schema-kind`, `x-schema-variant`.
That is load-bearing — the identifier is the Grafana plugin id that a dashboard puts in `VizConfigKind.group` (panels)
or `DataQueryKind.group` (dataqueries), and it is not always the document name:
`annotationslist` publishes as `annolist`.
So `bin/fetch-grafana-schemas.sh` fetches both renderings, distills those three fields into `packages.json`, and keeps only that.

The vendored `.jsonschema.json` files are byte-identical to upstream, which is why the directory is excluded from the
formatters (see `.pre-commit-config.yaml` and `.ecrc`).
Normalization that codegen needs happens in `bin/gen-grafana-models.sh` instead.

The current pin is the grafana-foundation-sdk release tag `v0.0.18` (June 12, 2026), generated there by cog `v0.1.20`.
Renovate maintains the `FSDK_REF` pin in the script via the `bin/*.sh` custom manager, and the `auto-format` workflow
re-runs the script on PRs that touch it — Renovate can move the tag but cannot regenerate the vendored tree, so
without that step a bump would land with stale schemas.
A `packageRules` entry caps the tag range below `0.1.0`: the upstream repo tags three families off one namespace
(`v0.0.x` releases, a parallel `go/v0.0.x` set for the Go module, and `dashboard-converter-v1.0.0`), and
`semver-coerced` reads that last one as a 1.0.0 major bump.

Upstream also publishes JSONSchema (draft 2020-12) renderings of the same documents under `jsonschema/`; we do not vendor those.

## Dashboard v1 schema

Dashboard v1 was the schema used in Grafana 10 and 11 (earlier versions did not have a particular schema).
Grafana 12 supported v1 by default but had an experimental option to use v2beta1.
Dashboard v1 schemas are automatically migrated to v2 in later Grafana versions.

The v1 schema is vendored at `packages/mzmon-lib/schemas/grafana/dashboard.jsonschema.json`.
At the current pin it is generated from Grafana `v11.6.0`'s `public/openapi3.json`, as are the panel and datasource
packages (from kind registry `v11.6.x`).

## Dashboard v2 schema

Grafana 12 previewed v2 as `v2beta1` (not the default). Grafana 13 supports v2 by default and is the version we target.

Dashboard v2 cannot be automatically downgraded to v1 inside Grafana; we provide best-effort generation of close v1
dashboards as a second-class output.

Reference schemas, generated at the current pin from Grafana `v13.0.2`'s `apps/dashboard/kinds/` cue sources:

- v2beta1: `packages/mzmon-lib/schemas/grafana/dashboardv2beta1.jsonschema.json`
- v2: `packages/mzmon-lib/schemas/grafana/dashboardv2.jsonschema.json`

## Rust models

`bin/gen-grafana-models.sh` generates Rust types from the vendored schemas into
`packages/mzmon-lib/src/grafana/generated/`, using [typify](https://github.com/oxidecomputer/typify) (pure Rust — no
Java or Node in the build).
The module is named `generated` rather than `gen` because `gen` is a reserved keyword in Rust edition 2024, which this workspace uses.

Three properties of the upstream schemas shape everything built on top of them:

- **No cross-document `$ref`s.** Every document is self-contained, so shared types are duplicated into each document
  that uses them: 1285 definitions under 769 distinct names, and 44 of the duplicated names genuinely disagree
  (`Options` exists in 24 documents with 24 different shapes).
  A flat namespace is impossible — one Rust module per document is the only mapping that works.
  That is what the upstream Go and Python SDKs do too.
  It also means `common` is not worth generating: nothing can `$ref` it, and every panel document already carries its own copy.
- **Panel options are not typed in the dashboard document.** `VizConfigSpec.options` is a free-form map, so a typed
  `timeseries::Options` has to be serialized into it, paired with the plugin id from `packages.json`.
- **The `*Kind` unions carry no `discriminator`**, even though every variant has a `kind` field with a `const` value.
  The generated enums are therefore `#[serde(untagged)]` — exact on the write path, but a deserialization failure
  reports only "data did not match any variant".
  This is the one place the foundation SDK adds something the schemas do not: hand-written unmarshallers keyed on `kind`.

`PACKAGES` in the generation script is deliberately narrow — generating all 53 usable documents costs ~92k lines of mostly redundant copies.
It currently tracks the six panel plugins `env-top.yaml` renders, plus the log panel and Loki dataquery for log dashboards.

Two documents cannot be generated at all:
`alerting` (typify panics deriving variant names for the `MatchType` enum `["=", "!=", "=~", "!~"]`) and, without
normalization, `table` (cog emits a `default` that violates its own schema — `Options.footer` defaults `reducer` to
null where `TableFooterOptions` requires an array).

### The dashboards crate

`packages/dashboards` (`mz-dashboards`) holds the dashboards themselves, one module per backend under `src/`.
`grafana/` is the only one today; a Datadog or Google Cloud Monitoring backend belongs beside it rather than inside
it, because little is genuinely shared — the queries differ per engine, and each backend's SDK has its own panel and
layout model.
What *is* shared lives in the query registry: a query defined once renders to PromQL, LogQL, or a Datadog metric query through `mzmon_lib::query`.

Each dashboard owns two coordination modules:

- **`theme.rs`** — the per-tab colours, in one file.
  Every tab takes one colour from the *qualitative* palette (no ordering, and deliberately free of red, since red reads
  as a health verdict and a tab's identity is not one).
  Keeping the assignment central is what makes the scheme reviewable and a recolouring one edit.
  The Summary tab is deliberately shadeless: its panels are pointers into other tabs, so each borrows the shade of the
  tab it points at — `theme::KUBERNETES.shade` rather than a hex literal, so it stays correct if Kubernetes is ever
  recoloured.
- **`selector.rs`** — the PromQL fragments that reference dashboard variables, so no panel query names one directly.
  A variable rename touches one function instead of sixty expressions, and a typo cannot introduce an undefined variable
  because the names come from `context::variables`.

`tests/env_top_parity.rs` holds the port to the baseline.
All 69 panels are ported, and it checks every panel's title, description, plugin, unit and queries against the
pre-rendered YAML, printing per-tab coverage.
Deliberate divergences are enumerated in two lists rather than scattered through assertions, and the description list
is checked in both directions — an entry that no longer diverges fails too, so it cannot go stale.

Six of those entries fix **broken cross-references in the baseline**.
Panel descriptions navigate by naming a destination as `_Where -> What_`, where `Where` is a tab or a row.
The baseline points at `_Storage -> Sources_`, `_Storage Objects -> Sink Throughput_` and `_Compute -> Freshness_`,
none of which is a tab — they are `Sources and Sinks` and `Compute Objects`.
It reads like a tab rename that never reached the prose.
`no_description_points_somewhere_that_does_not_exist` validates the arrow form against the tab titles `theme.rs`
declares plus every row title in the built layout, which is only possible because those titles live in one place.

### Dashboard shell

`mzmon_lib::grafana::dashboard::Dashboard` is the last layer: a title, a `Layout`, and a variable set produce either a
bare `dashboardv2::Dashboard` (what the Grafana HTTP API takes) or the Kubernetes-style `Resource` the chart deploys.

`build_spec` runs the check the variable-naming work was for:
**every `$variable` a panel references must be defined by the dashboard**.
An undefined Grafana variable interpolates to nothing, so the selector matches no series and the panel renders empty while looking healthy.
Grafana's own built-ins (`$__rate_interval`, `$__range`, …) are exempt, and `label_replace` capture groups (`$1`) are not variables.
Only panels are checked — a variable's own query legitimately references its predecessors, and
`variable::environment_scoped` checks that chain itself.

Three fields come from the load-and-save round trip, emitted so a UI save diffs clean:
`liveNow: false`, the built-in `Annotations & Alerts` query (Grafana adds it to any dashboard that has none), and `cursorSync`.

**`cursorSync` defaults to `Crosshair`, not Grafana's `Off`.** The baseline left it `Off`, which on a dashboard whose
purpose is correlating across panels — "did memory climb when the lag did?" — means every comparison is done by eye
against two independently-hovered charts.
`Tooltip` adds every panel's tooltip at once, which suits a small focused dashboard rather than a 28-panel tab.

`metadata.annotations` carries the `monitoring.materialize.cloud/*` hints the docsite shortcode reads.
Grafana replaces them on a UI save, so they cannot gate anything without a reconcile step.

### Variables

`mzmon_lib::grafana::variable` ports `dashboards.variables`.
`environment_scoped(sql_metric_prefix, multi)` returns the standard chained set in dependency order — datasource,
environment, namespace, system-cluster toggle, cluster, replica, ad-hoc filter — each query narrowing the last.

Two details worth knowing.
The discovery queries read `mz_compute_commands_total`, genuine instrumentation present in every deployment and
therefore never SQL-prefixed; only the cluster variable reads the SQL-derived `compute_cluster_status` and so must
match the panels' prefix.
And `clusters` uses `query_result` rather than `label_values` so the regex can put `compute_cluster_id` in the value
and `compute_cluster_name` in the display text — Grafana sorts labels alphabetically, and `compute_cluster_id` sorts
before `compute_cluster_name`, which is what makes that regex stable.

Tests assert the set defines every `REQUIRED_VARIABLES` entry, that the chain references only names the set defines,
and that the prefix lands on the cluster query alone.

### Rendering

`mz-monitoring-build gen-dashboards` is the entrypoint.
`packages/dashboards/src/grafana/render.rs` does the work; `packages/dashboards/src/grafana/mod.rs` holds the registry
of what can be rendered.

Two consumers, one command:

| Output | Consumer | Make target |
| --- | --- | --- |
| `--format yaml` into `charts/…/pre-rendered/dashboards/grafana/` | the Helm chart, which globs by filename stem | `charts/…/dashboards/grafana` |
| `--format json` into `docs/assets/dashboards/grafana/` | the docsite, which offers the file for download | `docs/assets/dashboards/grafana` |

Both trees are checked in, which makes byte-stability the property that matters most.
Every render therefore goes through `render::canonical` first, which does two things:

- **Sorts every object's keys.** This reaches further than choosing an ordered map for the generated structs: the
  free-form `options` and `spec` blocks are `serde_json::Map`, whose order under this crate's `preserve_order` is
  insertion order — which is whatever the panel builder happened to do.
- **Writes whole-valued floats as integers.** The schemas type most numerics as `number`, so typify gives them `f64`,
  and an `f64` serializes as `3.0` even when it holds a count. `maxColumnCount: 3.0` reads as a mistake.

`--queries-dir` points at the query registry (default `packages/queries`), matching the sibling `gen-*` commands:
the registry is a source input, not a lookup path.

`--check` renders and compares against what is on disk without writing, exiting non-zero if they differ.
The `dashboards` workflow runs `make -B dashboards` and then asserts `git status` is clean, so a change to a panel or
to the generator cannot merge with stale output.
`-B` matters: the Makefile targets are the output *directories*, and a fresh checkout's mtimes are arbitrary, so a
plain `make` can consider them up to date and skip rendering — which would make the freshness check vacuous.
Deleting the outputs first does not fix it either, because removing files inside a directory target updates that
directory's mtime, making it look newer than its prerequisites rather than missing.

#### Cloud variants (retired)

There is one render per dashboard, and nothing left in the CLI that selects a cloud.

The Python branched on `cloud_hint` in eleven panels, because GKE's *managed* cAdvisor and kube-state-metrics shipped a
reduced metric allowlist that omitted the container limit and spec series:

- Two panels had no denominator for a percent-of-limit gauge, so `cpu-usage-current` and `memory-usage-current` fell
  back to absolute cores and bytes — a different panel type (stat, not gauge), title, unit and threshold ladder.
- Nine more swapped their `no_value` text for one naming the GKE gap.

The gap is closed: `alloy-gateway` scrapes `/metrics/cadvisor` on every kubelet directly rather than consuming GKE's
subset, and every `container_*` series these queries reference is present — see
[Scraping]({{< relref "../../../metrics/scraping.md" >}}#kubelet).
So the fallbacks are obsolete, and GCP gets the percent-of-limit gauges like everywhere else.

Once the panels converged, the `gcp-` artifact recorded nothing but its own name in a
`monitoring.materialize.cloud/target-cloud` annotation, so it and the machinery behind it — the `Cloud` enum, the
`cloud` render option, and the `--cloud` / `--prefix` flags — were removed.
`render.rs` keeps a test asserting no dashboard emits that annotation, so a half-reverted reintroduction is caught.
If a cloud ever needs different panels again, the branch goes back in `env_top::render`, which says so.

<!-- One nuance if this comes up again: the kubelet scrape covers the cAdvisor series
(`container_spec_cpu_quota`, `container_spec_memory_limit_bytes`, `container_start_time_seconds`). The CPU gauge's
denominator, `kube_pod_container_resource_limits`, is a kube-state-metrics series, which the chart discovers via
monitors rather than scraping itself. -->

### Parity against the Python

`packages/dashboards/tests/env_top_parity.rs` compares the port against
`packages/mzmon-lib/tests/fixtures/env-top.python-baseline.yaml` — the last render the Python produced, frozen.

It is a fixture rather than a read of the checked-in artifact because the Rust generator now writes that file.
Comparing the output to itself would make every assertion vacuous, and would *invert* the ones that assert a
deliberate divergence: those started failing the moment the switchover landed, which is how the problem surfaced.
`mzmon-lib`'s `grafana_golden.rs` reads the same fixture, for the different reason that it wants a real Dashboard v2
document to round-trip through the generated models.

Six checks, each covering a class the others cannot see:

| Check | Why it is separate |
| --- | --- |
| titles | catches invented prose — it found eight fabricated panel titles, and a row titled from nothing |
| plugin and unit | a panel can carry the right title and render as the wrong type |
| queries (whitespace-collapsed) | the expressions are hand-indented, so exact newlines would test the formatter |
| transformations, options included | invisible to every check above; its absence dropped an `extractFields` step a data link depended on |
| the tab/row skeleton | the panel checks say nothing about the frame; its absence let a row ship under an invented title, at the wrong column cap |
| the variable set | a query naming an undefined variable interpolates to nothing and matches no series — a panel that looks right and is empty |

Descriptions are no longer compared against the baseline: they come from the registry, which carries better prose than
the baseline did, so `descriptions_come_from_the_registry` asserts every panel's description *is* some registry query's
rendered text instead.

Query divergences from the baseline are allow-listed with a reason. Switching the panels to registry queries surfaced
four real disagreements, all fixed in `packages/queries/` rather than worked around:

| Fix | Was |
| --- | --- |
| `max without (job)` restored on 14 storage/compute rates | a target scraped by two jobs was counted twice |
| regex-escaped list params in the pod-name matchers | the bare variable interpolates a multi-value as a glob, meaningless in a regex |
| `capacity.all_containers` queries added | only the exporter-*excluding* form existed, so the Kubernetes tab stopped including it |
| `mzObjectName` added to `arrangements.records.system` | system collections rendered with a blank `{{name}}` legend; `.user` already had the join |

Also `rangeWindow`, the bare (unbracketed) range: `range` carries its own brackets, so `%%{range}:1m` cannot express a
subquery. `materialize.info.max_lag` carried a FIXME asking for exactly this and hardcoded `1h`; it now follows the
caller's window, which in a dashboard is the panel's own time picker.

What remains allow-listed is genuinely equivalent, not deferred: the `orZero` helper parenthesizes its operand,
quantiles are written `0.50` rather than `0.5`, `topk` sits inside the enrichment's disjoint `unless on (…)` branches
(so their union is exactly the topk set), and one `max by (source_id)` already subsumes the job dedup.

Divergences are allow-listed at the top of the file with the reason, and the allow-list is checked in both directions —
an entry that no longer diverges fails too, so the list cannot rot into a set of stale excuses.

### End to end

`packages/mzmon-lib/tests/grafana_end_to_end.rs` builds a complete dashboard through every layer — registry, context,
bridge, panel presets, thresholds, layout, shell — and deserializes the result back through the generated models.
Since those carry `deny_unknown_fields`, anything invented along the way fails there rather than at push time.
It also checks both output shapes (enveloped resource and bare spec), referential integrity between `elements` and the
layout, sequential panel ids, and that the round-trip fields are present.

### Render context

`mzmon_lib::grafana::context::dashboard_context` is the Grafana-flavored [`TemplateContext`].
The two contexts in `query::render` are extraction-flavored — `doc_context` and `tier_context` fill parameters with
recognizable sentinels (`[42m]`, `your-env-name`) because their job is to be parsed, not displayed.
This one resolves parameters to Grafana built-ins (`$__rate_interval`, `$__range`) and dashboard variables, and its
enrichment functions do the real catalog joins where the extraction contexts use the identity.

`DashboardScope` carries the deployment-specific values that are *not* dashboard variables: the SQL metric prefix
(`mz_` self-managed, `v2_mz_` Cloud — `DashboardScope::cloud()`), the operator and system namespace selectors, and the
optional environment-exclusion fragment.

**Any `$variable` a rendered query references must be defined by the dashboard.**
An undefined Grafana variable interpolates to nothing and the selector silently matches no series — a panel that looks correct and is empty.
`REQUIRED_VARIABLES` is the list to assert a dashboard's variable set against.
That is also why the operator and system namespaces are plain values rather than `$…` references: no dashboard defines
a variable for either, so referencing one would match nothing.

`NODE_VARIABLES` is separate.
The node-exporter families write `instance=~"$nodeList"` literally in their templates (220 occurrences across
`node-health.yaml` and `node-debug.yaml`, a convention inherited from the Node Exporter Full dashboard), so no
parameter can supply it and a node dashboard has to define it.
The pre-rendered `env-top` defines the four in `REQUIRED_VARIABLES` plus `includeSystemClusters`, `metricAdhoc` and the datasource — not `nodeList`.

Those two families need vetting before a dashboard uses them: they were authored separately from the dashboards, so
their conventions were not held to the same standard as the Materialize families.
Treat their content as unreviewed rather than as a baseline.

### Variable naming

Two conventions, both load-bearing:

- **`*List` marks the interpolation contract**, not multi-select.
  A `*List` variable is always interpolated as a pattern — write `instance=~"$nodeList"`, never `instance="$nodeList"` —
  because a single-select variable still renders as an alternation when "All" is chosen, and `=` against `a|b` matches
  nothing.
  The failure is silent, which is why the marker is worth the noise.
- **`Name` vs `Id` records which identifier is stable.** `mzClusterList` carries the cluster *id* as its value with the
  name as display text, because cluster names are neither stable nor unique.
  `environmentNameList` is the reverse: self-managed Materialize has no cloud organization id at all, so the name is the only stable identifier.

The Rust implementation renamed two variables the Python still spells differently — `environmentIdList` ->
`environmentNameList` (the old name was simply wrong: it holds `materialize_cloud_organization_name` values) and
`node` -> `nodeList` (the sole pattern variable missing the marker).
Until the Rust takes over rendering, a dashboard rendered by each is not permalink-compatible with the other: the
variable name appears in Grafana URLs as `?var-environmentNameList=…`.

Catalog enrichment lives in `mzmon_lib::query::enrich`, beside the renderer rather than under `grafana`, because the
joins are plain PromQL shared between the registry's template functions and the dashboards — the same split the Python
makes, for the same reason.
Output is byte-identical to the Python `enrich` it was ported from, pinned by tests against captured output from it, so the two
implementations do not churn against each other.

One gap:
**`mzEnvironmentName` is the identity.** The registry declares it and 25 queries use it, but no implementation exists
on either side — there is no verified info metric mapping a namespace to an environment name, and the Python never
exercised it because its dashboards hand-write their PromQL.
Those queries render, and their legends show the namespace rather than a friendly name.
This is a stub, not intended behavior.

Note that dashboard-flavored PromQL is deliberately *not* parseable:
`[$__rate_interval]` is not a duration.
That is precisely why extraction substitutes `[42m]`.
So `every_query_renders_through_the_dashboard_context` checks what is checkable instead — that all 207 PromQL queries
render, that nothing is left unsubstituted, and that every `$name` in the output is a Grafana built-in or a declared
variable.

### Panel API

`mzmon_lib::grafana::panel` builds panels.
Its shape comes from an analysis of `env-top.yaml` rather than from porting the Python: of 4667 written fields there,
47% carried no panel-specific information.

Three findings drive it:

- **`kind` discriminants are never authored.** They accounted for ~600 field-writes of pure noise; `Panel::build` sets them.
- **A panel's `options` block belongs to the plugin, not the panel.** `timeseries`, `table`, `gauge` and `barchart` each
  used exactly one options block across all 69 panels; `piechart` used two and `stat` five, differing only in display
  knobs.
  Each plugin gets a preset whose `Default` is the block the baseline used, and only the knobs that varied are exposed.
  `Panel<O>` is generic over its plugin's options, so `log_scale` on a stat panel does not compile.
- **`fieldConfig` holds the real variation**, and it is a short list.
  `unit`, `min`, `noValue`, `custom`, `thresholds`, `color`, plus three one-offs.
  Those are the shared builder methods.
  `NoValue` names the recurring messages, four of which describe a missing scrape target rather than a filter miss.

Two deviations from the baseline are deliberate, both taken from the load-and-save round trip:

- `reduceOptions` emits `values` as well as `calcs`.
  The baseline emitted only `calcs`, and Grafana responded by filling the rest in *and* stamping `vizConfig.version` — a
  migration pass on ten panels every load.
- `gauge`'s `showThresholdMarkers` defaults to `false`.
  The baseline asked for `true` and Grafana rewrote it to `false` on save, so `true` was never what rendered.

`vizConfig.version` stays `""`:
Grafana stamps the running plugin version on load and it is not predictable, so a guess would only be a value the server overwrites.

`panel_presets_reproduce_the_golden_options_blocks` in `grafana_golden.rs` rebuilds all 69 baseline panels through the
API and compares options blocks, allowing only the two deviations above.
A new divergence fails the test.

### Thresholds and palettes

`mzmon_lib::grafana::palette` and `mzmon_lib::grafana::threshold` port `dashboards.palette` and `dashboards.threshold`.
Five ladder generators — `health`, `utilization`, `errors`, `load`, `stability` — plus `health_mapping` for the
value-mapping equivalent, and `Ladder` for hand-built ladders.

**The base step is the substantive change.** Grafana's first threshold step *is* the base: it colours everything below
the second step and its own value is ignored.
The schema says so — `Threshold.value` is nullable, documented as "Value null means -Infinity" — and the load-and-save
round trip confirms it, Grafana rewriting whatever first value it is given to `0`.

The Python does not model that.
Only `health_thresholds` supplies a base, as `-2147483647`; the other four generators emit their first real threshold
as step zero, so Grafana promotes it to the base and its colour bleeds down over everything beneath.
On the baseline's error column — authored as `1 -> light orange` so that "non-zero jumps out visually" — that means
**zero errors renders in the first error colour**.

So every ladder emits an explicit base with `value: None`:

- `health`, `load`, `stability`: the base repeats the first band, so rendering is unchanged and the step merely becomes honest.
- `errors`: the base is the healthy colour, so a count below `min_errors` is no longer coloured as errors.
- `utilization`: the base is the palette's low colour, so the region below `min_value` no longer inherits the `min_value` band.

`Ladder::base` overrides it.
Grafana normalises `None` back to `0` on save, so a UI round trip shows one changed field per ladder — cosmetic, and worth it:
`0` is a real boundary for a metric that can go negative, `None` is not.

**`errors` and `load` were respaced.** The Python divided the range by the colour *count* rather than by the number of
gaps, so the top band opened at `min + (n-1)/n * (max - min)` and `max` was never reached:
`errors(1, 100)` topped out at 80.2, and `load(0, 1)` at 0.909.
That contradicts `error_thresholds`' own docstring ("how many errors for the highest color").
Dividing by the gaps instead puts the last colour on `max`, and gives `load` clean tenths.

That moves four of the baseline's ladders — both `load` panels and the two `errors(1, 10)` sink panels.
The golden test does not simply allowlist them: a respaced ladder has to keep the golden's colour sequence exactly,
and its step gap has to exceed the golden's by precisely `n / (n - 1)` with every step following from that gap, so any
other drift still fails.

One quirk is preserved:
`errors(1, 1)` yields five steps all at value 1, which the baseline's `sources-errors` panel relies on to mean "any error is the worst colour".

`threshold_generators_reproduce_the_golden_ladders` checks all 9 ladders the baseline carries against the generator
calls the Python dashboards make, comparing steps above the base.

### Layout

`mzmon_lib::grafana::layout` owns both halves of a dashboard's panel arrangement.
A v2 dashboard keeps panels in a flat `elements` map and describes their arrangement in a separate `layout` tree that
references them by name, so the two have to agree exactly — a reference with no element silently drops a panel, an
element nobody references is invisible.
`Layout::assemble` builds both together, which is the only place that can guarantee they line up; it rejects duplicate
element names and an empty layout.

`AutoGridLayout` is why there is no partition arithmetic: a `GridLayout` needs an x/y/w/h per panel, while an
auto-grid takes a column cap and flows panels into it.
Every row of the baseline uses one.

Nesting follows the baseline — tabs of rows of auto-grids, or `Layout::rows` for a dashboard with no tabs.
The schema also allows `GridLayout` anywhere and arbitrary re-nesting of rows and tabs; none of that is modelled,
since nothing uses it and the generated types remain available.

Two things the layout owns because they are generated rather than authored:

- **Panel ids.** Sequential from `FIRST_PANEL_ID` (1000, matching the baseline's `1000..1068`).
  `Panel::build` takes an id for direct use, and the assembly overwrites it — only the assembly knows the full set.
- **Nothing else.** Element *names* are authored, not derived: only 14 of the baseline's 69 are slugs of their titles,
  and the rest are deliberately short stable ids (`availability-percent` for "Environment Availability (Select Time
  Range)").
  Deriving them from titles would couple a permalink-visible identifier to display-text churn.

`the_layout_api_reproduces_the_golden_tree` rebuilds the baseline's whole tree — 6 tabs, 22 rows, 69 placements —
through the API and compares it field for field, allowing only `collapse` and `hideHeader`, which are emitted
deliberately (Grafana writes `collapse` on save; `hideHeader` has no schema default, so absent and `false` behave
identically).

### Query bridge

`mzmon_lib::grafana::query` turns a registry query into the two things a panel needs.
`panel_query(registry, id, ctx)` looks an id up, renders it for the engine the `TemplateContext` names, and returns a
`QueryGroupKind` plus a markdown description.

The context's engine picks the datasource:
`PromQl` builds Prometheus dataqueries against `${metricsDatasource}`, `LogQl` builds Loki ones against `${logsDatasource}`.
Datadog and Honeycomb render fine but have no Grafana datasource here, so the bridge rejects them rather than emitting
an expression that would silently return nothing.

Two details worth knowing:

- **Prometheus compat fields.** Grafana's Prometheus datasource reads `query` and `qryType`, while the schema cog
  generates calls them `expr` and `queryType`.
  Sending only the schema spelling loses data on push, so the bridge emits both, as the Python it was ported from did.
- **Description formatting.** The registry's `Description` is structured (summary / nominal / degraded / unhealthy /
  notes).
  The bridge emits the summary in bold followed by the behavioral fields as labeled paragraphs, and unwraps the YAML hard-wrapping.
  That is one function (`format_description`) if the convention should change.

Legends are deliberately *not* a registry field, and stay with the panel: the same query legitimately reads `{{name}}`
on one panel and `{{cluster_name}} / {{name}}` on another.
`PanelQuery::legend` labels every series; `legends` labels each positionally against the query's `promQL` list, and a
count mismatch is an error rather than silent mislabeling of the wrong series.

### Panels do not write PromQL

Every panel names a registry query and takes **both** its expression and its description from it —
`packages/dashboards/src/grafana/queries.rs` is the handle, `Panel::query` sets both at once.
The registry is where a query's semantics and the prose explaining it are maintained together, and a dashboard is one
of several consumers of that pair; a panel that retypes either has opted out and will drift the first time the query
changes.

What stays with the panel is presentation: legend, unit, panel type, thresholds, transformations, shade, empty-state
text.

Ids are written inline at the panel that uses them, and there is deliberately **no** checked-in list of them.
The guarantee comes from the build instead: `Queries` records every lookup that fails and `build()` refuses to return a
dashboard, so a renamed or deleted registry query is a build failure naming every affected panel:

```
Error: building env-top: dashboard "env-top" has 1 unresolved registry query/queries:
  materialize.info.version: no query "materialize.info.version" in the registry
```

An id list would not catch this any earlier — the same failure, at the same moment — while adding an alias layer that
can drift from the id's meaning and a hand-maintained `ALL` array where a forgotten entry is silently untested.
`tests/registry_contract.rs` pins the mechanism, since it is what makes the inline ids safe.

Lookups are infallible for the same reason: threading a `Result` through sixty-odd panel builders to carry a case that
only fires on a typo costs more than it buys, so a failed lookup yields a placeholder and is collected. A placeholder
cannot reach an artifact because assembly fails first.

The registry-to-dashboard path is new: the Python dashboards hand-write their PromQL inline with f-strings rather than
going through the registry, so the pre-rendered dashboards are not a byte-level baseline for it.
The context that resolves parameters to Grafana built-ins and dashboard variables is `dashboard_context`, above.

`packages/mzmon-lib/tests/grafana_query_bridge.rs` runs the bridge over every PromQL query in the registry (207 at the
time of writing) plus the named availability query in detail.
The registry has no LogQL queries yet, so the Loki path is covered only by unit tests.

`packages/mzmon-lib/tests/grafana_golden.rs` parses the pre-rendered `env-top.yaml` into the generated models and
checks that re-serializing loses nothing.
That test is what caught the one place the schemas are narrower than Grafana itself:
`MatcherConfig.options` is typed `object`, but Grafana's `byName` matcher takes a bare field-name string, which the
golden dashboard emits and Grafana accepts.
The generation script widens it, with the reasoning recorded inline.

## Why not the Grafana Foundation SDK

The dashboards were originally built in Python against **grafana-foundation-sdk**, with a `py-mzmon-lib` layer of
wrappers and shims over it. Both are gone; the schemas the SDK is generated *from* are vendored directly instead, and
the Rust models come from those — see [Vendored schemas](#vendored-schemas) and [Rust models](#rust-models).

The reason is that the foundation SDK adds little over the schemas for what these dashboards do. Its builders are
mostly information shuttling, and the one place it genuinely adds something — hand-written unmarshallers keyed on
`kind`, which the schemas cannot express because the `*Kind` unions carry no `discriminator` — matters only on the
read path, which a generator does not take.

The Python is recoverable from git history if a comparison is ever wanted; the last render it produced is frozen at
`packages/mzmon-lib/tests/fixtures/env-top.python-baseline.yaml` and still backs the parity suite.
