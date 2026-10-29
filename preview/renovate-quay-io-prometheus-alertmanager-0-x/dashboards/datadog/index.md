# Datadog




# Datadog

The Materialize Environment Overview, `env-top`, has a Datadog preview.
It holds the cluster and replica counts from the Grafana dashboard's Cluster Objects / Replicas tab.
The rest of the Grafana dashboards are tracked as [DEP-115](https://linear.app/materializeinc/issue/DEP-115), on the [Dashboards roadmap](/materialize-monitoring/preview/renovate-quay-io-prometheus-alertmanager-0-x/reference/development/roadmap/#dashboards).

## Import the preview

1. Download the Datadog file from the `env-top` row of [Available Dashboards](/materialize-monitoring/preview/renovate-quay-io-prometheus-alertmanager-0-x/dashboards/all/#materialize-environment-overview-env-top).
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

[Common Queries](/materialize-monitoring/preview/renovate-quay-io-prometheus-alertmanager-0-x/reference/common-queries/) renders the registry's queries with a **Datadog** tab beside the PromQL wherever a query has a Datadog form, in the metric query syntax you paste into a dashboard widget or a monitor.
The translations are validated against a live Datadog organization that receives a self-managed install's metrics through the gateway.
Where Datadog's query language can't express what the PromQL does, the translation is the closest form, and the registry says what it drops.

Before you paste one, check the following:

- **Replace the placeholders with Datadog filters.** A placeholder such as `mzEnvironmentFilter` takes a Datadog filter term, such as `materialize_cloud_organization_name:your-env`, and terms join with `AND`.
- **Write tag values in lowercase.** Datadog stores tag values lowercased and matches filters case-sensitively, so `phase:running` matches and `phase:Running` doesn't.
- **Enable percentiles for `p50:`, `p95:`, and `p99:` queries.** A percentile query fails until percentiles are enabled for its metric under **Metrics** > **Summary** in Datadog.
- **Read `job` as `service` and `instance` as `service.instance.id`.** The gateway's exporter renames the two Prometheus target labels on the way to Datadog.

For the conventions behind the translations, including how metric names, labels, and types land in Datadog, see [Datadog translations](/materialize-monitoring/preview/renovate-quay-io-prometheus-alertmanager-0-x/reference/development/queries/datadog/).

## Getting the metrics there

Dashboards are the second half of the problem; the metrics have to reach Datadog first.

The Alloy gateway forwards over OTLP natively, with a per-destination importance tier so you choose how much crosses.
See [Storing Metrics](/materialize-monitoring/preview/renovate-quay-io-prometheus-alertmanager-0-x/metrics/storing/#datadog) for the exporter, and the `otel-metrics-fanout` profile for the assembled shape.

> [!TIP]
>   Forwarding to Datadog does not mean giving up the bundled stack.
>   The gateway fans out to several destinations at once, so Thanos can keep the full-fidelity copy while Datadog receives the tier you select.

