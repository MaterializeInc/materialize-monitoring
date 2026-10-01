# Cloud Provider Metrics




# Cloud Provider Metrics

The gateway can pull what a cloud provider's monitoring API publishes about the metadata database and the buckets a Materialize deployment depends on,
and about whether the provider can supply the nodes its cluster asks for.
The result is written beside every other metric, with the same retention, the same PromQL, and the same destinations.
A provider series is therefore joinable with `mz_persist_*` in a single expression.
A Grafana CloudWatch, Cloud Monitoring or Azure Monitor datasource cannot offer that.

The **Infrastructure Cloud Provider** dashboard (`infra-cloud`) draws the database and bucket pulls, and **Infrastructure Autoscaling** (`infra-autoscaling`) the node and quota pulls on its Cloud Capacity tab, each with rows for whichever provider it finds; see [Available Dashboards](/materialize-monitoring/preview/claude-dashboard-terminology-updates-c58b28/dashboards/all/).

Provider collection is off by default and adds to what the clients already report about the same dependencies.
Persist, Loki and Thanos measure every request they make against the database and the bucket, at full resolution and at no cost.
The provider adds what no client can see: CPU, memory and storage headroom, the burst credits that throttle a volume, transaction-ID consumption, and bucket growth.
For the cluster's nodes it adds the provider's own view: host and instance health, node groups that cannot reach their desired size, compute quota,
and on AKS the managed cluster autoscaler, which runs where nothing in the cluster can scrape it.
The [external-dependency design](../../../reference/internal/design-docs/20260920-external-dependency-monitoring/#pulling-provider-metrics-into-the-pipeline) records the reasoning.

<!--
Agent note: each pull is a custom component in packages/alloy-pipelines/gateway-provider.yaml, and
charts/materialize-monitoring/templates/_alloy_provider_helpers.tpl renders only the instances. The metric
sets on this page are that file's; change them there, not here first. packages/queries/infra-cloud.yaml and
packages/dashboards/src/grafana/infra_cloud/ draw the database and bucket pulls, and
packages/queries/infra-autoscaling.yaml and infra_autoscaling/cloud.rs the EKS, Compute Engine and AKS
pulls, so a metric added to a pull reaches no panel until those follow, and a metric removed leaves a panel
empty. The numbers on this page
(series counts, API calls, sample ages) were measured on 2026-09-26 against the wrapper-provisioned
GKE and EKS test installs, and on 2026-09-29 against the AKS one, with Alloy v1.20.0 locally and
v1.19.2 in-cluster, and are the ones to re-measure if the exporters are bumped. The Azure call counts
are read from the exporter's code (Alloy v1.19.2, azure-metrics-exporter 5092ac0), since it exports
no counter of its own. The EKS, Compute Engine quota and AKS pulls were measured on 2026-09-30 by
running the rendered components under a local Alloy v1.20.0 against the same three installs' accounts,
not in-cluster: the test installs' gateway identities hold none of the grants they need.
No quota refusal has been observed, so `exceeded` is documented from Cloud Monitoring's descriptors.
-->

## What is pulled

| Provider | Service | Default metrics | Resolution |
|---|---|---|---|
| `cloudwatch` | RDS instance | `CPUUtilization`, `CPUCreditBalance`, `FreeableMemory`, `FreeStorageSpace`, `DatabaseConnections`, `ReadLatency`, `WriteLatency`, `DiskQueueDepth`, `BurstBalance`, `EBSIOBalance%`, `EBSByteBalance%`, `MaximumUsedTransactionIDs` | One minute, summarised per five-minute period |
| `cloudwatch` | S3 bucket | `BucketSizeBytes` per storage class, `NumberOfObjects` | Daily |
| `gcp` | Cloud SQL instance | `cpu/utilization`, `memory/utilization`, `disk/utilization`, `postgresql/num_backends`, `postgresql/transaction_id_utilization`, `up` | One minute |
| `gcp` | GCS bucket | `storage/v2/total_bytes`, `storage/v2/total_count`, split into live, noncurrent and soft-deleted objects | Daily, repeated every five minutes |
| `azure` | PostgreSQL Flexible Server | `cpu_percent`, `cpu_credits_remaining`, `memory_percent`, `storage_percent`, `active_connections`, `connections_failed`, `disk_queue_depth`, `disk_iops_consumed_percentage`, `disk_bandwidth_consumed_percentage`, `maximum_used_transactionIDs`, `is_db_alive` | One minute, summarised per five-minute bucket |
| `azure` | Blob Storage account | `BlobCapacity`, `BlobCount` | Hourly, refreshed about once a day |
| `azure` | Blob Storage account | `Availability`, `SuccessServerLatency`, `SuccessE2ELatency` | One minute, summarised per five-minute bucket |
| `cloudwatch` | EKS cluster's nodes | `StatusCheckFailed_System`, `StatusCheckFailed_Instance`, `StatusCheckFailed_AttachedEBS` per node | One minute, summarised per five-minute period |
| `cloudwatch` | EKS cluster's managed node groups | `GroupDesiredCapacity`, `GroupInServiceInstances`, `GroupPendingInstances`, `GroupMaxSize` | One minute, summarised per five-minute period |
| `cloudwatch` | The region, when any EKS cluster is listed | On-Demand vCPUs in use in the standard families, account-wide (`AWS/Usage` `ResourceCount`) | One minute, summarised per five-minute period |
| `gcp` | Compute Engine region and its zones | `quota/cpus_per_vm_family` and `quota/local_ssd_total_storage_per_vm_family`: usage, limit and refusals | A few times a day; refusals as they happen |
| `azure` | AKS cluster | `cluster_autoscaler_unschedulable_pods_count`, `cluster_autoscaler_cluster_safe_to_autoscale`, `cluster_autoscaler_scale_down_in_cooldown`, `cluster_autoscaler_unneeded_nodes_count` | One minute, summarised per five-minute bucket |
| `azure` | AKS cluster's node scale sets | `VmAvailabilityMetric` per node VM | One minute, summarised per five-minute bucket |

The metric sets are fixed, and values only name the resources.
Each pull is a custom component in the gateway's `gateway-provider` pipeline, so changing what is pulled is a change to that pipeline.
CloudWatch takes one instance of it per listed resource; GCP and Azure take one per service, whose filter names every listed resource.
The Cloud SQL entries are metric-type prefixes, so `postgresql/num_backends` also pulls `num_backends_by_state` and `num_backends_by_application`, and `up` also pulls `uptime`.

Azure applies one aggregation list to every metric in a call.
The Flexible Server pull therefore asks for all four and keeps the one or two each metric needs.
Those are the statistics the RDS pull asks CloudWatch for: average and maximum CPU, minimum credits, maximum connections, and so on.
That is 11 series per server, and 12 on a Burstable tier, which also publishes `cpu_credits_remaining`.

### The nodes and the quota behind them

A node that cannot be had looks the same from inside the cluster whatever the reason: pods stay pending, and the autoscaler reports a failed launch.
These pulls say which reason it is, as far as the provider publishes one.

| Question | EKS | GKE | AKS |
|---|---|---|---|
| Is the host under a node failing | `StatusCheckFailed_System` | — | `VmAvailabilityMetric`, with `Context` saying whether the platform took it down |
| Is a node group short of its desired size | Desired against in-service, for managed node groups | — | Unschedulable pods while the autoscaler is allowed to scale |
| Is quota the limit | vCPUs in use, account-wide; the limit is not in CloudWatch | CPUs and local SSD per family, against the limit, and each refusal | — |

EKS nodes are the one resource found by tag rather than named, because Karpenter and the node groups replace them.
EKS and Karpenter tag every node they launch with `aws:eks:cluster-name`, and each managed node group's Auto Scaling group with `eks:cluster-name`,
and the pull matches those tags to the listed cluster names exactly.
Karpenter's node pools have no group of their own, so the group metrics cover only managed node groups; Karpenter's own metrics cover the rest.
The vCPU usage counts every On-Demand instance in the account and region, not only the cluster's, because that is what the quota counts.
Its limit is the Service Quotas console's `L-1216C47A`.

The Compute Engine quotas are per machine family: C4 and C4A each have their own, and the regional `CPUS` quota covers only older families.
A family appears only once something in the project uses it, so a project running only C4 and C4A has a handful of series per region.
Zonal limits read 2^63 − 1 where no zonal quota applies.
A region is matched with its zones and nothing else, so `europe-west1` does not also pull `europe-west10`.

AKS runs its cluster autoscaler inside the managed control plane.
Its metrics reach no scrape, and Azure Monitor publishes these four instead.
Each keeps the aggregation that shows the worst of its five minutes:
the most pods waiting and the most unneeded nodes, any cooldown, and any moment it was unsafe to scale.
The node pools are scale sets in the cluster's node resource group, which the pull finds through the cluster.

### Metric names

| Provider | Name shape | Example | Resource label |
|---|---|---|---|
| `cloudwatch` | `aws_<service>_<metric>_<statistic>` | `aws_rds_free_storage_space_minimum` | `dimension_DBInstanceIdentifier`, `dimension_BucketName` |
| `gcp` | `stackdriver_<resource type>_<metric type>` | `stackdriver_cloudsql_database_cloudsql_googleapis_com_database_cpu_utilization` | `database_id` (`project:instance`), `bucket_name` |
| `azure` | `azure_<resource type>_<metric>_<aggregation>_<unit>`, lowercased | `azure_microsoft_dbforpostgresql_flexibleservers_storage_percent_maximum_percent` | `resourceName`, and `resourceID` lowercased |

The node and quota pulls follow the same shapes.

| Provider | Families | Resource labels |
|---|---|---|
| `cloudwatch` | `aws_ec2_status_check_failed_{system,instance,attached_ebs}_maximum` | `dimension_InstanceId`, plus `tag_karpenter_sh_nodepool` or `tag_eks_nodegroup_name` |
| `cloudwatch` | `aws_autoscaling_group_{desired_capacity,in_service_instances,pending_instances,max_size}_average` | `dimension_AutoScalingGroupName`, `tag_eks_nodegroup_name` |
| `cloudwatch` | `aws_usage_resource_count_maximum` | `dimension_Class="Standard/OnDemand"`, `region` |
| `gcp` | `stackdriver_compute_googleapis_com_location_compute_googleapis_com_quota_{cpus,local_ssd_total_storage}_per_vm_family_{usage,limit,exceeded}` | `location` (region or zone), `vm_family`, `limit_name` |
| `azure` | `azure_microsoft_containerservice_managedclusters_cluster_autoscaler_*` | `resourceName` |
| `azure` | `azure_microsoft_compute_virtualmachinescalesets_vmavailabilitymetric_minimum_count` | `dimensionVmname` (`<scale set>_<index>`), `dimensionContext`, `resourceName` (the scale set) |

A node joins to `kube_node_info` on its provider ID: the instance ID on EKS, and the scale set and index on AKS.
The discovery pull also writes an `aws_<service>_info` series carrying every tag on the resource, which the pull drops, since tags are free text;
the two tags it keeps are set by EKS and Karpenter.

CloudWatch series also carry `region`, `account_id` and `account_alias`, and `name`, which is `rds` or `s3` on every series.
Join on the `dimension_*` label.

Azure series also carry `resourceGroup`, `subscriptionID` and `subscriptionName`.
They also carry the `interval` and `timespan` each pull reads, as ISO 8601 durations.
The Blob Storage families are named for the blob service, as `azure_microsoft_storage_storageaccounts_blobservices_*`.
No resource tag is copied onto a series; the exporter copies `owner` by default, and the pull turns that off.

Every provider series carries `job="integrations/cloudwatch"`, `job="integrations/gcp"` or `job="integrations/azure"`.
`instance` is the resource on RDS and S3 series, since each resource has its own pull, and `eks` on the EKS pull, which covers every listed cluster.
On Cloud Monitoring series it is `cloudsql`, `gcs`, `compute_quota` or `compute_quota_exceeded`,
since one pull covers every listed resource of a service.
Azure series are the same, with `postgres`, `blob_capacity`, `blob_requests`, `aks_autoscaler` or `aks_nodes`.

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
        eks:
          clusters: [mz-prod-eks]
      gcp:
        enabled: true
        projectId: my-project
        cloudSql:
          instances: [mz-prod-pg]
        gcs:
          buckets: [mz-prod-storage]
        compute:
          regions: [us-east1]
      azure:
        enabled: true
        subscriptionId: 00000000-0000-0000-0000-000000000000
        postgres:
          servers: [mz-prod-pg]
        blob:
          storageAccounts: [mzprodstorage]
        aks:
          clusters: [mz-prod-aks]
```

A Cloud SQL entry is the instance name, not the `project:region:instance` connection name.
A Flexible Server entry is the server name, not its FQDN, and a storage account entry is the account name, not its endpoint.
Both kinds of Azure name are unique across Azure, so no resource group is needed.
An EKS or AKS entry is the cluster's name, not its ARN or resource ID.
AKS names are unique only within a resource group, so two clusters of one name in the subscription are both pulled, and told apart by `resourceGroup`.
A Compute Engine entry is a region, such as `us-east1`; its zones are included, and a zone is refused.
The chart refuses anything else at render, since the names sit inside the Resource Graph query's filter.
`subscriptionId` is the subscription's GUID.
On Azure Government or Azure China, set `cloudEnvironment` to `azureusgovernmentcloud` or `azurechinacloud`.

The chart refuses, at render, the values that would otherwise stop the gateway starting: a scrape timeout longer than the interval, and two buckets whose names differ only in `.` and `-`, which would become the same component label.
Alloy accepts both in `alloy validate` and exits on them at load, which would stop logs and metrics along with the pull.

The full set of keys is in the [values reference](../../../reference/helm/materialize-monitoring-values/), under `pipeline.metrics.provider`.

## Identity and grants

Credentials never travel through values.
The pull runs as the gateway pod's own cloud identity, bound through `alloy-gateway.serviceAccount.annotations`.

| Provider | Identity | Grant |
|---|---|---|
| `cloudwatch` | IRSA (`eks.amazonaws.com/role-arn`), EKS Pod Identity, or static keys as `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` in the `mzmon-alloy-gateway-env` Secret | `cloudwatch:GetMetricStatistics`. `iam:ListAccountAliases` fills `account_alias`; without it every pull logs a warning. EKS clusters also need `cloudwatch:GetMetricData`, `cloudwatch:ListMetrics`, `tag:GetResources` and `autoscaling:DescribeAutoScalingGroups` |
| `gcp` | Workload Identity (`iam.gke.io/gcp-service-account`) | `roles/monitoring.viewer` on the project, which also reads its quota |
| `azure` | Workload identity (`azure.workload.identity/client-id`, plus a pod label), or a service principal as `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and `AZURE_CLIENT_SECRET` in the `mzmon-alloy-gateway-env` Secret | Monitoring Reader on each named server and storage account, and on each named AKS cluster and its node resource group |

Azure workload identity needs two things on the gateway, not one.
The annotation names the identity.
The Entra webhook injects it only into pods labelled `azure.workload.identity/use: "true"`, set through `alloy-gateway.controller.podLabels`.
The chart's Terraform module sets the label whenever the gateway's annotations include `azure.workload.identity/client-id`.
On Azure the grant can be scoped to each resource, unlike on the other two clouds.
Resource Graph returns only the resources the identity can read, so the grant is also what decides which named resources are pulled.

On GCP, the service account the Terraform module creates for the Google Cloud Metrics exporter holds `roles/monitoring.metricWriter`, which writes metrics and cannot read them.
Reading needs `roles/monitoring.viewer` added to the same account.

The chart warns at render when a provider is enabled and the gateway's service account carries no matching annotation.
On Azure it also warns when the annotation has no pod label beside it.
It cannot tell whether EKS Pod Identity, a static key, or a direct Workload Identity principal is in use instead, so the warning is advisory.

The providers fail differently without a credential.
CloudWatch resolves its credential on each pull, so a missing one produces empty pulls and the gateway keeps running.
The GCP exporter resolves its credential when it starts.
On GKE the metadata server always supplies one, so the result is again empty pulls.
Outside Google Cloud there is no metadata server, so `GOOGLE_APPLICATION_CREDENTIALS` has to point at a mounted key or a Workload Identity Federation configuration.
Without one the exporter cannot start, and the gateway fails to load along with every log and metric it carries.
Azure, like CloudWatch, resolves its credential on the first pull, so the gateway keeps running.
On AKS without the pod label, the pull falls through to the node's managed identity, which Resource Graph refuses with a 403.
`up` is 0, and the gateway logs `service discovery failed`.

## Lag

Provider data is minutes old when it arrives, and that decides how it can be used.

| Source | Age on arrival | Timestamp |
|---|---|---|
| CloudWatch RDS | A few minutes | The scrape's |
| CloudWatch S3 | Up to a day | The scrape's |
| Cloud SQL | About three minutes | Cloud Monitoring's |
| GCS | Over ten minutes | Cloud Monitoring's |
| Azure Flexible Server, Blob requests | About a minute | The scrape's |
| Azure Blob capacity | Up to a day | The scrape's |
| CloudWatch EC2, Auto Scaling, vCPU usage | A few minutes | The scrape's |
| Compute Engine quota usage and limit | Up to a day; measured at three points a day | The scrape's |
| Compute Engine quota refusals | A few minutes | Cloud Monitoring's |
| Azure AKS autoscaler and node VMs | About a minute | The scrape's |

CloudWatch and Azure samples are stamped at scrape time.
Stamping a daily S3 datapoint with its own time would put it a day in the past, and the bundled Thanos Receive, which accepts no out-of-order samples, refuses samples that old.
Compute Engine quota usage and limits are stamped at scrape time for the same reason,
since each pull reads back a day to find a point that may be hours old.
A usage point therefore reads as current until the next one lands, and a change can take hours to show.

Cloud Monitoring samples keep their own timestamps.
An instant query at "now" looks back five minutes by default and finds no GCS sample at all.
Queries on provider families need `last_over_time(<series>[15m])` or a wider window.

**Every gateway restart leaves a gap as long as the lag.**
Remote write forwards only samples stamped after it started, so that a restart does not resend what was already written.
A Cloud Monitoring sample stamped before the restart is therefore never sent.
After each restart, Cloud SQL series resume a few minutes later and GCS series over ten minutes later.
CloudWatch and Azure samples carry the scrape's time, so they have no gap.

No provider signal backs a fast page.
Provider alerts cover the slow-moving conditions — storage headroom, burst-credit exhaustion, connection ceilings — with `for:` windows well above the publication delay.

## Cost

The provider bills each pull, and the bill depends on configuration, not on how many dashboards are open.

| Provider | Calls per pull | Measured |
|---|---|---|
| `cloudwatch` | 12 `GetMetricStatistics` calls per RDS instance and 2 per bucket | 30 calls for two instances and three buckets, in-cluster |
| `gcp` | One descriptor listing per metric prefix, and one time-series listing per matching metric type | 19 calls for two instances and two buckets, counted by `stackdriver_monitoring_api_calls_total` |
| `azure` | One Resource Graph query per service, then one metrics call per resource, since each set is under Azure's twenty-per-call limit. Blob Storage is two pulls, capacity and requests | 9 calls for two servers and two accounts, counted from the exporter's code, since it exports no call counter |
| `cloudwatch` EKS | One `tag:GetResources` and one `DescribeAutoScalingGroups`, one `ListMetrics` per metric, `GetMetricData` billed per metric requested, and one `GetMetricStatistics` for vCPU usage | 26 metrics requested for eight nodes and one node group, counted by `yace_cloudwatch_getmetricdata_metrics_requested_total` |
| `gcp` quota | Four descriptor listings and four time-series listings for usage and limit, and two of each for refusals | 12 calls, counted by `stackdriver_monitoring_api_calls_total` |
| `azure` AKS | Two Resource Graph queries, then one metrics call per cluster and one per node scale set | 5 calls for one cluster with two node pools |

`scrapeInterval` is the main lever, and defaults to five minutes.
Rates change, so current provider pricing is the reference for what a call costs.

The gateway runs several replicas, and every replica runs the exporter.
The scrape of it is clustered, so one replica owns it and each provider is called once per interval.

## Which destinations receive it

Each provider assigns its families a tier through `metricImportance`, which defaults to `extended`.
The families `infra-cloud` and `infra-autoscaling` draw are also named in the query registry, at `diagnostic`, the lowest tier.
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
| Azure Blob `Transactions` | The window each pull reads ends at the scrape, so its newest five-minute bucket is a minute or so short, and a count read from it under-counts. `Availability` already falls with throttling and server errors, and the clients report the same requests exactly |
| Azure resource tags | The exporter copies the `owner` tag onto every series by default. Tags are free text, and the pull copies none |
| Resources found by tag | Tag discovery pulls every matching resource in the account, and bills for each. EKS nodes are the exception, found by the cluster tag EKS and Karpenter set, because they are replaced too often to name |
| The EC2 vCPU quota's limit | CloudWatch publishes the usage but not the limit, which only CloudWatch metric math and the Service Quotas API return, and the exporter supports neither |
| Capacity refusals | No provider publishes a metric for a launch it could not fulfil, such as `InsufficientInstanceCapacity`, `ZONE_RESOURCE_POOL_EXHAUSTED` or `AllocationFailed`. Karpenter and the cluster autoscaler report them |
| Azure compute quota | Not an Azure Monitor metric; it is only in the Compute usage API |
| GKE node pool instance groups | `instance_group/size` has no target size beside it, so it says no more than the node count, and the group names are truncated beyond reliable matching |
| The GKE cluster autoscaler | GKE publishes no metrics for it, only its visibility log in Cloud Logging and Kubernetes events |
| AKS `kube_*` and `node_*` platform metrics | kube-state-metrics and node-exporter already report the same, at full resolution |

## Checking it works

**On CloudWatch and GCP, `up` does not say whether a pull succeeded.**
An exporter whose provider call fails still answers its scrape, with no provider series in it.
Measured with credentials missing — for CloudWatch anywhere, and for GCP where a metadata server exists — both exporters return HTTP 200 and report healthy, so `up` stays 1.
`up` only says the exporter exists and is being scraped.

On Azure it says half of it.
A Resource Graph query that fails — no identity, or one with no access to the subscription — fails the scrape, and `up` is 0.
A metrics call that fails for one resource only drops that resource's series and logs a warning.
A resource the identity cannot read is never found at all.
`up` stays 1 in both cases.

| Question | Query |
|---|---|
| Is the exporter running and scraped | `up{job=~"integrations/(cloudwatch\|gcp\|azure)"}` |
| Did the last GCP pull fail | `stackdriver_monitoring_last_scrape_error == 1` |
| Is CloudWatch returning data, per instance | `count by (dimension_DBInstanceIdentifier) (aws_rds_cpuutilization_average)` |
| Is Cloud Monitoring returning data, per instance | `count by (database_id) (last_over_time(stackdriver_cloudsql_database_cloudsql_googleapis_com_database_up[15m]))` |
| Is Azure returning data, per server | `count by (resourceName) (azure_microsoft_dbforpostgresql_flexibleservers_is_db_alive_minimum_count)` |
| Are EKS nodes being found | `count(aws_ec2_status_check_failed_system_maximum)`, against the cluster's node count |
| Is Compute Engine quota arriving | `count by (location) (stackdriver_compute_googleapis_com_location_compute_googleapis_com_quota_cpus_per_vm_family_usage)` |
| Are AKS node VMs being found | `count by (resourceName) (azure_microsoft_compute_virtualmachinescalesets_vmavailabilitymetric_minimum_count)` |

CloudWatch publishes no equivalent of the GCP error series.
Its `yace_cloudwatch_getmetricstatistics_requests_total` counts billed calls, but it is one counter per gateway replica, and every CloudWatch target a replica owns reports it.
Measured with five targets across two replicas, the five series read 12, 14, 26, 28 and 2: running totals of 28 and 2 calls, not 82.
It cannot be summed across `instance`, so the cost is best read from the configuration: 12 calls per RDS instance and 2 per bucket, each interval.
A CloudWatch pull that fails for want of an identity or a grant is visible only as missing series and as errors in the gateway's logs, such as `Couldn't get account Id`.
An EKS pull missing `tag:GetResources` finds no nodes, and one missing `autoscaling:DescribeAutoScalingGroups` finds no node groups,
while the vCPU usage still arrives.

The Collection tab of the Infrastructure Cloud Provider dashboard (`infra-cloud`) draws these checks for every provider it finds.

