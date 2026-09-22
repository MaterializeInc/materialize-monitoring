{{/*
Helm template helpers.

Deliberately a local copy rather than anything shared with the umbrella chart:
Helm has no way to import a `.tpl` across charts without making one a dependency
of the other, and a dependency is exactly what the split exists to avoid. The
names are prefixed `mzmond.` so that the two never collide if a future install
does end up rendering both in one release.
*/}}

{{- define "mzmond.name" -}}
  {{- default $.Chart.Name $.Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- /*
Fully qualified app name, truncated to the 63 characters a DNS label allows.
If the release name already contains the chart name it is used as-is.
*/}}
{{- define "mzmond.fullname" }}
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

{{- define "mzmond.chart" }}
  {{- printf "%s-%s" $.Chart.Name $.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "mzmond.namespace" -}}
  {{- default $.Release.Namespace $.Values.namespaceOverride -}}
{{- end }}

{{- define "mzmond.selectorLabels" -}}
app.kubernetes.io/name: {{ include "mzmond.name" . }}
app.kubernetes.io/instance: {{ $.Release.Name }}
{{- end }}

{{- define "mzmond.annotations" -}}
meta.helm.sh/release-name: {{ $.Release.Name }}
meta.helm.sh/release-namespace: {{ $.Release.Namespace }}
  {{- with $.Values.commonAnnotations }}
{{ toYaml . }}
  {{- end }}
{{- end }}

{{- define "mzmond.labels" -}}
helm.sh/chart: {{ include "mzmond.chart" $ | quote }}
{{ include "mzmond.selectorLabels" $ }}
  {{- if $.Chart.AppVersion }}
app.kubernetes.io/version: {{ $.Chart.AppVersion | quote }}
  {{- end }}
app.kubernetes.io/managed-by: {{ $.Release.Service }}
  {{- with $.Values.commonLabels }}
{{ toYaml . }}
  {{- end }}
{{- end }}

{{- /*
Names of the dashboards `selected` resolves to.

Globs the rendered dashboards once so the resources and the install notes cannot
disagree about what was installed. Returns a YAML list.

Usage:
  {{- range $name := include "mzmond.dashboards" $ | fromYamlArray }}
*/}}
{{- define "mzmond.dashboards" }}
  {{- $names := list }}
  {{- range $selectPattern := $.Values.selected }}
    {{- range $path, $_ := $.Files.Glob ( printf "pre-rendered/dashboards/grafana/%s.yaml" $selectPattern ) }}
      {{- $name := base $path | trimSuffix ".yaml" | lower | replace "_" "-" }}
      {{- if not ( has $name $names ) }}
        {{- $names = append $names $name }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $names | toYaml }}
{{- end }}

{{- /*
Whether this chart should render anything at all.

Returns a truthy string when it should, empty when not.

Usage:
  {{- if ( include "mzmond.enabled" $ ) }}
*/}}
{{- define "mzmond.enabled" }}
  {{- if and $.Values.grafana.enabled ( eq $.Values.grafana.mode "operator" ) }}
    {{- "true" }}
  {{- end }}
{{- end }}
