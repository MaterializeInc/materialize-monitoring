{{- /* Grafana helpers. */}}

{{- /*
Check if the bundled Grafana server is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.grafana.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.grafana.enabled" }}
  {{- $values := $.Values.grafana | required "grafana is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "managed-grafana" ) ( index $tags "grafana-standalone" ) ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Check if grafana-operator is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.grafanaOperator.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.grafanaOperator.enabled" }}
  {{- $values := index $.Values "grafana-operator" | required "grafana-operator is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "managed-grafana" ) ( index $tags "grafana-operator" ) ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get the grafana-operator namespace.

Usage:
  {{ include "mzmon.grafanaOperator.namespace" $ }}
*/}}
{{- define "mzmon.grafanaOperator.namespace" }}
  {{- $values := index $.Values "grafana-operator" | default dict }}
  {{- $ns := $values.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Namespace the bundled Grafana runs in.

Mirrors the `grafana` subchart's own `grafana.namespace` helper, so the URL we
hand grafana-operator always points at the namespace the Service actually
lands in. The `split-namespace` profile moves Grafana out of the release
namespace, which is exactly the case a hardcoded `.Release.Namespace` breaks.

Usage:
  {{ include "mzmon.grafana.namespace" $ }}
*/}}
{{- define "mzmon.grafana.namespace" }}
  {{- $ns := $.Values.grafana.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Resource name prefix for the bundled Grafana.

Mirrors the `grafana` subchart's `grafana.fullname` helper. `values.yaml` pins
`grafana.fullnameOverride` to `grafana` for a deterministic name, but the
release-name derivation is reproduced here so that clearing the override still
resolves to whatever the subchart actually names its Service — the parent
chart's own `mzmon.fullname` is *not* the same string.

Usage:
  {{ include "mzmon.grafana.fullname" $ }}
*/}}
{{- define "mzmon.grafana.fullname" }}
  {{- $values := $.Values.grafana | required "grafana is missing from values." }}
  {{- if $values.fullnameOverride }}
    {{- $values.fullnameOverride | trunc 63 | trimSuffix "-" }}
  {{- else }}
    {{- $name := $values.nameOverride | default "grafana" }}
    {{- if contains $name $.Release.Name }}
      {{- $.Release.Name | trunc 63 | trimSuffix "-" }}
    {{- else }}
      {{- printf "%s-%s" $.Release.Name $name | trunc 63 | trimSuffix "-" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
HTTP port the bundled Grafana Service listens on.

The `grafana` subchart defaults `service.port` to 80 and routes it to the
container's 3000, so the pod port is the wrong number to dial. Read the value
rather than hardcoding either one.

Usage:
  {{ include "mzmon.grafana.servicePort" $ }}
*/}}
{{- define "mzmon.grafana.servicePort" }}
  {{- dig "service" "port" 80 ( $.Values.grafana | default dict ) }}
{{- end }}

{{- /*
In-cluster base URL for the bundled Grafana.

Usage:
  {{ include "mzmon.grafana.url" $ }}
*/}}
{{- define "mzmon.grafana.url" }}
  {{- printf "http://%s.%s:%s"
    ( include "mzmon.grafana.fullname" $ )
    ( include "mzmon.grafana.namespace" $ )
    ( include "mzmon.grafana.servicePort" $ ) }}
{{- end }}

{{- /*
Name of the Secret holding the bundled Grafana's admin credentials.

With no `grafana.admin.existingSecret`, the subchart creates a Secret named
after itself. Otherwise it consumes the named Secret and creates nothing.

Usage:
  {{ include "mzmon.grafana.adminSecret" $ }}
*/}}
{{- define "mzmon.grafana.adminSecret" }}
  {{- $admin := dig "admin" dict ( $.Values.grafana | default dict ) }}
  {{- $admin.existingSecret | default ( include "mzmon.grafana.fullname" $ ) }}
{{- end }}

{{- /*
Keys within `mzmon.grafana.adminSecret` holding the admin user and password.

`grafana.admin.userKey` / `passwordKey` only apply to an `existingSecret` — the
Secret the subchart generates itself always uses the literal `admin-user` and
`admin-password` keys regardless of those settings.

Usage:
  {{ include "mzmon.grafana.adminUserKey" $ }}
  {{ include "mzmon.grafana.adminPasswordKey" $ }}
*/}}
{{- define "mzmon.grafana.adminUserKey" }}
  {{- $admin := dig "admin" dict ( $.Values.grafana | default dict ) }}
  {{- if $admin.existingSecret }}
    {{- $admin.userKey | default "admin-user" }}
  {{- else }}
    {{- "admin-user" }}
  {{- end }}
{{- end }}

{{- define "mzmon.grafana.adminPasswordKey" }}
  {{- $admin := dig "admin" dict ( $.Values.grafana | default dict ) }}
  {{- if $admin.existingSecret }}
    {{- $admin.passwordKey | default "admin-password" }}
  {{- else }}
    {{- "admin-password" }}
  {{- end }}
{{- end }}

{{- /*
Namespace the `Grafana` resource itself lives in.

In `bundled` mode this follows the Grafana it points at, because
grafana-operator resolves the credential references on a `Grafana` resource
within that resource's *own* namespace — a `SecretKeySelector` carries no
namespace and cannot reach across one. Under the `split-namespace` profile that
keeps the resource next to the Secret the `grafana` subchart owns.

Every other mode has no such Secret to sit beside, so the resource stays with
the rest of this chart's resources.

Usage:
  {{ include "mzmon.grafana.instanceNamespace" $ }}
*/}}
{{- define "mzmon.grafana.instanceNamespace" }}
  {{- if ( eq $.Values.connections.grafana.mode "bundled" ) }}
    {{- include "mzmon.grafana.namespace" $ }}
  {{- else }}
    {{- include "mzmon.namespace" $ }}
  {{- end }}
{{- end }}

{{- /*
Whether Grafana resources need to match an instance outside their own namespace.

`allowCrossNamespaceImport` defaults to false in the CRDs, so a dashboard only
ever finds a `Grafana` in its own namespace unless this is set. Honors an
explicit `dashboards.config.grafana.manifest.allowCrossNamespaceImport`, and
otherwise infers it by comparing where the resources land.

Returns a truthy string when required, empty when not.

Usage:
  {{- if ( include "mzmon.grafana.crossNamespace" $ ) }}
*/}}
{{- define "mzmon.grafana.crossNamespace" }}
  {{- $explicit := $.Values.dashboards.config.grafana.manifest.allowCrossNamespaceImport }}
  {{- if not ( kindIs "invalid" $explicit ) }}
    {{- if $explicit }}true{{ end }}
  {{- else if ne ( include "mzmon.grafana.instanceNamespace" $ ) ( include "mzmon.namespace" $ ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Labels identifying the Grafana instance this chart's Grafana resources target.

These are applied to the `Grafana` resource *and* used as the default
`instanceSelector` for everything the chart pushes into it, so the two sides
cannot drift apart.

The static label matters for more than convenience: grafana-operator treats an
empty `matchLabels` as "match every Grafana instance", not "match none", and it
watches all namespaces by default. Without a non-empty selector the Materialize
dashboards land in every Grafana in the cluster.

`connections.grafana.labels` is merged over the static label. Add to it to
narrow the selector further (e.g. per-release scoping when two
`materialize-monitoring` releases share a cluster); it can also replace the
static value, since both sides are rendered from this same helper.

Usage:
  labels:
    {{- include "mzmon.grafana.instanceLabels" $ | nindent 4 }}
*/}}
{{- define "mzmon.grafana.instanceLabels" }}
  {{- $static := dict "monitoring.materialize.cloud/grafana-instance" "mzmon" }}
  {{- $labels := deepCopy ( $.Values.connections.grafana.labels | default dict ) }}
  {{- merge $labels $static | toYaml }}
{{- end }}

{{- /*
Names of the dashboards `dashboards.selected` resolves to.

Globs the pre-rendered dashboards once so the resources and the install notes
cannot disagree about what was installed. Returns a YAML list.

Usage:
  {{- range $name := include "mzmon.grafana.dashboards" $ | fromYamlArray }}
*/}}
{{- define "mzmon.grafana.dashboards" }}
  {{- $names := list }}
  {{- range $selectPattern := $.Values.dashboards.selected }}
    {{- range $path, $_ := $.Files.Glob ( printf "pre-rendered/dashboards/grafana/%s.yaml" $selectPattern ) }}
      {{- $name := base $path | trimSuffix ".yaml" | lower | replace "_" "-" }}
      {{- if not ( has $name $names ) }}
        {{- $names = append $names $name }}
      {{- end }}
    {{- end }}
  {{- end }}
  {{- $names | toYaml }}
{{- end }}

{{- /*
Whether a bundled datasource should be provisioned.

Honors an explicit `connections.datasources.<name>.enabled`, and otherwise
follows whether the backend it points at is deployed by this release. Pointing
at a backend outside the release means setting `enabled` explicitly.

Returns a truthy string when it should be, empty when not.

Usage:
  {{- if ( include "mzmon.grafana.datasource.enabled" ( dict "root" $ "name" "thanos" ) ) }}
*/}}
{{- define "mzmon.grafana.datasource.enabled" }}
  {{- $root := .root }}
  {{- $ds := index $root.Values.connections.datasources .name }}
  {{- if not $root.Values.connections.datasources.enabled }}
    {{- /* The whole group is off. */}}
  {{- else if not ( kindIs "invalid" $ds.enabled ) }}
    {{- ternary "true" "" $ds.enabled }}
  {{- else }}
    {{- include ( printf "mzmon.%s.enabled" .name ) $root }}
  {{- end }}
{{- end }}

{{- /*
Tenant the Loki datasource reads as, sent as `X-Scope-OrgID`.

The bundled Loki runs `auth_enabled: true`, so reads carry a tenant or fail.
Defaults to the tenant the pipeline writes to, which is only unambiguous while
`pipeline.logging.tenancy.tenantMap` is uniformly `static` — see
`mzmon.grafana.validate`.

An explicit empty string is honored as "send no header"; `null` means default.

Usage:
  {{ include "mzmon.grafana.loki.tenant" $ }}
*/}}
{{- define "mzmon.grafana.loki.tenant" }}
  {{- $tenant := $.Values.connections.datasources.loki.tenant }}
  {{- if kindIs "invalid" $tenant }}
    {{- $.Values.pipeline.logging.tenancy.staticTenant }}
  {{- else }}
    {{- $tenant }}
  {{- end }}
{{- end }}

{{- /*
Validate the Grafana datasource configuration.

Usage:
  {{- $res := include "mzmon.grafana.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $ds := $.Values.connections.datasources }}

  {{- if $ds.enabled }}
    {{- /*
    The dashboards resolve `${metricsDatasource}` to the instance's default
    Prometheus-type datasource. Without one, every panel is silently empty.
    */}}
    {{- if and ( include "mzmon.grafana.datasource.enabled" ( dict "root" $ "name" "thanos" ) ) ( not $ds.thanos.isDefault ) }}
      {{- $warnings = append $warnings "connections.datasources.thanos.isDefault is disabled; the bundled dashboards render empty unless another Prometheus datasource is the default in Grafana." }}
    {{- end }}

    {{- if ( include "mzmon.grafana.datasource.enabled" ( dict "root" $ "name" "loki" ) ) }}
      {{- $tenant := include "mzmon.grafana.loki.tenant" $ }}
      {{- /*
      One datasource carries one tenant header. Any non-static tenancy spreads
      logs across tenants that a single datasource cannot read.
      */}}
      {{- $modes := ( values $.Values.pipeline.logging.tenancy.tenantMap ) | uniq | sortAlpha }}
      {{- if ne ( join "," $modes ) "static" }}
        {{- if kindIs "invalid" $.Values.connections.datasources.loki.tenant }}
          {{- $warnings = append $warnings ( printf "pipeline.logging.tenancy.tenantMap is not uniformly \"static\" (%s), so logs are spread across tenants; the Loki datasource reads only %q. Set connections.datasources.loki.tenant, or add a datasource per tenant." ( join ", " $modes ) $tenant ) }}
        {{- end }}
      {{- else if not $tenant }}
        {{- $warnings = append $warnings "connections.datasources.loki.tenant is empty; reads fail with \"no org id\" unless Loki runs with auth_enabled: false." }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
