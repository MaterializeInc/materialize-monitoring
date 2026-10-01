{{- /* The Loki gateway: its nginx.conf, and the validators for it. */}}

{{- /*
The Loki gateway's nginx.conf.

**Rendered by the Loki subchart, not by this chart.** `loki.gateway.nginxConfig.file`
is `tpl`-evaluated in the subchart's context, so `.` here is that context:
`.Values` is the `loki:` block, and the subchart's own helpers (`loki.namespace`,
`loki.resourceName`, `loki.deployment.*`) are in scope. Nothing outside `loki:`
is visible — which is why the TLS settings live under
`loki.gateway.nginxConfig.tls` rather than following `certificates.*`.

It replaces the subchart's `loki.nginxFile` rather than extending it, because
the upstream file routes far more than Grafana needs and nginx has no way to
remove a `location` another file declared. What it serves:

| Route | Backend | Methods |
| --- | --- | --- |
| `/loki/api/v1/*` | query frontend | any |
| `/loki/api/v1/tail` | query frontend, as a websocket | any |
| `/prometheus/api/v1/rules`, `/prometheus/api/v1/alerts` | ruler | `GET` |
| `/` | the gateway itself, for the readiness probe | any |

What it refuses, with Loki's own `404 page not found`:

  * **Writes.** `/loki/api/v1/push` and `/otlp/v1/logs` belong to the
    alloy-gateway, which is the only writer this chart configures.
  * **Rule definitions.** `/loki/api/v1/rules*` and `/api/prom/rules*` are the
    ruler's configuration API. The alloy-gateway's `loki.rules.kubernetes` owns
    those definitions and reverts any change it did not make, so an editable
    copy in Grafana would be a lie. Grafana probes this API to decide whether a
    datasource's rules are editable, and reads exactly this body as "not
    supported" — the rule list stays visible, without an Edit button that fails
    on save.
  * **Administration.** Deletes, cache generation numbers, ring and status
    pages, `/flush`, `/config`. Reach a component directly with
    `kubectl port-forward` for those.

**Every request resolves its backend afresh.** `proxy_pass` takes a variable, so
nginx looks the name up through `resolver` per request instead of once at
startup, reusing an answer for at most ten seconds (`valid=10s`, whatever TTL the
cluster's DNS hands out). A Service deleted and recreated under the same name is
followed without a restart; measured against the pinned image, both workers
moved to a backend's new address inside that window. Upstream connections are
not kept alive, so kube-proxy balances each request rather than each connection.

**The status listener is loopback-only.** `stub_status` is served on
`127.0.0.1:8081` in plaintext, never on the client port, so turning on TLS for
clients does not break the metrics exporter's scrape of it. The exporter is
pointed there by `loki.gateway.metrics.extraArgs` in `values.yaml`; the two
move together.

**Certificates are re-read from disk.** A certificate named with a variable is
loaded per handshake rather than at startup, and `ssl_certificate_cache` bounds
that to one read a minute. `$mzmon_tls_reload` is an always-empty variable that
exists only to put the paths in that mode, so a certificate cert-manager renews
is picked up without restarting nginx. The CA bundles are read at startup.

Usage (from the Loki subchart's values):
  file: '{{ include "mzmon.loki.gateway.nginxConf" . }}'
*/}}
{{- define "mzmon.loki.gateway.nginxConf" -}}
{{- $gw := .Values.gateway }}
{{- $nginx := $gw.nginxConfig }}
{{- $tls := $nginx.tls | default dict }}
{{- $namespace := include "loki.namespace" . }}
{{- $domain := .Values.global.clusterDomain }}
{{- $lokiPort := .Values.loki.server.http_listen_port | toString }}
{{- $scheme := $nginx.schema | default "http" }}

{{- /* Which Service answers reads and which answers the ruler API, by
       deployment mode. The same mapping `loki.nginxFile` uses. */}}
{{- $readHost := include "loki.resourceName" ( dict "ctx" . "component" "query-frontend" ) }}
{{- $rulerHost := include "loki.resourceName" ( dict "ctx" . "component" "ruler" ) }}
{{- if eq ( include "loki.deployment.isMonolithic" . ) "true" }}
  {{- $readHost = include "loki.fullname" . }}
  {{- $rulerHost = include "loki.fullname" . }}
{{- else if eq ( include "loki.deployment.isScalable" . ) "true" }}
  {{- $readHost = include "loki.resourceName" ( dict "ctx" . "component" "read" ) }}
  {{- $rulerHost = include "loki.resourceName" ( dict "ctx" . "component" "backend" ) }}
{{- end }}
{{- $readUrl := printf "%s://%s.%s.svc.%s:%s" $scheme $readHost $namespace $domain $lokiPort }}
{{- $rulerUrl := printf "%s://%s.%s.svc.%s:%s" $scheme $rulerHost $namespace $domain $lokiPort -}}

worker_processes  2;
error_log  /dev/stderr;
pid        /tmp/nginx.pid;
worker_rlimit_nofile 8192;

events {
  worker_connections  4096;
}

http {
  client_body_temp_path /tmp/client_temp;
  proxy_temp_path       /tmp/proxy_temp_path;
  fastcgi_temp_path     /tmp/fastcgi_temp;
  uwsgi_temp_path       /tmp/uwsgi_temp;
  scgi_temp_path        /tmp/scgi_temp;

  client_max_body_size  {{ $nginx.clientMaxBodySize }};

  # Grafana gives up on a datasource request after its own timeout; these only
  # need to outlast it. A backend that cannot be reached at all fails fast.
  proxy_connect_timeout 10s;
  proxy_read_timeout    600s;
  proxy_send_timeout    600s;

  proxy_http_version    1.1;

  # The refusals below are `return 404 "<text>"`, which takes this type. Grafana
  # passes a plain-text error body through to its UI and redacts an HTML one, and
  # it is the body that tells Grafana a ruler API is not offered here.
  default_type  text/plain;
  server_tokens off;

  log_format   {{ $nginx.logFormat }}

  {{- if $gw.metrics.enabled }}

  # The access-log exporter sidecar reads every request but the probes over
  # syslog. The format is the subchart's `simple_upstream` preset, which the
  # exporter's `loki` preset parses by field position.
  map $request_uri $track {
    default 1;
    ~^/$ 0;
  }
  log_format access_log_exporter '$http_host\t$request_method\t$status\t$request_completion\t$request_time\t$request_length\t$bytes_sent\t$upstream_addr\t$upstream_connect_time\t$upstream_header_time\t$upstream_response_time\t$request_uri';
  access_log syslog:server=127.0.0.1:8514,nohostname access_log_exporter if=$track;
  {{- end }}

  {{- if $gw.verboseLogging }}

  access_log   /dev/stderr  main;
  {{- else }}

  map $status $loggable {
    ~^[23]  0;
    default 1;
  }
  access_log   /dev/stderr  main  if=$loggable;
  {{- end }}

  {{- if $nginx.resolver }}

  resolver {{ $nginx.resolver }};
  {{- else }}

  # `valid` caps how long an answer is reused whatever TTL the cluster's DNS
  # hands out, so a recreated Service is followed within ten seconds.
  resolver {{ .Values.global.dnsService }}.{{ .Values.global.dnsNamespace }}.svc.{{ $domain }}. valid=10s;
  {{- end }}

  {{- with $nginx.httpSnippet }}

  {{ tpl . $ | nindent 2 | trim }}
  {{- end }}

  # Grafana describes each query in X-Grafana-* headers; Loki logs X-Query-Tags
  # as key/value pairs, which is how a slow query is traced back to its panel.
  map $http_x_query_tags $query_tags {
    ""        "noop=";
    default   $http_x_query_tags;
  }

  # Always empty. Naming a certificate with a variable makes nginx read it per
  # handshake, through the cache below, instead of once at startup.
  map "" $mzmon_tls_reload {
    default "";
  }

  server {
    listen 127.0.0.1:8081;
    access_log off;

    location = /stub_status {
      stub_status;
    }

    location / {
      return 404 "404 page not found\n";
    }
  }

  server {
    listen {{ $gw.containerPort }}{{ if $nginx.ssl }} ssl{{ end }};
    {{- if $nginx.enableIPv6 }}
    listen [::]:{{ $gw.containerPort }}{{ if $nginx.ssl }} ssl{{ end }};
    {{- end }}

    {{- if $nginx.ssl }}

    ssl_certificate       {{ $tls.certFile }}$mzmon_tls_reload;
    ssl_certificate_key   {{ $tls.keyFile }}$mzmon_tls_reload;
    ssl_certificate_cache max=4;
    ssl_protocols         TLSv1.2 TLSv1.3;
    {{- if $tls.clientCaFile }}

    # Verify a client certificate when one is presented, and serve a client
    # that presents none: the kubelet's probe is one, and cannot.
    ssl_client_certificate {{ $tls.clientCaFile }};
    ssl_verify_client      optional;
    ssl_verify_depth       2;
    {{- end }}
    {{- end }}

    {{- if eq $scheme "https" }}

    proxy_ssl_trusted_certificate {{ $tls.upstreamCaFile }};
    proxy_ssl_verify              on;
    proxy_ssl_verify_depth        2;
    proxy_ssl_server_name         on;
    proxy_ssl_protocols           TLSv1.2 TLSv1.3;
    {{- if $tls.upstreamCertFile }}
    proxy_ssl_certificate         {{ $tls.upstreamCertFile }}$mzmon_tls_reload;
    proxy_ssl_certificate_key     {{ $tls.upstreamKeyFile }}$mzmon_tls_reload;
    proxy_ssl_certificate_cache   max=4;
    {{- end }}
    {{- end }}

    location = / {
      access_log off;
      return 200 'OK';
    }

    # Ruler: rule and alert state, for Grafana's alert list.
    location = /prometheus/api/v1/rules {
      limit_except GET {
        deny all;
      }
      {{- include "mzmon.loki.gateway.locationSnippet" . }}
      set $backend     "{{ $rulerUrl }}";
      proxy_pass       $backend$request_uri;
    }
    location = /prometheus/api/v1/alerts {
      limit_except GET {
        deny all;
      }
      {{- include "mzmon.loki.gateway.locationSnippet" . }}
      set $backend     "{{ $rulerUrl }}";
      proxy_pass       $backend$request_uri;
    }

    # Refused: rule definitions, writes, and administration.
    location ^~ /loki/api/v1/rules {
      return 404 "404 page not found\n";
    }
    location ^~ /api/prom/rules {
      return 404 "404 page not found\n";
    }
    location = /loki/api/v1/push {
      return 404 "404 page not found\n";
    }
    location ^~ /loki/api/v1/delete {
      return 404 "404 page not found\n";
    }
    location ^~ /loki/api/v1/cache/ {
      return 404 "404 page not found\n";
    }

    # Query frontend.
    location = /loki/api/v1/tail {
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection "upgrade";
      {{- include "mzmon.loki.gateway.locationSnippet" . }}
      set $backend     "{{ $readUrl }}";
      proxy_pass       $backend$request_uri;
    }
    location ^~ /loki/api/v1/ {
      proxy_set_header X-Query-Tags "${query_tags},user=${http_x_grafana_user},dashboard_id=${http_x_dashboard_uid},dashboard_title=${http_x_dashboard_title},panel_id=${http_x_panel_id},panel_title=${http_x_panel_title},source_rule_uid=${http_x_rule_uid},rule_name=${http_x_rule_name},rule_folder=${http_x_rule_folder},rule_version=${http_x_rule_version},rule_source=${http_x_rule_source},rule_type=${http_x_rule_type}";
      {{- include "mzmon.loki.gateway.locationSnippet" . }}
      set $backend     "{{ $readUrl }}";
      proxy_pass       $backend$request_uri;
    }

    location / {
      return 404 "404 page not found\n";
    }

    {{- with $nginx.serverSnippet }}

    {{ . | nindent 4 | trim }}
    {{- end }}
  }
}
{{- end }}

{{- /*
`loki.gateway.nginxConfig.locationSnippet`, indented for a proxied location.

The subchart's default snippet sets `X-Scope-OrgID` from the basic-auth user
when `loki.tenants` is set, so it is honoured here as upstream honours it —
in every location that proxies, and in none that answers on its own.
*/}}
{{- define "mzmon.loki.gateway.locationSnippet" }}
  {{- with .Values.gateway.nginxConfig.locationSnippet }}
    {{- with ( tpl . $ | trim ) }}
      {{- . | nindent 6 }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Validate the Loki gateway against the things on either side of it.

The gateway is the one hop with a server and a client of its own: Grafana dials
it, and it dials Loki. Each side has a scheme, and every mismatch fails the same
quiet way — Grafana's datasource errors, and every log panel renders empty with
nothing on the dashboard to say why. Hence errors rather than warnings.

The TLS checks apply only while `nginxConfig.file` is this chart's: a
replacement file is free to ignore `nginxConfig.tls`, and its author owns it.

Usage:
  {{- $res := include "mzmon.loki.validate.gateway" $ | fromYaml }}
*/}}
{{- define "mzmon.loki.validate.gateway" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $loki := $.Values.loki | default dict }}
  {{- $gw := $loki.gateway | default dict }}
  {{- $nginx := $gw.nginxConfig | default dict }}
  {{- $tls := $nginx.tls | default dict }}
  {{- $namespace := include "mzmon.loki.namespace" $ }}

  {{- /* The datasource, if it addresses the gateway this release runs. */}}
  {{- $dsUrl := "" }}
  {{- $dsAtGateway := false }}
  {{- if ( include "mzmon.grafana.datasource.enabled" ( dict "root" $ "name" "loki" ) ) }}
    {{- $dsUrl = tpl ( dig "datasources" "loki" "url" "" ( $.Values.connections | default dict ) | toString ) $ }}
    {{- $dsAtGateway = contains ( printf "://loki-gateway.%s.svc" $namespace ) $dsUrl }}
  {{- end }}

  {{- if not $gw.enabled }}
    {{- if $dsAtGateway }}
      {{- $errors = append $errors ( printf "loki.gateway.enabled is off, but connections.datasources.loki.url is %q, which is the gateway's Service. Nothing would answer there, and every log panel renders empty. Point the datasource at the query frontend (http://loki-query-frontend.%s.svc:3100) — Grafana then lists no Loki rules — or turn the gateway back on." $dsUrl $namespace ) }}
    {{- end }}
  {{- else if contains "mzmon.loki.gateway.nginxConf" ( $nginx.file | default "" | toString ) }}
    {{- $https := eq ( $nginx.schema | default "http" | toString ) "https" }}

    {{- /* Gateway -> Loki. */}}
    {{- if and ( include "mzmon.certificates.lokiServerTls" $ ) ( not $https ) }}
      {{- $errors = append $errors "loki.loki.server.http_tls_config is set, so Loki serves TLS on 3100 — but loki.gateway.nginxConfig.schema is not https, so the gateway would send plaintext at a TLS port. Every read through Grafana fails. Set schema: https and loki.gateway.nginxConfig.tls.upstreamCaFile, as profiles/mtls.values.yaml does." }}
    {{- end }}
    {{- if and $https ( not ( include "mzmon.certificates.lokiServerTls" $ ) ) }}
      {{- $errors = append $errors "loki.gateway.nginxConfig.schema is https, but Loki serves plaintext (loki.loki.server.http_tls_config is unset). The gateway would open TLS against a plaintext port and every read through Grafana fails. Use schema: http, or move Loki to TLS." }}
    {{- end }}
    {{- if and $https ( not $tls.upstreamCaFile ) }}
      {{- $errors = append $errors "loki.gateway.nginxConfig.schema is https with no loki.gateway.nginxConfig.tls.upstreamCaFile. The gateway verifies Loki's certificate and needs the CA to verify it against; nginx refuses to start without one." }}
    {{- end }}
    {{- if ne ( empty $tls.upstreamCertFile ) ( empty $tls.upstreamKeyFile ) }}
      {{- $errors = append $errors "loki.gateway.nginxConfig.tls sets one of upstreamCertFile and upstreamKeyFile without the other. A client certificate needs both, and nginx refuses to start with one." }}
    {{- end }}
    {{- if and $tls.upstreamCertFile ( not $https ) }}
      {{- $errors = append $errors "loki.gateway.nginxConfig.tls.upstreamCertFile is set, but schema is not https, so there is no TLS connection to present it on. The gateway would read from Loki without the certificate this setting asks for." }}
    {{- end }}

    {{- /* Grafana -> gateway. */}}
    {{- if $nginx.ssl }}
      {{- if not ( and $tls.certFile $tls.keyFile ) }}
        {{- $errors = append $errors "loki.gateway.nginxConfig.ssl is on, but loki.gateway.nginxConfig.tls.certFile and keyFile are not both set. The listener has no certificate to serve, and every handshake fails." }}
      {{- end }}
      {{- if ne ( dig "readinessProbe" "httpGet" "scheme" "" $gw | upper ) "HTTPS" }}
        {{- $errors = append $errors "loki.gateway.nginxConfig.ssl is on, but loki.gateway.readinessProbe.httpGet.scheme is not HTTPS. The kubelet probes a TLS listener over plaintext, both gateway replicas stay unready, and the Service has no endpoints." }}
      {{- end }}
      {{- if and ( dig "livenessProbe" "httpGet" "path" "" $gw ) ( ne ( dig "livenessProbe" "httpGet" "scheme" "" $gw | upper ) "HTTPS" ) }}
        {{- $errors = append $errors "loki.gateway.nginxConfig.ssl is on, but loki.gateway.livenessProbe.httpGet.scheme is not HTTPS. The probe fails against the TLS listener and the kubelet restarts the gateway on a loop." }}
      {{- end }}
    {{- else if $tls.clientCaFile }}
      {{- $errors = append $errors "loki.gateway.nginxConfig.tls.clientCaFile is set, but loki.gateway.nginxConfig.ssl is off, so there is no TLS listener to verify client certificates on. Turn ssl on, or drop clientCaFile." }}
    {{- end }}

    {{- if $dsAtGateway }}
      {{- if and $nginx.ssl ( hasPrefix "http://" $dsUrl ) }}
        {{- $errors = append $errors ( printf "loki.gateway.nginxConfig.ssl is on, but connections.datasources.loki.url is %q. Grafana would send plaintext to a TLS listener, and every log panel renders empty. Use https://." $dsUrl ) }}
      {{- end }}
      {{- if and ( not $nginx.ssl ) ( hasPrefix "https://" $dsUrl ) }}
        {{- $errors = append $errors ( printf "connections.datasources.loki.url is %q, but loki.gateway.nginxConfig.ssl is off, so the gateway answers in plaintext. Every log panel renders empty. Turn ssl on, as profiles/mtls.values.yaml does, or use http://." $dsUrl ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
