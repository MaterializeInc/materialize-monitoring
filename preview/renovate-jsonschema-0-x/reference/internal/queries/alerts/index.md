# Authoring Alerts




# Authoring Alerts

An alert is written once, in the query registry, and becomes a Prometheus rule through `mz-monitoring-build gen-rules`.
The chart installs that rule as a `PrometheusRule` wherever it applies, and the Thanos ruler evaluates it.
This page is the conventions a contributor follows when adding or changing an alert, and what the tooling checks on their behalf.
Where the alert goes once it fires is [Alert Channels](/materialize-monitoring/preview/renovate-jsonschema-0-x/alerting/channels/); why the stack is shaped this way is the [alerting design doc](/materialize-monitoring/preview/renovate-jsonschema-0-x/reference/internal/design-docs/20260917-alerting-self-managed/).


<blockquote class="book-hint note">
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in
<a href="https://datatracker.ietf.org/doc/html/rfc2119" rel="external" class="external-link">RFC 2119</a>.
</blockquote>


<!--
Agent note: the `alerting` skill routes here. Keep the contract section in step with what gen-rules and the chart
validators actually enforce; a rule listed as checked here that nothing checks is worse than no rule.
-->

## From registry entry to installed rule

| Stage | Where | What happens |
|---|---|---|
| Author | `packages/queries/*.yaml`, an `alerts:` entry | The alert names its query inline or by `queryId`, its `severity` and `component` labels, and its prose; the file's `alertLabels` supply its `audience` |
| Validate | `bin/mz-monitoring-check check-queries` (pre-commit) | Schema validation, including every `%%{…}` placeholder name |
| Render | `make rules` (`mz-monitoring-build gen-rules`) | Renders through the alerting context, infers capabilities, and fails on any problem |
| Output | `charts/materialize-monitoring/pre-rendered/rules/prometheus/` | One `groups:` file per registry file, and `_index.yaml` |
| Install | `templates/alerts/prometheusrules.yaml` | Fills the placeholders from values and keeps the rules that apply |
| Check | `make rules-check` | `promtool check rules` on several rendered scenarios, then `promtool test rules` |

The generated files are committed, and CI fails when they are stale.

## Where an alert goes

An alert belongs in the file for the people who act on it.

| File | `audience` | Holds |
|---|---|---|
| `materialize-alerts.yaml` | `platform` | The Materialize deployment: environmentd, the system clusters, clusterd crashes, persist, auth and the console |
| `materialize-workload-alerts.yaml` | `workload` | What runs on the deployment: user clusters' freshness, hydration and sizing, and the sources feeding them |
| `infra-alerts.yaml` | `platform` | The Kubernetes platform under Materialize, and the monitoring stack |

Each file sets `audience` for all of its alerts with `alertLabels`, and an alert MAY set its own.
`gen-rules` rejects an alert whose `audience` is not `platform` or `workload`.
A route matches on the label to send the two to different people.

Who acts on a signal can depend on the cluster it comes from.
A user cluster falling behind is the workload owner's to fix, and a system cluster falling behind is the platform's.
Such a signal MUST be two alerts, one in each file, each scoped by cluster id.

| Series | Scoped by |
|---|---|
| environmentd's per-collection series, such as `mz_dataflow_wallclock_lag_seconds` | `instance_id=~"u.*"` or `"s.*"` |
| clusterd's own series, such as `mz_metrics_resource_usage` | `cluster_environmentd_materialize_cloud_cluster_id=~"u.*"` or `"s.*"` |
| kube-state-metrics and cAdvisor series | the replica pod name, `pod=~".*-cluster-u[0-9]+-replica-.*"`, lifted into the cluster-id label with `label_replace` where the alert names the cluster |

## The alerting context

A dashboard renders a query against variables its viewer chooses.
A ruler has no viewer, so alerts render through a third context, `packages/mzmon-lib/src/query/rules/context.rs`, which differs from the dashboard context in two ways.

**Deployment-specific values render as placeholders.**
Rules are rendered once, at build time, while which namespaces hold Materialize, which workloads the cluster counts as core infrastructure, and which metric prefix Materialize uses are facts about one install.
Those parameters render to a `__mzmon_*__` token that the chart replaces at install time.

| Parameter | Renders to | The chart fills it from |
|---|---|---|
| `mzEnvironmentNamespaceFilter` | `namespace=~"__mzmon_environment_namespaces__"` | `rules.namespaces.environment`, else `materialize.namespaces`, else `materialize-system.namespace` |
| `mzOperatorNamespaceFilter` | `namespace=~"__mzmon_operator_namespaces__"` | `rules.namespaces.operator`, else `materialize-operator.namespace` |
| `excludeMzDeploymentNamespaceFilter` | `namespace!~"<operator>\|<environments>"` | both of the above |
| `excludeEnvironmentFilter` | `namespace!~"__mzmon_excluded_namespaces__"` | `rules.namespaces.exclude` |
| `mzSqlPrefix` | `__mzmon_sql_prefix__` | `materialize.deploymentMode`: `mz_`, or `v2_mz_` for `cloud` |
| `infraCoreWorkloadList` | `__mzmon_core_workloads__`, a bare value: `container=~"%%{infraCoreWorkloadList}"` | `rules.infraWorkloads.core` |
| `infraImportantWorkloadList` | `__mzmon_important_workloads__` | `rules.infraWorkloads.important` |
| `infraNonessentialWorkloadList` | `__mzmon_nonessential_workloads__` | `rules.infraWorkloads.nonessential` |
| `infraDaemonsetWorkloadList` | `__mzmon_daemonset_workloads__` | `rules.infraWorkloads.daemonset` |
| `mzEnvironmentFilter` | `materialize_cloud_organization_name=~".+"` | nothing; a rule covers every environment |

An empty namespace list or workload tier renders as `a^`, a regex that matches nothing, rather than as an empty string.

An infrastructure alert MUST read a workload tier rather than list container or Deployment names.
Which workloads a cluster cannot run without is a fact about that cluster, and a list written into the expression is one no operator can correct.
Each tier holds container and Deployment names alike, so the same parameter serves `container=~` and `deployment=~`.

**Selection parameters are absent.**
`interval`, `range`, `rangeWindow`, the log pickers and the generation filters mean "whatever the viewer chose".
A query that uses one fails to render in this context.
An alert that needs a window MUST write it out, since the window is a decision about how long a condition persists, and it belongs to the alert.

The `mzEnvironmentName` function attaches `materialize_cloud_organization_name` to a series by joining on the given label, normally `namespace`, against the Materialize scrape targets.
A namespace holding two environments stays unlabelled instead of failing the evaluation, and a series with no match passes through unchanged.
The `mzClusterName` function attaches `cluster_name` from `mz_cluster_info`, joining on the namespace and the cluster-id label it is given.
A cluster id is unique only within one environment, so the dashboard join, which keys on the id alone, is not used here.
`mzObjectName` is not available to rules, because nothing yet scopes its catalog join to one environment.

## Capabilities

A rule installs only where every capability it requires is present.
Capabilities name what a deployment contains, such as a CNI, a metadata-database flavor or an exporter, and never who operates it.

Most requirements are **inferred**.
`gen-rules` reads the metrics an alert names and maps each through the ordered table in `packages/mzmon-lib/src/query/rules/capability.rs`, so an alert reading `cilium_*` requires `cilium` without saying so.
A metric the table does not claim fails the build.

**`requires` declares what metric names cannot show.**
An alert whose only metric is `up` MUST declare what it is about, since `up` exists for every target.
An alert that depends on a label only some deployments add, such as a node label, SHOULD declare the capability that adds it.
The effective set, inferred and declared, is recorded per rule in `_index.yaml`, which is how a reviewer sees it.

| Kind | Capabilities | Present when |
|---|---|---|
| Derived | `materialize`, `materialize-sql`, `materialize-operator`, `kube-state-metrics`, `cadvisor`, `node-exporter`, `loki`, `alloy` | This chart runs the component and collects its metrics |
| Explicit | `synthetic-uptime`, `external-uptime`, `feature-flags`, `frontegg-auth`, `memory-limiter`, `crdb-dedicated`, `cilium`, `coredns`, `cert-manager`, `kubelet-metrics`, `swap-nodes`, `egress-gateway` | An operator lists it in `rules.capabilities` |

Adding a capability means adding it to the `Capability` enum, to the `capability` enum in `mzmon-query.schema.yaml`, and, for a derived one, to `mzmon.rules.derivedCapabilities` in the chart.
Tests fail when the three disagree.
A capability SHOULD NOT be added for a single rule; a tag that applies to one rule is a label pretending to be a category.

## The default set

`enabledByDefault: true` puts an alert in the default set, meaning it installs wherever it applies without an operator selecting it.
Everything else installs only when named in `rules.selected`.

An alert MUST NOT enter the default set until its expression has been evaluated against a live self-managed install and does not fire falsely there.
It SHOULD also have a unit test (see below), and it SHOULD have evidence that the condition matters, such as incident history or a Cloud counterpart that pages.
Entering the default set commits the alert's name; renaming it afterwards owes a changelog entry and, after 1.0, a deprecation cycle.

An alert whose normal duration depends on the workload SHOULD carry that duration in `for`, and its notes SHOULD say so.
`rules.overrides` changes a rule's `for` and labels per deployment, and never its expression.
Hydration is the standing example: most clusters hydrate in minutes, and a large one can take hours with nothing wrong.

An alert that fires on workloads behind by design MUST stay out of the default set, however useful it is where it applies.
A materialized view on a refresh schedule lags by up to its interval between refreshes, so an absolute freshness threshold on every user cluster pages on those clusters permanently.

## What `gen-rules` rejects

`gen-rules` reports every problem it finds and writes nothing until there are none.

| Check | Why |
|---|---|
| The name is kebab-case, and the group snake_case | Names become a committed surface |
| `severity` is `critical`, `warning` or `notice`, and `component` is set | The routing presets route by severity; an unknown one has no class |
| `audience` is `platform` or `workload` | Routes match on it; a missing one sends the alert to neither audience's receiver |
| `for` and `keepFiringFor` are Prometheus durations | promtool would reject the file, and the ruler with it |
| The query exists and has exactly one PromQL expression | A rule is one expression; LogQL rules are not rendered yet |
| The expression renders and parses | A group with one bad rule is dropped whole |
| No `%%{…}`, Grafana variable or unknown `__mzmon_*__` token remains | A ruler resolves none of them, and the selector matches nothing |
| Every metric has a known source | A rule reading a metric nothing produces never fires |
| `deploymentMode` is not a label | Applicability is `requires`, not a label nothing reads |

## The contract for a shipped alert

These are the rules the audit behind this tooling found broken most often.
The first two are checked mechanically; the rest are for the author and the reviewer.

1. An alert MUST be able to fire on a stock self-managed install, or its capabilities MUST say what it needs.
2. An alert MUST NOT name a namespace, release, job or container that belongs to one install. It uses the context's parameters, or a suffix match such as `job=~".*/.*materialize-clusterd"`.
3. A per-environment alert SHOULD keep `namespace` in its `by (…)` clause. The routing tree groups by `[alertname, cluster, namespace]`, and an alert that aggregates it away sends every environment's condition as one notification.
4. A ratio MUST guard its denominator. A container with no memory limit reports a limit of `0`, and dividing by it gives `+Inf`, which fires forever.
5. An alert on a terminated container MUST pair the exit code with a recent restart. `kube_pod_container_status_last_terminated_exitcode` persists for the life of the pod.
6. A join between series of different granularity MUST name its labels with `on (…)`. A pod-level and a container-level series never share a full label set.
7. The name SHOULD describe the condition rather than its grade, so that re-grading does not force a rename.
8. The `summary` MUST be actionable without following the runbook link, since an air-gapped install cannot follow it.

## Testing an alert

**Unit tests.**
`packages/queries/tests/<registry-file>.test.yaml` holds `promtool test rules` cases, run by `make rules-check` against the rules the chart renders with every rule selected.
A test SHOULD give one input that fires the alert and one that it previously misfired on, and assert on the `ALERTS` series rather than restating the annotations.

**Live evaluation.**
Before an alert enters the default set, its expression SHOULD be run against a real install, read-only, with the placeholders substituted: once as it stands, to confirm it is quiet, and once over a window in which the condition is known to have happened, to confirm it fires.
`gcx metrics query` does this without cluster access.

**Cloud's rules are prior art, not a source.**
Materialize Cloud's rules (`infra/prometheus/alerting.py` and the Grafana-managed set) are worth reading for thresholds and history.
An alert MUST NOT be copied from them verbatim: they use Cloud's namespaces, labels and job names, and some carry customer names that MUST NOT enter this repository.

