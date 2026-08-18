---
title: "Datadog"
weight: 20
---

# Datadog

There is no Materialize dashboard set for Datadog yet.
It is tracked as [DEP-115](https://linear.app/materializeinc/issue/DEP-115) and scheduled for OO-M3.

What exists today is the query material to build one yourself.
[Common Queries]({{< relref "../reference/stable-metrics/common-queries.md" >}}) renders every query in the registry with a **Datadog** tab beside its PromQL, in the metric query syntax you paste into a dashboard widget or a monitor.

Those translations have not been run against a live Datadog account, so expect to correct some of them.
The conventions behind them — how Prometheus metric names and labels land in Datadog, and where Datadog's query language cannot express what the PromQL does — are written up in [Datadog Translations]({{< relref "../reference/internal/queries/datadog.md" >}}) (internal).

To get the metrics there in the first place, see [Storing Metrics]({{< relref "../metrics/storing.md" >}}) and the `otel-metrics-fanout` profile.
