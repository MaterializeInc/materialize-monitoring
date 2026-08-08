{{/* Priority class helpers. */}}

{{- /*
The subchart value paths that carry a `priorityClassName`, keyed by the subchart
name as it appears in `.Subcharts` (the alias, where one is declared).

The value is the dotted path *below* the subchart key. A subchart absent from
this map is one the chart does not set a priority for; adding a `priorityClassName`
to a subchart block without adding it here only costs the validation below, not
the setting itself.

Loki's two memcached StatefulSets are listed separately from `global` on purpose:
their template reads its own component key and does not fall back to
`loki.global.priorityClassName`.
*/}}
{{- define "mzmon.priorityClass.paths" }}
alloy-agent:
  - controller.priorityClassName
alloy-gateway:
  - controller.priorityClassName
node-exporter:
  - priorityClassName
kube-state-metrics:
  - priorityClassName
metrics-server:
  - priorityClassName
loki:
  - global.priorityClassName
  - chunksCache.priorityClassName
  - resultsCache.priorityClassName
thanos:
  - global.priorityClassName
grafana:
  - priorityClassName
grafana-operator:
  - priorityClassName
alertmanager:
  - priorityClassName
{{- end }}

{{- /*
The set of PriorityClass names this chart creates.

Empty when `priorityClasses.create` is false, which is what makes the validation
below able to tell "we made this" from "you promised this exists".

Usage:
  {{- $names := include "mzmon.priorityClass.created" $ | fromYamlArray }}
*/}}
{{- define "mzmon.priorityClass.created" }}
  {{- $names := list }}
  {{- $pcs := $.Values.priorityClasses | default dict }}
  {{- if $pcs.create }}
    {{- range $tier := ( list "critical" "scalable" ) }}
      {{- $pc := index $pcs $tier | default dict }}
      {{- with $pc.name }}
        {{- $names = append $names . }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $names | toYaml }}
{{- end }}

{{- /*
Entrypoint for priority class validation.

Catches the failure mode that does not degrade gracefully: a subchart pointing
at a PriorityClass name nothing creates. Kubernetes rejects such a pod outright,
so the workload never starts and the only evidence is an admission error on a
ReplicaSet nobody is watching.

Only enabled subcharts are checked — `.Subcharts` holds exactly the dependencies
Helm resolved for this render, so a chart switched off by tag or circuit breaker
cannot raise a warning about a value that will never be read.

Warnings rather than errors throughout: pointing at an externally-managed class
is legitimate, and the chart cannot see the cluster to know whether it exists.

Usage:
  {{- $res := include "mzmon.priorityClass.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.priorityClass.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $created := include "mzmon.priorityClass.created" $ | fromYamlArray }}
  {{- $paths := include "mzmon.priorityClass.paths" $ | fromYaml }}

  {{- range $chart, $chartPaths := $paths }}
    {{- if hasKey $.Subcharts $chart }}
      {{- $values := index $.Values $chart | default dict }}
      {{- range $path := $chartPaths }}
        {{- /* `dig` takes its path as separate arguments, which a template
               cannot spread from a list — so walk the dotted path by hand. */}}
        {{- $cursor := $values }}
        {{- range $segment := splitList "." $path }}
          {{- if kindIs "map" $cursor }}
            {{- $cursor = index $cursor $segment }}
          {{- else }}
            {{- $cursor = "" }}
          {{- end }}
        {{- end }}
        {{- $name := $cursor | default "" | toString }}
        {{- /* `system-cluster-critical` and `system-node-critical` are built
               into every cluster; naming one is never a mistake. */}}
        {{- if and $name ( not ( hasPrefix "system-" $name ) ) }}
          {{- if not ( has $name $created ) }}
            {{- $warnings = append $warnings ( printf "%s.%s is %q, which this chart does not create; it must already exist in the cluster or the pods will be rejected. Set priorityClasses.create=true, or align the name with priorityClasses.critical.name / priorityClasses.scalable.name." $chart $path $name ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
