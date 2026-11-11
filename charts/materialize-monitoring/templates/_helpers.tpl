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
The components `otherReleases` can name, each with the helper that says whether
this release runs it.

Usage:
  {{- $components := include "mzmon.otherReleases.components" $ | fromYaml }}
*/}}
{{- define "mzmon.otherReleases.components" }}
alloy-agent: mzmon.alloyAgent.enabled
alloy-gateway: mzmon.alloyGateway.enabled
loki: mzmon.loki.enabled
thanos: mzmon.thanos.enabled
alertmanager: mzmon.alertmanager.enabled
{{- end }}

{{- /*
Whether another release runs a component in place of this one: it is named in
`otherReleases` and this release does not run it.

A component this release runs is this release's, whatever `otherReleases` says,
so its own checks keep applying to it.

Returns a truthy string when it does, empty when not.

Usage:
  {{- if ( include "mzmon.otherReleases.runs" ( dict "root" $ "component" "thanos" ) ) }}
*/}}
{{- define "mzmon.otherReleases.runs" }}
  {{- $root := .root | required ".root must be specified" }}
  {{- $component := .component | required ".component must be specified" }}
  {{- $helper := index ( include "mzmon.otherReleases.components" $root | fromYaml ) $component | required ( printf "%s is not a component otherReleases can name" $component ) }}
  {{- if and ( dig $component false ( $root.Values.otherReleases | default dict ) ) ( not ( include $helper $root ) ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Whether a component's Services exist in the cluster: this release runs it, or
`otherReleases` says another release does.

For the checks that ask whether an address reaches anything. A check that
compares this release's settings for the component asks `mzmon.<component>.enabled`
instead, because another release's settings are not in these values.

Returns a truthy string when they do, empty when not.

Usage:
  {{- if ( include "mzmon.deployed" ( dict "root" $ "component" "thanos" ) ) }}
*/}}
{{- define "mzmon.deployed" }}
  {{- $root := .root | required ".root must be specified" }}
  {{- $component := .component | required ".component must be specified" }}
  {{- $helper := index ( include "mzmon.otherReleases.components" $root | fromYaml ) $component | required ( printf "%s is not a component otherReleases can name" $component ) }}
  {{- if or ( include $helper $root ) ( include "mzmon.otherReleases.runs" . ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Validate `otherReleases`.

An unknown key is an error rather than ignored: it is a component name the
operator expected to silence a check with, and the check it names fails anyway,
pointing at the wrong thing.

Usage:
  {{- $res := include "mzmon.otherReleases.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.otherReleases.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $components := include "mzmon.otherReleases.components" $ | fromYaml }}

  {{- range $component, $declared := ( $.Values.otherReleases | default dict ) }}
    {{- $helper := index $components $component }}
    {{- if not $helper }}
      {{- $errors = append $errors ( printf "otherReleases.%s is not a component this chart runs. Name one of %s." $component ( keys $components | sortAlpha | join ", " ) ) }}
    {{- else if and $declared ( include $helper $ ) }}
      {{- $warnings = append $warnings ( printf "otherReleases.%s is true, but this release runs %s as well, so it has no effect: this release's addresses and checks apply to its own %s. Turn the component off here, or set otherReleases.%s: false." $component $component $component $component ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

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

  {{- $res := include "mzmon.otherReleases.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.loki.validate" $ | fromYaml }}
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

  {{- $res = include "mzmon.alertmanager.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.priorityClass.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.networkPolicy.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.networkPolicy.validate.collectors" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.certificates.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.rules.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res = include "mzmon.postgresExporter.validate" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- /* Last, so the overrides see every check's output. */}}
  {{- include "mzmon.validate.overrides" ( dict "root" $ "errors" $errors "warnings" $warnings ) }}
{{- end }}

{{- /*
Apply `footgunOverrides` to the collected validation report.

Downgrading happens before silencing, and a downgraded error is exempt from
`silenceWarnings`: the chart still considers that configuration broken, so the
report keeps a trace of it however broad the silencing is.

An override that is not a list of strings is an error, and no override can
downgrade it. An expression that does not compile fails the render from
`mustRegexMatch`, naming the expression.

Usage:
  {{- include "mzmon.validate.overrides" ( dict "root" $ "errors" $errors "warnings" $warnings ) }}
*/}}
{{- define "mzmon.validate.overrides" }}
  {{- $overrides := .root.Values.footgunOverrides | default dict }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $invalid := list }}

  {{- $lists := dict }}
  {{- range $key := list "downgradeErrors" "silenceWarnings" }}
    {{- $entries := index $overrides $key | default list }}
    {{- $valid := list }}
    {{- if not ( kindIs "slice" $entries ) }}
      {{- $invalid = append $invalid ( printf "footgunOverrides.%s is %s. Set it to a list of regular expressions." $key ( toJson $entries ) ) }}
    {{- else }}
      {{- range $i, $entry := $entries }}
        {{- if and ( kindIs "string" $entry ) $entry }}
          {{- $valid = append $valid $entry }}
        {{- else }}
          {{- $invalid = append $invalid ( printf "footgunOverrides.%s[%d] is %s. Each entry is a regular expression, as a non-empty string." $key $i ( toJson $entry ) ) }}
        {{- end }}
      {{- end }}
    {{- end }}
    {{- $_ := set $lists $key $valid }}
  {{- end }}

  {{- $used := dict }}
  {{- $downgraded := list }}
  {{- range $message := .errors }}
    {{- $by := "" }}
    {{- range $pattern := $lists.downgradeErrors }}
      {{- if not $by }}
        {{- if mustRegexMatch $pattern $message }}
          {{- $by = $pattern }}
        {{- end }}
      {{- end }}
    {{- end }}
    {{- if $by }}
      {{- $_ := set $used $by true }}
      {{- $downgraded = append $downgraded ( printf "%s (An error, downgraded by footgunOverrides.downgradeErrors entry %q.)" $message $by ) }}
    {{- else }}
      {{- $errors = append $errors $message }}
    {{- end }}
  {{- end }}

  {{- $stale := list }}
  {{- range $pattern := $lists.downgradeErrors }}
    {{- if not ( hasKey $used $pattern ) }}
      {{- $stale = append $stale ( printf "footgunOverrides.downgradeErrors entry %q matches no error. Remove it, so it can't downgrade an error you haven't seen yet." $pattern ) }}
    {{- end }}
  {{- end }}

  {{- range $message := concat .warnings $stale }}
    {{- $silenced := false }}
    {{- range $pattern := $lists.silenceWarnings }}
      {{- if and ( not $silenced ) ( mustRegexMatch $pattern $message ) }}
        {{- $silenced = true }}
      {{- end }}
    {{- end }}
    {{- if not $silenced }}
      {{- $warnings = append $warnings $message }}
    {{- end }}
  {{- end }}

  {{- dict "errors" ( concat $invalid $errors ) "warnings" ( concat $downgraded $warnings ) | toYaml }}
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
