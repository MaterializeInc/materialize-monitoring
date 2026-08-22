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
Grafana's own `grafana.ini` config, as a dict.

The key has a dot in it, so it is not reachable with `dig`'s path form.

Usage:
  {{- $ini := include "mzmon.grafana.ini" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.ini" }}
  {{- index ( $.Values.grafana | default dict ) "grafana.ini" | default dict | toYaml }}
{{- end }}

{{- /*
One section of `grafana.ini`, as a dict, safe when it is absent or explicitly
null.

`dig`'s multi-key form walks the intermediate levels with a type assertion, so
`unified_alerting: null` — which is how a values file *removes* a section a
profile set — errors the render rather than reading as absent. Every section
lookup goes through here instead.

Usage:
  {{- $db := include "mzmon.grafana.iniSection" ( dict "root" $ "name" "database" ) | fromYaml }}
*/}}
{{- define "mzmon.grafana.iniSection" }}
  {{- $ini := include "mzmon.grafana.ini" .root | fromYaml }}
  {{- $section := index $ini .name }}
  {{- if kindIs "map" $section }}
    {{- $section | toYaml }}
  {{- else }}
    {{- dict | toYaml }}
  {{- end }}
{{- end }}

{{- /*
One sub-map of the `grafana` subchart's values, safe when absent or null.

Same hazard as `mzmon.grafana.iniSection`, one level up.

Usage:
  {{- $svc := include "mzmon.grafana.section" ( dict "root" $ "name" "service" ) | fromYaml }}
*/}}
{{- define "mzmon.grafana.section" }}
  {{- $values := .root.Values.grafana | default dict }}
  {{- $section := index $values .name }}
  {{- if kindIs "map" $section }}
    {{- $section | toYaml }}
  {{- else }}
    {{- dict | toYaml }}
  {{- end }}
{{- end }}

{{- /*
Whether Grafana's state lives in a database more than one replica can share.

SQLite — the default, on `emptyDir` or on a PersistentVolume — tolerates exactly
one writer, so it is what pins the bundled Grafana to a single replica.

Returns a truthy string when the backend is shared, empty when not.

Usage:
  {{- if ( include "mzmon.grafana.sharedDatabase" $ ) }}
*/}}
{{- define "mzmon.grafana.sharedDatabase" }}
  {{- $db := include "mzmon.grafana.iniSection" ( dict "root" $ "name" "database" ) | fromYaml }}
  {{- $type := $db.type | default "" }}
  {{- if has $type ( list "postgres" "mysql" ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
The most Grafana replicas this configuration can run at once.

`replicas` is the floor; an enabled HPA raises the ceiling to its `maxReplicas`.
Correctness checks have to read the ceiling, not the floor — an HPA that scales
a SQLite-backed Grafana out is just as wrong as setting `replicas: 3`, and it
happens later, under load, rather than at install.

Usage:
  {{- $max := int ( include "mzmon.grafana.maxReplicas" $ ) }}
*/}}
{{- define "mzmon.grafana.maxReplicas" }}
  {{- $replicas := int ( dig "replicas" 1 ( $.Values.grafana | default dict ) ) }}
  {{- $auto := include "mzmon.grafana.section" ( dict "root" $ "name" "autoscaling" ) | fromYaml }}
  {{- if $auto.enabled }}
    {{- $max := int ( $auto.maxReplicas | default 1 ) }}
    {{- if gt $max $replicas }}
      {{- $replicas = $max }}
    {{- end }}
  {{- end }}
  {{- $replicas }}
{{- end }}

{{- /*
Whether Grafana is reachable from outside the cluster.

An Ingress or any Service type other than `ClusterIP` puts Grafana on a network
this chart cannot characterize, which is the point at which its state, its TLS
posture, and its authentication stop being demo concerns.

Returns a truthy string when exposed, empty when not.

Usage:
  {{- if ( include "mzmon.grafana.exposed" $ ) }}
*/}}
{{- define "mzmon.grafana.exposed" }}
  {{- $service := include "mzmon.grafana.section" ( dict "root" $ "name" "service" ) | fromYaml }}
  {{- $ingress := include "mzmon.grafana.section" ( dict "root" $ "name" "ingress" ) | fromYaml }}
  {{- if or $ingress.enabled ( ne ( $service.type | default "ClusterIP" ) "ClusterIP" ) }}
    {{- "true" }}
  {{- end }}
{{- end }}

{{- /*
Validate the Grafana configuration.

Usage:
  {{- $res := include "mzmon.grafana.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- /*
  These read the `grafana` subchart's values, so they only mean anything when
  this release actually deploys it. An `external` or `operator` instance is
  configured somewhere this chart cannot see.
  */}}
  {{- if ( include "mzmon.grafana.enabled" $ ) }}
    {{- range $name := list "persistence" "exposure" "disruption" "security" "networkPolicy" }}
      {{- $res := include ( printf "mzmon.grafana.validate.%s" $name ) $ | fromYaml }}
      {{- $errors = concat $errors $res.errors | default list }}
      {{- $warnings = concat $warnings $res.warnings | default list }}
    {{- end }}
  {{- end }}

  {{- $res := include "mzmon.grafana.validate.datasources" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate that Grafana's own state survives the pod it runs in.

Grafana keeps users, service accounts and tokens, annotations, dashboard
versions and permissions, preferences, and alert-rule state in a database of its
own — separate from the observability data in Thanos and Loki, which is never at
risk here. The dashboards this chart installs are re-pushed by grafana-operator
every `resyncPeriod`, so they come back on their own. Everything a human created
through the UI does not.

Usage:
  {{- $res := include "mzmon.grafana.validate.persistence" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate.persistence" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.grafana | default dict }}
  {{- $db := include "mzmon.grafana.iniSection" ( dict "root" $ "name" "database" ) | fromYaml }}
  {{- $shared := include "mzmon.grafana.sharedDatabase" $ }}
  {{- $replicas := int ( include "mzmon.grafana.maxReplicas" $ ) }}
  {{- $persistence := include "mzmon.grafana.section" ( dict "root" $ "name" "persistence" ) | fromYaml }}
  {{- $modes := $persistence.accessModes | default ( list "ReadWriteOnce" ) }}
  {{- $rwo := not ( or ( has "ReadWriteMany" $modes ) ( has "ReadWriteOncePod" $modes ) ) }}

  {{- if and ( gt $replicas 1 ) ( not $shared ) }}
    {{- $errors = append $errors ( printf "Grafana can run up to %d replicas but grafana.ini.database is not set, so each one carries its own SQLite file. Users, service accounts, annotations, and preferences would differ depending on which pod answered the request. Point grafana.ini.database at PostgreSQL (see the grafana-postgres profile), or run a single replica." $replicas ) }}
  {{- end }}

  {{- if $persistence.enabled }}
    {{- if and ( gt $replicas 1 ) $rwo }}
      {{- $errors = append $errors ( printf "grafana.persistence is enabled with %s access but Grafana can run up to %d replicas. Only one pod can attach the volume, so the rest stay Pending. Use ReadWriteMany, or drop the volume and use PostgreSQL." ( join "/" $modes ) $replicas ) }}
    {{- end }}
    {{- $strategy := ( include "mzmon.grafana.section" ( dict "root" $ "name" "deploymentStrategy" ) | fromYaml ).type | default "RollingUpdate" }}
    {{- $sts := or ( dig "useStatefulSet" false $values ) ( has ( $persistence.type | default "pvc" ) ( list "sts" "StatefulSet" "statefulset" ) ) }}
    {{- if and $rwo ( not $sts ) ( ne $strategy "Recreate" ) }}
      {{- $errors = append $errors ( printf "grafana.persistence is enabled on a %s volume but grafana.deploymentStrategy.type is %q. A rolling update deadlocks: the replacement pod waits for a volume the outgoing pod has not released, and the outgoing pod is not terminated until the replacement is Ready. Set grafana.deploymentStrategy.type=Recreate, which accepts a short outage on every upgrade in exchange for upgrades that finish." ( join "/" $modes ) $strategy ) }}
    {{- end }}
  {{- else if not $shared }}
    {{- /*
    Losing UI state every restart is tolerable while Grafana is a bundled extra
    reached through `port-forward`. It stops being tolerable the moment Grafana
    is the primary interface to the stack, so warn at exactly that point rather
    than on every demo install.
    */}}
    {{- if ( include "mzmon.grafana.exposed" $ ) }}
      {{- $warnings = append $warnings "Grafana is reachable from outside the cluster but stores its state in SQLite on an emptyDir, so every user, service-account token, annotation, and preference created through the UI is lost on the next restart, upgrade, or reschedule. Apply the grafana-postgres profile, or grafana-pvc if no database is available." }}
    {{- end }}
  {{- end }}

  {{- if $shared }}
    {{- $host := $db.host | default "" }}
    {{- if not $host }}
      {{- $errors = append $errors ( printf "grafana.ini.database.type is %q but no host is set, so Grafana falls back to localhost and crash-loops on connect. Set grafana.ini.database.host, including the port." ( $db.type | default "" ) ) }}
    {{- else if contains "<" $host }}
      {{- $errors = append $errors ( printf "grafana.ini.database.host is still the profile placeholder (%s). Set it to the real database host, including the port." $host ) }}
    {{- else if not ( contains ":" $host ) }}
      {{- $warnings = append $warnings ( printf "grafana.ini.database.host is %q with no port. Grafana does not default one for a bare host and the connection fails; write it as host:5432." $host ) }}
    {{- end }}
    {{- $sslMode := $db.ssl_mode | default "" }}
    {{- if or ( not $sslMode ) ( eq $sslMode "disable" ) }}
      {{- $warnings = append $warnings ( printf "grafana.ini.database.ssl_mode is %q, so Grafana's credentials and every row it reads cross the network unencrypted. Prefer verify-full with ca_cert_path; require encrypts but does not authenticate the server." ( $sslMode | default "unset" ) ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate how Grafana is reached.

Follows the load-balancer convention the Terraform modules already enforce:
internal by default, and public only against an explicit allowlist. Where the
chart can see the allowlist it requires one; where it cannot — an Ingress, whose
scope lives in the controller's annotations — it checks the things it can see
instead: TLS, `root_url`, and whether anything but the admin password stands
between the internet and the data.

Usage:
  {{- $res := include "mzmon.grafana.validate.exposure" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate.exposure" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $ini := include "mzmon.grafana.ini" $ | fromYaml }}
  {{- $ingress := include "mzmon.grafana.section" ( dict "root" $ "name" "ingress" ) | fromYaml }}
  {{- $service := include "mzmon.grafana.section" ( dict "root" $ "name" "service" ) | fromYaml }}
  {{- $svcType := $service.type | default "ClusterIP" }}
  {{- $acked := $.Values.connections.grafana.allowPublicAccess }}

  {{- if $ingress.enabled }}
    {{- $hosts := $ingress.hosts | default list }}
    {{- if not $hosts }}
      {{- $errors = append $errors "grafana.ingress is enabled but grafana.ingress.hosts is empty, so the Ingress carries no rules and routes nothing. Set the hostname Grafana should answer on." }}
    {{- else if has "chart-example.local" $hosts }}
      {{- $errors = append $errors "grafana.ingress.hosts still contains the upstream placeholder chart-example.local. Set the hostname Grafana should answer on." }}
    {{- end }}
    {{- if not ( $ingress.tls | default list ) }}
      {{- $warnings = append $warnings "grafana.ingress has no tls block. Grafana authenticates with a session cookie, so without TLS that cookie and the admin password cross the network in the clear. Terminate TLS at the Ingress, or at the load balancer in front of it if the certificate lives there." }}
    {{- end }}
  {{- end }}

  {{- if eq $svcType "LoadBalancer" }}
    {{- if not ( $service.loadBalancerSourceRanges | default list ) }}
      {{- $msg := "grafana.service.type is LoadBalancer with no grafana.service.loadBalancerSourceRanges, so Grafana answers every address the load balancer accepts. Set the CIDRs allowed to reach it." }}
      {{- if $acked }}
        {{- $warnings = append $warnings ( printf "%s connections.grafana.allowPublicAccess is set, so this is permitted — confirm the allowlist really is enforced somewhere the chart cannot see it, such as a security group or an authenticating proxy." $msg ) }}
      {{- else }}
        {{- $errors = append $errors ( printf "%s If the allowlist is enforced elsewhere — a security group, an egress firewall, an authenticating proxy — set connections.grafana.allowPublicAccess=true to say so." $msg ) }}
      {{- end }}
    {{- end }}
  {{- else if eq $svcType "NodePort" }}
    {{- if not $acked }}
      {{- $errors = append $errors "grafana.service.type is NodePort, which opens a port on every node with no allowlist mechanism of its own — reachable by anything that can route to a node. Prefer an Ingress, or set connections.grafana.allowPublicAccess=true if node access is already restricted." }}
    {{- end }}
  {{- end }}

  {{- if ( include "mzmon.grafana.exposed" $ ) }}
    {{- if not ( ( include "mzmon.grafana.iniSection" ( dict "root" $ "name" "server" ) | fromYaml ).root_url | default "" ) }}
      {{- $warnings = append $warnings "Grafana is exposed but grafana.ini.server.root_url is unset, so it falls back to its own Service address. Share links, alert notification links, and OAuth redirect URIs are all built from that value, and all three break silently when it does not match the URL users actually reach." }}
    {{- end }}

    {{- /*
    Every `auth.*` section Grafana understands names a provider; `auth` itself
    holds cross-provider settings, and `auth.anonymous` grants access rather
    than establishing identity.
    */}}
    {{- $providers := list }}
    {{- range $section, $config := $ini }}
      {{- if and ( hasPrefix "auth." $section ) ( ne $section "auth.anonymous" ) ( kindIs "map" $config ) }}
        {{- if ( dig "enabled" true $config ) }}
          {{- $providers = append $providers $section }}
        {{- end }}
      {{- end }}
    {{- end }}
    {{- if not $providers }}
      {{- $warnings = append $warnings "Grafana is exposed with no identity provider configured, so the only account is the generated admin and the only credential is its password. Configure one under grafana.ini — any auth.* section Grafana supports — and map an IdP group claim onto Grafana roles with role_attribute_path so membership does the provisioning." }}
    {{- end }}

    {{- if ( include "mzmon.grafana.iniSection" ( dict "root" $ "name" "auth.anonymous" ) | fromYaml ).enabled }}
      {{- $msg := "grafana.ini has auth.anonymous enabled while Grafana is reachable from outside the cluster, so anyone who can reach it reads every dashboard and every datasource behind it without signing in." }}
      {{- if $acked }}
        {{- $warnings = append $warnings ( printf "%s connections.grafana.allowPublicAccess is set, so this is permitted." $msg ) }}
      {{- else }}
        {{- $errors = append $errors ( printf "%s Disable it, or set connections.grafana.allowPublicAccess=true if the exposure is deliberate and gated elsewhere." $msg ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate scheduling, disruption, and autoscaling for the bundled Grafana.

Usage:
  {{- $res := include "mzmon.grafana.validate.disruption" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate.disruption" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.grafana | default dict }}
  {{- $maxReplicas := int ( include "mzmon.grafana.maxReplicas" $ ) }}
  {{- $pdb := include "mzmon.grafana.section" ( dict "root" $ "name" "podDisruptionBudget" ) | fromYaml }}
  {{- $auto := include "mzmon.grafana.section" ( dict "root" $ "name" "autoscaling" ) | fromYaml }}
  {{- $requests := ( include "mzmon.grafana.section" ( dict "root" $ "name" "resources" ) | fromYaml ).requests | default dict }}

  {{- if and ( gt $maxReplicas 1 ) ( not $pdb ) }}
    {{- $warnings = append $warnings ( printf "Grafana can run up to %d replicas with no PodDisruptionBudget, so a node drain can evict all of them at once. Set grafana.podDisruptionBudget.maxUnavailable=1." $maxReplicas ) }}
  {{- end }}
  {{- if and $pdb.minAvailable ( le $maxReplicas 1 ) }}
    {{- $warnings = append $warnings "grafana.podDisruptionBudget sets minAvailable on a single replica, which permits no voluntary eviction at all, so node drains hang indefinitely. Use maxUnavailable: 1 instead; on a singleton it is a harmless no-op." }}
  {{- end }}

  {{- if $auto.enabled }}
    {{- if and $auto.targetCPU ( not $requests.cpu ) }}
      {{- $warnings = append $warnings "grafana.autoscaling.targetCPU is set but grafana.resources.requests.cpu is not. An HPA measures utilization against the request, so with no request there is no denominator and it never scales." }}
    {{- end }}
    {{- if and $auto.targetMemory ( not $requests.memory ) }}
      {{- $warnings = append $warnings "grafana.autoscaling.targetMemory is set but grafana.resources.requests.memory is not, so the HPA has nothing to measure utilization against." }}
    {{- end }}
  {{- end }}

  {{- if not $requests }}
    {{- $warnings = append $warnings "grafana.resources.requests is empty, so Grafana is BestEffort — first to be evicted under node pressure, and invisible to the scheduler when it packs the node. Set at least a CPU and memory request." }}
  {{- end }}

  {{- /*
  Grafana's unified alerting is not HA on its own: without gossip every replica
  evaluates every rule and notifies independently. This only affects rules
  created *in* Grafana — the Prometheus rules this chart ships are evaluated
  elsewhere and routed by Alertmanager — but unified alerting is on by default,
  so the trap is latent from the first rule someone writes.
  */}}
  {{- $haPeers := ( include "mzmon.grafana.iniSection" ( dict "root" $ "name" "unified_alerting" ) | fromYaml ).ha_peers | default "" }}
  {{- if and ( gt $maxReplicas 1 ) ( not $haPeers ) }}
    {{- $warnings = append $warnings ( printf "Grafana can run up to %d replicas with no gossip between them, so any Grafana-managed alert rule is evaluated by each replica and notifies %d times. Set grafana.headlessService=true and grafana.ini.unified_alerting.ha_peers, as the grafana-postgres profile does. Rules this chart ships are Prometheus rules and are unaffected." $maxReplicas $maxReplicas ) }}
  {{- end }}
  {{- /*
  `ha_peers` names the headless Service by DNS. Without it the name does not
  resolve, and a failed join is not a failed start — the replicas come up
  healthy and quietly duplicate every notification.
  */}}
  {{- if and $haPeers ( not ( dig "headlessService" false $values ) ) }}
    {{- $warnings = append $warnings "grafana.ini.unified_alerting.ha_peers is set but grafana.headlessService is false, so there is no headless Service for that name to resolve to. The replicas start healthy, never find each other, and each notifies separately. Set grafana.headlessService=true." }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the bundled Grafana's security-relevant surface.

Usage:
  {{- $res := include "mzmon.grafana.validate.security" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate.security" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.grafana | default dict }}

  {{- if ( include "mzmon.grafana.section" ( dict "root" $ "name" "imageRenderer" ) | fromYaml ).enabled }}
    {{- $warnings = append $warnings "grafana.imageRenderer is enabled. The renderer is a headless Chromium that fetches URLs on Grafana's behalf, which makes it both a large attack surface and a server-side request forgery pivot into the cluster network. Leave it off in production and export panels client-side." }}
  {{- end }}

  {{- if not ( dig "assertNoLeakedSecrets" true $values ) }}
    {{- $warnings = append $warnings "grafana.assertNoLeakedSecrets is disabled, so nothing stops a database or OAuth secret being inlined into grafana.ini — which renders into a ConfigMap, visible in the release manifest and in helm get values. Re-enable it and use $__file{} or $__env{} expansion instead." }}
  {{- end }}

  {{- range $plugin := ( $values.plugins | default list ) }}
    {{- if not ( contains "@" $plugin ) }}
      {{- $warnings = append $warnings ( printf "grafana.plugins entry %q pins no version, so it is re-resolved from grafana.com on every pod start and can change underneath a pinned Grafana. Pin it as name@version, or bake the plugin into the image — which is also the only option on a hardened base image, since installing at start needs a shell." $plugin ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the Grafana datasource configuration.

Usage:
  {{- $res := include "mzmon.grafana.validate.datasources" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate.datasources" }}
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

{{- /*
Validate the NetworkPolicy on the bundled Grafana.

The subchart's policy template is unusually rigid — one ingress rule, on
`service.targetPort`, and no way to add a second port through values — so most
of what can go wrong here is a combination that renders an object which enforces
nothing, or one that quietly closes a port something else needs.

Usage:
  {{- $res := include "mzmon.grafana.validate.networkPolicy" $ | fromYaml }}
*/}}
{{- define "mzmon.grafana.validate.networkPolicy" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $np := $.Values.grafana.networkPolicy | default dict }}

  {{- if $np.enabled }}
    {{- $egress := $np.egress | default dict }}
    {{- /* policyTypes is built from these two flags alone. With both off the
           chart emits a NetworkPolicy with no policy types and no rules: a real
           object, selecting Grafana's pods, enforcing nothing. */}}
    {{- if and ( not $np.ingress ) ( not $egress.enabled ) }}
      {{- $errors = append $errors "grafana.networkPolicy.enabled is on but both grafana.networkPolicy.ingress and grafana.networkPolicy.egress.enabled are off, so the rendered policy has no policy types and no rules. It would be created and enforce nothing. Turn one of them on, or set grafana.networkPolicy.enabled=false." }}
    {{- end }}

    {{- if and ( not $np.allowExternal ) ( not $np.explicitNamespacesSelector ) ( not $np.explicitIpBlocks ) }}
      {{- /* Without either escape hatch the only permitted sources are pods
             carrying the chart's client label, which nothing in this release
             sets. An ingress controller in another namespace is the usual
             casualty, and Grafana simply stops answering. */}}
      {{- $warnings = append $warnings "grafana.networkPolicy.allowExternal is off with neither explicitNamespacesSelector nor explicitIpBlocks set, so the only sources allowed in are pods labelled `<fullname>-client: \"true\"` in Grafana's own namespace. An ingress controller, a load balancer, or `kubectl port-forward` would all be denied." }}
    {{- end }}

    {{- /* The subchart's policy opens `service.targetPort` and nothing else, so
           on its own it closes the alerting gossip port on the very pods that
           need it. `templates/networkpolicies.yaml` renders the missing rule
           alongside; this fires when that supplement has been turned off and the
           gap is real again. Read through `mzmon.grafana.iniSection`, which
           tolerates a nulled `grafana.ini` section — `dig` does not. */}}
    {{- if $np.ingress }}
      {{- $haPeers := ( include "mzmon.grafana.iniSection" ( dict "root" $ "name" "unified_alerting" ) | fromYaml ).ha_peers | default "" }}
      {{- if and $haPeers ( not ( include "mzmon.networkPolicy.grafanaGossip.enabled" $ ) ) }}
        {{- $warnings = append $warnings "grafana.ini.unified_alerting.ha_peers is set but networkPolicies.grafanaGossip is off, and grafana.networkPolicy only ever opens service.targetPort. Port 9094 stays closed, the replicas never find each other, and every Grafana-managed alert notifies once per replica. Turn networkPolicies.grafanaGossip back on, or supply an equivalent policy of your own." }}
      {{- end }}
    {{- end }}
  {{- else }}
    {{- $warnings = append $warnings "grafana.networkPolicy.enabled is recommended in production." }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Render a datasource's TLS settings into the pair Grafana actually reads.

Grafana splits one logical setting across two places and names neither of them
after TLS in an obvious way:

  * `jsonData.tlsAuthWithCACert` / `tlsAuth` / `tlsSkipVerify` / `serverName`
    are the switches, and
  * `secureJsonData.tlsCACert` / `tlsClientCert` / `tlsClientKey` carry the PEM.

The PEM is never written inline here. It comes from a Secret through the
`GrafanaDatasource` CRD's `valuesFrom`, so the material stays in a Secret and the
rendered manifest carries a reference rather than a certificate.

**`valuesFrom` substitutes into a placeholder; it does not create the field.**
grafana-operator replaces a `${...}` token found at `targetPath`, so a
`valuesFrom` entry whose target path does not already exist in the datasource
body is silently dropped — the datasource applies cleanly, `secureJsonFields`
simply never gains the key, and the first symptom is
`x509: certificate signed by unknown authority` from a datasource that looks
correctly configured in the CR. Every reference below therefore emits its
placeholder alongside. Found on a live cluster.

**This hop does not renew like the others.** Grafana stores what it is given in
its own database, so new material takes effect when grafana-operator
re-provisions the datasource — on `resyncPeriod`, not on the certificate's
renewal. That is a property of Grafana's provisioning model, not something the
chart can route around, and it is the reason the docs call this hop out
separately.

Returns `{jsonData: {...}, secureJsonData: {...}, valuesFrom: [...]}`; the caller
merges all three.

Usage:
  {{- $tls := include "mzmon.grafana.datasource.tls" ( dict "root" $ "name" "loki" "url" $url ) | fromYaml }}
*/}}
{{- define "mzmon.grafana.datasource.tls" }}
  {{- $root := .root | required ".root must be specified" }}
  {{- $name := .name | required ".name must be specified" }}
  {{- $url := .url | default "" }}
  {{- $cfg := dig "datasources" $name "tls" dict ( $root.Values.connections | default dict ) }}

  {{- /* Unset follows the URL scheme, so moving a URL to https is one edit
         rather than two, and the two cannot disagree. */}}
  {{- $enabled := $cfg.enabled }}
  {{- if typeIs "<nil>" $enabled }}
    {{- $enabled = hasPrefix "https://" $url }}
  {{- end }}

  {{- $jsonData := dict }}
  {{- $secureJsonData := dict }}
  {{- $valuesFrom := list }}

  {{- if $enabled }}
    {{- $caSecret := $cfg.caSecret | default dict }}
    {{- $jsonData = merge $jsonData ( dict "tlsAuthWithCACert" true ) }}
    {{- /* **The placeholder has to be the Secret *key*, not a name of our
           choosing.** grafana-operator substitutes `${<key>}` — the key from the
           `secretKeyRef`, verbatim, dots and all — and leaves anything else
           alone. A mnemonic token like `${tlsCACert}` therefore reaches Grafana
           unsubstituted, is stored as a literal, and every query fails with
           "Unable to load TLS certificate". Measured on operator v5.24.0: the CR
           is accepted, the operator logs nothing, and `secureJsonFields` reports
           `tlsCACert: true` — so the only symptom is at query time.

           Inline still wins when both are given: `caPem` needs no operator
           involvement at all. But `caSecret` is the better default where the
           caller can use it, because an inlined copy goes stale when the CA
           rotates and a reference does not. */}}
    {{- if $cfg.caPem }}
      {{- $secureJsonData = merge $secureJsonData ( dict "tlsCACert" $cfg.caPem ) }}
    {{- else if $caSecret.name }}
      {{- $caKey := $caSecret.key | default "ca.crt" }}
      {{- /* Templated like the URL beside it, so a profile can name a Secret
             whose name depends on the release — `mzmon.certificates.secretName`
             for one. Without it a profile can move a datasource to https but
             cannot say what to trust, which leaves the operator to supply by
             hand the one value the chart already knows. */}}
      {{- $caName := tpl $caSecret.name $root }}
      {{- $secureJsonData = merge $secureJsonData ( dict "tlsCACert" ( printf "${%s}" $caKey ) ) }}
      {{- $valuesFrom = append $valuesFrom ( dict
          "targetPath" "secureJsonData.tlsCACert"
          "valueFrom" ( dict "secretKeyRef" ( dict
            "name" $caName
            "key" $caKey ) ) ) }}
    {{- end }}

    {{- $client := $cfg.clientCert | default dict }}
    {{- if $client.secretName }}
      {{- $certKey := $client.certKey | default "tls.crt" }}
      {{- $keyKey := $client.keyKey | default "tls.key" }}
      {{- /* Same rule as the CA above. Distinct keys are required rather than
             merely conventional: two targets sharing one placeholder would both
             receive whichever value the operator substituted last. */}}
      {{- if eq $certKey $keyKey }}
        {{- fail ( printf "connections.datasources.%s.tls.clientCert sets certKey and keyKey to the same key (%q). grafana-operator substitutes by key name, so both the certificate and the private key would resolve to the same value." $name $certKey ) }}
      {{- end }}
      {{- $jsonData = merge $jsonData ( dict "tlsAuth" true ) }}
      {{- $secureJsonData = merge $secureJsonData ( dict
          "tlsClientCert" ( printf "${%s}" $certKey )
          "tlsClientKey" ( printf "${%s}" $keyKey ) ) }}
      {{- $clientName := tpl $client.secretName $root }}
      {{- $valuesFrom = append $valuesFrom ( dict
          "targetPath" "secureJsonData.tlsClientCert"
          "valueFrom" ( dict "secretKeyRef" ( dict
            "name" $clientName
            "key" $certKey ) ) ) }}
      {{- $valuesFrom = append $valuesFrom ( dict
          "targetPath" "secureJsonData.tlsClientKey"
          "valueFrom" ( dict "secretKeyRef" ( dict
            "name" $clientName
            "key" $keyKey ) ) ) }}
    {{- end }}

    {{- with $cfg.serverName }}
      {{- $jsonData = merge $jsonData ( dict "serverName" . ) }}
    {{- end }}
  {{- end }}

  {{- dict "jsonData" $jsonData "secureJsonData" $secureJsonData "valuesFrom" $valuesFrom | toYaml }}
{{- end }}
