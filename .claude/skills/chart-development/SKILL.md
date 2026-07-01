---
name: chart-development
description: |
  This skill should be used when developing Helm charts under `charts/*` —
  templates, `values.yaml`, subchart wrapping, profile overlays, helm-docs
  annotations, and helm-unittest tests. Covers more than just templates.
---

# Chart Development

## Overview

Many examples of helm templates on the internet are not well-suited for production use
and vary by quality greatly.
This skill brings strong opinions about quality and maintainability of helm templates
and how to test them.

## YAML Best Practices

For YAML best practices, consult the [yaml-development skill](../yaml-development/SKILL.md)
which generally applies to helm templates.
Do note that helm templates are not strictly valid YAML files and helm template rules
supercede normal YAML rules.

### Linting scope

- `charts/*/templates/**` is **excluded** from `check-yaml` and `yamllint`
  (Go template syntax is not valid YAML).
- `charts/*/values.yaml`, `Chart.yaml`, and `examples/*.values.yaml` **are**
  linted by yamllint with the repo's default config — write them as
  valid YAML. The `__mainSection:` helm-docs sentinel pattern is
  explicitly allowed (see [yaml-development](../yaml-development/SKILL.md#yaml-linting)).
- `charts/*/pre-rendered/**` is excluded globally as generated output.

## Helm Values Best Practices

Helm values (`values.yaml`) are the main inputs for a user to configure a chart.

We use `values.yaml` as the source of truth for our configuration and also
where we document all parameters that are exposed to a chart's README.md.

### README.md Generation from values.yaml

We use [`helm-docs`](https://github.com/norwoodj/helm-docs) to generate
`README.md` (and the docsite values reference page) from each chart's
`values.yaml` plus a `README.md.gotmpl` template.

`helm-docs` is pinned via the `tool` directive in `go.mod` and invoked
through Make: `make charts/<chart>/README.md` regenerates the chart
README, and `make helm-docs` regenerates all helm-docs outputs (chart
README + docsite reference).

The pre-commit hook in `.pre-commit-config.yaml` reruns the same Make
targets when any of the source files change, so the generated outputs
stay in sync with `values.yaml`.

#### Annotation conventions

- `# -- <description>` directly above a key documents that key. The **first
  line** is the short descriptor shown in the table; any following comment
  lines (up to the value) are appended as additional description.
- **Only keys with a leading `# --` are rendered.** A key with no `# --`
  (or only plain `#` comments) is omitted from the docs. Use plain `#`
  comments for notes aimed at `values.yaml` readers that should *not* appear
  in the generated table — e.g. per-list-item caveats inside a block.
- For a **large block or list**, put a single `# --` on the parent key; the
  whole default object/list is rendered as its value. Nested keys inside it
  do not each need a `# --`.
- `# @notationType -- (sectionstart) <section title>` starts a new section automatically.
  This is a feature that is not present in `helm-docs` but we implemented
  within `tools/chartlib/helm-docs-lib.gotmpl` to allow for better organization of the generated documentation.
  Sections have no explicit end — the next `(sectionstart)` closes the prior one.
- `# @default -- <override>` overrides the auto-rendered default
  (useful when the literal default is `{}` / `[]` or large/complex).
- `# @raw` lets a description carry multi-line raw markdown.
- After editing, run `make helm-docs` and eyeball the generated table — it is
  the fastest way to confirm your descriptions render and read sensibly.

Section prose within `values.yaml`. It generally needs a `@raw` annotation.
See [references/values.example.yaml](references/values.example.yaml)
and [references/readme.example.md](references/readme.example.md) for
the conventions in practice.

## Wrapping Upstream Subcharts

When this chart wraps an upstream subchart (e.g. `loki`, `grafana`, `thanos`),
its values live under the subchart's alias key and are **deep-merged over the
subchart's own `values.yaml` defaults**. A few non-obvious consequences:

- **Override only your deltas.** Your own validators and templates see the
  *merged* values, so you can rely on the subchart's defaults being present —
  don't restate the whole structure. Set only what differs from upstream or
  from a plain default.
- **Clearing a subchart default needs `null`, not `{}` / `[]`.** An empty map
  or list contributes nothing to the merge, so the subchart's default
  **survives**. To actually remove a default (e.g. drop a chart's built-in
  hard pod anti-affinity so your own soft rule can win), set the key to
  `null`. You can null a nested list too, e.g.
  `affinity.podAntiAffinity.requiredDuringSchedulingIgnoredDuringExecution: null`.
  Always confirm with `helm template … --show-only <path>` — this is easy to
  get wrong and silently ineffective.
- **Subcharts ship their own validation templates** (`templates/validate.yaml`
  and friends) that run against the merged values at render time and can
  `fail` your entire render. Read them before assuming a mode "just works."
  Example: the `grafana/loki` chart defaults distributed components to
  `replicas: 0` (so they deploy nothing until you set them) *and* defaults the
  Simple Scalable `read`/`write`/`backend` targets to `3` — its `validate.yaml`
  then refuses to render because both topologies look active. In Distributed
  mode you must zero the SSD targets. Render early and often to catch these.
- **Per-component value shapes may differ within one subchart.** e.g. in the
  loki chart the ingester's persistence uses a `claims:` list while the
  compactor/index-gateway/ruler use a flat `persistence.size`. Check the
  subchart `values.yaml` per component rather than assuming uniformity.

## Helm Template Best Practices

### Helpers ("Named Templates")

Helper functions (canonically named
["named templates"](https://helm.sh/docs/chart_template_guide/named_templates),
but that is confusing with general use of "template") are methodic snippets which
can be reused across templates.

These should be defined inside of `_*.tpl` files.

### Helpers in Markdown

When writing markdown referencing helpers / named templates, use `handlebars`
for code blocks.
This is closest to go templating syntax supported in Github's highlighting
engine (highlight.js).

```markdown
```handlebars
{{- define "mychart.helpername" -}}
  {{/* helper implementation */}}
{{- end -}}
```

## Testing Helm Charts

Consult [references/testing.md](references/testing.md) for implementation details
and best practices for testing.

Unit tests should be written and updated as templates are updated.

Snapshot tests should be generally updated as part of feature changes, but
do require careful reviews on the changesets to ensure that the changes are
expected and correct.

Helm unittests are safe to run locally, do not require a live Kubernetes cluster,
and do run quite quickly, so they should be run frequently during development.

### Installing Helm Unittest Plugin

Helm unittest is a BDD plugin for writing and testing helm unit tests.

`helm plugin install https://github.com/helm-unittest/helm-unittest`

See notes in [references/testing.md## Installing Helm Unittest Plugin](references/testing.md)
about workarounds for installation issues and verifying installation.

### Unit Test Layout

Helm unittest does not support recursive directories by default, so prefer
to place all unit tests in a single flat directory within charts/*/tests/
(sibling to the templates/ directory) using the default `*_test.yaml` pattern.
Prefer to use one unit test file per resource template.

### Path resolution gotchas

- A test's `template:` path is resolved relative to the chart's `templates/`
  directory — so a **subchart** template is `../charts/<sub>/templates/...`.
- A test's `values:` (profile overlay) path is resolved relative to the
  **`tests/`** directory — so a repo `profiles/foo.values.yaml` is
  `../profiles/foo.values.yaml`. These two bases differ; mixing them up is a
  common footgun.
- Loading a size/shape profile via `values: [../profiles/<name>.values.yaml]`
  plus a shared `set:` block (for enablement, bucket names, required
  selectors) is the clean way to test profile overlays.
- Targeting a template that renders **zero documents** (e.g. a validate-only
  template that only `fail`s or emits nothing) works with `notFailedTemplate`,
  but do **not** add `documentIndex` or document-scoped asserts — they error
  with "document index 0 is out of range."
