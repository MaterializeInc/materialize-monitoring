{{/*
Helpers take a context of `dict "root" $ "role" <role>`, because every object
this chart renders belongs to one role.
*/}}

{{/* The roles `gen-load` accepts. */}}
{{- define "bench.roles" -}}
{{- list "environmentd" "clusterd" "balancerd" "orchestratord" "console" | toJson }}
{{- end }}

{{/*
Fails on a key under `roles` that is no role, which would otherwise render
nothing and say nothing.
*/}}
{{- define "bench.validate" -}}
{{- $roles := include "bench.roles" . | fromJsonArray }}
{{- range $key, $_ := .Values.roles }}
{{- if not (has $key $roles) }}
{{- fail (printf "roles.%s is not a role; want one of %s" $key (join ", " $roles)) }}
{{- end }}
{{- end }}
{{- end }}

{{/*
A role's object name. At most 52 characters, so a StatefulSet's
`controller-revision-hash` label, its name plus 11, stays within 63.
*/}}
{{- define "bench.name" -}}
{{- printf "%s-%s" .root.Release.Name .role | trunc 52 | trimSuffix "-" }}
{{- end }}

{{- define "bench.selectorLabels" -}}
app.kubernetes.io/name: mz-monitoring-benchmark
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .role }}
{{- end }}

{{- define "bench.labels" -}}
{{ include "bench.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .root.Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .root.Chart.Name .root.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
The container name the real process runs under. The gateway chooses its log
parsing (multiline panics, the JSON and text formats) by container name, and
the operator's container is `materialize-operator`, not its binary's name.
*/}}
{{- define "bench.containerName" -}}
{{- get (dict "orchestratord" "materialize-operator") .role | default .role }}
{{- end }}

{{/* The endpoints the real process serves, as a JSON list. */}}
{{- define "bench.paths" -}}
{{- if eq .role "environmentd" -}}
{{- list "/metrics" "/metrics/mz_compute" "/metrics/mz_frontier" "/metrics/mz_storage" "/metrics/mz_usage" | toJson }}
{{- else -}}
{{- list "/metrics" | toJson }}
{{- end }}
{{- end }}

{{/* Whether a role renders: present, and with at least one replica. */}}
{{- define "bench.enabled" -}}
{{- $r := get .root.Values.roles .role }}
{{- if and $r (gt (int (dig "replicas" 0 $r)) 0) }}true{{ end }}
{{- end }}
