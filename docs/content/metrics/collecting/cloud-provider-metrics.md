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

The **Infrastructure Cloud Provider** dashboard (`infra-cloud`) draws them, with rows for whichever provider it finds; see [Available Dashboards]({{< relref "../../dashboards/all.md" >}}).

Provider collection is off by default and adds to what the clients already report about the same dependencies.
Persist, Loki and Thanos measure every request they make against the database and the bucket, at full resolution and at no cost.
The provider adds what no client can see: CPU, memory and storage headroom, the burst credits that throttle a volume, transaction-ID consumption, and bucket growth.
The [external-dependency design](../../../reference/internal/design-docs/20260920-external-dependency-monitoring/#pulling-provider-metrics-into-the-pipeline) records the reasoning.

<!--
Agent note: each pull is a custom component in packages/alloy-pipelines/gateway-provider.yaml, and
charts/materialize-monitoring/templates/_alloy_provider_helpers.tpl renders only the instances. The metric
sets on this page are that file's; change them there, not here first. packages/queries/infra-cloud.yaml and
packages/dashboards/src/grafana/infra_cloud/ draw them, so a metric added to a pull reaches no panel until
those follow, and a metric removed leaves a panel empty. The numbers on this page
(series counts, API calls, sample ages) were measured on 2026-09-26 against the wrapper-provisioned
GKE and EKS test installs, with Alloy v1.20.0 locally and v1.19.2 in-cluster, and are the ones to
re-measure if the exporters are bumped.
-->

## What is pulled

| Provider | Service | Default metrics | Resolution |
|---|---|---|---|
| `cloudwatch` | RDS instance | `CPUUtilization`, `CPUCreditBalance`, `FreeableMemory`, `FreeStorageSpace`, `DatabaseConnections`, `ReadLatency`, `WriteLatency`, `DiskQueueDepth`, `BurstBalance`, `EBSIOBalance%`, `EBSByteBalance%`, `MaximumUsedTransactionIDs` | One minute, summarised per five-minute period |
| `cloudwatch` | S3 bucket | `BucketSizeBytes` per storage class, `NumberOfObjects` | Daily |
| `gcp` | Cloud SQL instance | `cpu/utilization`, `memory/utilization`, `disk/utilization`, `postgresql/num_backends`, `postgresql/transaction_id_utilization`, `up` | One minute |
| `gcp` | GCS bucket | `storage/v2/total_bytes`, `storage/v2/total_count`, split into live, noncurrent and soft-deleted objects | Daily, repeated every five minutes |

The metric sets are fixed, and values only name the resources.
Each pull is a custom component in the gateway's `gateway-provider` pipeline, instantiated once per listed resource, so changing what is pulled is a change to that pipeline.
The Cloud SQL entries are metric-type prefixes, so `postgresql/num_backends` also pulls `num_backends_by_state` and `num_backends_by_application`, and `up` also pulls `uptime`.

Azure Monitor is not supported yet.

### Metric names

| Provider | Name shape | Example | Resource label |
|---|---|---|---|
| `cloudwatch` | `aws_<service>_<metric>_<statistic>` | `aws_rds_free_storage_space_minimum` | `dimension_DBInstanceIdentifier`, `dimension_BucketName` |
| `gcp` | `stackdriver_<resource type>_<metric type>` | `stackdriver_cloudsql_database_cloudsql_googleapis_com_database_cpu_utilization` | `database_id` (`project:instance`), `bucket_name` |

CloudWatch series also carry `region`, `account_id` and `account_alias`, and `name`, which is `rds` or `s3` on every series.
Join on the `dimension_*` label.

Every provider series carries `job="integrations/cloudwatch"` or `job="integrations/gcp"`.
`instance` is the resource on CloudWatch series, since each resource has its own pull, and `cloudsql` or `gcs` on Cloud Monitoring series, since one pull covers every listed resource of a service.

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

The chart refuses, at render, the values that would otherwise stop the gateway starting: a scrape timeout longer than the interval, and two buckets whose names differ only in `.` and `-`, which would become the same component label.
Alloy accepts both in `alloy validate` and exits on them at load, which would stop logs and metrics along with the pull.

The full set of keys is in the [values reference](../../../reference/helm/materialize-monitoring-values/), under `pipeline.metrics.provider`.

## Identity and grants

Credentials never travel through values.
The pull runs as the gateway pod's own cloud identity, bound through `alloy-gateway.serviceAccount.annotations`.

| Provider | Identity | Grant |
|---|---|---|
| `cloudwatch` | IRSA (`eks.amazonaws.com/role-arn`), EKS Pod Identity, or static keys as `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` in the `mzmon-alloy-gateway-env` Secret | `cloudwatch:GetMetricStatistics`. `iam:ListAccountAliases` fills `account_alias`; without it every pull logs a warning |
| `gcp` | Workload Identity (`iam.gke.io/gcp-service-account`) | `roles/monitoring.viewer` on the project |

On GCP, the service account the Terraform module creates for the Google Cloud Metrics exporter holds `roles/monitoring.metricWriter`, which writes metrics and cannot read them.
Reading needs `roles/monitoring.viewer` added to the same account.

The chart warns at render when a provider is enabled and the gateway's service account carries no matching annotation.
It cannot tell whether EKS Pod Identity, a static key, or a direct Workload Identity principal is in use instead, so the warning is advisory.

The two providers fail differently without a credential.
CloudWatch resolves its credential on each pull, so a missing one produces empty pulls and the gateway keeps running.
The GCP exporter resolves its credential when it starts.
On GKE the metadata server always supplies one, so the result is again empty pulls.
Outside Google Cloud there is no metadata server, so `GOOGLE_APPLICATION_CREDENTIALS` has to point at a mounted key or a Workload Identity Federation configuration.
Without one the exporter cannot start, and the gateway fails to load along with every log and metric it carries.

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
Queries on provider families need `last_over_time(<series>[15m])` or a wider window.

**Every gateway restart leaves a gap as long as the lag.**
Remote write forwards only samples stamped after it started, so that a restart does not resend what was already written.
A Cloud Monitoring sample stamped before the restart is therefore never sent.
After each restart, Cloud SQL series resume a few minutes later and GCS series over ten minutes later.
CloudWatch samples carry the scrape's time, so they have no gap.

No provider signal backs a fast page.
Provider alerts cover the slow-moving conditions — storage headroom, burst-credit exhaustion, connection ceilings — with `for:` windows well above the publication delay.

## Cost

The provider bills each pull, and the bill depends on configuration, not on how many dashboards are open.

| Provider | Calls per pull | Measured |
|---|---|---|
| `cloudwatch` | 12 `GetMetricStatistics` calls per RDS instance and 2 per bucket | 30 calls for two instances and three buckets, in-cluster |
| `gcp` | One descriptor listing per metric prefix, and one time-series listing per matching metric type | 19 calls for two instances and two buckets, counted by `stackdriver_monitoring_api_calls_total` |

`scrapeInterval` is the main lever, and defaults to five minutes.
Rates change, so current provider pricing is the reference for what a call costs.

The gateway runs several replicas, and every replica runs the exporter.
The scrape of it is clustered, so one replica owns it and each provider is called once per interval.

## Which destinations receive it

Each provider assigns its families a tier through `metricImportance`, which defaults to `extended`.
The families `infra-cloud` draws are also named in the query registry, at `diagnostic`, the lowest tier.
A registry tier admits a metric at that tier and above, so `metricImportance` decides for every destination floor above `diagnostic`.

| Destination `minMetricImportance` | Receives provider families at the default |
|---|---|
| `all` (the bundled Thanos) | Yes |
| `diagnostic`, `extended` | Yes |
| `recommended`, `essential` | No |

A destination that bills per series, such as Datadog or a BYOC fan-out, therefore does not receive them unless its floor or the provider's tier is changed.

The Google Cloud Monitoring destination deserves the same care on an install that also pulls from GCP.
Its default floor is `recommended`, which keeps the pulled families out.
Raising either would write each series back into Cloud Monitoring as a custom `prometheus.googleapis.com/` metric: a billed second copy of data Cloud Monitoring already holds.
The pull does not read those types back, so it does not loop.

## What is deliberately not pulled

| Signal | Why |
|---|---|
| GCS `api/request_count` | A per-minute DELTA, and the exporter adds only the newest point of each pull to its counter. At a five-minute interval it reports about a fifth of the real count. The clients report the same requests exactly |
| S3 request metrics | Opt-in per bucket on the AWS side, and billed as custom metrics |
| Resources found by tag | Tag discovery pulls every matching resource in the account, and bills for each |

## Checking it works

**`up` does not say whether a pull succeeded.**
An exporter whose provider call fails still answers its scrape, with no provider series in it.
Measured with credentials missing — for CloudWatch anywhere, and for GCP where a metadata server exists — both exporters return HTTP 200 and report healthy, so `up` stays 1.
`up` only says the exporter exists and is being scraped.

| Question | Query |
|---|---|
| Is the exporter running and scraped | `up{job=~"integrations/(cloudwatch\|gcp)"}` |
| Did the last GCP pull fail | `stackdriver_monitoring_last_scrape_error == 1` |
| Is CloudWatch returning data, per instance | `count by (dimension_DBInstanceIdentifier) (aws_rds_cpuutilization_average)` |
| Is Cloud Monitoring returning data, per instance | `count by (database_id) (last_over_time(stackdriver_cloudsql_database_cloudsql_googleapis_com_database_up[15m]))` |

CloudWatch publishes no equivalent of the GCP error series.
Its `yace_cloudwatch_getmetricstatistics_requests_total` counts billed calls, but it is one counter per gateway replica, and every CloudWatch target a replica owns reports it.
Measured with five targets across two replicas, the five series read 12, 14, 26, 28 and 2: running totals of 28 and 2 calls, not 82.
It cannot be summed across `instance`, so the cost is best read from the configuration: 12 calls per RDS instance and 2 per bucket, each interval.
A CloudWatch pull that fails for want of an identity or a grant is visible only as missing series and as errors in the gateway's logs, such as `Couldn't get account Id`.

The Collection tab of the Infrastructure Cloud Provider dashboard (`infra-cloud`) draws these checks for every provider it finds.
