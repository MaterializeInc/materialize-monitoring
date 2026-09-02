# Datadog




# Datadog

> [!NOTE]
>   **There is no Materialize dashboard set for Datadog yet.**
>   It is tracked as [DEP-115](https://linear.app/materializeinc/issue/DEP-115) and scheduled for **OO-M3**, alongside the Google Cloud Monitoring and Honeycomb sets.
>   See the [Dashboards workstream]({{< relref "../reference/internal/roadmap.md" >}}#dashboards) for where it sits against the rest of the work.

What exists today is the query material to build one yourself.

[Common Queries]({{< relref "../reference/stable-metrics/common-queries.md" >}}) renders every query in the registry with a **Datadog** tab beside its PromQL, in the metric query syntax you paste into a dashboard widget or a monitor.

> [!WARNING]
>   **These translations have not been run against a live Datadog account**, so expect to correct some of them.
>   Where Datadog's query language cannot express what the PromQL does, the translation is approximate rather than equivalent.

The conventions behind them — how Prometheus metric names and labels land in Datadog, and where the two query languages diverge — are written up in [Datadog Translations]({{< relref "../reference/internal/queries/datadog.md" >}}) (internal).

## Getting the metrics there

Dashboards are the second half of the problem; the metrics have to reach Datadog first.

The Alloy gateway forwards over OTLP natively, with a per-destination importance tier so you choose how much crosses.
See [Storing Metrics]({{< relref "../metrics/storing.md" >}}#datadog) for the exporter, and the `otel-metrics-fanout` profile for the assembled shape.

> [!TIP]
>   Forwarding to Datadog does not mean giving up the bundled stack.
>   The gateway fans out to several destinations at once, so Thanos can keep the full-fidelity copy while Datadog receives the tier you select.

