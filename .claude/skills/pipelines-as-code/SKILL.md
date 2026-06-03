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
  ├── raw.schema.yaml                           – {raw: <block>} escape hatch + AST primitives
  ├── loki.schema.yaml                          – loki.*, stage.* $defs, shared `rule` $def
  └── discovery.schema.yaml                     – discovery.*, cross-ref'ing `rule` from loki

packages/mzmon-lib/src/alloy/                   ← AST, render, validate, pipeline (Rust)
  ├── ast.rs                                    – Block / AttributeValue / Expression
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
| `packages/alloy-pipelines/gateway.yaml` | `charts/.../pipelines/gateway.alloy` | placeholder (single raw `loki.echo "echo" { }`) |
| `packages/alloy-pipelines/agent.yaml` | `charts/.../pipelines/agent.alloy` | placeholder (single raw `loki.echo "echo" { }`) |

Both pass `alloy validate`. Neither yet uses the typed schemas in anger — they're stubs that exercise the end-to-end build pipeline. The Python reference under `packages/ref-alloy-pipelines/` (not checked in) describes the eventual gateway processor shape.

## Typed schema coverage

**Top-level loki.* components**: `loki.echo`, `loki.process`, `loki.relabel`, `loki.source.journal`, `loki.source.file`.

**Top-level discovery.* components**: `discovery.kubernetes`, `discovery.relabel`.

**`loki.process` stages** (and `stage.match` body, recursively): `stage.match`, `stage.drop`, `stage.limit`, `stage.regex`, `stage.replace`, `stage.template`, `stage.logfmt`, `stage.json`, `stage.timestamp`, `stage.labels`, `stage.static_labels`, `stage.label_drop`, `stage.structured_metadata`, `stage.structured_metadata_drop`, `stage.sampling`.

**`discovery.kubernetes` sub-blocks**: `selectors`, `attach_metadata`. Other sub-blocks (e.g. `namespaces`) use `raw:`.

**`*.relabel` sub-blocks**: `rule` (shared `$def` between `loki.relabel` and `discovery.relabel` via cross-file `$ref` to `loki.schema.yaml#/$defs/ruleBlock`).

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
- **Schemas are the reference docs**: descriptions render in IDE hover and (eventually) in published reference; keep them user-facing. Each `$def` ends with a `See:` link to canonical alloy upstream.

## Cleanup / refactor candidates

- **`write_expression` uses a `rendered_expr` flag**: the idiomatic shape is a tuple-match over `(&env, &raw, &function, &ref_name, &operator)`, which encodes "exactly one head set" structurally. The flag pattern bit us once (forgotten assignments); a refactor would prevent recurrence.
- **Renderer block-attribute alignment (task #15)**: current rule "any multi-line value disables alignment" is too aggressive; alloy fmt aligns in more cases. Some test YAML works around it (one attribute per rule block); a precise rule would let real pipelines stay canonical.
- **Schema drift watch**: `description` blocks link to canonical alloy docs. When alloy upstream renames a field or adds one we use, the schema description and `properties` need a corresponding update. There's no automation here; it's a manual sweep when bumping alloy versions.
- **`packages/alloy-pipelines/*.yaml` are placeholders**. Filling in the real gateway processor pipeline (port of `processor_pipeline.py`) is the next major authoring task. Typed schemas + `raw:` escape + all Rust sugar are ready for it.
- **`with_capacity` + push loops in `to_block` impls** (~5 sites): idiomatic form is `self.blocks.iter().map(ToBlock::to_block).collect::<Result<Vec<_>>>()?`. Cosmetic; sweep opportunistically.
- **Capsule newtype fields are `pub`** (`LogsReceiver(pub Identifier)`): if ref-path validation is ever added, switch to private field + `fn new() -> Result<Self>`.
