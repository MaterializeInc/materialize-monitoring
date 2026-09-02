{{- /*
  Overrides the llms-txt module's `layouts/index.md` (the home page's Markdown
  output). See `list.md` for why we render shortcodes instead of emitting raw
  content; the module's `{{ site.Params.description }}` fallback is dropped for
  the same `<no value>` reason.
*/ -}}
# {{ .Title | default site.Title }}
{{ with site.Params.metadata.description | default site.Params.description }}
> {{ . }}
{{- end }}
{{- with .RenderShortcodes }}

{{ . }}
{{- end }}
