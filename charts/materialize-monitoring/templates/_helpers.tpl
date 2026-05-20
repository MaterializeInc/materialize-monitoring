{{/* Helm template helpers. */}}

{{/*
Expand the name of the chart.
*/}}
{{- define "mzmon.name" -}}
  {{- default $.Chart.Name $.Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this
(by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "mzmon.fullname" -}}
  {{- if $.Values.fullnameOverride -}}
    {{- $.Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
  {{- else -}}
    {{- $name := default $.Chart.Name $.Values.nameOverride -}}
    {{- if contains $name $.Release.Name -}}
      {{- printf "%s" $.Release.Name | trunc 63 | trimSuffix "-" -}}
    {{- else -}}
      {{- printf "%s-%s" $.Release.Name $name | trunc 63 | trimSuffix "-" -}}
    {{- end -}}
  {{- end -}}
{{- end -}}

{{/*
Chart name and version as used by the chart label.
*/}}
{{- define "mzmon.chart" }}
  {{- printf "%s-%s" $.Chart.Name $.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "mzmon.selectorLabels" -}}
app.kubernetes.io/name: {{ include "mzmon.name" . }}
app.kubernetes.io/instance: {{ $.Release.Name }}
{{- end }}

{{/*
Common annotations.

Usage:

  annotations:
    {{- include "mzmon.annotations" $ | nindent 4 }}
    less-common: "annotation"
*/}}
{{- define "mzmon.annotations" }}
meta.helm.sh/release-name: {{ $.Release.Name }}
meta.helm.sh/release-namespace: {{ $.Release.Namespace }}
{{- end }}

{{/*
Common labels.

Usage:

  labels:
    {{- include "mzmon.labels" $ | nindent 4 }}
    less-common: "label"
*/}}
{{- define "mzmon.labels" }}
helm.sh/chart: {{ include "mzmon.chart" $ | quote }}
{{ include "mzmon.selectorLabels" $ }}
  {{- if $.Chart.AppVersion }}
app.kubernetes.io/version: {{ $.Chart.AppVersion | quote }}
  {{- end }}
app.kubernetes.io/managed-by: {{ $.Release.Service }}
{{- end }}
