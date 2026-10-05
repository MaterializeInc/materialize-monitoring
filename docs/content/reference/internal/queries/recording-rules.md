---
title: "Authoring Recording Rules"
weight: 15
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Authoring Recording Rules

A recording rule is written once, in the query registry, under `rules:`, and becomes a rule through `mz-monitoring-build gen-rules`, beside the alerts.
The chart installs it as a `PrometheusRule`, the Thanos ruler evaluates it, and its result travels back through the alloy-gateway into Thanos like any other metric.
This page is the conventions a contributor follows when adding one, and what the tooling checks.
The first recording rules are the normalized `ext:*` layer for the metadata database, and the [external dependency design doc]({{< relref "../design-docs/20260920-external-dependency-monitoring.md" >}}#a-normalized-contract-with-flavor-native-passthrough) is why that layer exists.
[Authoring Alerts]({{< relref "alerts.md" >}}) covers what the two kinds of rule share: the alerting context, its placeholders, and capability inference.

{{< rfc-2119 >}}

<!--
Agent note: the `alerting` skill routes here for recording rules. Keep "What gen-rules rejects" in step with
render_record() and check_record_collisions() in packages/mzmon-lib/src/query/rules/render.rs.
-->

## From registry entry to recorded series

| Stage | Where | What happens |
|---|---|---|
| Author | `packages/queries/*.yaml`, a `rules:` entry | The rule names its query inline or by `queryId`, its `record` name, its `group`, and static `labels` |
| Validate | `bin/mz-monitoring-check check-queries` (pre-commit) | Schema validation, including the `record` name's shape |
| Render | `make rules` (`mz-monitoring-build gen-rules`) | Renders through the alerting context, infers capabilities, and fails on any problem |
| Output | `charts/materialize-monitoring/pre-rendered/rules/prometheus/` | The registry file's groups, recording rules beside any alerts, and an entry under `records:` in `_index.yaml`, keyed `<group>/<record>` |
| Install | `templates/alerts/prometheusrules.yaml` | Fills the placeholders, and keeps every recording rule whose capabilities are present |
| Evaluate | The Thanos ruler | Remote-writes the result to the alloy-gateway, which forwards it to every destination |
| Check | `make rules-check`, and `thanos::consensus_recorded` in the E2E suite | promtool over the rendered chart and the unit tests; a live comparison of what is recorded against what it reads |

A recording rule has no default set and is never selected.
It installs wherever its capabilities are present, because a recorded series costs a few series and every consumer of one needs it to be there.
`rules.selected`, `rules.disabled` and `rules.overrides` name alerts only.

A recorded sample reaches Thanos with more labels than the rule wrote.
The gateway stamps `job` and `instance` as `thanos-ruler`, the destination adds `cluster`, and Thanos Receive adds `tenant_id`.
Each ruler replica also stamps its own `ruler_replica`, which Thanos Query deduplicates on, so a reader sees one series per name.
A consumer that joins on a recorded series SHOULD name its labels with `on (…)`.

## Naming

**A record name is `<level>:<metric>[:<operation>]`,** after the [Prometheus convention](https://prometheus.io/docs/practices/rules/).
`gen-rules` and the schema both reject any other shape.

| Part | Says | Example |
|---|---|---|
| `level` | The layer the series belongs to | `ext`, the normalized external-dependency layer |
| `metric` | What is measured, and its unit, as an exported metric would name them | `consensus_commit_latency_seconds`, `consensus_xid_used_ratio` |
| `operation` | A window or a statistic the value depends on | `:rate5m`, `:p99` |

The operation suffix is how a reader tells a p99 from a mean without reading the rule.
A later p50 is then a new name rather than a change to what an existing name means.
A value that does not depend on a window, such as a ratio read from a provider gauge, carries no operation.

A record name MUST NOT encode which source measured it.
That is the `flavor` label's job, and it is what lets every source record one name.

A record name is a committed surface from the release that first ships it, like an alert name.
Renaming one after that owes the deprecation cycle in [Versioning]({{< relref "../versioning.md" >}}#stability-guarantees).

## The `ext:*` contract

An **adapter** is one source of measurements for an external dependency, such as Materialize's own persist client or a cloud provider's monitoring API.
Each adapter is one rule group, and every adapter that can measure a series records it under the same name.

| Rule | Why |
|---|---|
| Every `ext:*` series MUST carry a static `flavor` label naming its adapter | It tells two adapters' series apart, and it is what a dashboard discovers adapters from. `gen-rules` rejects an `ext:*` rule without one |
| An adapter that cannot measure a series MUST record nothing for it | Absence and zero mean different things to an alert. CloudWatch publishes no allocated storage for RDS, so the `rds` adapter records no storage ratio |
| An adapter MUST NOT record a value derived from a different measurement | A mean is not a p99, and an average write latency is not a commit latency |
| A quantity with a ceiling SHOULD be recorded as a ratio from 0 to 1 | A threshold on bytes has to be re-derived for every instance size |
| A series SHOULD carry the identity of what it measures and nothing else | `namespace` for the persist adapter; `resource`, the provider's name for the database, for a provider adapter. The label set is part of the contract |

The adapters for the metadata database are in `packages/queries/ext-consensus.yaml`, and [Recorded Series]({{< relref "../../recorded-series.md" >}}) lists what each records.

A group records a given name once, and two rules MUST NOT write the same series.
`gen-rules` rejects two rules with the same name and the same static labels, since they would overwrite each other's samples whenever both have data.
A group holds recording rules or alerts, not both, because alerts are selected by group and recording rules are not.

## Install-time facts

A provider pull watches every database it is told to, including databases that are not a metadata database: the Terraform modules add Grafana's to the same pull.
The provider cannot tell them apart, so which ones are a metadata database is an install-time value, `externalDependencies.consensus` in the chart.
Each provider adapter matches its resource label against a placeholder the chart fills from it.

| Parameter | Renders to | The chart fills it from |
|---|---|---|
| `consensusRdsResources` | `__mzmon_consensus_rds__` | The `resourceId` of each `externalDependencies.consensus` entry with `flavor: rds` |
| `consensusCloudsqlResources` | `__mzmon_consensus_cloudsql__` | The same, for `flavor: cloudsql` |
| `consensusAzurePostgresResources` | `__mzmon_consensus_azure_postgres__` | The same, for `flavor: azure-postgres` |

An undeclared flavor renders as `a^`, which matches nothing, so its adapter records nothing.
The adapters match with `(?i)`, because the providers do not promise the case a name was declared in.

## Capabilities and tiers

A recording rule's capabilities are inferred from its metrics, exactly as an alert's are.
The three provider pulls are capabilities the chart derives from `pipeline.metrics.provider.*`: `cloudwatch`, `cloud-monitoring` and `azure-monitor`.
A recording rule that reads only `up` MUST declare what it is about with `requires`.

A registry file of recording rules SHOULD set `metricImportanceHint: diagnostic`.
A file's hint rolls up to every metric its queries read, and a recording rule is no reason to send a metric to more destinations.

A recorded series has a metric tier only when a registry query names it, which an alert reading it does: `ext:consensus_up` takes the tier of `materialize-alerts.yaml`.
A destination filtering by `minMetricImportance` drops the others.
The bundled `thanos` destination keeps everything by default, and the chart warns, naming them, when it does not.
Giving every recorded series a tier is [DEP-368](https://linear.app/materializeinc/issue/DEP-368).

## What `gen-rules` rejects

| Check | Why |
|---|---|
| The name is `<level>:<metric>[:<operation>]`, lowercase snake_case | Names become a committed surface |
| The group is snake_case, and holds no alerts | Alerts are selected by group, and recording rules are not |
| Static label names are valid and do not start with `__`, and no value is empty | An empty value is no label at all |
| An `ext:*` rule carries `flavor` | The contract above |
| No other rule writes the same name with the same labels | The two would overwrite each other |
| The query is PromQL, with exactly one expression | Nothing delivers recording rules to the Loki ruler |
| The expression renders and parses, and no placeholder, Grafana variable or unknown token remains | As for an alert |
| Every metric has a known source | A rule reading a metric nothing produces records nothing |
| A recording rule reads no other recorded series | One rule's applicability would depend on another's through the chart, and nothing needs that |

## Reading a recorded series

An alert MAY read a recorded series, and the normalized layer exists so that alerts read it rather than a flavor-native family.
A recorded series exists wherever any one of its recording rules installs, and capabilities can only say that all of a set are present.
So `gen-rules` records each recorded series an alert reads beside its capabilities, under `reads` in `_index.yaml`, with the recording rules that could produce what the alert selects.
The chart installs the alert only where, for every series it reads, at least one of those rules installs.

| Selector | Could be produced by |
|---|---|
| `ext:consensus_up` | Every rule recording it: `persist`, `cloudsql`, `azure-postgres`. The alert installs wherever Materialize is |
| `ext:consensus_up{flavor="cloudsql"}` | `cloudsql` only. The alert installs where the Cloud Monitoring pull runs |
| `ext:consensus_up{namespace="prod"}` | Every rule recording it, since none sets `namespace` statically |

A rule is ruled out only by its static labels, because a matcher on any other label could match whatever the rule's expression produces.
A selector that every rule's static labels contradict is an error.
The alert's other metrics still add their capabilities as usual, so an alert reading `ext:consensus_up` and `mz_*` needs `materialize` as well.
[Common Alerts]({{< relref "../../common-alerts.md" >}}) lists what each alert reads.

## Testing a recording rule

**Unit tests.**
`packages/queries/tests/<registry-file>.test.yaml` holds `promtool test rules` cases, run by `make rules-check` against the `all` scenario.
That scenario enables every provider pull and declares one database per flavor, so every adapter installs.
A test SHOULD assert the recorded value and labels with `promql_expr_test`, and SHOULD include the case where the adapter has nothing to say and records nothing.
A file asserting an interpolated value, such as a quantile, SHOULD set `fuzzy_compare: true`.
Go fuses the interpolation's multiply-add on arm64 and not on amd64, so the same promtool image rounds the last bit differently on a laptop and in CI.

**Live evaluation.**
Before a rule ships, its expression SHOULD be run against a real install, read-only, with the placeholders substituted, and its result compared with the flavor-native query it normalizes.
A provider adapter SHOULD also be run with nothing declared, to confirm it records nothing.

**End to end.**
`thanos::consensus_recorded` checks that every environment whose metadata-database calls reach Thanos also has `ext:consensus_up` recorded.
A recording rule that never produces a series passes every render test, and this is the check that sees it.
