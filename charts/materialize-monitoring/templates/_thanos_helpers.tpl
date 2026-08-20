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
Effective Receive replication factor.

The chart only passes `--receive.replication-factor` in split mode, so
standalone falls back to Thanos's default of 1 unless `receive.extraArgs`
supplies it.

Usage:
  {{- $rf := include "mzmon.thanos.receive.replicationFactor" $ | int }}
*/}}
{{- define "mzmon.thanos.receive.replicationFactor" }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}
  {{- if eq ( dig "receive" "mode" "standalone" $values ) "split" }}
    {{- dig "receive" "router" "replicationFactor" 1 $values | int }}
  {{- else }}
    {{- $rf := 1 }}
    {{- range ( dig "receive" "extraArgs" list $values ) }}
      {{- if hasPrefix "--receive.replication-factor=" ( . | toString ) }}
        {{- $rf = ( trimPrefix "--receive.replication-factor=" ( . | toString ) ) | int }}
      {{- end }}
    {{- end }}
    {{- $rf }}
  {{- end }}
{{- end }}

{{- /*
Whether a PodDisruptionBudget renders for a Thanos component.

The subchart tests `or <component>.pdb.enabled global.pdb.enabled`, so a
global `true` cannot be opted out of per component.

Usage:
  {{- if ( include "mzmon.thanos.pdb.enabled" ( dict "root" $ "name" "receive" ) ) }}
*/}}
{{- define "mzmon.thanos.pdb.enabled" }}
  {{- $values := .root.Values.thanos | required "thanos is missing from values." }}
  {{- if or ( dig .name "pdb" "enabled" false $values ) ( dig "global" "pdb" "enabled" false $values ) }}
    {{- "true" }}
  {{- end }}
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

    {{- $res := include "mzmon.thanos.validate.disruption" $ | fromYaml }}
    {{- $errors = concat $errors $res.errors | default list }}
    {{- $warnings = concat $warnings $res.warnings | default list }}

    {{- $res := include "mzmon.thanos.validate.networkPolicy" $ | fromYaml }}
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
Validate voluntary-disruption and autoscaling settings.

Three failure shapes, none of which surface until a node drain or an upgrade:

  - a multi-replica component with no PDB, so a drain can take all of it;
  - a Receive PDB that permits more simultaneous loss than write quorum
    tolerates, so a routine drain breaks ingestion;
  - `minAvailable` on the single-replica Compactor, which permits no eviction
    at all and hangs node drains indefinitely.

Autoscaling has its own trap: the subchart templates `replicas:` on every
workload unconditionally, including the ones that also ship an HPA. Every
`helm upgrade` — and every GitOps reconcile — therefore resets the replica
count out from under the HPA.
*/}}
{{- define "mzmon.thanos.validate.disruption" }}
  {{- $errors := list }}
  {{- $warnings := list }}
  {{- $values := $.Values.thanos | required "thanos is missing from values." }}

  {{- /* Components that can carry a PDB, and where their replica count lives. */}}
  {{- $components := list
        ( dict "name" "query" "replicaPath" "query.replicaCount" "default" 2 )
        ( dict "name" "queryFrontend" "replicaPath" "queryFrontend.replicaCount" "default" 2 )
        ( dict "name" "storegateway" "replicaPath" "storegateway.replicaCount" "default" 2 )
        ( dict "name" "receive" "replicaPath" "receive.replicaCount" "default" 3 )
        ( dict "name" "ruler" "replicaPath" "ruler.replicaCount" "default" 2 ) }}

  {{- range $components }}
    {{- $name := .name }}
    {{- if dig $name "enabled" false $values }}
      {{- $replicas := dig $name "replicaCount" .default $values | int }}
      {{- $hasPdb := include "mzmon.thanos.pdb.enabled" ( dict "root" $ "name" $name ) }}
      {{- if and ( gt $replicas 1 ) ( not $hasPdb ) }}
        {{- $warnings = append $warnings ( printf "thanos.%s runs %d replicas with no PodDisruptionBudget, so a node drain can evict all of them at once. Set thanos.global.pdb.enabled=true (maxUnavailable: 1)." $name $replicas ) }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* Receive's PDB has to stay inside the write-quorum budget. */}}
  {{- if dig "receive" "enabled" false $values }}
    {{- $hasPdb := include "mzmon.thanos.pdb.enabled" ( dict "root" $ "name" "receive" ) }}
    {{- if $hasPdb }}
      {{- /* Component value wins only when non-empty; upstream's own PDB
             template resolves it with the same `| default` chain, and `dig`
             alone would stop at the empty per-component default. */}}
      {{- $maxUnavail := ( dig "receive" "pdb" "maxUnavailable" "" $values ) | default ( dig "global" "pdb" "maxUnavailable" "" $values ) }}
      {{- $rf := include "mzmon.thanos.receive.replicationFactor" $ | int }}
      {{- $tolerated := sub $rf ( add ( div $rf 2 ) 1 ) | int }}
      {{- /* Only meaningful once replication provides a budget to respect. At
             a factor of 1 or 2 nothing is tolerated, but the fix is the
             replication factor — which the replication validator already
             reports — not a maxUnavailable of 0, which would block every
             drain. Reporting both would double up on one root cause.
             A percentage cannot be compared to a pod count, so it is skipped. */}}
      {{- if and $maxUnavail ( gt $tolerated 0 ) ( not ( hasSuffix "%" ( $maxUnavail | toString ) ) ) }}
        {{- if gt ( $maxUnavail | int ) $tolerated }}
          {{- $errors = append $errors ( printf "The Receive PodDisruptionBudget allows %v pods unavailable, but a replication factor of %d only tolerates %d (write quorum is (rf/2)+1). A node drain would break ingestion. Lower maxUnavailable, or raise the replication factor." $maxUnavail $rf $tolerated ) }}
        {{- end }}
      {{- end }}
      {{- if ( dig "receive" "pdb" "minAvailable" "" $values ) | default ( dig "global" "pdb" "minAvailable" "" $values ) }}
        {{- $warnings = append $warnings "The Receive PodDisruptionBudget uses minAvailable. Prefer maxUnavailable, which scales with the replica count and stays comparable against the write-quorum budget." }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* The Compactor is a singleton: minAvailable on it deadlocks drains. */}}
  {{- if dig "compactor" "enabled" false $values }}
    {{- if include "mzmon.thanos.pdb.enabled" ( dict "root" $ "name" "compactor" ) }}
      {{- if ( dig "compactor" "pdb" "minAvailable" "" $values ) | default ( dig "global" "pdb" "minAvailable" "" $values ) }}
        {{- $warnings = append $warnings "The Compactor is a single-replica singleton, and its PodDisruptionBudget sets minAvailable — which permits no voluntary eviction, so node drains hang indefinitely. Use maxUnavailable: 1 instead; on a singleton it is a harmless no-op." }}
      {{- end }}
    {{- end }}
  {{- end }}

  {{- /* HPA vs the statically templated replica count.
         The subchart renders `replicas:` unconditionally, so every upgrade and
         every GitOps reconcile writes it back. That is only disruptive when it
         disagrees with the autoscaling floor: equal values make the reset a
         no-op, so this warns on the mismatch rather than on autoscaling
         itself. Components that are disabled are inert and say nothing. */}}
  {{- range list "query" "queryFrontend" "storegateway" }}
    {{- $name := . }}
    {{- if and ( dig $name "enabled" false $values ) ( dig $name "autoscaling" "enabled" false $values ) }}
      {{- $minReplicas := dig $name "autoscaling" "minReplicas" 0 $values | int }}
      {{- $replicas := dig $name "replicaCount" 2 $values | int }}
      {{- if and $minReplicas ( ne $replicas $minReplicas ) }}
        {{- $warnings = append $warnings ( printf "thanos.%s.replicaCount is %d but autoscaling.minReplicas is %d. The subchart templates a static replicas value even when an HPA exists, so every helm upgrade or GitOps reconcile resets the replica count to %d and the HPA has to scale back out. Set them equal so the reset lands on the autoscaling floor." $name $replicas $minReplicas $replicas ) }}
      {{- end }}
    {{- end }}
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

{{- /*
Validate the Thanos NetworkPolicies.

There is exactly one knob — `thanos.global.networkPolicies` — and the rules
behind it are fixed by the subchart, so this checks the switch and says what the
switch actually buys. That is the useful part: the policies it renders allow
ingress on each component's own service ports *from anywhere*, which is a
narrower claim than "Thanos is policed" and worth not misreading.

Usage:
  {{- $res := include "mzmon.thanos.validate.networkPolicy" $ | fromYaml }}
*/}}
{{- define "mzmon.thanos.validate.networkPolicy" }}
  {{- $errors := list }}
  {{- $warnings := list }}

  {{- $global := $.Values.thanos.global | default dict }}
  {{- if not $global.networkPolicies }}
    {{- $warnings = append $warnings "thanos.global.networkPolicies is recommended in production. It is the subchart's only switch, and it closes every port on the Thanos pods that is not a declared service port." }}
  {{- end }}

  {{- /* final output */}}
  {{- dict "errors" $errors "warnings" $warnings | toYaml }}
{{- end }}
