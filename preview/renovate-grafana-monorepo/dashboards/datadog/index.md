# Datadog




# Datadog

The Materialize Environment Overview, `env-top`, has a Datadog version.
It holds the Grafana dashboard's panels, apart from the differences listed on this page, under the same tabs, with a group for each Grafana row.
The rest of the Grafana dashboards are tracked as [DEP-115](https://linear.app/materializeinc/issue/DEP-115), on the [Dashboards roadmap](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/development/roadmap/#dashboards).
On this page, "you" are an operator who sends Materialize metrics to Datadog.

## Import the dashboard

1. Download the Datadog file from the `env-top` row of [Available Dashboards](/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/all/#materialize-environment-overview-env-top).
2. Create the dashboard through Datadog's API, with an API key and an application key that has the `dashboards_write` permission:

   ```bash
   curl -X POST "https://api.DATADOG_SITE/api/v1/dashboard" \
     -H "DD-API-KEY: ${DD_API_KEY}" \
     -H "DD-APPLICATION-KEY: ${DD_APP_KEY}" \
     -H "Content-Type: application/json" \
     --data @datadog-env-top.json
   ```

   Replace `DATADOG_SITE` with your Datadog site, such as `datadoghq.com`.
   If you manage Datadog with Terraform, pass the same file to the Datadog provider's `datadog_dashboard_json` resource instead.
3. Pick one environment in the **environment** variable, and the namespace it runs in in the **namespace** variable.

## Pick what the dashboard shows

The dashboard's template variables default to `*`, which selects every value.
Pick one value or `*` in each of the following variables:

**environment**
: Scopes the Materialize panels to one environment, by `materialize_cloud_organization_name`.
  Environments reuse cluster and replica ids, so the panels merge them when several environments are selected.

**namespace**
: Scopes the Kubernetes panels to one namespace.
  cAdvisor and kube-state-metrics series carry no environment tag, so these panels read the namespace instead.
  At `*`, they show every pod in the Kubernetes cluster.

**cluster**
: Narrows the panels to one cluster, by id, such as `u1`.

**replica**
: Narrows the panels to one replica, by id.

**Least Fresh Objects** and **Slowest Hydrating Objects** always show the last five minutes, as their Grafana panels do when the time range ends now.
They don't follow the time picker into the past.

## Enable percentiles

The **Peek Latency** and **Iceberg Commit Latency** panels read percentiles of a histogram.
Datadog doesn't compute them until percentiles are enabled for the metric, and until then the panels show an error.
Enable percentiles for `mz_compute_peek_duration_seconds` and `mz_sink_iceberg_commit_duration_seconds` under **Metrics** > **Summary** in Datadog.
Enabled percentiles count toward your Datadog custom metrics.

## How the Datadog version differs

The Datadog panels show cluster, replica, and object ids where the Grafana panels show names.
The following table lists the Grafana panels that the Datadog version draws differently or leaves out:

<!-- Agent note: keep this table in step with `DIFFERENCES` in packages/dashboards/src/datadog/env_top.rs,
which the parity ledger in packages/dashboards/tests/datadog_parity.rs enforces. -->

| Grafana panel | In Datadog | Why |
|---|---|---|
| **Environment Alerts Firing**, **Alerts**, and **Alerts Firing Over Time** | Left out | Datadog alerts are monitors, not a series to draw |
| **Last Restart Time** | Left out | Datadog has no `time()`, so the age of the newest container can't be derived from its start timestamp |
| **Environment Status** | A percentage, colored by health | Grafana maps the percentage to **Healthy**, **Degraded**, or **Unhealthy** |
| **Deployment Readiness** | A table, with **Ready** and **Unavailable** columns | The two counts are separate queries, and a table shows them side by side, one row per namespace |

## Build the rest yourself

[Common Queries](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/common-queries/) renders the registry's queries with a **Datadog** tab beside the PromQL wherever a query has a Datadog form, in the metric query syntax you paste into a dashboard widget or a monitor.
The translations are validated against a live Datadog organization that receives a self-managed install's metrics through the gateway.
Where Datadog's query language can't express what the PromQL does, the translation is the closest form, and the registry says what it drops.

Before you paste one, check the following:

- **Replace the placeholders with Datadog filters.** A placeholder such as `mzEnvironmentFilter` takes a Datadog filter term, such as `materialize_cloud_organization_name:your-env`, and terms join with `AND`.
- **Write tag values in lowercase.** Datadog stores tag values lowercased and matches filters case-sensitively, so `phase:running` matches and `phase:Running` doesn't.
- **Enable percentiles for `p50:`, `p95:`, and `p99:` queries.** A percentile query fails until percentiles are enabled for its metric under **Metrics** > **Summary** in Datadog.
- **Read `job` as `service` and `instance` as `service.instance.id`.** The gateway's exporter renames the two Prometheus target labels on the way to Datadog.

For the conventions behind the translations, including how metric names, labels, and types land in Datadog, see [Datadog translations](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/development/queries/datadog/).

## Getting the metrics there

Dashboards are the second half of the problem; the metrics have to reach Datadog first.

The Alloy gateway forwards over OTLP natively, with a per-destination importance tier so you choose how much crosses.
See [Storing Metrics](/materialize-monitoring/preview/renovate-grafana-monorepo/metrics/storing/#datadog) for the exporter, and the `otel-metrics-fanout` profile for the assembled shape.

> [!TIP]
>   Forwarding to Datadog does not mean giving up the bundled stack.
>   The gateway fans out to several destinations at once, so Thanos can keep the full-fidelity copy while Datadog receives the tier you select.

