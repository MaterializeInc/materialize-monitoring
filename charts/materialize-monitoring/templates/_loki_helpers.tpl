{{- /* Loki helpers and validators. */}}

{{- /*
Check if loki is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.loki.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.loki.enabled" }}
  {{- $values := $.Values.loki | required "loki is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or ( index $tags "bundled-backends" ) $tags.loki ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Entrypoint for loki validation checks.

Usage:
  {{- include "mzmon.loki.validate" $ }}
*/}}
{{- define "mzmon.loki.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if ( include "mzmon.loki.enabled" $ ) }}
    {{- $res := include "mzmon.loki.validate.microservices" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- $res := include "mzmon.loki.validate.networkPolicy" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- $res := include "mzmon.loki.validate.storage" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate loki microservices.
*/}}
{{- define "mzmon.loki.validate.microservices" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- /* NOTE: other services are conditionally rendered for other deploymentModes, so it's fine to have a subset. */}}
  {{- $distributedExpected := list "distributor" "ingester" "querier" "queryFrontend" "compactor" "indexGateway" "ruler" }}
  {{- $distributedRecommended := list "queryScheduler" "memcached" }}
  {{- $distributedUnexpected := list "gateway" "bloomGateway" "bloomPlanner" "bloomBuilder" }}
  {{- $autoscaleRecommended := list "distributor" "querier" "queryFrontend" }}
  {{- $noPdb := list "compactor" "memcached" }}

  {{- if ( include "mzmon.loki.enabled" $ ) }}
    {{- if ne $.Values.loki.deploymentMode "Distributed" }}
      {{- $warnings = append $warnings ( printf "loki.deploymentMode is %s. This is not recommended for production." $.Values.loki.deploymentMode ) }}
    {{- else }}

      {{- /* Check expected sets of microservices when Distributed is enabled */}}
      {{- range $svc := $distributedExpected }}
        {{- if not ( index $.Values.loki $svc ) }}
          {{- $errors = append $errors ( printf "loki.%s is missing entirely." $svc ) }}
        {{- else }}
          {{- if not ( index $.Values.loki $svc ).enabled }}
            {{- $errors = append $errors ( printf "loki.%s.enabled is required for distributed mode." $svc ) }}
          {{- else }}
            {{- if ( has $svc $autoscaleRecommended ) }}
              {{- $res := include "mzmon.loki.validate.autoscaling" ( dict "context" $ "svc" $svc ) | fromYaml }}
              {{- $errors = concat $errors $res.errors | default list }}
              {{- $warnings = concat $warnings $res.warnings | default list }}
            {{- end }}
            {{- if not ( has $svc $noPdb ) }}
              {{- $res := include "mzmon.loki.validate.pdb" ( dict "context" $ "svc" $svc ) | fromYaml }}
              {{- $errors = concat $errors $res.errors | default list }}
              {{- $warnings = concat $warnings $res.warnings | default list }}
            {{- end }}
          {{- end }}
        {{- end }}
      {{- end }}

      {{- /* Check recommended (not required) microservices as well (sorry for duplication). */}}
      {{- range $svc := $distributedRecommended }}
        {{- if not ( index $.Values.loki $svc ) }}
          {{- $errors = append $errors ( printf "loki.%s is missing entirely." $svc ) }}
        {{- else }}
          {{- if not ( index $.Values.loki $svc ).enabled }}
            {{- $warnings = append $warnings ( printf "loki.%s.enabled is recommended for scaled deployments." $svc ) }}
          {{- else }}
            {{- if ( has $svc $autoscaleRecommended ) }}
              {{- $res := include "mzmon.loki.validate.autoscaling" ( dict "context" $ "svc" $svc ) | fromYaml }}
              {{- $errors = concat $errors $res.errors | default list }}
              {{- $warnings = concat $warnings $res.warnings | default list }}
            {{- end }}
            {{- if not ( has $svc $noPdb ) }}
              {{- $res := include "mzmon.loki.validate.pdb" ( dict "context" $ "svc" $svc ) | fromYaml }}
              {{- $errors = concat $errors $res.errors | default list }}
              {{- $warnings = concat $warnings $res.warnings | default list }}
            {{- end }}
          {{- end }}
        {{- end }}
      {{- end }}

      {{- range $svc := $distributedUnexpected }}
        {{- if not ( index $.Values.loki $svc ) }}
          {{- $errors = append $errors ( printf "loki.%s is missing entirely." $svc ) }}
        {{- else }}
          {{- if ( index $.Values.loki $svc ).enabled }}
            {{- $warnings = append $warnings ( printf "loki.%s.enabled is not recommended for production." $svc ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate autoscaling for a single microservice.

Usage:
  {{- include "mzmon.loki.validate.autoscaling" ( dict "context" $ "svc" "distributor" ) }}
*/}}
{{- define "mzmon.loki.validate.autoscaling" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $context := .context | required ".context must be specified" }}
  {{- $svc := .svc | required ".svc must be specified" }}
  {{- $svcValues := index $context.Values.loki $svc | required ( printf "loki.%s is missing entirely." $svc ) }}
  {{- $_ := $svcValues.autoscaling | required $svc }}
  {{- $_ := $svcValues.kedaAutoscaling | required $svc }}

  {{- if $svcValues.enabled }}
    {{- if not ( or $svcValues.autoscaling.enabled $svcValues.kedaAutoscaling.enabled ) }}
      {{- $warnings = append $warnings ( printf "loki.%s microservice autoscaling is recommended for production." $svc ) }}
    {{- else }}
      {{- if ne $svcValues.kind "Deployment" }}
        {{- $errors = append $errors ( printf "loki.%s microservice autoscaling is only supported for Deployment kind." $svc ) }}
      {{- end }}
      {{- /* go uses <nil> for null values */}}
      {{- if not ( typeIs "<nil>" $svcValues.replicas ) }}
        {{- $warnings = append $warnings ( printf "loki.%s microservice replicas should be null when autoscaling is enabled." $svc ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate PDB for a single microservice.

Usage:
  {{- include "mzmon.loki.validate.pdb" ( dict "context" $ "svc" "distributor" ) }}
*/}}
{{- define "mzmon.loki.validate.pdb" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $context := .context | required ".context must be specified" }}
  {{- $svc := .svc | required ".svc must be specified" }}
  {{- $svcValues := index $context.Values.loki $svc | required ( printf "loki.%s is missing entirely." $svc ) }}
  {{- $_ := $svcValues.podDisruptionBudget | required $svc }}

  {{- if $svcValues.enabled }}
    {{- if not $svcValues.podDisruptionBudget.enabled }}
      {{- $warnings = append $warnings ( printf "loki.%s microservice PDB is recommended for production." $svc ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate loki networkPolicy.
*/}}
{{- define "mzmon.loki.validate.networkPolicy" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- if ( include "mzmon.loki.enabled" $ ) }}
    {{- $np := $.Values.loki.networkPolicy | required "loki.networkPolicy is missing." }}
    {{- if $np.enabled }}
      {{- if not ( or $np.metrics.namespaceSelector $np.metrics.podSelector ) }}
        {{- $errors = append $errors "loki.networkPolicy.metrics.namespaceSelector is required when networkPolicy is enabled." }}
      {{- end }}
      {{- if not ( or $np.ingress.namespaceSelector $np.ingress.podSelector ) }}
        {{- $errors = append $errors "loki.networkPolicy.ingress.namespaceSelector is required when networkPolicy is enabled." }}
      {{- end }}
    {{- else }}
      {{- $warnings = append $warnings "loki.networkPolicy.enabled is recommended in production." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate loki storage configuration.

Note that loki.loki.storage is the correct path.
*/}}
{{- define "mzmon.loki.validate.storage" }}
  {{- $values := $.Values.loki | required "loki is missing from values." }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- if ( include "mzmon.loki.enabled" $ ) }}
    {{- if or ( not $values.loki.storage.bucketNames.chunks ) ( eq $values.loki.storage.bucketNames.chunks "<REPLACE-ME>" ) }}
      {{- $errors = append $errors "loki.loki.storage.bucketNames.chunks is required when loki is enabled." }}
    {{- end }}
    {{- if or ( not $values.loki.storage.bucketNames.ruler ) ( eq $values.loki.storage.bucketNames.ruler "<REPLACE-ME>" ) }}
      {{- $errors = append $errors "loki.loki.storage.bucketNames.ruler is required when loki is enabled." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
