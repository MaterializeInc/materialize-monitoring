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
  {{- if (index $.Values "pipelines") -}}
    {{- fail "pipelines.* is the wrong key. Use pipeline.*" }}
  {{- end }}

  {{- /* Output main snippet */}}
  {{- $.Files.Get "pre-rendered/pipelines/gateway.alloy" }}

  {{- /* Metric processors */}}
  {{- $.Files.Get "pre-rendered/pipelines/gateway-metrics.alloy" }}

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
  {{- $metricsForward := list }}
  {{- $metricsPromForward := list }}
  {{- if $pipelineValues.logging.gateway.destination.loki.enabled }}
    {{- $logForward = append $logForward "loki.write.destination.receiver" }}
    {{- include "mzmon.alloyGateway.pipeline.loki.dest" $ }}
  {{- end }}
  {{- if $pipelineValues.metrics.gateway.destination.prometheusRemoteWrite.enabled }}
    {{- $metricsPromForward = append $metricsPromForward "prometheus.remote_write.destination.receiver" }}
    {{- $metricsForward = append $metricsForward "otelcol.exporter.prometheus.outputBridge.input" }}
    {{- include "mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest" $ }}
  {{- end }}
  {{- if ( include "mzmon.alloyGateway.otelDest.enabled" $ ) }}
    {{- if $.Values.pipeline.metrics.gateway.destination.otel.enabled }}
      {{- $metricsForward = append $metricsForward "otelcol.processor.filter.egressFanOut.input" }}
    {{- end }}
    {{- if $.Values.pipeline.logging.gateway.destination.otel.enabled }}
      {{- $logForward = append $logForward "otelcol.receiver.loki.outputBridge.receiver" }}
    {{- end }}
    {{- include "mzmon.alloyGateway.otelDest.render" $ }}
  {{- end }}
loki.process "egress" {
	  forward_to = [
  {{- range $logForward }}
        {{ . }},
  {{- end }}
    ]
}


prometheus.relabel "egress" {
    forward_to = [
  {{- range $metricsPromForward }}
        {{ . }},
  {{- end }}
    ]
}


otelcol.processor.filter "egress" {
    output {
        metrics = [
  {{- range $metricsForward }}
            {{ . }},
  {{- end }}
        ]
    }
}
{{- end }}

{{/*
Generate the alloy-gateway loki.write.destination blocks.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.loki.dest" $ }}
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
}
{{- end }}


{{/*
Generate the alloy-gateway prometheus.remote_write blocks.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest" }}
  {{- $gatewayMetricsValues := $.Values.pipeline.metrics.gateway }}
prometheus.remote_write "destination" {
    external_labels = {
        cluster = sys.env("CLUSTER_NAME"),
    }
    endpoint {
        url = sys.env("GATEWAY_PROM_DEST")
      {{- if eq $gatewayMetricsValues.destination.prometheusRemoteWrite.authType "none" }}
      {{- else if eq $gatewayMetricsValues.destination.prometheusRemoteWrite.authType "sigv4" }}

        sigv4 {
        {{- if $gatewayMetricsValues.destination.prometheusRemoteWrite.sigv4.region }}
            region = {{ $gatewayMetricsValues.destination.prometheusRemoteWrite.sigv4.region | quote }}
        {{- end }}
        {{- if $gatewayMetricsValues.destination.prometheusRemoteWrite.sigv4.roleArn }}
            role_arn = {{ $gatewayMetricsValues.destination.prometheusRemoteWrite.sigv4.roleArn | quote }}
        {{- end }}
        }
      {{- else if eq $gatewayMetricsValues.destination.prometheusRemoteWrite.authType "basicAuth" }}

        basic_auth {
            username = sys.env({{ $gatewayMetricsValues.destination.prometheusRemoteWrite.basicAuth.usernameEnv | required "basicAuth.usernameEnv" | quote }})
            password = sys.env({{ $gatewayMetricsValues.destination.prometheusRemoteWrite.basicAuth.passwordEnv | required "basicAuth.passwordEnv" | quote }})
        }
      {{- else if eq $gatewayMetricsValues.destination.prometheusRemoteWrite.authType "bearer" }}

        authorization {
            type = "Bearer"
            credentials = sys.env({{ $gatewayMetricsValues.destination.prometheusRemoteWrite.bearer.tokenEnv | required "bearer.tokenEnv" | quote }})
        }
      {{- else }}
        {{- printf "Unsupported authType: %s" $gatewayMetricsValues.destination.prometheusRemoteWrite.authType | fail }}
      {{- end }}
    }
}
{{- end }}


{{/*
Check if alloy-gateway OpenTelemetry destination is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

This is true if EITHER the metrics gateway destination is enabled OR the logging gateway destination is enabled.

Usage:
  {{- if ( include "mzmon.alloyGateway.otelDest.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.enabled" }}
  {{- if $.Values.pipeline.metrics.gateway.destination.otel.enabled }}
    {{- "true" }}
  {{- else if $.Values.pipeline.logging.gateway.destination.otel.enabled }}
    {{- "true" }}
  {{- end }}
{{- end }}


{{/*
Render the alloy-gateway metric egress filter.

Use otelcol.processor.filter.$processorName.input as the fanout chained input.

Args:
  forwardTo: list of destinations to forward metrics to
  processorName: name of the processor
  unfilteredMetricsEnv: environment variable containing unfiltered metrics

Usage:
  {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
    "forwardTo" $otelDestValues.otlpExporter.handlers
    "processorName" "otlpMetricEgressFilter"
    "unfilteredMetricsEnv" $otelDestValues.otlpExporter.unfilteredMetricsEnv
  ) }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.egressFilter" }}
  {{- $forwardTo := .forwardTo | required "at least one destination is required" }}
  {{- $unfilteredMetricsEnv := .unfilteredMetricsEnv | required "unfilteredMetricsEnv is required" }}
  {{- $processorName := .processorName | required "processorName is required" }}
  {{- $root := .root | required "root context is required" -}}

otelcol.processor.filter "{{ $processorName }}" {
    metric_conditions {
        context = "metric"
        conditions = [
            "not IsMatch(metric.name, \"^(?:" + coalesce(sys.env({{ $unfilteredMetricsEnv | quote }}), ".*") + ")$\")",
        ]
    }

	  output {
		    metrics = [
{{- range $forwardTo }}
            {{ tpl . $root }},
{{- end }}
		    ]
    }
}
{{- end }}

{{/*
Render the alloy-gateway OpenTelemetry destination blocks.

Usage:
  {{- include "mzmon.alloyGateway.otelDest.render" $ }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.render" }}
  {{- $otelDestValues := $.Values.pipeline.metrics.gateway.destination.otel }}
  {{- $blocks := list }}
  {{- $forwardTo := list }}
  {{- /* Logs are forwarded only to logs-capable exporters (otlp/datadog), by
         their exporter input — never through the metric egress filters, and
         never to the metrics-only googlecloud exporter. */}}
  {{- $logsForwardTo := list }}

  {{- if $otelDestValues.otlpExporter.enabled }}
    {{- $blocks = append $blocks ( tpl $otelDestValues.otlpExporter.config $ ) }}
    {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
      "forwardTo" $otelDestValues.otlpExporter.handlers
      "processorName" "otlpMetricEgressFilter"
      "unfilteredMetricsEnv" $otelDestValues.otlpExporter.unfilteredMetricsEnv
      "root" $
    ) | nindent 0 }}
    {{- $forwardTo = append $forwardTo "otelcol.processor.filter.otlpMetricEgressFilter.input" }}
    {{- range $otelDestValues.otlpExporter.handlers }}
      {{- $logsForwardTo = append $logsForwardTo (tpl . $) }}
    {{- end }}
  {{- end }}

  {{- if $otelDestValues.googleCloudExporter.enabled }}
    {{- $blocks = append $blocks ( tpl $otelDestValues.googleCloudExporter.config $ ) }}
    {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
      "forwardTo" $otelDestValues.googleCloudExporter.handlers
      "processorName" "googleCloudMetricEgressFilter"
      "unfilteredMetricsEnv" $otelDestValues.googleCloudExporter.unfilteredMetricsEnv
      "root" $
    ) | nindent 0 }}
    {{- $forwardTo = append $forwardTo "otelcol.processor.filter.googleCloudMetricEgressFilter.input" -}}
  {{- end }}

  {{- if $otelDestValues.datadogExporter.enabled }}
    {{- $blocks = append $blocks ( tpl $otelDestValues.datadogExporter.config $ ) }}
    {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
      "forwardTo" $otelDestValues.datadogExporter.handlers
      "processorName" "datadogMetricEgressFilter"
      "unfilteredMetricsEnv" $otelDestValues.datadogExporter.unfilteredMetricsEnv
      "root" $
    ) | nindent 0 }}
    {{- $forwardTo = append $forwardTo "otelcol.processor.filter.datadogMetricEgressFilter.input" }}
    {{- range $otelDestValues.datadogExporter.handlers }}
      {{- $logsForwardTo = append $logsForwardTo (tpl . $) }}
    {{- end }}
  {{- end }}

  {{- if ( include "mzmon.alloyGateway.otelDest.authEnabled" $ ) }}
    {{- $blocks = append $blocks ( include "mzmon.alloyGateway.otelDest.auth.render" $ ) }}
  {{- end }}

  {{- if $otelDestValues.enabled }}

otelcol.processor.filter "egressFanOut" {
    output {
        metrics = [
    {{- range $forwardTo }}
            {{ . }},
    {{- end }}
        ]
    }
}
  {{- end }}
  {{- if $.Values.pipeline.logging.gateway.destination.otel.enabled }}
    {{- if not $logsForwardTo }}
      {{- fail "pipeline.logging.gateway.destination.otel is enabled, but no logs-capable otel exporter (otlpExporter or datadogExporter) is enabled to receive them (the googlecloud exporter is metrics-only)." }}
    {{- end }}

otelcol.receiver.loki "outputBridge" {
    output {
        logs = [
            otelcol.processor.batch.outputLogsBatch.input,
        ]
    }
}

otelcol.processor.batch "outputLogsBatch" {
    output {
        logs = [
    {{- range $logsForwardTo }}
            {{ . }},
    {{- end }}
        ]
    }
}
  {{- end }}

  {{- /* output blocks */}}
  {{- printf "\n\n" }}
  {{- $blocks | join "\n\n" }}
{{- end }}


{{/*
Check if alloy-gateway OpenTelemetry destination auth is enabled.

Usage:
  {{- if ( include "mzmon.alloyGateway.otelDest.authEnabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.authEnabled" }}
  {{- $otelDestValues := $.Values.pipeline.metrics.gateway.destination.otel }}
  {{- if ne $otelDestValues.auth.authType "none" }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{/*
Get the auth handler for an alloy-gateway OpenTelemetry destination.

Usage:
  {{- include "mzmon.alloyGateway.otelDest.authHandler" $ }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.authHandler" }}
  {{- $otelDestValues := $.Values.pipeline.metrics.gateway.destination.otel }}
  {{- if eq $otelDestValues.auth.authType "basic" }}
    {{- tpl $otelDestValues.auth.basic.handler $ }}
  {{- else if eq $otelDestValues.auth.authType "bearer" }}
    {{- tpl $otelDestValues.auth.bearer.handler $ }}
  {{- else if or ( eq $otelDestValues.auth.authType "sigv4" ) ( eq $otelDestValues.auth.authType "awsSigv4" ) }}
    {{- tpl $otelDestValues.auth.awsSigv4.handler $ }}
  {{- else if eq $otelDestValues.auth.authType "custom" }}
    {{- tpl $otelDestValues.auth.custom.handler $ }}
  {{- else }}
    {{- printf "Unsupported authType (%s)" $otelDestValues.auth.authType | fail }}
  {{- end }}
{{- end }}

{{/*
Render an auth block.

Usage:
  {{- include "mzmon.alloyGateway.otelDest.auth.render" $ }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.auth.render" }}
  {{- $otelDestValues := $.Values.pipeline.metrics.gateway.destination.otel }}
  {{- if eq $otelDestValues.auth.authType "basic" }}
    {{- tpl $otelDestValues.auth.basic.config $ }}
  {{- else if eq $otelDestValues.auth.authType "bearer" }}
    {{- tpl $otelDestValues.auth.bearer.config $ }}
  {{- else if or ( eq $otelDestValues.auth.authType "sigv4" ) ( eq $otelDestValues.auth.authType "awsSigv4" ) }}
    {{- tpl $otelDestValues.auth.awsSigv4.config $ }}
  {{- else if eq $otelDestValues.auth.authType "custom" }}
    {{- tpl $otelDestValues.auth.custom.config $ }}
  {{- else }}
    {{- printf "Unsupported authType (%s)" $otelDestValues.auth.authType | fail }}
  {{- end }}
{{- end }}


{{/*
Generate metric filter for a given metric exporter.

Filters are regex joined by `|` (most metrics are single named).
This is not anchored by ^$ (that's handled in the pipeline).

Args:
  context: The context object, typically `$`.
  minMetricImportance: The minimum metric importance level to filter by.
    Valid values are "essential", "recommended", "extended", "diagnostic"

Usage:
  {{ $otlpExporter.unfilteredMetricsEnv }}: {{ include "mzmon.alloyGateway.metricFilter" ( dict
      "context" $
      "minMetricImportance" $otlpExporter.minMetricImportance
    ) | quote }}
*/}}
{{- define "mzmon.alloyGateway.metricFilter" }}
  {{- $context := .context | required "context is required" }}
  {{- $minMetricImportance := .minMetricImportance | required "minMetricImportance is required" }}
  {{- $metricTiers := $context.Files.Get "pre-rendered/metrics/metric-tiers.yaml" | required "metrics-tiers.yaml cannot be missing/empty" | fromYaml }}
  {{- $metricPatterns := list }}

  {{- if eq $minMetricImportance "all" }}
    {{- $metricPatterns = list ".*" }}
  {{- end }}
  {{- /* Each tier includes the subsequent metrics (unless .* is already present) */}}
  {{- if eq $minMetricImportance "diagnostic" }}
    {{- $metricPatterns = concat $metricPatterns $metricTiers.diagnostic }}
    {{- $minMetricImportance = "extended" }}
  {{- end }}
  {{- if eq $minMetricImportance "extended" }}
    {{- $metricPatterns = concat $metricPatterns $metricTiers.extended }}
    {{- $minMetricImportance = "recommended" }}
  {{- end }}
  {{- if eq $minMetricImportance "recommended" }}
    {{- $metricPatterns = concat $metricPatterns $metricTiers.recommended }}
    {{- $minMetricImportance = "essential" }}
  {{- end }}
  {{- if eq $minMetricImportance "essential" }}
    {{- $metricPatterns = concat $metricPatterns $metricTiers.essential }}
  {{- end }}

  {{- if not $metricPatterns }}
    {{- printf "No metrics patterns found for %s" .minMetricImportance | fail }}
  {{- end }}

  {{- /* final output */}}
  {{- join "|" $metricPatterns }}
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

{{- /*
Validate the Alloy agent and gateway pipeline wiring.

Usage:
  {{- $res := include "mzmon.alloy.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $res := include "mzmon.alloy.validate.reachability" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res := include "mzmon.alloy.validate.destinations" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- if ( include "mzmon.alloyGateway.enabled" $ ) }}
    {{- $gw := $.Values.pipeline }}
    {{- $res := include "mzmon.alloy.validate.destAuth" ( dict
          "context" $
          "role" "alloy-gateway"
          "path" "pipeline.logging.gateway.destination.loki"
          "dest" $gw.logging.gateway.destination.loki
          "enabled" $gw.logging.gateway.destination.loki.enabled ) | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- $res := include "mzmon.alloy.validate.destAuth" ( dict
          "context" $
          "role" "alloy-gateway"
          "path" "pipeline.metrics.gateway.destination.prometheusRemoteWrite"
          "dest" $gw.metrics.gateway.destination.prometheusRemoteWrite
          "enabled" $gw.metrics.gateway.destination.prometheusRemoteWrite.enabled ) | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate that the agent has a gateway to write to.
*/}}
{{- define "mzmon.alloy.validate.reachability" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $agentOn := include "mzmon.alloyAgent.enabled" $ }}
  {{- $gatewayOn := include "mzmon.alloyGateway.enabled" $ }}

  {{- if and $agentOn ( not $gatewayOn ) }}
    {{- $url := tpl ( $.Values.pipeline.logging.agent.destination.loki.url | toString ) $ }}
    {{- if contains "alloy-gateway" $url }}
      {{- $errors = append $errors ( printf "The Alloy agent is enabled and writes to the bundled gateway (%s), but alloy-gateway is not enabled. Node logs would be collected and then dropped. Enable the gateway, or repoint pipeline.logging.agent.destination.loki.url at a collector you run." $url ) }}
    {{- end }}
  {{- end }}

  {{- if and $gatewayOn ( not $agentOn ) }}
    {{- $warnings = append $warnings "alloy-gateway is enabled but the alloy-agent is not, so nothing collects node or pod logs. This is correct only if you push logs to the gateway from elsewhere (loki.source.api or OTLP)." }}
  {{- end }}

  {{- /* The bundled Loki is the gateway's default log destination. */}}
  {{- if $gatewayOn }}
    {{- $lokiDest := $.Values.pipeline.logging.gateway.destination.loki }}
    {{- if $lokiDest.enabled }}
      {{- $url := tpl ( $lokiDest.url | toString ) $ }}
      {{- /* Require the in-cluster Service shape so an external host that
             happens to start with `loki-` is not flagged. */}}
      {{- if and ( contains "loki-" $url ) ( contains ".svc" $url ) ( not ( include "mzmon.loki.enabled" $ ) ) }}
        {{- $errors = append $errors ( printf "pipeline.logging.gateway.destination.loki.url points at the bundled Loki (%s) but Loki is not enabled. Logs would be written to a Service that does not exist." $url ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate that each signal has somewhere to go.
*/}}
{{- define "mzmon.alloy.validate.destinations" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if ( include "mzmon.alloyGateway.enabled" $ ) }}
    {{- $logging := $.Values.pipeline.logging.gateway.destination }}
    {{- if and ( not $logging.loki.enabled ) ( not $logging.otel.enabled ) }}
      {{- $warnings = append $warnings "Every gateway log destination is disabled (pipeline.logging.gateway.destination.loki.enabled and .otel.enabled are both false). Logs are processed and then discarded." }}
    {{- end }}

    {{- $metrics := $.Values.pipeline.metrics.gateway.destination }}
    {{- if and ( not $metrics.prometheusRemoteWrite.enabled ) ( not $metrics.otel.enabled ) }}
      {{- $warnings = append $warnings "Every gateway metric destination is disabled (pipeline.metrics.gateway.destination.prometheusRemoteWrite.enabled and .otel.enabled are both false). Metrics are scraped and then discarded." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate that a destination's declared authType has credentials behind it.

The rendered pipeline references `sys.env(...)` for every credential, but the
env ConfigMap only sets those variables when an inline value is present in
values. Anything else has to arrive through `extraEnv` or `envFrom`, and
nothing checked that — so a typo produced an empty credential and an auth
failure at run time rather than a render error.

Inline credentials are worth a warning of their own: the pipeline env template
writes them into a ConfigMap, not a Secret.

Usage:
  {{- $res := include "mzmon.alloy.validate.destAuth" ( dict
        "context" $ "role" "alloy-gateway" "path" "..." "dest" $dest "enabled" true ) | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.destAuth" }}
  {{- $ctx := .context | required ".context must be specified" }}
  {{- $role := .role | required ".role must be specified" }}
  {{- $path := .path | required ".path must be specified" }}
  {{- $dest := .dest | required ".dest must be specified" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if and .enabled ( ne ( $dest.authType | toString ) "none" ) }}
    {{- $authType := $dest.authType | toString }}

    {{- /* Which (value, envVar) pairs this authType needs. sigv4 is signed
           from ambient AWS credentials, so it needs none. */}}
    {{- $needed := list }}
    {{- if eq $authType "basicAuth" }}
      {{- $needed = list
            ( dict "field" "basicAuth.username" "value" $dest.basicAuth.username "env" $dest.basicAuth.usernameEnv )
            ( dict "field" "basicAuth.password" "value" $dest.basicAuth.password "env" $dest.basicAuth.passwordEnv "secret" true ) }}
    {{- else if eq $authType "bearer" }}
      {{- $needed = list
            ( dict "field" "bearer.token" "value" $dest.bearer.token "env" $dest.bearer.tokenEnv "secret" true ) }}
    {{- else if eq $authType "oauth2" }}
      {{- $needed = list
            ( dict "field" "oauth2.clientId" "value" $dest.oauth2.clientId "env" $dest.oauth2.clientIdEnv )
            ( dict "field" "oauth2.clientSecret" "value" $dest.oauth2.clientSecret "env" $dest.oauth2.clientSecretEnv "secret" true )
            ( dict "field" "oauth2.tokenUrl" "value" $dest.oauth2.tokenUrl "env" $dest.oauth2.tokenUrlEnv ) }}
    {{- end }}

    {{- $alloy := dig "alloy" dict ( index $ctx.Values $role | default dict ) }}
    {{- $extraEnv := dig "extraEnv" list $alloy }}
    {{- $envFrom := dig "envFrom" list $alloy }}
    {{- $extraEnvNames := list }}
    {{- range $extraEnv }}
      {{- if .name }}
        {{- $extraEnvNames = append $extraEnvNames .name }}
      {{- end }}
    {{- end }}

    {{- range $needed }}
      {{- $envName := .env | toString }}
      {{- if .value }}
        {{- if .secret }}
          {{- $warnings = append $warnings ( printf "%s.%s is set inline, so it renders into the pipeline env ConfigMap in plaintext — not a Secret. Prefer leaving it empty and supplying %s through %s.alloy.envFrom (a secretRef) or .extraEnv." $path .field $envName $role ) }}
        {{- end }}
      {{- else if has $envName $extraEnvNames }}
        {{- /* explicitly provided */}}
      {{- else if $envFrom }}
        {{- $warnings = append $warnings ( printf "%s.authType is %q but %s.%s is empty, so %s must come from %s.alloy.envFrom. That cannot be verified at render time; if the source does not set it, the credential resolves empty and authentication fails at run time." $path $authType $path .field $envName $role ) }}
      {{- else }}
        {{- $errors = append $errors ( printf "%s.authType is %q but %s.%s is empty and %s is set by neither %s.alloy.extraEnv nor .envFrom. The rendered pipeline reads sys.env(%q), which would resolve empty." $path $authType $path .field $envName $role $envName ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
