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

Pinned to `alertmanager` by `alertmanager.fullnameOverride`, like Loki, Thanos and
Grafana. The fallback mirrors the subchart's own `alertmanager.fullname` for a
deployment that clears the override, including the `contains` rule that
collapses `<release>-alertmanager` to `<release>` when the release name already
contains the chart name.

The two rulers cannot call this: their addresses are rendered inside the Loki and
Thanos subcharts' own `tpl`, where `.Values` is theirs. They name
`alertmanager-headless` literally, and `mzmon.alertmanager.validate` warns when
that and this disagree.

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
  {{- printf "%s.%s.svc.%s:%v" ( include "mzmon.alertmanager.fullname" $ ) ( include "mzmon.alertmanager.namespace" $ ) ( include "mzmon.clusterDomain" $ ) $port }}
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
The receiver that notifies nobody.

It is the root route's receiver, the target of every `suppressed` matrix cell,
and where every alert goes while `alerting.receivers` is empty. An alert that
reaches it still fires and still shows in Alertmanager and Grafana. The name is
reserved: a receiver of that name under `alerting.receivers` fails the render.
*/}}
{{- define "mzmon.alerting.nullReceiver" -}}
mzmon-null
{{- end }}

{{- /*
The receivers under `alerting.receivers` that are actually present.

A receiver removed with `null` — how a values file drops one a profile set — is
still a key with a nil value, so everything iterating receivers goes through here
rather than ranging over the raw map.

Usage:
  {{- $receivers := include "mzmon.alerting.receivers" $ | fromYaml }}
*/}}
{{- define "mzmon.alerting.receivers" }}
  {{- $out := dict }}
  {{- range $name, $r := ( $.Values.alerting | default dict ).receivers | default dict }}
    {{- if kindIs "map" $r }}
      {{- $_ := set $out $name $r }}
    {{- end }}
  {{- end }}
  {{- toYaml $out }}
{{- end }}

{{- /*
The classes one receiver serves, as a list. `class` takes a string or a list.

Usage:
  {{- $classes := include "mzmon.alerting.receiverClasses" $receiver | fromYamlArray }}
*/}}
{{- define "mzmon.alerting.receiverClasses" }}
  {{- $class := .class }}
  {{- if kindIs "string" $class }}
    {{- $class = list $class }}
  {{- else if not ( kindIs "slice" $class ) }}
    {{- $class = list }}
  {{- end }}
  {{- toYaml $class }}
{{- end }}

{{- /*
Every class, mapped to the sorted names of the receivers that serve it.

Usage:
  {{- $byClass := include "mzmon.alerting.classReceivers" $ | fromYaml }}
*/}}
{{- define "mzmon.alerting.classReceivers" }}
  {{- $receivers := include "mzmon.alerting.receivers" $ | fromYaml }}
  {{- $out := dict }}
  {{- range $name := keys $receivers | sortAlpha }}
    {{- range $class := include "mzmon.alerting.receiverClasses" ( index $receivers $name ) | fromYamlArray }}
      {{- if kindIs "string" $class }}
        {{- $_ := set $out $class ( append ( index $out $class | default list ) $name ) }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- toYaml $out }}
{{- end }}

{{- /*
The route delivering one class.

One receiver is a plain route. Several are a parent holding one child per
receiver, each with `continue: true`: an Alertmanager route names exactly one
receiver, and siblings that all continue is how one alert reaches several. The
parent's own receiver is never used, since a child with no matchers matches
everything that reached the parent.

A receiver's `route` options are merged into whichever route names it, so a
pager can repeat faster than a ticket queue without the matrix knowing.

`suppressed`, and a class nobody serves, route to the null receiver. The second
is a render-time error whenever any receiver exists; this only keeps the
rendered configuration valid while the validator reports it.

Usage:
  {{- $route := include "mzmon.alerting.classRoute" ( dict "receivers" $receivers "byClass" $byClass "class" "page" "matchers" ( list "severity=\"critical\"" ) ) | fromYaml }}
*/}}
{{- define "mzmon.alerting.classRoute" }}
  {{- $null := include "mzmon.alerting.nullReceiver" . }}
  {{- $route := dict }}
  {{- with .matchers }}
    {{- $_ := set $route "matchers" . }}
  {{- end }}
  {{- $names := list }}
  {{- if ne .class "suppressed" }}
    {{- $names = index .byClass .class | default list }}
  {{- end }}
  {{- if eq ( len $names ) 0 }}
    {{- $_ := set $route "receiver" $null }}
  {{- else if eq ( len $names ) 1 }}
    {{- $name := first $names }}
    {{- $route = merge $route ( deepCopy ( ( index .receivers $name ).route | default dict ) ) }}
    {{- $_ := set $route "receiver" $name }}
  {{- else }}
    {{- $children := list }}
    {{- range $name := $names }}
      {{- $child := deepCopy ( ( index $.receivers $name ).route | default dict ) }}
      {{- $_ := set $child "receiver" $name }}
      {{- $_ := set $child "continue" true }}
      {{- $children = append $children $child }}
    {{- end }}
    {{- $_ := set $route "receiver" $null }}
    {{- $_ := set $route "routes" $children }}
  {{- end }}
  {{- toYaml $route }}
{{- end }}

{{- /*
The Alertmanager configuration, rendered from `alerting`.

The routing tree, top to bottom:

  1. `alerting.routes.extra`, verbatim and in order, so a specific match wins.
  2. One route per severity in the selected `alerting.matrix` entry, delivering
     that severity's class.
  3. A catch-all delivering the class of `alerting.unknownSeverity`, so an alert
     with no `severity` label, or one the matrix does not name, still reaches
     somebody.

The root route's receiver is the null receiver, and the matrix is left out
entirely while no receiver exists: every alert then lands on the root, which is
the honest rendering of "configured to notify nobody".

Usage:
  {{ include "mzmon.alertmanager.config" $ }}
*/}}
{{- define "mzmon.alertmanager.config" }}
  {{- $alerting := $.Values.alerting | default dict }}
  {{- $null := include "mzmon.alerting.nullReceiver" $ }}
  {{- $receivers := include "mzmon.alerting.receivers" $ | fromYaml }}
  {{- $byClass := include "mzmon.alerting.classReceivers" $ | fromYaml }}

  {{- $routes := list }}
  {{- range ( dig "routes" "extra" list $alerting | default list ) }}
    {{- $routes = append $routes . }}
  {{- end }}
  {{- if $receivers }}
    {{- $row := index ( $alerting.matrix | default dict ) ( $alerting.criticality | default "" ) | default dict }}
    {{- range $severity := keys $row | sortAlpha }}
      {{- $routes = append $routes ( include "mzmon.alerting.classRoute" ( dict
        "receivers" $receivers
        "byClass" $byClass
        "class" ( index $row $severity | toString )
        "matchers" ( list ( printf "severity=%q" $severity ) )
      ) | fromYaml ) }}
    {{- end }}
    {{- $fallback := index $row ( $alerting.unknownSeverity | default "warning" ) | default "suppressed" }}
    {{- $routes = append $routes ( include "mzmon.alerting.classRoute" ( dict
      "receivers" $receivers
      "byClass" $byClass
      "class" ( $fallback | toString )
    ) | fromYaml ) }}
  {{- end }}

  {{- $root := deepCopy ( dig "routes" "root" dict $alerting | default dict ) }}
  {{- $_ := set $root "receiver" $null }}
  {{- if $routes }}
    {{- $_ := set $root "routes" $routes }}
  {{- end }}

  {{- $receiverList := list ( dict "name" $null ) }}
  {{- range $name := keys $receivers | sortAlpha }}
    {{- $body := deepCopy ( ( index $receivers $name ).config | default dict ) }}
    {{- $_ := set $body "name" $name }}
    {{- $receiverList = append $receiverList $body }}
  {{- end }}

  {{- $config := dict
    "route" $root
    "receivers" $receiverList
    "templates" ( list "/etc/alertmanager/config/*.tmpl" )
  }}
  {{- with $alerting.global }}
    {{- $_ := set $config "global" . }}
  {{- end }}
  {{- with $alerting.inhibitRules }}
    {{- $_ := set $config "inhibit_rules" . }}
  {{- end }}
  {{- with $alerting.timeIntervals }}
    {{- $_ := set $config "time_intervals" . }}
  {{- end }}
  {{- toYaml $config }}
{{- end }}

{{- /*
Flatten a values subtree into its scalar leaves, for the validators to scan.

Each leaf carries its dotted `path`, the nearest map `key` above it (list items
inherit their list's key, so `mute_time_intervals: [a, b]` yields two leaves
keyed `mute_time_intervals`), and `ctx`, the nearest enclosing `*_configs` key —
which is what tells a Slack `api_url`, a credential, from an Opsgenie one, a base
URL.

Usage:
  {{- $leaves := include "mzmon.alerting.leaves" ( dict "path" "alerting.global" "node" $.Values.alerting.global ) | fromYamlArray }}
*/}}
{{- define "mzmon.alerting.leaves" }}
  {{- $out := list }}
  {{- if kindIs "map" .node }}
    {{- range $k, $v := .node }}
      {{- $ctx := $.ctx | default "" }}
      {{- if hasSuffix "_configs" $k }}
        {{- $ctx = $k }}
      {{- end }}
      {{- $out = concat $out ( include "mzmon.alerting.leaves" ( dict "path" ( printf "%s.%s" $.path $k ) "key" $k "ctx" $ctx "node" $v ) | fromYamlArray ) }}
    {{- end }}
  {{- else if kindIs "slice" .node }}
    {{- range $i, $v := .node }}
      {{- $out = concat $out ( include "mzmon.alerting.leaves" ( dict "path" ( printf "%s[%d]" $.path $i ) "key" ( $.key | default "" ) "ctx" ( $.ctx | default "" ) "node" $v ) | fromYamlArray ) }}
    {{- end }}
  {{- else if not ( kindIs "invalid" .node ) }}
    {{- $out = append $out ( dict "path" .path "key" ( .key | default "" ) "ctx" ( .ctx | default "" ) "value" .node ) }}
  {{- end }}
  {{- toYaml $out }}
{{- end }}

{{- /*
Validate the bundled Alertmanager: its wiring to the two rulers, its shape, and
the configuration `alerting` renders into it.

Usage:
  {{- $res := include "mzmon.alertmanager.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.alertmanager.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $enabled := include "mzmon.alertmanager.enabled" $ }}
  {{- $thanosRuler := include "mzmon.thanos.ruler.enabled" $ }}
  {{- $lokiRuler := include "mzmon.loki.ruler.enabled" $ }}
  {{- $alerting := $.Values.alerting | default dict }}
  {{- $receivers := include "mzmon.alerting.receivers" $ | fromYaml }}

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

  {{- if and ( not $enabled ) $receivers }}
    {{- $warnings = append $warnings "alerting.receivers is set but the bundled Alertmanager is disabled, so none of alerting.* is rendered. An Alertmanager this chart does not deploy is routed by its own configuration." }}
  {{- end }}

  {{- /* The `cluster` label on alerts. Checked whether or not the bundled
         Alertmanager is on: an Alertmanager shared by several clusters is where
         the label matters most, since identical label sets from two clusters are
         otherwise one alert there. */}}
  {{- if or $thanosRuler $lokiRuler }}
    {{- $clusterName := include "mzmon.clusterName" $ }}
    {{- if not $clusterName }}
      {{- $errors = append $errors "pipeline.env.CLUSTER_NAME is empty. Both rulers stamp it on every alert as `cluster`, and the Thanos ruler refuses an external label with no value." }}
    {{- else if not ( regexMatch "^[A-Za-z0-9][A-Za-z0-9._/-]*$" $clusterName ) }}
      {{- /* The Loki ruler reads it through `-config.expand-env`, which
             substitutes it into the YAML every Loki component parses, unquoted.
             A colon or a space there crash-loops all of Loki, not only the
             ruler. */}}
      {{- $errors = append $errors ( printf "pipeline.env.CLUSTER_NAME is %q. It is substituted unquoted into the configuration every Loki component parses, so it is limited to letters, digits, and . _ / -, starting with a letter or digit." $clusterName ) }}
    {{- end }}
  {{- end }}
  {{- if $thanosRuler }}
    {{- $hasLabel := false }}
    {{- range ( dig "ruler" "extraArgs" list ( $.Values.thanos | default dict ) | default list ) }}
      {{- if hasPrefix "--label=cluster=" ( toString . ) }}
        {{- $hasLabel = true }}
      {{- end }}
    {{- end }}
    {{- if not $hasLabel }}
      {{- $warnings = append $warnings "thanos.ruler.extraArgs no longer carries --label=cluster=\"$(CLUSTER_NAME)\", so alerts from the Thanos ruler carry `cluster` only when their expression kept it. Restate it alongside any flags you add; the list replaces the chart's rather than merging with it." }}
    {{- end }}
  {{- end }}
  {{- if $lokiRuler }}
    {{- $relabelsCluster := false }}
    {{- range ( dig "loki" "rulerConfig" "alert_relabel_configs" list ( $.Values.loki | default dict ) | default list ) }}
      {{- if and ( kindIs "map" . ) ( eq ( toString .target_label ) "cluster" ) }}
        {{- $relabelsCluster = true }}
      {{- end }}
    {{- end }}
    {{- if not $relabelsCluster }}
      {{- $warnings = append $warnings "loki.loki.rulerConfig.alert_relabel_configs no longer sets `cluster`, so alerts from the Loki ruler carry it only when their expression kept it. Restate the chart's entry alongside any you add; the list replaces the chart's rather than merging with it." }}
    {{- end }}
  {{- end }}

  {{- if $enabled }}
    {{- $values := index $.Values "alertmanager" | default dict }}
    {{- $null := include "mzmon.alerting.nullReceiver" $ }}
    {{- $replicas := dig "replicaCount" 1 $values | int }}

    {{- /* --- the workload --------------------------------------------------- */}}

    {{- /* The subchart mounts its own ConfigMap at /etc/alertmanager when this is
           on, which is the parent of the directory the chart's configuration is
           mounted at. Two sources of truth and a nested mount; refuse it. */}}
    {{- if dig "config" "enabled" false $values }}
      {{- $errors = append $errors "alertmanager.config.enabled is true, but the chart renders Alertmanager's configuration itself, from alerting.*. The subchart's ConfigMap would be mounted over /etc/alertmanager, the directory holding the chart's. Configure receivers and routes under alerting, and leave alertmanager.config.enabled false." }}
    {{- else }}
      {{- /* Before the chart rendered the configuration, routing was written under
             the subchart's own `alertmanager.config`. That block is still in the
             merged values — the subchart's defaults guarantee it — and is now read
             by nothing, so a deployment carrying its old routing forward would
             upgrade cleanly and page nobody. The subchart's defaults are one
             receiver with no integrations, a route with no children, and empty
             `global`; anything past that is someone's routing. */}}
      {{- $legacy := $values.config | default dict }}
      {{- $legacyReceivers := $legacy.receivers | default list }}
      {{- $customised := or
        ( gt ( len $legacyReceivers ) 1 )
        ( $legacy.global )
        ( $legacy.inhibit_rules )
        ( $legacy.time_intervals )
        ( $legacy.mute_time_intervals )
        ( dig "route" "routes" nil $legacy )
      }}
      {{- range $legacyReceivers }}
        {{- if and ( kindIs "map" . ) ( gt ( len ( omit . "name" ) ) 0 ) }}
          {{- $customised = true }}
        {{- end }}
      {{- end }}
      {{- if $customised }}
        {{- $errors = append $errors "alertmanager.config carries receivers or routes, but nothing reads it any more: the chart renders Alertmanager's configuration from alerting.*. Move receivers to alerting.receivers (each with a class), routes to alerting.routes.extra, and inhibit_rules, time_intervals and global to alerting.inhibitRules, alerting.timeIntervals and alerting.global. See the Alert Channels page." }}
      {{- end }}
    {{- end }}

    {{- $configFile := dig "extraArgs" "config.file" "" $values }}
    {{- if ne $configFile "/etc/alertmanager/config/alertmanager.yml" }}
      {{- $warnings = append $warnings ( printf "alertmanager.extraArgs.config.file is %q rather than /etc/alertmanager/config/alertmanager.yml, so Alertmanager does not load the configuration rendered from alerting.*. Unset, it loads the image's built-in example, whose one receiver posts to localhost." $configFile ) }}
    {{- end }}

    {{- /* The rulers name `alertmanager-headless` literally, because their
           addresses render inside the Loki and Thanos subcharts where this
           chart's helpers cannot run. A renamed Alertmanager, or a ruler pointed
           at the load-balanced Service, leaves a replica without the alerts its
           peer holds — gossip does not replicate alerts. A warning, since an
           operator may be pointing a ruler at an Alertmanager of their own. */}}
    {{- $headless := printf "%s-headless" ( include "mzmon.alertmanager.fullname" $ ) }}
    {{- if $thanosRuler }}
      {{- $thanosAm := dig "ruler" "alertmanagers" "config" "" ( $.Values.thanos | default dict ) | toString }}
      {{- if not ( contains $headless $thanosAm ) }}
        {{- $warnings = append $warnings ( printf "thanos.ruler.alertmanagers.config does not name %s, the headless Service of the bundled Alertmanager. The ruler has to resolve every replica (dns+%s.<namespace>.svc...:9093), because gossip does not replicate alerts; otherwise a surviving replica may hold none of them." $headless $headless ) }}
      {{- end }}
    {{- end }}
    {{- if $lokiRuler }}
      {{- $lokiAm := dig "loki" "rulerConfig" "alertmanager_url" "" ( $.Values.loki | default dict ) | toString }}
      {{- $discovery := dig "loki" "rulerConfig" "enable_alertmanager_discovery" false ( $.Values.loki | default dict ) }}
      {{- if and $lokiAm ( not ( contains $headless $lokiAm ) ) }}
        {{- $warnings = append $warnings ( printf "loki.loki.rulerConfig.alertmanager_url does not name %s, the headless Service of the bundled Alertmanager. The ruler has to resolve every replica (http://_http._tcp.%s.<namespace>.svc... with enable_alertmanager_discovery), because gossip does not replicate alerts." $headless $headless ) }}
      {{- else if and ( contains "_tcp." $lokiAm ) ( not $discovery ) }}
        {{- $warnings = append $warnings "loki.loki.rulerConfig.alertmanager_url is an SRV name (_http._tcp....) but enable_alertmanager_discovery is off, so the ruler dials it as a hostname, which resolves to nothing, and every alert it evaluates is dropped." }}
      {{- end }}
    {{- end }}

    {{- /* Alertmanager takes its route prefix from the path of --web.external-url
           unless told otherwise, which would move the API both rulers post to,
           both probes, the reloader's /-/reload and the Grafana datasource. */}}
    {{- $baseURL := $values.baseURL | default "" | toString }}
    {{- if $baseURL }}
      {{- $parsed := urlParse $baseURL }}
      {{- if and ( ne ( trimSuffix "/" ( $parsed.path | default "" ) ) "" ) ( not ( dig "extraArgs" "web.route-prefix" "" $values ) ) }}
        {{- $errors = append $errors ( printf "alertmanager.baseURL has a path (%s) and alertmanager.extraArgs.web.route-prefix is unset, so Alertmanager moves every endpoint under that path: the API both rulers post to, its probes, the reloader and the Grafana datasource all miss. Set alertmanager.extraArgs.web.route-prefix: / and have the ingress strip the prefix." $parsed.path ) }}
      {{- end }}
      {{- $grafanaRoot := dig "grafana.ini" "server" "root_url" "" ( $.Values.grafana | default dict ) | toString }}
      {{- if and $grafanaRoot ( eq ( urlParse $grafanaRoot ).host $parsed.host ) }}
        {{- $warnings = append $warnings ( printf "alertmanager.baseURL (%s) points at Grafana's host. Alertmanager builds Alertmanager-UI links from it (/#/alerts, /#/silences/new), which Grafana does not serve, so every link lands on Grafana's home page. Leave it empty unless Alertmanager itself is exposed, and build Grafana links in alerting.templates." $baseURL ) }}
      {{- end }}
    {{- end }}

    {{- if not ( dig "configmapReload" "enabled" false $values ) }}
      {{- $warnings = append $warnings "alertmanager.configmapReload.enabled is false, so a change under alerting.* reaches the running replicas only when they next restart. The subchart's pod-template checksum covers only the ConfigMap it no longer renders." }}
    {{- end }}

    {{- /* HA. Each of these is the single-notifier failure in a different
           costume: one replica, two with nothing stopping a drain from taking
           both, or two with nothing keeping them apart. */}}
    {{- if lt $replicas 2 }}
      {{- $warnings = append $warnings ( printf "alertmanager.replicaCount is %d. A single Alertmanager is lost to an ordinary node drain, holds the only copy of every silence, and cannot report its own absence. Run 2, which gossip and deduplicate." $replicas ) }}
      {{- if not ( dig "persistence" "enabled" true $values ) }}
        {{- $warnings = append $warnings "alertmanager.persistence.enabled is false with a single replica, so every restart loses all silences and the notification log, and the next evaluation re-sends every notification that already went out." }}
      {{- end }}
    {{- else }}
      {{- if not ( $values.podDisruptionBudget ) }}
        {{- $warnings = append $warnings ( printf "alertmanager.replicaCount is %d but alertmanager.podDisruptionBudget is unset, so a drain may evict every replica at once. Set maxUnavailable: 1." $replicas ) }}
      {{- end }}
      {{- if and ( not $values.topologySpreadConstraints ) ( not $values.podAntiAffinity ) ( not ( dig "affinity" "podAntiAffinity" nil $values ) ) }}
        {{- $warnings = append $warnings ( printf "alertmanager.replicaCount is %d but no topologySpreadConstraints or pod anti-affinity keeps the replicas apart. Replicas that share a node or a zone are lost together." $replicas ) }}
      {{- end }}
    {{- end }}

    {{- /* Two replicas gossip; two replicas that cannot gossip send everything
           twice. The NetworkPolicy opens 9094 on both transports, so these only
           fire when someone has narrowed it. TCP alone is the worse half: the
           peers find each other, the cluster forms, and it never converges. */}}
    {{- if gt $replicas 1 }}
      {{- $np := dig "alertmanager" "enabled" nil $.Values.networkPolicies }}
      {{- $npOn := ternary ( $.Values.networkPolicies.enabled ) $np ( typeIs "<nil>" $np ) }}
      {{- $ports := dig "alertmanager" "ingress" "ports" list $.Values.networkPolicies | default list }}
      {{- if and $npOn ( not $ports ) }}
        {{- $warnings = append $warnings ( printf "alertmanager.replicaCount is %d but networkPolicies.alertmanager.ingress.ports is empty, so the gossip port is closed. Peers that cannot gossip do not deduplicate, and every notification goes out once per replica." $replicas ) }}
      {{- else if $npOn }}
        {{- $tcp := false }}
        {{- $udp := false }}
        {{- range $p := $ports }}
          {{- if kindIs "map" $p }}
            {{- if eq ( toString $p.port ) "9094" }}
              {{- if eq ( upper ( $p.protocol | default "TCP" ) ) "UDP" }}
                {{- $udp = true }}
              {{- else }}
                {{- $tcp = true }}
              {{- end }}
            {{- end }}
          {{- else if eq ( toString $p ) "9094" }}
            {{- $tcp = true }}
          {{- end }}
        {{- end }}
        {{- if not ( and $tcp $udp ) }}
          {{- $warnings = append $warnings ( printf "networkPolicies.alertmanager.ingress.ports does not open 9094 on both TCP and UDP (TCP: %t, UDP: %t). The mesh joins over TCP and gossips over UDP; with either missing the replicas never converge, and every notification goes out once per replica." $tcp $udp ) }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if not ( dig "securityContext" "readOnlyRootFilesystem" false $values ) }}
      {{- $warnings = append $warnings "alertmanager.securityContext.readOnlyRootFilesystem is not true. Alertmanager writes only to its storage volume, so a writable root filesystem is attack surface with no use." }}
    {{- end }}

    {{- /* --- the configuration ---------------------------------------------- */}}

    {{- $matrix := $alerting.matrix | default dict }}
    {{- $criticality := $alerting.criticality | default "" | toString }}
    {{- $row := index $matrix $criticality }}
    {{- $byClass := include "mzmon.alerting.classReceivers" $ | fromYaml }}

    {{- if not ( kindIs "map" $row ) }}
      {{- $errors = append $errors ( printf "alerting.criticality is %q, which is not an entry of alerting.matrix (%s)." $criticality ( keys $matrix | sortAlpha | join ", " ) ) }}
    {{- else }}
      {{- $unknown := $alerting.unknownSeverity | default "" | toString }}
      {{- if not ( hasKey $row $unknown ) }}
        {{- $errors = append $errors ( printf "alerting.unknownSeverity is %q, which alerting.matrix.%s does not name (%s). Alerts with no recognised severity would have no class to route to." $unknown $criticality ( keys $row | sortAlpha | join ", " ) ) }}
      {{- end }}
      {{- /* An unroutable severity is found during the incident it should have
             reported, so this is an error, not a drop. Only once a receiver
             exists: with none, everything reaching nobody is already the
             warning below, and one error per cell would bury it. */}}
      {{- if $receivers }}
        {{- range $severity := keys $row | sortAlpha }}
          {{- $class := index $row $severity }}
          {{- if not ( kindIs "string" $class ) }}
            {{- $errors = append $errors ( printf "alerting.matrix.%s.%s must be a class name or `suppressed`." $criticality $severity ) }}
          {{- else if and ( ne $class "suppressed" ) ( not ( hasKey $byClass $class ) ) }}
            {{- $errors = append $errors ( printf "alerting.matrix.%s.%s routes %s alerts to class %q, which no receiver serves. Add %q to a receiver's class under alerting.receivers, or set the cell to `suppressed`." $criticality $severity $severity $class $class ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- if not $receivers }}
      {{- $warnings = append $warnings ( printf "alerting.receivers is empty, so every alert reaches the %s receiver and notifies nobody. Alerts still show in Alertmanager and Grafana. Configure a receiver; see the Alert Channels page." $null ) }}
    {{- end }}

    {{- /* Reserved route keys: the chart owns where a route sends and what it
           matches, and a `route` override setting them would silently rewrite
           the matrix. */}}
    {{- $reservedRouteKeys := list "receiver" "routes" "matchers" "match" "match_re" "continue" }}
    {{- range $key := keys ( dig "routes" "root" dict $alerting | default dict ) | sortAlpha }}
      {{- if has $key $reservedRouteKeys }}
        {{- $errors = append $errors ( printf "alerting.routes.root.%s is set, but the chart owns the root route's %s. Use alerting.routes.extra to add a route." $key $key ) }}
      {{- end }}
    {{- end }}

    {{- $timeRefs := list }}
    {{- range $name := keys $receivers | sortAlpha }}
      {{- $r := index $receivers $name }}
      {{- if eq $name $null }}
        {{- $errors = append $errors ( printf "alerting.receivers.%s uses a reserved name. %s is the chart's receiver that notifies nobody." $name $null ) }}
      {{- end }}
      {{- range $class := include "mzmon.alerting.receiverClasses" $r | fromYamlArray }}
        {{- if not ( kindIs "string" $class ) }}
          {{- $errors = append $errors ( printf "alerting.receivers.%s.class must be a class name or a list of them." $name ) }}
        {{- else if eq $class "suppressed" }}
          {{- $errors = append $errors ( printf "alerting.receivers.%s.class names `suppressed`, which is reserved for severities that notify nobody." $name ) }}
        {{- end }}
      {{- end }}
      {{- $config := $r.config | default dict }}
      {{- if not ( kindIs "map" $config ) }}
        {{- $errors = append $errors ( printf "alerting.receivers.%s.config must be a map of Alertmanager integrations, such as slack_configs." $name ) }}
      {{- else }}
        {{- if not $config }}
          {{- $warnings = append $warnings ( printf "alerting.receivers.%s.config has no integrations, so every alert routed to it notifies nobody." $name ) }}
        {{- end }}
        {{- range $key := keys $config | sortAlpha }}
          {{- if eq $key "name" }}
            {{- $errors = append $errors ( printf "alerting.receivers.%s.config.name is set. The receiver's name is its key under alerting.receivers." $name ) }}
          {{- else if not ( hasSuffix "_configs" $key ) }}
            {{- $errors = append $errors ( printf "alerting.receivers.%s.config.%s is not an Alertmanager integration. Integration keys end in `_configs` — slack_configs, pagerduty_configs, webhook_configs — and Alertmanager rejects anything else, keeping its previous configuration." $name $key ) }}
          {{- else if not ( kindIs "slice" ( index $config $key ) ) }}
            {{- $errors = append $errors ( printf "alerting.receivers.%s.config.%s must be a list." $name $key ) }}
          {{- end }}
        {{- end }}
      {{- end }}
      {{- $override := $r.route | default dict }}
      {{- range $key := keys $override | sortAlpha }}
        {{- if has $key $reservedRouteKeys }}
          {{- $errors = append $errors ( printf "alerting.receivers.%s.route.%s is set, but the matrix decides which alerts reach a receiver. Use alerting.routes.extra for a route of your own." $name $key ) }}
        {{- end }}
      {{- end }}
      {{- $timeRefs = concat $timeRefs ( include "mzmon.alerting.leaves" ( dict "path" ( printf "alerting.receivers.%s.route" $name ) "node" $override ) | fromYamlArray ) }}
    {{- end }}

    {{- /* Extra routes. A receiver named here that does not exist is a
           configuration Alertmanager refuses to load, which on a reload keeps
           the old routing and on a fresh start crash-loops. */}}
    {{- $extra := dig "routes" "extra" list $alerting | default list }}
    {{- $extraLeaves := include "mzmon.alerting.leaves" ( dict "path" "alerting.routes.extra" "node" $extra ) | fromYamlArray }}
    {{- range $leaf := $extraLeaves }}
      {{- if and ( eq $leaf.key "receiver" ) ( ne ( toString $leaf.value ) $null ) ( not ( hasKey $receivers ( toString $leaf.value ) ) ) }}
        {{- $errors = append $errors ( printf "%s names receiver %q, which is not defined under alerting.receivers." $leaf.path ( toString $leaf.value ) ) }}
      {{- end }}
    {{- end }}
    {{- range $i, $route := $extra }}
      {{- if not ( kindIs "map" $route ) }}
        {{- $errors = append $errors ( printf "alerting.routes.extra[%d] must be an Alertmanager route." $i ) }}
      {{- else if and ( not $route.receiver ) ( not $route.continue ) }}
        {{- $warnings = append $warnings ( printf "alerting.routes.extra[%d] names no receiver and does not continue, so an alert it matches that none of its child routes catch goes to %s and notifies nobody. Name a receiver, or set continue: true to fall through to the matrix." $i $null ) }}
      {{- end }}
    {{- end }}
    {{- $timeRefs = concat $timeRefs $extraLeaves }}

    {{- $intervals := list }}
    {{- range ( $alerting.timeIntervals | default list ) }}
      {{- if kindIs "map" . }}
        {{- $intervals = append $intervals ( .name | default "" | toString ) }}
      {{- end }}
    {{- end }}
    {{- range $leaf := $timeRefs }}
      {{- if and ( or ( eq $leaf.key "mute_time_intervals" ) ( eq $leaf.key "active_time_intervals" ) ) ( not ( has ( toString $leaf.value ) $intervals ) ) }}
        {{- $errors = append $errors ( printf "%s names time interval %q, which is not defined under alerting.timeIntervals." $leaf.path ( toString $leaf.value ) ) }}
      {{- end }}
    {{- end }}

    {{- range $name := keys ( $alerting.templates | default dict ) | sortAlpha }}
      {{- if not ( hasSuffix ".tmpl" $name ) }}
        {{- $errors = append $errors ( printf "alerting.templates.%s does not end in .tmpl, so Alertmanager would not load it." $name ) }}
      {{- else if not ( regexMatch "^[-._a-zA-Z0-9]+$" $name ) }}
        {{- $errors = append $errors ( printf "alerting.templates.%s is not a valid file name; it becomes a Secret key." $name ) }}
      {{- end }}
    {{- end }}

    {{- /* Credentials and the files that hold them. */}}
    {{- $credLeaves := include "mzmon.alerting.leaves" ( dict "path" "alerting.global" "node" ( $alerting.global | default dict ) ) | fromYamlArray }}
    {{- range $name := keys $receivers | sortAlpha }}
      {{- $credLeaves = concat $credLeaves ( include "mzmon.alerting.leaves" ( dict "path" ( printf "alerting.receivers.%s.config" $name ) "node" ( ( index $receivers $name ).config | default dict ) ) | fromYamlArray ) }}
    {{- end }}

    {{- /* Fields holding a credential inline, each with a `_file` twin. `api_url`
           is a credential only for Slack, where it is the webhook; for Opsgenie,
           VictorOps and others it is a base URL. */}}
    {{- $credKeys := list
      "password" "credentials" "bearer_token" "client_secret" "secrets"
      "routing_key" "service_key" "api_key" "api_secret" "auth_password" "auth_secret"
      "user_key" "token" "token_id" "bot_token" "webhook_url" "secret_key" "alert_source_token"
      "slack_api_url" "smtp_auth_password" "smtp_auth_secret" "opsgenie_api_key"
      "victorops_api_key" "wechat_api_secret" "rocketchat_token" "rocketchat_token_id"
    }}
    {{- if dig "assertNoInlineCredentials" true $alerting }}
      {{- range $leaf := $credLeaves }}
        {{- if and ( kindIs "string" $leaf.value ) ( ne $leaf.value "" ) }}
          {{- if or ( has $leaf.key $credKeys ) ( and ( eq $leaf.key "api_url" ) ( eq $leaf.ctx "slack_configs" ) ) }}
            {{- $errors = append $errors ( printf "%s is an inline credential. Put it in a Secret mounted through alertmanager.extraSecretMounts and reference it with the field's `_file` variant (%s_file) — values files are committed, diffed and pasted, so a credential in one is a credential published. Set alerting.assertNoInlineCredentials: false to allow it." $leaf.path $leaf.key ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- /* A `*_file` path under no mount fails at send time, as a notification
           that never arrives. The image's own trust store under /etc/ssl is
           the one legitimate unmounted path. */}}
    {{- $mounts := list }}
    {{- range ( concat ( $values.extraSecretMounts | default list ) ( $values.extraVolumeMounts | default list ) ) }}
      {{- if and ( kindIs "map" . ) .mountPath }}
        {{- $mounts = append $mounts ( trimSuffix "/" .mountPath ) }}
      {{- end }}
    {{- end }}
    {{- range $leaf := $credLeaves }}
      {{- if and ( or ( hasSuffix "_file" $leaf.key ) ( eq $leaf.key "files" ) ) ( kindIs "string" $leaf.value ) ( ne $leaf.value "" ) ( not ( hasPrefix "/etc/ssl/" $leaf.value ) ) }}
        {{- $covered := false }}
        {{- range $m := $mounts }}
          {{- if or ( eq $leaf.value $m ) ( hasPrefix ( printf "%s/" $m ) $leaf.value ) }}
            {{- $covered = true }}
          {{- end }}
        {{- end }}
        {{- if not $covered }}
          {{- $errors = append $errors ( printf "%s reads %s, which no volume mounts. Mounted: %s. Add the Secret to alertmanager.extraSecretMounts, or correct the path; otherwise every notification through it fails at send time." $leaf.path $leaf.value ( $mounts | join ", " | default "nothing" ) ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
