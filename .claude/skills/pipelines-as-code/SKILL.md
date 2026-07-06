---
name: pipelines-as-code
description: |
  Use this skill when working with Materialize's alloy log-processing or metric-processing pipelines — anything under `packages/alloy-pipelines/`. Triggers include: building, modifying, reviewing, or rendering pipelines; working with the embedded JSONSchemas under `packages/mzmon-lib/schemas/alloy/`; deciding between typed blocks and the `raw:` escape hatch; extending the schemas to add a new component, stage, or attribute; debugging `mz-monitoring-build gen-pipelines` output; writing Materialize-specific log-processing patterns (level normalization, structured metadata, label families, drop/limit conventions); writing or reviewing alloy stages, loki.process pipelines, discovery.kubernetes, loki.relabel rules, or sources like loki.source.file / loki.source.journal. Also use it whenever someone refers to "log processing", "metric processing", or anything alloy-shaped, even if they don't use the word "pipeline".
---

# Pipelines as Code

This skill is the entry point for the Materialize pipelines-as-code project. **Stable conventions live in the repo docsite** under [`docs/content/reference/internal/pipelines/`](../../../docs/content/reference/internal/pipelines/) — this file is intentionally slim and links into the docsite at heading-level granularity. The non-link content below is the **state snapshot**: what currently exists, what's in flight, and what's queued.

## Audience reminder

The **pipelines themselves** target alloy (the binary), so authoring decisions favor *contributors* rather than end users. There is no panel-description-voice equivalent — readability of the YAML and the rendered `.alloy` is the goal.

The **docsite reference pages** target repo contributors (SRE, Field Engineering, CloudOps, Database Engineers) and AI agents reading this skill.

## Where to find what

| Looking for… | Read |
|---|---|
| Pipeline model, strict-attributes policy, `raw:` escape rules, how to extend the schema | [Authoring](../../../docs/content/reference/internal/pipelines/authoring.md) |
| Log pipeline conventions, label families, retention | [Logging](../../../docs/content/reference/internal/pipelines/logging.md) (stub) |
| Metrics pipeline conventions | [Metrics](../../../docs/content/reference/internal/pipelines/metrics.md) (stub) |

Frequently needed deep links into Authoring:

- [The strict-attributes / raw-escape policy](../../../docs/content/reference/internal/pipelines/authoring.md#the-strict-attributes--raw-escape-policy) — what to do when an undocumented attribute is rejected
- [Schema layout](../../../docs/content/reference/internal/pipelines/authoring.md#schema-layout) — file/`$id` map and how cross-file `$ref`s resolve
- [Adding an attribute](../../../docs/content/reference/internal/pipelines/authoring.md#adding-an-attribute-to-an-existing-component) — minimal extension
- [Adding a typed sub-block](../../../docs/content/reference/internal/pipelines/authoring.md#adding-a-typed-sub-block) — the `selectors`-style pattern
- [Adding a typed component](../../../docs/content/reference/internal/pipelines/authoring.md#adding-a-typed-component) — Rust struct + schema + ComponentBlock variant
- [Reference-valued attributes](../../../docs/content/reference/internal/pipelines/authoring.md#reference-valued-attributes) — why `forward_to` / `targets` need `Expression::ref_name`, not `String`
- [Load-bearing invariants](../../../docs/content/reference/internal/pipelines/authoring.md#load-bearing-invariants) — AttributeValue order, alignment quirk, schemas-as-docs
- [Sub-block recursion and the `raw` escape](../../../docs/content/reference/internal/pipelines/authoring.md#sub-block-recursion-and-the-raw-escape)
- [How validation interacts with rendering](../../../docs/content/reference/internal/pipelines/authoring.md#how-validation-interacts-with-rendering) — YAML → `serde_json::Value` → schema validation → typed `Pipeline` → `config.alloy`

## Schema and code map

```
packages/alloy-pipelines/                       ← YAML inputs
  ├── gateway.yaml
  └── agent.yaml

packages/mzmon-lib/schemas/alloy/               ← validation schemas (embedded into the binary)
  ├── mzmon-alloy.schema.yaml                   – entry
  ├── top.schema.yaml                           – description / logging / livedebugging / blocks
  ├── loki.schema.yaml                          – loki.*, stage.* $defs, shared `rule` $def
  ├── discovery.schema.yaml                     – discovery.*, cross-ref'ing `rule` from loki
  └── common/                                   – shared fragments. $id tail MUST match the file path
      ├── raw.schema.yaml                       – {raw: <block>} escape hatch + block primitives
      ├── attribute.schema.yaml                 – attributeValue (anyOf: literal | expression | …)
      └── expression.schema.yaml                – sys.env / function / operator / ref expression
  (each fragment's `$id` + the `ID_*` const in validate.rs + the relative `$ref`s must agree)

packages/mzmon-lib/src/alloy/                   ← AST, render, validate, pipeline (Rust)
  ├── ast.rs                                    – Block / AttributeValue / Expression / Expressable<T>
  ├── render.rs                                 – write_to + alloy-fmt-canonical formatting
  ├── validate.rs                               – embedded schemas + jsonschema validator + hints
  ├── pipeline.rs                               – Pipeline::from_yaml_str (YAML → Value → validate → typed)
  ├── test_support.rs                           – assert_renders (oracle: pipes through `alloy fmt`)
  └── components/                               ← typed sugar; tests colocated with impl
      ├── top.rs                                – LoggingBlock, LiveDebuggingBlock
      ├── capsule.rs                            – LogsReceiver, RelabelRules, TargetEntry (+ string_map,
                                                  logs_receiver_list, target_list helpers)
      ├── loki.rs                               – LokiEchoBlock, LokiSourceJournalBlock, ...
      ├── relabel.rs                            – RelabelRule + RelabelSubBlock (shared by *.relabel)
      └── discovery.rs                          – DiscoveryKubernetesBlock, DiscoveryRelabelBlock,
                                                  KubernetesSubBlock variants

packages/mz-monitoring-build/                   ← CLI: `mz-monitoring-build gen-pipelines`
```

Build/test:

```
make pipelines                          # render YAML → .alloy + run `alloy validate` per target
cargo test -p mzmon-lib                 # unit + oracle tests (uses `alloy fmt` if present)
cargo clippy -p mzmon-lib --all-targets # lints
cargo fmt                               # the team runs this in pre-commit; run it before
                                        #   committing any Rust changes
```

---

# Current Pipeline State

This section captures the live state so the next session has something concrete to start from. **Update it when state changes meaningfully** (new pipeline, new typed component, schema gap closed, etc).

## Pipeline inventory

| YAML | Rendered to | Status |
|---|---|---|
| `packages/alloy-pipelines/gateway.yaml` | `charts/.../pipelines/gateway.alloy` | **implemented & fully typed** — faithful port of the reference `processor.alloy` `loki.process` block (level/timestamp/per-service field normalization, per-level rate limits, final label shaping) + a static `loki.write` stub |
| `packages/alloy-pipelines/agent.yaml` | `charts/.../pipelines/agent.alloy` | **implemented & largely typed** — staging-agent parity (journal + node-local pod logs → gateway, sampled debug tap) |

Both pass `alloy validate`. `agent.yaml` is a faithful port of the reference `staging-agent.alloy` (`packages/ref-alloy-pipelines/`, not checked in). It started with several `raw:` escapes; most have since been graduated to typed forms (`stage.static_labels`/`selectors.field` via `Expressable`, `attach_metadata.namespace`, `stage.cri`). As of the last session **only two `raw:` blocks remain**, both expected:
- `loki.source.file`'s `file_match` sub-block — `loki.source.file` has no typed sub-block support yet (queued).
- the `loki.write "gateway"` sink — intentionally a static stub; richer/remote endpoint + auth config is deliberately deferred (it's exceptional: shared by agents and gateways, may write to remote sinks).

`gateway.yaml` is a faithful port of the reference `processor.alloy` (`packages/ref-alloy-pipelines/`). Every stage in the processor is now typed — closing the `stage.regex.labels_from_groups` gap was the last blocker. The only `raw:` block is the `loki.write "default"` sink (same deferred stub rationale as the agent's gateway sink). Two things are deliberately **not** wired yet, tracked as follow-ups: (1) the source that receives entries from alloy-agent and feeds `loki.process.input_processor` (e.g. `loki.source.api`); (2) real endpoint/auth on the sink. The ~11 identical mz-system service blocks (json → error/msg templates → timestamp → structured_metadata) are written once via YAML anchors (`&mz_svc_*`) and aliased; anchors expand at parse time so the rendered `.alloy` is fully explicit. Note: attribute *order* within a rendered block follows the typed sugar's `to_block` insertion order, not YAML/reference order — so `gateway.alloy` won't byte-match `processor.alloy` inside `stage.drop`/`stage.regex`/`stage.replace`, but it's semantically identical (block attribute order is irrelevant to alloy).

Configurable knobs in `agent.yaml`: `cluster`/`node` labels via `sys.env(...)` (typed `stage.static_labels` values, `Expressable<String>`); `selectors.field` via a `"spec.nodeName=" + coalesce(sys.env(...), constants.hostname)` expression. `stage.limit` rate/burst read from env with a `encoding.from_json(coalesce(sys.env("…"), "<default>"))` expression — the `from_json` is a string→int coercion hack (alloy has no `to_int`, and `sys.env` returns strings while `rate`/`burst` want numbers). Treat that coercion as provisional, not a blessed pattern.

## Typed schema coverage

**Top-level loki.* components**: `loki.echo`, `loki.process`, `loki.relabel`, `loki.source.journal`, `loki.source.file`.

**Top-level discovery.* components**: `discovery.kubernetes`, `discovery.relabel`.

**`loki.process` stages** (and `stage.match` body, recursively): `stage.match`, `stage.drop`, `stage.limit`, `stage.regex`, `stage.replace`, `stage.template`, `stage.logfmt`, `stage.json`, `stage.timestamp`, `stage.labels`, `stage.static_labels`, `stage.label_drop`, `stage.structured_metadata`, `stage.structured_metadata_drop`, `stage.sampling`, `stage.cri` (empty, no attributes).

**`discovery.kubernetes` sub-blocks**: `selectors` (incl. `field`/`label` as `Expressable<String>`), `attach_metadata` (`node` + `namespace`). Other sub-blocks (e.g. `namespaces`) use `raw:`.

**`*.relabel` sub-blocks**: `rule` (shared `$def` between `loki.relabel` and `discovery.relabel` via cross-file `$ref` to `loki.schema.yaml#/$defs/ruleBlock`).

**Literal-or-expression fields** use `Expressable<T>` (`ast.rs`): a field typed `Expressable<f64|String|bool>` (or `Option<…>`) accepts either a scalar literal or an inline expression object (`{env}`, `{function}`, `{operator}`, `{ref}`). In use on `stage.limit` rate/burst, `stage.drop` older_than, `stage.static_labels` values, `selectors` field/label. Schema side: `anyOf: [{type: <scalar>}, {$ref: common/expression.schema.yaml}]` (safe — scalar vs object are disjoint). *Scalars only* — never `Expressable<map/object>` (a literal map would collide with the expression object, the same overlap that forced `anyOf` in the raw `attributeValue`). This is an actively-expanding pattern; adopt per-field as needed.

**Every sub-block `oneOf` ends with a `raw:` branch** — the escape hatch is non-negotiable in the design.

Only the attributes we routinely use are documented per component — not exhaustive vs. alloy upstream. **Strict `additionalProperties: false`** is enforced; undocumented attributes are rejected at validation time with a hint pointing at the `raw:` escape vs. schema-extension choice. See [Authoring §The strict-attributes / raw-escape policy](../../../docs/content/reference/internal/pipelines/authoring.md#the-strict-attributes--raw-escape-policy).

## Rust sugar deserialization status

The `ComponentBlock` enum in `pipeline.rs` dispatches to typed sugar structs (via `#[serde(rename = "loki.echo")]` etc.). Each typed component has a `pub struct …Block { fields }` + an `impl ToBlock` that normalizes to the generic `Block` AST for rendering. Sub-blocks follow the same pattern via per-component sub-block enums (`KubernetesSubBlock`, `RelabelSubBlock`). The `ToBlock` enum impls are generated by the `impl_to_block_dispatch!` macro (`ast.rs`, `pub(crate)`, `$crate::` paths) — list every variant in the invocation; a missed one is a compile error.

**Done: every component in the typed-schema coverage list above round-trips through the typed path.** No pending sugar work.

**Capsule types** (`components/capsule.rs`) make the bare-ref invariant structural: `LogsReceiver` (`forward_to`), `RelabelRules` (`relabel_rules`), and `TargetEntry` (`targets`, untagged `Ref(Identifier) | Literal(IndexMap)`). `targets` accepts mixed refs and literal label maps in one array — alloy type-checks `list(capsule)` and flattens list-valued elements (verified against `alloy validate`; pinned by round-trip tests in loki.rs and discovery.rs). Schema side: `$defs/target` (discovery.schema.yaml, generic) vs `$defs/fileTargetEntry` (loki.schema.yaml, `required: [__path__]`) — strictness lives in the schema, the Rust type stays generic. A new capsule type (e.g. `otelcol.Consumer` when otelcol lands) is a newtype + `From<&T> for AttributeValue` via `Expression::name_to_ref` + a schema `$def`.

## Load-bearing invariants

These are *non-obvious things that must stay true* — flagged here so reorders or refactors don't silently break them. Detail in [Authoring §Load-bearing invariants](../../../docs/content/reference/internal/pipelines/authoring.md#load-bearing-invariants).

- **`AttributeValue` variant order**: `Null`/`Bool`/`Number`/`String`/`Array` *must* come before `Expression`. Serde's untagged struct deserializer accepts a sequence by positional-field assignment, so `["a", "b"]` would misroute to `Expression { raw: Some("a"), env: Some("b") }`. Regression-tested in `ast.rs`.
- **`#[serde(deny_unknown_fields)]` on `Expression`**: keeps generic objects (`{mapping: ...}`) from silently matching `Expression` with all heads `None`. Also regression-tested.
- **Ref-valued attributes render as bare refs, never quoted strings** — and this is now *enforced by type*: declare the field with a capsule type (`Vec<LogsReceiver>`, `Option<RelabelRules>`, `Vec<TargetEntry>`) and convert via the capsule helpers / `Expression::name_to_ref`. Do NOT hand-build `Expression { ref_name: ... }` in new `to_block` impls; if a new capsule kind is needed, add it to `components/capsule.rs`.
- **`raw:` escape is always the last `oneOf` branch** in every sub-block list. Adding a new typed branch goes *before* `raw`.
- **`raw.schema.yaml`'s `attributeValue` uses `anyOf`, NOT `oneOf`**: an expression-shaped object (`{ref}`, `{env}`, `{operator}`, `{function}`) is *also* a valid generic `attributeObject`, and a `{value: ...}` is also a `commentedValue` — so exactly-one `oneOf` rejected every expression/commented raw value (this blocked the first real expression-valued raw block in `agent.yaml`). The Rust `AttributeValue` deserializer disambiguates by variant order + `deny_unknown_fields`; the schema just needs "is *some* legal raw value," which `anyOf` gives. Don't revert it to `oneOf`. Regression-tested in `pipeline.rs::raw_block_with_expression_values_round_trips`.
- **`Error::Multiple` renders its children** (header + indented bullet per child, recursive). Earlier it displayed a bare "multiple errors" and swallowed the detail — `gen-pipelines` failures were undiagnosable. Tested in `error.rs`.
- **A schema file's `$id` tail MUST match its path** (and the `ID_*` key in `validate.rs`). The Rust `Registry` registers each resource under an explicit key so it tolerates a mismatch, but `yaml-language-server` keys on the in-file `$id` and breaks (refs point at a URI no schema claims). Keep filename ↔ `$id` ↔ `ID_*` in lockstep.
- **Known tooling bug — yaml-language-server `Maximum call stack size exceeded`**: fires on our (necessarily) recursive schemas during the LSP's meta-schema validation; it's upstream (fixed by redhat-developer/yaml-language-server#1269), NOT our content. The CLI validator is the source of truth — don't flatten the recursion to appease the editor. Full writeup + debugging recipe in [authoring.md](../../../docs/content/reference/internal/pipelines/authoring.md#known-tooling-issue-yaml-language-server-maximum-call-stack-size-exceeded).
- **Schemas are the reference docs**: descriptions render in IDE hover and (eventually) in published reference; keep them user-facing. Each `$def` ends with a `See:` link to canonical alloy upstream.
- **GOTCHA — schemas are embedded at compile time (`include_str!`).** After editing any `schemas/alloy/*.yaml`, you MUST `cargo build --bin mz-monitoring-build` before a manual `gen-pipelines`, or it validates against the **stale** embedded schema and reports phantom "doesn't match any typed schema" errors. `make pipelines` rebuilds the binary as a dependency (safe); `cargo test` recompiles the lib so tests see the fresh schema — which means **unit tests can pass while a manual render of the same construct fails**. If a render rejects something your tests accept, rebuild the binary first. (A new schema fragment file also needs registering in `validate.rs`: a `SCHEMA_*` `include_str!`, an `ID_*` const matching its `$id`, and an entry in the `Registry` list.)
- **GOTCHA — `assert_renders` enforces alloy-fmt-canonical output.** It does an exact-bytes match AND (when `alloy` is on PATH) asserts the rendered output is what `alloy fmt` would produce. So a test fails if the renderer's output isn't canonical, even when your expected string matches the renderer. The renderer's alignment quirk (see Cleanup) makes some shapes non-canonical — write tests around shapes known to be canonical (single-line attribute groups, object literals) until the quirk is fixed.

## Cleanup / refactor candidates

- **`write_expression` uses a `rendered_expr` flag**: the idiomatic shape is a tuple-match over `(&env, &raw, &function, &ref_name, &operator)`, which encodes "exactly one head set" structurally. The flag pattern bit us once (forgotten assignments); a refactor would prevent recurrence.
- **Renderer block-attribute alignment (task #15)**: current rule "any multi-line value disables alignment for the whole group" is too aggressive; alloy fmt aligns in more cases. Concretely, the only divergences in the rendered `agent.alloy` vs `alloy fmt` are (a) `rule` blocks where a single-line attr (`action`, `separator`, `replacement`) sits in a group that also contains the multi-line `source_labels` array, and (b) `loki.source.file`'s `targets` (single-line) next to the multi-line `forward_to` array. Everything else is canonical. Two ways out (an open decision): fix the alignment rule, or post-process rendered output through `alloy fmt` (we have alloy in CI now). Until then, `assert_renders` will reject tests that exercise those non-canonical shapes.
- **Schema drift watch**: `description` blocks link to canonical alloy docs. When alloy upstream renames a field or adds one we use, the schema description and `properties` need a corresponding update. There's no automation here; it's a manual sweep when bumping alloy versions.
- **`gateway.yaml` is still a placeholder**. Porting the real gateway processor pipeline (`processor_pipeline.py`) is a deferred authoring task. Typed schemas + `raw:` escape + Rust sugar are ready for it. (`agent.yaml` is implemented — see Pipeline inventory.)
- **`with_capacity` + push loops in `to_block` impls** (~5 sites): idiomatic form is `self.blocks.iter().map(ToBlock::to_block).collect::<Result<Vec<_>>>()?`. Cosmetic; sweep opportunistically.
- **Capsule newtype fields are `pub`** (`LogsReceiver(pub Identifier)`): if ref-path validation is ever added, switch to private field + `fn new() -> Result<Self>`.

## Queued work (non-binding — directions, not commitments)

Rough backlog from recent sessions; shapes may change. In loose priority:

- **`replace_map` sugar** on `discovery.relabel`/`loki.relabel`: a `{source_label: target_label}` map expanding to one `action: replace` rule each, in source order (relies on the `preserve_order` already enabled). Covers only the 1:1 replace case — multi-source / `separator` / `replacement` rules (e.g. `job`, `__path__`) stay explicit. Biggest readability win for the relabel-heavy blocks.
- **`loki.source.file` sub-blocks** (incl. typed `file_match`): the component has no nested-`blocks` support yet; adding it removes the last non-stub `raw:` in `agent.yaml`.
- **Renderer alignment vs `alloy fmt`** (the open decision above): fix the rule, or post-process through `alloy fmt`.
- **`$comment` / inline-comment rendering**: the schema already declares `$comment`/`commentedValue`, but `Block` has no comment field and the renderer drops them — pure plumbing. Doc-gen from comments is a stretch follow-on.
- **Ref-resolution pass**: `forward_to`/`targets`/`relabel_rules` are free strings; nothing checks the referenced component exists (alloy validate catches it, but later + with worse messages).
- **Reusable `Expressable` schema `$defs`** (`numberOrExpr`/`stringOrExpr`): fields currently inline the `anyOf`; a couple of named defs would DRY it. Cosmetic.
- **Golden snapshot test** for the full rendered `agent.alloy`, once the above settle (so it doesn't churn).
- **CI freshness**: the `pipelines` job in `.github/workflows/test.yaml` asserts committed `.alloy` matches a fresh render — keep rendered output committed.

A few decisions are deliberately deferred: the `gateway.yaml` processor port; `loki.write` remote/auth config (it's exceptional — shared by agents + gateways, may target remote sinks); and a real (build- or runtime-) parameterization mechanism for numeric knobs like `stage.limit` rates (current `encoding.from_json` env coercion is provisional).
