{{- /*
  Overrides the llms-txt module's `_default/list.md`.

  Two changes from the module's version:

  Section indexes here carry frontmatter only (see CLAUDE.md), and the module
  falls back to `{{ .Params.description }}` when `.RawContent` is empty — which
  renders a literal `<no value>` when there is no description either. Emit the
  description and body only when they actually exist.

  `.RenderShortcodes` rather than `.RawContent`: raw content still holds
  unexpanded `{{< relref >}}` and friends, so the published Markdown would
  carry template calls where its cross-links belong. RenderShortcodes expands
  them and leaves the surrounding Markdown alone.
*/ -}}
# {{ .Title }}
{{ with .Params.description }}
> {{ . }}
{{- end }}
{{- with .RenderShortcodes }}

{{ . }}
{{- end }}
