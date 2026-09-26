{{- /*
Cloud provider metric pulls on the alloy-gateway.

Rendered by Helm rather than pre-rendered, for the same reason the destination
is: the shape depends on values. CloudWatch needs one `static` block per
resource, and a pre-rendered pipeline cannot repeat a block. So this output
never passes through the pipeline JSONSchema; the pre-validate job's
`alloy validate` is its check, and `gateway-provider-stub.yaml` keeps the
argument names honest at build time.

Every exporter feeds a clustered `prometheus.scrape`. Each replica runs the
exporter and exports an identical target, so the clustered scrape assigns it to
exactly one replica and the provider API is called once per interval. The
exporters call the provider on scrape, not on a timer, so the replicas that do
not own the target cost nothing. That is also why CloudWatch's
`decoupled_scraping` is never rendered: it polls on a timer in every replica,
owner or not.
*/}}

{{- /*
Importance levels, most to least important. `all` is a destination floor only.
*/}}
{{- define "mzmon.alloyGateway.provider.importanceLevels" }}
  {{- list "essential" "recommended" "extended" "diagnostic" | toYaml }}
{{- end }}

{{- /*
Render every enabled provider pull.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.provider" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.provider" }}
  {{- $provider := dig "metrics" "provider" dict ( $.Values.pipeline | default dict ) }}
  {{- if dig "cloudwatch" "enabled" false $provider }}
    {{- include "mzmon.alloyGateway.pipeline.provider.cloudwatch" $ }}
  {{- end }}
  {{- if dig "gcp" "enabled" false $provider }}
    {{- include "mzmon.alloyGateway.pipeline.provider.gcp" $ }}
  {{- end }}
{{- end }}

{{- /*
The clustered scrape every provider exporter feeds.

The exporter sets `instance` to a hash of its own arguments. That is the same
on every replica, so clustering is unaffected, but it changes whenever the
arguments do: adding one bucket would re-label every series the provider
produces. The relabel pins `instance` to the provider name. `job` stays the
exporter's own `integrations/<name>`.

Args:
  name: provider name (`cloudwatch`, `gcp`), used in component labels and `instance`.
  exporter: the exporter component reference, e.g. `prometheus.exporter.gcp.provider`.
  values: the provider's values.
*/}}
{{- define "mzmon.alloyGateway.pipeline.provider.scrape" }}
  {{- $name := .name | required "name is required" }}
  {{- $exporter := .exporter | required "exporter is required" }}
  {{- $values := .values | required "values are required" }}

discovery.relabel "provider_{{ $name }}" {
    targets = {{ $exporter }}.targets

    rule {
        target_label = "instance"
        replacement  = "{{ $name }}"
    }
}

prometheus.scrape "provider_{{ $name }}" {
    targets         = discovery.relabel.provider_{{ $name }}.output
    forward_to      = [otelcol.receiver.prometheus.inputBridge.receiver]
    scrape_interval = {{ $values.scrapeInterval | required ( printf "pipeline.metrics.provider.%s.scrapeInterval is required" $name ) | quote }}
    scrape_timeout  = {{ $values.scrapeTimeout | required ( printf "pipeline.metrics.provider.%s.scrapeTimeout is required" $name ) | quote }}

    clustering {
        enabled = true
    }
}
{{- end }}

{{- /*
Render the Cloud Monitoring pull.

Only services with resources listed contribute metric prefixes, and each
contributes a filter naming exactly those resources. A prefix without its
filter would pull every instance or bucket in the project.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.provider.gcp" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.provider.gcp" }}
  {{- $gcp := $.Values.pipeline.metrics.provider.gcp }}
  {{- /* Emptiness is reported by mzmon.alloy.validate.provider, with the values path. */}}
  {{- $project := $gcp.projectId | default "" | toString }}
  {{- $prefixes := list }}
  {{- $filters := list }}

  {{- $sql := $gcp.cloudSql | default dict }}
  {{- if $sql.instances }}
    {{- range $sql.metrics }}
      {{- $prefixes = append $prefixes ( printf "cloudsql.googleapis.com/database/%s" . ) }}
    {{- end }}
    {{- /* database_id is `project:instance`. */}}
    {{- $ids := list }}
    {{- range $sql.instances }}
      {{- $ids = append $ids ( printf "%s:%s" $project . | quote ) }}
    {{- end }}
    {{- $filters = append $filters ( printf "cloudsql.googleapis.com/database:resource.labels.database_id=one_of(%s)" ( join "," $ids ) ) }}
  {{- end }}

  {{- $gcs := $gcp.gcs | default dict }}
  {{- if $gcs.buckets }}
    {{- range $gcs.metrics }}
      {{- $prefixes = append $prefixes ( printf "storage.googleapis.com/%s" . ) }}
    {{- end }}
    {{- $names := list }}
    {{- range $gcs.buckets }}
      {{- $names = append $names ( . | quote ) }}
    {{- end }}
    {{- $filters = append $filters ( printf "storage.googleapis.com:resource.labels.bucket_name=one_of(%s)" ( join "," $names ) ) }}
  {{- end }}

prometheus.exporter.gcp "provider" {
    project_ids      = [{{ $project | quote }}]
    metrics_prefixes = [
  {{- range $prefixes }}
        {{ . | quote }},
  {{- end }}
    ]
    extra_filters = [
  {{- range $filters }}
        {{ . | quote }},
  {{- end }}
    ]
    request_interval = {{ $gcp.requestInterval | quote }}
    ingest_delay     = true
}
  {{- include "mzmon.alloyGateway.pipeline.provider.scrape" ( dict
        "name" "gcp"
        "exporter" "prometheus.exporter.gcp.provider"
        "values" $gcp ) }}
{{- end }}

{{- /*
Render the CloudWatch pull.

One `static` job per resource. The job label becomes the series' `name`
label, and Alloy requires it to be an identifier, so it is the resource name
with every other character mapped to `_` and a service prefix
(`rds_mz_prod_db`). The resource's real name is on the series as
`dimension_DBInstanceIdentifier` or `dimension_BucketName`, and that is what
to join on. S3 reports size per storage class and count across all classes,
under different `StorageType` dimensions, so each bucket takes one job per
storage type plus one for the object count, all with the same label.

`nil_to_zero` is set false on every job because Alloy defaults it to true,
which reports a datapoint that carries no value as zero. A resource CloudWatch
publishes nothing for produces no series either way (measured: `BurstBalance`
and `CPUCreditBalance` on a gp3, non-burstable instance), so this only matters
for the value-less case, and there a zero is the wrong answer too.

`period` and `length` are set on each `metric`, because the exporter ignores
them on a `static` job.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.provider.cloudwatch" $ }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.provider.cloudwatch" }}
  {{- $cw := $.Values.pipeline.metrics.provider.cloudwatch }}
  {{- /* Emptiness is reported by mzmon.alloy.validate.provider, with the values path. */}}
  {{- $region := $cw.region | default "" | toString }}
  {{- $rds := $cw.rds | default dict }}
  {{- $s3 := $cw.s3 | default dict }}

prometheus.exporter.cloudwatch "provider" {
    sts_region = {{ $region | quote }}
  {{- range $id := $rds.instances }}

    static {{ printf "rds_%s" ( regexReplaceAll "[^A-Za-z0-9_]" $id "_" ) | quote }} {
        regions     = [{{ $region | quote }}]
        namespace   = "AWS/RDS"
        nil_to_zero = false
        dimensions  = {
            "DBInstanceIdentifier" = {{ $id | quote }},
        }
    {{- include "mzmon.alloyGateway.pipeline.provider.cloudwatch.role" $cw }}
    {{- range $m := $rds.metrics }}

        metric {
            name       = {{ $m.name | quote }}
            statistics = [{{ range $i, $s := $m.statistics }}{{ if $i }}, {{ end }}{{ $s | quote }}{{ end }}]
            period     = {{ $m.period | default $rds.period | quote }}
            length     = {{ $m.length | default $rds.length | quote }}
        }
    {{- end }}
    }
  {{- end }}
  {{- range $bucket := $s3.buckets }}
    {{- $label := printf "s3_%s" ( regexReplaceAll "[^A-Za-z0-9_]" $bucket "_" ) }}
    {{- range $storageType := $s3.storageTypes }}

    static {{ $label | quote }} {
        regions     = [{{ $region | quote }}]
        namespace   = "AWS/S3"
        nil_to_zero = false
        dimensions  = {
            "BucketName"  = {{ $bucket | quote }},
            "StorageType" = {{ $storageType | quote }},
        }
      {{- include "mzmon.alloyGateway.pipeline.provider.cloudwatch.role" $cw }}

        metric {
            name       = "BucketSizeBytes"
            statistics = ["Average"]
            period     = "24h"
            length     = "48h"
        }
    }
    {{- end }}

    static {{ $label | quote }} {
        regions     = [{{ $region | quote }}]
        namespace   = "AWS/S3"
        nil_to_zero = false
        dimensions  = {
            "BucketName"  = {{ $bucket | quote }},
            "StorageType" = "AllStorageTypes",
        }
    {{- include "mzmon.alloyGateway.pipeline.provider.cloudwatch.role" $cw }}

        metric {
            name       = "NumberOfObjects"
            statistics = ["Average"]
            period     = "24h"
            length     = "48h"
        }
    }
  {{- end }}
}
  {{- include "mzmon.alloyGateway.pipeline.provider.scrape" ( dict
        "name" "cloudwatch"
        "exporter" "prometheus.exporter.cloudwatch.provider"
        "values" $cw ) }}
{{- end }}

{{- /*
The optional `role` block on a CloudWatch job.

Usage:
  {{- include "mzmon.alloyGateway.pipeline.provider.cloudwatch.role" $cw }}
*/}}
{{- define "mzmon.alloyGateway.pipeline.provider.cloudwatch.role" }}
  {{- if .roleArn }}

        role {
            role_arn = {{ .roleArn | quote }}
          {{- if .externalId }}
            external_id = {{ .externalId | quote }}
          {{- end }}
        }
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
a CloudWatch `length` shorter than its `period` and a `scrape_timeout` longer
than the interval. `alloy run` then fails its initial load and exits, and the
gateway crashloops with every log and metric it carries. This validator is the
only thing between those values and that outcome.

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
    {{- $rds := $cw.rds | default dict }}
    {{- $s3 := $cw.s3 | default dict }}
    {{- if and ( not $rds.instances ) ( not $s3.buckets ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.cloudwatch.enabled is true but lists no resources. Set rds.instances or s3.buckets; resources are named, never discovered." }}
    {{- end }}
    {{- if and $rds.instances ( not $rds.metrics ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.cloudwatch.rds.instances is set but rds.metrics is empty, so nothing would be pulled for them." }}
    {{- end }}
    {{- range $m := $rds.metrics }}
      {{- if or ( not $m.name ) ( not $m.statistics ) }}
        {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.rds.metrics has an entry without both name and statistics (%s)." ( toJson $m ) ) }}
      {{- else }}
        {{- $period := include "mzmon.alloyGateway.provider.seconds" ( $m.period | default $rds.period ) }}
        {{- $length := include "mzmon.alloyGateway.provider.seconds" ( $m.length | default $rds.length ) }}
        {{- if or ( not $period ) ( not $length ) }}
          {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.rds.metrics %s has a period or length that is not whole hours, minutes and seconds, such as 5m." $m.name ) }}
        {{- else if lt ( atoi $length ) ( atoi $period ) }}
          {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.rds.metrics %s has a length shorter than its period. The exporter refuses that when it starts, and the whole gateway fails to start." $m.name ) }}
        {{- end }}
        {{- /* Static jobs go through GetMetricStatistics, which takes the
               five basic statistics plus percentiles, and the exporter indexes
               the basic list unconditionally — so a percentile-only entry
               cannot be allowed through. */}}
        {{- $basic := 0 }}
        {{- range $stat := $m.statistics }}
          {{- if has ( $stat | toString ) ( list "Average" "Minimum" "Maximum" "Sum" "SampleCount" ) }}
            {{- $basic = add1 $basic }}
          {{- else if not ( regexMatch "^p(\\d{1,2}(\\.\\d{0,2})?|100)$" ( $stat | toString ) ) }}
            {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.rds.metrics %s requests statistic %q. Use Average, Minimum, Maximum, Sum, SampleCount, or a percentile such as p99." $m.name ( $stat | toString ) ) }}
          {{- end }}
        {{- end }}
        {{- if eq $basic 0 }}
          {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.rds.metrics %s requests only percentiles. Add at least one of Average, Minimum, Maximum, Sum or SampleCount; the exporter fails on a percentile-only request." $m.name ) }}
        {{- end }}
      {{- end }}
    {{- end }}
    {{- if and $s3.buckets ( not $s3.storageTypes ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.cloudwatch.s3.buckets is set but s3.storageTypes is empty, so no bucket size would be pulled." }}
    {{- end }}
    {{- if and ( not $cw.roleArn ) ( not ( hasKey $saAnnotations "eks.amazonaws.com/role-arn" ) ) }}
      {{- $warnings = append $warnings "pipeline.metrics.provider.cloudwatch is enabled with no roleArn and no eks.amazonaws.com/role-arn annotation on alloy-gateway.serviceAccount. The pull will use whatever the AWS default credential chain finds — EKS Pod Identity, or AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY from the mzmon-alloy-gateway-env Secret. If neither is present, every pull fails at run time." }}
    {{- end }}
  {{- end }}

  {{- $gcp := $provider.gcp | default dict }}
  {{- if $gcp.enabled }}
    {{- if not $gcp.projectId }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.enabled is true but projectId is empty." }}
    {{- end }}
    {{- $sql := $gcp.cloudSql | default dict }}
    {{- $gcs := $gcp.gcs | default dict }}
    {{- if and ( not $sql.instances ) ( not $gcs.buckets ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.enabled is true but lists no resources. Set cloudSql.instances or gcs.buckets; resources are named, never discovered." }}
    {{- end }}
    {{- if and $sql.instances ( not $sql.metrics ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.cloudSql.instances is set but cloudSql.metrics is empty, so nothing would be pulled for them." }}
    {{- end }}
    {{- if and $gcs.buckets ( not $gcs.metrics ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.gcs.buckets is set but gcs.metrics is empty, so nothing would be pulled for them." }}
    {{- end }}
    {{- range $sql.instances }}
      {{- if contains ":" . }}
        {{- $errors = append $errors ( printf "pipeline.metrics.provider.gcp.cloudSql.instances entry %q contains a colon. List the instance name alone; the project is prefixed from projectId." . ) }}
      {{- end }}
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
