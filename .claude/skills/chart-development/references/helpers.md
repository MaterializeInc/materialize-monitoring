# Helm Helpers ("Named Templates")

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

WARNING: Helm named templates DO NOT use handlebars for its syntax.

```markdown
```handlebars
{{- define "mychart.helperName" -}}
  {{/* helper implementation */}}
{{- end -}}
```

## Helpers are global

There is no scoping or imports for helpers.
Including subcharts implicitly loads that chart's helpers.

## Always use a prefix for helpers

To scope helpers, generally use a well-defined prefix.
It's common to use the project name.

```handlebars
{{- define "mychart.helperName" -}}
  {{/* helper implementation */}}
{{- end -}}
```

## Use pascalCase for helper names

Use pascalCase (first lower, single upper for each new word) for consistency.

```handlebars
{{- define "mychart.helperName" -}}
  {{/* helper implementation */}}
{{- end -}}
```

## Style Guidelines for Helpers

### Use spaces within braces and parentheses

Prefer to use a single space after `{{`, `{{-`, `{{/*` and before `}}`, `-}}`, `*/}}` for readability.

```handlebars
{{- define "mychart.helperName" -}}
  {{/* helper implementation */}}
{{- end -}}
```

When calling methods, use a single space after the `(` and before the `)`.

```handlebars
{{- include "mychart.helperName" ( dict "arg1" "value1" "arg2" "value2" ) -}}
```

### For multi-line function calls, put indent the arguments on separate lines

For readability, put each argument on a separate line and indent them when calling functions with multiple arguments.

For dicts, prefer to put the keys and values on the same line.

```handlebars
{{- include "mychart.helperNameWithArgs" ( dict
  "arg1" "value1"
  "arg2" "value2"
  "context" $
) -}}
```

### Trim whitespace by default

Use `{{-` and `-}}` to trim whitespace by default for helpers for cleaner output.
This causes all whitespace before and after the helper to be removed.

You should skip the trailing `}}` if you want to have a subsequent block be
rendered with its spacing.

```handlebars
{{- define "mychart.helperName" }}
someValue:
  {{- if $.Values.someCondition }}
  condOne: 1
  {{- else }}
  condTwo: 2
  {{- end -}}
{{- end -}}
```

### Use logically nested indentation for conditions, loops, and sub-contexts

Helpers should use 2-space indentation and be further indented for each
sub-context / logical block (e.g., `if`, `range`, `with`, etc.) for readability.

Note that content and logic are separately indented.

```handlebars
{{- define "mychart.helperName" -}}
  {{- if $.Values.someCondition -}}
    {{- range $.Values.someList -}}
      {{/* helper implementation */}}
    {{- end -}}
  {{- else -}}
    {{/* helper implementation */}}
  {{- end -}}
{{- end -}}
```

## Prefer using the root context

Helm has two contexts: the root context (`$`) and the current context (`.`).
The root context (`$`) contains the top-level context by default, and some
functions such as `range` will enter a sub-context.
Until a sub-context is entered, both `$` and `.` refer to the same context.

Many helm charts will use `.` and `$` interchangeably until the current context
is changed, but that is very inconsistent.

Use `$.Values` instead of `.Values`.

### Exception for helm helpers that take additional arguments

Some helpers will take additional arguments and pass a `( dict )` with some
named arguments.
In this case, you will want to create a variable named `$context` to hold
the root context for use in the helper.

Use `$context.Values` instead of `$.Values` in this case.

```handlebars
{{/*
Args:
  arg1: description of arg1
  arg2: description of arg2
  context: the root context (usually $)

Usage: {{- include "mychart.helperName" (dict "arg1" arg1 "arg2" arg2 "context" $)}}
*/}}
{{- define "mychart.helperName" -}}
  {{- $context := $.context -}}
  {{/* helper implementation */}}
  {{- someVar := $context.Values.someValue -}}
{{- end -}}
```

## Always Pass Context

When calling a helper, always pass a root context (`$`) to ensure it has access to all necessary values.

```handlebars
{{- include "mychart.helperName" $ -}}

{{- include "mychart.helperNameWithArgs" ( dict "arg1" "value1" "arg2" "value2" "context" $ ) -}}
```
