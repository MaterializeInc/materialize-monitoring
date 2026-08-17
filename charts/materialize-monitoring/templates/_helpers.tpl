{{/* Helm template helpers. */}}

{{- /*
Expand the name of the chart.

For umbrella charts, a smaller name may be used (mzmon) in prefix contexts.
*/}}
{{- define "mzmon.name" -}}
  {{- default $.Chart.Name $.Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- /*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this
(by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "mzmon.fullname" }}
  {{- $defaultName := "mzmon" }}
  {{- if $.Values.fullnameOverride }}
    {{- $.Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
  {{- else }}
    {{- $name := $.Values.nameOverride | default $.Chart.Name }}
    {{- if contains $name $.Release.Name }}
      {{- printf "%s" $.Release.Name | trunc 63 | trimSuffix "-" }}
    {{- else }}
      {{- printf "%s-%s" $.Release.Name $name | trunc 63 | trimSuffix "-" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Chart name and version as used by the chart label.
*/}}
{{- define "mzmon.chart" }}
  {{- printf "%s-%s" $.Chart.Name $.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- /*
Configured namespace for workloads in this chart.
Note that some subcharts may have their own namespace configuration.
*/}}
{{- define "mzmon.namespace" -}}
  {{- default $.Release.Namespace $.Values.namespaceOverride -}}
{{- end }}

{{- /*
Selector labels.
*/}}
{{- define "mzmon.selectorLabels" -}}
app.kubernetes.io/name: {{ include "mzmon.name" . }}
app.kubernetes.io/instance: {{ $.Release.Name }}
{{- end }}

{{- /*
Common annotations.

Usage:

  annotations:
    {{- include "mzmon.annotations" $ | nindent 4 }}
    less-common: "annotation"
*/}}
{{- define "mzmon.annotations" -}}
meta.helm.sh/release-name: {{ $.Release.Name }}
meta.helm.sh/release-namespace: {{ $.Release.Namespace }}
{{- end }}

{{- /*
Common labels.

Usage:

  labels:
    {{- include "mzmon.labels" $ | nindent 4 }}
    less-common: "label"
*/}}
{{- define "mzmon.labels" -}}
helm.sh/chart: {{ include "mzmon.chart" $ | quote }}
{{ include "mzmon.selectorLabels" $ }}
  {{- if $.Chart.AppVersion }}
app.kubernetes.io/version: {{ $.Chart.AppVersion | quote }}
  {{- end }}
app.kubernetes.io/managed-by: {{ $.Release.Service }}
{{- end }}

{{- /*
Image pull secrets for the workloads this chart renders itself.

Both global spellings are read, because the subcharts do not agree on one and a
consumer should not have to care which: Alloy reads `global.image.pullSecrets`
and never `global.imagePullSecrets`, while Thanos, Grafana, kube-state-metrics
and node-exporter read the latter and never the former. Merging them here is
what makes `global.imagePullSecrets` mean what `values.yaml` says it means.

Entries are accepted as `{name: x}` maps or as bare strings — some charts in the
wild take the second form — and are normalized to the Kubernetes shape and
deduplicated by name, so a consumer that sets both globals to the same Secret
gets one entry rather than two.

Returns the empty string when there is nothing to emit, so a caller can guard on
it without rendering a line of trailing whitespace.

Usage:

  {{- $pullSecrets := include "mzmon.imagePullSecrets" (dict "root" $ "extra" $someList) }}
  {{- if $pullSecrets }}
  imagePullSecrets:
    {{- $pullSecrets | nindent 4 }}
  {{- end }}
*/}}
{{- define "mzmon.imagePullSecrets" -}}
  {{- $global := .root.Values.global | default dict -}}
  {{- $candidates := concat
      ($global.imagePullSecrets | default list)
      (($global.image | default dict).pullSecrets | default list)
      (.extra | default list)
  -}}
  {{- $merged := list -}}
  {{- $seen := dict -}}
  {{- range $candidates -}}
    {{- /*
      An if/else rather than `ternary`, which evaluates both branches eagerly
      and so would blow up on `.name` the moment an entry is a bare string.
    */ -}}
    {{- $name := . -}}
    {{- if kindIs "map" . -}}
      {{- $name = .name -}}
    {{- end -}}
    {{- if and $name (not (hasKey $seen $name)) -}}
      {{- $seen = set $seen $name true -}}
      {{- $merged = append $merged (dict "name" $name) -}}
    {{- end -}}
  {{- end -}}
  {{- if $merged -}}
    {{- toYaml $merged -}}
  {{- end -}}
{{- end -}}

{{- /*
Validation collection.

This is called twice:
  1. in NOTES.txt to be displayed
  2. inside validate.yaml to be templated/actually fail

Usage:
  {{- $res := include "mzmon.validate.collect" $ | fromYaml }}
*/}}
{{- define "mzmon.validate.collect" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $res := include "mzmon.loki.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.grafana.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.thanos.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.alloy.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.priorityClass.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validation entrypoint inside NOTES.txt.

Usage:
  {{- include "mzmon.validate.format" $ | nindent 0 }}
*/}}
{{- define "mzmon.validate.format" }}
  {{- $res := include "mzmon.validate.collect" $ | fromYaml }}

  {{- range $res.warnings }}
    {{- printf "**WARNING**: %s\n" . }}
  {{- end }}

  {{- range $res.errors -}}
    {{- printf "**ERROR**: %s\n" . }}
  {{- end }}
{{- end }}

{{- /*
validation.yaml template writer.

This will emit a failure if any validation errors are actually found.

WARNINGS are written to output (as yaml comments).
*/}}
{{- define "mzmon.validate" }}
  {{- $res := include "mzmon.validate.collect" $ | fromYaml }}

  {{- range $res.warnings }}
    {{- printf "# WARNING: %s\n" . }}
  {{- end }}

  {{- if $res.errors }}
    {{- printf "Validation failed:\n%s" ( join "\n" $res.errors ) | fail }}
  {{- end }}
{{- end }}
