{{- /*
  Overrides the llms-txt module's `_default/list.md`.

  Section indexes here carry frontmatter only (see CLAUDE.md), and the module's
  template falls back to `{{ .Params.description }}` when `.RawContent` is
  empty — which renders a literal `<no value>` when there is no description
  either. Emit the description and body only when they actually exist.
*/ -}}
# {{ .Title }}
{{ with .Params.description }}
> {{ . }}
{{- end }}
{{- with .RawContent }}

{{ . }}
{{- end }}
