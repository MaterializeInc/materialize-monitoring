{{- /*
  Overrides the llms-txt module's `_default/single.md`.
  See `list.md` for why we render shortcodes instead of emitting raw content.
*/ -}}
# {{ .Title }}
{{ with .Date }}
date: {{ .Format "2006-01-02" }}
{{- end }}
{{ with .Params.description }}
> {{ . }}
{{- end }}
{{- with .RenderShortcodes }}

{{ . }}
{{- end }}
