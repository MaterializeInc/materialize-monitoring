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

Use `yamllint` to ensure YAML files are properly formatted and follow our best practices.

## KYAML Best Practices

KYAML is a subset of YAML with stricter rules.

KYAML can be thought of as YAML with required braces and brackets (and no power features)
or as JSON with comments, multiple documents, and trailing commas.

## KYAML Linting

Use `yamllint` with the `.yamllint-kyaml.kyaml` configuration to ensure KYAML
files are properly formatted and follow our best practices for KYAML files.
