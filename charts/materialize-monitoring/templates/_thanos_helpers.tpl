{{- /* Thanos helpers and validators. */}}

{{- /*
Check if thanos is enabled.

This returns a truthy string if enabled and a falsy string (empty) if not.

Usage:
  {{- if ( include "mzmon.thanos.enabled" $ ) }}
    ...
  {{- end }}
*/}}
{{- define "mzmon.thanos.enabled" }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}
  {{- $tags := $.Values.tags }}
  {{- if hasKey $values "enabled" }}
    {{- ternary "true" "" $values.enabled }}
  {{- else }}
    {{- if ( or $tags.default ( index $tags "bundled-backends" ) $tags.thanos ) }}
      {{- "true" }}
    {{- end }}
  {{- end }}
{{- end }}

{{- /*
Get thanos namespace.

Usage:
  {{- include "mzmon.thanos.namespace" $ }}
*/}}
{{- define "mzmon.thanos.namespace" }}
  {{- $ns := $.Values.thanos.namespaceOverride | default ( include "mzmon.namespace" $ ) }}
  {{- printf "%s" $ns }}
{{- end }}

{{- /*
Validate the bundled Thanos configuration.

Usage:
  {{- $res := include "mzmon.thanos.validate" $ | fromYaml }}
*/}}
{{- define "mzmon.thanos.validate" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- if ( include "mzmon.thanos.enabled" $ ) }}
    {{- $res := include "mzmon.thanos.validate.objstore" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- $res := include "mzmon.thanos.validate.topology" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}
  {{- end }}

  {{- /* Reachability runs whether or not Thanos is enabled: the point is to
         catch writers and readers aimed at a Thanos that is not there. */}}
  {{- $res := include "mzmon.thanos.validate.reachability" $ | fromYaml }}
  {{- $errors = concat $errors $res.errors | default list }}
  {{- $warnings = concat $warnings $res.warnings | default list }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the Thanos object-storage configuration.

Two things go wrong here and neither surfaces until a pod is already crashing:
the placeholder config shipping as-is, and a workload-identity annotation that
names a different cloud than the objstore backend.
*/}}
{{- define "mzmon.thanos.validate.objstore" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}
  {{- $objstore := dig "global" "objstore" dict $values }}
  {{- $config := dig "config" "" $objstore | toString }}

  {{- if dig "createSecret" false $objstore }}
    {{- /* The default `config` is a placeholder carrying a `fail` directive,
           which the Thanos subchart `tpl`-renders — so an unset config already
           aborts the install from `secret-objstore.yaml`. Restating it here
           costs nothing and names both ways out, for whichever fires first. */}}
    {{- if or ( not $config ) ( contains "Either configure this secret" $config ) }}
      {{- $errors = append $errors "thanos.global.objstore.config is still the placeholder. Set a real objstore config, or set thanos.global.objstore.createSecret=false and provide the Secret named by thanos.global.objstore.secretName yourself." }}
    {{- end }}
  {{- else if not ( dig "secretName" "" $objstore ) }}
    {{- $errors = append $errors "thanos.global.objstore.createSecret is false, so thanos.global.objstore.secretName must name an existing Secret holding the objstore config." }}
  {{- end }}

  {{- /* Backend the config names, and the cloud its identity annotation implies. */}}
  {{- $lower := lower $config }}
  {{- $backend := "" }}
  {{- if regexMatch "type:\\s*s3" $lower }}
    {{- $backend = "AWS" }}
  {{- else if regexMatch "type:\\s*gcs" $lower }}
    {{- $backend = "GCP" }}
  {{- else if regexMatch "type:\\s*azure" $lower }}
    {{- $backend = "Azure" }}
  {{- end }}

  {{- $annotations := dig "global" "serviceAccount" "annotations" dict $values }}
  {{- $identity := "" }}
  {{- $identityKey := "" }}
  {{- if hasKey $annotations "eks.amazonaws.com/role-arn" }}
    {{- $identity = "AWS" }}
    {{- $identityKey = "eks.amazonaws.com/role-arn" }}
  {{- else if hasKey $annotations "iam.gke.io/gcp-service-account" }}
    {{- $identity = "GCP" }}
    {{- $identityKey = "iam.gke.io/gcp-service-account" }}
  {{- else if hasKey $annotations "azure.workload.identity/client-id" }}
    {{- $identity = "Azure" }}
    {{- $identityKey = "azure.workload.identity/client-id" }}
  {{- end }}

  {{- if and $backend $identity ( ne $backend $identity ) }}
    {{- $errors = append $errors ( printf "thanos.global.serviceAccount.annotations carries %q (%s workload identity) but thanos.global.objstore.config names a %s backend. One of the two is wrong; the pod will fail to authenticate to object storage." $identityKey $identity $backend ) }}
  {{- end }}

  {{- /* No identity and no inline credentials means the pod falls back to
         ambient node credentials, which works but is rarely intended. */}}
  {{- if and $backend ( not $identity ) }}
    {{- if not ( regexMatch "(access_key|secret_key|service_account|storage_account_key)" $lower ) }}
      {{- $warnings = append $warnings ( printf "thanos.global.objstore.config names a %s backend but thanos.global.serviceAccount.annotations sets no workload-identity annotation and the config carries no inline credentials. Thanos will fall back to ambient node credentials." $backend ) }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate the Thanos component topology.
*/}}
{{- define "mzmon.thanos.validate.topology" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}

  {{- if not ( dig "query" "enabled" false $values ) }}
    {{- $errors = append $errors "thanos.query.enabled is false, so nothing serves PromQL. The Thanos datasource and every metrics dashboard read through Query." }}
  {{- end }}

  {{- if not ( dig "receive" "enabled" false $values ) }}
    {{- $warnings = append $warnings "thanos.receive.enabled is false, so there is no remote-write endpoint. Metrics only land if something else writes blocks to object storage." }}
  {{- else }}
    {{- $res := include "mzmon.thanos.validate.receiveReplication" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}
  {{- end }}

  {{- if not ( dig "storegateway" "enabled" false $values ) }}
    {{- $warnings = append $warnings "thanos.storegateway.enabled is false, so queries cannot read historical blocks from object storage — only what Receive still holds locally." }}
  {{- end }}

  {{- if not ( dig "compactor" "enabled" false $values ) }}
    {{- $warnings = append $warnings "thanos.compactor.enabled is false: blocks are never compacted or downsampled and thanos.compactor.retention is not enforced, so object-storage cost grows without bound." }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate Receive's redundancy.

`mode` is a *topology* choice, not an availability one, and conflating the two
produces a false alarm on the chart defaults:

  - `standalone` is RouterIngestor mode — one workload that both routes and
    ingests. It still builds a ketama hashring across the StatefulSet pods
    (`hashrings.autogen`), so N replicas shard writes and add capacity.
  - `split` separates Router and Ingester so they scale independently.

What actually determines redundancy is the **replication factor**, and the
chart only passes `--receive.replication-factor` in split mode. Standalone
therefore runs at Thanos's default of 1 unless `receive.extraArgs` sets it:
every series lands on exactly one ingester, so losing a pod fails the writes
that hash to it (the ring assignment is deterministic — writes are not
rerouted) and leaves up to `tsdb.retention` of not-yet-uploaded data with no
second copy.
*/}}
{{- define "mzmon.thanos.validate.receiveReplication" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}
  {{- $split := eq ( dig "receive" "mode" "standalone" $values ) "split" }}

  {{- /* Resolve (replicas, replication factor, where each came from) per mode. */}}
  {{- $replicas := 1 }}
  {{- $rf := 1 }}
  {{- $replicaPath := "" }}
  {{- $rfPath := "" }}
  {{- if $split }}
    {{- $replicas = dig "receive" "ingester" "replicaCount" 1 $values | int }}
    {{- $rf = dig "receive" "router" "replicationFactor" 1 $values | int }}
    {{- $replicaPath = "thanos.receive.ingester.replicaCount" }}
    {{- $rfPath = "thanos.receive.router.replicationFactor" }}
  {{- else }}
    {{- $replicas = dig "receive" "replicaCount" 1 $values | int }}
    {{- $replicaPath = "thanos.receive.replicaCount" }}
    {{- $rfPath = "the --receive.replication-factor entry in thanos.receive.extraArgs" }}
    {{- /* Standalone gets no --receive.replication-factor from the chart, so
           extraArgs is the only place it can come from. */}}
    {{- range ( dig "receive" "extraArgs" list $values ) }}
      {{- if hasPrefix "--receive.replication-factor=" ( . | toString ) }}
        {{- $rf = ( trimPrefix "--receive.replication-factor=" ( . | toString ) ) | int }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* Thanos writeQuorum() is `(replicationFactor / 2) + 1` on integer
         division, so tolerated losses are `rf - quorum`. */}}
  {{- $quorum := add ( div $rf 2 ) 1 | int }}
  {{- $tolerated := sub $rf $quorum | int }}

  {{- if le $replicas 1 }}
    {{- $warnings = append $warnings ( printf "%s is 1: a single ingestion path, so any restart or node loss drops writes and takes its un-uploaded TSDB with it." $replicaPath ) }}
  {{- else if gt $rf $replicas }}
    {{- $errors = append $errors ( printf "The replication factor is %d (%s) but %s is %d. Each write is forwarded to that many ingesters, so it can never reach quorum." $rf $rfPath $replicaPath $replicas ) }}
  {{- else if eq $rf 1 }}
    {{- $hint := ternary "Raise it to 3 for a quorum that tolerates one loss." "Set thanos.receive.extraArgs: [\"--receive.replication-factor=3\"] for a quorum that tolerates one loss." $split }}
    {{- $warnings = append $warnings ( printf "%s is %d, so writes are sharded across a hashring — but the replication factor is 1, so each series lands on exactly one ingester. Losing one pod fails the writes that hash to it and leaves its un-uploaded TSDB with no second copy. %s" $replicaPath $replicas $hint ) }}
  {{- else if eq $tolerated 0 }}
    {{- /* The trap, and the reason a factor of 2 is the wrong instinct:
           quorum is `rf/2 + 1`, so at rf=2 every copy must succeed. That is
           the same fault tolerance as rf=1, and on a small ring it is worse —
           more series depend on any one pod while quorum still demands all of
           their copies. On a 3-pod ring, losing one pod fails ~1/3 of series
           at rf=1 but ~2/3 at rf=2. */}}
    {{- $warnings = append $warnings ( printf "The replication factor is %d (%s), so Thanos write quorum is (rf/2)+1 = %d — every copy must succeed and no ingester can be lost. That is the same fault tolerance as a factor of 1, and on a small ring it fails a larger share of writes, because more series depend on any one pod while quorum still demands all their copies. Use an odd factor: 3 gives a quorum of 2 and tolerates one loss." $rf $rfPath $quorum ) }}
  {{- else if eq ( mod $rf 2 ) 0 }}
    {{- /* Even factors above 2 do tolerate losses, but never more than the
           odd factor below them — the extra copy buys durability, not
           availability. */}}
    {{- $warnings = append $warnings ( printf "The replication factor is %d (%s) is even: quorum is (rf/2)+1 = %d, tolerating %d ingester loss — the same as a factor of %d, for one more copy per write. The extra copy adds durability, not availability. Prefer an odd factor." $rf $rfPath $quorum $tolerated ( sub $rf 1 ) ) }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}

{{- /*
Validate that the things pointed at Thanos can actually reach it.

Catches the two silent failures: metrics written to a Receive that is not
deployed, and a Query Frontend deployed that nothing reads through.
*/}}
{{- define "mzmon.thanos.validate.reachability" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}
  {{- $enabled := include "mzmon.thanos.enabled" $ }}

  {{- /* Only meaningful when something is actually writing: with the gateway
         disabled, a stale destination URL is inert. */}}
  {{- $promDest := $.Values.pipeline.metrics.gateway.destination.prometheusRemoteWrite }}
  {{- if and ( include "mzmon.alloyGateway.enabled" $ ) $promDest.enabled }}
    {{- $url := tpl ( $promDest.url | toString ) $ }}
    {{- /* Require the in-cluster Service shape, so an external host that
           happens to be named `thanos-receive.<domain>` is not flagged. */}}
    {{- if and ( contains "thanos-receive" $url ) ( contains ".svc" $url ) }}
      {{- if not $enabled }}
        {{- $errors = append $errors ( printf "pipeline.metrics.gateway.destination.prometheusRemoteWrite.url points at the bundled Thanos (%s) but Thanos is not enabled. Metrics would be written to a Service that does not exist." $url ) }}
      {{- else if not ( dig "receive" "enabled" false $values ) }}
        {{- $errors = append $errors ( printf "pipeline.metrics.gateway.destination.prometheusRemoteWrite.url points at thanos-receive (%s) but thanos.receive.enabled is false." $url ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- if ( include "mzmon.grafana.datasource.enabled" ( dict "root" $ "name" "thanos" ) ) }}
    {{- $dsUrl := tpl ( $.Values.connections.datasources.thanos.url | toString ) $ }}
    {{- if and ( contains "thanos-query" $dsUrl ) ( contains ".svc" $dsUrl ) ( not $enabled ) }}
      {{- $errors = append $errors ( printf "connections.datasources.thanos is enabled and points at the bundled Thanos (%s), but Thanos is not enabled." $dsUrl ) }}
    {{- end }}
    {{- /* Query Frontend is the caching read path. Deploying it while the
           datasource still addresses Query means the cache is bypassed. */}}
    {{- if and $enabled ( dig "queryFrontend" "enabled" false $values ) }}
      {{- if not ( contains "query-frontend" $dsUrl ) }}
        {{- $warnings = append $warnings ( printf "thanos.queryFrontend.enabled is true but connections.datasources.thanos.url (%s) does not address it, so reads bypass the cache. Point the datasource at the query-frontend Service." $dsUrl ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
