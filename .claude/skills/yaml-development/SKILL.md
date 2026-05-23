---
name: yaml-development
description: |
  This skill should be used when making changes to files with the `.yaml` or `.kyaml` extension.
---

# YAML Development

## YAML Best Practices

YAML is quite unopionated about formatting by default, so we offer strong opinions
to ensure that we are consistent.

Note that while helm templates use the `.yaml` extension, they are not strictly
valid YAML and helm template rules supercede our normal YAML best practices.

YAML files should use the `.yaml` extension unless `.yml` is strictly required.

### YAML Examples

See [references/yaml-comprehensive.yaml](references/yaml-comprehensive.yaml) for a
comprehensive example of YAML best practices.

Also see `.yamllint.yaml` in the root of this repo for the linting rules
that enforce these best practices.

### YAML Linting

`yamllint` enforces these best practices and runs automatically via
[pre-commit](../../../.pre-commit-config.yaml) on `.yaml` / `.yml`
files. Configuration lives in `.yamllint.yaml` at the repo root.

Notes on the configuration intentionally trade enforcement for lower toil:

- **`quoted-strings` is demoted to a warning.** The convention is still
  double-quoted values for strings containing YAML metacharacters
  (`{`, `[`, `:`) or starting with a non-alphanumeric, but it does not
  block commits.
- **`empty-values: forbid-in-block-mappings` is disabled.** This allows
  idiomatic empty block keys like `pull_request:` in GitHub Actions
  workflows and `__mainSection:` sentinels used by helm-docs. Flow
  mappings and sequences are still checked, so `{key:}` or trailing `- `
  in a sequence will still fail.
- **`line-length` is a warning** (max 120, with non-breakable-word allowance).
- Helm chart templates under `charts/*/templates/` are excluded since
  Go templating syntax is not valid YAML. Helm `values.yaml` is **not**
  excluded — author with the conventions above in mind.
- Generated YAML under `charts/*/pre-rendered/` and `docs/assets/dashboards/`
  is excluded globally.

When in doubt, run `uv run yamllint -c .yamllint.yaml <path>` locally
to preview what the hook will say.

## KYAML Best Practices

KYAML is a subset of YAML with stricter rules.

KYAML can be thought of as YAML with required braces and brackets (and no power features)
or as JSON with comments, multiple documents, and trailing commas.

## KYAML Linting

KYAML files (`.kyaml` extension) are linted with the stricter
`.yamllint-kyaml.kyaml` configuration via a separate pre-commit hook
entry. KYAML enforcement is run with `--strict`, so warnings (such as
`line-length`) also block — the lower-toil relaxations applied to
regular YAML do **not** apply here. Write KYAML to the letter.
