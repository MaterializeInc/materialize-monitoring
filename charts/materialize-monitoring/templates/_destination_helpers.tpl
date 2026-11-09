{{- /*
Helpers for `pipeline.destinations`: the gateway's named destinations, shared by
every signal.

One entry is one wire: an endpoint, its credentials, and its queue. Each signal
the entry carries is switched on and shaped in its own block (`logs`,
`metrics`), so a backend that takes only logs needs no metrics block to switch
off, and a backend that takes both — the BYOC control plane — has one
credential and one entry.

Two types exist: `loki` (logs, through `loki.write`) and `otlp` (logs and
metrics, through `otelcol.exporter.otlphttp` or `otelcol.exporter.otlp`). The
older destinations still live under `pipeline.logging.gateway.destination.otel`
and `pipeline.metrics.gateway.destination.*`.

**Component labels.** A `loki` destination's `loki.write` is labeled with the
bare name. An `otlp` destination's components are labeled `dest_<name>`: the
gateway already runs dozens of `otelcol.*` components (the metric pipeline, the
Google Cloud chain, the egress filters), and the prefix keeps a destination name
from ever colliding with one of them.
*/}}


{{- /*
The destination types this chart can render, and the signals each carries.

Usage:
  {{- $types := include "mzmon.destinations.types" $ | fromYaml }}
  {{- $signals := index $types "loki" }}
*/}}
{{- define "mzmon.destinations.types" }}
  {{- dict
      "loki" ( list "logs" )
      "otlp" ( list "logs" "metrics" )
      "googleCloud" ( list "metrics" )
      "datadog" ( list "logs" "metrics" )
      "prometheus" ( list "metrics" )
      "s3" ( list "logs" )
    | toYaml }}
{{- end }}


{{- /*
Per-type defaults the resolver and the validator share.

  url                  the endpoint when the destination names none
  urlRequired          whether the destination is unusable without a URL;
                       Datadog derives its endpoints from `site`
  minMetricImportance  the metric tier floor. The metered backends default to
                       `recommended`, as the exporters they replace did
  authTypes            the `authType` values the type accepts, default first.
                       Google Cloud authenticates with the gateway's ambient
                       Google identity, and Datadog with its API key

Usage:
  {{- $defaults := index ( include "mzmon.destinations.typeDefaults" $ | fromYaml ) "otlp" }}
*/}}
{{- define "mzmon.destinations.typeDefaults" }}
  {{- dict
      "loki" ( dict "url" "" "urlRequired" true "minMetricImportance" "all"
                    "authTypes" ( list "none" "basicAuth" "bearer" ) )
      "otlp" ( dict "url" "" "urlRequired" true "minMetricImportance" "all"
                    "authTypes" ( list "none" "basicAuth" "bearer" "headers" "sigv4" ) )
      "googleCloud" ( dict "url" "https://telemetry.googleapis.com" "urlRequired" true "minMetricImportance" "recommended"
                    "authTypes" ( list "none" ) )
      "datadog" ( dict "url" "" "urlRequired" false "minMetricImportance" "recommended"
                    "authTypes" ( list "apiKey" ) )
      "prometheus" ( dict "url" "" "urlRequired" true "minMetricImportance" "all"
                    "authTypes" ( list "none" "basicAuth" "bearer" "sigv4" ) )
      "s3" ( dict "url" "" "urlRequired" false "minMetricImportance" "all"
                    "authTypes" ( list "none" ) )
    | toYaml }}
{{- end }}


{{- /*
The `queue` fields each type accepts. Each maps onto a different block —
`loki.write`'s `queue_config`, the OTLP exporter's `sending_queue` — and only
`blockOnOverflow` means the same thing in both.

Usage:
  {{- $fields := index ( include "mzmon.destinations.queueFields" $ | fromYaml ) "otlp" }}
*/}}
{{- define "mzmon.destinations.queueFields" }}
  {{- $otel := list "queueSize" "numConsumers" "blockOnOverflow" }}
  {{- dict
      "loki" ( list "capacity" "minShards" "blockOnOverflow" "drainTimeout" )
      "otlp" $otel
      "googleCloud" $otel
      "datadog" $otel
      "prometheus" list
      "s3" $otel
    | toYaml }}
{{- end }}


{{- /*
Whether a destination of this type blocks the fan-out when its queue is full,
when `queue.blockOnOverflow` is unset. These are the components' own defaults:
`loki.write` waits for room, and the OTLP exporters drop.

Usage:
  {{- $blocks := index ( include "mzmon.destinations.blocksByDefault" $ | fromYaml ) $type }}
*/}}
{{- define "mzmon.destinations.blocksByDefault" }}
  {{- dict "loki" true "otlp" false "googleCloud" false "datadog" false "prometheus" false "s3" false | toYaml }}
{{- end }}


{{- /*
Names a destination may not take.

A destination's name is the label on its Alloy components, and `egress` is the
log fan-out seam (`loki.process "egress"`) every destination hangs off.
`destination` was the label of the single `loki.write` before the map, and is
kept out so nothing reads as the old component.

Usage:
  {{- $reserved := include "mzmon.destinations.reservedNames" $ | fromYamlArray }}
*/}}
{{- define "mzmon.destinations.reservedNames" }}
  {{- list "egress" "destination" | toYaml }}
{{- end }}


{{- /*
Overlay one values map onto another, recursively.

Not `mergeOverwrite`: mergo's `WithOverride` does not override with a zero
value, so `enabled: false` and `tls.verify: false` in the overlay would be
discarded. A `null` in the overlay is how Helm records a cleared key, and it
leaves the base alone.

Args:
  base: the map underneath.
  over: the map whose keys win.

Usage:
  {{- $merged := include "mzmon.destinations.overlay" ( dict "base" $a "over" $b ) | fromYaml }}
*/}}
{{- define "mzmon.destinations.overlay" }}
  {{- $out := deepCopy ( .base | default dict ) }}
  {{- range $key, $value := ( .over | default dict ) }}
    {{- $under := index $out $key }}
    {{- if and ( kindIs "map" $value ) ( kindIs "map" $under ) }}
      {{- $_ := set $out $key ( include "mzmon.destinations.overlay" ( dict "base" $under "over" $value ) | fromYaml ) }}
    {{- else if not ( kindIs "invalid" $value ) }}
      {{- $_ := set $out $key $value }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
The keys an install still sets under the deprecated
`pipeline.logging.gateway.destination.loki`.

The chart ships that key empty, so anything under it came from an override.
`mzmon.destinations.raw` folds it onto the `loki` destination, and
`mzmon.destinations.validate` warns about it.

Usage:
  {{- $legacy := include "mzmon.destinations.legacyLoki" $ | fromYaml }}
*/}}
{{- define "mzmon.destinations.legacyLoki" }}
  {{- $legacy := dig "logging" "gateway" "destination" "loki" dict ( $.Values.pipeline | default dict ) }}
  {{- if not ( kindIs "map" $legacy ) }}
    {{- $legacy = dict }}
  {{- end }}
  {{- $legacy | toYaml }}
{{- end }}


{{- /*
The deprecated `pipeline.metrics.gateway.destination.prometheusRemoteWrite`
entries, translated into `pipeline.destinations` entries of type `prometheus`.

The chart ships that map empty, so every entry in it is an override. Each one
keeps working:

  * `minMetricImportance` and `unfilteredMetricsEnv` move under `metrics`; every
    other field has the same name and meaning on the map. `oauth2` is dropped,
    as it never rendered.
  * Every entry keeps the credential variable names the remote-write map
    derived (`GATEWAY_PROMETHEUS_DEST_<NAME>_PASSWORD`, `…_BEARER_TOKEN`, the
    TLS ones), unless it or its counterpart names its own, so a Secret written
    for it still reaches it. That holds for a retune of `thanos` too.
  * An entry with no counterpart in `pipeline.destinations` also keeps its URL
    and allowlist names (`GATEWAY_PROM_DEST_<NAME>`,
    `GATEWAY_UNFILTERED_PROM_METRICS_<NAME>`). An entry with a counterpart, such
    as `thanos`, is applied over it and takes the counterpart's: those two are
    written by the chart's own env ConfigMap, so nothing outside reads them.

The single-destination keys from before the map (`url`, `enabled`, …) are not
entries; `mzmon.alloy.validate.promDests` refuses them.

Returns `{entries: {<name>: <entry>}, conflicts: [<name>], invalid: [<name>]}`.
A conflict is a name the map already gives to a destination of another type.
An invalid entry is one that is neither a map nor `null`, such as a bare URL:
it would otherwise vanish without a word.

Usage:
  {{- $legacy := include "mzmon.destinations.legacyPrometheus" $ | fromYaml }}
*/}}
{{- define "mzmon.destinations.legacyPrometheus" }}
  {{- $legacy := dig "metrics" "gateway" "destination" "prometheusRemoteWrite" dict ( $.Values.pipeline | default dict ) }}
  {{- if not ( kindIs "map" $legacy ) }}
    {{- $legacy = dict }}
  {{- end }}
  {{- $current := $.Values.pipeline.destinations | default dict }}
  {{- $preMap := include "mzmon.alloyGateway.promDest.legacyKeys" $ | fromYamlArray }}
  {{- $entries := dict }}
  {{- $conflicts := list }}
  {{- $invalid := list }}
  {{- range $name := ( keys $legacy | sortAlpha ) }}
    {{- $raw := index $legacy $name }}
    {{- if and ( not ( has $name $preMap ) ) ( not ( kindIs "map" $raw ) ) ( not ( kindIs "invalid" $raw ) ) }}
      {{- $invalid = append $invalid $name }}
    {{- end }}
    {{- if and ( not ( has $name $preMap ) ) ( kindIs "map" $raw ) }}
      {{- $slug := $name | regexReplaceAll "[^A-Za-z0-9]" "_" | upper }}
      {{- $p := printf "GATEWAY_PROMETHEUS_DEST_%s" $slug }}
      {{- $credentialNames := dict
            "basicAuth" ( dict "usernameEnv" ( printf "%s_USERNAME" $p ) "passwordEnv" ( printf "%s_PASSWORD" $p ) )
            "bearer" ( dict "tokenEnv" ( printf "%s_BEARER_TOKEN" $p ) )
            "tls" ( dict "caEnv" ( printf "%s_TLS_CA" $p ) "certEnv" ( printf "%s_TLS_CERT" $p ) "keyEnv" ( printf "%s_TLS_KEY" $p ) ) }}
      {{- $entry := omit $raw "minMetricImportance" "unfilteredMetricsEnv" "oauth2" }}
      {{- $metrics := dict }}
      {{- range $key := list "minMetricImportance" "unfilteredMetricsEnv" }}
        {{- if not ( kindIs "invalid" ( index $raw $key ) ) }}
          {{- $_ := set $metrics $key ( index $raw $key ) }}
        {{- end }}
      {{- end }}
      {{- $_ := set $entry "metrics" $metrics }}
      {{- $counterpart := index $current $name }}
      {{- if kindIs "map" $counterpart }}
        {{- if ne ( dig "type" "" $counterpart | toString ) "prometheus" }}
          {{- $conflicts = append $conflicts $name }}
        {{- end }}
        {{- /* The counterpart's own settings win over the legacy credential
               names, and the legacy entry wins over both. */}}
        {{- $under := include "mzmon.destinations.overlay" ( dict "base" $credentialNames "over" $counterpart ) | fromYaml }}
        {{- $entry = include "mzmon.destinations.overlay" ( dict "base" $under "over" $entry ) | fromYaml }}
      {{- else }}
        {{- $names := merge ( dict
              "type" "prometheus"
              "urlEnv" ( printf "GATEWAY_PROM_DEST_%s" $slug )
              "metrics" ( dict "unfilteredMetricsEnv" ( printf "GATEWAY_UNFILTERED_PROM_METRICS_%s" $slug ) ) ) $credentialNames }}
        {{- $entry = include "mzmon.destinations.overlay" ( dict "base" $names "over" $entry ) | fromYaml }}
      {{- end }}
      {{- $_ := set $entries $name $entry }}
    {{- end }}
  {{- end }}
  {{- dict "entries" $entries "conflicts" $conflicts "invalid" $invalid | toYaml }}
{{- end }}


{{- /*
The destination map as values describe it, with the deprecated destinations
folded in: the single Loki destination onto the `loki` entry, and the
remote-write map's entries onto entries of type `prometheus`.

Usage:
  {{- $raw := include "mzmon.destinations.raw" $ | fromYaml }}
*/}}
{{- define "mzmon.destinations.raw" }}
  {{- $dests := deepCopy ( $.Values.pipeline.destinations | default dict ) }}
  {{- $legacy := include "mzmon.destinations.legacyLoki" $ | fromYaml }}
  {{- if $legacy }}
    {{- $base := index $dests "loki" }}
    {{- if not ( kindIs "map" $base ) }}
      {{- $base = dict "type" "loki" }}
    {{- end }}
    {{- $_ := set $dests "loki" ( include "mzmon.destinations.overlay" ( dict "base" $base "over" $legacy ) | fromYaml ) }}
  {{- end }}
  {{- $legacyProm := include "mzmon.destinations.legacyPrometheus" $ | fromYaml }}
  {{- range $name, $entry := $legacyProm.entries }}
    {{- if not ( has $name $legacyProm.conflicts ) }}
      {{- $_ := set $dests $name ( include "mzmon.destinations.overlay" ( dict "base" ( index $dests $name | default dict ) "over" $entry ) | fromYaml ) }}
    {{- end }}
  {{- end }}
  {{- $dests | toYaml }}
{{- end }}


{{- /*
Resolve one destination against the per-destination defaults.

The env ConfigMap, the pipeline, and the validators all read a destination
through this helper, so a default written here cannot disagree between them.

`active` is whether anything renders for the destination: it is enabled and
carries at least one signal. A destination that is off, or carries nothing,
needs no URL or credentials.

**A signal is on** when its block is present, or when the type carries only
that signal (so a `loki` destination needs no `logs` block), unless the block
says `enabled: false`. An `otlp` destination carries nothing until it names a
signal.

Booleans whose default is `true` are resolved by hand rather than with `dig`:
`dig` returns empty for an explicit `null`, which is how Helm records a cleared
key, so `enabled: null` would read as `false`. Absent and `null` both mean "use
the default".

Environment variable names derive from the name as `GATEWAY_<NAME>_DEST*`, which
gives the `loki` destination exactly the names the single Loki destination used
(`GATEWAY_LOKI_DEST`, `GATEWAY_LOKI_DEST_PASSWORD`, …). A credential an install
already supplies through `extraEnv` keeps reaching it.

Every derived name is `GATEWAY_<NAME>_DEST` plus a suffix, and no suffix
contains `_DEST`, so two destinations whose names differ by more than case
cannot derive the same variable. Keep new names under that prefix: a name
outside it (`GATEWAY_UNFILTERED_<NAME>_…`) can equal another destination's
derived name. `mzmon.destinations.validate` catches the collisions that are
left, from explicit `*Env` overrides and case-only differences.

Args:
  name: the destination's key in the map, and its Alloy component label.
  dest: the raw values for that destination.

Usage:
  {{- $d := include "mzmon.destinations.resolve" ( dict "name" $name "dest" $raw ) | fromYaml }}
*/}}
{{- define "mzmon.destinations.resolve" }}
  {{- $name := .name | required "name is required" | toString }}
  {{- $raw := .dest }}
  {{- if not ( kindIs "map" $raw ) }}
    {{- $raw = dict }}
  {{- end }}
  {{- $slug := $name | regexReplaceAll "[^A-Za-z0-9]" "_" | upper }}
  {{- $prefix := printf "GATEWAY_%s_DEST" $slug }}
  {{- $type := dig "type" "" $raw | toString }}
  {{- $signals := index ( include "mzmon.destinations.types" . | fromYaml ) $type | default list }}
  {{- $defaults := index ( include "mzmon.destinations.typeDefaults" . | fromYaml ) $type | default ( dict
        "url" "" "urlRequired" true "minMetricImportance" "all" "authTypes" ( list "none" ) ) }}
  {{- $headers := dig "headers" list $raw }}
  {{- if not ( kindIs "slice" $headers ) }}
    {{- $headers = list }}
  {{- end }}

  {{- $tls := dig "tls" dict $raw }}
  {{- if not ( kindIs "map" $tls ) }}
    {{- $tls = dict }}
  {{- end }}
  {{- $queue := dig "queue" dict $raw }}
  {{- if not ( kindIs "map" $queue ) }}
    {{- $queue = dict }}
  {{- end }}
  {{- $signalBlocks := dict }}
  {{- $signalOn := dict }}
  {{- range $signal := list "logs" "metrics" }}
    {{- $block := index $raw $signal }}
    {{- $on := or ( kindIs "map" $block ) ( and ( eq ( len $signals ) 1 ) ( has $signal $signals ) ) }}
    {{- if not ( kindIs "map" $block ) }}
      {{- $block = dict }}
    {{- end }}
    {{- if and ( hasKey $block "enabled" ) ( not ( kindIs "invalid" ( index $block "enabled" ) ) ) }}
      {{- $on = index $block "enabled" }}
    {{- end }}
    {{- $_ := set $signalBlocks $signal $block }}
    {{- $_ := set $signalOn $signal $on }}
  {{- end }}
  {{- $logs := index $signalBlocks "logs" }}
  {{- $metrics := index $signalBlocks "metrics" }}

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
  {{- $logsEnabled := index $signalOn "logs" }}
  {{- $keepUnknown := true }}
  {{- if and ( hasKey $logs "keepUnknown" ) ( not ( kindIs "invalid" ( index $logs "keepUnknown" ) ) ) }}
    {{- $keepUnknown = index $logs "keepUnknown" }}
  {{- end }}
  {{- /* A rate limit is configured when its block is a non-empty map; an
         explicit `null` clears it like an absent one. `rate` is kept as given,
         so the validator can tell a missing rate from a bad one. `burst`
         defaults to the rate rounded up, because Alloy takes it as a positive
         integer and a fractional rate would otherwise fail the load. */}}
  {{- $rateLimitRaw := index $logs "rateLimit" }}
  {{- if not ( kindIs "map" $rateLimitRaw ) }}
    {{- $rateLimitRaw = dict }}
  {{- end }}
  {{- $rlRate := index $rateLimitRaw "rate" }}
  {{- $rlBurst := index $rateLimitRaw "burst" }}
  {{- /* A number, or a numeric string: `--set rate=0.5` arrives as a string. */}}
  {{- $rlRateOk := and ( not ( kindIs "invalid" $rlRate ) ) ( regexMatch "^[0-9]*\\.?[0-9]+$" ( toString $rlRate ) ) ( gt ( $rlRate | float64 ) 0.0 ) }}
  {{- if and ( kindIs "invalid" $rlBurst ) $rlRateOk }}
    {{- $rlBurst = max 1 ( ceil $rlRate | int ) }}
  {{- end }}
  {{- $rateLimit := dict
        "configured" ( gt ( len $rateLimitRaw ) 0 )
        "rate" $rlRate
        "burst" $rlBurst }}
  {{- $metricsEnabled := index $signalOn "metrics" }}

  {{- /* Only the queue settings the destination names are kept, so a
         destination that sets none renders no queue block. Every known field
         is kept whatever the type, so the validator can name a field the type
         does not take. */}}
  {{- $resolvedQueue := dict }}
  {{- range $key := list "capacity" "minShards" "blockOnOverflow" "drainTimeout" "queueSize" "numConsumers" }}
    {{- if and ( hasKey $queue $key ) ( not ( kindIs "invalid" ( index $queue $key ) ) ) }}
      {{- $_ := set $resolvedQueue $key ( index $queue $key ) }}
    {{- end }}
  {{- end }}

  {{- dict
    "name" $name
    "type" $type
    "enabled" $enabled
    "url" ( dig "url" $defaults.url $raw | toString )
    "urlEnv" ( dig "urlEnv" $prefix $raw | toString )
    "urlRequired" $defaults.urlRequired
    "retries" ( dict
      "maxBackoffRetries" ( dig "retries" "maxBackoffRetries" 10 $raw )
      "minBackoffPeriod" ( dig "retries" "minBackoffPeriod" "1s" $raw | toString )
      "maxBackoffPeriod" ( dig "retries" "maxBackoffPeriod" "5m" $raw | toString )
      "retryOnHttp429" ( dig "retries" "retryOnHttp429" true $raw ) )
    "authType" ( dig "authType" ( first $defaults.authTypes ) $raw | toString )
    "basicAuth" ( dict
      "username" ( dig "basicAuth" "username" "" $raw | toString )
      "usernameEnv" ( dig "basicAuth" "usernameEnv" ( printf "%s_USERNAME" $prefix ) $raw | toString )
      "password" ( dig "basicAuth" "password" "" $raw | toString )
      "passwordEnv" ( dig "basicAuth" "passwordEnv" ( printf "%s_PASSWORD" $prefix ) $raw | toString ) )
    "bearer" ( dict
      "token" ( dig "bearer" "token" "" $raw | toString )
      "tokenEnv" ( dig "bearer" "tokenEnv" ( printf "%s_BEARER_TOKEN" $prefix ) $raw | toString ) )
    "headers" $headers
    "sigv4" ( dict
      "region" ( dig "sigv4" "region" "" $raw | toString )
      "service" ( dig "sigv4" "service" "" $raw | toString )
      "roleArn" ( dig "sigv4" "roleArn" "" $raw | toString ) )
    "apiKey" ( dict
      "key" ( dig "apiKey" "key" "" $raw | toString )
      "keyEnv" ( dig "apiKey" "keyEnv" ( printf "%s_API_KEY" $prefix ) $raw | toString ) )
    "site" ( dig "site" "datadoghq.com" $raw | toString )
    "hostMetadata" ( dig "hostMetadata" false $raw )
    "project" ( dig "project" "" $raw | toString )
    "externalLabels" ( dig "externalLabels" dict $raw )
    "bucket" ( dig "bucket" "" $raw | toString )
    "region" ( dig "region" "" $raw | toString )
    "prefix" ( dig "prefix" "logs" $raw | toString )
    "partitionFormat" ( dig "partitionFormat" "" $raw | toString )
    "partitionTimezone" ( dig "partitionTimezone" "UTC" $raw | toString )
    "filePrefix" ( dig "filePrefix" "" $raw | toString )
    "storageClass" ( dig "storageClass" "" $raw | toString )
    "marshaler" ( dig "marshaler" "otlp_json" $raw | toString )
    "forcePathStyle" ( dig "forcePathStyle" false $raw )
    "roleArn" ( dig "roleArn" "" $raw | toString )
    "batch" ( dict
      "timeout" ( dig "batch" "timeout" "" $raw | toString )
      "size" ( dig "batch" "size" 0 $raw ) )
    "location" ( dig "location" "" $raw | toString )
    "tls" ( dict
      "enabled" $tlsEnabled
      "verify" $tlsVerify
      "ca" ( dig "ca" "" $tls | toString )
      "caEnv" ( dig "caEnv" ( printf "%s_TLS_CA" $prefix ) $tls | toString )
      "cert" ( dig "cert" "" $tls | toString )
      "certEnv" ( dig "certEnv" ( printf "%s_TLS_CERT" $prefix ) $tls | toString )
      "key" ( dig "key" "" $tls | toString )
      "keyEnv" ( dig "keyEnv" ( printf "%s_TLS_KEY" $prefix ) $tls | toString )
      "caFile" ( dig "caFile" "" $tls | toString )
      "certFile" ( dig "certFile" "" $tls | toString )
      "keyFile" ( dig "keyFile" "" $tls | toString )
      "serverName" ( dig "serverName" "" $tls | toString )
      "minVersion" ( dig "minVersion" "TLS13" $tls | toString )
      "reloadInterval" ( dig "reloadInterval" "" $tls | toString ) )
    "protocol" ( dig "protocol" "http" $raw | toString )
    "compression" ( dig "compression" "gzip" $raw | toString )
    "queue" $resolvedQueue
    "logs" ( dict
      "enabled" $logsEnabled
      "url" ( dig "url" "" $logs | toString )
      "urlEnv" ( dig "urlEnv" ( printf "%s_LOGS" $prefix ) $logs | toString )
      "selector" ( dig "selector" "" $logs | toString )
      "minLevel" ( dig "minLevel" "" $logs | toString | upper )
      "keepUnknown" $keepUnknown
      "sampleRate" ( dig "sampleRate" 1 $logs )
      "rateLimit" $rateLimit )
    "metrics" ( dict
      "enabled" $metricsEnabled
      "minMetricImportance" ( dig "minMetricImportance" $defaults.minMetricImportance $metrics | toString )
      "url" ( dig "url" "" $metrics | toString )
      "urlEnv" ( dig "urlEnv" ( printf "%s_METRICS" $prefix ) $metrics | toString )
      "unfilteredMetricsEnv" ( dig "unfilteredMetricsEnv" ( printf "%s_METRICS_ALLOWLIST" $prefix ) $metrics | toString ) )
    "label" ( ternary $name ( printf "dest_%s" $name ) ( has $type ( list "loki" "prometheus" ) ) )
    "active" ( and $enabled ( or $logsEnabled $metricsEnabled ) )
    | toYaml }}
{{- end }}


{{- /*
Every destination, resolved, sorted by name.

Sorted because Go map iteration is randomized: an unsorted list would reorder the
rendered components on every render and roll the gateway for nothing, since the
pod template hashes the pipeline ConfigMap.

Usage:
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.destinations.list" }}
  {{- $dests := include "mzmon.destinations.raw" $ | fromYaml }}
  {{- $out := list }}
  {{- range $name := ( keys $dests | sortAlpha ) }}
    {{- $out = append $out ( include "mzmon.destinations.resolve" ( dict
          "name" $name
          "dest" ( index $dests $name ) ) | fromYaml ) }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
The enabled destinations that carry logs.

Usage:
  {{- range $dest := ( include "mzmon.destinations.logs" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.destinations.logs" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
    {{- if and $dest.enabled $dest.logs.enabled }}
      {{- $out = append $out $dest }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
The enabled destinations that carry metrics.

Usage:
  {{- range $dest := ( include "mzmon.destinations.metrics" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.destinations.metrics" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
    {{- if and $dest.enabled $dest.metrics.enabled }}
      {{- $out = append $out $dest }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
The enabled destinations that carry any signal, which is everything the
pipeline and the env ConfigMap render a destination for.

Usage:
  {{- range $dest := ( include "mzmon.destinations.active" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.destinations.active" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
    {{- if $dest.active }}
      {{- $out = append $out $dest }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
The enabled log destinations that write to the bundled in-cluster Loki.

The ruler sync, the TLS pairing, and the client-auth handshake are about this
hop specifically, and applying them to an external Loki would be a false
failure. Matched on the host rather than the destination's name: a `loki` or
`loki-<component>` Service in Loki's namespace. That covers the distributor
(`loki-distributor`), the simple-scalable write target, and the single binary's
bare `loki`, and leaves an external host that happens to start with `loki-`
alone.

Usage:
  {{- range $dest := ( include "mzmon.destinations.bundledLoki" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.destinations.bundledLoki" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.destinations.logs" $ | fromYamlArray ) }}
    {{- if eq $dest.type "loki" }}
      {{- $url := tpl ( $dest.url | toString ) $ }}
      {{- $service := printf `://loki(-[a-z0-9-]+)?\.%s\.svc` ( include "mzmon.loki.namespace" $ | regexQuoteMeta ) }}
      {{- if regexMatch $service $url }}
        {{- $out = append $out ( merge ( dict "resolvedUrl" $url ) $dest ) }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
The levels a log destination's `minLevel` can name, lowest first, and the
levels it drops for each.

`UNKNOWN` is not on the ladder, so no floor drops it; `logs.keepUnknown`
decides it on its own, and defaults to true, so a line the gateway couldn't
level is dropped only when a destination asks. Panics are not `UNKNOWN`: the
gateway levels them `CRITICAL`.

Usage:
  {{- $below := index ( include "mzmon.destinations.levelsBelow" $ | fromYaml ) "WARN" }}
*/}}
{{- define "mzmon.destinations.levelsBelow" }}
  {{- $ladder := list "TRACE" "DEBUG" "INFO" "WARN" "ERROR" "CRITICAL" }}
  {{- $out := dict }}
  {{- range $i, $level := $ladder }}
    {{- $_ := set $out $level ( slice $ladder 0 $i ) }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
Whether a log destination reduces what it receives, which is when the gateway
renders its `loki.process "dest_<name>"`.

Usage:
  {{- if ( include "mzmon.destinations.reducesLogs" $dest ) }}
*/}}
{{- define "mzmon.destinations.reducesLogs" }}
  {{- $logs := .logs }}
  {{- if or $logs.selector $logs.minLevel ( not $logs.keepUnknown ) ( lt ( $logs.sampleRate | float64 ) 1.0 ) $logs.rateLimit.configured }}
    {{- "true" }}
  {{- end }}
{{- end }}


{{- /*
Every variable the env ConfigMap writes for a destination, in the order it
writes them, as a list of `{key, value, owner}`.

The ConfigMap renders from this list and the validator checks it for duplicate
keys, so the two cannot disagree about what is written. `owner` is the values
path of the field that names the key, for the error message.

A credential variable is written only when its value is inline. One supplied
through `envFrom` or `extraEnv` is not written, which is what lets two
destinations share it.

Usage:
  {{- range $entry := ( include "mzmon.destinations.envEntries" $ | fromYamlArray ) }}
*/}}
{{- define "mzmon.destinations.envEntries" }}
  {{- $out := list }}
  {{- range $dest := ( include "mzmon.destinations.active" $ | fromYamlArray ) }}
    {{- $at := printf "pipeline.destinations.%s" $dest.name }}
    {{- if or $dest.urlRequired $dest.url }}
      {{- $out = append $out ( dict
            "key" $dest.urlEnv
            "value" ( include "mzmon.alloy.destUrl" ( dict "url" ( tpl $dest.url $ ) "tls" $dest.tls ) )
            "owner" ( printf "%s.urlEnv" $at ) ) }}
    {{- end }}
    {{- if and $dest.logs.enabled $dest.logs.url }}
      {{- $out = append $out ( dict
            "key" $dest.logs.urlEnv
            "value" ( include "mzmon.alloy.destUrl" ( dict "url" ( tpl $dest.logs.url $ ) "tls" $dest.tls ) )
            "owner" ( printf "%s.logs.urlEnv" $at ) ) }}
    {{- end }}
    {{- if and $dest.metrics.enabled $dest.metrics.url }}
      {{- $out = append $out ( dict
            "key" $dest.metrics.urlEnv
            "value" ( include "mzmon.alloy.destUrl" ( dict "url" ( tpl $dest.metrics.url $ ) "tls" $dest.tls ) )
            "owner" ( printf "%s.metrics.urlEnv" $at ) ) }}
    {{- end }}
    {{- if $dest.metrics.enabled }}
      {{- $out = append $out ( dict
            "key" $dest.metrics.unfilteredMetricsEnv
            "value" ( include "mzmon.alloyGateway.metricFilter" ( dict "context" $ "minMetricImportance" $dest.metrics.minMetricImportance ) )
            "owner" ( printf "%s.metrics.unfilteredMetricsEnv" $at ) ) }}
    {{- end }}
    {{- if and ( eq $dest.authType "basicAuth" ) $dest.basicAuth.username }}
      {{- $out = append $out ( dict "key" $dest.basicAuth.usernameEnv "value" ( tpl $dest.basicAuth.username $ ) "owner" ( printf "%s.basicAuth.usernameEnv" $at ) ) }}
    {{- end }}
    {{- if and ( eq $dest.authType "basicAuth" ) $dest.basicAuth.password }}
      {{- $out = append $out ( dict "key" $dest.basicAuth.passwordEnv "value" ( tpl $dest.basicAuth.password $ ) "owner" ( printf "%s.basicAuth.passwordEnv" $at ) ) }}
    {{- end }}
    {{- if and ( eq $dest.authType "bearer" ) $dest.bearer.token }}
      {{- $out = append $out ( dict "key" $dest.bearer.tokenEnv "value" ( tpl $dest.bearer.token $ ) "owner" ( printf "%s.bearer.tokenEnv" $at ) ) }}
    {{- end }}
    {{- if and ( eq $dest.authType "apiKey" ) $dest.apiKey.key }}
      {{- $out = append $out ( dict "key" $dest.apiKey.keyEnv "value" ( tpl $dest.apiKey.key $ ) "owner" ( printf "%s.apiKey.keyEnv" $at ) ) }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}


{{- /*
Validate the destination map.

Catches the mistakes that otherwise present as a working install:

  1. **A name Alloy cannot use as a component label**, or one already taken.
     Alloy refuses the whole config at load, with a line number a long way from
     the values key that caused it.
  2. **An unknown type**, a signal switched on for a type that cannot carry
     it, a queue field the type does not take, or an `otlp` setting that does
     not apply: an unknown protocol, a per-signal URL over gRPC, inline TLS
     material. An enabled destination that carries no signal is a warning.
  3. **An active destination with no URL.** `sys.env` of an unset variable is
     empty, and `loki.write` accepts an empty endpoint at load, so this would
     surface only as failed writes.
  4. **Two fields writing one key into the env ConfigMap**, across every key
     `mzmon.destinations.envEntries` lists and the remote-write destinations'
     URL and allowlist variables. A ConfigMap keeps one value per key, so one
     writer would read the other's endpoint or allowlist. This comes from an
     explicit `*Env` override or names that differ only in case. A credential
     supplied from outside the ConfigMap is not written, so sharing one is
     allowed.
  5. **More than one log destination that blocks when its queue fills.** The
     fan-out hands each entry to every destination in turn, so a blocking
     destination that stalls holds back the others. One may block — the
     customer's own copy should — and the rest have to shed. Each type's
     default comes from `mzmon.destinations.blocksByDefault`.

It also warns, once each, about the deprecated destinations still in use:
values set under `pipeline.logging.gateway.destination.loki`, and the
`otel` exporters under `pipeline.metrics.gateway.destination.otel`. The `otel`
block keeps rendering as it did; it is not folded onto the map, because its
raw `config` and `handlers` overrides and its shared `auth` have no faithful
equivalent there.

Usage:
  {{- $res := include "mzmon.destinations.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.destinations.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $path := "pipeline.destinations" }}
  {{- $types := include "mzmon.destinations.types" $ | fromYaml }}
  {{- $queueFields := include "mzmon.destinations.queueFields" $ | fromYaml }}
  {{- $blocksByDefault := include "mzmon.destinations.blocksByDefault" $ | fromYaml }}
  {{- $reserved := include "mzmon.destinations.reservedNames" $ | fromYamlArray }}

  {{- $otel := dig "metrics" "gateway" "destination" "otel" dict ( $.Values.pipeline | default dict ) }}
  {{- $otelLogs := dig "logging" "gateway" "destination" "otel" "enabled" false ( $.Values.pipeline | default dict ) }}
  {{- if or ( dig "enabled" false $otel ) $otelLogs }}
    {{- $moves := list }}
    {{- range $exporter := list ( list "otlpExporter" "otlp" ) ( list "googleCloudExporter" "googleCloud" ) ( list "datadogExporter" "datadog" ) }}
      {{- if dig ( index $exporter 0 ) "enabled" false $otel }}
        {{- $moves = append $moves ( printf "%s becomes an entry of type %s" ( index $exporter 0 ) ( index $exporter 1 ) ) }}
      {{- end }}
    {{- end }}
    {{- $warnings = append $warnings ( printf "pipeline.metrics.gateway.destination.otel is deprecated, and still renders as before. Replace each exporter with an entry in %s, which gives it its own credentials, metric tier, queue, and log reduction: %s. Turn a signal on with a metrics or logs block on the entry, in place of otel.enabled and pipeline.logging.gateway.destination.otel.enabled, and set the entry's authType in place of otel.auth." $path ( $moves | default ( list "no exporter is enabled" ) | join "; " ) ) }}
  {{- end }}

  {{- $legacy := include "mzmon.destinations.legacyLoki" $ | fromYaml }}
  {{- if $legacy }}
    {{- $warnings = append $warnings ( printf "pipeline.logging.gateway.destination.loki is deprecated, and its keys (%s) are applied to %s.loki. Move them there: pipeline.logging.gateway.destination.loki.<key> becomes %s.loki.<key>." ( keys $legacy | sortAlpha | join ", " ) $path $path ) }}
  {{- end }}

  {{- $legacyProm := include "mzmon.destinations.legacyPrometheus" $ | fromYaml }}
  {{- if $legacyProm.entries }}
    {{- $warnings = append $warnings ( printf "pipeline.metrics.gateway.destination.prometheusRemoteWrite is deprecated, and its entries (%s) are applied as %s entries of type prometheus. Move each one there: pipeline.metrics.gateway.destination.prometheusRemoteWrite.<name> becomes %s.<name> with type: prometheus, and its minMetricImportance moves under metrics. While an entry stays here it keeps the credential variable names this map derived (GATEWAY_PROMETHEUS_DEST_<NAME>_PASSWORD, …); name them with the *Env fields when you move it, if a Secret uses them." ( keys $legacyProm.entries | sortAlpha | join ", " ) $path $path ) }}
  {{- end }}
  {{- range $name := $legacyProm.invalid }}
    {{- $errors = append $errors ( printf "pipeline.metrics.gateway.destination.prometheusRemoteWrite.%s is %s, not a destination. Each entry is a map with a url and the rest; better, move it to %s.%s with type: prometheus." $name ( kindOf ( index $.Values.pipeline.metrics.gateway.destination.prometheusRemoteWrite $name ) ) $path $name ) }}
  {{- end }}
  {{- range $name := $legacyProm.conflicts }}
    {{- $errors = append $errors ( printf "pipeline.metrics.gateway.destination.prometheusRemoteWrite.%s names a remote-write destination, but %s.%s is already a destination of type %s. Both maps feed one, so the remote-write entry cannot apply. Rename it." $name $path $name ( dig $name "type" "" ( $.Values.pipeline.destinations | default dict ) ) ) }}
  {{- end }}

  {{- $written := dict }}
  {{- range $entry := ( include "mzmon.destinations.envEntries" $ | fromYamlArray ) }}
    {{- if hasKey $written $entry.key }}
      {{- $errors = append $errors ( printf "%s and %s both write %s to the gateway's env ConfigMap, which keeps one value per key, so one of them would read the other's. Give one a different variable name, or rename a destination so the names differ by more than case." ( index $written $entry.key ) $entry.owner $entry.key ) }}
    {{- else }}
      {{- $_ := set $written $entry.key $entry.owner }}
    {{- end }}
  {{- end }}

  {{- $blocking := list }}
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
    {{- $name := $dest.name }}
    {{- if not ( regexMatch "^[a-zA-Z_][a-zA-Z0-9_]*$" $name ) }}
      {{- $errors = append $errors ( printf "%s.%s is not a usable name. It becomes an Alloy component label, so it must match [a-zA-Z_][a-zA-Z0-9_]* — no dashes, dots, or leading digits." $path $name ) }}
    {{- else if has $name $reserved }}
      {{- $errors = append $errors ( printf "%s.%s uses a name this pipeline has already taken (%s). Two components cannot share a label, so alloy would refuse the whole config at load. Pick another name." $path $name ( join ", " $reserved ) ) }}
    {{- end }}

    {{- if not ( hasKey $types $dest.type ) }}
      {{- $errors = append $errors ( printf "%s.%s.type is %q. Supported types are: %s." $path $name $dest.type ( keys $types | sortAlpha | join ", " ) ) }}
    {{- else }}
      {{- range $signal := list "logs" "metrics" }}
        {{- if and ( index $dest $signal ).enabled ( not ( has $signal ( index $types $dest.type ) ) ) }}
          {{- $errors = append $errors ( printf "%s.%s turns %s on, but a %s destination does not carry %s." $path $name $signal $dest.type $signal ) }}
        {{- end }}
      {{- end }}
      {{- range $field := ( keys $dest.queue | sortAlpha ) }}
        {{- if not ( has $field ( index $queueFields $dest.type ) ) }}
          {{- $errors = append $errors ( printf "%s.%s.queue.%s does not apply to a %s destination, which takes: %s." $path $name $field $dest.type ( index $queueFields $dest.type | join ", " ) ) }}
        {{- end }}
      {{- end }}
      {{- if and $dest.enabled ( not $dest.active ) }}
        {{- $warnings = append $warnings ( printf "%s.%s is enabled but carries no signal, so nothing is sent to it. Add a logs or metrics block." $path $name ) }}
      {{- end }}
    {{- end }}

    {{- $authTypes := dig $dest.type "authTypes" ( list "none" ) ( include "mzmon.destinations.typeDefaults" $ | fromYaml ) }}
    {{- if not ( has $dest.authType $authTypes ) }}
      {{- $errors = append $errors ( printf "%s.%s.authType is %q, which a %s destination does not take. Supported values are: %s." $path $name $dest.authType $dest.type ( join ", " $authTypes ) ) }}
    {{- end }}
    {{- if eq $dest.authType "headers" }}
      {{- if not $dest.headers }}
        {{- $errors = append $errors ( printf "%s.%s.authType is headers, but %s.%s.headers is empty, so the exporter would send no credential." $path $name $path $name ) }}
      {{- end }}
      {{- range $i, $header := $dest.headers }}
        {{- $h := $header | default dict }}
        {{- if not ( kindIs "map" $h ) }}
          {{- $h = dict }}
        {{- end }}
        {{- if not $h.key }}
          {{- $errors = append $errors ( printf "%s.%s.headers[%d] has no key." $path $name $i ) }}
        {{- end }}
        {{- if eq ( len ( compact ( list ( $h.value | default "" ) ( $h.valueEnv | default "" ) ) ) ) 2 }}
          {{- $errors = append $errors ( printf "%s.%s.headers[%d] sets both value and valueEnv. Set value for a literal that renders into the pipeline ConfigMap, or valueEnv to read a secret from the environment." $path $name $i ) }}
        {{- else if not ( or $h.value $h.valueEnv ) }}
          {{- $errors = append $errors ( printf "%s.%s.headers[%d] sets neither value nor valueEnv." $path $name $i ) }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if $dest.logs.enabled }}
      {{- $levels := include "mzmon.destinations.levelsBelow" $ | fromYaml }}
      {{- if and $dest.logs.minLevel ( not ( hasKey $levels $dest.logs.minLevel ) ) }}
        {{- $errors = append $errors ( printf "%s.%s.logs.minLevel is %q. Supported values are: TRACE, DEBUG, INFO, WARN, ERROR, CRITICAL." $path $name $dest.logs.minLevel ) }}
      {{- end }}
      {{- if and $dest.logs.selector ( not ( hasPrefix "{" ( $dest.logs.selector | trim ) ) ) }}
        {{- $errors = append $errors ( printf "%s.%s.logs.selector is %q, which is not a LogQL stream selector. Write it as {label=\"value\"}, optionally followed by line filters." $path $name $dest.logs.selector ) }}
      {{- end }}
      {{- $rate := $dest.logs.sampleRate | float64 }}
      {{- if or ( le $rate 0.0 ) ( gt $rate 1.0 ) }}
        {{- $errors = append $errors ( printf "%s.%s.logs.sampleRate is %v. It is the fraction of entries kept, so it must be greater than 0 and at most 1. To send no logs, set logs.enabled: false." $path $name $dest.logs.sampleRate ) }}
      {{- end }}
      {{- /* Rate and burst are checked independently: a token bucket with a
             burst below its rate still passes the rate, evenly spaced. */}}
      {{- $rl := $dest.logs.rateLimit }}
      {{- if $rl.configured }}
        {{- if or ( kindIs "invalid" $rl.rate ) ( not ( regexMatch "^[0-9]*\\.?[0-9]+$" ( toString $rl.rate ) ) ) ( le ( $rl.rate | float64 ) 0.0 ) }}
          {{- $errors = append $errors ( printf "%s.%s.logs.rateLimit.rate is %v. A rate limit needs a rate in lines per second greater than 0; to remove the limit, remove the rateLimit block." $path $name ( ternary "missing" ( toString $rl.rate ) ( kindIs "invalid" $rl.rate ) ) ) }}
        {{- else if or ( kindIs "invalid" $rl.burst ) ( not ( regexMatch "^[0-9]+$" ( toString $rl.burst ) ) ) ( lt ( $rl.burst | float64 ) 1.0 ) }}
          {{- $errors = append $errors ( printf "%s.%s.logs.rateLimit.burst is %v. Alloy takes the burst as a whole number of lines, at least 1." $path $name $rl.burst ) }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if eq $dest.type "googleCloud" }}
      {{- range $field := list "ca" "cert" "key" "caFile" "certFile" "keyFile" }}
        {{- if index $dest.tls $field }}
          {{- $errors = append $errors ( printf "%s.%s.tls.%s does not apply to a googleCloud destination, which reaches Google's public endpoint with Google's own credentials." $path $name $field ) }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if eq $dest.type "s3" }}
      {{- range $field := list "ca" "cert" "key" "caFile" "certFile" "keyFile" }}
        {{- if index $dest.tls $field }}
          {{- $errors = append $errors ( printf "%s.%s.tls.%s does not apply to an s3 destination, which authenticates with the gateway's AWS credentials. An http:// url turns TLS off for an S3-compatible endpoint." $path $name $field ) }}
        {{- end }}
      {{- end }}
      {{- if $dest.active }}
        {{- range $field := list "bucket" "region" }}
          {{- if not ( index $dest $field ) }}
            {{- $errors = append $errors ( printf "%s.%s.%s is required on an s3 destination." $path $name $field ) }}
          {{- end }}
        {{- end }}
      {{- end }}
      {{- if not ( has $dest.marshaler ( list "otlp_json" "otlp_proto" "body" ) ) }}
        {{- $errors = append $errors ( printf "%s.%s.marshaler is %q. Supported values are: otlp_json, otlp_proto, body." $path $name $dest.marshaler ) }}
      {{- end }}
      {{- if not ( has $dest.compression ( list "none" "gzip" ) ) }}
        {{- $errors = append $errors ( printf "%s.%s.compression is %q. Supported values are: none, gzip." $path $name $dest.compression ) }}
      {{- end }}
    {{- end }}

    {{- if eq $dest.type "otlp" }}
      {{- if not ( has $dest.protocol ( list "http" "grpc" ) ) }}
        {{- $errors = append $errors ( printf "%s.%s.protocol is %q. Supported values are: http, grpc." $path $name $dest.protocol ) }}
      {{- end }}
      {{- range $signal := list "logs" "metrics" }}
        {{- if and ( eq $dest.protocol "grpc" ) ( index $dest $signal ).url }}
          {{- $errors = append $errors ( printf "%s.%s.%s.url overrides the endpoint for one signal, which only the http protocol supports. gRPC serves every signal on the one endpoint." $path $name $signal ) }}
        {{- end }}
      {{- end }}
      {{- /* The inline carriers read variables nothing writes (DEP-465), so a
             new destination type does not take them. */}}
      {{- range $field := list "ca" "cert" "key" }}
        {{- if index $dest.tls $field }}
          {{- $errors = append $errors ( printf "%s.%s.tls.%s is not supported on an otlp destination. Mount the material and use tls.%sFile, which also survives certificate renewal." $path $name $field $field ) }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if and $dest.active $dest.urlRequired ( not $dest.url ) }}
      {{- $errors = append $errors ( printf "%s.%s is enabled but has no url. The rendered pipeline reads sys.env(%q), which would resolve empty — alloy accepts that at load and every write fails at run time." $path $name $dest.urlEnv ) }}
    {{- end }}

    {{- if and $dest.active $dest.logs.enabled }}
      {{- $blocks := index $blocksByDefault $dest.type | default false }}
      {{- if hasKey $dest.queue "blockOnOverflow" }}
        {{- $blocks = $dest.queue.blockOnOverflow }}
      {{- end }}
      {{- if $blocks }}
        {{- $blocking = append $blocking $name }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* The call-home profile's destination sends logs to Materialize. A
         credential the secret filter didn't replace crosses with them, so the
         filter has to be on while it does. */}}
  {{- $secretFilterMode := include "mzmon.secretFilter.mode" $ }}
  {{- range $dest := ( include "mzmon.destinations.logs" $ | fromYamlArray ) }}
    {{- if and ( eq $dest.name "controlPlane" ) ( ne $secretFilterMode "on" ) }}
      {{- $warnings = append $warnings ( printf "%s.controlPlane sends logs to Materialize, but pipeline.logging.secretFilter.mode is %q, so a credential a component logs crosses unredacted. Set the mode to \"on\", as the call-home profile does, or set %s.controlPlane.logs.enabled to false." $path $secretFilterMode $path ) }}
    {{- end }}
  {{- end }}

  {{- if gt ( len $blocking ) 1 }}
    {{- $warnings = append $warnings ( printf "Log destinations %s all block when their queue fills (the default for a loki destination). Every log entry goes to each destination in turn, so any one of them stalling holds back the others. Set queue.blockOnOverflow: false on every destination but the one whose copy must be complete, so the rest drop instead of stalling it." ( join ", " $blocking ) ) }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}


{{- /*
The client TLS settings for the gateway's hop to the bundled Loki.

The Loki ruler sync talks to the same Loki with the same scheme and certificate
as the writer does, so it borrows the writer's TLS: the first enabled
destination that writes to the bundled Loki, else the `loki` entry even when it
is off, else none.

Usage:
  {{- $tls := include "mzmon.destinations.bundledLokiTls" $ | fromYaml }}
*/}}
{{- define "mzmon.destinations.bundledLokiTls" }}
  {{- $tls := dict }}
  {{- $bundled := include "mzmon.destinations.bundledLoki" $ | fromYamlArray }}
  {{- if $bundled }}
    {{- $tls = ( first $bundled ).tls }}
  {{- else }}
    {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
      {{- if eq $dest.name "loki" }}
        {{- $tls = $dest.tls }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $tls | toYaml }}
{{- end }}
