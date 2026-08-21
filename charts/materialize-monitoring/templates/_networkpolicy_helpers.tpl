{{- /* NetworkPolicy helpers and validators for the chart-rendered policies. */}}

{{- /*
The subcharts this chart renders NetworkPolicies for, because they ship none of
their own.

Each entry carries what the policy needs and nothing more:

  name:      the `app.kubernetes.io/name` its pods actually carry, which for
             every one of these is the subchart's own chart name (none of them
             is aliased in `Chart.yaml`, and none sets `nameOverride`).
  namespace: the helper that resolves where it lands, or the empty string for a
             subchart that has no `namespaceOverride` at all.

A subchart that grows its own `networkpolicy.yaml` upstream should be removed
from here rather than left to render two overlapping policies — NetworkPolicies
are additive, so the tighter one would stop meaning anything.

Usage:
  {{- $apps := include "mzmon.networkPolicy.apps" $ | fromYaml }}
*/}}
{{- define "mzmon.networkPolicy.apps" }}
alertmanager:
  name: alertmanager
  namespaceHelper: mzmon.alertmanager.namespace
grafana-operator:
  name: grafana-operator
  namespaceHelper: mzmon.grafanaOperator.namespace
metrics-server:
  name: metrics-server
  namespaceHelper: ""
{{- end }}

{{- /*
Namespace the bundled Alertmanager runs in.

Mirrors the subchart's own `alertmanager.namespace` helper. There is no
`mzmon.alertmanager.*` helper file, and this is the only thing that needs one.

Usage:
  {{ include "mzmon.alertmanager.namespace" $ }}
*/}}
{{- define "mzmon.alertmanager.namespace" }}
  {{- $values := $.Values.alertmanager | default dict }}
  {{- $ns := $values.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Whether this chart should render its own NetworkPolicy for a given subchart.

Three things all have to hold, and they are checked in this order:

  1. The subchart is part of this render. `.Subcharts` holds exactly the
     dependencies Helm resolved, so a chart switched off by tag or by circuit
     breaker cannot leave a policy behind selecting pods that do not exist.
  2. `networkPolicies.enabled` is on, unless the per-app key overrides it.
  3. The per-app key, when set, wins in both directions — `false` opts one
     component out of a policed stack, `true` opts one in when the master
     switch is off.

Returns a truthy string when the policy should render, and the empty string
otherwise.

Usage:
  {{- if ( include "mzmon.networkPolicy.enabled" ( dict "context" $ "app" "alertmanager" ) ) }}
*/}}
{{- define "mzmon.networkPolicy.enabled" }}
  {{- $context := .context | required ".context must be specified" }}
  {{- $app := .app | required ".app must be specified" }}
  {{- if hasKey $context.Subcharts $app }}
    {{- $nps := $context.Values.networkPolicies | default dict }}
    {{- $cfg := index $nps $app | default dict }}
    {{- /* `typeIs "<nil>"` rather than a plain truth test: `enabled: false` has
           to be distinguishable from `enabled: null`, and only the second one
           means "follow the master switch". */}}
    {{- if typeIs "<nil>" $cfg.enabled }}
      {{- ternary "true" "" ( $nps.enabled | default false ) }}
    {{- else }}
      {{- ternary "true" "" $cfg.enabled }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Entrypoint for NetworkPolicy validation.

Scoped to the policies this chart renders itself. Each subchart's own policy is
validated next to that subchart — `mzmon.loki.validate.networkPolicy` is the
pattern the rest follow.

Usage:
  {{- $res := include "mzmon.networkPolicy.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.networkPolicy.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $nps := $.Values.networkPolicies | required "networkPolicies is missing from values." }}
  {{- $apps := include "mzmon.networkPolicy.apps" $ | fromYaml }}

  {{- range $app, $_ := $apps }}
    {{- /* Only speak about components this render actually contains. */}}
    {{- if hasKey $.Subcharts $app }}
      {{- $cfg := index $nps $app | required ( printf "networkPolicies.%s is missing from values." $app ) }}
      {{- if ( include "mzmon.networkPolicy.enabled" ( dict "context" $ "app" $app ) ) }}
        {{- $res := include "mzmon.networkPolicy.validate.app" ( dict "app" $app "cfg" $cfg ) | fromYaml }}
        {{- $errors = concat $errors $res.errors | default list }}
        {{- $warnings = concat $warnings $res.warnings | default list }}
      {{- else }}
        {{- $warnings = append $warnings ( printf "networkPolicies.%s is off, so %s runs unpoliced in a namespace where the rest of the stack is not. Enabling it is recommended in production." $app $app ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- $res := include "mzmon.networkPolicy.validate.splitNamespace" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res := include "mzmon.networkPolicy.validate.grafanaOperatorEgress" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate one chart-rendered policy.

The errors here are the shapes that render a policy which cannot work at all;
the warnings are the ones that work but give up most of what the policy was for.

Usage:
  {{- include "mzmon.networkPolicy.validate.app" ( dict "app" "alertmanager" "cfg" $cfg ) }}
*/}}
{{- define "mzmon.networkPolicy.validate.app" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $app := .app | required ".app must be specified" }}
  {{- $cfg := .cfg | required ".cfg must be specified" }}

  {{- $ingress := $cfg.ingress | required ( printf "networkPolicies.%s.ingress is missing." $app ) }}
  {{- $egress := $cfg.egress | required ( printf "networkPolicies.%s.egress is missing." $app ) }}

  {{- /* A policy that names no port and adds no rule of its own selects the pod
         and permits nothing to reach it. That is a deny-all wearing the shape of
         an allowlist, and it is never what someone meant to write. */}}
  {{- if and ( not $ingress.ports ) ( not $ingress.extra ) }}
    {{- $errors = append $errors ( printf "networkPolicies.%s.ingress opens no ports and adds no extra rules, which denies all ingress to %s rather than allowing anything. List the ports it serves, add an `extra` rule, or set networkPolicies.%s.enabled=false." $app $app $app ) }}
  {{- end }}

  {{- /* DNS is not optional for anything that dials a Service or a hostname, and
         its absence presents as timeouts against names that resolve fine from
         every other pod. */}}
  {{- if not $egress.dns }}
    {{- $warnings = append $warnings ( printf "networkPolicies.%s.egress.dns is off. Every destination %s reaches by name — a Service, an SMTP relay, the API server — fails to resolve, and the symptom is a timeout rather than a policy error." $app $app ) }}
  {{- end }}

  {{- /* An external block with ports and no CIDRs is the trap in this shape: the
         rendered rule has `ports` and no `to`, which Kubernetes reads as "these
         ports, anywhere" — broader than the author of a CIDR-less list intended. */}}
  {{- $external := $egress.external | default dict }}
  {{- if and $external.ports ( not $external.cidrs ) }}
    {{- $warnings = append $warnings ( printf "networkPolicies.%s.egress.external.ports is set with no cidrs, which allows those ports to every destination rather than to none. Set cidrs (0.0.0.0/0 if that is what you mean), or clear ports." $app ) }}
  {{- end }}

  {{- if $ingress.allowExternal }}
    {{- /* True is correct for metrics-server and a real loosening anywhere else,
           so say so once rather than leaving it to a reader of the diff. */}}
    {{- if ne $app "metrics-server" }}
      {{- $warnings = append $warnings ( printf "networkPolicies.%s.ingress.allowExternal is on, so ports %v accept connections from any namespace and from outside the cluster. Prefer an `extra` rule naming the namespaces that need in." $app $ingress.ports ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Warn when the stack is spread across namespaces but the policies still speak in
namespace-local terms.

Every default rule in this chart's policies — and in `alloy-agent`,
`alloy-gateway` and `kube-state-metrics`, which select the gateway by pod label —
resolves within one namespace. Move a component out with `namespaceOverride` and
those rules do not error, they simply match nothing, and the traffic they were
written to allow is dropped. `profiles/split-namespace.values.yaml` does exactly
that, and calls itself best-effort for reasons this is one of.

Usage:
  {{- include "mzmon.networkPolicy.validate.splitNamespace" $ }}
*/}}
{{- define "mzmon.networkPolicy.validate.splitNamespace" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $release := include "mzmon.namespace" $ }}
  {{- $split := list }}
  {{- range $chart := ( list "alloy-agent" "alloy-gateway" "loki" "thanos" "grafana" "grafana-operator" "alertmanager" ) }}
    {{- if hasKey $.Subcharts $chart }}
      {{- $values := index $.Values $chart | default dict }}
      {{- $ns := $values.namespaceOverride | default "" }}
      {{- if and $ns ( ne $ns $release ) }}
        {{- $split = append $split $chart }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- if $split }}
    {{- $policed := list }}
    {{- range $chart := ( list "alloy-agent" "alloy-gateway" "kube-state-metrics" ) }}
      {{- if hasKey $.Subcharts $chart }}
        {{- $np := ( index $.Values $chart | default dict ).networkPolicy | default dict }}
        {{- if $np.enabled }}
          {{- $policed = append $policed $chart }}
        {{- end }}
      {{- end }}
    {{- end }}
    {{- range $app, $_ := ( include "mzmon.networkPolicy.apps" $ | fromYaml ) }}
      {{- if ( include "mzmon.networkPolicy.enabled" ( dict "context" $ "app" $app ) ) }}
        {{- $policed = append $policed $app }}
      {{- end }}
    {{- end }}
    {{- if $policed }}
      {{- $warnings = append $warnings ( printf "%v run outside the release namespace %q, but the NetworkPolicies on %v select their peers by pod label alone, which does not cross a namespace boundary. Those rules will match nothing and the traffic they allow will be dropped. Add namespaceSelector entries (networkPolicies.<app>.ingress.extra / egress.extra, and the subcharts' own ingress lists), or turn the affected policies off." ( sortAlpha $split ) $release ( sortAlpha $policed ) ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the NetworkPolicies on the two per-node/per-cluster collectors.

`kube-state-metrics` and `node-exporter` have no `_<chart>_helpers.tpl` of their
own, so their policy validation lives here rather than in a file created to hold
four checks. Everything else validates next to its own subchart, the way
`mzmon.loki.validate.networkPolicy` does.

Both subcharts render `policyTypes: [Ingress, Egress]` unconditionally, which is
what makes an omitted rule list a deny rather than an absence — and what the
checks below are mostly about.

Usage:
  {{- $res := include "mzmon.networkPolicy.validate.collectors" $ | fromYaml }}
*/}}
{{- define "mzmon.networkPolicy.validate.collectors" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if hasKey $.Subcharts "kube-state-metrics" }}
    {{- $values := index $.Values "kube-state-metrics" | required "kube-state-metrics is missing from values." }}
    {{- $np := $values.networkPolicy | default dict }}
    {{- if $np.enabled }}
      {{- /* The failure this exists for. kube-state-metrics is a watch on the
             API server and nothing else; deny its egress and it lists no
             objects, stays Ready, and every `kube_*` series disappears with no
             error anywhere. The subchart declares the Egress policy type
             whether or not you give it rules, so the empty case is a deny. */}}
      {{- if not $np.egress }}
        {{- $errors = append $errors "kube-state-metrics.networkPolicy.egress is empty while the policy is enabled, which denies all egress — the subchart always declares the Egress policy type. kube-state-metrics would stay Ready and stop producing every kube_* series, because it could no longer reach the API server. Allow DNS and 443/6443 at minimum." }}
      {{- end }}
      {{- if not $np.ingress }}
        {{- $warnings = append $warnings "kube-state-metrics.networkPolicy.ingress is empty, so the subchart falls back to allowing its metrics ports from every source in the cluster. Name the scraper (podSelector on app.kubernetes.io/name: alloy-gateway) to make the policy mean something." }}
      {{- end }}
    {{- else }}
      {{- $warnings = append $warnings "kube-state-metrics.networkPolicy.enabled is recommended in production." }}
    {{- end }}
  {{- end }}

  {{- if hasKey $.Subcharts "node-exporter" }}
    {{- $values := index $.Values "node-exporter" | required "node-exporter is missing from values." }}
    {{- $np := $values.networkPolicy | default dict }}
    {{- if $np.enabled }}
      {{- if not $np.ingress }}
        {{- $warnings = append $warnings "node-exporter.networkPolicy.ingress is empty, so the subchart falls back to allowing its service port from every source. Name the scraper instead." }}
      {{- end }}
      {{- /* The hostNetwork caveat — that most CNIs do not apply pod policy to
             these pods at all — is deliberately not a warning. It is true of
             every default install, so it would fire on every render and teach
             people to skim the warning block. It lives at length next to
             `node-exporter.networkPolicy` in values.yaml instead. */}}
    {{- else }}
      {{- $warnings = append $warnings "node-exporter.networkPolicy.enabled is recommended in production." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Whether the Grafana gossip policy should render.

Four conditions, and the last two are each load-bearing for a different reason:

  1. The `grafana` subchart is part of this render.
  2. `networkPolicies.grafanaGossip.enabled` is on, or unset and following
     `networkPolicies.enabled`.
  3. **`grafana.networkPolicy.enabled` is on.** This is a supplement to that
     policy and is not safe on its own. A NetworkPolicy selecting a pod isolates
     it for every direction it declares, so a lone `Ingress` policy opening 9094
     would deny everything else — including the UI on `service.targetPort`. With
     the main policy off there is nothing to supplement and nothing to isolate,
     so the right answer is to render nothing.
  4. `grafana.ini.unified_alerting.ha_peers` is set. That is the value that makes
     the replicas dial the gossip port; without it nothing connects to 9094 and
     the rule would be an allowance for traffic that does not exist.

Usage:
  {{- if ( include "mzmon.networkPolicy.grafanaGossip.enabled" $ ) }}
*/}}
{{- define "mzmon.networkPolicy.grafanaGossip.enabled" }}
  {{- if hasKey $.Subcharts "grafana" }}
    {{- $nps := $.Values.networkPolicies | default dict }}
    {{- $cfg := $nps.grafanaGossip | default dict }}
    {{- $on := $nps.enabled | default false }}
    {{- if not ( typeIs "<nil>" $cfg.enabled ) }}
      {{- $on = $cfg.enabled }}
    {{- end }}
    {{- $main := ( $.Values.grafana.networkPolicy | default dict ).enabled }}
    {{- if and $on $main }}
      {{- $ha := ( include "mzmon.grafana.iniSection" ( dict "root" $ "name" "unified_alerting" ) | fromYaml ).ha_peers | default "" }}
      {{- if $ha }}
        {{- "true" }}
      {{- end }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Normalize a port list into `NetworkPolicyPort` entries.

Entries may be written either way, and both appear in `values.yaml`:

  ports:
    - 9093              # a bare number, which means TCP
    - port: 9094        # a map, for anything that is not
      protocol: UDP

The bare form covers almost everything, and spelling `protocol: TCP` on every
line would bury the one entry where the protocol is the point. The map form
exists because a memberlist-style gossip port needs **both** transports — TCP
for the join and for state pushes, UDP for the gossip itself — and a TCP-only
rule produces a cluster that forms and then never converges. Alertmanager's
`9094` is exactly that case.

Returns a YAML array, so the caller reads it back with `fromYamlArray`.

Usage:
  {{- $ports := include "mzmon.networkPolicy.ports" $ingress.ports | fromYamlArray }}
*/}}
{{- define "mzmon.networkPolicy.ports" }}
  {{- $out := list }}
  {{- range $entry := . }}
    {{- if kindIs "map" $entry }}
      {{- $port := $entry.port | required "a networkPolicies port entry written as a map must set `port`." }}
      {{- $out = append $out ( dict "port" $port "protocol" ( $entry.protocol | default "TCP" ) ) }}
    {{- else }}
      {{- $out = append $out ( dict "port" $entry "protocol" "TCP" ) }}
    {{- end }}
  {{- end }}
  {{- $out | toYaml }}
{{- end }}

{{- /*
Warn when grafana-operator's egress cannot reach the Grafana it is pointed at.

Only fires under `connections.grafana.mode: external`, where the instance lives
outside the cluster and `egress.external.ports` is the only rule that can reach
it. The shipped list is `443`/`6443`, which covers an HTTPS Grafana on the
default port and nothing else — so a self-hosted instance on `:3000`, or a plain
`http://` one on `:80`, leaves the operator running, healthy, and reconciling
into a connection that never opens.

The port is read off the URL rather than assumed: an explicit `:port` wins, and
otherwise the scheme decides. Anything this cannot parse confidently — an IPv6
literal, a templated host — is left alone rather than guessed at, because a
warning that fires on a working install is worse than no warning.

Usage:
  {{- $res := include "mzmon.networkPolicy.validate.grafanaOperatorEgress" $ | fromYaml }}
*/}}
{{- define "mzmon.networkPolicy.validate.grafanaOperatorEgress" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if ( include "mzmon.networkPolicy.enabled" ( dict "context" $ "app" "grafana-operator" ) ) }}
    {{- $conn := dig "grafana" "mode" "" ( $.Values.connections | default dict ) }}
    {{- $url := dig "grafana" "external" "url" "" ( $.Values.connections | default dict ) }}
    {{- if and ( eq $conn "external" ) $url }}
      {{- $scheme := regexFind "^[a-zA-Z][a-zA-Z0-9+.-]*://" $url | trimSuffix "://" | lower }}
      {{- $authority := regexReplaceAll "^[a-zA-Z][a-zA-Z0-9+.-]*://" $url "" | splitList "/" | first }}
      {{- $port := "" }}
      {{- if eq ( len ( splitList ":" $authority ) ) 2 }}
        {{- $port = splitList ":" $authority | last }}
      {{- else if not ( contains ":" $authority ) }}
        {{- if eq $scheme "https" }}{{- $port = "443" }}{{- end }}
        {{- if eq $scheme "http" }}{{- $port = "80" }}{{- end }}
      {{- end }}
      {{- /* `regexMatch` guards against a templated or otherwise non-numeric
             authority segment, which `int` would silently turn into 0. */}}
      {{- if and $port ( regexMatch "^[0-9]+$" $port ) }}
        {{- $cfg := index $.Values.networkPolicies "grafana-operator" | default dict }}
        {{- $allowed := list }}
        {{- range $entry := ( dig "egress" "external" "ports" list $cfg ) }}
          {{- if kindIs "map" $entry }}
            {{- $allowed = append $allowed ( $entry.port | toString ) }}
          {{- else }}
            {{- $allowed = append $allowed ( $entry | toString ) }}
          {{- end }}
        {{- end }}
        {{- if not ( has $port $allowed ) }}
          {{- $warnings = append $warnings ( printf "connections.grafana.external.url points at port %s, but networkPolicies.grafana-operator.egress.external.ports is %v. The operator would come up healthy and reconcile nothing, because the connection to Grafana is denied rather than refused. Add %s to that list, or an equivalent networkPolicies.grafana-operator.egress.extra rule." $port $allowed $port ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
