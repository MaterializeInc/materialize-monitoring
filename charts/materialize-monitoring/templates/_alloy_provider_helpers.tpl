{{- /*
Cloud provider metric pulls on the alloy-gateway.

The pulls are custom components in `pre-rendered/pipelines/gateway-provider.alloy`,
rendered from `packages/alloy-pipelines/gateway-provider.yaml` through the
pipeline schema and validated by `make pipelines`. That file deploys as-is. The
only thing rendered here is instances of those components, one per resource
named in `pipeline.metrics.provider.*`: the shape is the pipeline's, and the
count is the chart's.

A `declare` body cannot repeat a block, so a component takes one resource
(CloudWatch RDS and S3) or one service's worth of resources (EKS, GCP and
Azure, whose filters name them all). Each instance is a flat block of arguments, and `alloy validate` rejects
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
    {{- $clusters := dig "eks" "clusters" list $cw }}
    {{- if $clusters }}

provider_cloudwatch_eks "provider" {
    clusters        = {{ $clusters | toJson }}
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
    {{- $regions := dig "compute" "regions" list $gcp }}
    {{- if $regions }}

provider_gcp_compute_quota "provider" {
    project_id       = {{ $project | quote }}
    regions          = {{ $regions | toJson }}
    request_interval = {{ $gcp.requestInterval | quote }}
    scrape_interval  = {{ $gcp.scrapeInterval | quote }}
    scrape_timeout   = {{ $gcp.scrapeTimeout | quote }}
    forward_to       = {{ $forwardTo }}
}
    {{- end }}
  {{- end }}

  {{- $az := $provider.azure | default dict }}
  {{- if $az.enabled }}
    {{- $subscription := $az.subscriptionId | default "" | toString }}
    {{- $servers := dig "postgres" "servers" list $az }}
    {{- if $servers }}

provider_azure_postgres "provider" {
    subscription_id   = {{ $subscription | quote }}
    servers           = {{ $servers | toJson }}
    cloud_environment = {{ $az.cloudEnvironment | quote }}
    scrape_interval   = {{ $az.scrapeInterval | quote }}
    scrape_timeout    = {{ $az.scrapeTimeout | quote }}
    forward_to        = {{ $forwardTo }}
}
    {{- end }}
    {{- $accounts := dig "blob" "storageAccounts" list $az }}
    {{- if $accounts }}

provider_azure_blob "provider" {
    subscription_id   = {{ $subscription | quote }}
    storage_accounts  = {{ $accounts | toJson }}
    cloud_environment = {{ $az.cloudEnvironment | quote }}
    scrape_interval   = {{ $az.scrapeInterval | quote }}
    scrape_timeout    = {{ $az.scrapeTimeout | quote }}
    forward_to        = {{ $forwardTo }}
}
    {{- end }}
    {{- $clusters := dig "aks" "clusters" list $az }}
    {{- if $clusters }}

provider_azure_aks "provider" {
    subscription_id   = {{ $subscription | quote }}
    clusters          = {{ $clusters | toJson }}
    cloud_environment = {{ $az.cloudEnvironment | quote }}
    scrape_interval   = {{ $az.scrapeInterval | quote }}
    scrape_timeout    = {{ $az.scrapeTimeout | quote }}
    forward_to        = {{ $forwardTo }}
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
        "cloudwatch" ( list "aws_rds_.*" "aws_s3_.*" "aws_ec2_.*" "aws_autoscaling_.*" "aws_usage_.*" )
        "gcp" ( list "stackdriver_cloudsql_database_.*" "stackdriver_gcs_bucket_.*" "stackdriver_compute_googleapis_com_location_.*" )
        "azure" ( list "azure_microsoft_dbforpostgresql_flexibleservers_.*" "azure_microsoft_storage_storageaccounts_blobservices_.*" "azure_microsoft_containerservice_managedclusters_.*" "azure_microsoft_compute_virtualmachinescalesets_.*" ) }}

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

  {{- range $name := list "cloudwatch" "gcp" "azure" }}
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
    {{- $clusters := dig "eks" "clusters" list $cw }}
    {{- if and ( not $instances ) ( not $buckets ) ( not $clusters ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.cloudwatch.enabled is true but lists no resources. Set rds.instances, s3.buckets or eks.clusters; resources are named, never discovered." }}
    {{- end }}
    {{- /* The names are joined into an anchored regex for the tag filters, so
           a character outside EKS's own naming rule would change what it
           matches rather than fail. */}}
    {{- range $clusters }}
      {{- if not ( kindIs "string" . ) }}
        {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.eks.clusters entry %v is a %s, not a string. Quote it." . ( kindOf . ) ) }}
      {{- else if not ( regexMatch "^[0-9A-Za-z][A-Za-z0-9_-]{0,99}$" . ) }}
        {{- $errors = append $errors ( printf "pipeline.metrics.provider.cloudwatch.eks.clusters entry %q is not an EKS cluster name. List the name alone — letters, digits, hyphens and underscores — not its ARN or endpoint." . ) }}
      {{- end }}
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
    {{- $regions := dig "compute" "regions" list $gcp }}
    {{- if and ( not $instances ) ( not ( dig "gcs" "buckets" list $gcp ) ) ( not $regions ) }}
      {{- $errors = append $errors "pipeline.metrics.provider.gcp.enabled is true but lists no resources. Set cloudSql.instances, gcs.buckets or compute.regions; resources are named, never discovered." }}
    {{- end }}
    {{- /* A region goes into a Cloud Monitoring regex that also matches its
           zones, so a zone would match nothing and silently pull nothing. */}}
    {{- range $regions }}
      {{- if not ( regexMatch "^[a-z]+-[a-z]+[0-9]+$" ( . | toString ) ) }}
        {{- $errors = append $errors ( printf "pipeline.metrics.provider.gcp.compute.regions entry %q is not a Compute Engine region. List the region, such as us-east1; its zones are included." ( . | toString ) ) }}
      {{- end }}
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

  {{- $az := $provider.azure | default dict }}
  {{- if $az.enabled }}
    {{- $path := "pipeline.metrics.provider.azure" }}
    {{- $subscription := $az.subscriptionId | default "" | toString }}
    {{- if not $subscription }}
      {{- $errors = append $errors ( printf "%s.enabled is true but subscriptionId is empty." $path ) }}
    {{- else if not ( regexMatch "^[0-9a-fA-F]{8}-([0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}$" $subscription ) }}
      {{- $errors = append $errors ( printf "%s.subscriptionId is %q. Use the subscription ID, a GUID, not its display name." $path $subscription ) }}
    {{- end }}
    {{- $environments := list "azurecloud" "azurechinacloud" "azureusgovernmentcloud" }}
    {{- if not ( has ( $az.cloudEnvironment | toString ) $environments ) }}
      {{- $errors = append $errors ( printf "%s.cloudEnvironment is %q. It must be one of %s." $path ( $az.cloudEnvironment | toString ) ( join ", " $environments ) ) }}
    {{- end }}
    {{- $servers := dig "postgres" "servers" list $az }}
    {{- $accounts := dig "blob" "storageAccounts" list $az }}
    {{- $clusters := dig "aks" "clusters" list $az }}
    {{- if and ( not $servers ) ( not $accounts ) ( not $clusters ) }}
      {{- $errors = append $errors ( printf "%s.enabled is true but lists no resources. Set postgres.servers, blob.storageAccounts or aks.clusters; resources are named, never discovered." $path ) }}
    {{- end }}
    {{- /* The names go into a Kusto filter between single quotes, and a name the
           filter does not match is a pull that silently returns nothing. Both
           shapes are Azure's own naming rules, so anything else is a mistake:
           usually an FQDN, an endpoint URL or a resource ID.

           Each must also be a string. An all-digit name is valid on Azure, and
           unquoted YAML reads it as a number, which does not round-trip: Helm
           parses `012345` as octal and renders 5349, which then passes the
           shape check. */}}
    {{- range $servers }}
      {{- if not ( kindIs "string" . ) }}
        {{- $errors = append $errors ( printf "%s.postgres.servers entry %v is a %s, not a string. Quote it: YAML does not keep a numeric name as written." $path . ( kindOf . ) ) }}
      {{- else if not ( regexMatch "^[a-z0-9][a-z0-9-]*$" . ) }}
        {{- $errors = append $errors ( printf "%s.postgres.servers entry %q is not a Flexible Server name. List the name alone — lowercase letters, digits and hyphens — not its FQDN or resource ID." $path . ) }}
      {{- end }}
    {{- end }}
    {{- range $accounts }}
      {{- if not ( kindIs "string" . ) }}
        {{- $errors = append $errors ( printf "%s.blob.storageAccounts entry %v is a %s, not a string. Quote it: YAML does not keep a numeric name as written." $path . ( kindOf . ) ) }}
      {{- else if not ( regexMatch "^[a-z0-9]+$" . ) }}
        {{- $errors = append $errors ( printf "%s.blob.storageAccounts entry %q is not a storage account name. List the name alone — lowercase letters and digits — not its endpoint or resource ID." $path . ) }}
      {{- end }}
    {{- end }}
    {{- range $clusters }}
      {{- if not ( kindIs "string" . ) }}
        {{- $errors = append $errors ( printf "%s.aks.clusters entry %v is a %s, not a string. Quote it: YAML does not keep a numeric name as written." $path . ( kindOf . ) ) }}
      {{- else if not ( regexMatch "^[A-Za-z0-9][A-Za-z0-9_-]{0,62}$" . ) }}
        {{- $errors = append $errors ( printf "%s.aks.clusters entry %q is not an AKS cluster name. List the name alone — letters, digits, hyphens and underscores — not its resource ID." $path . ) }}
      {{- end }}
    {{- end }}
    {{- /* Like CloudWatch, and unlike GCP, the credential is resolved on the first
           pull rather than when the exporter is built, so no case here stops
           the gateway. With no workload identity injected, the default chain
           reaches the node's managed identity through IMDS instead. Measured on
           AKS, Resource Graph refuses that identity outright, so `up` is 0 and
           the gateway logs `service discovery failed`. A node identity that can
           read anything in the subscription would instead get an empty result,
           with `up` at 1. */}}
    {{- $podLabels := dig "controller" "podLabels" dict ( index $.Values "alloy-gateway" | default dict ) | default dict }}
    {{- $clientIdSource := include "mzmon.alloy.envSource" ( dict
          "context" $ "role" "alloy-gateway" "env" "AZURE_CLIENT_ID" ) | trim }}
    {{- /* An empty annotation names no identity, so it counts as absent. */}}
    {{- if get $saAnnotations "azure.workload.identity/client-id" }}
      {{- if ne ( get $podLabels "azure.workload.identity/use" | toString ) "true" }}
        {{- $warnings = append $warnings ( printf "%s is enabled and alloy-gateway.serviceAccount carries an azure.workload.identity/client-id annotation, but alloy-gateway.controller.podLabels has no azure.workload.identity/use: \"true\". The Entra webhook injects the workload identity only into pods with that label, so without it the pull runs as the node's managed identity. That identity usually has no access to the subscription, so Resource Graph refuses every pull and up is 0." $path ) }}
      {{- end }}
    {{- else if ne $clientIdSource "extraEnv" }}
      {{- $warnings = append $warnings ( printf "%s is enabled with no azure.workload.identity/client-id annotation on alloy-gateway.serviceAccount and no AZURE_CLIENT_ID in alloy-gateway.alloy.extraEnv. The pull will use whatever the Azure default credential chain finds — AZURE_CLIENT_ID with AZURE_CLIENT_SECRET from the mzmon-alloy-gateway-env Secret, or on AKS the node's managed identity. The identity needs Monitoring Reader on each named resource. Without any access in the subscription every pull is refused and up is 0; with access elsewhere but not to a named resource, that resource is silently left out." $path ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
