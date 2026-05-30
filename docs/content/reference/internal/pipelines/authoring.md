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

## Extending the schema: a worked example

Adding a new attribute to an existing stage, say `drop_malformed` on `stage.json`:

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

4. Add or extend a test in `packages/mzmon-lib/src/alloy/pipeline.rs` that exercises the new attribute through `Pipeline::from_yaml_str`.
5. Run `cargo test -p mzmon-lib`, `cargo fmt`, and `make pipelines`.

Adding a whole new typed component (e.g., a new `prometheus.*` family) is the same shape one level up: a new file `prometheus.schema.yaml`, registered in `validate.rs`, referenced from `top.schema.yaml`'s `blocks` `oneOf`, with `$defs` per component and per sub-block. Mirror the `loki.schema.yaml` structure.

## Sub-block recursion and the `raw` escape

Sub-block lists (a `loki.process` body, a `stage.match` body, a `loki.relabel`'s `rule` list) are typed as a `oneOf` whose **last branch is always `$ref: "./raw.schema.yaml"`**. This is non-negotiable in the design: every sub-block context has an escape hatch. New typed branches are added before the `raw` branch so the escape stays as the final fallback.

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
