{{- /*
Cloud provider metric pulls on the alloy-gateway.

The pulls are custom components in `pre-rendered/pipelines/gateway-provider.alloy`,
rendered from `packages/alloy-pipelines/gateway-provider.yaml` through the
pipeline schema and validated by `make pipelines`. That file deploys as-is. The
only thing rendered here is instances of those components, one per resource
named in `pipeline.metrics.provider.*`: the shape is the pipeline's, and the
count is the chart's.

A `declare` body cannot repeat a block, so a component takes one resource
(CloudWatch) or one service's worth of resources (GCP, whose filter names them
all). Each instance is a flat block of arguments, and `alloy validate` rejects
an argument the component does not declare, so these instances are checked
against the module's contract at install by the pre-validate job, and at build
by `gateway-provider-stub.yaml`.
*/}}

{{- /*
Importance levels, most to least important. `all` is a destination floor only.
*/}}
{{- define "mzmon.alloyGateway.provider.importanceLevels" }}
  {{- list "essential" "recommended" "extended" "diagnostic" | toYaml }}
{{- end }}

{{- /*
The component label for one resource: a service prefix, then the resource name
with every character Alloy does not allow in a label mapped to `_`. The prefix
keeps a name that starts with a digit valid.

Usage:
  {{- include "mzmon.alloyGateway.provider.label" ( list "rds" $id ) }}
*/}}
{{- define "mzmon.alloyGateway.provider.label" }}
  {{- printf "%s_%s" ( index . 0 ) ( regexReplaceAll "[^A-Za-z0-9_]" ( index . 1 ) "_" ) }}
{{- end }}

{{- /*
Render one instance of each provider component the values call for.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.provider" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.provider" }}
  {{- $provider := dig "metrics" "provider" dict ( $.Values.pipeline | default dict ) }}
  {{- $forwardTo := "otelcol.receiver.prometheus.inputBridge.receiver" }}

  {{- $cw := $provider.cloudwatch | default dict }}
  {{- if $cw.enabled }}
    {{- range $id := dig "rds" "instances" list $cw }}

provider_cloudwatch_rds {{ include "mzmon.alloyGateway.provider.label" ( list "rds" $id ) | quote }} {
    instance_id     = {{ $id | quote }}
    region          = {{ $cw.region | default "" | quote }}
    scrape_interval = {{ $cw.scrapeInterval | quote }}
    scrape_timeout  = {{ $cw.scrapeTimeout | quote }}
    forward_to      = {{ $forwardTo }}
}
    {{- end }}
    {{- range $bucket := dig "s3" "buckets" list $cw }}

provider_cloudwatch_s3 {{ include "mzmon.alloyGateway.provider.label" ( list "s3" $bucket ) | quote }} {
    bucket          = {{ $bucket | quote }}
    region          = {{ $cw.region | default "" | quote }}
    scrape_interval = {{ $cw.scrapeInterval | quote }}
    scrape_timeout  = {{ $cw.scrapeTimeout | quote }}
    forward_to      = {{ $forwardTo }}
}
    {{- end }}
  {{- end }}

  {{- $gcp := $provider.gcp | default dict }}
  {{- if $gcp.enabled }}
    {{- $project := $gcp.projectId | default "" | toString }}
    {{- $instances := dig "cloudSql" "instances" list $gcp }}
    {{- if $instances }}
      {{- /* database_id is `project:instance`. */}}
      {{- $ids := list }}
      {{- range $instances }}
        {{- $ids = append $ids ( printf "%s:%s" $project . ) }}
      {{- end }}

provider_gcp_cloudsql "provider" {
    project_id       = {{ $project | quote }}
    database_ids     = {{ $ids | toJson }}
    request_interval = {{ $gcp.requestInterval | quote }}
    scrape_interval  = {{ $gcp.scrapeInterval | quote }}
    scrape_timeout   = {{ $gcp.scrapeTimeout | quote }}
    forward_to       = {{ $forwardTo }}
}
    {{- end }}
    {{- $buckets := dig "gcs" "buckets" list $gcp }}
    {{- if $buckets }}

provider_gcp_gcs "provider" {
    project_id       = {{ $project | quote }}
    buckets          = {{ $buckets | toJson }}
    request_interval = {{ $gcp.requestInterval | quote }}
    scrape_interval  = {{ $gcp.scrapeInterval | quote }}
    scrape_timeout   = {{ $gcp.scrapeTimeout | quote }}
    forward_to       = {{ $forwardTo }}
}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Metric-name patterns for the provider families a destination admits.

A provider family has no registry query to take a tier from, so its tier is
assigned in values (`metricImportance`) and it joins the allowlist of every
destination whose floor is at or below that tier.

Args:
  context: the root context.
  minMetricImportance: the destination's floor.

Usage:
  {{- $patterns := include "mzmon.alloyGateway.provider.metricPatterns" ( dict
        "context" $ "minMetricImportance" "extended" ) | fromYamlArray }}
*/}}
{{- define "mzmon.alloyGateway.provider.metricPatterns" }}
  {{- $ctx := .context | required "context is required" }}
  {{- $floor := .minMetricImportance | required "minMetricImportance is required" }}
  {{- $levels := include "mzmon.alloyGateway.provider.importanceLevels" $ctx | fromYamlArray }}
  {{- $provider := dig "metrics" "provider" dict ( $ctx.Values.pipeline | default dict ) }}
  {{- $patterns := list }}

  {{- /* Which families each provider produces, by its Prometheus name prefix. */}}
  {{- $families := dict
        "cloudwatch" ( list "aws_rds_.*" "aws_s3_.*" )
        "gcp" ( list "stackdriver_cloudsql_database_.*" "stackdriver_gcs_bucket_.*" ) }}

  {{- if has $floor $levels }}
    {{- $floorRank := 0 }}
    {{- range $i, $level := $levels }}
      {{- if eq $level $floor }}
        {{- $floorRank = $i }}
      {{- end }}
    {{- end }}
    {{- range $name := keys $families | sortAlpha }}
      {{- $p := get $provider $name | default dict }}
      {{- if $p.enabled }}
        {{- $rank := -1 }}
        {{- range $i, $level := $levels }}
          {{- if eq $level ( $p.metricImportance | toString ) }}
            {{- $rank = $i }}
          {{- end }}
        {{- end }}
        {{- if and ( ge $rank 0 ) ( le $rank $floorRank ) }}
          {{- $patterns = concat $patterns ( get $families $name ) }}
        {{- end }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- $patterns | toYaml }}
{{- end }}

{{- /*
A duration in whole seconds, for the `h`/`m`/`s` forms values use, or empty
when the string is not one of them.

Usage:
  {{- $seconds := include "mzmon.alloyGateway.provider.seconds" "1h30m" }}
*/}}
{{- define "mzmon.alloyGateway.provider.seconds" }}
  {{- $d := . | toString }}
  {{- if regexMatch "^([0-9]+h)?([0-9]+m)?([0-9]+s)?$" $d }}
    {{- if $d }}
      {{- $total := 0 }}
      {{- range $part := regexFindAll "[0-9]+[hms]" $d -1 }}
        {{- $n := trimSuffix "h" ( trimSuffix "m" ( trimSuffix "s" $part ) ) | atoi }}
        {{- if hasSuffix "h" $part }}{{ $total = add $total ( mul $n 3600 ) }}
        {{- else if hasSuffix "m" $part }}{{ $total = add $total ( mul $n 60 ) }}
        {{- else }}{{ $total = add $total $n }}{{ end }}
      {{- end }}
      {{- $total }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Validate the provider pulls.

Everything here is decidable before install, and every case is one that Alloy
would otherwise accept and turn into a pull that silently returns nothing, or
one that fails the whole gateway config at load with a message naming a line
number rather than a values key.

The second kind matters more than it looks. `alloy validate`, which is all the
pre-validate job runs, type-checks and does not build components, so it passes
a `scrape_timeout` longer than the interval, and two components with the same
label. `alloy run` then fails its initial load and exits, and the gateway
crashloops with every log and metric it carries. This validator is the only
thing between those values and that outcome.

Usage:
  {{- $res := include "mzmon.alloy.validate.provider" $ | fromYaml }}
*/}}
{{- define "mzmon.alloy.validate.provider" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $provider := dig "metrics" "provider" dict ( $.Values.pipeline | default dict ) }}
  {{- $levels := include "mzmon.alloyGateway.provider.importanceLevels" $ | fromYamlArray }}
  {{- $saAnnotations := dig "serviceAccount" "annotations" dict ( index $.Values "alloy-gateway" | default dict ) | default dict }}

  {{- range $name := list "cloudwatch" "gcp" }}
    {{- $p := get $provider $name | default dict }}
    {{- $path := printf "pipeline.metrics.provider.%s" $name }}
    {{- if $p.enabled }}
      {{- $interval := include "mzmon.alloyGateway.provider.seconds" $p.scrapeInterval }}
      {{- $timeout := include "mzmon.alloyGateway.provider.seconds" $p.scrapeTimeout }}
      {{- if not $interval }}
        {{- $errors = append $errors ( printf "%s.scrapeInterval is %q. Use whole hours, minutes and seconds, such as 5m or 1h30m." $path ( $p.scrapeInterval | toString ) ) }}
      {{- end }}
      {{- if not $timeout }}
        {{- $errors = append $errors ( printf "%s.scrapeTimeout is %q. Use whole hours, minutes and seconds, such as 2m." $path ( $p.scrapeTimeout | toString ) ) }}
      {{- end }}
      {{- if and $interval $timeout ( gt ( atoi $timeout ) ( atoi $interval ) ) }}
        {{- $errors = append $errors ( printf "%s.scrapeTimeout (%s) is longer than scrapeInterval (%s). Alloy refuses that at load, and the whole gateway fails to start." $path ( $p.scrapeTimeout | toString ) ( $p.scrapeInterval | toString ) ) }}
      {{- end }}
      {{- if not ( include "mzmon.alloyGateway.enabled" $ ) }}
        {{- $errors = append $errors ( printf "%s.enabled is true, but alloy-gateway is not enabled. The pull runs on the gateway, so nothing would collect it." $path ) }}
      {{- end }}
      {{- if not ( has ( $p.metricImportance | toString ) $levels ) }}
        {{- $errors = append $errors ( printf "%s.metricImportance is %q. It must be one of %s. Any other value keeps these families out of every destination that filters by tier." $path ( $p.metricImportance | toString ) ( join ", " $levels ) ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- $cw := $provider.cloudwatch | default dict }}
  {{- if $cw.enabled }}
    {{- if not $cw.region }}
      {{- $errors = append $errors "pipeline.metrics.provider.cloudwatch.enabled is true but region is empty. CloudWatch is regional and the exporter needs it to reach STS." }}
    {{- end }}
    {{- $instances := dig "rds" "instances" list $cw }}
    {{- $buckets := dig "s3" "buckets" list $cw }}
    {{- if and ( not $instances ) ( not $buckets ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.cloudwatch.enabled is true but lists no resources. Set rds.instances or s3.buckets; resources are named, never discovered." }}
    {{- end }}
    {{- /* Each resource becomes a component, and two components cannot share a
           label: the gateway would refuse its whole config at load. */}}
    {{- range $kind := list ( list "rds" "rds.instances" $instances ) ( list "s3" "s3.buckets" $buckets ) }}
      {{- $seen := dict }}
      {{- range $name := index $kind 2 }}
        {{- $label := include "mzmon.alloyGateway.provider.label" ( list ( index $kind 0 ) $name ) }}
        {{- if hasKey $seen $label }}
          {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.%s lists %q and %q, which both become the component label %q. List each resource once; names differing only in `.` and `-` cannot both be pulled." ( index $kind 1 ) ( get $seen $label ) $name $label ) }}
        {{- end }}
        {{- $_ := set $seen $label $name }}
      {{- end }}
    {{- end }}
    {{- if not ( hasKey $saAnnotations "eks.amazonaws.com/role-arn" ) }}
      {{- $warnings = append $warnings "pipeline.metrics.provider.cloudwatch is enabled with no eks.amazonaws.com/role-arn annotation on alloy-gateway.serviceAccount. The pull will use whatever the AWS default credential chain finds — EKS Pod Identity, or AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY from the mzmon-alloy-gateway-env Secret. If neither is present, every pull fails at run time." }}
    {{- end }}
  {{- end }}

  {{- $gcp := $provider.gcp | default dict }}
  {{- if $gcp.enabled }}
    {{- if not $gcp.projectId }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.enabled is true but projectId is empty." }}
    {{- end }}
    {{- $instances := dig "cloudSql" "instances" list $gcp }}
    {{- if and ( not $instances ) ( not ( dig "gcs" "buckets" list $gcp ) ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.enabled is true but lists no resources. Set cloudSql.instances or gcs.buckets; resources are named, never discovered." }}
    {{- end }}
    {{- range $instances }}
      {{- if contains ":" . }}
        {{- $errors = append $errors ( printf "pipeline.metrics.provider.gcp.cloudSql.instances entry %q contains a colon. List the instance name alone; the project is prefixed from projectId." . ) }}
      {{- end }}
    {{- end }}
    {{- if not ( include "mzmon.alloyGateway.provider.seconds" $gcp.requestInterval ) }}
      {{- $errors = append $errors ( printf "pipeline.metrics.provider.gcp.requestInterval is %q. Use whole hours, minutes and seconds, such as 10m." ( $gcp.requestInterval | toString ) ) }}
    {{- end }}
    {{- /* Unlike CloudWatch, the GCP exporter resolves its credential when it is
           built. With no ADC source at all — no GOOGLE_APPLICATION_CREDENTIALS
           and no metadata server — it fails to build, and so does the gateway's
           whole initial load. On GKE the metadata server always answers, so
           there it degrades to empty pulls instead. The render cannot tell a
           direct Workload Identity principal (valid, no annotation) from an
           install outside Google Cloud, so this is a warning. */}}
    {{- $adcSource := include "mzmon.alloy.envSource" ( dict
          "context" $ "role" "alloy-gateway" "env" "GOOGLE_APPLICATION_CREDENTIALS" ) | trim }}
    {{- if and ( not ( hasKey $saAnnotations "iam.gke.io/gcp-service-account" ) ) ( ne $adcSource "extraEnv" ) }}
      {{- $warnings = append $warnings "pipeline.metrics.provider.gcp is enabled, and the gateway has neither an iam.gke.io/gcp-service-account annotation on alloy-gateway.serviceAccount nor GOOGLE_APPLICATION_CREDENTIALS in alloy-gateway.alloy.extraEnv. On GKE the pull runs as whatever identity the metadata server gives the pod, which needs roles/monitoring.viewer or every pull returns nothing. Outside Google Cloud, with no credential at all, the exporter cannot be built and the whole gateway fails to start." }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
