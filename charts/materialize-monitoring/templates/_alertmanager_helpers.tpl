{{- /* Alertmanager helpers and validators. */}}

{{- /*
Check if alertmanager is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.alertmanager.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.alertmanager.enabled" }}
  {{- $values := index $.Values "alertmanager" | required "alertmanager is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "bundled-backends" ) $tags.alertmanager ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get alertmanager namespace.

Usage:
  {{- include "mzmon.alertmanager.namespace" $ }}
*/}}
{{- define "mzmon.alertmanager.namespace" }}
  {{- $values := index $.Values "alertmanager" | default dict }}
  {{- $ns := $values.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Get the alertmanager resource name.

**Alertmanager is the one subchart this chart does not pin a `fullnameOverride`
on**, so unlike `loki`, `thanos` and `grafana` its Service name is derived from
the release name at install time — `mzmon-alertmanager` under the conventional
release name, something else under any other. Every consumer of that name has to
go through here rather than hardcoding it.

This reimplements the subchart's own `alertmanager.fullname`, including the
`contains` shortcut that collapses `alertmanager-alertmanager` to a single
segment. Keep the two in step across subchart bumps; the failure mode is a ruler
notifying a Service that does not exist, which looks exactly like no alerts.

Usage:
  {{- include "mzmon.alertmanager.fullname" $ }}
*/}}
{{- define "mzmon.alertmanager.fullname" }}
  {{- $values := index $.Values "alertmanager" | default dict }}
  {{- if $values.fullnameOverride }}
    {{- $values.fullnameOverride | trunc 63 | trimSuffix "-" }}
  {{- else }}
    {{- $name := $values.nameOverride | default "alertmanager" }}
    {{- if contains $name $.Release.Name }}
      {{- $.Release.Name | trunc 63 | trimSuffix "-" }}
    {{- else }}
      {{- printf "%s-%s" $.Release.Name $name | trunc 63 | trimSuffix "-" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get the alertmanager `host:port` authority, fully qualified.

Fully qualified rather than bare, because `split-namespace` puts Alertmanager
somewhere other than the rulers and a short name would only resolve from its own
namespace.

Usage:
  {{- include "mzmon.alertmanager.hostPort" $ }}
*/}}
{{- define "mzmon.alertmanager.hostPort" }}
  {{- $values := index $.Values "alertmanager" | default dict }}
  {{- $port := dig "service" "port" 9093 $values }}
  {{- printf "%s.%s.svc.cluster.local:%v" ( include "mzmon.alertmanager.fullname" $ ) ( include "mzmon.alertmanager.namespace" $ ) $port }}
{{- end }}

{{- /*
Get the alertmanager base URL.

Usage:
  {{- include "mzmon.alertmanager.url" $ }}
*/}}
{{- define "mzmon.alertmanager.url" }}
  {{- printf "http://%s" ( include "mzmon.alertmanager.hostPort" $ ) }}
{{- end }}

{{- /*
Validate Alertmanager against the two rulers that now notify it.

Usage:
  {{- $res := include "mzmon.alertmanager.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.alertmanager.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $enabled := include "mzmon.alertmanager.enabled" $ }}
  {{- $thanosRuler := include "mzmon.thanos.ruler.enabled" $ }}
  {{- $lokiRuler := include "mzmon.loki.ruler.enabled" $ }}

  {{- /* A ruler with nowhere to notify evaluates rules and drops every alert on
         the floor, reporting healthy while it does it.

         A warning rather than an error, deliberately. `tags.thanos: true` on its
         own is a legitimate metrics-only install, and the rulers default on
         because they are components of their subcharts rather than things a tag
         gates. Failing that render would turn "you probably want Alertmanager
         too" into "this chart will not install", which is not the chart's call
         to make. The stack still runs; it just cannot tell anyone about it. */}}
  {{- if and ( not $enabled ) ( or $thanosRuler $lokiRuler ) }}
    {{- $which := list }}
    {{- if $thanosRuler }}
      {{- $which = append $which "thanos.ruler" }}
    {{- end }}
    {{- if $lokiRuler }}
      {{- $which = append $which "loki.ruler" }}
    {{- end }}
    {{- $warnings = append $warnings ( printf "%s %s enabled but Alertmanager is not, so every alert they evaluate is discarded. Enable `tags.alertmanager`, or turn the rulers off — a ruler that cannot notify is indistinguishable from a healthy one." ( join " and " $which ) ( ternary "is" "are" ( eq ( len $which ) 1 ) ) ) }}
  {{- end }}

  {{- /* Two replicas gossip; two replicas that cannot gossip send everything
         twice. The NetworkPolicy already opens 9094 on both transports, so this
         only fires when someone has closed it. */}}
  {{- if $enabled }}
    {{- $values := index $.Values "alertmanager" | default dict }}
    {{- $replicas := dig "replicaCount" 1 $values | int }}
    {{- if gt $replicas 1 }}
      {{- $np := dig "alertmanager" "enabled" nil $.Values.networkPolicies }}
      {{- $npOn := ternary ( $.Values.networkPolicies.enabled ) $np ( typeIs "<nil>" $np ) }}
      {{- if and $npOn ( not ( dig "alertmanager" "ingress" "ports" list $.Values.networkPolicies ) ) }}
        {{- $warnings = append $warnings ( printf "alertmanager.replicaCount is %d but networkPolicies.alertmanager.ingress.ports is empty, so the gossip port is closed. Peers that cannot gossip do not deduplicate, and every notification goes out once per replica." $replicas ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
