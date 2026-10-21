# Datadog




# Datadog

The Materialize Environment Overview, `env-top`, has a Datadog preview.
It holds the cluster and replica counts from the Grafana dashboard's Cluster Objects / Replicas tab.
The rest of the Grafana dashboards are tracked as [DEP-115](https://linear.app/materializeinc/issue/DEP-115), on the [Dashboards roadmap](/materialize-monitoring/preview/claude-datadog-dashboard-machinery/reference/development/roadmap/#dashboards).

## Import the preview

1. Download the Datadog file from the `env-top` row of [Available Dashboards](/materialize-monitoring/preview/claude-datadog-dashboard-machinery/dashboards/all/#materialize-environment-overview-env-top).
2. Create the dashboard through Datadog's API, with an API key and an application key that has the `dashboards_write` permission:

   ```bash
   curl -X POST "https://api.DATADOG_SITE/api/v1/dashboard" \
     -H "DD-API-KEY: ${DD_API_KEY}" \
     -H "DD-APPLICATION-KEY: ${DD_APP_KEY}" \
     -H "Content-Type: application/json" \
     --data @env-top.json
   ```

   Replace `DATADOG_SITE` with your Datadog site, such as `datadoghq.com`.
   If you manage Datadog with Terraform, pass the same file to the Datadog provider's `datadog_dashboard_json` resource instead.
3. Pick one environment in the dashboard's **environment** variable.
   Environments reuse cluster and replica ids, so the counts merge them when several are selected.

## Build the rest yourself

[Common Queries](/materialize-monitoring/preview/claude-datadog-dashboard-machinery/reference/common-queries/) renders every query in the registry with a **Datadog** tab beside its PromQL, in the metric query syntax you paste into a dashboard widget or a monitor.

> [!WARNING]
>   **Most of these translations have not been run against a live Datadog account**, so expect to correct some of them.
>   Where Datadog's query language cannot express what the PromQL does, the translation is approximate rather than equivalent.

The conventions behind them — how Prometheus metric names and labels land in Datadog, and where the two query languages diverge — are written up in [Datadog Translations](/materialize-monitoring/preview/claude-datadog-dashboard-machinery/reference/development/queries/datadog/).

## Getting the metrics there

Dashboards are the second half of the problem; the metrics have to reach Datadog first.

The Alloy gateway forwards over OTLP natively, with a per-destination importance tier so you choose how much crosses.
See [Storing Metrics](/materialize-monitoring/preview/claude-datadog-dashboard-machinery/metrics/storing/#datadog) for the exporter, and the `otel-metrics-fanout` profile for the assembled shape.

> [!TIP]
>   Forwarding to Datadog does not mean giving up the bundled stack.
>   The gateway fans out to several destinations at once, so Thanos can keep the full-fidelity copy while Datadog receives the tier you select.

