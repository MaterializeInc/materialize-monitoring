---
name: alerting
description: |
  This skill should be used when adding, changing, reviewing or vetting an
  alerting rule or a recording rule: an `alerts:` or `rules:` entry under
  `packages/queries/`, its severity, capabilities (`requires`) or default-set
  membership (`enabledByDefault`), the normalized `ext:*` series, the
  `gen-rules` output under `charts/materialize-monitoring/pre-rendered/rules/`,
  the chart's `rules.*` or `externalDependencies` values, or the promtool unit
  tests in `packages/queries/tests/`. Also use it when porting a rule from
  Materialize Cloud's alerting.
---

# Alerting

The substance is in the docs; this file routes there and keeps the habits that
make an alert trustworthy. An alert that never fires reads exactly like one with
nothing to report, so almost every mistake here is silent.

## Read first

- [Authoring Alerts](../../../docs/content/reference/internal/queries/alerts.md)
  — how an entry becomes an installed rule, the alerting context and its
  placeholders, capabilities, the default set, what `gen-rules` rejects, and
  the contract every shipped alert keeps. Read the contract before writing PromQL.
- [Authoring Recording Rules](../../../docs/content/reference/internal/queries/recording-rules.md)
  — the `<level>:<metric>[:<operation>]` naming convention, the `ext:*`
  contract (a `flavor` per adapter, absence over a guessed value), and the
  install-time facts the provider adapters need.
- [Configuring Alerting](../../../docs/content/alerting/configuring.md) — the
  operator's side: which rules install and the `rules.*` values.
- [The alerting design doc](../../../docs/content/reference/internal/design-docs/20260917-alerting-self-managed.md)
  — why the stack is two rulers and one Alertmanager, and what is still owed.
- [Queries as Code](../../../docs/content/reference/internal/queries/overview.md)
  — the registry itself. Panel work is `dashboards-as-code`, not this skill.

## The loop

`make rules` renders and checks, `make rules-check` runs promtool over the
rendered chart and the unit tests (`PROMTOOL=promtool` skips docker), and
`cargo test -p mzmon-lib rules::` covers the renderer. `gen-rules` lists every
problem at once; fix the list rather than the first line.

## Habits

**Evaluate before proposing.** Run the expression against a live install,
read-only, with the `__mzmon_*__` placeholders substituted:
`gcx --context <ctx> metrics query -d mzmon-thanos '<expr>'`. Check it is quiet
now, that each metric it names returns series, and that it fires over a window
in which the condition is known to have happened. Quiet on its own proves
nothing. A LogQL rule is the same with
`gcx --context <ctx> logs metrics -d mzmon-loki '<expr>' --since 5m`, its range
widened to cover a line known to have been logged.

**Prove the fix with a unit test.** Every behaviour worth fixing is a
three-series `promtool test rules` case: one input that fires the alert, and
the one it used to misfire on. Assert on `ALERTS`, not on the annotations. Loki
has no rule test, so for a LogQL rule the live evaluation above is the proof.

**Match a log rule on structure before text.** Stream labels in the selector,
then the pipeline's structured metadata (a panic is already `level="CRITICAL"`
with `panic_location`), and a line filter last. A matched message is not a
contract, so name the source file that logs it.

**Re-derive from Cloud; never copy.** Cloud's rules and the Grafana-managed
set are prior art for thresholds and history. They use Cloud's namespaces,
labels and job names, and some carry customer names. Nothing pulled from them
is committed as it stands, and no customer name, organization or environment
identifier enters this repository.

**Confirm a label before building on it.** `count by (<label>) (<metric>)`
settles what a series carries. The Cloud spellings (`kubernetes_io_hostname`,
`app_kubernetes_io_component`, `environment-*` namespaces) return empty results
on self-managed, and empty reads as healthy.

**Put an alert where the person who acts on it will see it.**
The deployment and the system clusters (`s*`) are `materialize-alerts.yaml`,
with `audience: platform`; user clusters (`u*`) and their sources are
`materialize-workload-alerts.yaml`, with `audience: workload`. A signal that
belongs to both is two alerts, scoped by cluster id. Authoring Alerts has the
scoping labels.

**Calibrate against more than two installs.** A threshold quiet on the test
installs can page on a fleet. Aggregate queries over a larger fleet, with no
customer identifiers in what comes back, are how the freshness defaults were
found to page on clusters that are behind by design.

**Let the capability table decide applicability.** A metric `gen-rules` cannot
place is a question about what produces it, answered in
`packages/mzmon-lib/src/query/rules/capability.rs`, not by declaring
`requires` to make the error go away.

**Treat a default-set entry as a committed name.** Adding `enabledByDefault`
ships the name to every deployment it applies to. Name for the condition, not
the grade, before it goes in.
