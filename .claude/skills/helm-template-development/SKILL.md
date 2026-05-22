---
name: helm-template-development
description: |
  This skill should be used when making changes to `charts/*/templates/`
  or `charts/*/values.yaml` which are the underlying kubernetes resource
  templates and configurations used to generate resources in a helm release.
---

# Helm Template Development

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

- `# -- <description>` directly above a key documents that key.
- `# @section -- <Section name>` placed in the same comment block as
  the `# --` assigns the key to a named section in the rendered table.
  **There is no propagation** — every documented key needs its own
  `# @section --` if you want sections.
- `# @default -- <override>` overrides the auto-rendered default
  (useful when the literal default is `{}` / `[]` or large/complex).
- `# @raw` lets a description carry multi-line raw markdown.

Section prose lives in the gotmpl templates, not in `values.yaml`.
See [references/values.example.yaml](references/values.example.yaml)
and [references/readme.example.md](references/readme.example.md) for
the conventions in practice.

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
