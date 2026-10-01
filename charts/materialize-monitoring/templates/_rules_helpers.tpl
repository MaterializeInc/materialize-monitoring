{{- /* Alerting-rule helpers and validators.

The rules themselves are rendered at build time by `mz-monitoring-build
gen-rules` into `pre-rendered/rules/`: one `groups:` document per query-registry
file and ruler, PromQL under `prometheus/` and LogQL under `loki/`, and
`_index.yaml`, which records each rule's engine, its capabilities and whether it
is in the default set. These helpers decide which of those rules install, and
fill in the placeholders the build left for facts only an install knows.
Both engines install as PrometheusRules, labelled with `mzmon.rules.flavorLabel`:
`templates/alerts/prometheusrules.yaml` emits the PromQL ones for the Thanos
ruler's importer, and `templates/alerts/lokirules.yaml` the LogQL ones, which the
alloy-gateway writes into the Loki ruler.
*/}}

{{- /*
The label that says which ruler a PrometheusRule is for: `promql` or `logql`.
The Thanos importer's selector excludes `logql`, and the gateway's
`loki.rules.kubernetes` selects it.
*/}}
{{- define "mzmon.rules.flavorLabel" -}}
mzmon.materialize.cloud/flavor
{{- end }}

{{- /*
Whether this release delivers LogQL rules: it runs the Loki ruler they are for,
and the alloy-gateway whose `loki.rules.kubernetes` writes them into it.

Returns a truthy string if so and an empty one if not.
*/}}
{{- define "mzmon.rules.logRuleSync.enabled" }}
  {{- if and ( include "mzmon.loki.ruler.enabled" $ ) ( include "mzmon.alloyGateway.enabled" $ ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Check if kube-state-metrics is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.
*/}}
{{- define "mzmon.kubeStateMetrics.enabled" }}
  {{- $values := index $.Values "kube-state-metrics" | required "kube-state-metrics is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "cluster-metrics" ) ( index $tags "kube-state-metrics" ) ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Check if node-exporter is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.
*/}}
{{- define "mzmon.nodeExporter.enabled" }}
  {{- $values := index $.Values "node-exporter" | required "node-exporter is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "cluster-metrics" ) ( index $tags "node-exporter" ) ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
The rule index written by gen-rules, parsed.

Usage:
  {{- $index := include "mzmon.rules.index" $ | fromYaml }}
*/}}
{{- define "mzmon.rules.index" }}
  {{- $index := $.Files.Get "pre-rendered/rules/_index.yaml" | required "pre-rendered/rules/_index.yaml cannot be missing/empty; run `make rules`" | fromYaml }}
  {{- /* `fromYaml` reports a parse failure as an `Error` key rather than failing. */}}
  {{- if hasKey $index "Error" }}
    {{- fail ( printf "pre-rendered/rules/_index.yaml does not parse: %s" $index.Error ) }}
  {{- end }}
  {{- $index | toYaml }}
{{- end }}

{{- /*
Capabilities the chart derives from what it deploys, as a YAML list.

Each is present only where the chart both runs the component and collects its
metrics, which in this chart means through the alloy-gateway. Must derive
exactly the capabilities `_index.yaml` marks `derived: true`; the rules
helm-unittest pins the list.

Usage:
  {{- $derived := include "mzmon.rules.derivedCapabilities" $ | fromYamlArray }}
*/}}
{{- define "mzmon.rules.derivedCapabilities" }}
  {{- $caps := list }}
  {{- if ( include "mzmon.alloyGateway.enabled" $ ) }}
    {{- $mz := $.Values.materialize }}
    {{- if and $mz.environmentd.serviceMonitor.enabled $mz.clusterd.serviceMonitor.enabled }}
      {{- $caps = append $caps "materialize" }}
    {{- end }}
    {{- if $mz.environmentdSQL.serviceMonitor.enabled }}
      {{- $caps = append $caps "materialize-sql" }}
    {{- end }}
    {{- if ( index $.Values "materialize-operator" ).serviceMonitor.enabled }}
      {{- $caps = append $caps "materialize-operator" }}
    {{- end }}
    {{- if ( include "mzmon.kubeStateMetrics.enabled" $ ) }}
      {{- $caps = append $caps "kube-state-metrics" }}
    {{- end }}
    {{- /* The gateway scrapes every kubelet's cAdvisor endpoint unconditionally. */}}
    {{- $caps = append $caps "cadvisor" }}
    {{- if ( include "mzmon.nodeExporter.enabled" $ ) }}
      {{- $caps = append $caps "node-exporter" }}
    {{- end }}
    {{- if and ( include "mzmon.loki.enabled" $ ) ( dig "monitoring" "serviceMonitor" "enabled" false $.Values.loki ) }}
      {{- $caps = append $caps "loki" }}
    {{- end }}
    {{- if and ( include "mzmon.alloyAgent.enabled" $ ) ( dig "serviceMonitor" "enabled" false ( index $.Values "alloy-agent" ) ) }}
      {{- $caps = append $caps "alloy" }}
    {{- end }}
  {{- end }}
  {{- $caps | toYaml }}
{{- end }}

{{- /*
Every capability this deployment has: derived, plus `rules.capabilities`.
*/}}
{{- define "mzmon.rules.capabilities" }}
  {{- $derived := include "mzmon.rules.derivedCapabilities" $ | fromYamlArray }}
  {{- concat $derived ( $.Values.rules.capabilities | default list ) | uniq | toYaml }}
{{- end }}

{{- /*
The effective namespace lists, as a dict of lists.

Values cannot reference other values, so the defaults are computed here:
environments come from `rules.namespaces.environment`, else
`materialize.namespaces`, else `materialize-system.namespace`; the operator from
`rules.namespaces.operator`, else `materialize-operator.namespace`.
*/}}
{{- define "mzmon.rules.namespaceLists" }}
  {{- $ns := $.Values.rules.namespaces | default dict }}
  {{- $environment := $ns.environment | default list }}
  {{- if not $environment }}
    {{- $environment = $.Values.materialize.namespaces | default list }}
  {{- end }}
  {{- if not $environment }}
    {{- $system := index $.Values "materialize-system" | default dict }}
    {{- with $system.namespace }}
      {{- $environment = list . }}
    {{- end }}
  {{- end }}
  {{- $operator := $ns.operator | default list }}
  {{- if not $operator }}
    {{- with ( index $.Values "materialize-operator" ).namespace }}
      {{- $operator = list . }}
    {{- end }}
  {{- end }}
  {{- dict "environment" $environment "operator" $operator "exclude" ( $ns.exclude | default list ) | toYaml }}
{{- end }}

{{- /*
Placeholder token -> install-time value, as a dict.

A namespace list or workload tier renders as a regex alternation. An empty one
renders as `a^`, which matches no string, and never as an empty string:
`namespace=~""` would match nothing without saying so, and `namespace!~""` would
drop every series that has a namespace.
*/}}
{{- define "mzmon.rules.substitutions" }}
  {{- $lists := include "mzmon.rules.namespaceLists" $ | fromYaml }}
  {{- $alternation := dict }}
  {{- range $key, $list := $lists }}
    {{- $_ := set $alternation $key ( ternary ( join "|" $list ) "a^" ( gt ( len $list ) 0 ) ) }}
  {{- end }}
  {{- $workloads := $.Values.rules.infraWorkloads | default dict }}
  {{- range $tier := list "core" "important" "nonessential" "daemonset" }}
    {{- $list := get $workloads $tier | default list }}
    {{- $_ := set $alternation $tier ( ternary ( join "|" $list ) "a^" ( gt ( len $list ) 0 ) ) }}
  {{- end }}
  {{- $prefix := ternary "v2_mz_" "mz_" ( eq ( $.Values.materialize.deploymentMode | toString ) "cloud" ) }}
  {{- dict
      "__mzmon_environment_namespaces__" $alternation.environment
      "__mzmon_operator_namespaces__" $alternation.operator
      "__mzmon_excluded_namespaces__" $alternation.exclude
      "__mzmon_core_workloads__" $alternation.core
      "__mzmon_important_workloads__" $alternation.important
      "__mzmon_nonessential_workloads__" $alternation.nonessential
      "__mzmon_daemonset_workloads__" $alternation.daemonset
      "__mzmon_sql_prefix__" $prefix
    | toYaml }}
{{- end }}

{{- /*
The names of the rules that install, as a YAML list.

A rule installs when rules are enabled, every capability it requires is
present, it is in the default set or named (or its group named, or `*`) in
`rules.selected`, and it is not named in `rules.disabled`. A LogQL rule also
needs this release to deliver it (`mzmon.rules.logRuleSync.enabled`).
*/}}
{{- define "mzmon.rules.installed" }}
  {{- $installed := list }}
  {{- if $.Values.rules.enabled }}
    {{- $index := include "mzmon.rules.index" $ | fromYaml }}
    {{- $caps := include "mzmon.rules.capabilities" $ | fromYamlArray }}
    {{- $selected := $.Values.rules.selected | default list }}
    {{- $disabled := $.Values.rules.disabled | default list }}
    {{- $logRuleSync := include "mzmon.rules.logRuleSync.enabled" $ }}
    {{- range $name, $rule := $index.rules }}
      {{- $applies := true }}
      {{- range $rule.requires }}
        {{- if not ( has . $caps ) }}
          {{- $applies = false }}
        {{- end }}
      {{- end }}
      {{- if and ( eq ( $rule.engine | default "promql" ) "logql" ) ( not $logRuleSync ) }}
        {{- $applies = false }}
      {{- end }}
      {{- $chosen := or $rule.enabledByDefault ( has $name $selected ) ( has $rule.group $selected ) ( has "*" $selected ) }}
      {{- if and $applies $chosen ( not ( has $name $disabled ) ) }}
        {{- $installed = append $installed $name }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $installed | toYaml }}
{{- end }}

{{- /*
One pre-rendered rule file as it installs, as a YAML dict with `groups`.

Each file has its placeholders filled from values, its rules filtered to the
ones that install (`mzmon.rules.installed`) with `rules.overrides` applied, and
any group left empty dropped. A file with nothing left returns no groups. Both
engines' files go through this, so a rule installs, and is overridden, the same
way whichever ruler evaluates it.

**The file is read as data and never passed through `tpl`.** Its annotations
may carry `{{ $labels.namespace }}`, which is Prometheus template syntax for the
ruler to expand, and `tpl` would try to expand it here.

Usage:
  {{- $doc := include "mzmon.rules.groups" ( dict "root" $ "path" $path ) | fromYaml }}
*/}}
{{- define "mzmon.rules.groups" }}
  {{- $root := .root }}
  {{- $path := .path }}
  {{- $index := include "mzmon.rules.index" $root | fromYaml }}
  {{- $installed := include "mzmon.rules.installed" $root | fromYamlArray }}
  {{- $substitutions := include "mzmon.rules.substitutions" $root | fromYaml }}
  {{- $overrides := $root.Values.rules.overrides | default dict }}
  {{- $raw := $root.Files.Get $path }}
  {{- /* `placeholders` is longest first, so no token is a prefix of one replaced later. */}}
  {{- range $token := $index.placeholders }}
    {{- $value := get $substitutions $token | required ( printf "no install-time value for rule placeholder %s" $token ) }}
    {{- $raw = replace $token $value $raw }}
  {{- end }}
  {{- with regexFind "__mzmon_[a-z_]+__" $raw }}
    {{- fail ( printf "%s carries %s, a placeholder this chart does not know how to fill. Regenerate it with `make rules`." $path . ) }}
  {{- end }}
  {{- $doc := $raw | fromYaml }}
  {{- if hasKey $doc "Error" }}
    {{- fail ( printf "%s does not parse: %s" $path $doc.Error ) }}
  {{- end }}
  {{- $groups := list }}
  {{- range $group := $doc.groups }}
    {{- $rules := list }}
    {{- range $rule := $group.rules }}
      {{- if has $rule.alert $installed }}
        {{- /* `rules.overrides` changes one rule's `for` and labels, never its expression. */}}
        {{- with get $overrides $rule.alert }}
          {{- $rule = deepCopy $rule }}
          {{- with .for }}
            {{- $_ := set $rule "for" . }}
          {{- end }}
          {{- with .labels }}
            {{- $_ := set $rule "labels" ( mergeOverwrite ( deepCopy $rule.labels ) . ) }}
          {{- end }}
        {{- end }}
        {{- $rules = append $rules $rule }}
      {{- end }}
    {{- end }}
    {{- if $rules }}
      {{- $groups = append $groups ( dict "name" $group.name "rules" $rules ) }}
    {{- end }}
  {{- end }}
  {{- dict "groups" $groups | toYaml }}
{{- end }}

{{- /*
The Loki tenants the log rules install for, as a YAML list.

A Loki rule group belongs to one tenant, and the ruler does not evaluate across
tenants, so the gateway writes the log rules into each tenant whose logs they
should read. `rules.logTenants` lists them; empty means the tenant the pipeline
writes every class to under `static` tenancy,
`pipeline.logging.tenancy.staticTenant`.

Usage:
  {{- $tenants := include "mzmon.rules.logTenants" $ | fromYamlArray }}
*/}}
{{- define "mzmon.rules.logTenants" }}
  {{- $tenants := $.Values.rules.logTenants | default list }}
  {{- if not $tenants }}
    {{- $tenants = list $.Values.pipeline.logging.tenancy.staticTenant }}
  {{- end }}
  {{- $tenants | uniq | toYaml }}
{{- end }}

{{- /*
Validation for the rules surface.
*/}}
{{- define "mzmon.rules.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.rules | default dict }}

  {{- $config := $.Values.config | default dict }}
  {{- if or ( hasKey $config "rules" ) ( hasKey $config "alerts" ) }}
    {{- $warnings = append $warnings "config.rules and config.alerts were never read by any template and have been removed; which alerting rules install is configured under `rules`." }}
  {{- end }}

  {{- $mode := $.Values.materialize.deploymentMode | toString }}
  {{- if not ( has $mode ( list "self-managed" "cloud" ) ) }}
    {{- $errors = append $errors ( printf "materialize.deploymentMode is %q; it must be `self-managed` or `cloud`, because it decides the metric prefix the alerting rules read." $mode ) }}
  {{- end }}

  {{- $index := include "mzmon.rules.index" $ | fromYaml }}
  {{- $known := keys $index.capabilities | sortAlpha }}
  {{- range ( $values.capabilities | default list ) }}
    {{- if not ( has . $known ) }}
      {{- $errors = append $errors ( printf "rules.capabilities names %q, which is not a capability. Known capabilities: %s." . ( join ", " $known ) ) }}
    {{- end }}
  {{- end }}

  {{- $names := keys $index.rules }}
  {{- $groups := list }}
  {{- range $index.rules }}
    {{- $groups = append $groups .group }}
  {{- end }}
  {{- range ( $values.selected | default list ) }}
    {{- if not ( or ( eq . "*" ) ( has . $names ) ( has . $groups ) ) }}
      {{- $errors = append $errors ( printf "rules.selected names %q, which is neither an alert nor a rule group. A misspelt name would otherwise select nothing without saying so." . ) }}
    {{- end }}
  {{- end }}
  {{- range ( $values.disabled | default list ) }}
    {{- if not ( has . $names ) }}
      {{- $errors = append $errors ( printf "rules.disabled names %q, which is not an alert. A misspelt name would otherwise leave the alert installed." . ) }}
    {{- end }}
  {{- end }}

  {{- $lists := include "mzmon.rules.namespaceLists" $ | fromYaml }}
  {{- range $key, $list := $lists }}
    {{- range $list }}
      {{- if not ( regexMatch "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$" ( toString . ) ) }}
        {{- $errors = append $errors ( printf "rules.namespaces.%s contains %q, which is not a Kubernetes namespace name." $key ( toString . ) ) }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- range $name, $override := ( $values.overrides | default dict ) }}
    {{- if not ( has $name $names ) }}
      {{- $errors = append $errors ( printf "rules.overrides names %q, which is not an alert. A misspelt name would otherwise change nothing without saying so." $name ) }}
    {{- else if not ( kindIs "map" $override ) }}
      {{- $errors = append $errors ( printf "rules.overrides.%s must be a map with `for` and/or `labels`." $name ) }}
    {{- else }}
      {{- range $key, $_ := $override }}
        {{- if not ( has $key ( list "for" "labels" ) ) }}
          {{- $errors = append $errors ( printf "rules.overrides.%s.%s is not something an override can change; it takes `for` and `labels`." $name $key ) }}
        {{- end }}
      {{- end }}
      {{- /* Presence, not truthiness: `for: 0` or `labels: ""` must fail, not vanish. */}}
      {{- if hasKey $override "for" }}
        {{- $for := get $override "for" }}
        {{- if not ( and ( kindIs "string" $for ) ( regexMatch "^([0-9]+(ms|s|m|h|d|w|y))+$" ( toString $for ) ) ) }}
          {{- $errors = append $errors ( printf "rules.overrides.%s.for is %s, which is not a Prometheus duration such as `30m`, `2h` or `0s`. Remove the key to keep the rule's own." $name ( toJson $for ) ) }}
        {{- end }}
      {{- end }}
      {{- if and ( hasKey $override "labels" ) ( not ( kindIs "map" ( get $override "labels" ) ) ) }}
        {{- $errors = append $errors ( printf "rules.overrides.%s.labels is %s; it must be a map of label names to string values." $name ( toJson ( get $override "labels" ) ) ) }}
      {{- end }}
      {{- range $label, $value := ( ternary ( get $override "labels" ) dict ( kindIs "map" ( get $override "labels" ) ) ) }}
        {{- if not ( regexMatch "^[a-zA-Z_][a-zA-Z0-9_]*$" $label ) }}
          {{- $errors = append $errors ( printf "rules.overrides.%s.labels.%s is not a valid label name." $name $label ) }}
        {{- else if not ( kindIs "string" $value ) }}
          {{- $errors = append $errors ( printf "rules.overrides.%s.labels.%s must be a string." $name $label ) }}
        {{- else if and ( eq $label "severity" ) ( not ( has $value ( list "critical" "warning" "notice" ) ) ) }}
          {{- $errors = append $errors ( printf "rules.overrides.%s.labels.severity is %q; the routing presets route critical, warning and notice." $name $value ) }}
        {{- else if and ( eq $label "audience" ) ( not ( has $value ( list "platform" "workload" ) ) ) }}
          {{- $errors = append $errors ( printf "rules.overrides.%s.labels.audience is %q; it is platform or workload." $name $value ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* A tenant becomes a directory name under the rules sidecar's folder and the
         `X-Scope-OrgID` the ruler queries as, so it has to be a valid Loki tenant
         id, which also keeps it one path segment. */}}
  {{- /* The effective list, so the staticTenant fallback is checked too. */}}
  {{- $tenantSource := ternary "rules.logTenants" "pipeline.logging.tenancy.staticTenant (the default for rules.logTenants)" ( gt ( len ( $values.logTenants | default list ) ) 0 ) }}
  {{- range ( include "mzmon.rules.logTenants" $ | fromYamlArray ) }}
    {{- if not ( and ( kindIs "string" . ) ( regexMatch "^[a-zA-Z0-9!._*'()-]{1,150}$" ( toString . ) ) ( not ( has ( toString . ) ( list "." ".." ) ) ) ) }}
      {{- $errors = append $errors ( printf "%s contains %s, which is not a Loki tenant id: up to 150 letters, digits and the characters ! - _ . * ' ( ), and not `.` or `..`." $tenantSource ( toJson . ) ) }}
    {{- end }}
  {{- end }}

  {{- /* Entries land inside a PromQL string literal in a YAML block scalar. */}}
  {{- $workloads := $values.infraWorkloads | default dict }}
  {{- range $tier, $list := $workloads }}
    {{- if not ( has $tier ( list "core" "important" "nonessential" "daemonset" ) ) }}
      {{- $errors = append $errors ( printf "rules.infraWorkloads.%s is not a tier. The tiers are core, important, nonessential and daemonset." $tier ) }}
    {{- else }}
      {{- range ( $list | default list ) }}
        {{- if not ( regexMatch "^[^\"\\\\\\s]+$" ( toString . ) ) }}
          {{- $errors = append $errors ( printf "rules.infraWorkloads.%s contains %q. An entry is a regex fragment matched against a workload name, and may not be empty or contain a quote, a backslash or whitespace." $tier ( toString . ) ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- if not $lists.environment }}
    {{- $warnings = append $warnings "No Materialize environment namespace is known (rules.namespaces.environment, materialize.namespaces and materialize-system.namespace are all empty), so rules scoped to an environment's pods match nothing." }}
  {{- end }}

  {{- if $values.enabled }}
    {{- if not ( include "mzmon.thanos.ruler.enabled" $ ) }}
      {{- $warnings = append $warnings "rules.enabled renders PrometheusRule resources, but thanos.ruler is off, so nothing in this release evaluates the PromQL rules. That is only correct if another component in the cluster imports PrometheusRules." }}
    {{- end }}

    {{- $caps := include "mzmon.rules.capabilities" $ | fromYamlArray }}
    {{- $installedNames := include "mzmon.rules.installed" $ | fromYamlArray }}
    {{- range $name, $_ := ( $values.overrides | default dict ) }}
      {{- if and ( has $name $names ) ( not ( has $name $installedNames ) ) }}
        {{- $warnings = append $warnings ( printf "rules.overrides names %q, which is not installed, so the override has no effect. Select it in rules.selected if it should install." $name ) }}
      {{- end }}
    {{- end }}
    {{- $logRuleSync := include "mzmon.rules.logRuleSync.enabled" $ }}
    {{- range $name := ( $values.selected | default list ) }}
      {{- with ( get $index.rules $name ) }}
        {{- if and ( eq ( .engine | default "promql" ) "logql" ) ( not $logRuleSync ) }}
          {{- $warnings = append $warnings ( printf "rules.selected names %q, a LogQL rule, but this release does not run both the Loki ruler it is for and the alloy-gateway that delivers it, so it is not installed." $name ) }}
        {{- end }}
        {{- $missing := list }}
        {{- range .requires }}
          {{- if not ( has . $caps ) }}
            {{- $missing = append $missing . }}
          {{- end }}
        {{- end }}
        {{- if $missing }}
          {{- $warnings = append $warnings ( printf "rules.selected names %q, which requires %s that this deployment does not have, so it is not installed. Add %s to rules.capabilities if the deployment does." $name ( join ", " $missing ) ( join ", " $missing ) ) }}
        {{- end }}
      {{- end }}
    {{- end }}

    {{- /* One Loki rule group reads one tenant. Under anything but uniformly
           static tenancy the logs are spread across tenants the chart cannot
           enumerate, so the rules cover only the ones named. */}}
    {{- $logRules := list }}
    {{- range $name := $installedNames }}
      {{- if eq ( ( get $index.rules $name ).engine | default "promql" ) "logql" }}
        {{- $logRules = append $logRules $name }}
      {{- end }}
    {{- end }}
    {{- if and $logRules ( not $values.logTenants ) }}
      {{- $modes := values ( $.Values.pipeline.logging.tenancy.tenantMap | default dict ) | uniq | sortAlpha }}
      {{- if ne ( join "," $modes ) "static" }}
        {{- $warnings = append $warnings ( printf "pipeline.logging.tenancy.tenantMap is not uniformly \"static\" (%s), so logs are spread across Loki tenants, and the log-derived rules are written only into %q. List every tenant whose logs they should read in rules.logTenants." ( join ", " $modes ) $.Values.pipeline.logging.tenancy.staticTenant ) }}
      {{- end }}
    {{- end }}

    {{- /* The Thanos importer reads every PrometheusRule it selects as PromQL. A
           LogQL one fails its reload, and it keeps evaluating the old set while
           reporting nothing. The subchart joins the selector map into
           `key=value`, so a `<label>!` key renders `<label>!=logql`. */}}
    {{- if and $logRules ( include "mzmon.thanos.ruler.enabled" $ ) ( dig "ruler" "autoImportPrometheusRules" "enabled" false ( $.Values.thanos | default dict ) ) }}
      {{- $flavor := include "mzmon.rules.flavorLabel" $ }}
      {{- $selector := dig "ruler" "autoImportPrometheusRules" "labelSelector" dict ( $.Values.thanos | default dict ) | default dict }}
      {{- $excludes := eq ( get $selector ( printf "%s!" $flavor ) | toString ) "logql" }}
      {{- $onlyPromql := eq ( get $selector $flavor | toString ) "promql" }}
      {{- if not ( or $excludes $onlyPromql ) }}
        {{- $errors = append $errors ( printf "thanos.ruler.autoImportPrometheusRules.labelSelector (%s) does not leave out the log-derived rules, which are PrometheusRules labelled %s: logql. The Thanos ruler would read them as PromQL, fail its reload, and keep evaluating the rules it had. Keep `%s!: logql` in the selector, or select `%s: promql`." ( toJson $selector ) $flavor $flavor $flavor ) }}
      {{- end }}
    {{- end }}

    {{- /* A destination filtering below a rule's metrics starves the rule. */}}
    {{- $floor := dig "pipeline" "metrics" "gateway" "destination" "prometheusRemoteWrite" "thanos" "minMetricImportance" "all" ( $.Values | toYaml | fromYaml ) | toString }}
    {{- $rank := dict "essential" 0 "recommended" 1 "extended" 2 "diagnostic" 3 "all" 4 }}
    {{- if hasKey $rank $floor }}
      {{- range $name := ( include "mzmon.rules.installed" $ | fromYamlArray ) }}
        {{- $rule := get $index.rules $name }}
        {{- with $rule.minImportance }}
          {{- if gt ( get $rank . | int ) ( get $rank $floor | int ) }}
            {{- $warnings = append $warnings ( printf "Alert %q reads a metric in the %q tier, but the thanos destination keeps only %q and above, so the rule cannot see it." $name . $floor ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
