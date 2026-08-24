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

  {{- /* Output rendered sources */}}
  {{- include "mzmon.alloyGateway.pipeline.sources" $ }}

  {{- /* Output rendered destination */}}
  {{- include "mzmon.alloyGateway.pipeline.destination" $ }}
{{- end }}

{{/*
Generate the alloy-gateway pipeline sources.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.sources" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.sources" }}
  {{- /* Two server blocks, because the listeners belong to two pipeline trees
         and an operator securing one should not silently secure the other. The
         logs tree covers loki.source.api and otelcol.receiver.otlp; the metrics
         tree covers prometheus.receive_http. */}}
  {{- $tls := dig "logging" "gateway" "server" "tls" dict ( $.Values.pipeline | default dict ) }}
  {{- $metricTls := dig "metrics" "gateway" "server" "tls" dict ( $.Values.pipeline | default dict ) }}
loki.source.api "gateway" {
    forward_to = [
        loki.process.sampleDebug.receiver,
        loki.process.inputProcessor.receiver,
    ]

    http {
        listen_port = encoding.from_json(coalesce(sys.env("ALLOY_LOKI_PORT"), "3100"))
      {{- include "mzmon.alloy.serverTls" ( dict "tls" $tls "indent" 8 ) }}
    }
}

otelcol.receiver.otlp "gateway" {
    grpc {
      {{- include "mzmon.alloy.serverTls" ( dict "tls" $tls "flavor" "otelcol" "indent" 8 ) }}
    }

    http {
      {{- include "mzmon.alloy.serverTls" ( dict "tls" $tls "flavor" "otelcol" "indent" 8 ) }}
    }

    output {
        metrics = [
            otelcol.processor.filter.inputMetricProcessor.input,
        ]
        logs = [
            otelcol.exporter.loki.bridge.input,
        ]
    }
}

{{- /* The third listener. It lived in the pre-rendered metrics pipeline until
       this moved here, which is why it was the one ingress port that could not
       be given TLS from values — a secured logs path in front of a plaintext
       metrics path. Same dskit-flavoured server block as loki.source.api. */}}
prometheus.receive_http "gateway" {
    forward_to = [
        otelcol.receiver.prometheus.inputBridge.receiver,
    ]

    http {
        listen_port = 9090
      {{- include "mzmon.alloy.serverTls" ( dict "tls" $metricTls "flavor" "alloy" "indent" 8 ) }}
    }
}
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
  {{- /* Each remote-write destination hangs off the shared prometheus.relabel
         "egress" seam by its own per-destination tier filter. The OTLP -> prom
         bridge feeding that seam is shared and added once, however many
         destinations there are. */}}
  {{- range $dest := ( include "mzmon.alloyGateway.promDests" $ | fromYamlArray ) }}
    {{- if $dest.enabled }}
      {{- $metricsPromForward = append $metricsPromForward ( printf "prometheus.relabel.%s.receiver" $dest.name ) }}
    {{- end }}
  {{- end }}
  {{- if $metricsPromForward }}
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
      {{- include "mzmon.alloy.tlsConfig" ( dict "tls" $gatewayLogValues.destination.loki.tls "indent" 8 ) }}
    }
}
{{- end }}


{{/*
Reserved Alloy component labels a Prometheus remote-write destination may not use.

Each destination renders `prometheus.relabel "<name>"` and
`prometheus.remote_write "<name>"`, so a name colliding with a component this
pipeline already defines produces a duplicate-label config alloy rejects at load.
`egress` is the fan-out seam every destination hangs off.

Usage:
  {{- if has $name ( include "mzmon.alloyGateway.promDest.reservedNames" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.alloyGateway.promDest.reservedNames" }}
  {{- list "egress" | toYaml }}
{{- end }}


{{/*
Legacy keys of the single-destination `prometheusRemoteWrite` shape.

`prometheusRemoteWrite` used to *be* one destination; it is now a map of them.
Helm deep-merges values, so a chart that simply moved the keys would accept an
old override silently — `prometheusRemoteWrite.url` would sit beside `thanos`,
apply to nothing, and the install would keep writing to the default endpoint
with no error and no metric gap to notice. These names are what
`mzmon.alloy.validate.promDests` refuses, so the upgrade fails loudly instead.

Usage:
  {{- range include "mzmon.alloyGateway.promDest.legacyKeys" $ | fromYamlArray }}
*/}}
{{- define "mzmon.alloyGateway.promDest.legacyKeys" }}
  {{- list "enabled" "url" "urlEnv" "minMetricImportance" "unfilteredMetricsEnv"
           "externalLabels" "authType" "basicAuth" "bearer" "oauth2" "sigv4" "tls" | toYaml }}
{{- end }}


{{/*
Resolve one Prometheus remote-write destination against the per-destination defaults.

A destination in values carries only what differs, so every consumer would
otherwise have to `dig` each field with the same default — and the env
ConfigMap and the pipeline disagreeing on one of those defaults is a
misconfiguration neither side can detect. This is the single place they are
written down.

Fields are resolved with `dig` rather than `mergeOverwrite`, deliberately.
`mergeOverwrite` is mergo's `WithOverride`, which does not override with a
*zero* value — so `enabled: false` and `tls.verify: false` would both be
silently discarded and the destination would stay on. Every boolean here has a
`true` default or a caller who needs to turn it off, so that failure mode is not
hypothetical.

Environment variable names are derived from the destination name when not given.
The caller can still name them, which is what the Terraform module's
`mzmon-alloy-gateway-env` Secret needs (see DEP-204): the Secret supplies the
value, the values supply the name, and the two cannot drift.

Args:
  name: the destination's key in the map — also its Alloy component label.
  dest: the raw values for that destination.

Usage:
  {{- $d := include "mzmon.alloyGateway.promDest.resolve" ( dict
        "name" $name "dest" $raw ) | fromYaml }}
*/}}
{{- define "mzmon.alloyGateway.promDest.resolve" }}
  {{- $name := .name | required "name is required" | toString }}
  {{- /* Not `| default dict`: a destination set to an explicit `null` (how Helm
         records a cleared key) and a leftover scalar from the pre-map shape both
         arrive here as a non-map, and `dig` panics on those with a Go type error
         rather than anything an operator can act on. `mzmon.alloy.validate.promDests`
         is what reports the leftover scalar properly. */}}
  {{- $raw := .dest }}
  {{- if not ( kindIs "map" $raw ) }}
    {{- $raw = dict }}
  {{- end }}
  {{- /* Component labels allow more than env var names do, so the env fragment
         is the name with everything else folded to `_`. */}}
  {{- $slug := $name | regexReplaceAll "[^A-Za-z0-9]" "_" | upper }}

  {{- $tls := dig "tls" dict $raw }}
  {{- if not ( kindIs "map" $tls ) }}
    {{- $tls = dict }}
  {{- end }}

  {{- /* The three booleans whose default is `true`, resolved by hand rather
         than with `dig`. `dig` treats an explicit `null` as a *value* and
         returns empty for it, and `null` is exactly how Helm records a key a
         later values file cleared — so `enabled: null` would read as
         `enabled: false` and silently take the destination off the fan-out.
         Absent and null both have to mean "use the default"; only a real
         `false` may switch it off. */}}
  {{- $enabled := true }}
  {{- if and ( hasKey $raw "enabled" ) ( not ( kindIs "invalid" ( index $raw "enabled" ) ) ) }}
    {{- $enabled = index $raw "enabled" }}
  {{- end }}
  {{- $tlsEnabled := false }}
  {{- if and ( hasKey $tls "enabled" ) ( not ( kindIs "invalid" ( index $tls "enabled" ) ) ) }}
    {{- $tlsEnabled = index $tls "enabled" }}
  {{- end }}
  {{- $tlsVerify := true }}
  {{- if and ( hasKey $tls "verify" ) ( not ( kindIs "invalid" ( index $tls "verify" ) ) ) }}
    {{- $tlsVerify = index $tls "verify" }}
  {{- end }}

  {{- dict
    "name" $name
    "enabled" $enabled
    "url" ( dig "url" "" $raw | toString )
    "urlEnv" ( dig "urlEnv" ( printf "GATEWAY_PROM_DEST_%s" $slug ) $raw | toString )
    "minMetricImportance" ( dig "minMetricImportance" "all" $raw | toString )
    "unfilteredMetricsEnv" ( dig "unfilteredMetricsEnv" ( printf "GATEWAY_UNFILTERED_PROM_METRICS_%s" $slug ) $raw | toString )
    "externalLabels" ( dig "externalLabels" dict $raw )
    "authType" ( dig "authType" "none" $raw | toString )
    "basicAuth" ( dict
      "username" ( dig "basicAuth" "username" "" $raw | toString )
      "usernameEnv" ( dig "basicAuth" "usernameEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_USERNAME" $slug ) $raw | toString )
      "password" ( dig "basicAuth" "password" "" $raw | toString )
      "passwordEnv" ( dig "basicAuth" "passwordEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_PASSWORD" $slug ) $raw | toString ) )
    "bearer" ( dict
      "token" ( dig "bearer" "token" "" $raw | toString )
      "tokenEnv" ( dig "bearer" "tokenEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_BEARER_TOKEN" $slug ) $raw | toString ) )
    "oauth2" ( dict
      "clientId" ( dig "oauth2" "clientId" "" $raw | toString )
      "clientIdEnv" ( dig "oauth2" "clientIdEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_OAUTH2_CLIENT_ID" $slug ) $raw | toString )
      "clientSecret" ( dig "oauth2" "clientSecret" "" $raw | toString )
      "clientSecretEnv" ( dig "oauth2" "clientSecretEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_OAUTH2_CLIENT_SECRET" $slug ) $raw | toString )
      "scopes" ( dig "oauth2" "scopes" list $raw )
      "tokenUrl" ( dig "oauth2" "tokenUrl" "" $raw | toString )
      "tokenUrlEnv" ( dig "oauth2" "tokenUrlEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_OAUTH2_TOKEN_URL" $slug ) $raw | toString ) )
    "sigv4" ( dict
      "region" ( dig "sigv4" "region" "" $raw | toString )
      "roleArn" ( dig "sigv4" "roleArn" "" $raw | toString ) )
    "tls" ( dict
      "enabled" $tlsEnabled
      "verify" $tlsVerify
      "ca" ( dig "ca" "" $tls | toString )
      "caEnv" ( dig "caEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_TLS_CA" $slug ) $tls | toString )
      "cert" ( dig "cert" "" $tls | toString )
      "certEnv" ( dig "certEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_TLS_CERT" $slug ) $tls | toString )
      "key" ( dig "key" "" $tls | toString )
      "keyEnv" ( dig "keyEnv" ( printf "GATEWAY_PROMETHEUS_DEST_%s_TLS_KEY" $slug ) $tls | toString )
      "caFile" ( dig "caFile" "" $tls | toString )
      "certFile" ( dig "certFile" "" $tls | toString )
      "keyFile" ( dig "keyFile" "" $tls | toString )
      "serverName" ( dig "serverName" "" $tls | toString )
      "minVersion" ( dig "minVersion" "TLS13" $tls | toString ) )
    | toYaml }}
{{- end }}


{{/*
Every Prometheus remote-write destination, resolved, in a stable order.

Returns a YAML list of resolved destinations — enabled and disabled alike, each
carrying its own `name` — so a caller filters on `.enabled` rather than
re-reading values. Sorted by name, because Go map iteration is randomized and an
unsorted list would reorder the rendered components on every render, rolling the
gateway for no reason (the pod template hashes the pipeline ConfigMap).

Usage:
  {{- range include "mzmon.alloyGateway.promDests" $ | fromYamlArray }}
    {{- if .enabled }}
*/}}
{{- define "mzmon.alloyGateway.promDests" }}
  {{- $dests := $.Values.pipeline.metrics.gateway.destination.prometheusRemoteWrite | default dict }}
  {{- $legacy := include "mzmon.alloyGateway.promDest.legacyKeys" $ | fromYamlArray }}
  {{- $out := list }}
  {{- range $name := ( keys $dests | sortAlpha ) }}
    {{- /* A leftover key from the pre-map shape is not a destination. Skipped
           rather than rendered as one, so the render reaches
           `mzmon.alloy.validate.promDests` and fails with the migration
           instruction instead of on a nonsense component named `url`. */}}
    {{- if not ( has $name $legacy ) }}
      {{- $out = append $out ( include "mzmon.alloyGateway.promDest.resolve" ( dict
            "name" $name
            "dest" ( index $dests $name ) ) | fromYaml ) }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{/*
The enabled remote-write destinations that write to the bundled in-cluster Thanos.

Several checks — Thanos reachability, the Receive TLS pairing, the client-auth
handshake — are about *this hop specifically*, not about remote-write in
general. With one destination the two were the same question; with a map they
are not, and applying a Thanos-shaped rule to a destination pointing at AMP
would be a false failure.

Matched on the in-cluster Service shape rather than the name alone, so an
external host that happens to be called `thanos-receive.<domain>` is left alone.

Usage:
  {{- range $dest := ( include "mzmon.alloyGateway.promDests.thanos" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.alloyGateway.promDests.thanos" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.alloyGateway.promDests" $ | fromYamlArray ) }}
    {{- if $dest.enabled }}
      {{- $url := tpl ( $dest.url | toString ) $ }}
      {{- if and ( contains "thanos-receive" $url ) ( contains ".svc" $url ) }}
        {{- $out = append $out ( merge ( dict "resolvedUrl" $url ) $dest ) }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{/*
Generate the alloy-gateway prometheus.remote_write blocks.

One `prometheus.relabel` + one `prometheus.remote_write` per enabled
destination, so each gets its own importance tier and its own WAL. The tier
filter sits *upstream* of the component on purpose: `write_relabel_config` on
the endpoint would filter on the way out of the WAL, so every destination would
pay full-firehose disk whatever tier it asked for.

The `keep` rule is unconditional even at the `all` tier, where the regex
resolves to `.*`. Rendering the rule only when it bites would make the tier a
structural change rather than a value change, and a `.*` match on `__name__` is
not a cost worth a second code path.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.prometheusRemoteWrite.dest" }}
  {{- range $dest := ( include "mzmon.alloyGateway.promDests" $ | fromYamlArray ) }}
    {{- if $dest.enabled }}

prometheus.relabel {{ $dest.name | quote }} {
    {{- /* Prometheus relabel regexes are fully anchored, so the tier union
           matches whole metric names without the `^(?:…)$` the otelcol filter
           has to write itself. */}}
    rule {
        action        = "keep"
        source_labels = ["__name__"]
        regex         = coalesce(sys.env({{ $dest.unfilteredMetricsEnv | quote }}), ".*")
    }

    forward_to = [
        prometheus.remote_write.{{ $dest.name }}.receiver,
    ]
}

prometheus.remote_write {{ $dest.name | quote }} {
    external_labels = {
      {{- if not ( hasKey $dest.externalLabels "cluster" ) }}
        cluster = sys.env("CLUSTER_NAME"),
      {{- end }}
      {{- range $k := ( keys $dest.externalLabels | sortAlpha ) }}
        {{ $k }} = {{ index $dest.externalLabels $k | toString | quote }},
      {{- end }}
    }
    endpoint {
        url = sys.env({{ $dest.urlEnv | quote }})
      {{- if eq $dest.authType "none" }}
      {{- else if eq $dest.authType "sigv4" }}

        sigv4 {
        {{- if $dest.sigv4.region }}
            region = {{ $dest.sigv4.region | quote }}
        {{- end }}
        {{- if $dest.sigv4.roleArn }}
            role_arn = {{ $dest.sigv4.roleArn | quote }}
        {{- end }}
        }
      {{- else if eq $dest.authType "basicAuth" }}

        basic_auth {
            username = sys.env({{ $dest.basicAuth.usernameEnv | required "basicAuth.usernameEnv" | quote }})
            password = sys.env({{ $dest.basicAuth.passwordEnv | required "basicAuth.passwordEnv" | quote }})
        }
      {{- else if eq $dest.authType "bearer" }}

        authorization {
            type = "Bearer"
            credentials = sys.env({{ $dest.bearer.tokenEnv | required "bearer.tokenEnv" | quote }})
        }
      {{- else }}
        {{- printf "Unsupported authType for pipeline.metrics.gateway.destination.prometheusRemoteWrite.%s: %s" $dest.name $dest.authType | fail }}
      {{- end }}
      {{- include "mzmon.alloy.tlsConfig" ( dict "tls" $dest.tls "indent" 8 ) }}
    }
}
    {{- end }}
  {{- end }}
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
  {{- else if eq $otelDestValues.auth.authType "headers" }}
    {{- tpl $otelDestValues.auth.headers.handler $ }}
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
  {{- else if eq $otelDestValues.auth.authType "headers" }}
    {{- tpl $otelDestValues.auth.headers.config $ }}
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

  {{- /* Output rendered destination */}}
  {{- include "mzmon.alloyAgent.pipeline.destination" $ }}
{{- end }}

{{/*
Generate the alloy-agent pipeline destinations.

Usage:
  {{- include "mzmon.alloyAgent.pipeline.destination" $ }}
*/}}
{{- define "mzmon.alloyAgent.pipeline.destination" }}
  {{- $dest := dig "logging" "agent" "destination" "loki" dict ( $.Values.pipeline | default dict ) }}
  {{- $tenancy := dig "logging" "tenancy" dict ( $.Values.pipeline | default dict ) }}
loki.process "egress" {
    forward_to = [
        loki.write.gateway.receiver,
    ]
}

loki.write "gateway" {
    endpoint {
        url = sys.env("AGENT_LOKI_DEST")
      {{- with $dest.retries }}
        max_backoff_period = {{ .maxBackoffPeriod | quote }}
        max_backoff_retries = {{ .maxBackoffRetries }}
        min_backoff_period = {{ .minBackoffPeriod | quote }}
        retry_on_http_429 = {{ .retryOnHttp429 }}
      {{- end }}
      {{- /* The agent stamps the tenant so the gateway does not have to infer
             it from a connection it cannot attribute. Static only: the agent
             has no per-stream tenancy of its own. */}}
      {{- if eq ( dig "tenantMap" "default" "" $tenancy ) "static" }}
        tenant_id = {{ $tenancy.staticTenant | quote }}
      {{- end }}
      {{- if eq ( $dest.authType | default "none" ) "none" }}
      {{- else if eq $dest.authType "basicAuth" }}

        basic_auth {
            username = sys.env({{ $dest.basicAuth.usernameEnv | required "basicAuth.usernameEnv" | quote }})
            password = sys.env({{ $dest.basicAuth.passwordEnv | required "basicAuth.passwordEnv" | quote }})
        }
      {{- else if eq $dest.authType "bearer" }}

        authorization {
            type = "Bearer"
            credentials = sys.env({{ $dest.bearer.tokenEnv | required "bearer.tokenEnv" | quote }})
        }
      {{- else }}
        {{- printf "Unsupported .Values.pipeline.logging.agent.destination.loki.authType (%s)" $dest.authType | fail }}
      {{- end }}
      {{- include "mzmon.alloy.tlsConfig" ( dict "tls" $dest.tls "indent" 8 ) }}
    }
}
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

  {{- $res := include "mzmon.alloy.validate.serverTls" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- range $release := ( list "alloy-agent" "alloy-gateway" ) }}
    {{- if hasKey $.Subcharts $release }}
      {{- $res := include "mzmon.alloy.validate.networkPolicy" ( dict "context" $ "release" $release ) | fromYaml }}
      {{- $errors = concat $errors $res.errors | default list }}
      {{- $warnings = concat $warnings $res.warnings | default list }}
    {{- end }}
  {{- end }}

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

    {{- $res := include "mzmon.alloy.validate.promDests" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- /* Auth is checked per destination against the *resolved* destination,
           so a credential left to a derived env var name is checked under the
           name the pipeline will actually read. */}}
    {{- range $dest := ( include "mzmon.alloyGateway.promDests" $ | fromYamlArray ) }}
      {{- $res := include "mzmon.alloy.validate.destAuth" ( dict
            "context" $
            "role" "alloy-gateway"
            "path" ( printf "pipeline.metrics.gateway.destination.prometheusRemoteWrite.%s" $dest.name )
            "dest" $dest
            "enabled" $dest.enabled ) | fromYaml }}
      {{- $errors = concat $errors $res.errors | default list }}
      {{- $warnings = concat $warnings $res.warnings | default list }}
    {{- end }}

    {{- if ( include "mzmon.alloyGateway.otelDest.authEnabled" $ ) }}
      {{- $res := include "mzmon.alloy.validate.otelDestAuth" $ | fromYaml }}
      {{- $errors = concat $errors $res.errors | default list }}
      {{- $warnings = concat $warnings $res.warnings | default list }}
    {{- end }}
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
    {{- $anyPromDest := false }}
    {{- range $dest := ( include "mzmon.alloyGateway.promDests" $ | fromYamlArray ) }}
      {{- if $dest.enabled }}
        {{- $anyPromDest = true }}
      {{- end }}
    {{- end }}
    {{- if and ( not $anyPromDest ) ( not $metrics.otel.enabled ) }}
      {{- $warnings = append $warnings "Every gateway metric destination is disabled (no pipeline.metrics.gateway.destination.prometheusRemoteWrite entry is enabled, and .otel.enabled is false). Metrics are scraped and then discarded." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the shape of the Prometheus remote-write destination map.

Three failures this catches, all of which otherwise present as a working install
that quietly writes somewhere other than where it was told to:

  1. **A pre-map override.** `prometheusRemoteWrite` used to be one destination.
     Helm deep-merges, so an old `prometheusRemoteWrite.url` lands beside
     `thanos` rather than replacing it: the key applies to nothing, the default
     Thanos endpoint keeps receiving, and nothing errors. Refused by name, with
     the destination-scoped path to move it to.

  2. **A name Alloy cannot use as a component label**, or one this pipeline has
     already taken. Alloy rejects the config at load with a parse error naming a
     line number, which is a long way from the values key that caused it.

  3. **An enabled destination with no URL.** `sys.env` of an unset variable is
     the empty string, and `prometheus.remote_write` accepts an empty endpoint
     URL at load, so this surfaces only as writes failing at run time.

Usage:
  {{- $res := include "mzmon.alloy.validate.promDests" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.promDests" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $path := "pipeline.metrics.gateway.destination.prometheusRemoteWrite" }}
  {{- $dests := $.Values.pipeline.metrics.gateway.destination.prometheusRemoteWrite | default dict }}
  {{- $legacy := include "mzmon.alloyGateway.promDest.legacyKeys" $ | fromYamlArray }}
  {{- $reserved := include "mzmon.alloyGateway.promDest.reservedNames" $ | fromYamlArray }}

  {{- $names := keys $dests | sortAlpha }}
  {{- $legacyFound := list }}
  {{- range $name := $names }}
    {{- if has $name $legacy }}
      {{- $legacyFound = append $legacyFound $name }}
    {{- end }}
  {{- end }}

  {{- if $legacyFound }}
    {{- /* One error rather than one per key: they are a single mistake, and
           the migration is the same move for all of them. */}}
    {{- $errors = append $errors ( printf "%s is now a map of named destinations, but it still carries the single-destination key(s) %s. Helm merges those alongside the named destinations instead of replacing them, so they would apply to nothing and the install would keep writing to the default endpoint with no error. Move them under a name — %s.<name>.%s — choosing `thanos` to keep the bundled Thanos Receive." $path ( join ", " $legacyFound ) $path ( first $legacyFound ) ) }}
  {{- end }}

  {{- range $dest := ( include "mzmon.alloyGateway.promDests" $ | fromYamlArray ) }}
    {{- $name := $dest.name }}
    {{- if has $name $legacy }}
      {{- /* Already reported above; do not also complain about its shape. */}}
    {{- else }}
      {{- if not ( regexMatch "^[a-zA-Z_][a-zA-Z0-9_]*$" $name ) }}
        {{- $errors = append $errors ( printf "%s.%s is not a usable name. It becomes an Alloy component label, so it must match [a-zA-Z_][a-zA-Z0-9_]* — no dashes, dots, or leading digits." $path $name ) }}
      {{- else if has $name $reserved }}
        {{- $errors = append $errors ( printf "%s.%s uses a name this pipeline has already taken (%s). Two components cannot share a label, so alloy would refuse the whole config at load. Pick another name." $path $name ( join ", " $reserved ) ) }}
      {{- end }}

      {{- if and $dest.enabled ( not $dest.url ) }}
        {{- $errors = append $errors ( printf "%s.%s is enabled but has no url. The rendered pipeline reads sys.env(%q), which would resolve empty — alloy accepts that at load and every remote write fails at run time." $path $name $dest.urlEnv ) }}
      {{- end }}
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

    {{- range $needed }}
      {{- $envName := .env | toString }}
      {{- $source := include "mzmon.alloy.envSource" ( dict
            "context" $ctx "role" $role "env" $envName ) | trim }}
      {{- if .value }}
        {{- if .secret }}
          {{- $warnings = append $warnings ( printf "%s.%s is set inline, so it renders into the pipeline env ConfigMap in plaintext — not a Secret. Prefer leaving it empty and supplying %s through %s.alloy.envFrom (a secretRef) or .extraEnv." $path .field $envName $role ) }}
        {{- end }}
      {{- else if eq $source "extraEnv" }}
        {{- /* explicitly provided */}}
      {{- else if eq $source "envFrom" }}
        {{- $warnings = append $warnings ( printf "%s.authType is %q but %s.%s is empty, so %s must come from %s.alloy.envFrom. That cannot be verified at render time; if the source does not set it, the credential resolves empty and authentication fails at run time." $path $authType $path .field $envName $role ) }}
      {{- else }}
        {{- $errors = append $errors ( printf "%s.authType is %q but %s.%s is empty and %s is set by neither %s.alloy.extraEnv nor .envFrom. The rendered pipeline reads sys.env(%q), which would resolve empty." $path $authType $path .field $envName $role $envName ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Where an Alloy role could get a given environment variable from.

Returns "extraEnv" when the role's `alloy.extraEnv` names it outright,
"envFrom" when the role has any `alloy.envFrom` source — those are ConfigMap and
Secret references whose contents are not readable at render time, so the most
that can be said is that something *might* supply it — and "" when neither
could.

Usage:
  {{- $source := include "mzmon.alloy.envSource" ( dict
        "context" $ "role" "alloy-gateway" "env" "GATEWAY_OTEL_DEST_..." ) | trim }}
*/}}
{{- define "mzmon.alloy.envSource" }}
  {{- $ctx := .context | required ".context must be specified" }}
  {{- $role := .role | required ".role must be specified" }}
  {{- $envName := .env | required ".env must be specified" | toString }}

  {{- $alloy := dig "alloy" dict ( index $ctx.Values $role | default dict ) }}
  {{- $extraEnvNames := list }}
  {{- range dig "extraEnv" list $alloy }}
    {{- if .name }}
      {{- $extraEnvNames = append $extraEnvNames ( .name | toString ) }}
    {{- end }}
  {{- end }}

  {{- if has $envName $extraEnvNames }}
    {{- "extraEnv" }}
  {{- else if dig "envFrom" list $alloy }}
    {{- "envFrom" }}
  {{- end }}
{{- end }}

{{- /*
Validate the gateway's OTLP destination auth.

This block is shaped differently from the Loki and Prometheus destinations —
one `auth` stanza shared by every OTLP exporter, selected by `authType` — so it
is checked here rather than through `mzmon.alloy.validate.destAuth`.

Only `headers` is checked structurally. The other types render a fixed config
whose env var names are fixed too, and `custom` is a raw escape hatch with
nothing to check. `headers` is caller-built, so it can be built wrong: an empty
list renders an `otelcol.auth.headers` block that attaches nothing, and a header
with neither `value` nor `valueEnv` sends an empty string. Both are accepted by
Alloy and rejected by the backend, at run time, as an authentication failure
that names nothing.

Usage:
  {{- $res := include "mzmon.alloy.validate.otelDestAuth" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.otelDestAuth" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $path := "pipeline.metrics.gateway.destination.otel.auth" }}
  {{- $auth := $.Values.pipeline.metrics.gateway.destination.otel.auth }}

  {{- if eq ( $auth.authType | toString ) "headers" }}
    {{- $headers := dig "headers" "headers" list $auth }}
    {{- if not $headers }}
      {{- $errors = append $errors ( printf "%s.authType is \"headers\" but %s.headers.headers is empty, so the rendered otelcol.auth.headers block would attach no headers at all." $path $path ) }}
    {{- end }}

    {{- range $i, $h := $headers }}
      {{- $at := printf "%s.headers.headers[%d]" $path $i }}
      {{- if not $h.key }}
        {{- $errors = append $errors ( printf "%s.key is empty; every header needs a name." $at ) }}
      {{- end }}

      {{- if and $h.value $h.valueEnv }}
        {{- $errors = append $errors ( printf "%s sets both .value and .valueEnv. Set exactly one — .value renders the header inline, .valueEnv reads it from the environment — because the rendered config can only use one and silently prefers .valueEnv." $at ) }}
      {{- else if not ( or $h.value $h.valueEnv ) }}
        {{- $errors = append $errors ( printf "%s sets neither .value nor .valueEnv, so header %q would be sent with an empty value." $at ( $h.key | toString ) ) }}
      {{- else if $h.valueEnv }}
        {{- $envName := $h.valueEnv | toString }}
        {{- $source := include "mzmon.alloy.envSource" ( dict
              "context" $ "role" "alloy-gateway" "env" $envName ) | trim }}
        {{- if eq $source "extraEnv" }}
          {{- /* explicitly provided */}}
        {{- else if eq $source "envFrom" }}
          {{- $warnings = append $warnings ( printf "%s.valueEnv is %q, so header %q must come from alloy-gateway.alloy.envFrom. That cannot be verified at render time; if the source does not set it, the header is sent empty and the destination rejects the request." $at $envName ( $h.key | toString ) ) }}
        {{- else }}
          {{- $errors = append $errors ( printf "%s.valueEnv is %q, but that variable is set by neither alloy-gateway.alloy.extraEnv nor .envFrom. The rendered config reads sys.env(%q), which would resolve empty." $at $envName $envName ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the NetworkPolicy on one Alloy release.

Both releases wrap the same subchart, so both get the same checks; the caller
names which one. The subchart's template renders nothing at all unless
`flavor` is `kubernetes`, and it declares only the policy types listed in
`policyTypes` — so the ways to end up with a policy that is absent, inert, or a
silent deny are all shapes of the values rather than shapes of the rules.

Usage:
  {{- include "mzmon.alloy.validate.networkPolicy" ( dict "context" $ "release" "alloy-agent" ) }}
*/}}
{{- define "mzmon.alloy.validate.networkPolicy" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $context := .context | required ".context must be specified" }}
  {{- $release := .release | required ".release must be specified" }}
  {{- $values := index $context.Values $release | required ( printf "%s is missing from values." $release ) }}
  {{- $np := $values.networkPolicy | default dict }}

  {{- if $np.enabled }}
    {{- $flavor := $np.flavor | default "" }}
    {{- /* The template is gated on `eq flavor "kubernetes"` and there is no
           else branch, so any other value renders no policy whatsoever — an
           `enabled: true` that produces nothing and reports nothing. */}}
    {{- if ne $flavor "kubernetes" }}
      {{- $errors = append $errors ( printf "%s.networkPolicy.flavor is %q, and the subchart renders a policy only for \"kubernetes\". Nothing would be created, and the render would not tell you." $release $flavor ) }}
    {{- end }}

    {{- $types := $np.policyTypes | default list }}
    {{- if not $types }}
      {{- $errors = append $errors ( printf "%s.networkPolicy.policyTypes is empty, which leaves the policy with no direction to enforce." $release ) }}
    {{- end }}

    {{- if has "Ingress" $types }}
      {{- if not $np.ingress }}
        {{- $warnings = append $warnings ( printf "%s.networkPolicy declares the Ingress type with no rules, which denies every inbound connection. The gateway's scrape of /metrics on 12345 is the first thing that stops, and it stops silently." $release ) }}
      {{- else }}
        {{- /* The subchart ships `ingress: [{}]` — an empty rule, which allows
               everything. Turning the policy on without replacing it is the
               most likely way to end up believing the pod is protected. */}}
        {{- range $rule := $np.ingress }}
          {{- if not $rule }}
            {{- $warnings = append $warnings ( printf "%s.networkPolicy.ingress still contains the subchart's empty `{}` rule, which allows every source on every port. The policy renders and enforces nothing." $release ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if has "Egress" $types }}
      {{- /* Alloy is an API-server client before it is anything else:
             discovery.kubernetes backs pod-log collection on the agent and every
             ServiceMonitor/PodMonitor target on the gateway. With no egress rule
             the process starts, reports healthy, and discovers nothing. */}}
      {{- if not $np.egress }}
        {{- $errors = append $errors ( printf "%s.networkPolicy declares the Egress type with no rules, which denies all egress. Alloy would come up healthy and collect nothing: discovery.kubernetes could not reach the API server, and the agent could not reach the gateway. Either give it rules or drop \"Egress\" from policyTypes." $release ) }}
      {{- end }}
    {{- end }}
  {{- else }}
    {{- $warnings = append $warnings ( printf "%s.networkPolicy.enabled is recommended in production." $release ) }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Derive a destination URL's scheme from whether TLS is on.

Every in-cluster destination default is written `http://`, and `tls.enabled: true`
against an `http://` URL is either ignored or fails confusingly depending on the
component — a plaintext request to a TLS port, which reports as a protocol error
rather than as a configuration one. Deriving the scheme is what stops a correct
TLS config from presenting as a broken endpoint.

An explicit `https://` is left alone, and so is any scheme the operator wrote
that is not `http://`: the point is to fix the chart's own default, not to
second-guess a URL someone typed.

Usage:
  {{ include "mzmon.alloy.destUrl" ( dict "url" $rendered "tls" $dest.tls ) }}
*/}}
{{- define "mzmon.alloy.destUrl" -}}
  {{- $url := .url | toString }}
  {{- $tls := .tls | default dict }}
  {{- if and $tls.enabled ( hasPrefix "http://" $url ) -}}
    {{- printf "https://%s" ( trimPrefix "http://" $url ) -}}
  {{- else -}}
    {{- $url -}}
  {{- end -}}
{{- end }}

{{- /*
Render an alloy `tls_config` block from a destination's `tls` values.

Emits nothing when TLS is off, so a caller can include it unconditionally.

File carriers win over the inline/env ones when both are set, and the block is
built so that only what is configured appears: a server-only TLS hop (phase 1 of
a rollout) has a CA and no client keypair, and emitting empty `cert_file` /
`key_file` attributes would make alloy reject the config rather than skip them.

Usage:
  {{- include "mzmon.alloy.tlsConfig" ( dict "tls" $dest.tls "indent" 8 ) }}
*/}}
{{- define "mzmon.alloy.tlsConfig" }}
  {{- $tls := .tls | default dict }}
  {{- if $tls.enabled }}
    {{- $pad := repeat ( .indent | default 8 | int ) " " }}
    {{- $lines := list }}
    {{- /* `not ( $tls.verify | default true )` would be dead: `default` returns
           its default for any *falsy* input, so `false | default true` is
           `true` and the negation is always false. Test presence and value
           separately. */}}
    {{- if and ( hasKey $tls "verify" ) ( not $tls.verify ) }}
      {{- $lines = append $lines "insecure_skip_verify = true" }}
    {{- end }}
    {{- if $tls.caFile }}
      {{- $lines = append $lines ( printf "ca_file = %s" ( $tls.caFile | quote ) ) }}
    {{- else if $tls.ca }}
      {{- $lines = append $lines ( printf "ca_pem = sys.env(%s)" ( $tls.caEnv | required "tls.caEnv is required when tls.ca is set" | quote ) ) }}
    {{- end }}
    {{- if $tls.certFile }}
      {{- $lines = append $lines ( printf "cert_file = %s" ( $tls.certFile | quote ) ) }}
    {{- else if $tls.cert }}
      {{- $lines = append $lines ( printf "cert_pem = sys.env(%s)" ( $tls.certEnv | required "tls.certEnv is required when tls.cert is set" | quote ) ) }}
    {{- end }}
    {{- if $tls.keyFile }}
      {{- $lines = append $lines ( printf "key_file = %s" ( $tls.keyFile | quote ) ) }}
    {{- else if $tls.key }}
      {{- $lines = append $lines ( printf "key_pem = sys.env(%s)" ( $tls.keyEnv | required "tls.keyEnv is required when tls.key is set" | quote ) ) }}
    {{- end }}
    {{- with $tls.serverName }}
      {{- $lines = append $lines ( printf "server_name = %s" ( . | quote ) ) }}
    {{- end }}
    {{- with $tls.minVersion }}
      {{- $lines = append $lines ( printf "min_version = %s" ( . | quote ) ) }}
    {{- end }}
{{ $pad }}tls_config {
    {{- range $line := $lines }}
{{ $pad }}    {{ $line }}
    {{- end }}
{{ $pad }}}
  {{- end }}
{{- end }}

{{- /*
Render a server-side `tls` block for one of the gateway's listeners.

The mirror of `mzmon.alloy.tlsConfig`, which configures Alloy as a client. Emits
nothing when TLS is off, so a caller can include it unconditionally.

**Two flavours, because the listeners do not share a TLS schema** — measured
against the alloy binary this chart pins, not read off documentation:

  * `flavor: alloy` — `loki.source.api`'s `http` block. dskit-style, and the only
    one that takes `client_auth_type`, so it is the only listener with a real
    verify-if-given state.
  * `flavor: otelcol` — `otelcol.receiver.otlp`'s `grpc` / `http` blocks. Rejects
    `client_auth_type` outright; **client auth is implied by the presence of
    `client_ca_file`**, and that means require-and-verify. Same shape as Thanos
    Receive, and the same trap: there is no middle state, so the CA must not
    appear until every client already presents one. It does take
    `reload_interval`, which is the best renewal behaviour of anything here —
    the CA and keypair are re-read on a timer rather than only on new
    connections.

So one `clientAuth` value produces different behaviour on different listeners of
the same process. `mzmon.alloy.validate.serverTls` says so out loud rather than
leaving it to be discovered.

Usage:
  {{- include "mzmon.alloy.serverTls" ( dict "tls" $tls "flavor" "otelcol" "indent" 8 ) }}
*/}}
{{- define "mzmon.alloy.serverTls" }}
  {{- $tls := .tls | default dict }}
  {{- if $tls.enabled }}
    {{- $flavor := .flavor | default "alloy" }}
    {{- $pad := repeat ( .indent | default 8 | int ) " " }}
    {{- $auth := $tls.clientAuth | default "NoClientCert" }}
    {{- $lines := list }}
    {{- $lines = append $lines ( printf "cert_file = %s" ( $tls.certFile | required "pipeline.*.gateway.server.tls.certFile is required when server TLS is enabled" | quote ) ) }}
    {{- $lines = append $lines ( printf "key_file = %s" ( $tls.keyFile | required "pipeline.*.gateway.server.tls.keyFile is required when server TLS is enabled" | quote ) ) }}
    {{- if ne $auth "NoClientCert" }}
      {{- $lines = append $lines ( printf "client_ca_file = %s" ( $tls.clientCAFile | required ( printf "pipeline.*.gateway.server.tls.clientAuth is %q but clientCAFile is unset, so the listener has nothing to verify presented certificates against. The CA and the policy arrive together in profiles/mtls-phase2.values.yaml — composing phase 3 without phase 2 is what produces this." $auth ) | quote ) ) }}
      {{- /* Only the dskit-flavoured listener can be told *how hard* to insist;
             on otelcol the CA above has already said "require". */}}
      {{- if eq $flavor "alloy" }}
        {{- $lines = append $lines ( printf "client_auth_type = %s" ( $auth | quote ) ) }}
      {{- end }}
    {{- end }}
    {{- with $tls.minVersion }}
      {{- /* **The same concept has three vocabularies in one binary**, and two
             of them fail in different ways. Client blocks (`tls_config`) take
             `TLS13`. The dskit server block takes `VersionTLS13` and rejects
             `TLS13` at load, crashlooping the pod. otelcol takes `1.3` and
             rejects the other two *silently* — the component goes unhealthy,
             the listener never binds, and the process stays up. `alloy validate`
             catches none of it. Values use one vocabulary and this translates. */}}
      {{- $version := . | toString | trimPrefix "Version" }}
      {{- if eq $flavor "alloy" }}
        {{- $version = printf "Version%s" $version }}
      {{- else }}
        {{- /* otelcol speaks OpenTelemetry's dotted form and rejects both other
               spellings — and it does so *silently*: the component goes
               unhealthy with `unsupported TLS version`, the listener never
               binds, and alloy keeps running. Nothing crashes and nothing in
               `alloy validate` or the pod's status says so. */}}
        {{- $version = get ( dict "TLS10" "1.0" "TLS11" "1.1" "TLS12" "1.2" "TLS13" "1.3" ) $version | default $version }}
      {{- end }}
      {{- $lines = append $lines ( printf "min_version = %s" ( $version | quote ) ) }}
    {{- end }}
    {{- if eq $flavor "otelcol" }}
      {{- with $tls.reloadInterval }}
        {{- $lines = append $lines ( printf "reload_interval = %s" ( . | quote ) ) }}
      {{- end }}
    {{- end }}
{{ $pad }}tls {
    {{- range $line := $lines }}
{{ $pad }}    {{ $line }}
    {{- end }}
{{ $pad }}}
  {{- end }}
{{- end }}

{{- /*
Validate the gateway's server-side TLS against what its listeners can express.

Three things go wrong here and none of them is visible in the values:

  * **The agent and the gateway have to move together.** A client on TLS against
    a plaintext listener, or the reverse, fails as a protocol error that reads
    like the peer is broken.
  * **One `clientAuth` means different things on different listeners of the same
    process.** `loki.source.api` honours it; `otelcol.receiver.otlp` has no such
    attribute and treats the presence of a client CA as require-and-verify. So
    at `VerifyClientCertIfGiven` the logs listener tolerates a client that
    presents nothing while the OTLP listeners reject it.

Usage:
  {{- $res := include "mzmon.alloy.validate.serverTls" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.serverTls" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $pipeline := $.Values.pipeline | default dict }}
  {{- $logServer := dig "logging" "gateway" "server" "tls" dict $pipeline }}
  {{- $metricServer := dig "metrics" "gateway" "server" "tls" dict $pipeline }}
  {{- $agentDest := dig "logging" "agent" "destination" "loki" dict $pipeline }}
  {{- $agentTls := $agentDest.tls | default dict }}

  {{- /* Only the four names the translation understands. An unrecognized value
         reaches the listener verbatim and fails at load, not at render. */}}
  {{- range $side := ( list ( dict "path" "logging" "tls" $logServer ) ( dict "path" "metrics" "tls" $metricServer ) ) }}
    {{- $v := ( $side.tls.minVersion | default "" ) | toString | trimPrefix "Version" }}
    {{- if and $side.tls.enabled $v ( not ( has $v ( list "TLS10" "TLS11" "TLS12" "TLS13" ) ) ) }}
      {{- $errors = append $errors ( printf "pipeline.%s.gateway.server.tls.minVersion is %q, which is not one of TLS10/TLS11/TLS12/TLS13. An unrecognized version reaches the listener verbatim and fails at load rather than at render — the gateway crashloops with `TLS version ... not recognized`, which alloy validate does not catch." $side.path $side.tls.minVersion ) }}
    {{- end }}
  {{- end }}

  {{- /* The two ingress trees can be secured independently, which is a real
         choice — the metrics port has different clients from the logs ports —
         but securing only one is almost always an oversight rather than that
         choice, so it warns in both directions. */}}
  {{- if and $logServer.enabled ( not $metricServer.enabled ) }}
    {{- $warnings = append $warnings "pipeline.logging.gateway.server.tls is on but pipeline.metrics.gateway.server.tls is not, so the gateway's logs listeners (3100, 4317, 4318) serve TLS while prometheus.receive_http on 9090 stays plaintext. Anything that can reach 9090 can still write arbitrary series. Secure both, or keep the NetworkPolicy on that port and say why." }}
  {{- end }}
  {{- if and $metricServer.enabled ( not $logServer.enabled ) }}
    {{- $warnings = append $warnings "pipeline.metrics.gateway.server.tls is on but pipeline.logging.gateway.server.tls is not, so the remote-write listener on 9090 serves TLS while the log-ingest listeners on 3100/4317/4318 stay plaintext." }}
  {{- end }}

  {{- if ( include "mzmon.alloyGateway.enabled" $ ) }}
    {{- if $logServer.enabled }}
      {{- $auth := $logServer.clientAuth | default "NoClientCert" }}
      {{- if eq $auth "VerifyClientCertIfGiven" }}
        {{- $warnings = append $warnings "pipeline.logging.gateway.server.tls.clientAuth is VerifyClientCertIfGiven, which the OTLP listeners cannot express: otelcol.receiver.otlp has no client_auth_type and treats the client CA as require-and-verify. So loki.source.api will serve a client that presents nothing while 4317/4318 reject it. That is safe for the agent hop (the agent presents once phase 2 is applied) and will break any other OTLP sender that has not rolled." }}
      {{- end }}
    {{- end }}

    {{- /* Both halves of the agent hop. */}}
    {{- if ( include "mzmon.alloyAgent.enabled" $ ) }}
      {{- if and $agentTls.enabled ( not $logServer.enabled ) }}
        {{- $errors = append $errors "pipeline.logging.agent.destination.loki.tls.enabled is on but pipeline.logging.gateway.server.tls.enabled is off, so the agent would send TLS at a plaintext listener. Every log push fails with a protocol error that reads like the gateway is broken." }}
      {{- end }}
      {{- if and $logServer.enabled ( not $agentTls.enabled ) }}
        {{- $errors = append $errors "pipeline.logging.gateway.server.tls.enabled is on but pipeline.logging.agent.destination.loki.tls.enabled is off, so the agent would send plaintext at a TLS listener and every log push fails. Turn the agent's destination TLS on in the same release." }}
      {{- end }}
      {{- /* Phase 3 on the listener with a client that presents nothing. */}}
      {{- if and ( has ( $logServer.clientAuth | default "NoClientCert" ) ( list "RequireAndVerifyClientCert" "RequireAnyClientCert" ) ) $agentTls.enabled }}
        {{- if not ( and $agentTls.certFile $agentTls.keyFile ) }}
          {{- $errors = append $errors ( printf "pipeline.logging.gateway.server.tls.clientAuth is %q but the agent presents no client certificate (no certFile/keyFile on pipeline.logging.agent.destination.loki.tls). Every log push would be refused at the TLS handshake. Give the agent its keypair first and let the DaemonSet roll before requiring one." ( $logServer.clientAuth ) ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
