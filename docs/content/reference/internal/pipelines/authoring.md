---
title: "Authoring"
weight: 5
---

# Authoring Pipelines

This page covers the model and conventions for authoring alloy pipelines as YAML under `packages/alloy-pipelines/`. The runtime aspects (label families, retention) live in [Logging]({{< relref "logging.md" >}}) and [Metrics]({{< relref "metrics.md" >}}).

## Pipeline model

A pipeline is a YAML document that maps roughly 1:1 to an alloy config file:

```yaml
description: |
  What this pipeline does, why it exists, what it forwards to.
logging:
  level: info
  format: logfmt
blocks:
  - loki.process:
      label: input_processor
      forward_to: [{ref: "loki.write.gateway.receiver"}]
      blocks:
        - stage.drop:
            older_than: "12h"
            drop_counter_reason: "backlog > 12hr"
        - stage.match:
            selector: '{app="alloy"}'
            blocks:
              - stage.logfmt:
                  mapping: { msg: msg, level: level }
```

Each entry in `blocks` is a **single-key object** whose key is either:

- a typed component name (`loki.process`, `loki.echo`, `discovery.kubernetes`, ...), validated by the per-component schema; or
- the literal key `raw`, validated by `raw.schema.yaml` and used for anything the typed schemas don't cover.

The same rule applies recursively to nested sub-blocks (e.g. stages inside `loki.process`, rules inside `discovery.relabel`).

**Typed blocks are flat** — attributes live directly under the discriminator key alongside `label` and (where applicable) `blocks`. **Raw blocks keep the `component / label / attributes / blocks` partition** because raw needs `component` as its own discriminator and serves as a generic container.

## The strict-attributes / raw-escape policy

Each typed component declares `additionalProperties: false` on the block body. **Undocumented attributes are rejected by design.**

When the validator says

```
schema violation at `/blocks/0/loki.process`:
  Additional properties are not allowed ('drop_malformed' was unexpected)
  hint: this key isn't typed in the schema. Either use a `raw:` block
        for one-off usage, or extend the relevant schema $def to add it.
```

…you have two choices, depending on whether the attribute is reusable:

1. **Extend the schema** — preferred when the attribute is something the team will plausibly use again. Edit the relevant `$def` in `packages/mzmon-lib/schemas/alloy/loki.schema.yaml` (or `discovery.schema.yaml`), add a `properties` entry with a `description`, an optional `examples`, and a `type`. The validator picks it up at build time (the schema is `include_str!`-embedded), and the attribute now shows up in IDE autocomplete and in reference docs.

2. **Use a `raw:` block** — preferred for one-off or experimental usage. Replace the typed block entirely with the raw form:

   ```yaml
   - raw:
       component: stage.json
       attributes:
         expressions: { msg: message }
         drop_malformed: true   # undocumented in our schema; raw bypasses validation
   ```

   `raw:` is **all-or-nothing per block** — you can't mix typed attributes with `raw:` attributes inside the same block. If you only need one undocumented attribute, you have to write the whole block raw.

When in doubt, prefer `raw:` and graduate to the schema if it becomes a pattern. The schemas are meant to grow with usage, not to mirror alloy upstream exhaustively.

### Why strict?

The trade-off is deliberate and worth understanding:

| Strict (current) | Loose |
|---|---|
| Catches typos (`older_then`) immediately | Typos slip through; caught only by `alloy validate` later, if at all |
| Schema is a contract: documented == accepted | Schema is a hint: documented ⊂ accepted |
| Forces explicit `raw:` for anything undocumented | Lets undocumented attributes pass silently |
| Schema is the source of truth for reference docs | Reference docs may not reflect what's actually accepted |

We chose strict because the schemas double as reference documentation. Loose would let the docs drift from reality silently; strict makes the gap explicit at validation time.

## Schema layout

```
packages/mzmon-lib/schemas/alloy/
├── mzmon-alloy.schema.yaml    # entry point ($ref top + future imports)
├── top.schema.yaml            # description / logging / livedebugging / blocks
├── raw.schema.yaml            # the {raw: <block>} escape hatch + AST primitives
├── loki.schema.yaml           # loki.*, plus stage.* $defs and the shared `rule` $def
└── discovery.schema.yaml      # discovery.*, cross-references `rule` from loki
```

Each component file declares its own `$id` URL. The validator (`packages/mzmon-lib/src/alloy/validate.rs`) registers all schemas in a `referencing::Registry` so cross-file `$ref`s resolve at runtime against the embedded copies, with no network or filesystem dependency.

### Cross-file `$ref`s

Cross-file references use a relative path (`./loki.schema.yaml#/$defs/ruleBlock`) which the registry resolves through `$id`. The same path also works in the IDE — `yaml-language-server` follows relative paths directly. **Don't use absolute URLs in `$ref`s**; they only work if `$id`s match exactly and break the local IDE experience.

## Extending the schema

Three recipes, in increasing scope: add an attribute, add a typed sub-block, add a whole component. All three follow the same pattern: schema first (so validation + IDE hover get the change), Rust struct + `ToBlock` if it's a typed sugar variant, then a round-trip test colocated with the impl.

### Adding an attribute to an existing component

Say `drop_malformed` on `stage.json`:

1. Open `packages/mzmon-lib/schemas/alloy/loki.schema.yaml`.
2. Find the `stage.json` `$def` under `$defs`.
3. Add the attribute alongside the existing properties (typed schemas are flat — no `attributes:` nesting):

   ```yaml
   stage.json:
     # ...
     type: object
     properties:
       expressions: { ... }
       source: { ... }
       drop_malformed:                  # ← new
         description: |
           When true, entries with unparseable JSON are dropped.
         type: boolean
         examples: [true]
     required: [expressions]
     additionalProperties: false
   ```

4. If the component has a Rust sugar struct (e.g. `LokiSourceJournalBlock`), add the corresponding field with `#[serde(default, skip_serializing_if = "Option::is_none")]` and extend `to_block` to emit the attribute when set. (For schema-only stages without Rust sugar, you can stop here — the schema documents the attribute and validation enforces the type.)
5. Add or extend a round-trip test (preferably colocated with the Rust impl in `components/*.rs`, or in `pipeline.rs` for top-level concerns).
6. Run `cargo test -p mzmon-lib`, `cargo fmt`, and `make pipelines`.

### Adding a typed sub-block

The `discovery.kubernetes`'s `selectors` and `attach_metadata` blocks are the canonical example. Pattern:

1. **Schema** — in `discovery.schema.yaml`, the component's `blocks.items` refs a `<componentName>SubBlock` `$def` (a `oneOf` over typed branches + a final `raw` escape):
   ```yaml
   kubernetesSubBlock:
     oneOf:
       - type: object
         properties:
           selectors:       { $ref: "#/$defs/selectors" }
         required: [selectors]
         additionalProperties: false
       - type: object
         properties:
           attach_metadata: { $ref: "#/$defs/attach_metadata" }
         required: [attach_metadata]
         additionalProperties: false
       - $ref: "./raw.schema.yaml"     # ← always the last branch
   ```
2. **`$def` for each sub-block** — typed properties, `required`, `additionalProperties: false`, a `See:` link to canonical alloy docs.
3. **Rust struct** — flat, `#[serde(deny_unknown_fields)]`. One per typed sub-block (e.g. `DiscoveryKubernetesSelector`).
4. **`impl ToBlock`** — populate an `IndexMap<String, AttributeValue>`, return `Block { component: "<sub-block name>", label: None, attributes, blocks: Vec::new() }`.
5. **Sub-block enum** — extend the component's existing `<Component>SubBlock` enum with a new variant, `#[serde(rename = "<yaml-key>")]`:
   ```rust
   pub enum KubernetesSubBlock {
       #[serde(rename = "selectors")]
       Selectors(DiscoveryKubernetesSelector),
       #[serde(rename = "attach_metadata")]
       AttachMetadata(DiscoveryKubernetesAttachMetadata),
       #[serde(rename = "raw")]
       Raw(Block),
   }
   ```
6. **Dispatch in the enum's `ToBlock`** — add a match arm.
7. **Test** — a round-trip test through `Pipeline::from_yaml_str` that exercises the new sub-block, colocated in the same `components/*.rs` file.

Sub-block enums *always* keep `Raw(Block)` as a fallback so unsupported sub-blocks don't require a schema change.

### Adding a typed component

Adding a whole new typed component (e.g. a `prometheus.*` family) is the same shape one level up:

1. **New schema file** `prometheus.schema.yaml`, registered in `validate.rs` (a fresh `ID_PROMETHEUS` constant + an entry in the `Registry::extend` list).
2. Add a branch to `top.schema.yaml`'s `blocks.items.oneOf` pointing at the new file.
3. **Rust struct + `ToBlock`** in `components/prometheus.rs`.
4. **Variant in `ComponentBlock`** (`pipeline.rs`) with `#[serde(rename = "<exact-component-name>")]`.
5. **Dispatch in `ComponentBlock::to_block`**.
6. Tests colocated in `components/prometheus.rs`.

Mirror the `loki.schema.yaml` / `components/loki.rs` structure as a template.

## Reference-valued attributes

A real foot-gun worth its own section. Some component attributes are *references to other components' exports*, NOT string literals:

- `loki.process.forward_to` — array of `loki.write.*` receivers
- `loki.source.journal.forward_to` — same
- `loki.relabel.forward_to` — same
- `discovery.relabel.targets` — refs to another `discovery.*` component's `.targets`
- `loki.source.journal.relabel_rules` — ref to a `discovery.relabel` rule set

These render in alloy as **bare identifiers**:

```alloy
forward_to = [loki.write.gateway.receiver]   // ← bare ref, NOT "loki.write.gateway.receiver"
targets    = [discovery.kubernetes.pods.targets]
```

Wrapping a `TargetRef` (a `String` alias in Rust) as `AttributeValue::String` would render it *quoted* — which is syntactically valid alloy but semantically wrong (alloy would treat it as a string literal, not a component reference). The pipeline would silently misbehave at runtime.

**The fix in `to_block`**: wrap each `TargetRef` as an `Expression::ref_name`:

```rust
AttributeValue::Array(
    self.forward_to
        .iter()
        .map(|s| AttributeValue::Expression(Expression {
            ref_name: Some(s.clone()),
            ..Default::default()
        }))
        .collect(),
)
```

When a component has even one ref-valued attribute, walk every `to_block` site for that struct and make sure the wrap is in place. The renderer produces bare refs only for `AttributeValue::Expression { ref_name: Some(_) }`.

## Load-bearing invariants

These are *non-obvious things that must stay true* — flagged here so future refactors don't silently break them. Each has a regression test in `ast.rs` that pins the behavior.

### `AttributeValue` variant order

The `AttributeValue` enum in `ast.rs` uses `#[serde(untagged)]`. Serde tries variants top-to-bottom and picks the first that deserializes. **The current order is load-bearing:**

```rust
#[serde(untagged)]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<AttributeValue>),
    Expression(Expression),
    Object(IndexMap<Identifier, AttributeValue>),
}
```

Two specific rules:

- **`String` and `Array` MUST come before `Expression`.** Serde's struct deserializer accepts a sequence by *positional field assignment* by default. Without this order, `["a", "b"]` would silently deserialize as `Expression { raw: Some("a"), env: Some("b"), ... }`, then fail at render time with "Too many expressions" (or worse, succeed and render wrong).
- **`Expression` must come before `Object`.** Otherwise the catch-all map would swallow expression-shaped objects (`{ref: "..."}`, `{env: "..."}`) before they can be recognized.

The `deny_unknown_fields` attribute on `Expression` is the third piece: without it, a generic object like `{mapping: ...}` would silently match `Expression` (all heads `None`, unknown field ignored) instead of falling through to `Object`.

### `raw:` is always the last `oneOf` branch

Every sub-block `oneOf` ends with `$ref: "./raw.schema.yaml"`. New typed branches go *before* the raw branch, never after. The raw escape is the contract — without it, undocumented sub-blocks have nowhere to go.

### Schemas-as-docs

The schemas double as reference documentation; `description` text renders in IDE hover and (eventually) in the published reference site. Keep descriptions user-facing, concise (first line ≤80 chars), and link to canonical alloy upstream via a trailing `See:` line. When alloy upstream renames a field, update both the description and the `properties` entry — leave the `See:` link stable so future contributors can audit drift.

### Renderer alignment quirk

The renderer's block-attribute alignment rule is *currently* "any multi-line value disables `=`-alignment for the surrounding attribute group." This is more aggressive than `alloy fmt`'s actual rule, which aligns in more cases. Tests sometimes need to work around this — e.g. by writing rule blocks with one attribute each — until the rule is refined. The investigation log lives on the renderer-alignment task. If your test's `assert_renders` fails on a single attribute getting unexpectedly padded (or unpadded), suspect this quirk before suspecting the rendered output itself.

## Sub-block recursion and the `raw` escape

Sub-block lists (a `loki.process` body, a `stage.match` body, a `loki.relabel`'s `rule` list, a `discovery.kubernetes` body) are typed as a `oneOf` whose **last branch is always `$ref: "./raw.schema.yaml"`** (or the cross-file equivalent). This is non-negotiable in the design: every sub-block context has an escape hatch. New typed branches are added before the `raw` branch so the escape stays as the final fallback.

`stage.match` is recursive: its body refs `#/$defs/stageBlock`, which itself includes `stage.match`. JSONSchema handles the cycle natively via `$ref`.

## Linking to canonical alloy docs

Each `$def` description ends with a `See:` line linking to the canonical Grafana alloy documentation:

```yaml
description: |
  Drops log entries matching the configured condition.

  See: https://grafana.com/docs/alloy/latest/reference/components/loki/loki.process/#stagedrop-block
```

When alloy upstream renames a field or changes semantics, **update the schema's `description` and `properties` to match**, leaving the `See:` link stable. The schema is a snapshot, not a mirror; explicitly noting the drift on the link's destination is part of the contributor workflow.

## How validation interacts with rendering

The `Pipeline::from_yaml_str` entry point does three things in order:

1. Parses the YAML into a generic `serde_json::Value`.
2. Validates the value against the embedded JSONSchema, collecting *all* violations into `Error::Multiple`.
3. Deserializes the value into the typed `Pipeline` struct (serde, externally-tagged enums).

This means schema errors fire **before** any serde decoding errors, and any single document can report multiple problems in one pass. The renderer (`Pipeline::render`) then produces canonical `config.alloy` output verified by the `alloy fmt` oracle in tests; see `mzmon-lib/src/alloy/test_support.rs`.

## CLI

`mz-monitoring-build gen-pipelines` is the entry point:

```
mz-monitoring-build gen-pipelines \
    --output-dir charts/materialize-monitoring/pre-rendered/pipelines \
    --target gateway        # optional; defaults to all *.yaml in --input-dir
```

The Makefile target `make pipelines` invokes this once per target and then runs `alloy validate` on the rendered output as a second-layer sanity check. Both layers should be green before merging.
