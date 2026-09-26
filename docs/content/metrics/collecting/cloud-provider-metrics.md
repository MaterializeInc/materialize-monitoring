---
title: "Cloud Provider Metrics"
weight: 50
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Cloud Provider Metrics

The gateway can pull what a cloud provider's monitoring API publishes about the metadata database and the buckets a Materialize deployment depends on.
The result is written beside every other metric, with the same retention, the same PromQL, and the same destinations.
A provider series is therefore joinable with `mz_persist_*` in a single expression, which a Grafana CloudWatch or Cloud Monitoring datasource cannot offer.

Provider collection is off by default and adds to what the clients already report about the same dependencies.
Persist, Loki and Thanos measure every request they make against the database and the bucket, at full resolution and at no cost.
The provider adds what no client can see: CPU, memory and storage headroom, the burst credits that throttle a volume, transaction-ID consumption, and bucket growth.
The [external-dependency design](../../../reference/internal/design-docs/20260920-external-dependency-monitoring/#pulling-provider-metrics-into-the-pipeline) records the reasoning.

<!--
Agent note: the pull is rendered by charts/materialize-monitoring/templates/_alloy_provider_helpers.tpl,
not pre-rendered, because CloudWatch needs one `static` block per resource. The numbers on this page
(series counts, API calls, sample ages) were measured on 2026-09-26 against the heather-mzmon test
resources with Alloy v1.20.0 and are the ones to re-measure if the exporters are bumped.
-->

## What is pulled

| Provider | Service | Default metrics | Resolution |
|---|---|---|---|
| `cloudwatch` | RDS instance | `CPUUtilization`, `CPUCreditBalance`, `FreeableMemory`, `FreeStorageSpace`, `DatabaseConnections`, `ReadLatency`, `WriteLatency`, `DiskQueueDepth`, `BurstBalance`, `EBSIOBalance%`, `EBSByteBalance%`, `MaximumUsedTransactionIDs` | One minute, summarised per five-minute period |
| `cloudwatch` | S3 bucket | `BucketSizeBytes` per storage class, `NumberOfObjects` | Daily |
| `gcp` | Cloud SQL instance | `cpu/utilization`, `memory/utilization`, `disk/utilization`, `postgresql/num_backends`, `postgresql/transaction_id_utilization`, `up` | One minute |
| `gcp` | GCS bucket | `storage/v2/total_bytes`, `storage/v2/total_count`, split into live, noncurrent and soft-deleted objects | Daily, repeated every five minutes |

Both metric lists are values and can be changed.
A GCP entry is a metric-type **prefix**, so `postgresql/num_backends` also pulls `num_backends_by_state` and `num_backends_by_application`.

Azure Monitor is not supported yet.

### Metric names

| Provider | Name shape | Example | Resource label |
|---|---|---|---|
| `cloudwatch` | `aws_<service>_<metric>_<statistic>` | `aws_rds_free_storage_space_minimum` | `dimension_DBInstanceIdentifier`, `dimension_BucketName` |
| `gcp` | `stackdriver_<resource type>_<metric type>` | `stackdriver_cloudsql_database_cloudsql_googleapis_com_database_cpu_utilization` | `database_id` (`project:instance`), `bucket_name` |

CloudWatch series also carry `region`, `account_id`, `account_alias`, and `name`, which is the job's identifier-safe form of the resource name.
Join on the `dimension_*` label rather than on `name`.

Every provider series carries `job="integrations/cloudwatch"` or `job="integrations/gcp"`, and `instance` set to the provider name.

## Configuring it

Each provider lists the resources to watch.
Nothing else is pulled, and a provider that is enabled with no resources fails at render.

```yaml
pipeline:
  metrics:
    provider:
      cloudwatch:
        enabled: true
        region: us-east-1
        rds:
          instances: [mz-prod-db]
        s3:
          buckets: [mz-prod-storage-a1b2]
      gcp:
        enabled: true
        projectId: my-project
        cloudSql:
          instances: [mz-prod-pg]
        gcs:
          buckets: [mz-prod-storage]
```

A Cloud SQL entry is the instance name, not the `project:region:instance` connection name.

The chart refuses, at render, the values that would otherwise stop the gateway starting: a scrape timeout longer than the interval, and an RDS metric whose `length` is shorter than its `period`.
Alloy accepts both in `alloy validate` and exits on them at load, which would stop logs and metrics along with the pull.

The full set of keys is in the [values reference](../../../reference/helm/materialize-monitoring-values/), under `pipeline.metrics.provider`.

## Identity and grants

Credentials never travel through values.
The pull runs as the gateway pod's own cloud identity, bound through `alloy-gateway.serviceAccount.annotations`.

| Provider | Identity | Grant |
|---|---|---|
| `cloudwatch` | IRSA (`eks.amazonaws.com/role-arn`), EKS Pod Identity, or static keys as `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` in the `mzmon-alloy-gateway-env` Secret | `cloudwatch:GetMetricStatistics`. `iam:ListAccountAliases` fills `account_alias`; without it every pull logs a warning. `sts:AssumeRole` on `roleArn`, when set |
| `gcp` | Workload Identity (`iam.gke.io/gcp-service-account`) | `roles/monitoring.viewer` on the project |

`cloudwatch.roleArn` makes every job assume that role first, which is the shape for a read-only role in another account.

On GCP, the service account the Terraform module creates for the Google Cloud Metrics exporter holds `roles/monitoring.metricWriter`, which writes metrics and cannot read them.
Reading needs `roles/monitoring.viewer` added to the same account.

The chart warns at render when a provider is enabled and the gateway's service account carries no matching annotation.
It cannot tell whether EKS Pod Identity or a static key is in use instead, so the warning is advisory.

## Lag

Provider data is minutes old when it arrives, and that decides how it can be used.

| Source | Age on arrival | Timestamp |
|---|---|---|
| CloudWatch RDS | A few minutes | The scrape's |
| CloudWatch S3 | Up to a day | The scrape's |
| Cloud SQL | About three minutes | Cloud Monitoring's |
| GCS | Over ten minutes | Cloud Monitoring's |

CloudWatch samples are stamped at scrape time.
Stamping a daily S3 datapoint with its own time would put it a day in the past, and the bundled Thanos Receive, which accepts no out-of-order samples, refuses samples that old.

Cloud Monitoring samples keep their own timestamps.
An instant query at "now" looks back five minutes by default and finds no GCS sample at all.
Queries on provider families SHOULD use `last_over_time(<series>[15m])` or a wider window.

No provider signal backs a fast page.
Provider alerts cover the slow-moving conditions — storage headroom, burst-credit exhaustion, connection ceilings — with `for:` windows well above the publication delay.

## Cost

The provider bills each pull, and the bill depends on configuration, not on how many dashboards are open.

| Provider | Calls per pull | Measured |
|---|---|---|
| `cloudwatch` | One `GetMetricStatistics` per RDS metric per instance, one per bucket per storage type, and one per bucket for the object count | 28 calls for two instances and two buckets with the default lists, counted by `yace_cloudwatch_getmetricstatistics_requests_total` |
| `gcp` | One descriptor listing per metric prefix, and one time-series listing per matching metric type | 19 calls for two instances and two buckets with the default lists, counted by `stackdriver_monitoring_api_calls_total` |

`scrapeInterval` is the main lever, and defaults to five minutes.
Rates change, so current provider pricing is the reference for what a call costs.

The gateway runs several replicas, and every replica runs the exporter.
The scrape of it is clustered, so one replica owns it and each provider is called once per interval.

## Which destinations receive it

Provider families have no dashboard or alert reading them yet, so they have no tier from the query registry.
Each provider assigns its families one instead, through `metricImportance`, which defaults to `extended`.

| Destination `minMetricImportance` | Receives provider families at the default |
|---|---|
| `all` (the bundled Thanos) | Yes |
| `diagnostic`, `extended` | Yes |
| `recommended`, `essential` | No |

A destination that bills per series, such as Datadog or a BYOC fan-out, therefore does not receive them unless its floor or the provider's tier is changed.

## What is deliberately not pulled

| Signal | Why |
|---|---|
| GCS `api/request_count` | A per-minute DELTA, and the exporter adds only the newest point of each pull to its counter. At a five-minute interval it reports about a fifth of the real count. The clients report the same requests exactly |
| S3 request metrics | Opt-in per bucket on the AWS side, and billed as custom metrics |
| Resources found by tag | Tag discovery pulls every matching resource in the account, and bills for each |

## Checking it works

**`up` does not say whether a pull succeeded.**
An exporter whose provider call fails still answers its scrape, with no provider series in it.
Measured with no credentials at all, both exporters return HTTP 200 and report healthy, so `up` stays 1.
`up` only says the exporter exists and is being scraped.

| Question | Query |
|---|---|
| Is the exporter running and scraped | `up{job=~"integrations/(cloudwatch\|gcp)"}` |
| Did the last GCP pull fail | `stackdriver_monitoring_last_scrape_error == 1` |
| Is CloudWatch returning data, per instance | `count by (dimension_DBInstanceIdentifier) (aws_rds_cpuutilization_average)` |
| Is Cloud Monitoring returning data, per instance | `count by (database_id) (last_over_time(stackdriver_cloudsql_database_cloudsql_googleapis_com_database_up[15m]))` |

CloudWatch publishes no equivalent of the GCP error series.
Its `yace_cloudwatch_getmetricstatistics_requests_total` counts the billed calls, which makes it the measure of cost, not of health.
A CloudWatch pull that fails for want of an identity or a grant is visible only as missing series and as errors in the gateway's logs, such as `Couldn't get account Id`.
