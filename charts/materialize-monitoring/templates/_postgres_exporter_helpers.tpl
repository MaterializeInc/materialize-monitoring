{{- /* postgres_exporter helpers and validators.

The subchart runs the exporter; this chart owns its scrape, one ServiceMonitor
endpoint per `postgres-exporter.targets` entry, in
`templates/scrapers/monitor-postgres-exporter.yaml`.
*/}}

{{- /*
Check if postgres-exporter is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not. It
is in no tag group, so only its own tag or circuit breaker turns it on.
*/}}
{{- define "mzmon.postgresExporter.enabled" }}
  {{- $values := index $.Values "postgres-exporter" | required "postgres-exporter is missing from values." }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else if index $.Values.tags "postgres-exporter" }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
`postgres-exporter.targets` with defaults filled in, as a YAML list. Entries
that fail validation are left out; the validator reports them.

Usage:
  {{- $targets := include "mzmon.postgresExporter.targets" $ | fromYamlArray }}
*/}}
{{- define "mzmon.postgresExporter.targets" }}
  {{- $out := list }}
  {{- $seen := list }}
  {{- $targets := ( index $.Values "postgres-exporter" ).targets | default list }}
  {{- if kindIs "slice" $targets }}
    {{- range $targets }}
      {{- if and ( kindIs "map" . ) ( include "mzmon.postgresExporter.validName" .name ) ( kindIs "string" .host ) .host }}
        {{- if not ( has ( lower .name ) $seen ) }}
          {{- $seen = append $seen ( lower .name ) }}
          {{- $out = append $out ( dict
              "name" .name
              "host" .host
              "port" ( .port | default 5432 | int )
              "database" ( .database | default "postgres" )
              "authModule" ( .authModule | default .name )
            ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}

{{- /*
Whether a target name is valid: letters, digits and hyphens, starting and ending
with a letter or digit. It becomes a label value and lands inside a PromQL
string in the recording rules, so it is kept to characters that need no
escaping there.
*/}}
{{- define "mzmon.postgresExporter.validName" }}
  {{- if and ( kindIs "string" . ) ( regexMatch "^[a-zA-Z0-9]([-a-zA-Z0-9]*[a-zA-Z0-9])?$" ( toString . ) ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Validate `postgres-exporter`.

Returns `errors` and `warnings` lists, as YAML.
*/}}
{{- define "mzmon.postgresExporter.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := index $.Values "postgres-exporter" }}
  {{- $targets := $values.targets | default list }}
  {{- $fields := list "name" "host" "port" "database" "authModule" }}

  {{- if not ( kindIs "slice" $targets ) }}
    {{- $errors = append $errors ( printf "postgres-exporter.targets is %s; it is a list of databases, each with a `name` and a `host`." ( toJson $targets ) ) }}
  {{- else }}
    {{- $names := list }}
    {{- range $i, $t := $targets }}
      {{- if not ( kindIs "map" $t ) }}
        {{- $errors = append $errors ( printf "postgres-exporter.targets[%d] is %s; it is a map with a `name` and a `host`." $i ( toJson $t ) ) }}
      {{- else }}
        {{- range $key, $_ := $t }}
          {{- if not ( has $key $fields ) }}
            {{- $errors = append $errors ( printf "postgres-exporter.targets[%d].%s is not a field; a target has %s." $i $key ( join ", " $fields ) ) }}
          {{- end }}
        {{- end }}
        {{- /* Compared ignoring case, because the recording rules match target
               names with `(?i)`: two names differing only in case would both be
               recorded when either is declared a metadata database. */}}
        {{- if not ( include "mzmon.postgresExporter.validName" $t.name ) }}
          {{- $errors = append $errors ( printf "postgres-exporter.targets[%d].name is %s; it is letters, digits and hyphens, and becomes the target's `target` label." $i ( toJson $t.name ) ) }}
        {{- else if has ( lower $t.name ) $names }}
          {{- $errors = append $errors ( printf "postgres-exporter.targets[%d].name %q is used by an earlier target, ignoring case; each target's name is its identity." $i $t.name ) }}
        {{- else }}
          {{- $names = append $names ( lower $t.name ) }}
        {{- end }}
        {{- if not ( and ( kindIs "string" $t.host ) $t.host ) }}
          {{- $errors = append $errors ( printf "postgres-exporter.targets[%d].host is %s; it is the database host the exporter connects to." $i ( toJson $t.host ) ) }}
        {{- end }}
        {{- /* An optional field that is present is checked even when it is
               falsy: `with` would skip a `0` or a `false`, and the targets helper
               would then quietly put the default in its place. Only an absent or
               `null` field takes the default. */}}
        {{- $port := index $t "port" }}
        {{- if and ( hasKey $t "port" ) ( not ( typeIs "<nil>" $port ) ) }}
          {{- if not ( and ( or ( kindIs "float64" $port ) ( kindIs "int64" $port ) ( kindIs "int" $port ) ) ( ge ( int $port ) 1 ) ( le ( int $port ) 65535 ) ( eq ( float64 $port ) ( float64 ( int $port ) ) ) ) }}
            {{- $errors = append $errors ( printf "postgres-exporter.targets[%d].port is %s; it is a port number." $i ( toJson $port ) ) }}
          {{- end }}
        {{- end }}
        {{- range $key := list "database" "authModule" }}
          {{- $v := index $t $key }}
          {{- if and ( hasKey $t $key ) ( not ( typeIs "<nil>" $v ) ) }}
            {{- if not ( and ( kindIs "string" $v ) ( regexMatch "^[^/?#&=\\s]+$" $v ) ) }}
              {{- $errors = append $errors ( printf "postgres-exporter.targets[%d].%s is %s; it is a name, and may not contain whitespace or any of / ? # & =." $i $key ( toJson $v ) ) }}
            {{- end }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if include "mzmon.postgresExporter.enabled" $ }}
      {{- if not $targets }}
        {{- $warnings = append $warnings "postgres-exporter is enabled with no targets, so it runs and watches nothing. List each database under postgres-exporter.targets." }}
      {{- end }}
      {{- if not ( include "mzmon.alloyGateway.enabled" $ ) }}
        {{- $warnings = append $warnings "postgres-exporter is enabled, but the alloy-gateway that scrapes it is not, so nothing collects what it exports." }}
      {{- end }}
      {{- if not ( dig "config" "existingSecret" "enabled" false $values ) }}
        {{- $errors = append $errors "postgres-exporter.config.existingSecret.enabled is off. The exporter reads each target's credentials from that Secret's postgres_exporter.yml and from nowhere else." }}
      {{- end }}
    {{- else if $targets }}
      {{- $warnings = append $warnings "postgres-exporter.targets lists databases, but postgres-exporter is not enabled, so none of them is watched. Set tags.postgres-exporter: true." }}
    {{- end }}
  {{- end }}

  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
