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

  {{- /* Cloud provider pulls, as custom components. Always included: a
         `declare` nothing instantiates costs nothing, and this way the file the
         pre-validate job checks is the one `make pipelines` validated. */}}
  {{- $.Files.Get "pre-rendered/pipelines/gateway-provider.alloy" }}

  {{- /* Output rendered sources */}}
  {{- include "mzmon.alloyGateway.pipeline.sources" $ }}

  {{- /* Output cloud provider pulls, when any is enabled */}}
  {{- include "mzmon.alloyGateway.pipeline.provider" $ }}

  {{- /* Output rendered destination */}}
  {{- include "mzmon.alloyGateway.pipeline.destination" $ }}

  {{- /* Deliver the LogQL alerting rules to the Loki ruler */}}
  {{- include "mzmon.alloyGateway.pipeline.lokiRules" $ }}
{{- end }}

{{/*
Deliver the LogQL alerting rules to the bundled Loki ruler.

The chart's log-derived rules are PrometheusRules labelled
`mzmon.materialize.cloud/flavor: logql` (`templates/alerts/lokirules.yaml`).
`loki.rules.kubernetes` watches them and writes their groups into the ruler
through its API, under Loki rule namespaces prefixed `mzmon`. It removes the
namespaces it wrote whose PrometheusRule is gone and leaves every other one
alone, so rules written to the API by anything else survive. A deployment's own
LogQL PrometheusRule carrying the label is delivered the same way.

**One block per tenant** in `mzmon.rules.logTenants`, because a Loki rule group
belongs to one tenant and the component writes as one. The component label is
the tenant's position rather than its id, which may hold characters an Alloy
label may not.

Every gateway replica runs these blocks. The component does not cluster, and
each write sets a whole rule group, so the replicas converge on the same set.

The address is the Service that serves the ruler in Loki's deployment mode
(`mzmon.loki.ruler.service`), with the scheme and TLS settings of the Loki
destination: the bundled Loki serves one scheme and one certificate on every
component, and `certificates.components.loki.services` names the ruler.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.lokiRules" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.lokiRules" }}
  {{- if ( include "mzmon.rules.logRuleSync.enabled" $ ) }}
    {{- $lokiTls := include "mzmon.destinations.bundledLokiTls" $ | fromYaml }}
    {{- $ruler := printf "http://%s.%s.svc:3100" ( include "mzmon.loki.ruler.service" $ ) ( include "mzmon.loki.namespace" $ ) }}
    {{- $address := include "mzmon.alloy.destUrl" ( dict "url" $ruler "tls" $lokiTls ) }}
    {{- range $i, $tenant := ( include "mzmon.rules.logTenants" $ | fromYamlArray ) }}

loki.rules.kubernetes {{ printf "tenant_%d" $i | quote }} {
    address = {{ $address | quote }}
    tenant_id = {{ $tenant | quote }}
    loki_namespace_prefix = "mzmon"

    rule_selector {
        match_labels = {
            {{ include "mzmon.rules.flavorLabel" $ | quote }} = "logql",
        }
    }
      {{- include "mzmon.alloy.tlsConfig" ( dict "tls" $lokiTls "indent" 4 ) }}
}
    {{- end }}
  {{- end }}
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
        loki.process.inputPrefilter.receiver,
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
            otelcol.processor.transform.inputMetricProcessor.input,
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
        prometheus.relabel.receiveIdentity.receiver,
    ]

    http {
        listen_port = 9090
      {{- include "mzmon.alloy.serverTls" ( dict "tls" $metricTls "flavor" "alloy" "indent" 8 ) }}
    }
}

{{- /* The OTLP bridge refuses a sample without both `job` and `instance`
       ("job or instance cannot be found from labels"), and refuses the whole
       remote-write batch it arrived in with a 500, which the sender retries
       forever. Scraped series always carry both; pushed ones need not, and
       rule results never do. So a pushed sample missing either gets one here,
       before the bridge, and a sample that has both is left alone.

       The Thanos ruler is recognised by `ruler_replica`, the pod name its
       subchart stamps on every sample. Its `instance` is a constant, not that
       pod name: Thanos Query deduplicates the two replicas on `ruler_replica`
       alone, and a per-pod `instance` would keep every alert and recording
       rule twice. It cannot fill these itself: Thanos
       loads its remote-write file without a label-name validation scheme, and
       a `replace` in `write_relabel_configs` that reads a label, as filling
       only a missing one must, panics the ruler on its first sample.
       The Loki ruler fills its own, in `loki.loki.rulerConfig`. */}}
prometheus.relabel "receiveIdentity" {
    forward_to = [
        otelcol.receiver.prometheus.inputBridge.receiver,
    ]

    rule {
        source_labels = ["job", "ruler_replica"]
        regex         = ";.+"
        target_label  = "job"
        replacement   = "thanos-ruler"
    }

    rule {
        source_labels = ["instance", "ruler_replica"]
        regex         = ";.+"
        target_label  = "instance"
        replacement   = "thanos-ruler"
    }

    rule {
        source_labels = ["job"]
        regex         = ""
        target_label  = "job"
        replacement   = "remote-write"
    }

    rule {
        source_labels = ["instance"]
        regex         = ""
        target_label  = "instance"
        replacement   = "unknown"
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
  {{- /* Each log destination hangs off the shared loki.process "egress" seam
         by its own writer, so each has its own queue and its own drop policy. */}}
  {{- range $dest := ( include "mzmon.destinations.logs" $ | fromYamlArray ) }}
    {{- $writer := "" }}
    {{- if eq $dest.type "loki" }}
      {{- $writer = printf "loki.write.%s.receiver" $dest.label }}
      {{- include "mzmon.alloyGateway.pipeline.loki.dest" ( dict "context" $ "dest" $dest ) }}
    {{- else }}
      {{- $writer = printf "otelcol.receiver.loki.%s.receiver" $dest.label }}
    {{- end }}
    {{- /* A destination that reduces its logs gets a loki.process of its own
           between the seam and its writer, so the reduction applies to its
           copy and no other. */}}
    {{- if ( include "mzmon.destinations.reducesLogs" $dest ) }}
      {{- include "mzmon.alloyGateway.pipeline.logReduction" ( dict "context" $ "dest" $dest "forwardTo" $writer ) }}
      {{- $writer = printf "loki.process.dest_%s.receiver" $dest.name }}
    {{- end }}
    {{- $logForward = append $logForward $writer }}
  {{- end }}
  {{- /* An OTLP-family destination's metrics hang off the
         otelcol.processor.filter "egress" seam by their own tier filter. Only
         `loki` carries no metrics, and the validator refuses metrics on it. */}}
  {{- range $dest := ( include "mzmon.destinations.metrics" $ | fromYamlArray ) }}
    {{- if has $dest.type ( list "otlp" "googleCloud" "datadog" "s3" ) }}
      {{- $metricsForward = append $metricsForward ( printf "otelcol.processor.transform.%s.input" $dest.label ) }}
    {{- end }}
  {{- end }}
  {{- range $dest := ( include "mzmon.destinations.active" $ | fromYamlArray ) }}
    {{- if has $dest.type ( list "otlp" "googleCloud" "datadog" "s3" ) }}
      {{- include "mzmon.alloyGateway.pipeline.otel.dest" ( dict "context" $ "dest" $dest ) }}
    {{- end }}
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
Generate the `loki.write` block for one log destination.

The `queue_config` block appears only when the destination sets a queue field,
so a destination that leaves the queue alone renders as it always has. Alloy
marks `queue_config` experimental, which the gateway's stability level allows.

Args:
  context: the root context.
  dest: a destination resolved by `mzmon.destinations.resolve`.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.loki.dest" ( dict "context" $ "dest" $dest ) }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.loki.dest" }}
  {{- $root := .context }}
  {{- $dest := .dest }}
loki.write {{ $dest.label | quote }} {
    endpoint {
        url = sys.env({{ $dest.urlEnv | quote }})
        max_backoff_period = {{ $dest.retries.maxBackoffPeriod | quote }}
        max_backoff_retries = {{ $dest.retries.maxBackoffRetries }}
        min_backoff_period = {{ $dest.retries.minBackoffPeriod | quote }}
        retry_on_http_429 = {{ $dest.retries.retryOnHttp429 }}
  {{- if eq $root.Values.pipeline.logging.tenancy.tenantMap.default "static" }}
        tenant_id = {{ $root.Values.pipeline.logging.tenancy.staticTenant | quote }}
  {{- end }}
      {{- if eq $dest.authType "none" }}
      {{- else if eq $dest.authType "basicAuth" }}

        basic_auth {
            username = sys.env({{ $dest.basicAuth.usernameEnv | quote }})
            password = sys.env({{ $dest.basicAuth.passwordEnv | quote }})
        }
      {{- else if eq $dest.authType "bearer" }}

        authorization {
            type = "Bearer"
            credentials = sys.env({{ $dest.bearer.tokenEnv | quote }})
        }
      {{- else }}
        {{- printf "Unsupported pipeline.destinations.%s.authType (%s)" $dest.name $dest.authType | fail }}
      {{- end }}
      {{- with $dest.queue }}

        queue_config {
        {{- if hasKey . "blockOnOverflow" }}
            block_on_overflow = {{ .blockOnOverflow }}
        {{- end }}
        {{- if hasKey . "capacity" }}
            capacity = {{ .capacity | toString | quote }}
        {{- end }}
        {{- if hasKey . "drainTimeout" }}
            drain_timeout = {{ .drainTimeout | toString | quote }}
        {{- end }}
        {{- if hasKey . "minShards" }}
            min_shards = {{ .minShards | int }}
        {{- end }}
        }
      {{- end }}
      {{- include "mzmon.alloy.tlsConfig" ( dict "tls" $dest.tls "indent" 8 ) }}
    }
}
{{- end }}


{{/*
Generate the `loki.process "dest_<name>"` that reduces one destination's logs.

Rendered only for a destination that sets a reduction field, between the
`egress` seam and the destination's writer, so the customer's own copy keeps
everything. The stages run cheapest first, and each drop is counted in
`loki_process_dropped_lines_total` under its own reason:

  1. **selector**, an allowlist. LogQL cannot negate a stream selector, so every
     entry is marked `__dest_keep="false"`, entries the selector matches are
     re-marked `"true"`, and the rest are dropped. The marker is removed before
     the entry leaves.
  2. **minLevel**, dropping the levels below it, and `UNKNOWN` as well when
     `keepUnknown` is false; see `mzmon.destinations.levelsBelow`.
  3. **sampleRate**, through `stage.sampling`.
  4. **rateLimit**, through `stage.limit` with `drop = true`. Without `drop` the
     stage backpressures, which would hold back every other destination.

Args:
  context: the root context.
  dest: a destination resolved by `mzmon.destinations.resolve`.
  forwardTo: the destination writer's receiver.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.logReduction" ( dict "context" $ "dest" $dest "forwardTo" $writer ) }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.logReduction" }}
  {{- $root := .context }}
  {{- $dest := .dest }}
  {{- $logs := $dest.logs }}

loki.process {{ printf "dest_%s" $dest.name | quote }} {
    forward_to = [{{ .forwardTo }}]
  {{- with $logs.selector }}

    stage.static_labels {
        values = {
            "__dest_keep" = "false",
        }
    }

    stage.match {
        selector = {{ . | quote }}

        stage.static_labels {
            values = {
                "__dest_keep" = "true",
            }
        }
    }

    stage.match {
        selector = "{__dest_keep=\"false\"}"
        action = "drop"
        drop_counter_reason = "destination_selector"
    }

    stage.label_drop {
        values = ["__dest_keep"]
    }
  {{- end }}
  {{- $dropLevels := list }}
  {{- with $logs.minLevel }}
    {{- $dropLevels = index ( include "mzmon.destinations.levelsBelow" $root | fromYaml ) . | default list }}
  {{- end }}
  {{- if not $logs.keepUnknown }}
    {{- $dropLevels = append $dropLevels "UNKNOWN" }}
  {{- end }}
  {{- if $dropLevels }}

    stage.match {
        selector = {{ printf "{level=~%q}" ( join "|" $dropLevels ) | quote }}
        action = "drop"
        drop_counter_reason = "destination_level"
    }
  {{- end }}
  {{- if lt ( $logs.sampleRate | float64 ) 1.0 }}

    stage.sampling {
        rate = {{ $logs.sampleRate }}
        drop_counter_reason = "destination_sampling"
    }
  {{- end }}
  {{- if $logs.rateLimit.configured }}

    stage.limit {
        rate = {{ $logs.rateLimit.rate }}
        burst = {{ $logs.rateLimit.burst | int }}
        drop = true
    }
  {{- end }}
}
{{- end }}


{{/*
Render a `sending_queue` block from a destination's resolved `queue`, or
nothing when it sets no field. Shared by every otelcol exporter a destination
renders; each drops when its queue is full unless `blockOnOverflow` is true.

Usage:
  {{- include "mzmon.alloyGateway.otel.sendingQueue" $dest.queue }}
*/}}
{{- define "mzmon.alloyGateway.otel.sendingQueue" }}
  {{- with . }}

    sending_queue {
    {{- if hasKey . "blockOnOverflow" }}
        block_on_overflow = {{ .blockOnOverflow }}
    {{- end }}
    {{- if hasKey . "numConsumers" }}
        num_consumers = {{ .numConsumers | int }}
    {{- end }}
    {{- if hasKey . "queueSize" }}
        queue_size = {{ .queueSize | int }}
    {{- end }}
    }
  {{- end }}
{{- end }}


{{/*
Generate the components for one OTLP-family destination: `otlp`, `datadog`, or
`googleCloud`.

  * The exporter. `otlp` renders `otelcol.exporter.otlphttp`, or
    `otelcol.exporter.otlp` for gRPC. `datadog` renders
    `otelcol.exporter.datadog`, with its endpoints only when overridden, so they
    follow `site`. `googleCloud` renders the Telemetry API chain
    (`googleCloud.chain`), labeled `gcp_<name>`. Its filter and transform
    need a label of their own beside the tier filter's `dest_<name>`, and no
    `dest_` label can equal a `gcp_` one: a suffix such as `dest_<name>_gcp`
    would be the tier filter's label for a destination named `<name>_gcp`.
  * For logs, `otelcol.receiver.loki` → `otelcol.processor.batch` in front of
    the exporter. The receiver converts one entry at a time, and the batch is
    what turns that back into requests of a useful size.
  * For metrics, the per-destination tier filter (`otelDest.egressFilter`) in
    front of the exporter, or of the Google Cloud chain. Metrics are already
    batched upstream of the seam.
  * For an `otlp` destination with credentials, an `otelcol.auth.*` handler of
    its own.

Every component is labeled `dest_<name>` (`$dest.label`).

An `otelcol` TLS block encrypts by default, so an `otlp` destination with TLS
off renders `insecure = true` rather than nothing: without it a gRPC exporter
would try TLS against a plaintext listener. otelcol spells the minimum version
`1.3`, and rejects `TLS13` silently, so the value is translated.

Args:
  context: the root context.
  dest: a destination resolved by `mzmon.destinations.resolve`.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.otel.dest" ( dict "context" $ "dest" $dest ) }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.otel.dest" }}
  {{- $root := .context }}
  {{- $dest := .dest }}
  {{- $label := $dest.label }}
  {{- $exporter := "" }}
  {{- if eq $dest.type "otlp" }}
    {{- $exporter = ternary "otlp" "otlphttp" ( eq $dest.protocol "grpc" ) }}
  {{- else if eq $dest.type "datadog" }}
    {{- $exporter = "datadog" }}
  {{- else if eq $dest.type "s3" }}
    {{- $exporter = "awss3" }}
  {{- end }}
  {{- $exporterInput := printf "otelcol.exporter.%s.%s.input" $exporter $label }}
  {{- $metricsTarget := $exporterInput }}
  {{- if eq $dest.type "googleCloud" }}
    {{- $metricsTarget = printf "otelcol.processor.filter.gcp_%s.input" $dest.name }}
  {{- end }}
  {{- $authHandler := get ( dict "basicAuth" "basic" "bearer" "bearer" "headers" "headers" "sigv4" "sigv4" ) $dest.authType }}
  {{- if eq $dest.type "otlp" }}
    {{- if eq $dest.authType "basicAuth" }}

otelcol.auth.basic {{ $label | quote }} {
    client_auth {
        username = sys.env({{ $dest.basicAuth.usernameEnv | quote }})
        password = sys.env({{ $dest.basicAuth.passwordEnv | quote }})
    }
}
    {{- else if eq $dest.authType "bearer" }}

otelcol.auth.bearer {{ $label | quote }} {
    token = sys.env({{ $dest.bearer.tokenEnv | quote }})
}
    {{- else if eq $dest.authType "headers" }}

otelcol.auth.headers {{ $label | quote }} {
      {{- range $dest.headers }}
    header {
        key   = {{ .key | quote }}
        value = {{ if .valueEnv }}sys.env({{ .valueEnv | quote }}){{ else }}{{ .value | quote }}{{ end }}
    }
      {{- end }}
}
    {{- else if eq $dest.authType "sigv4" }}

otelcol.auth.sigv4 {{ $label | quote }} {
      {{- with $dest.sigv4.region }}
    region = {{ . | quote }}
      {{- end }}
      {{- with $dest.sigv4.service }}
    service = {{ . | quote }}
      {{- end }}
      {{- with $dest.sigv4.roleArn }}

    assume_role {
        arn = {{ . | quote }}
    }
      {{- end }}
}
    {{- end }}
  {{- end }}
  {{- if $dest.logs.enabled }}

otelcol.receiver.loki {{ $label | quote }} {
    output {
        logs = [otelcol.processor.batch.{{ $label }}.input]
    }
}

otelcol.processor.batch {{ $label | quote }} {
    {{- if eq $dest.type "s3" }}
    {{- /* One S3 object per batch, so the batch is the object size. The
           default 200ms would write thousands of small objects an hour. */}}
    timeout             = {{ $dest.batch.timeout | default "1m" | quote }}
    send_batch_size     = {{ $dest.batch.size | default 8192 | int }}
    send_batch_max_size = {{ mul ( $dest.batch.size | default 8192 | int ) 2 }}

    {{- end }}
    output {
        logs = [{{ $exporterInput }}]
    }
}
  {{- end }}
  {{- if $dest.metrics.enabled }}
    {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
          "forwardTo" ( list $metricsTarget )
          "processorName" $label
          "unfilteredMetricsEnv" $dest.metrics.unfilteredMetricsEnv
          "root" $root ) | nindent 0 }}
  {{- end }}
  {{- if eq $dest.type "googleCloud" }}

    {{- include "mzmon.alloyGateway.googleCloud.chain" ( dict
          "label" ( printf "gcp_%s" $dest.name )
          "endpointExpr" ( printf "sys.env(%q)" $dest.urlEnv )
          "project" $dest.project
          "location" $dest.location
          "compression" $dest.compression
          "queue" $dest.queue ) }}
  {{- else if eq $dest.type "s3" }}

otelcol.exporter.awss3 {{ $label | quote }} {
    s3_uploader {
        region              = {{ $dest.region | quote }}
        s3_bucket           = {{ $dest.bucket | quote }}
        s3_prefix           = {{ $dest.prefix | quote }}
    {{- with $dest.partitionFormat }}
        s3_partition_format = {{ . | quote }}
    {{- end }}
        s3_partition_timezone = {{ $dest.partitionTimezone | quote }}
    {{- with $dest.filePrefix }}
        file_prefix         = {{ . | quote }}
    {{- end }}
    {{- with $dest.storageClass }}
        storage_class       = {{ . | quote }}
    {{- end }}
        compression         = {{ $dest.compression | quote }}
    {{- with $dest.roleArn }}
        role_arn            = {{ . | quote }}
    {{- end }}
    {{- if $dest.url }}
        endpoint            = sys.env({{ $dest.urlEnv | quote }})
      {{- if hasPrefix "http://" ( tpl $dest.url $root ) }}
        disable_ssl         = true
      {{- end }}
    {{- end }}
    {{- if $dest.forcePathStyle }}
        s3_force_path_style = true
    {{- end }}
    }

    marshaler {
        type = {{ $dest.marshaler | quote }}
    }
    {{- include "mzmon.alloyGateway.otel.sendingQueue" $dest.queue }}
}
  {{- else if eq $dest.type "datadog" }}

otelcol.exporter.datadog {{ $label | quote }} {
    api {
        api_key = sys.env({{ $dest.apiKey.keyEnv | quote }})
        fail_on_invalid_key = true
        site = {{ $dest.site | quote }}
    }
    {{- if and $dest.metrics.enabled $dest.metrics.url }}

    metrics {
        endpoint = sys.env({{ $dest.metrics.urlEnv | quote }})
    }
    {{- end }}
    {{- if and $dest.logs.enabled $dest.logs.url }}

    logs {
        endpoint = sys.env({{ $dest.logs.urlEnv | quote }})
    }
    {{- end }}

    host_metadata {
        enabled = {{ $dest.hostMetadata | toString }}
    }
    {{- include "mzmon.alloyGateway.otel.sendingQueue" $dest.queue }}
}
  {{- else }}

otelcol.exporter.{{ $exporter }} {{ $label | quote }} {
  {{- if and ( eq $exporter "otlphttp" ) $dest.logs.enabled $dest.logs.url }}
    logs_endpoint = sys.env({{ $dest.logs.urlEnv | quote }})
  {{- end }}
  {{- if and ( eq $exporter "otlphttp" ) $dest.metrics.enabled $dest.metrics.url }}
    metrics_endpoint = sys.env({{ $dest.metrics.urlEnv | quote }})
  {{- end }}
    client {
        endpoint = sys.env({{ $dest.urlEnv | quote }})
        compression = {{ $dest.compression | quote }}
  {{- with $authHandler }}
        auth = otelcol.auth.{{ . }}.{{ $label }}.handler
  {{- end }}

        tls {
  {{- if not $dest.tls.enabled }}
            insecure = true
  {{- else }}
    {{- if not $dest.tls.verify }}
            insecure_skip_verify = true
    {{- end }}
    {{- with $dest.tls.caFile }}
            ca_file = {{ . | quote }}
    {{- end }}
    {{- with $dest.tls.certFile }}
            cert_file = {{ . | quote }}
    {{- end }}
    {{- with $dest.tls.keyFile }}
            key_file = {{ . | quote }}
    {{- end }}
    {{- with $dest.tls.serverName }}
            server_name = {{ . | quote }}
    {{- end }}
    {{- with $dest.tls.minVersion }}
            min_version = {{ get ( dict "TLS10" "1.0" "TLS11" "1.1" "TLS12" "1.2" "TLS13" "1.3" ) ( . | toString | trimPrefix "Version" ) | default . | quote }}
    {{- end }}
    {{- with $dest.tls.reloadInterval }}
            reload_interval = {{ . | quote }}
    {{- end }}
  {{- end }}
        }
    }
  {{- include "mzmon.alloyGateway.otel.sendingQueue" $dest.queue }}
}
  {{- end }}
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
Every Prometheus remote-write destination, resolved, in a stable order.

The `prometheus` entries of `pipeline.destinations` (with the deprecated
`prometheusRemoteWrite` map folded in by `mzmon.destinations.raw`), in the
shape the remote-write render and the Thanos-hop validators read. Returns
enabled and disabled alike, each carrying its own `name`, sorted by name.

`enabled` is the destination's `active`: enabled, and carrying metrics.

Usage:
  {{- range include "mzmon.alloyGateway.promDests" $ | fromYamlArray }}
    {{- if .enabled }}
*/}}
{{- define "mzmon.alloyGateway.promDests" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
    {{- if eq $dest.type "prometheus" }}
      {{- $out = append $out ( dict
            "name" $dest.name
            "enabled" $dest.active
            "url" $dest.url
            "urlEnv" $dest.urlEnv
            "minMetricImportance" $dest.metrics.minMetricImportance
            "unfilteredMetricsEnv" $dest.metrics.unfilteredMetricsEnv
            "externalLabels" ( $dest.externalLabels | default dict )
            "authType" $dest.authType
            "basicAuth" $dest.basicAuth
            "bearer" $dest.bearer
            "sigv4" $dest.sigv4
            "tls" $dest.tls ) }}
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
        {{- printf "Unsupported authType for pipeline.destinations.%s: %s" $dest.name $dest.authType | fail }}
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
Render the alloy-gateway metric egress filter: one destination's tier allowlist.

Use otelcol.processor.transform.$processorName.input as the fanout chained input.

The allowlist (`metric-tiers.yaml`, through `unfilteredMetricsEnv`) names
Prometheus series, as the registry's PromQL does: `foo_total`, `foo_bucket`,
`foo_count`. The gateway's scrapes honor metadata, so a histogram reaches here
as ONE metric named for its family (`foo`) carrying its buckets, count and sum,
and a summary as one metric whose quantile series carry the family name.
Matching metric.name alone would match no histogram at all, and every one
would silently drop out of the destination.

So the match is per Prometheus series, and it takes two components that share
`$processorName`:

  * the transform pulls a histogram's count and sum out as metrics of their own
    when they are allowed and its buckets are not (or a summary's, when its
    quantiles are not). A family split across tiers —
    `loki_request_duration_seconds_count` at `essential`, its `_bucket` at
    `extended` — sends only the count at `essential`, as it did before types.
  * the filter then keeps a histogram only if its buckets are allowed, and
    anything else only if its name is.

`inputMetricDeny` in gateway-metrics.yaml is the same idea, inverted.

Args:
  forwardTo: list of destinations to forward metrics to
  processorName: label of the transform and filter pair
  unfilteredMetricsEnv: environment variable containing unfiltered metrics

Usage:
  {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
    "forwardTo" $otelDestValues.otlpExporter.handlers
    "processorName" "otlpMetricEgressFilter"
    "unfilteredMetricsEnv" $otelDestValues.otlpExporter.unfilteredMetricsEnv
    "root" $
  ) }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.egressFilter" }}
  {{- $forwardTo := .forwardTo | required "at least one destination is required" }}
  {{- $unfilteredMetricsEnv := .unfilteredMetricsEnv | required "unfilteredMetricsEnv is required" }}
  {{- $processorName := .processorName | required "processorName is required" }}
  {{- $root := .root | required "root context is required" -}}
  {{- /* An OTTL regex literal, assembled at config load: the allowlist, or
         everything when the variable is unset. */}}
  {{- $re := printf `\"^(?:" + coalesce(sys.env(%s), ".*") + ")$\"` ( $unfilteredMetricsEnv | quote ) }}
  {{- $bucket := `Concat([metric.name, \"_bucket\"], \"\")` }}
  {{- $count := `Concat([metric.name, \"_count\"], \"\")` }}
  {{- $sum := `Concat([metric.name, \"_sum\"], \"\")` }}
  {{- /* A histogram whose buckets are not allowed, or a summary whose
         quantiles are not. */}}
  {{- $partial := printf `(metric.type == METRIC_DATA_TYPE_HISTOGRAM and not IsMatch(%s, %s)) or (metric.type == METRIC_DATA_TYPE_SUMMARY and not IsMatch(metric.name, %s))` $bucket $re $re }}

otelcol.processor.transform "{{ $processorName }}" {
    error_mode = "ignore"

    metric_statements {
        context = "metric"
        statements = [
            "extract_count_metric(true) where ({{ $partial }}) and IsMatch({{ $count }}, {{ $re }})",
            "extract_sum_metric(true) where ({{ $partial }}) and IsMatch({{ $sum }}, {{ $re }})",
        ]
    }

    output {
        metrics = [otelcol.processor.filter.{{ $processorName }}.input]
    }
}

otelcol.processor.filter "{{ $processorName }}" {
    metric_conditions {
        context = "metric"
        conditions = [
            "metric.type == METRIC_DATA_TYPE_HISTOGRAM and not IsMatch({{ $bucket }}, {{ $re }})",
            "metric.type != METRIC_DATA_TYPE_HISTOGRAM and not IsMatch(metric.name, {{ $re }})",
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
Render the Google Cloud metrics export chain: OTLP to the Telemetry API, which
writes Google Cloud Managed Service for Prometheus (`prometheus.googleapis.com/`)
metric types. The default `googleCloudExporter.config`.

Follows Google's reference collector configuration for Prometheus data
(https://docs.cloud.google.com/stackdriver/docs/otlp-metrics/deploy-collector),
in Alloy's components:

  filter              staleness markers, which Google's exporters drop too
  resourcedetection   the project and `location`, from GKE's metadata server
  transform           `gcp.project_id`, which the Telemetry API requires;
                      untyped series also written as counters; labels that
                      would collide with prometheus_target renamed; `cluster`
                      and `collected_by`; the scope cleared
  groupbyattrs        `namespace` and `cluster` lifted onto the resource
  metric_start_time   the start time a cumulative point needs
  batch               the Telemetry API's 200 points per request
  otlphttp            with otelcol.auth.google (ADC; Workload Identity on GKE)

Every component takes the one `label`, so the chain is one unit and two
chains never collide: the deprecated `googleCloudExporter` renders it as
`googleCloud`, a `googleCloud` destination as `gcp_<name>`.

Args:
  label: the label every component in the chain takes.
  endpointExpr: the exporter's endpoint, as a rendered Alloy expression.
  project, location: overrides for the detected project and location.
  compression: the request compression.
  queue: optional `sending_queue` fields, as `mzmon.destinations.resolve`
    resolves them.

Usage:
  {{- include "mzmon.alloyGateway.googleCloud.chain" ( dict "label" "x" "endpointExpr" "\"https://…\"" … ) }}
*/}}
{{- define "mzmon.alloyGateway.googleCloud.chain" }}
  {{- $label := .label | required "label is required" }}
  {{- $gcm := . }}
  {{- $cluster := `\"" + sys.env("CLUSTER_NAME") + "\"` }}
// Google Cloud Managed Service for Prometheus, over OTLP to the Telemetry API.
//
// Points flagged "no recorded value" carry nothing to store. The bridge makes
// one from each Prometheus staleness marker: a series that disappears ends with
// one, and so does every series of a target that moves to another gateway
// replica. Google's own exporters drop them the same way.
otelcol.processor.filter {{ $label | quote }} {
    metric_conditions {
        context = "datapoint"
        conditions = [
            "flags == FLAG_NO_RECORDED_VALUE",
        ]
    }

    output {
        metrics = [otelcol.processor.resourcedetection.{{ $label }}.input]
    }
}

// The project (cloud.account.id) and `location` (cloud.region or
// cloud.availability_zone), from GKE's metadata server. Off GKE this finds
// nothing, and `project` and `location` supply them. The host attributes
// describe the gateway's own node, not the series, and are left off.
otelcol.processor.resourcedetection {{ $label | quote }} {
    detectors = ["gcp"]
    override  = false

    gcp {
        resource_attributes {
            host.id {
                enabled = false
            }
            host.name {
                enabled = false
            }
            host.type {
                enabled = false
            }
            gcp.gce.instance.hostname {
                enabled = false
            }
            gcp.gce.instance.name {
                enabled = false
            }
        }
    }

    output {
        metrics = [otelcol.processor.transform.{{ $label }}.input]
    }
}

otelcol.processor.transform {{ $label | quote }} {
    error_mode = "ignore"

    // The Telemetry API refuses a resource without `gcp.project_id` ("Resource
    // is missing required attribute"); cloud.account.id alone is not enough.
    // `project` overrides the detected project, and `location` the detected
    // region, which the Telemetry API reads before cloud.region.
    metric_statements {
        context = "resource"
        statements = [
  {{- with $gcm.project }}
            {{ printf "set(resource.attributes[%q], %q)" "gcp.project_id" . | quote }},
  {{- end }}
  {{- with $gcm.location }}
            {{ printf "set(resource.attributes[%q], %q)" "location" . | quote }},
  {{- end }}
            "set(resource.attributes[\"gcp.project_id\"], resource.attributes[\"cloud.account.id\"]) where resource.attributes[\"gcp.project_id\"] == nil",
        ]
    }

    // An untyped series is also written as a cumulative counter, so rate()
    // works on it: `<name>/unknown` and `<name>/unknown:counter`, as Google
    // Cloud Managed Service for Prometheus's own collectors write them.
    metric_statements {
        context = "metric"
        statements = [
            "copy_metric(Concat([metric.name, \"unknowncounter\"], \":\")) where metric.metadata[\"prometheus.type\"] == \"unknown\" and not HasSuffix(metric.name, \":unknowncounter\")",
            "convert_gauge_to_sum(\"cumulative\", true) where HasSuffix(metric.name, \":unknowncounter\")",
            "set(metric.name, Substring(metric.name, 0, Len(metric.name) - Len(\":unknowncounter\"))) where HasSuffix(metric.name, \":unknowncounter\")",
        ]
    }

    // prometheus_target's labels come from the resource. `job` and `instance`
    // are already there (the scrape target's); a series label of either name,
    // or of `location` or `project_id`, keeps its value as `exported_<label>`,
    // as a Prometheus scrape renames a clashing target label. `namespace` and
    // `cluster` are the series' own and go to the resource below, with
    // CLUSTER_NAME standing in for a series that has no `cluster`.
    //
    // `collected_by` sets these series apart from other Prometheus data in the
    // project. There is no metric prefix to do it any more, and GKE's managed
    // kube-state-metrics writes the same metric types under the same `job`.
    // It is one constant value, so it adds no series.
    metric_statements {
        context = "datapoint"
        statements = [
  {{- range $label := list "location" "project_id" "job" "instance" }}
            "set(attributes[\"exported_{{ $label }}\"], attributes[\"{{ $label }}\"]) where attributes[\"{{ $label }}\"] != nil",
            "delete_key(attributes, \"{{ $label }}\")",
  {{- end }}
            "set(attributes[\"cluster\"], {{ $cluster }}) where attributes[\"cluster\"] == nil and {{ $cluster }} != \"\"",
            "set(attributes[\"collected_by\"], \"materialize-monitoring\") where attributes[\"collected_by\"] == nil",
        ]
    }

    // The Telemetry API turns the instrumentation scope into `otel_scope_name`
    // and `otel_scope_version` labels. Every series here has the same scope,
    // the Prometheus receiver's, so they say nothing, and the version would
    // start a new series for everything at each Alloy upgrade. Thanos carries
    // neither.
    metric_statements {
        context = "scope"
        statements = [
            "set(scope.name, \"\")",
            "set(scope.version, \"\")",
        ]
    }

    output {
        metrics = [otelcol.processor.groupbyattrs.{{ $label }}.input]
    }
}

// Lift `namespace` and `cluster` from each series onto its resource, where the
// Telemetry API reads prometheus_target's labels. Left on the series they
// would collide with those labels; renamed, every query filtering on
// `namespace` would stop matching.
otelcol.processor.groupbyattrs {{ $label | quote }} {
    keys = ["namespace", "cluster"]

    output {
        metrics = [otelcol.processor.metric_start_time.{{ $label }}.input]
    }
}

// A cumulative point needs a start time, and a Prometheus scrape has none.
// Each series' first point becomes its baseline and is not sent; later points
// are sent relative to it. Stateful, per gateway replica: clustering keeps a
// target on one replica, and a target that moves restarts from a new baseline.
otelcol.processor.metric_start_time {{ $label | quote }} {
    strategy = "subtract_initial_point"

    output {
        metrics = [otelcol.processor.batch.{{ $label }}.input]
    }
}

// The Telemetry API takes at most 200 points per request.
otelcol.processor.batch {{ $label | quote }} {
    send_batch_size     = 200
    send_batch_max_size = 200
    timeout             = "5s"

    output {
        metrics = [otelcol.exporter.otlphttp.{{ $label }}.input]
    }
}

otelcol.exporter.otlphttp {{ $label | quote }} {
    client {
        endpoint    = {{ $gcm.endpointExpr }}
        compression = {{ $gcm.compression | quote }}
        auth        = otelcol.auth.google.{{ $label }}.handler
    }
  {{- include "mzmon.alloyGateway.otel.sendingQueue" ( $gcm.queue | default dict ) }}
}

otelcol.auth.google {{ $label | quote }} {
  {{- with $gcm.project }}
    project = {{ . | quote }}
  {{- end }}
}
{{- end }}


{{/*
The deprecated `googleCloudExporter`'s chain, from its own values. The default
`googleCloudExporter.config` calls this.

Usage:
  {{ include "mzmon.alloyGateway.otelDest.googleCloud" $ }}
*/}}
{{- define "mzmon.alloyGateway.otelDest.googleCloud" }}
  {{- $gcm := $.Values.pipeline.metrics.gateway.destination.otel.googleCloudExporter }}
  {{- include "mzmon.alloyGateway.googleCloud.chain" ( dict
        "label" "googleCloud"
        "endpointExpr" ( $gcm.endpoint | required "pipeline.metrics.gateway.destination.otel.googleCloudExporter.endpoint must be set" | quote )
        "project" $gcm.project
        "location" $gcm.location
        "compression" $gcm.compression ) }}
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
         never to the metrics-only Google Cloud chain. */}}
  {{- $logsForwardTo := list }}

  {{- if $otelDestValues.otlpExporter.enabled }}
    {{- $blocks = append $blocks ( tpl $otelDestValues.otlpExporter.config $ ) }}
    {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
      "forwardTo" $otelDestValues.otlpExporter.handlers
      "processorName" "otlpMetricEgressFilter"
      "unfilteredMetricsEnv" $otelDestValues.otlpExporter.unfilteredMetricsEnv
      "root" $
    ) | nindent 0 }}
    {{- $forwardTo = append $forwardTo "otelcol.processor.transform.otlpMetricEgressFilter.input" }}
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
    {{- $forwardTo = append $forwardTo "otelcol.processor.transform.googleCloudMetricEgressFilter.input" -}}
  {{- end }}

  {{- if $otelDestValues.datadogExporter.enabled }}
    {{- $blocks = append $blocks ( tpl $otelDestValues.datadogExporter.config $ ) }}
    {{- include "mzmon.alloyGateway.otelDest.egressFilter" ( dict
      "forwardTo" $otelDestValues.datadogExporter.handlers
      "processorName" "datadogMetricEgressFilter"
      "unfilteredMetricsEnv" $otelDestValues.datadogExporter.unfilteredMetricsEnv
      "root" $
    ) | nindent 0 }}
    {{- $forwardTo = append $forwardTo "otelcol.processor.transform.datadogMetricEgressFilter.input" }}
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
      {{- fail "pipeline.logging.gateway.destination.otel is enabled, but no logs-capable otel exporter (otlpExporter or datadogExporter) is enabled to receive them (the Google Cloud exporter is metrics-only)." }}
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

  {{- /* Provider-pulled families have no registry query to take a tier from,
         so they join at the tier their values assign them. */}}
  {{- $metricPatterns = concat $metricPatterns ( include "mzmon.alloyGateway.provider.metricPatterns" ( dict
        "context" $context
        "minMetricImportance" .minMetricImportance ) | fromYamlArray ) }}

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

  {{- $res := include "mzmon.alloy.validate.provider" $ | fromYaml }}
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
    {{- $res := include "mzmon.destinations.validate" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
      {{- $res := include "mzmon.alloy.validate.destAuth" ( dict
            "context" $
            "role" "alloy-gateway"
            "path" ( printf "pipeline.destinations.%s" $dest.name )
            "dest" $dest
            "enabled" $dest.active ) | fromYaml }}
      {{- $errors = concat $errors $res.errors | default list }}
      {{- $warnings = concat $warnings $res.warnings | default list }}
    {{- end }}

    {{- $res := include "mzmon.alloy.validate.promDests" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- if ( include "mzmon.alloyGateway.otelDest.authEnabled" $ ) }}
      {{- $res := include "mzmon.alloy.validate.otelDestAuth" $ | fromYaml }}
      {{- $errors = concat $errors $res.errors | default list }}
      {{- $warnings = concat $warnings $res.warnings | default list }}
    {{- end }}

    {{- $res := include "mzmon.alloy.validate.gatewayMetadata" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate what the gateway's typed metrics depend on.

  1. **The stability level.** Every gateway scrape sets `honor_metadata`, which
     Alloy refuses to build below `experimental`. `alloy validate` does not check
     a component's arguments against the level, so the pre-validate job passes
     and the gateway crashloops instead.

  2. **The `otelcol.exporter.googlecloud` settings.** The Google Cloud exporter
     writes OTLP to the Telemetry API now, which has no metric prefix and none of
     the old exporter's knobs. A value still setting one does nothing, which is
     worth saying rather than leaving it to look like it does.

Usage:
  {{- $res := include "mzmon.alloy.validate.gatewayMetadata" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.gatewayMetadata" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $level := dig "alloy" "stabilityLevel" "" ( index $.Values "alloy-gateway" ) }}
  {{- if ne $level "experimental" }}
    {{- $errors = append $errors ( printf "alloy-gateway.alloy.stabilityLevel is %q, but the gateway's scrapes set honor_metadata, which Alloy only builds at \"experimental\". The gateway would fail to start. Leave it at \"experimental\"." $level ) }}
  {{- end }}

  {{- $gcm := $.Values.pipeline.metrics.gateway.destination.otel.googleCloudExporter }}
  {{- range $key := list "prefix" "instrumentation_library_labels" "skip_create_descriptor" "service_resource_labels" }}
    {{- if hasKey $gcm $key }}
      {{- $warnings = append $warnings ( printf "pipeline.metrics.gateway.destination.otel.googleCloudExporter.%s no longer applies and is ignored. The exporter writes OTLP to the Telemetry API, whose metrics are always prometheus.googleapis.com/<name>/<kind>." $key ) }}
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
  {{- if and $gatewayOn ( not ( include "mzmon.loki.enabled" $ ) ) }}
    {{- range $dest := ( include "mzmon.destinations.bundledLoki" $ | fromYamlArray ) }}
      {{- $errors = append $errors ( printf "pipeline.destinations.%s.url points at the bundled Loki (%s) but Loki is not enabled. Logs would be written to a Service that does not exist. Set pipeline.destinations.%s.enabled: false, or enable Loki." $dest.name $dest.resolvedUrl $dest.name ) }}
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
    {{- $logDests := include "mzmon.destinations.logs" $ | fromYamlArray }}
    {{- if and ( not $logDests ) ( not $.Values.pipeline.logging.gateway.destination.otel.enabled ) }}
      {{- $warnings = append $warnings "Every gateway log destination is disabled (no pipeline.destinations entry carries logs, and pipeline.logging.gateway.destination.otel.enabled is false). Logs are processed and then discarded." }}
    {{- end }}

    {{- $metricDests := include "mzmon.destinations.metrics" $ | fromYamlArray }}
    {{- if and ( not $metricDests ) ( not $.Values.pipeline.metrics.gateway.destination.otel.enabled ) }}
      {{- $warnings = append $warnings "Every gateway metric destination is disabled (no pipeline.destinations entry carries metrics, and pipeline.metrics.gateway.destination.otel.enabled is false). Metrics are scraped and then discarded." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Refuse the single-destination keys of the deprecated remote-write map.

`prometheusRemoteWrite` used to be one destination. Helm deep-merges, so an old
`prometheusRemoteWrite.url` lands beside the named entries rather than
replacing anything: the key applies to nothing, the default Thanos endpoint
keeps receiving, and nothing errors. Refused by name, with the path to move it
to. The entries themselves are destinations, folded onto `pipeline.destinations`
and checked there by `mzmon.destinations.validate`.

Usage:
  {{- $res := include "mzmon.alloy.validate.promDests" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.promDests" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $path := "pipeline.metrics.gateway.destination.prometheusRemoteWrite" }}
  {{- $dests := dig "metrics" "gateway" "destination" "prometheusRemoteWrite" dict ( $.Values.pipeline | default dict ) }}
  {{- if not ( kindIs "map" $dests ) }}
    {{- $dests = dict }}
  {{- end }}
  {{- $legacy := include "mzmon.alloyGateway.promDest.legacyKeys" $ | fromYamlArray }}

  {{- $legacyFound := list }}
  {{- range $name := ( keys $dests | sortAlpha ) }}
    {{- if has $name $legacy }}
      {{- $legacyFound = append $legacyFound $name }}
    {{- end }}
  {{- end }}

  {{- if $legacyFound }}
    {{- /* One error rather than one per key: they are a single mistake, and
           the migration is the same move for all of them. Each key is named
           with its target, because two of them move under `metrics`, and a
           key moved to the same name there would apply to nothing. */}}
    {{- $moves := list }}
    {{- range $key := $legacyFound }}
      {{- if has $key ( list "minMetricImportance" "unfilteredMetricsEnv" ) }}
        {{- $moves = append $moves ( printf "%s to pipeline.destinations.thanos.metrics.%s" $key $key ) }}
      {{- else if eq $key "oauth2" }}
        {{- $moves = append $moves "oauth2 has no replacement: it never rendered" }}
      {{- else }}
        {{- $moves = append $moves ( printf "%s to pipeline.destinations.thanos.%s" $key $key ) }}
      {{- end }}
    {{- end }}
    {{- $errors = append $errors ( printf "%s still carries the single-destination key(s) %s, from before destinations were a map. They apply to nothing, and the install would keep writing to the default endpoint with no error. Move them onto the bundled Thanos Receive's entry: %s." $path ( join ", " $legacyFound ) ( join "; " $moves ) ) }}
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
    {{- else if eq $authType "apiKey" }}
      {{- $needed = list
            ( dict "field" "apiKey.key" "value" $dest.apiKey.key "env" $dest.apiKey.keyEnv "secret" true ) }}
    {{- else if eq $authType "headers" }}
      {{- /* A literal header renders into the pipeline config itself, so only
             the ones read from the environment need a source. */}}
      {{- range $i, $header := ( $dest.headers | default list ) }}
        {{- if and ( kindIs "map" $header ) $header.valueEnv }}
          {{- $needed = append $needed ( dict "field" ( printf "headers[%d].value" $i ) "value" "" "env" $header.valueEnv ) }}
        {{- end }}
      {{- end }}
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

{{- /*
Get the alloy-gateway remote-write endpoint.

This is `prometheus.receive_http`'s listener — the same port the agent pushes to
and the one every metric in this stack already arrives on. A ruler writing here
rather than straight to Thanos is what puts rule results in front of the
gateway's destination fan-out instead of only in Thanos.

Only usable from umbrella templates: it reads `alloy-gateway` out of the
umbrella's values, which a subchart's own `tpl` context cannot see. Values that
are consumed inside a subchart (`loki.loki.rulerConfig`) spell the URL out
instead.

Usage:
  {{- include "mzmon.alloyGateway.remoteWriteUrl" $ }}
*/}}
{{- define "mzmon.alloyGateway.remoteWriteUrl" }}
  {{- $tls := dig "metrics" "gateway" "server" "tls" dict ( $.Values.pipeline | default dict ) }}
  {{- $scheme := ternary "https" "http" ( $tls.enabled | default false ) }}
  {{- printf "%s://%s" $scheme ( include "mzmon.alloyGateway.remoteWriteAddress" $ ) }}
{{- end }}

{{- /*
The alloy-gateway remote-write endpoint without its scheme, for a caller that
decides the scheme itself.

Usage:
  {{- include "mzmon.alloyGateway.remoteWriteAddress" $ }}
*/}}
{{- define "mzmon.alloyGateway.remoteWriteAddress" }}
  {{- printf "alloy-gateway.%s.svc.%s:9090/api/v1/metrics/write" ( include "mzmon.alloyGateway.namespace" $ ) ( include "mzmon.clusterDomain" $ ) }}
{{- end }}
