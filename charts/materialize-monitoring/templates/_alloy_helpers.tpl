{{- /* Alloy helpers and validators. */}}

{{- /*
Check if alloy-gateway is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.alloyGateway.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.alloyGateway.enabled" }}
  {{- $values := index $.Values "alloy-gateway" | required "alloy-gateway is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default $tags.pipeline ( index $tags "alloy-gateway" ) ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Check if alloy-agent is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.alloyAgent.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.alloyAgent.enabled" }}
  {{- $values := index $.Values "alloy-agent" | required "alloy-agent is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default $tags.pipeline ( index $tags "alloy-agent" ) ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get alloy-gateway namespace.

Usage:
  {{- include "mzmon.alloyGateway.namespace" $ }}
*/}}
{{- define "mzmon.alloyGateway.namespace" }}
  {{- $values := index $.Values "alloy-gateway" | required "alloy-gateway is missing from values." }}
  {{- $ns := $values.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Get alloy-agent namespace.

Usage:
  {{- include "mzmon.alloyAgent.namespace" $ }}
*/}}
{{- define "mzmon.alloyAgent.namespace" }}
  {{- $values := index $.Values "alloy-agent" | required "alloy-agent is missing from values." }}
  {{- $ns := $values.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Get alloy-gateway fullname.

Usage:
  {{- include "mzmon.alloyGateway.fullname" $ }}
*/}}
{{- define "mzmon.alloyGateway.fullname" }}
  {{- $subChart := index $.Subcharts "alloy-gateway" }}
  {{- include "alloy.fullname" $subChart }}
{{- end }}

{{- /*
Get alloy-agent fullname.

Usage:
  {{- include "mzmon.alloyAgent.fullname" $ }}
*/}}
{{- define "mzmon.alloyAgent.fullname" }}
  {{- $subChart := index $.Subcharts "alloy-agent" }}
  {{- include "alloy.fullname" $subChart }}
{{- end }}

{{- /*
Get alloy-gateway serviceAccount.

Usage:
  {{- include "mzmon.alloyGateway.serviceAccountName" $ }}
*/}}
{{- define "mzmon.alloyGateway.serviceAccountName" }}
  {{- $subChart := index $.Subcharts "alloy-gateway" }}
  {{- include "alloy.serviceAccountName" $subChart }}
{{- end }}

{{- /*
Get alloy-agent serviceAccount.

Usage:
  {{- include "mzmon.alloyAgent.serviceAccountName" $ }}
*/}}
{{- define "mzmon.alloyAgent.serviceAccountName" }}
  {{- $subChart := index $.Subcharts "alloy-agent" }}
  {{- include "alloy.serviceAccountName" $subChart }}
{{- end }}

{{- /*
Get alloy-gateway image.

Usage:
  {{- include "mzmon.alloyGateway.image" $ }}
*/}}
{{- define "mzmon.alloyGateway.image" }}
  {{- $values := index $.Values "alloy-gateway" | required "alloy-gateway is missing from values." }}
  {{- $registry := $values.global.image.registry | default $values.image.registry }}
  {{- $repo := $values.image.repository }}
  {{- $subChart := index $.Subcharts "alloy-gateway" }}
  {{- $suffix := include "alloy.imageId" $subChart }}
  {{- printf "%s/%s%s" $registry $repo $suffix }}
{{- end }}

{{- /*
Get alloy-agent image.

Usage:
  {{- include "mzmon.alloyAgent.image" $ }}
*/}}
{{- define "mzmon.alloyAgent.image" }}
  {{- $values := index $.Values "alloy-agent" | required "alloy-agent is missing from values." }}
  {{- $registry := $values.global.image.registry | default $values.image.registry }}
  {{- $repo := $values.image.repository }}
  {{- $subChart := index $.Subcharts "alloy-agent" }}
  {{- $suffix := include "alloy.imageId" $subChart }}
  {{- printf "%s/%s%s" $registry $repo $suffix }}
{{- end }}

{{- /*
Get alloy-gateway configMap name.

Usage:
  {{- include "mzmon.alloyGateway.configMap.name" $ }}
*/}}
{{- define "mzmon.alloyGateway.configMap.name" }}
  {{- $subChart := index $.Subcharts "alloy-gateway" }}
  {{- include "alloy.config-map.name" $subChart }}
{{- end }}

{{- /*
Get alloy-agent configMap name.

Usage:
  {{- include "mzmon.alloyAgent.configMap.name" $ }}
*/}}
{{- define "mzmon.alloyAgent.configMap.name" }}
  {{- $subChart := index $.Subcharts "alloy-agent" }}
  {{- include "alloy.config-map.name" $subChart }}
{{- end }}

{{- /*
Get alloy-gateway configMap key.

Usage:
  {{- include "mzmon.alloyGateway.configMap.key" $ }}
*/}}
{{- define "mzmon.alloyGateway.configMap.key" }}
  {{- $subChart := index $.Subcharts "alloy-gateway" }}
  {{- include "alloy.config-map.key" $subChart }}
{{- end }}

{{- /*
Get alloy-agent configMap key.

Usage:
  {{- include "mzmon.alloyAgent.configMap.key" $ }}
*/}}
{{- define "mzmon.alloyAgent.configMap.key" }}
  {{- $subChart := index $.Subcharts "alloy-agent" }}
  {{- include "alloy.config-map.key" $subChart }}
{{- end }}

{{/*
Generate the alloy-gateway pipeline.

This is suitably formatted for a configmap.
Be sure to put this into a |- block.

Usage:
  {{ include "mzmon.alloyGateway.configMap.key" $ }}: |-
    {{- include "mzmon.alloyGateway.pipeline" $ | nindent 4 }}
*/}}
{{- define "mzmon.alloyGateway.pipeline" }}
  {{- include "mzmon.alloyGateway.pipeline.contents" $ | replace "\t" "    " }}
{{- end }}

{{/*
Generate the contents of an alloy-gateway pipeline.

Note that this has tabs in it, so it would end up not being yaml-literal-friendly.

Usage:
  Use mzmon.alloyGateway.pipeline instead.
*/}}
{{- define "mzmon.alloyGateway.pipeline.contents" }}
  {{- $values := index $.Values "alloy-gateway" | required "alloy-gateway is missing from values." }}
  {{- $pipelineValues := $.Values.pipeline }}

  {{- /* Output main snippet */}}
  {{- $.Files.Get "pre-rendered/pipelines/gateway.alloy" }}

  {{- /* Output rendered destination */}}
  {{- include "mzmon.alloyGateway.pipeline.destination" $ }}
{{- end }}

{{/*
Generate the alloy-gateway pipeline destinations.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.destination" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.destination" }}
  {{- $pipelineValues := $.Values.pipeline }}
  {{- $logForward := list }}
  {{- if $pipelineValues.logging.gateway.destination.loki.enabled }}
    {{- $logForward = append $logForward "loki.write.destination.receiver" }}
    {{- include "mzmon.alloyGateway.pipeline.loki.dest" $ }}
  {{- end }}
loki.process "egress" {
	  forward_to = [
  {{- range $logForward }}
      {{ . }},
  {{- end }}
    ]
}
{{- end }}

{{/*
Generate the alloy-gateway loki.write.destination blocks.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.loki.dest" $ | nindent 4 }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.loki.dest" }}
  {{- $gatewayLogValues := $.Values.pipeline.logging.gateway }}
loki.write "destination" {
    endpoint {
      url = sys.env("GATEWAY_LOKI_DEST")
      max_backoff_period = {{ $gatewayLogValues.destination.loki.retries.maxBackoffPeriod | quote }}
      max_backoff_retries = {{ $gatewayLogValues.destination.loki.retries.maxBackoffRetries }}
      min_backoff_period = {{ $gatewayLogValues.destination.loki.retries.minBackoffPeriod | quote }}
      retry_on_http_429 = {{ $gatewayLogValues.destination.loki.retries.retryOnHttp429 }}
  {{- if eq $.Values.pipeline.logging.tenancy.tenantMap.default "static" }}
      tenant_id = {{ $.Values.pipeline.logging.tenancy.staticTenant | quote }}
  {{- end }}
    }
  {{- if eq $gatewayLogValues.destination.loki.authType "none" }}
  {{- else if eq $gatewayLogValues.destination.loki.authType "basicAuth" }}

    basic_auth {
      username = sys.env({{ $gatewayLogValues.destination.loki.basicAuth.usernameEnv | required "basicAuth.usernameEnv" | quote }})
      password = sys.env({{ $gatewayLogValues.destination.loki.basicAuth.passwordEnv | required "basicAuth.passwordEnv" | quote }})
    }
  {{- else if eq $gatewayLogValues.destination.loki.authType "bearer" }}

    authorization {
      type = "Bearer"
      credentials = sys.env({{ $gatewayLogValues.destination.loki.bearer.tokenEnv | required "bearer.tokenEnv" | quote }})
    }
  {{- else }}
    {{- printf "Unsupported authType: %s" $gatewayLogValues.destination.loki.authType | fail }}
  {{- end }}
}
{{- end }}


{{/*
Generate the alloy-agent pipeline.

This is suitably formatted for a configmap.
Be sure to put this into a |- block.

Usage:
  {{- include "mzmon.alloyAgent.configMap.key" $ }}: |-
    {{- include "mzmon.alloyAgent.pipeline" $ | nindent 4 }}
*/}}
{{- define "mzmon.alloyAgent.pipeline" }}
  {{- include "mzmon.alloyAgent.pipeline.contents" $ | replace "\t" "    " }}
{{- end }}

{{/*
Generate the contents of an alloy-agent pipeline.

Note that this has tabs in it, so it would end up not being yaml-literal-friendly.

Usage:
  Use mzmon.alloyAgent.pipeline instead.
*/}}
{{- define "mzmon.alloyAgent.pipeline.contents" }}
  {{- $values := index $.Values "alloy-agent" | required "alloy-agent is missing from values." }}
  {{- $pipelineValues := $.Values.pipeline }}

  {{- /* Output main snippet */}}
  {{- $.Files.Get "pre-rendered/pipelines/agent.alloy" }}
{{- end }}
