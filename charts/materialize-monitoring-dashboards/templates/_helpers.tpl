{{/*
Helm template helpers.

Deliberately a local copy rather than anything shared with the umbrella chart:
Helm has no way to import a `.tpl` across charts without making one a dependency
of the other, and a dependency is exactly what the split exists to avoid.

Prefixed `mzmon-dashboards.` — the chart's own name — so the two never collide if
some future install renders both in one release.
*/}}

{{- define "mzmon-dashboards.name" -}}
  {{- default $.Chart.Name $.Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- /*
Fully qualified app name, truncated to the 63 characters a DNS label allows.
If the release name already contains the chart name it is used as-is.
*/}}
{{- define "mzmon-dashboards.fullname" }}
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

{{- define "mzmon-dashboards.chart" }}
  {{- printf "%s-%s" $.Chart.Name $.Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "mzmon-dashboards.namespace" -}}
  {{- default $.Release.Namespace $.Values.namespaceOverride -}}
{{- end }}

{{- define "mzmon-dashboards.selectorLabels" -}}
app.kubernetes.io/name: {{ include "mzmon-dashboards.name" . }}
app.kubernetes.io/instance: {{ $.Release.Name }}
{{- end }}

{{- define "mzmon-dashboards.annotations" -}}
meta.helm.sh/release-name: {{ $.Release.Name }}
meta.helm.sh/release-namespace: {{ $.Release.Namespace }}
  {{- with $.Values.commonAnnotations }}
{{ toYaml . }}
  {{- end }}
{{- end }}

{{- define "mzmon-dashboards.labels" -}}
helm.sh/chart: {{ include "mzmon-dashboards.chart" $ | quote }}
{{ include "mzmon-dashboards.selectorLabels" $ }}
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
  {{- range $name := include "mzmon-dashboards.dashboards" $ | fromYamlArray }}
*/}}
{{- define "mzmon-dashboards.dashboards" }}
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
  {{- if ( include "mzmon-dashboards.enabled" $ ) }}
*/}}
{{- define "mzmon-dashboards.enabled" }}
  {{- if and $.Values.grafana.enabled ( eq $.Values.grafana.mode "operator" ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Everything the render refuses, collected as {errors, warnings}.

Same shape as the umbrella chart's `mzmon.validate.collect`, so `validate.yaml`
and `NOTES.txt` read one result rather than each re-deriving it. Split per
concern so a check can be read, tested and extended on its own.

Usage:
  {{- $res := include "mzmon-dashboards.validate.collect" $ | fromYaml }}
*/}}
{{- define "mzmon-dashboards.validate.collect" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $res := include "mzmon-dashboards.validate.instanceSelector" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon-dashboards.validate.mode" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon-dashboards.validate.selection" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon-dashboards.validate.folderUids" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
The selector has to select something.

grafana-operator reads an empty `matchLabels` as *every* Grafana instance, not
none, and it watches all namespaces by default — so an empty selector pushes
these dashboards into every Grafana in the cluster rather than into none of them.
*/}}
{{- define "mzmon-dashboards.validate.instanceSelector" }}
  {{- $errors := list }}
  {{- if $.Values.grafana.enabled }}
    {{- $selector := $.Values.grafana.instanceSelector | default dict }}
    {{- if not ( or $selector.matchLabels $selector.matchExpressions ) }}
      {{- $errors = append $errors "grafana.instanceSelector is empty. grafana-operator reads that as *every* Grafana instance in every namespace it watches, not as none — set the labels the materialize-monitoring release put on its Grafana resource." }}
    {{- end }}
  {{- end }}
  {{- dict "errors" $errors "warnings" list | toYaml }}
{{- end }}

{{- /*
Only the operator mode renders anything here.

`standalone` provisions dashboards through the Grafana chart's own ConfigMap
sidecar, which this chart does not render — and which is subject to the same
1 MiB object ceiling the split exists to escape. GrafanaManifests would apply
cleanly and be read by nothing.
*/}}
{{- define "mzmon-dashboards.validate.mode" }}
  {{- $errors := list }}
  {{- if and $.Values.grafana.enabled ( not ( eq $.Values.grafana.mode "operator" ) ) }}
    {{- $errors = append $errors ( printf "grafana.mode is %q. This chart only supports \"operator\" — a standalone Grafana provisions dashboards through its own ConfigMap sidecar, which is subject to the same 1 MiB ceiling this split exists to escape. Set grafana.enabled=false if that is deliberate." $.Values.grafana.mode ) }}
  {{- end }}
  {{- dict "errors" $errors "warnings" list | toYaml }}
{{- end }}

{{- /*
A selection that matches nothing installs nothing while reporting success, which
is far more often a mistyped pattern than an intent.
*/}}
{{- define "mzmon-dashboards.validate.selection" }}
  {{- $errors := list }}
  {{- if ( include "mzmon-dashboards.enabled" $ ) }}
    {{- if not ( include "mzmon-dashboards.dashboards" $ | fromYamlArray ) }}
      {{- $errors = append $errors ( printf "selected (%s) matches none of the dashboards this chart carries. Patterns are filename stems — `env-*`, `infra-*` — matched against pre-rendered/dashboards/grafana/." ( join ", " $.Values.selected ) ) }}
    {{- end }}
  {{- end }}
  {{- dict "errors" $errors "warnings" list | toYaml }}
{{- end }}

{{- /*
A folder a selected dashboard asks for, with no UID configured for it.

A **warning**, not an error: dropping the annotation is a supported outcome — it
files that dashboard at the root — and emptying `folderUids` entirely is the
documented way to ask for exactly that. What it is not is something to discover
by noticing a dashboard in the wrong place, so the render says which.
*/}}
{{- define "mzmon-dashboards.validate.folderUids" }}
  {{- $warnings := list }}
  {{- if ( include "mzmon-dashboards.enabled" $ ) }}
    {{- $uids := $.Values.grafana.folderUids | default dict }}
    {{- $missing := list }}
    {{- range $name := include "mzmon-dashboards.dashboards" $ | fromYamlArray }}
      {{- $body := $.Files.Get ( printf "pre-rendered/dashboards/grafana/%s.yaml" $name ) | fromYaml }}
      {{- $folder := get ( get ( get $body "metadata" | default dict ) "annotations" | default dict ) "grafana.app/folder" }}
      {{- if and $folder ( not ( get $uids $folder ) ) }}
        {{- $missing = append $missing ( printf "%s -> %s" $name $folder ) }}
      {{- end }}
    {{- end }}
    {{- if $missing }}
      {{- $warnings = append $warnings ( printf "No grafana.folderUids entry for: %s. Those dashboards will be filed at the root of the Grafana. Add the UID the materialize-monitoring release created (its install notes print them), or leave this if the root is what you want." ( join ", " $missing ) ) }}
    {{- end }}
  {{- end }}
  {{- dict "errors" list "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
validate.yaml template writer: fail on errors, emit warnings as YAML comments.
*/}}
{{- define "mzmon-dashboards.validate" }}
  {{- $res := include "mzmon-dashboards.validate.collect" $ | fromYaml }}

  {{- range $res.warnings }}
    {{- printf "# WARNING: %s\n" . }}
  {{- end }}

  {{- if $res.errors }}
    {{- printf "Validation failed:\n%s" ( join "\n" $res.errors ) | fail }}
  {{- end }}
{{- end }}

{{- /*
Validation entrypoint inside NOTES.txt.

Usage:
  {{- include "mzmon-dashboards.validate.format" $ | nindent 0 }}
*/}}
{{- define "mzmon-dashboards.validate.format" }}
  {{- $res := include "mzmon-dashboards.validate.collect" $ | fromYaml }}

  {{- range $res.warnings }}
    {{- printf "**WARNING**: %s\n" . }}
  {{- end }}

  {{- range $res.errors -}}
    {{- printf "**ERROR**: %s\n" . }}
  {{- end }}
{{- end }}
