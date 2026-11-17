{{/*
Helm template helpers for the load-testing chart.

Every object this chart renders belongs to one role, so the role-scoped helpers
take `( dict "root" $ "role" $role )` and read the chart through `.root`.

Prefixed `mzmon.bench.` so these never collide with the umbrella chart's
`mzmon.` helpers if both are ever rendered together.
*/}}

{{- /*
The roles `gen-load` accepts, as a JSON list.
*/}}
{{- define "mzmon.bench.roles" -}}
  {{- list "environmentd" "clusterd" "balancerd" "orchestratord" "console" | toJson }}
{{- end }}

{{- /*
Fails on a key under `roles` that is no role, which would otherwise render
nothing and say nothing.

Usage:

  {{- include "mzmon.bench.validate" $ }}
*/}}
{{- define "mzmon.bench.validate" -}}
  {{- $roles := include "mzmon.bench.roles" $ | fromJsonArray }}
  {{- range $key, $_ := $.Values.roles }}
    {{- if not ( has $key $roles ) }}
      {{- fail ( printf "roles.%s is not a role; want one of %s" $key ( join ", " $roles ) ) }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
A role's object name: the release name, then the role.

At most 52 characters, so a StatefulSet's `controller-revision-hash` label, the
name plus 11, stays within 63. Only the release name is truncated, so the role
suffix survives whole and no two roles can share a name.
*/}}
{{- define "mzmon.bench.name" -}}
  {{- $prefix := .root.Release.Name | trunc ( sub 51 ( len .role ) | int ) | trimSuffix "-" }}
  {{- printf "%s-%s" $prefix .role }}
{{- end }}

{{- /*
Chart name and version as used by the chart label.
*/}}
{{- define "mzmon.bench.chart" -}}
  {{- printf "%s-%s" $.Chart.Name $.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- /*
Selector labels for one role.
*/}}
{{- define "mzmon.bench.selectorLabels" -}}
app.kubernetes.io/name: mz-monitoring-benchmark
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .role }}
{{- end }}

{{- /*
Common labels for one role.

Usage:

  labels:
    {{- include "mzmon.bench.labels" ( dict "root" $ "role" $role ) | nindent 4 }}
*/}}
{{- define "mzmon.bench.labels" -}}
helm.sh/chart: {{ include "mzmon.bench.chart" .root | quote }}
{{ include "mzmon.bench.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .root.Release.Service }}
{{- end }}

{{- /*
The image tag: `image.tag`, or else the crate's version.

`files/Cargo.toml` is a symlink to the crate's manifest, so a benchmark-images
release, which bumps the crate and publishes the image at that version, moves
this default too. Helm follows the symlink when it loads the chart from a
checkout.
*/}}
{{- define "mzmon.bench.imageTag" -}}
  {{- if $.Values.image.tag }}
    {{- $.Values.image.tag }}
  {{- else }}
    {{- $line := regexFind "(?m)^version = \"[^\"]+\"" ( $.Files.Get "files/Cargo.toml" ) }}
    {{- if not $line }}
      {{- fail "files/Cargo.toml has no version: it is a symlink to packages/mz-monitoring-benchmark/Cargo.toml, so install the chart from a checkout, or set image.tag" }}
    {{- end }}
    {{- $line | trimPrefix "version = \"" | trimSuffix "\"" }}
  {{- end }}
{{- end }}

{{- /*
The container name the real process runs under.

The gateway chooses its log parsing (multiline panics, the JSON and text
formats) by container name, and the operator's container is
`materialize-operator`, not its binary's name.
*/}}
{{- define "mzmon.bench.containerName" -}}
  {{- get ( dict "orchestratord" "materialize-operator" ) .role | default .role }}
{{- end }}

{{- /*
The endpoints the real process serves, as a JSON list.
*/}}
{{- define "mzmon.bench.paths" -}}
  {{- if eq .role "environmentd" }}
    {{- list "/metrics" "/metrics/mz_compute" "/metrics/mz_frontier" "/metrics/mz_storage" "/metrics/mz_usage" | toJson }}
  {{- else }}
    {{- list "/metrics" | toJson }}
  {{- end }}
{{- end }}

{{- /*
Whether a role renders: present, and with at least one replica. Empty when it
does not, so it reads as false in an `if`.
*/}}
{{- define "mzmon.bench.enabled" -}}
  {{- $role := get .root.Values.roles .role }}
  {{- if and $role ( gt ( int ( dig "replicas" 0 $role ) ) 0 ) }}true{{ end }}
{{- end }}
