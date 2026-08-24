---
title: "List of Metrics"
weight: 10
---

# List of Metrics

This has the list of metrics which are available for usage.
In many systems, only a subset of metrics get stored based
on how they would be used.

## Metric Tiers

Importance is about **storage**, not stability: it says which metrics are worth keeping when capacity is limited, not what we promise about their names.
For that, see [Stability and Deprecations](../../stability/) — in short, `mz_*` names come from Materialize itself and we disclose rather than freeze them.


Metrics are grouped by "metricImportance" levels (mzmon-specific).
These levels guide which metrics are prioritized in
metric stores which have limited capacity.

The **essential** metrics are the set of metrics that
are critical and you would always want to have available.
These are used in alerting.

The **recommended** metrics are the set of metrics that
are used in dashboards and are generally desirable for
troubleshooting.

The **extended** set of metrics are used for optional/experimental
dashboards.

The **diagnostic** set of metrics are used for in-depth
troubleshooting and analysis.

In our `materialize-monitoring` configuration, we also
provide an **all** min-importance for including
absolutely everything.
This is recommended if you have cheaper metric storage
like our bundled Thanos provider.

## Essential Metrics

> [!WARNING]
> FIXME: Some links for alerts mistakenly point at the common-queries page.

{{% list-metrics importance="essential" %}}

## Recommended Metrics

{{% list-metrics importance="recommended" %}}
