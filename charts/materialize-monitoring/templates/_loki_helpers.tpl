{{- /* Loki helpers and validators. */}}

{{- /*
Get loki namespace.

Usage:
  {{- include "mzmon.loki.namespace" $ }}
*/}}
{{- define "mzmon.loki.namespace" }}
  {{- $ns := $.Values.loki.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

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
    {{- if ( or $tags.default ( index $tags "bundled-backends" ) $tags.loki ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get loki replication factor.

Usage:
  {{- include "mzmon.loki.replicationFactor" $ }}
*/}}
{{- define "mzmon.loki.replicationFactor" }}
  {{- $values := $.Values.loki | required "loki is missing from values." }}
  {{- /* The Loki app config lives under the subchart's own `loki:` key, so the
         path from the umbrella is loki.loki.commonConfig.replication_factor. */}}
  {{- $rf := dig "loki" "commonConfig" "replication_factor" 3 $values }}
  {{- printf "%d" ( int $rf ) -}}
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

    {{- $res := include "mzmon.loki.validate.ingesterRollout" $ | fromYaml }}
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
    {{- else }}
      {{- /* Only one of minAvailable / maxUnavailable can be used */}}
      {{- $minAvailable := $svcValues.podDisruptionBudget.minAvailable }}
      {{- $maxUnavailable := $svcValues.podDisruptionBudget.maxUnavailable }}
      {{- if and ( not ( typeIs "<nil>" $minAvailable ) ) ( not ( typeIs "<nil>" $maxUnavailable ) ) }}
        {{- $errors = append $errors ( printf "loki.%s microservice PDB should specify either minAvailable or maxUnavailable, but not both." $svc ) }}
      {{- end }}
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

{{- /*
Warn if the ingester can lose more than one replica at once.

With replication_factor >= 2, taking more than one ingester down simultaneously
— whether by a rolling update (updateStrategy.rollingUpdate.maxUnavailable) or a
voluntary eviction (podDisruptionBudget.maxUnavailable) — can break write quorum.
Rolls should stay one-at-a-time; use zoneAwareReplication for a quorum-safe
zone-at-a-time burst instead.

Usage:
  {{- include "mzmon.loki.validate.ingesterRollout" $ }}
*/}}
{{- define "mzmon.loki.validate.ingesterRollout" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- if ( include "mzmon.loki.enabled" $ ) }}
    {{- /* replicationFactor emits a plain integer string — int it directly; fromYaml would choke on a scalar. */}}
    {{- $rf := include "mzmon.loki.replicationFactor" $ | int }}
    {{- if and ( eq $.Values.loki.deploymentMode "Distributed" ) ( gt ( int $rf ) 1 ) }}
      {{- $ing := $.Values.loki.ingester | required "loki.ingester is expected to be present." }}
      {{- $fields := dict
          "updateStrategy.rollingUpdate.maxUnavailable" ( dig "updateStrategy" "rollingUpdate" "maxUnavailable" nil $ing )
          "podDisruptionBudget.maxUnavailable" ( dig "podDisruptionBudget" "maxUnavailable" nil $ing ) }}
      {{- range $path, $v := $fields }}
        {{- $exceeds := false }}
        {{- if kindIs "invalid" $v }}
        {{- else if kindIs "string" $v }}
          {{- /* a percentage can resolve to more than one pod; flag it for review */}}
          {{- if hasSuffix "%" $v }}
            {{- if ne $v "0%" }}{{- $exceeds = true }}{{- end }}
          {{- else if gt ( int $v ) 1 }}{{- $exceeds = true }}{{- end }}
        {{- else if gt ( int $v ) 1 }}{{- $exceeds = true }}{{- end }}
        {{- if $exceeds }}
          {{- $warnings = append $warnings ( printf "loki.ingester.%s is %v: with replication_factor %v, taking more than one ingester down at once (rollout or eviction) can break write quorum. Keep it at 1 — use zoneAwareReplication for a quorum-safe zone-at-a-time burst." $path $v $rf ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- define "mzmon.loki.validate.storage" }}
  {{- $values := $.Values.loki | required "loki is missing from values." }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- if ( include "mzmon.loki.enabled" $ ) }}
    {{- if or ( not $values.loki.storage.bucketNames.chunks ) ( eq $values.loki.storage.bucketNames.chunks "<REPLACE-ME>" ) ( eq $values.loki.storage.bucketNames.chunks "your-loki-bucket-name" ) }}
      {{- $errors = append $errors "loki.loki.storage.bucketNames.chunks is required when loki is enabled." }}
    {{- end }}
    {{- if or ( not $values.loki.storage.bucketNames.ruler ) ( eq $values.loki.storage.bucketNames.ruler "<REPLACE-ME>" ) ( eq $values.loki.storage.bucketNames.ruler "your-loki-bucket-name" ) }}
      {{- $errors = append $errors "loki.loki.storage.bucketNames.ruler is required when loki is enabled." }}
    {{- end }}

    {{- /* Object-store backend consistency (thanos objstore mode).
           The backend has to be named in several places and the chart's defaults
           name s3 in all of them. Two of them do not degrade gracefully: the
           client is chosen by name and then validated against a config that was
           never populated, so the component crash-loops with "no s3 endpoint in
           config file". Those are errors; the inert one below is a warning. */}}
    {{- /* The same mismatch, on the pre-thanos path. `use_thanos_objstore: false`
           makes `storage.type` the live selector, and the compactor still reads
           delete_request_store — so a profile that switches the backend and
           forgets this one crash-loops with "at least one bucket name must be
           specified". The check below is gated on the thanos path, which is why
           this needs its own. */}}
    {{- $legacyOnly := dig "loki" "storage" "type" "" $values }}
    {{- if and ( not ( dig "loki" "storage" "use_thanos_objstore" false $values ) ) $legacyOnly }}
      {{- $delStore := dig "loki" "compactor" "delete_request_store" "" $values }}
      {{- if and $delStore ( ne $delStore $legacyOnly ) }}
        {{- $errors = append $errors ( printf "loki.loki.compactor.delete_request_store (%q) must match loki.loki.storage.type (%q) while use_thanos_objstore is off; otherwise the compactor fails at startup building a client for a backend that was never configured." $delStore $legacyOnly ) }}
      {{- end }}
    {{- end }}

    {{- $objType := dig "loki" "storage" "object_store" "type" "" $values }}
    {{- if and ( dig "loki" "storage" "use_thanos_objstore" false $values ) $objType }}
      {{- $delStore := dig "loki" "compactor" "delete_request_store" "" $values }}
      {{- if and $delStore ( ne $delStore $objType ) }}
        {{- $errors = append $errors ( printf "loki.loki.compactor.delete_request_store (%q) must match loki.loki.storage.object_store.type (%q); otherwise the compactor's delete-request store falls back and fails at startup with \"no s3 endpoint in config file\"." $delStore $objType ) }}
      {{- end }}

      {{- /* The legacy selector. Inert while use_thanos_objstore is on — Loki
             ignores the `ruler.storage` block the chart renders from it, and
             logs that it is doing so — but the chart renders it all the same, so
             a stale value leaves an s3 ruler store sitting in the config next to
             a gcs one. A warning, not an error: nothing reads it today, and it
             only becomes load-bearing if use_thanos_objstore is turned off. */}}
      {{- $legacyType := dig "loki" "storage" "type" "" $values }}
      {{- if and ( dig "ruler" "enabled" false $values ) $legacyType ( ne $legacyType $objType ) }}
        {{- $warnings = append $warnings ( printf "loki.loki.storage.type (%q) differs from loki.loki.storage.object_store.type (%q). Loki ignores it while use_thanos_objstore is on, so this is not a live misconfiguration — but the rendered ruler config contains a %s store pointed at a %s bucket name, and Loki logs \"ruler.storage ... will be ignored\" on every start. Set it to %q to make the rendered config agree with itself." $legacyType $objType $legacyType $objType $objType ) }}
      {{- end }}

      {{- /* Each schema period names the backend for its own chunk client, and
             the last period is the active one — every ingester resolves chunks
             through it. Earlier periods are allowed to differ: that is what an
             append-only backend migration looks like, and their chunks still
             have to be readable from where they were written. */}}
      {{- $schemaConfigs := dig "loki" "schemaConfig" "configs" list $values }}
      {{- $activeIdx := sub ( len $schemaConfigs ) 1 | int }}
      {{- range $i, $cfg := $schemaConfigs }}
        {{- $schemaStore := dig "object_store" "" $cfg }}
        {{- if and $schemaStore ( ne $schemaStore $objType ) }}
          {{- if eq $i $activeIdx }}
            {{- $errors = append $errors ( printf "loki.loki.schemaConfig.configs[%d] is the active schema period and its object_store (%q) must match loki.loki.storage.object_store.type (%q). Loki selects the chunk client by this name, so every ingester fails at startup with \"no s3 endpoint in config file\". To migrate backends, append a new period instead of editing this one." $i $schemaStore $objType ) }}
          {{- else }}
            {{- $warnings = append $warnings ( printf "loki.loki.schemaConfig.configs[%d].object_store (%q) differs from loki.loki.storage.object_store.type (%q). Expected for a period that predates a backend migration — chunks written under it must stay reachable at %s." $i $schemaStore $objType $schemaStore ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
