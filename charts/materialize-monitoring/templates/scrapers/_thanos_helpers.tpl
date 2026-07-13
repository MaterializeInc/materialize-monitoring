{{- /* Thanos helpers and validators. */}}

{{- /*
Get thanos namespace.

Usage:
  {{- include "mzmon.thanos.namespace" $ }}
*/}}
{{- define "mzmon.thanos.namespace" }}
  {{- $ns := $.Values.thanos.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}
