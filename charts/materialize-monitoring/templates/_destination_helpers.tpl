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
  {{- dict
      "loki" ( list "capacity" "minShards" "blockOnOverflow" "drainTimeout" )
      "otlp" ( list "queueSize" "numConsumers" "blockOnOverflow" )
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
  {{- dict "loki" true "otlp" false | toYaml }}
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
The destination map as values describe it, with the deprecated single Loki
destination folded onto the `loki` entry.

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
    "url" ( dig "url" "" $raw | toString )
    "urlEnv" ( dig "urlEnv" $prefix $raw | toString )
    "retries" ( dict
      "maxBackoffRetries" ( dig "retries" "maxBackoffRetries" 10 $raw )
      "minBackoffPeriod" ( dig "retries" "minBackoffPeriod" "1s" $raw | toString )
      "maxBackoffPeriod" ( dig "retries" "maxBackoffPeriod" "5m" $raw | toString )
      "retryOnHttp429" ( dig "retries" "retryOnHttp429" true $raw ) )
    "authType" ( dig "authType" "none" $raw | toString )
    "basicAuth" ( dict
      "username" ( dig "basicAuth" "username" "" $raw | toString )
      "usernameEnv" ( dig "basicAuth" "usernameEnv" ( printf "%s_USERNAME" $prefix ) $raw | toString )
      "password" ( dig "basicAuth" "password" "" $raw | toString )
      "passwordEnv" ( dig "basicAuth" "passwordEnv" ( printf "%s_PASSWORD" $prefix ) $raw | toString ) )
    "bearer" ( dict
      "token" ( dig "bearer" "token" "" $raw | toString )
      "tokenEnv" ( dig "bearer" "tokenEnv" ( printf "%s_BEARER_TOKEN" $prefix ) $raw | toString ) )
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
      "urlEnv" ( dig "urlEnv" ( printf "%s_LOGS" $prefix ) $logs | toString ) )
    "metrics" ( dict
      "enabled" $metricsEnabled
      "minMetricImportance" ( dig "minMetricImportance" "all" $metrics | toString )
      "url" ( dig "url" "" $metrics | toString )
      "urlEnv" ( dig "urlEnv" ( printf "%s_METRICS" $prefix ) $metrics | toString )
      "unfilteredMetricsEnv" ( dig "unfilteredMetricsEnv" ( printf "GATEWAY_UNFILTERED_%s_DEST_METRICS" $slug ) $metrics | toString ) )
    "label" ( ternary $name ( printf "dest_%s" $name ) ( eq $type "loki" ) )
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
  4. **Two active destinations reading one URL variable**, through `urlEnv` or
     names that differ only in case. The env ConfigMap would carry a duplicate
     key, and both writers would read the same endpoint. Credential variables
     are left alone: sharing one between destinations can be deliberate.
  5. **More than one log destination that blocks when its queue fills.** The
     fan-out hands each entry to every destination in turn, so a blocking
     destination that stalls holds back the others. One may block — the
     customer's own copy should — and the rest have to shed. Each type's
     default comes from `mzmon.destinations.blocksByDefault`.

It also warns, once, about values still set under the deprecated
`pipeline.logging.gateway.destination.loki`.

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

  {{- $legacy := include "mzmon.destinations.legacyLoki" $ | fromYaml }}
  {{- if $legacy }}
    {{- $warnings = append $warnings ( printf "pipeline.logging.gateway.destination.loki is deprecated, and its keys (%s) are applied to %s.loki. Move them there: pipeline.logging.gateway.destination.loki.<key> becomes %s.loki.<key>." ( keys $legacy | sortAlpha | join ", " ) $path $path ) }}
  {{- end }}

  {{- $blocking := list }}
  {{- $urlEnvs := dict }}
  {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
    {{- $name := $dest.name }}
    {{- if $dest.active }}
      {{- if hasKey $urlEnvs $dest.urlEnv }}
        {{- $errors = append $errors ( printf "%s.%s and %s.%s both read their URL from %s. The env ConfigMap would carry the key twice, and both writers would read one endpoint. Give one of them a different urlEnv, or a name that differs by more than case." $path ( index $urlEnvs $dest.urlEnv ) $path $name $dest.urlEnv ) }}
      {{- else }}
        {{- $_ := set $urlEnvs $dest.urlEnv $name }}
      {{- end }}
    {{- end }}
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

    {{- if not ( has $dest.authType ( list "none" "basicAuth" "bearer" ) ) }}
      {{- $errors = append $errors ( printf "%s.%s.authType is %q. Supported values are: none, basicAuth, bearer." $path $name $dest.authType ) }}
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

    {{- if and $dest.active ( not $dest.url ) }}
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
