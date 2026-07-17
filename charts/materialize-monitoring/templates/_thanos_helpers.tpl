{{- /* Thanos helpers and validators. */}}

{{- /*
Check if thanos is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.thanos.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.thanos.enabled" }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "bundled-backends" ) $tags.thanos ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get thanos namespace.

Usage:
  {{- include "mzmon.thanos.namespace" $ }}
*/}}
{{- define "mzmon.thanos.namespace" }}
  {{- $ns := $.Values.thanos.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}
