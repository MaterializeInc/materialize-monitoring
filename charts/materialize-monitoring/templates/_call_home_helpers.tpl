{{- /*
Call-home: how much telemetry crosses to Materialize's control plane, chosen by
one values key, `callHome.level`.

The levels are strictly nested, so each sends everything the one below it does
and more. `off` is the default and sends nothing. The level builds the
`controlPlane` destination; an entry the install writes under
`pipeline.destinations.controlPlane` is applied over it and can only send less.
Nothing in the chart reads a level from anywhere but values, so only the
install can raise it.
*/}}


{{- /*
The levels, lowest first, and what each one sends.

  rank     where the level sits on the ladder.
  metrics  the metric tier floor, or empty for no metrics.
  logs     the log reduction, or empty for no logs.

The `heartbeat` and `alerts` tiers are nested in `essential`, so each level's
tier holds the one below it.

Usage:
  {{- $levels := include "mzmon.callHome.levels" $ | fromYaml }}
*/}}
{{- define "mzmon.callHome.levels" }}
{{- /* Quoted: YAML 1.1 reads a bare `off` key as `false`. */}}
"off":
  rank: 0
heartbeat:
  rank: 1
  metrics: heartbeat
alerts:
  rank: 2
  metrics: alerts
metrics:
  rank: 3
  metrics: essential
diagnostics:
  rank: 4
  metrics: essential
  logs:
    minLevel: WARN
    keepUnknown: false
    rateLimit:
      rate: 500
      burst: 5000
{{- end }}


{{- /*
`callHome.level`, as a string.

YAML 1.1 reads an unquoted `off` as `false`, so a values file that writes
`level: off` arrives as a boolean; it maps back to `off`. `true` is not a
level, and `mzmon.callHome.validate` refuses it.

Usage:
  {{- $level := include "mzmon.callHome.level" $ }}
*/}}
{{- define "mzmon.callHome.level" }}
  {{- $level := dig "level" "off" ( $.Values.callHome | default dict ) }}
  {{- if kindIs "bool" $level }}
    {{- ternary "true" "off" $level }}
  {{- else if kindIs "invalid" $level }}
    {{- "off" }}
  {{- else }}
    {{- $level | toString }}
  {{- end }}
{{- end }}


{{- /*
The `controlPlane` destination the level builds, before the install's own
entry is applied over it. Empty at `off` and for a level that doesn't exist.

The wire settings are Materialize's production control plane over OTLP/HTTP,
authenticated with the client certificate from the destination mount. The
signal settings come from `mzmon.callHome.levels`.

Usage:
  {{- $base := include "mzmon.callHome.destination" $ | fromYaml }}
*/}}
{{- define "mzmon.callHome.destination" }}
  {{- $level := include "mzmon.callHome.level" $ }}
  {{- $spec := index ( include "mzmon.callHome.levels" $ | fromYaml ) $level }}
  {{- if and $spec ( gt ( int $spec.rank ) 0 ) }}
    {{- $dest := dict
          "type" "otlp"
          "url" "https://telemetry.c.cloud.materialize.com"
          "protocol" "http"
          "tls" ( dict
            "enabled" true
            "certFile" "/etc/mzmon/destination-tls/tls.crt"
            "keyFile" "/etc/mzmon/destination-tls/tls.key"
            "reloadInterval" "5m" ) }}
    {{- with $spec.metrics }}
      {{- $_ := set $dest "metrics" ( dict "minMetricImportance" . ) }}
    {{- end }}
    {{- with $spec.logs }}
      {{- $_ := set $dest "logs" ( deepCopy . ) }}
    {{- end }}
    {{- $dest | toYaml }}
  {{- end }}
{{- end }}


{{- /*
Whether logs cross to the control plane: the level sends them, and the
install's entry hasn't turned the destination or its logs off.

Read from values directly rather than through the resolved destinations,
because `mzmon.secretFilter.mode` asks on every render.

Usage:
  {{- if ( include "mzmon.callHome.sendsLogs" $ ) }}
*/}}
{{- define "mzmon.callHome.sendsLogs" }}
  {{- $spec := index ( include "mzmon.callHome.levels" $ | fromYaml ) ( include "mzmon.callHome.level" $ ) }}
  {{- if and $spec $spec.logs }}
    {{- $entry := dig "destinations" "controlPlane" dict ( $.Values.pipeline | default dict ) }}
    {{- if not ( kindIs "map" $entry ) }}
      {{- $entry = dict }}
    {{- end }}
    {{- $logs := index $entry "logs" }}
    {{- if not ( kindIs "map" $logs ) }}
      {{- $logs = dict }}
    {{- end }}
    {{- if and ( ne ( toString ( index $entry "enabled" ) ) "false" ) ( ne ( toString ( index $logs "enabled" ) ) "false" ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}


{{- /*
Validate `callHome.level` and the `controlPlane` destination against it.

The level is a ceiling. An install's entry under
`pipeline.destinations.controlPlane` can change the wire (the URL, the
certificate files, the queue) and can send less than the level, but nothing
past it:

  * No metric tier wider than the level's.
  * No logs below `diagnostics`, no level floor below `WARN`, no `UNKNOWN`
    lines, and no rate or burst above the level's limit.
  * The type stays `otlp`.

At `off`, an entry that isn't `enabled: false` is refused rather than
ignored, so an install that wrote the destination by hand finds out it isn't
sending. Above `off`, a release that doesn't record the heartbeat is warned:
the control plane reads a missing heartbeat as an install gone dark.

Usage:
  {{- $res := include "mzmon.callHome.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.callHome.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $levels := include "mzmon.callHome.levels" $ | fromYaml }}
  {{- $level := include "mzmon.callHome.level" $ }}
  {{- $spec := index $levels $level }}
  {{- $path := "pipeline.destinations.controlPlane" }}
  {{- $entry := dig "destinations" "controlPlane" dict ( $.Values.pipeline | default dict ) }}

  {{- if not $spec }}
    {{- $errors = append $errors ( printf "callHome.level is %q. It must be off, heartbeat, alerts, metrics, or diagnostics. Quote off, as \"off\", so YAML reads it as a string." $level ) }}
  {{- else if eq ( int $spec.rank ) 0 }}
    {{- if and ( kindIs "map" $entry ) $entry ( ne ( toString ( index $entry "enabled" ) ) "false" ) }}
      {{- $errors = append $errors ( printf "%s is the destination for Materialize's control plane, but callHome.level is off, so nothing is sent to it. Set callHome.level to the level you agree to send, for example metrics; the level builds the destination, and %s can only send less." $path $path ) }}
    {{- end }}
  {{- else }}
    {{- $recorded := has "mzmon_heartbeat/mzmon:heartbeat" ( include "mzmon.rules.installedRecords" $ | fromYamlArray ) }}
    {{- if not ( or $recorded ( include "mzmon.otherReleases.runs" ( dict "root" $ "component" "thanos" ) ) ) }}
      {{- $warnings = append $warnings ( printf "callHome.level is %s, but this release doesn't record the heartbeat, so Materialize can't tell whether the install is reporting. The heartbeat needs rules.enabled, and the bundled Thanos with thanos.global.serviceMonitor.enabled. If another release of this chart runs Thanos, set otherReleases.thanos: true." $level ) }}
    {{- end }}
    {{- range $dest := ( include "mzmon.destinations.list" $ | fromYamlArray ) }}
      {{- if eq $dest.name "controlPlane" }}
        {{- if ne $dest.type "otlp" }}
          {{- $errors = append $errors ( printf "%s.type is %q. The control plane takes OTLP, so the type is otlp." $path $dest.type ) }}
        {{- end }}

        {{- if $dest.metrics.enabled }}
          {{- /* Narrowest first; a level allows its own tier and those inside it. */}}
          {{- $allowed := list }}
          {{- range $tier := list "heartbeat" "alerts" "essential" }}
            {{- if not ( has $spec.metrics $allowed ) }}
              {{- $allowed = append $allowed $tier }}
            {{- end }}
          {{- end }}
          {{- if not ( has $dest.metrics.minMetricImportance $allowed ) }}
            {{- $errors = append $errors ( printf "callHome.level is %s, which sends metrics at the %s tier, but %s.metrics.minMetricImportance is %q. An entry can only send less than its level: set it to %s." $level $spec.metrics $path $dest.metrics.minMetricImportance ( join ", " $allowed ) ) }}
          {{- end }}
        {{- end }}

        {{- if $dest.logs.enabled }}
          {{- if not $spec.logs }}
            {{- $errors = append $errors ( printf "callHome.level is %s, which sends no logs, but %s turns logs on. To send logs, set callHome.level to diagnostics." $level $path ) }}
          {{- else }}
            {{- $allowed := list }}
            {{- $below := index ( include "mzmon.destinations.levelsBelow" $ | fromYaml ) $spec.logs.minLevel }}
            {{- range $l := list "TRACE" "DEBUG" "INFO" "WARN" "ERROR" "CRITICAL" }}
              {{- if not ( has $l $below ) }}
                {{- $allowed = append $allowed $l }}
              {{- end }}
            {{- end }}
            {{- if not ( has $dest.logs.minLevel $allowed ) }}
              {{- $errors = append $errors ( printf "callHome.level is %s, which sends logs at %s and above, but %s.logs.minLevel is %q. An entry can only send less than its level: set it to %s." $level $spec.logs.minLevel $path $dest.logs.minLevel ( join ", " $allowed ) ) }}
            {{- end }}
            {{- if and $dest.logs.keepUnknown ( not $spec.logs.keepUnknown ) }}
              {{- $errors = append $errors ( printf "callHome.level is %s, which doesn't send lines whose level the gateway couldn't determine, but %s.logs.keepUnknown is true. An entry can only send less than its level." $level $path ) }}
            {{- end }}
            {{- $rl := $dest.logs.rateLimit }}
            {{- $limit := $spec.logs.rateLimit }}
            {{- if not $rl.configured }}
              {{- $errors = append $errors ( printf "callHome.level is %s, which sends at most %v lines per second, but %s.logs.rateLimit is cleared. Set a rate of at most %v and a burst of at most %v." $level $limit.rate $path $limit.rate $limit.burst ) }}
            {{- else }}
              {{- if and ( not ( kindIs "invalid" $rl.rate ) ) ( regexMatch "^[0-9]*\\.?[0-9]+$" ( toString $rl.rate ) ) ( gt ( $rl.rate | float64 ) ( $limit.rate | float64 ) ) }}
                {{- $errors = append $errors ( printf "callHome.level is %s, which sends at most %v lines per second, but %s.logs.rateLimit.rate is %v. An entry can only send less than its level." $level $limit.rate $path $rl.rate ) }}
              {{- end }}
              {{- if and ( not ( kindIs "invalid" $rl.burst ) ) ( regexMatch "^[0-9]+$" ( toString $rl.burst ) ) ( gt ( $rl.burst | float64 ) ( $limit.burst | float64 ) ) }}
                {{- $errors = append $errors ( printf "callHome.level is %s, which allows bursts of at most %v lines, but %s.logs.rateLimit.burst is %v. An entry can only send less than its level." $level $limit.burst $path $rl.burst ) }}
              {{- end }}
            {{- end }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
