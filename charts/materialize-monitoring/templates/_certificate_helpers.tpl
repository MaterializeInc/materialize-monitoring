{{- /* Certificate helpers and validators. */}}

{{- /*
The cluster's DNS domain.

Read from `global` because Loki and Thanos already build real addresses from
`global.clusterDomain` and Helm propagates that key into every subchart — so one
value covers all three. Defaulted here rather than relied on from values, so a
consumer who replaces `global` wholesale still gets a usable ladder rather than
SANs ending in a bare dot.

Usage:
  {{ include "mzmon.clusterDomain" $ }}
*/}}
{{- define "mzmon.clusterDomain" -}}
  {{- ( $.Values.global | default dict ).clusterDomain | default "cluster.local" -}}
{{- end }}

{{- /*
The components this chart can issue a certificate for, and where each one runs.

The key is both the `certificates.components` key and the subchart key as it
appears in `.Subcharts`, which is what lets a single lookup answer "is this
component part of this render". The value is the helper that resolves its
namespace — always a helper, never `.Release.Namespace`, because
`profiles/split-namespace.values.yaml` moves most of these and a certificate
issued into the wrong namespace is mounted by nothing.

Usage:
  {{- $components := include "mzmon.certificates.components" $ | fromYaml }}
*/}}
{{- define "mzmon.certificates.components" }}
alloy-agent: mzmon.alloyAgent.namespace
alloy-gateway: mzmon.alloyGateway.namespace
loki: mzmon.loki.namespace
thanos: mzmon.thanos.namespace
grafana: mzmon.grafana.namespace
alertmanager: mzmon.alertmanager.namespace
{{- end }}

{{- /*
Whether a component certificate should render.

Same three-part shape as `mzmon.networkPolicy.enabled`: the subchart has to be
part of this render, the master switch has to be on, and a per-component
`enabled` overrides it in either direction. `typeIs "<nil>"` distinguishes
"unset, follow the master" from an explicit `false`.

Usage:
  {{- if ( include "mzmon.certificates.enabled" ( dict "context" $ "component" "loki" ) ) }}
*/}}
{{- define "mzmon.certificates.enabled" }}
  {{- $context := .context | required ".context must be specified" }}
  {{- $component := .component | required ".component must be specified" }}
  {{- if hasKey $context.Subcharts $component }}
    {{- $certs := $context.Values.certificates | default dict }}
    {{- $cfg := index ( $certs.components | default dict ) $component | default dict }}
    {{- if typeIs "<nil>" $cfg.enabled }}
      {{- ternary "true" "" ( $certs.enabled | default false ) }}
    {{- else }}
      {{- ternary "true" "" $cfg.enabled }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Whether the chart is bootstrapping its own self-signed root.

Usage:
  {{- if ( include "mzmon.certificates.selfSigned" $ ) }}
*/}}
{{- define "mzmon.certificates.selfSigned" }}
  {{- $certs := $.Values.certificates | default dict }}
  {{- if $certs.enabled }}
    {{- if ( dig "internal" "selfSigned" "enabled" false $certs ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Name of the issuer that signs internal certificates.

Either the operator's, or the CA issuer the chart bootstraps. Kept in one place
because the answer is referenced from every component `Certificate` and getting
it out of step with what was actually created leaves every one of them Pending
with `issuer not found` — a failure that looks like cert-manager is broken.

Usage:
  {{- $ref := include "mzmon.certificates.internalIssuerRef" $ | fromYaml }}
*/}}
{{- define "mzmon.certificates.internalIssuerRef" }}
  {{- $certs := $.Values.certificates | default dict }}
  {{- $internal := $certs.internal | default dict }}
  {{- if ( include "mzmon.certificates.selfSigned" $ ) }}
    {{- dict
        "name" ( printf "%s-internal-ca" ( include "mzmon.fullname" $ ) )
        "kind" ( dig "selfSigned" "kind" "ClusterIssuer" $internal )
        "group" "cert-manager.io" | toYaml }}
  {{- else }}
    {{- $ref := $internal.issuerRef | default dict }}
    {{- dict
        "name" ( $ref.name | default "" )
        "kind" ( $ref.kind | default "ClusterIssuer" )
        "group" ( $ref.group | default "cert-manager.io" ) | toYaml }}
  {{- end }}
{{- end }}

{{- /*
The namespace a bootstrapped CA's Secret has to live in.

**This is the trap in the self-signed path.** A `ClusterIssuer` with a `ca`
backend does not read its Secret from the namespace the `Certificate` was
created in — it reads from cert-manager's *cluster resource namespace*, which
defaults to `cert-manager`. Render the CA certificate into the release namespace
and the ClusterIssuer stays `False` with `secret not found`, while the Secret it
is looking straight at sits one namespace over.

A namespaced `Issuer` has the opposite rule: it reads from its own namespace, so
the release namespace is correct there.

Usage:
  {{ include "mzmon.certificates.caNamespace" $ }}
*/}}
{{- define "mzmon.certificates.caNamespace" -}}
  {{- $internal := ( $.Values.certificates | default dict ).internal | default dict }}
  {{- $selfSigned := $internal.selfSigned | default dict }}
  {{- if eq ( $selfSigned.kind | default "ClusterIssuer" ) "ClusterIssuer" -}}
    {{- $selfSigned.caSecretNamespace | default "cert-manager" -}}
  {{- else -}}
    {{- include "mzmon.namespace" $ -}}
  {{- end -}}
{{- end }}

{{- /*
Secret name for a component's certificate.

Usage:
  {{ include "mzmon.certificates.secretName" ( dict "context" $ "component" "loki" ) }}
*/}}
{{- define "mzmon.certificates.secretName" -}}
  {{- $context := .context | required ".context must be specified" }}
  {{- $component := .component | required ".component must be specified" }}
  {{- $cfg := index ( ( $context.Values.certificates | default dict ).components | default dict ) $component | default dict }}
  {{- $cfg.secretName | default ( printf "%s-%s-tls" ( include "mzmon.fullname" $context ) $component ) -}}
{{- end }}

{{- /*
The SAN ladder for one component.

Every rung, for every Service the component answers on:

  $svc
  $svc.$ns
  $svc.$ns.svc
  $svc.$ns.svc.$clusterDomain

All four, because **the chart's own URLs do not agree on which one to use**.
Every in-cluster destination this chart writes stops at `$svc.$ns.svc`, while
the Terraform test substrate writes `…svc.cluster.local`. A certificate carrying
only the fully-qualified form therefore fails verification against the exact
endpoints the chart ships, and the error reads as a broken certificate rather
than as a mismatch in name form. Issuing all four costs nothing.

`localhost` comes along for self-probes and anything a component reaches through
its own loopback.

Returns a YAML array; read it back with `fromYamlArray`.

Usage:
  {{- $sans := include "mzmon.certificates.sans" ( dict "context" $ "component" "loki" ) | fromYamlArray }}
*/}}
{{- define "mzmon.certificates.sans" }}
  {{- $context := .context | required ".context must be specified" }}
  {{- $component := .component | required ".component must be specified" }}
  {{- $components := include "mzmon.certificates.components" $context | fromYaml }}
  {{- $namespace := include ( index $components $component ) $context }}
  {{- $domain := include "mzmon.clusterDomain" $context }}
  {{- $cfg := index ( ( $context.Values.certificates | default dict ).components | default dict ) $component | default dict }}

  {{- $sans := list }}
  {{- range $svc := ( $cfg.services | default list ) }}
    {{- $sans = append $sans $svc }}
    {{- $sans = append $sans ( printf "%s.%s" $svc $namespace ) }}
    {{- $sans = append $sans ( printf "%s.%s.svc" $svc $namespace ) }}
    {{- $sans = append $sans ( printf "%s.%s.svc.%s" $svc $namespace $domain ) }}
  {{- end }}
  {{- $sans = append $sans "localhost" }}
  {{- $sans = concat $sans ( $cfg.extraDnsNames | default list ) }}
  {{- /* Grafana is the one component that may also answer on a public name,
         when an L4 load balancer passes TLS through to the pod. */}}
  {{- if eq $component "grafana" }}
    {{- $sans = concat $sans ( dig "external" "dnsNames" list ( $context.Values.certificates | default dict ) ) }}
  {{- end }}
  {{- $sans | uniq | toYaml }}
{{- end }}

{{- /*
Entrypoint for certificate validation.

Usage:
  {{- $res := include "mzmon.certificates.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.certificates.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $certs := $.Values.certificates | required "certificates is missing from values." }}

  {{- if $certs.enabled }}
    {{- $internal := $certs.internal | default dict }}
    {{- $selfSigned := $internal.selfSigned | default dict }}
    {{- $issuerName := dig "issuerRef" "name" "" $internal }}

    {{- /* Both paths at once is ambiguous rather than additive: the component
           certificates can only reference one issuer, so one of the two things
           the operator asked for would silently not happen. */}}
    {{- if and $selfSigned.enabled $issuerName }}
      {{- $errors = append $errors ( printf "certificates.internal sets both selfSigned.enabled and issuerRef.name (%q). They are alternatives — the chart either bootstraps a root or consumes yours, and component certificates can only reference one issuer. Clear whichever you did not mean." $issuerName ) }}
    {{- end }}
    {{- if and ( not $selfSigned.enabled ) ( not $issuerName ) }}
      {{- $errors = append $errors "certificates.enabled is on but no internal issuer is configured. Set certificates.internal.issuerRef.name to an existing cert-manager Issuer or ClusterIssuer, or set certificates.internal.selfSigned.enabled=true to have the chart bootstrap a self-signed root." }}
    {{- end }}

    {{- /* A namespaced Issuer can only sign Certificates in its own namespace,
           and split-namespace puts them in several. */}}
    {{- $ref := include "mzmon.certificates.internalIssuerRef" $ | fromYaml }}
    {{- if eq $ref.kind "Issuer" }}
      {{- $release := include "mzmon.namespace" $ }}
      {{- $outside := list }}
      {{- range $component, $nsHelper := ( include "mzmon.certificates.components" $ | fromYaml ) }}
        {{- if ( include "mzmon.certificates.enabled" ( dict "context" $ "component" $component ) ) }}
          {{- $ns := include $nsHelper $ }}
          {{- if ne $ns $release }}
            {{- $outside = append $outside $component }}
          {{- end }}
        {{- end }}
      {{- end }}
      {{- if $outside }}
        {{- $errors = append $errors ( printf "certificates.internal issuer is a namespaced Issuer, but %v run outside the release namespace %q. A namespaced Issuer signs only for Certificates in its own namespace, so those would stay Pending forever with no issuer found. Use kind: ClusterIssuer, which is what the split-namespace layout requires." ( sortAlpha $outside ) $release ) }}
      {{- end }}
    {{- end }}

    {{- /* An external certificate with no names is an empty spec cert-manager
           rejects; names with no issuer render nothing and look like they did. */}}
    {{- $external := $certs.external | default dict }}
    {{- $extName := dig "issuerRef" "name" "" $external }}
    {{- $extDns := $external.dnsNames | default list }}
    {{- if and $extName ( not $extDns ) }}
      {{- $errors = append $errors "certificates.external.issuerRef.name is set but certificates.external.dnsNames is empty. cert-manager rejects a Certificate with no names; set the hostname the load balancer answers on." }}
    {{- end }}
    {{- if and $extDns ( not $extName ) }}
      {{- $warnings = append $warnings ( printf "certificates.external.dnsNames is set to %v with no certificates.external.issuerRef.name, so no external certificate is issued. If your load balancer terminates TLS with a cloud-managed certificate that is correct — pass its ARN or resource ID through grafana.service.annotations instead, and clear dnsNames to say so." $extDns ) }}
    {{- end }}

    {{- $res := include "mzmon.certificates.validate.sans" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}
  {{- end }}

  {{- $res := include "mzmon.certificates.validate.clusterDomain" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- $res := include "mzmon.certificates.validate.serverTls" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Assert that every in-cluster destination URL matches a SAN.

The cheapest guard against the failure this whole SAN ladder exists to prevent:
a wrong or incomplete `services` list is valid YAML, installs clean, and fails
at the first handshake with a hostname mismatch that reads like a certificate
problem rather than a values problem.

Only hostnames the chart itself writes are checked — an operator pointing a
destination at something outside the release is their own trust decision.

Usage:
  {{- $res := include "mzmon.certificates.validate.sans" $ | fromYaml }}
*/}}
{{- define "mzmon.certificates.validate.sans" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- /* Destination URL -> the component whose certificate has to cover it. */}}
  {{- $pipeline := $.Values.pipeline | default dict }}
  {{- $checks := list
      ( dict "component" "alloy-gateway" "path" "pipeline.logging.agent.destination.loki.url"
             "url" ( dig "logging" "agent" "destination" "loki" "url" "" $pipeline ) )
      ( dict "component" "loki" "path" "pipeline.logging.gateway.destination.loki.url"
             "url" ( dig "logging" "gateway" "destination" "loki" "url" "" $pipeline ) )
      ( dict "component" "thanos" "path" "pipeline.metrics.gateway.destination.prometheusRemoteWrite.url"
             "url" ( dig "metrics" "gateway" "destination" "prometheusRemoteWrite" "url" "" $pipeline ) ) }}

  {{- range $check := $checks }}
    {{- if ( include "mzmon.certificates.enabled" ( dict "context" $ "component" $check.component ) ) }}
      {{- $url := tpl ( $check.url | toString ) $ }}
      {{- if $url }}
        {{- /* Strip scheme, then path, then port, leaving the host. */}}
        {{- $host := regexReplaceAll "^[a-zA-Z][a-zA-Z0-9+.-]*://" $url "" | splitList "/" | first | splitList ":" | first }}
        {{- if $host }}
          {{- $sans := include "mzmon.certificates.sans" ( dict "context" $ "component" $check.component ) | fromYamlArray }}
          {{- if not ( has $host $sans ) }}
            {{- $errors = append $errors ( printf "%s dials %q, which is not a SAN on the %s certificate. Issued SANs are %v. Add the Service to certificates.components.%s.services, or the hostname to its extraDnsNames — otherwise the handshake fails with a name mismatch that reads like a broken certificate." $check.path $host $check.component $sans $check.component ) }}
          {{- end }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Keep the one subchart that does not read `global.clusterDomain` in step.

Helm propagates `global` into every subchart, which covers Loki and Thanos.
`metrics-server` reads its own `tls.clusterDomain`, so a consumer who sets the
global and nothing else gets a metrics-server certificate whose SANs name a
domain the cluster does not use — and the APIService fails to verify it.

A warning rather than a fan-out, for the same reason the scheduling profile
carries its fan-out explicitly: the chart making the second write silently is
what stops anyone noticing there are two.

Usage:
  {{- $res := include "mzmon.certificates.validate.clusterDomain" $ | fromYaml }}
*/}}
{{- define "mzmon.certificates.validate.clusterDomain" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if hasKey $.Subcharts "metrics-server" }}
    {{- $domain := include "mzmon.clusterDomain" $ }}
    {{- $ms := dig "tls" "clusterDomain" "" ( index $.Values "metrics-server" | default dict ) }}
    {{- if and $ms ( ne $ms $domain ) }}
      {{- $warnings = append $warnings ( printf "global.clusterDomain is %q but metrics-server.tls.clusterDomain is %q. metrics-server does not read the global, so its certificate would carry SANs for a domain the cluster does not use and the APIService would fail to verify it. Set both." $domain $ms ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Whether Loki is configured to serve TLS on its HTTP port.

Read from the subchart passthrough rather than from a switch of our own: Loki's
`server.http_tls_config` is the thing that actually changes the listener, so
anything else would be a second source of truth that can disagree with it.

Usage:
  {{- if ( include "mzmon.certificates.lokiServerTls" $ ) }}
*/}}
{{- define "mzmon.certificates.lokiServerTls" }}
  {{- if hasKey $.Subcharts "loki" }}
    {{- if dig "loki" "server" "http_tls_config" "cert_file" "" ( $.Values.loki | default dict ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Whether Thanos Receive is configured to serve TLS on its remote-write listener.

`extraArgs` is a flat list of strings, so this looks for the flag rather than a
structured key. Narrower than Loki's case by nature: the flag scopes to the
remote-write server, so Receive's HTTP port — probes, metrics — is untouched.

Usage:
  {{- if ( include "mzmon.certificates.thanosReceiveServerTls" $ ) }}
*/}}
{{- define "mzmon.certificates.thanosReceiveServerTls" }}
  {{- if hasKey $.Subcharts "thanos" }}
    {{- range $arg := ( dig "receive" "extraArgs" list ( $.Values.thanos | default dict ) ) }}
      {{- if hasPrefix "--remote-write.server-tls-cert" ( $arg | toString ) }}
        {{- "true" }}
      {{- end }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Validate the surface that moves together when a backend starts serving TLS.

This is the part of the feature that is worth a validator at all. Turning on a
server's TLS is one key; keeping the deployment working is six, spread across
three subcharts and two of this chart's own trees, and **every one of them fails
quietly**:

  * a client still on `http://` gets a protocol error that reads like the server
    is broken,
  * a probe still on HTTP fails readiness on every pod at once, so the rollout
    stalls and looks like a crashloop,
  * a ServiceMonitor still on `http` makes the backend's own metrics vanish
    while `up` stays absent rather than zero,
  * a Grafana datasource still on `http://` renders empty panels with no error —
    the worst failure mode this stack has.

None of those name TLS in the symptom. Hence errors rather than warnings for
each: a half-converted stack is not a degraded stack, it is a broken one, and it
is better to refuse the render than to hand someone four separate outages to
correlate.

Usage:
  {{- $res := include "mzmon.certificates.validate.serverTls" $ | fromYaml }}
*/}}
{{- define "mzmon.certificates.validate.serverTls" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $loki := $.Values.loki | default dict }}
  {{- $pipeline := $.Values.pipeline | default dict }}

  {{- if ( include "mzmon.certificates.lokiServerTls" $ ) }}
    {{- /* The writer. */}}
    {{- if dig "logging" "gateway" "destination" "loki" "enabled" false $pipeline }}
      {{- if not ( dig "logging" "gateway" "destination" "loki" "tls" "enabled" false $pipeline ) }}
        {{- $errors = append $errors "loki.loki.server.http_tls_config is set, so Loki serves TLS on 3100 — but pipeline.logging.gateway.destination.loki.tls.enabled is off, so the gateway would send plaintext at a TLS port. Every write fails with a protocol error that reads like Loki is broken. Turn the destination's TLS on, and point its caFile at the mounted CA." }}
      {{- end }}
    {{- end }}

    {{- /* The probes. Loki's `_pod.tpl` coalesces component, defaults, and loki
           level, so `defaults` is the one place that covers every component. */}}
    {{- $probeScheme := dig "defaults" "readinessProbe" "httpGet" "scheme" "" $loki }}
    {{- if ne ( $probeScheme | upper ) "HTTPS" }}
      {{- $errors = append $errors "loki.loki.server.http_tls_config is set but loki.defaults.readinessProbe.httpGet.scheme is not HTTPS, so the kubelet probes a TLS listener over plaintext. Every Loki pod fails readiness at once and the rollout stalls looking like a crashloop. Set the scheme on loki.defaults so it reaches every component — a per-component probe does not." }}
    {{- end }}

    {{- /* The scrape. */}}
    {{- if dig "monitoring" "serviceMonitor" "enabled" false $loki }}
      {{- $smScheme := dig "monitoring" "serviceMonitor" "scheme" "http" $loki }}
      {{- if ne ( $smScheme | lower ) "https" }}
        {{- $errors = append $errors "loki.loki.server.http_tls_config is set but loki.monitoring.serviceMonitor.scheme is not https, so the gateway scrapes a TLS listener over plaintext. Loki's own metrics disappear — and because the target fails rather than reports zero, `up` is absent rather than 0, so an alert on `up == 0` does not fire either. Set the scheme and loki.monitoring.serviceMonitor.tlsConfig." }}
      {{- end }}
    {{- end }}

    {{- /* The reader. */}}
    {{- if ( include "mzmon.grafana.datasource.enabled" ( dict "root" $ "name" "loki" ) ) }}
      {{- $url := tpl ( dig "datasources" "loki" "url" "" ( $.Values.connections | default dict ) | toString ) $ }}
      {{- if hasPrefix "http://" $url }}
        {{- $errors = append $errors ( printf "loki.loki.server.http_tls_config is set but connections.datasources.loki.url is %q. Grafana would dial plaintext at a TLS port, and a datasource that cannot connect renders every log panel empty with no error on the dashboard. Use https:// and supply the CA through connections.datasources.loki.valuesFrom." $url ) }}
      {{- end }}
    {{- end }}

    {{- /* The canary renders from its own template rather than `_pod.tpl`, so
           nothing in `defaults` reaches it — the same asymmetry the
           priorityClassName note next to it already calls out. */}}
    {{- if dig "lokiCanary" "enabled" false $loki }}
      {{- $canaryArgs := dig "lokiCanary" "extraArgs" list $loki }}
      {{- $hasTls := false }}
      {{- range $arg := $canaryArgs }}
        {{- if hasPrefix "-tls" ( $arg | toString ) }}{{- $hasTls = true }}{{- end }}
      {{- end }}
      {{- if not $hasTls }}
        {{- $warnings = append $warnings "loki.lokiCanary is enabled and Loki serves TLS, but the canary's extraArgs carry no -tls flag. It renders from its own template, so loki.defaults does not reach it. The canary will fail its write→read loop and report the log store as broken when it is not — which is worse than having no canary, because it is the check you trust during an incident." }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- if ( include "mzmon.certificates.thanosReceiveServerTls" $ ) }}
    {{- if dig "metrics" "gateway" "destination" "prometheusRemoteWrite" "enabled" false $pipeline }}
      {{- if not ( dig "metrics" "gateway" "destination" "prometheusRemoteWrite" "tls" "enabled" false $pipeline ) }}
        {{- $errors = append $errors "thanos.receive.extraArgs enables remote-write TLS, but pipeline.metrics.gateway.destination.prometheusRemoteWrite.tls.enabled is off, so the gateway would send plaintext at a TLS port and every metric write fails. Turn the destination's TLS on." }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* Serving TLS from material this chart is not issuing is legitimate —
         someone else's PKI, mounted by hand — but it is also what a half-applied
         profile looks like, and the symptom is a pod that cannot read its own
         cert. */}}
  {{- if or ( include "mzmon.certificates.lokiServerTls" $ ) ( include "mzmon.certificates.thanosReceiveServerTls" $ ) }}
    {{- if not ( dig "enabled" false ( $.Values.certificates | default dict ) ) }}
      {{- $warnings = append $warnings "A backend is configured to serve TLS while certificates.enabled is off, so this chart is not issuing the material it references. That is correct if you mount your own, and is otherwise the signature of a half-applied mtls profile — the pods will not start, because the cert file they are pointed at does not exist." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
